//! Writes a domain [`Map`] into a `.dungeondraft_map` template.
//!
//! Only what the domain models is rewritten (walls, doors, floor shapes,
//! tiles, empty cave and terrain, node ids); the template provides the rest.

mod ground;
mod lights;
mod node_ids;
mod objects;
mod patterns;
mod pixels;
mod portal;
mod shapes;
mod tiles;
mod walls;

pub use self::pixels::CELL_SIZE_PX;

use std::collections::HashMap;

use dd_domain::{AssetPath, Door, GridSize, Map, WallRef};
use thiserror::Error;

use self::ground::{blank_cave, blank_terrain};
use self::lights::build_light;
use self::node_ids::NodeIdAllocator;
use self::objects::build_object;
use self::patterns::build_pattern;
use self::portal::{PortalSpec, build_portal};
use self::shapes::build_shapes;
use self::tiles::build_tiles;
use self::walls::{build_free_wall, build_wall};
use crate::Preserved;
use crate::model::{Level, Light, MapFile, NodeId, Object, Pattern, Wall};

const GROUND_LEVEL: &str = "0";

/// Radius Dungeondraft gives its default door.
pub const DEFAULT_PORTAL_RADIUS: f64 = 128.0;

/// Asset metrics the domain does not know about.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    /// Portal radius in pixels, by asset path.
    pub portal_radii: HashMap<String, f64>,
    pub default_portal_radius: f64,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            portal_radii: HashMap::new(),
            default_portal_radius: DEFAULT_PORTAL_RADIUS,
        }
    }
}

impl ExportSettings {
    fn portal_radius(&self, asset: &AssetPath) -> f64 {
        self.portal_radii
            .get(asset.as_str())
            .copied()
            .unwrap_or(self.default_portal_radius)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExportError {
    #[error("the template has no ground level \"0\"")]
    MissingGroundLevel,
}

/// Returns a copy of `template` describing `map`.
///
/// # Errors
/// Fails when the template has no ground level.
pub fn export_map(
    map: &Map,
    template: &MapFile,
    settings: &ExportSettings,
) -> Result<MapFile, ExportError> {
    let mut ids = NodeIdAllocator::default();
    let walls = WallBuilder::new(map, settings, &mut ids).build();
    let objects: Vec<Object> = map
        .props()
        .map(|(_, prop)| build_object(prop, ids.allocate()))
        .collect();
    let lights: Vec<Light> = map
        .lights()
        .map(|(_, light)| build_light(light, ids.allocate()))
        .collect();
    let patterns: Vec<Pattern> = map
        .patterns()
        .map(|(_, pattern)| build_pattern(pattern, ids.allocate()))
        .collect();
    let mut file = template.clone();
    let world = file.world_mut();
    let resized = (world.width, world.height) != (map.grid().width(), map.grid().height());
    world.width = map.grid().width();
    world.height = map.grid().height();
    world.next_node_id = ids.next_free();
    let level = world
        .levels
        .get_mut(GROUND_LEVEL)
        .ok_or(ExportError::MissingGroundLevel)?;
    write_level(
        level,
        map,
        Nodes {
            walls,
            objects,
            lights,
            patterns,
        },
    );
    if resized {
        reset_ground(level, map.grid());
    }
    Ok(file)
}

/// Records built from the domain, their node ids already allocated.
struct Nodes {
    walls: BuiltWalls,
    objects: Vec<Object>,
    lights: Vec<Light>,
    patterns: Vec<Pattern>,
}

/// Every wall, room walls first, with the ids of the room walls.
struct BuiltWalls {
    walls: Vec<Wall>,
    room_wall_ids: Vec<NodeId>,
}

/// Cave and terrain are kept from the template when the size allows it,
/// so that painting done in Dungeondraft survives; otherwise they are reset.
fn reset_ground(level: &mut Level, grid: GridSize) {
    let cave = blank_cave(grid, &level.cave);
    *level.cave = cave;
    let terrain = blank_terrain(grid, &level.terrain);
    *level.terrain = terrain;
}

fn write_level(level: &mut Level, map: &Map, nodes: Nodes) {
    let Nodes {
        walls,
        objects,
        lights,
        patterns,
    } = nodes;
    let BuiltWalls {
        walls,
        room_wall_ids,
    } = walls;
    *level.shapes = build_shapes(map.rooms().map(|(_, room)| room), room_wall_ids);
    level.walls = walls.into_iter().map(Preserved::new).collect();
    let lookup = std::mem::take(&mut level.tiles.lookup);
    *level.tiles = build_tiles(map, lookup);
    level.objects = objects.into_iter().map(Preserved::new).collect();
    level.lights = lights.into_iter().map(Preserved::new).collect();
    level.patterns = patterns.into_iter().map(Preserved::new).collect();
}

/// Builds one wall per room wall loop, then one per partition,
/// then hangs each door on its wall.
struct WallBuilder<'a> {
    map: &'a Map,
    settings: &'a ExportSettings,
    ids: &'a mut NodeIdAllocator,
    walls: Vec<Wall>,
    wall_index: HashMap<WallRef, usize>,
}

impl<'a> WallBuilder<'a> {
    fn new(map: &'a Map, settings: &'a ExportSettings, ids: &'a mut NodeIdAllocator) -> Self {
        Self {
            map,
            settings,
            ids,
            walls: Vec::new(),
            wall_index: HashMap::new(),
        }
    }

    fn build(mut self) -> BuiltWalls {
        for (room_id, room) in self.map.rooms() {
            for (wall_loop, ring) in room.wall_loops() {
                let wall = WallRef::Room {
                    room: room_id,
                    wall: wall_loop,
                };
                self.wall_index.insert(wall, self.walls.len());
                let node_id = self.ids.allocate();
                self.walls
                    .push(build_wall(ring, room.wall().as_str(), node_id));
            }
        }
        let room_wall_ids = self.walls.iter().map(|wall| wall.node_id).collect();
        for (id, partition) in self.map.partitions() {
            self.wall_index
                .insert(WallRef::Partition(id), self.walls.len());
            let node_id = self.ids.allocate();
            self.walls.push(build_free_wall(partition, node_id));
        }
        for (_, door) in self.map.doors() {
            self.add_door(door);
        }
        BuiltWalls {
            walls: self.walls,
            room_wall_ids,
        }
    }

    fn add_door(&mut self, door: &Door) {
        let segment = self
            .map
            .wall_segment(&door.position)
            .expect("the map only accepts doors on existing wall segments");
        let wall = &mut self.walls[self.wall_index[&door.position.wall]];
        let spec = PortalSpec {
            segment,
            segment_index: door.position.segment,
            t: door.position.t,
            wall_id: wall.node_id,
            node_id: self.ids.allocate(),
            texture: door.asset.as_str(),
            radius: self.settings.portal_radius(&door.asset),
        };
        wall.portals.push(Preserved::new(build_portal(&spec)));
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPoint, GridSize, Map, Polygon, Room};

    use super::{ExportError, ExportSettings, export_map};
    use crate::model::{MapFile, NodeId};

    fn template() -> MapFile {
        let text = include_str!("../../../../fixtures/test2.dungeondraft_map");
        MapFile::parse(text).unwrap()
    }

    fn room_with_pillar() -> Room {
        let mut room = Room::new(
            Polygon::rectangle(CellPoint::new(0, 0), 4, 4).unwrap(),
            AssetPath::new("wall.webp").unwrap(),
        );
        room.add_pillar(Polygon::rectangle(CellPoint::new(1, 1), 1, 1).unwrap())
            .unwrap();
        room
    }

    #[test]
    fn an_empty_map_clears_everything_the_domain_models() {
        let map = Map::new(GridSize::new(10, 8).unwrap());
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let world = exported.world();
        assert_eq!((world.width, world.height), (10, 8));
        assert_eq!(world.next_node_id, NodeId(0));
        let level = &world.levels["0"];
        assert!(level.walls.is_empty() && level.shapes.polygons.is_empty());
        assert!(level.objects.is_empty() && level.lights.is_empty());
        assert!(level.patterns.is_empty());
        assert_eq!(level.tiles.cells.0, vec![-1; 80]);
        assert_eq!(level.terrain.splat.0.len(), 10 * 4 * 8 * 4 * 4);
    }

    #[test]
    fn pillars_become_walls_with_their_own_ids() {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        let floor = AssetPath::new("floor.webp").unwrap();
        map.add_room(room_with_pillar(), &floor).unwrap();
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let level = &exported.world().levels["0"];
        let wall_ids: Vec<NodeId> = level.walls.iter().map(|wall| wall.node_id).collect();
        assert_eq!(wall_ids, [NodeId(0), NodeId(1)]);
        assert_eq!(level.shapes.walls, wall_ids);
        assert_eq!(exported.world().next_node_id, NodeId(2));
    }

    #[test]
    fn props_become_objects_after_walls_and_doors() {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        let floor = AssetPath::new("floor.webp").unwrap();
        map.add_room(room_with_pillar(), &floor).unwrap();
        map.add_prop(dd_domain::Prop {
            asset: AssetPath::new("res://textures/objects/crate.png").unwrap(),
            center: dd_domain::CellPosition::new(2.5, 0.5),
            rotation: dd_domain::QuarterTurn::Deg0,
            footprint: dd_domain::Footprint::new(1.0, 1.0).unwrap(),
        })
        .unwrap();
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let objects = &exported.world().levels["0"].objects;
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].node_id, NodeId(2));
        assert_eq!(objects[0].texture, "res://textures/objects/crate.png");
        assert_eq!(exported.world().next_node_id, NodeId(3));
    }

    #[test]
    fn partitions_become_free_walls_outside_floor_shapes() {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        let wall = AssetPath::new("wall.webp").unwrap();
        map.add_room(room_with_pillar(), &AssetPath::new("floor.webp").unwrap())
            .unwrap();
        let points = vec![CellPoint::new(0, 2), CellPoint::new(1, 2)];
        let partition = map
            .add_partition(dd_domain::Partition::new(points, wall.clone()).unwrap())
            .unwrap();
        let position = dd_domain::WallPosition {
            wall: dd_domain::WallRef::Partition(partition),
            segment: 0,
            t: 0.5,
        };
        map.add_door(dd_domain::Door {
            position,
            asset: AssetPath::new("door.webp").unwrap(),
        })
        .unwrap();
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let level = &exported.world().levels["0"];
        let types: Vec<i32> = level.walls.iter().map(|wall| wall.wall_type).collect();
        assert_eq!(types, [0, 0, 1]);
        assert_eq!(level.walls[2].portals.len(), 1);
        assert_eq!(level.walls[2].portals[0].wall_id, level.walls[2].node_id);
        assert_eq!(level.shapes.walls, [NodeId(0), NodeId(1)]);
        assert_eq!(level.shapes.polygons.len(), 2);
        assert_eq!(exported.world().next_node_id, NodeId(4));
    }

    #[test]
    fn lights_come_last_in_node_ids() {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        map.add_room(room_with_pillar(), &AssetPath::new("floor.webp").unwrap())
            .unwrap();
        map.add_light(dd_domain::Light {
            asset: AssetPath::new("res://textures/lights/soft.png").unwrap(),
            center: dd_domain::CellPosition::new(2.0, 2.0),
            glow: dd_domain::Glow::new(4.0, 1.0, 0xffff_ffff).unwrap(),
        })
        .unwrap();
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let lights = &exported.world().levels["0"].lights;
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].node_id, NodeId(2));
        assert_eq!(exported.world().next_node_id, NodeId(3));
    }

    #[test]
    fn keeps_the_painted_terrain_and_cave_of_a_template_of_the_same_size() {
        let template = template();
        let painted = template.world().levels["0"].terrain.splat.clone();
        let map = Map::new(GridSize::new(48, 27).unwrap());
        let exported = export_map(&map, &template, &ExportSettings::default()).unwrap();
        assert_eq!(exported.world().levels["0"].terrain.splat, painted);
    }

    #[test]
    fn requires_a_ground_level_in_the_template() {
        let text = r#"{"world": {"width": 1, "height": 1, "next_node_id": "0", "levels": {}}}"#;
        let template = MapFile::parse(text).unwrap();
        let map = Map::new(GridSize::new(1, 1).unwrap());
        assert_eq!(
            export_map(&map, &template, &ExportSettings::default()),
            Err(ExportError::MissingGroundLevel)
        );
    }
}
