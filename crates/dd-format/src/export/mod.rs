//! Writes a domain [`Map`] into a `.dungeondraft_map` template.
//!
//! Only what the domain models is rewritten (walls, doors, floor shapes,
//! tiles, empty cave and terrain, node ids); the template provides the rest.

mod cave;
mod ground;
mod lights;
mod node_ids;
mod objects;
mod packs;
mod patterns;
mod pixels;
mod portal;
mod shapes;
mod terrain;
mod tiles;
mod tracks;
mod walls;

pub use self::pixels::CELL_SIZE_PX;

use std::collections::HashMap;

use dd_domain::{AssetPath, Door, GridSize, Map, WallRef};
use thiserror::Error;

pub use self::cave::unpack as unpack_cave;
use self::cave::write_cave;
use self::ground::{blank_cave, blank_terrain};
use self::lights::build_light;
use self::node_ids::NodeIdAllocator;
use self::objects::build_object;
use self::packs::{AssetUse, declare_packs};
use self::patterns::build_pattern;
use self::portal::{PortalSpec, build_portal};
use self::shapes::build_shapes;
use self::terrain::write_terrain;
pub use self::terrain::{slot_textures, unpack_splat};
use self::tiles::build_tiles;
use self::tracks::build_path;
use self::walls::{build_free_wall, build_wall};
use crate::Preserved;
use crate::model::{
    Color, Environment, Level, Light, MapFile, NodeId, Object, Path, Pattern, Wall,
};

const GROUND_LEVEL: &str = "0";

/// Radius Dungeondraft gives its default door.
pub const DEFAULT_PORTAL_RADIUS: f64 = 128.0;

/// Asset metrics the domain does not know about.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    /// Portal radius in pixels, by asset path.
    pub portal_radii: HashMap<String, f64>,
    pub default_portal_radius: f64,
    /// What `pack.json` says of each known pack, by pack id.
    pub packs: HashMap<String, PackManifest>,
}

/// A pack as its `pack.json` describes it, to declare it in the map header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub keywords: String,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            portal_radii: HashMap::new(),
            default_portal_radius: DEFAULT_PORTAL_RADIUS,
            packs: HashMap::new(),
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
    let paths: Vec<Path> = map
        .tracks()
        .map(|(_, track)| build_path(track, ids.allocate()))
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
            paths,
        },
    );
    if resized {
        reset_ground(level, map.grid());
    }
    write_cave(&mut level.cave, map.cave());
    write_terrain(&mut level.terrain, map.terrain());
    if let Some(argb) = map.ambient_light() {
        set_ambient_light(level, Color(argb));
    }
    let cave_texture = (!map.cave().is_empty()).then(|| level.cave.texture.clone());
    let terrain: Vec<&str> = map
        .terrain()
        .used_textures()
        .map(AssetPath::as_str)
        .collect();
    let mut flagging: Vec<&str> = map
        .assets()
        .map(AssetPath::as_str)
        .filter(|asset| !terrain.contains(asset))
        .collect();
    flagging.extend(cave_texture.as_deref());
    let used = AssetUse {
        flagging: &flagging,
        terrain: &terrain,
    };
    if let Some(header) = file.header_mut() {
        declare_packs(header, &used, settings);
    }
    Ok(file)
}

/// Records built from the domain, their node ids already allocated.
struct Nodes {
    walls: BuiltWalls,
    objects: Vec<Object>,
    lights: Vec<Light>,
    patterns: Vec<Pattern>,
    paths: Vec<Path>,
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

fn set_ambient_light(level: &mut Level, ambient_light: Color) {
    match &mut level.environment {
        Some(environment) => environment.ambient_light = ambient_light,
        None => {
            level.environment = Some(Preserved::new(Environment {
                baked_lighting: true,
                ambient_light,
            }));
        }
    }
}

fn write_level(level: &mut Level, map: &Map, nodes: Nodes) {
    let Nodes {
        walls,
        objects,
        lights,
        patterns,
        paths,
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
    level.paths = paths.into_iter().map(Preserved::new).collect();
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

    use super::{ExportError, ExportSettings, PackManifest, export_map};
    use crate::model::{Color, MapFile, NodeId};

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
            rotation: dd_domain::Rotation::NONE,
            footprint: dd_domain::Footprint::new(1.0, 1.0).unwrap(),
            tint: None,
        })
        .unwrap();
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let objects = &exported.world().levels["0"].objects;
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].node_id, NodeId(2));
        assert_eq!(objects[0].texture, "res://textures/objects/crate.png");
        assert_eq!(exported.world().next_node_id, NodeId(3));
    }

    fn crate_from(pack: &str) -> dd_domain::Prop {
        dd_domain::Prop {
            asset: AssetPath::new(format!("res://packs/{pack}/textures/objects/crate.webp"))
                .unwrap(),
            center: dd_domain::CellPosition::new(2.5, 0.5),
            rotation: dd_domain::Rotation::NONE,
            footprint: dd_domain::Footprint::new(1.0, 1.0).unwrap(),
            tint: None,
        }
    }

    #[test]
    fn declares_every_pack_the_map_uses() {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        map.add_prop(crate_from("LORESP01")).unwrap();
        map.add_prop(crate_from("LOREAP01")).unwrap();
        map.add_prop(crate_from("LOREGD01")).unwrap();
        map.set_background(AssetPath::new("res://textures/terrain/terrain_void.png").unwrap());
        let terrain_only = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        assert!(!terrain_only.header().unwrap().uses_default_assets);
        map.add_prop(dd_domain::Prop {
            asset: AssetPath::new("res://textures/objects/crate.png").unwrap(),
            ..crate_from("LOREGD01")
        })
        .unwrap();
        let starpunx = PackManifest {
            name: "LORE Starpunx A 3.16".to_owned(),
            version: "3.16".to_owned(),
            author: "LORE".to_owned(),
            keywords: String::new(),
        };
        let settings = ExportSettings {
            packs: [("LORESP01".to_owned(), starpunx)].into(),
            ..ExportSettings::default()
        };
        let exported = export_map(&map, &template(), &settings).unwrap();
        let header = exported.header().unwrap();
        assert!(header.uses_default_assets);
        let declared: Vec<(&str, &str)> = header
            .asset_manifest
            .iter()
            .map(|pack| (pack.id.as_str(), pack.name.as_str()))
            .collect();
        assert_eq!(
            declared,
            [
                ("LOREAP01", "LOREAP01"),
                ("LOREGD01", "LORE Grimdorx A 3.09"),
                ("LORENT01", "LORE Natural Textures A 3.03"),
                ("LORESP01", "LORE Starpunx A 3.16"),
            ]
        );
        let text = exported.to_text().unwrap();
        assert!(
            text.contains(
                "\t\t\t\t\"name\": \"LORE Starpunx A 3.16\",\n\
                 \t\t\t\t\"id\": \"LORESP01\",\n\
                 \t\t\t\t\"version\": \"3.16\",\n\
                 \t\t\t\t\"author\": \"LORE\",\n\
                 \t\t\t\t\"keywords\": \"\",\n\
                 \t\t\t\t\"allow_3rd_party_mapping_software_to_read\": false,\n\
                 \t\t\t\t\"custom_color_overrides\": {\n\
                 \t\t\t\t\t\"enabled\": false,\n\
                 \t\t\t\t\t\"min_redness\": 0,"
            ),
            "{}",
            &text[..3000]
        );
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
    fn the_map_terrain_replaces_the_template_one() {
        let template = template();
        let map = Map::new(GridSize::new(48, 27).unwrap());
        let exported = export_map(&map, &template, &ExportSettings::default()).unwrap();
        let terrain = &exported.world().levels["0"].terrain;
        assert!(
            terrain
                .splat
                .0
                .chunks(4)
                .all(|pixel| pixel == [255, 0, 0, 0])
        );
        assert_eq!(
            terrain.texture_2,
            template.world().levels["0"].terrain.texture_2
        );
    }

    #[test]
    fn the_ambient_light_is_written_when_chosen() {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        let untouched = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let ambient = |file: &MapFile| {
            file.world().levels["0"]
                .environment
                .as_ref()
                .map(|environment| environment.ambient_light)
        };
        assert_eq!(ambient(&untouched), Some(Color::WHITE));
        map.set_ambient_light(0xff40_4858);
        let dimmed = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        assert_eq!(ambient(&dimmed), Some(Color(0xff40_4858)));
        assert!(
            dimmed
                .to_text()
                .unwrap()
                .contains("\"baked_lighting\": true,\n\t\t\t\t\t\"ambient_light\": \"ff404858\"")
        );
    }

    #[test]
    fn caves_bring_in_their_texture() {
        let mut map = Map::new(GridSize::new(48, 27).unwrap());
        let plain = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        assert!(!plain.header().unwrap().uses_default_assets);
        map.cave_mut()
            .dig(|position| position.x < 4.0 && position.y < 4.0, true);
        let dug = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        assert!(dug.header().unwrap().uses_default_assets);
    }

    #[test]
    fn tracks_become_paths_after_patterns() {
        let mut map = Map::new(GridSize::new(10, 8).unwrap());
        let cable = dd_domain::Track::new(
            AssetPath::new("res://textures/paths/rope_(thick).png").unwrap(),
            vec![
                dd_domain::CellPosition::new(1.0, 1.0),
                dd_domain::CellPosition::new(4.0, 1.0),
            ],
            0.5,
            false,
        )
        .unwrap();
        map.add_track(cable).unwrap();
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let paths = &exported.world().levels["0"].paths;
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].node_id, NodeId(0));
        assert_eq!(exported.world().next_node_id, NodeId(1));
    }

    #[test]
    fn a_background_covers_the_whole_terrain() {
        let mut map = Map::new(GridSize::new(48, 27).unwrap());
        let void = "res://textures/terrain/terrain_void.png";
        map.set_background(AssetPath::new(void).unwrap());
        let exported = export_map(&map, &template(), &ExportSettings::default()).unwrap();
        let terrain = &exported.world().levels["0"].terrain;
        assert_eq!(terrain.texture_1, void);
        assert!(
            terrain
                .splat
                .0
                .chunks(4)
                .all(|pixel| pixel == [255, 0, 0, 0])
        );
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
