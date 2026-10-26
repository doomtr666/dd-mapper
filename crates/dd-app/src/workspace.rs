//! The open maps of a session and the use cases acting on them.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use dd_catalog::{Asset, AssetKind, AssetQuery, Catalog};
use dd_domain::{
    CellPoint, CellPosition, Door, Footprint, Glow, GridSize, Light, Map, MapError, Partition,
    Pattern, Prop, Room, Rotation, Track, WallPosition,
};

use crate::assets::resolve;
use crate::color::parse_color;
use crate::describe::describe;
use crate::doors::{
    DoorWall, MAX_SNAP_DISTANCE, nearest_wall_position, partition_walls, room_walls,
};
use crate::ids::{
    WallOwner, door_label, light_label, map_label, parse_map, parse_prop, parse_wall_owner,
    partition_label, pattern_label, prop_label, room_label, track_label,
};
use crate::validate::validate;
use crate::{
    AppError, DoorRequest, FloorRequest, LightRequest, MapFiles, PathRequest, PatternRequest,
    PreviewRenderer, PropRequest, RoomRequest, SaveFacts, WallRequest,
};

/// Pixels per cell in the editor, to turn portal radii into cells.
const EDITOR_PIXELS_PER_CELL: f64 = 256.0;
/// Half-width of doors whose radius is unknown, in cells.
const DEFAULT_DOOR_HALF_WIDTH: f64 = 0.5;
/// Reach of lights without a range, in cells: a room-sized glow.
const DEFAULT_LIGHT_RANGE: f64 = 5.0;
const DEFAULT_LIGHT_INTENSITY: f64 = 1.0;
/// The warm white Dungeondraft offers first.
const DEFAULT_LIGHT_COLOR: u32 = 0xffec_cd8b;
/// Opaque white: the texture as it is.
const UNTINTED: u32 = 0xffff_ffff;
/// Width of paths when none is given, in cells.
const DEFAULT_PATH_WIDTH: f64 = 1.0;
/// Footprint of props whose size is unknown, in cells.
const DEFAULT_PROP_SIDE: f64 = 1.0;

/// What adding a room did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomReport {
    /// The room holding the new walls.
    pub id: String,
    /// Rooms merged into it, which no longer exist.
    pub absorbed: Vec<String>,
    /// Doors removed because their wall disappeared.
    pub removed_doors: Vec<String>,
}

/// What opening a file gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReport {
    pub id: String,
    /// What the file holds that the map cannot keep.
    pub warnings: Vec<String>,
}

/// A map of the session, and the file it was opened from.
struct OpenMap {
    map: Map,
    origin: Option<PathBuf>,
}

/// The catalog and the maps being designed.
pub struct Workspace {
    catalog: Catalog,
    maps: BTreeMap<u32, OpenMap>,
    next_map: u32,
    files: Box<dyn MapFiles>,
    renderer: Box<dyn PreviewRenderer>,
}

impl Workspace {
    #[must_use]
    pub fn new(
        catalog: Catalog,
        files: Box<dyn MapFiles>,
        renderer: Box<dyn PreviewRenderer>,
    ) -> Self {
        Self {
            catalog,
            maps: BTreeMap::new(),
            next_map: 1,
            files,
            renderer,
        }
    }

    /// Opens a `.dungeondraft_map` file as a new map of the session.
    /// Saving it later keeps what the map does not model from that file.
    ///
    /// # Errors
    /// Fails when the file cannot be read.
    pub fn open_map(&mut self, source: &Path) -> Result<OpenReport, AppError> {
        let catalog = &self.catalog;
        let footprint_of = |texture: &str| {
            let size = catalog.get(texture)?.metrics.size?;
            Some((size.width, size.height))
        };
        let opened = self.files.open(source, &footprint_of)?;
        let number = self.next_map;
        self.next_map += 1;
        self.maps.insert(
            number,
            OpenMap {
                map: opened.map,
                origin: Some(source.to_owned()),
            },
        );
        Ok(OpenReport {
            id: map_label(number),
            warnings: opened.warnings,
        })
    }

    #[must_use]
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Opens a new empty map and returns its id.
    ///
    /// # Errors
    /// Fails when a dimension is zero.
    pub fn new_map(&mut self, width: u32, height: u32) -> Result<String, AppError> {
        let grid = GridSize::new(width, height)?;
        let number = self.next_map;
        self.next_map += 1;
        self.maps.insert(
            number,
            OpenMap {
                map: Map::new(grid),
                origin: None,
            },
        );
        Ok(map_label(number))
    }

    /// # Errors
    /// Fails on an unknown map, invalid shapes or assets, or an impossible merge.
    pub fn add_room(&mut self, map: &str, request: &RoomRequest) -> Result<RoomReport, AppError> {
        let floor = resolve(&self.catalog, &request.floor, AssetKind::Tileset)?
            .path
            .clone();
        let wall = resolve(&self.catalog, &request.wall, AssetKind::Wall)?
            .path
            .clone();
        let mut room = Room::new(request.shape.to_polygon()?, wall);
        for pillar in &request.pillars {
            room.add_pillar(pillar.to_polygon()?)?;
        }
        let added = self.map_mut(map)?.add_room(room, &floor)?;
        Ok(RoomReport {
            id: room_label(added.id),
            absorbed: added.absorbed.into_iter().map(room_label).collect(),
            removed_doors: added.removed_doors.into_iter().map(door_label).collect(),
        })
    }

    /// Draws a path, such as a conveyor belt or a cable, and returns its id.
    ///
    /// # Errors
    /// Fails on an unknown map or path asset, fewer than two points, a width
    /// that is not positive, or a path wholly outside the map.
    pub fn add_path(&mut self, map: &str, request: &PathRequest) -> Result<String, AppError> {
        let asset = resolve(&self.catalog, &request.asset, AssetKind::Path)?
            .path
            .clone();
        let points = request
            .points
            .iter()
            .map(|&(x, y)| CellPosition::new(x, y))
            .collect();
        let width = request.width.unwrap_or(DEFAULT_PATH_WIDTH);
        let track = Track::new(asset, points, width, request.closed)?;
        let track = if request.sharp {
            track.with_sharp_corners()
        } else {
            track
        };
        let id = self.map_mut(map)?.add_track(track)?;
        Ok(track_label(id))
    }

    /// Repaints the floor under an area; returns how many cells changed.
    ///
    /// # Errors
    /// Fails on an unknown map, an asset that is not a tileset, or an invalid
    /// shape or one leaving the map.
    pub fn paint_floor(&mut self, map: &str, request: &FloorRequest) -> Result<usize, AppError> {
        let tileset = resolve(&self.catalog, &request.tileset, AssetKind::Tileset)?
            .path
            .clone();
        let area = request.shape.to_polygon()?;
        Ok(self.map_mut(map)?.paint_area(&area, &tileset)?)
    }

    /// Adds a free-standing wall, such as a partition, and returns its id.
    ///
    /// # Errors
    /// Fails on an unknown map or wall asset, or invalid or outside points.
    pub fn add_wall(&mut self, map: &str, request: &WallRequest) -> Result<String, AppError> {
        let wall = resolve(&self.catalog, &request.wall, AssetKind::Wall)?
            .path
            .clone();
        let points = request
            .points
            .iter()
            .map(|&(x, y)| CellPoint::new(x, y))
            .collect();
        let partition = Partition::new(points, wall)?;
        let id = self.map_mut(map)?.add_partition(partition)?;
        Ok(partition_label(id))
    }

    /// Puts a door on the wall point of the room nearest to the requested position.
    ///
    /// # Errors
    /// Fails on an unknown map, room or asset, or when no wall is close enough.
    pub fn add_door(&mut self, map: &str, request: &DoorRequest) -> Result<String, AppError> {
        let asset = resolve(&self.catalog, &request.asset, AssetKind::Portal)?;
        let half_width = door_half_width(asset);
        let asset = asset.path.clone();
        let owner = parse_wall_owner(&request.on)?;
        let map = self.map_mut(map)?;
        let position = snap_door(&door_walls(map, owner)?, request, half_width)?;
        let id = map.add_door(Door { position, asset })?;
        Ok(door_label(id))
    }

    /// # Errors
    /// Fails on an unknown map or pattern asset, an invalid shape or color,
    /// or a shape outside the map.
    pub fn add_pattern(&mut self, map: &str, request: &PatternRequest) -> Result<String, AppError> {
        let asset = resolve(&self.catalog, &request.asset, AssetKind::Pattern)?
            .path
            .clone();
        let tint = match &request.color {
            Some(text) => parse_color(text)?,
            None => UNTINTED,
        };
        let pattern = Pattern {
            asset,
            area: request.area.to_free_shape()?,
            tint,
            outlined: request.outlined,
        };
        let id = self.map_mut(map)?.add_pattern(pattern)?;
        Ok(pattern_label(id))
    }

    /// # Errors
    /// Fails on an unknown map or light asset, invalid settings, or a position
    /// outside the map.
    pub fn add_light(&mut self, map: &str, request: &LightRequest) -> Result<String, AppError> {
        let asset = resolve(&self.catalog, &request.asset, AssetKind::Light)?
            .path
            .clone();
        let color = match &request.color {
            Some(text) => parse_color(text)?,
            None => DEFAULT_LIGHT_COLOR,
        };
        let glow = Glow::new(
            request.range.unwrap_or(DEFAULT_LIGHT_RANGE),
            request.intensity.unwrap_or(DEFAULT_LIGHT_INTENSITY),
            color,
        )?;
        let light = Light {
            asset,
            center: CellPosition::new(request.x, request.y),
            glow,
        };
        let id = self.map_mut(map)?.add_light(light)?;
        Ok(light_label(id))
    }

    /// # Errors
    /// Fails on an unknown map or asset, a rotation that is not a number,
    /// or a position outside the map.
    pub fn place_prop(&mut self, map: &str, request: &PropRequest) -> Result<String, AppError> {
        let asset = resolve(&self.catalog, &request.asset, AssetKind::Object)?;
        let prop = Prop {
            asset: asset.path.clone(),
            center: CellPosition::new(request.x, request.y),
            rotation: Rotation::from_degrees(request.rotation_degrees)?,
            footprint: footprint(asset)?,
            tint: request.color.as_deref().map(parse_color).transpose()?,
        };
        let id = self.map_mut(map)?.add_prop(prop)?;
        Ok(prop_label(id))
    }

    /// # Errors
    /// Fails on an unknown map or prop.
    pub fn remove_prop(&mut self, map: &str, prop: &str) -> Result<(), AppError> {
        let id = parse_prop(prop)?;
        self.map_mut(map)?.remove_prop(id)?;
        Ok(())
    }

    /// Sets the light everywhere on the map; dim it so that lights stand out.
    /// Returns the ARGB color set.
    ///
    /// # Errors
    /// Fails on an unknown map or a malformed color.
    pub fn set_ambient_light(&mut self, map: &str, color: &str) -> Result<u32, AppError> {
        let argb = parse_color(color)?;
        self.map_mut(map)?.set_ambient_light(argb);
        Ok(argb)
    }

    /// Covers the whole ground, around and under rooms, with a terrain texture.
    ///
    /// # Errors
    /// Fails on an unknown map, or an asset that is not a terrain texture.
    pub fn set_background(&mut self, map: &str, terrain: &str) -> Result<(), AppError> {
        let terrain = resolve(&self.catalog, terrain, AssetKind::Terrain)?
            .path
            .clone();
        self.map_mut(map)?.set_background(terrain);
        Ok(())
    }

    /// # Errors
    /// Fails on an unknown map.
    pub fn describe(&self, map: &str) -> Result<String, AppError> {
        let (number, found) = self.map(map)?;
        Ok(describe(&map_label(number), found))
    }

    /// # Errors
    /// Fails on an unknown map.
    pub fn validate(&self, map: &str) -> Result<Vec<String>, AppError> {
        Ok(validate(self.map(map)?.1))
    }

    /// # Errors
    /// Fails on an unknown map, or when the preview cannot be drawn.
    pub fn preview_png(&self, map: &str, pixels_per_cell: u32) -> Result<Vec<u8>, AppError> {
        let (_, found) = self.map(map)?;
        Ok(self.renderer.render_png(found, pixels_per_cell)?)
    }

    /// # Errors
    /// Fails on an unknown map, or when the file cannot be written.
    pub fn save(&self, map: &str, destination: &Path) -> Result<(), AppError> {
        let (_, open) = self.entry(map)?;
        let facts = SaveFacts {
            portal_radii: self.portal_radii(),
            packs: self.catalog.packs().cloned().collect(),
        };
        self.files
            .save(&open.map, &facts, open.origin.as_deref(), destination)?;
        Ok(())
    }

    /// How many assets the catalog holds, by pack and by kind.
    #[must_use]
    pub fn catalog_summary(&self) -> String {
        let assets = self.catalog.search(&AssetQuery::default());
        let mut packs: BTreeMap<&str, usize> = BTreeMap::new();
        let mut kinds: BTreeMap<AssetKind, usize> = BTreeMap::new();
        for asset in &assets {
            *packs.entry(asset.pack.as_str()).or_default() += 1;
            *kinds.entry(asset.kind).or_default() += 1;
        }
        format!(
            "{} assets.\nBy pack: {}.\nBy kind: {}.\n",
            assets.len(),
            counts(&packs),
            counts(&kinds)
        )
    }

    #[must_use]
    pub fn search_catalog(&self, query: &AssetQuery) -> Vec<&Asset> {
        self.catalog.search(query)
    }

    pub(crate) fn map(&self, label: &str) -> Result<(u32, &Map), AppError> {
        let (number, open) = self.entry(label)?;
        Ok((number, &open.map))
    }

    fn entry(&self, label: &str) -> Result<(u32, &OpenMap), AppError> {
        let number = parse_map(label)?;
        let open = self
            .maps
            .get(&number)
            .ok_or_else(|| AppError::UnknownMap(label.to_owned()))?;
        Ok((number, open))
    }

    pub(crate) fn map_mut(&mut self, label: &str) -> Result<&mut Map, AppError> {
        let number = parse_map(label)?;
        self.maps
            .get_mut(&number)
            .map(|open| &mut open.map)
            .ok_or_else(|| AppError::UnknownMap(label.to_owned()))
    }

    /// Door and window radii known by the catalog, by asset path.
    fn portal_radii(&self) -> HashMap<String, f64> {
        let portals = self.catalog.search(&AssetQuery {
            kind: Some(AssetKind::Portal),
            ..AssetQuery::default()
        });
        portals
            .into_iter()
            .filter_map(|asset| {
                let radius = portal_radius(asset)?;
                Some((asset.path.as_str().to_owned(), radius))
            })
            .collect()
    }
}

/// The space a prop of `asset` takes, a cell square when unknown.
pub(crate) fn footprint(asset: &Asset) -> Result<Footprint, AppError> {
    let size = asset.metrics.size;
    Ok(Footprint::new(
        size.map_or(DEFAULT_PROP_SIDE, |size| size.width),
        size.map_or(DEFAULT_PROP_SIDE, |size| size.height),
    )?)
}

/// Half the width of a door, in cells.
fn door_half_width(asset: &Asset) -> f64 {
    portal_radius(asset).map_or(DEFAULT_DOOR_HALF_WIDTH, |radius| {
        radius / EDITOR_PIXELS_PER_CELL
    })
}

/// Half the width of a door in editor pixels: as saved maps tell, or half its
/// image's width, which is what Dungeondraft uses.
fn portal_radius(asset: &Asset) -> Option<f64> {
    asset.metrics.portal_radius.or_else(|| {
        let size = asset.metrics.size?;
        Some(size.width * EDITOR_PIXELS_PER_CELL / 2.0)
    })
}

/// The walls of a room or of a free-standing wall.
fn door_walls(map: &Map, owner: WallOwner) -> Result<Vec<DoorWall>, AppError> {
    let walls = match owner {
        WallOwner::Room(id) => room_walls(id, map.room(id).ok_or(MapError::UnknownRoom(id))?),
        WallOwner::Partition(id) => {
            partition_walls(id, map.partition(id).ok_or(MapError::UnknownPartition(id))?)
        }
    };
    Ok(walls)
}

/// The wall position nearest to the requested point, if close enough.
fn snap_door(
    walls: &[DoorWall],
    request: &DoorRequest,
    half_width: f64,
) -> Result<WallPosition, AppError> {
    let point = CellPosition::new(request.x, request.y);
    match nearest_wall_position(walls, point, half_width) {
        Some(snapped) if snapped.distance <= MAX_SNAP_DISTANCE => Ok(snapped.position),
        nearest => Err(AppError::NoWallNearby {
            owner: request.on.clone(),
            x: request.x,
            y: request.y,
            distance: nearest.map_or(f64::INFINITY, |snapped| snapped.distance),
        }),
    }
}

/// `name (count), name (count)`.
fn counts<K: std::fmt::Display>(counts: &BTreeMap<K, usize>) -> String {
    counts
        .iter()
        .map(|(name, count)| format!("{name} ({count})"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use dd_catalog::AssetKind;
    use dd_domain::{Map, PropError};

    use super::{RoomReport, Workspace};
    use crate::assets::tests::{
        BELT, CRATE, DOOR, FLOOR, GRATE, LIGHT, ROLLER, VOID, WALL, small_catalog,
    };
    use crate::{
        AppError, DoorRequest, FloorRequest, FootprintOf, LightRequest, MapFiles, OpenedMap,
        PathRequest, PatternArea, PatternRequest, PortError, PreviewRenderer, PropRequest,
        RoomRequest, SaveFacts, Shape, WallRequest,
    };

    /// What a save was asked: destination, facts, template and room count.
    type Save = (PathBuf, SaveFacts, Option<PathBuf>, usize);
    type Exports = Arc<Mutex<Vec<Save>>>;

    /// Opens a one-room map, telling the footprint it got for the crate;
    /// records what it was asked to write.
    struct FakeFiles(Exports);

    impl MapFiles for FakeFiles {
        fn open(&self, _: &Path, footprint_of: FootprintOf) -> Result<OpenedMap, PortError> {
            let mut map = Map::new(dd_domain::GridSize::new(10, 10).unwrap());
            let outline =
                dd_domain::Polygon::rectangle(dd_domain::CellPoint::new(1, 1), 4, 4).unwrap();
            let wall = dd_domain::AssetPath::new(WALL).unwrap();
            map.add_walls(dd_domain::Room::new(outline, wall)).unwrap();
            Ok(OpenedMap {
                map,
                warnings: vec![format!("crate footprint {:?}", footprint_of(CRATE))],
            })
        }

        fn save(
            &self,
            map: &Map,
            facts: &SaveFacts,
            template: Option<&Path>,
            destination: &Path,
        ) -> Result<(), PortError> {
            self.0.lock().unwrap().push((
                destination.to_owned(),
                facts.clone(),
                template.map(Path::to_owned),
                map.rooms().count(),
            ));
            Ok(())
        }
    }

    /// Answers the pixels per cell it was given.
    struct EchoRenderer;

    impl PreviewRenderer for EchoRenderer {
        fn render_png(&self, _: &Map, pixels_per_cell: u32) -> Result<Vec<u8>, PortError> {
            Ok(pixels_per_cell.to_be_bytes().to_vec())
        }
    }

    pub(crate) fn workspace() -> (Workspace, Exports) {
        let exports = Exports::default();
        let workspace = Workspace::new(
            small_catalog(),
            Box::new(FakeFiles(exports.clone())),
            Box::new(EchoRenderer),
        );
        (workspace, exports)
    }

    fn rectangle(x: i32, y: i32, width: u32, height: u32) -> Shape {
        Shape::Rectangle {
            x,
            y,
            width,
            height,
        }
    }

    fn room(shape: Shape) -> RoomRequest {
        RoomRequest {
            shape,
            pillars: vec![],
            floor: FLOOR.to_owned(),
            wall: WALL.to_owned(),
        }
    }

    fn door_at(on: &str, x: f64, y: f64) -> DoorRequest {
        DoorRequest {
            on: on.to_owned(),
            x,
            y,
            asset: DOOR.to_owned(),
        }
    }

    fn crate_at(x: f64, y: f64, rotation_degrees: f64) -> PropRequest {
        PropRequest {
            asset: CRATE.to_owned(),
            x,
            y,
            rotation_degrees,
            color: None,
        }
    }

    #[test]
    fn props_can_be_colored() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let green = PropRequest {
            color: Some("33ff33".to_owned()),
            ..crate_at(5.0, 5.0, 0.0)
        };
        workspace.place_prop(&map, &green).unwrap();
        assert!(
            workspace
                .describe(&map)
                .unwrap()
                .contains("2 x 1.5 cells, tint ff33ff33")
        );
        let unknown = PropRequest {
            color: Some("acid".to_owned()),
            ..crate_at(8.0, 5.0, 0.0)
        };
        assert_eq!(
            workspace.place_prop(&map, &unknown),
            Err(AppError::InvalidColor("acid".to_owned()))
        );
    }

    #[test]
    fn summarizes_the_catalog() {
        let (workspace, _) = workspace();
        assert_eq!(
            workspace.catalog_summary(),
            "10 assets.\n\
             By pack: LOREAP01 (1), LOREGD01 (8), default (1).\n\
             By kind: tilesets (1), walls (1), portals (2), objects (2), lights (1), patterns (1), \
             terrain (1), paths (1).\n"
        );
    }

    #[test]
    fn maps_get_their_own_ids() {
        let (mut workspace, _) = workspace();
        assert_eq!(workspace.new_map(30, 20), Ok("map-1".to_owned()));
        assert_eq!(workspace.new_map(10, 10), Ok("map-2".to_owned()));
        assert!(matches!(workspace.new_map(0, 3), Err(AppError::Grid(_))));
    }

    #[test]
    fn unknown_maps_are_reported() {
        let (mut workspace, _) = workspace();
        assert_eq!(
            workspace.add_room("map-7", &room(rectangle(0, 0, 2, 2))),
            Err(AppError::UnknownMap("map-7".to_owned()))
        );
    }

    #[test]
    fn rooms_merge_and_report_it() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let first = workspace
            .add_room(&map, &room(rectangle(0, 0, 4, 4)))
            .unwrap();
        let second = workspace
            .add_room(&map, &room(rectangle(2, 0, 4, 4)))
            .unwrap();
        assert_eq!(
            (first.id.as_str(), second),
            (
                "R0",
                RoomReport {
                    id: "R0".to_owned(),
                    absorbed: vec![],
                    removed_doors: vec![]
                }
            )
        );
    }

    #[test]
    fn rooms_need_a_floor_tileset_and_a_wall_texture() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let swapped = RoomRequest {
            floor: WALL.to_owned(),
            ..room(rectangle(0, 0, 2, 2))
        };
        assert_eq!(
            workspace.add_room(&map, &swapped),
            Err(AppError::WrongAssetKind {
                path: WALL.to_owned(),
                expected: AssetKind::Tileset,
                actual: AssetKind::Wall
            })
        );
    }

    #[test]
    fn rooms_can_have_pillars() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let hall = RoomRequest {
            pillars: vec![rectangle(3, 3, 1, 1)],
            ..room(rectangle(0, 0, 8, 8))
        };
        workspace.add_room(&map, &hall).unwrap();
        assert!(workspace.describe(&map).unwrap().contains("1 pillar"));
    }

    #[test]
    fn doors_snap_to_the_nearest_wall() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace
            .add_room(&map, &room(rectangle(0, 0, 6, 4)))
            .unwrap();
        assert_eq!(
            workspace.add_door(&map, &door_at("R0", 3.0, 4.2)),
            Ok("D0".to_owned())
        );
        assert!(
            workspace
                .describe(&map)
                .unwrap()
                .contains("D0: on R0 at (3, 4)")
        );
    }

    #[test]
    fn partitions_split_rooms_and_hold_doors() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace
            .add_room(&map, &room(rectangle(0, 0, 12, 8)))
            .unwrap();
        let partition = WallRequest {
            points: vec![(6, 0), (6, 8)],
            wall: WALL.to_owned(),
        };
        assert_eq!(workspace.add_wall(&map, &partition), Ok("W0".to_owned()));
        assert_eq!(
            workspace.add_door(&map, &door_at("W0", 6.2, 3.0)),
            Ok("D0".to_owned())
        );
        let description = workspace.describe(&map).unwrap();
        assert!(
            description.contains("W0: (6, 0) - (6, 8), Wall_Underhive_Industrial"),
            "{description}"
        );
        assert!(description.contains("D0: on W0 at (6, 3)"), "{description}");
    }

    #[test]
    fn partitions_need_a_wall_texture() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let partition = WallRequest {
            points: vec![(6, 0), (6, 8)],
            wall: DOOR.to_owned(),
        };
        assert!(matches!(
            workspace.add_wall(&map, &partition),
            Err(AppError::WrongAssetKind { .. })
        ));
    }

    #[test]
    fn patterns_can_be_cut_round_and_outlined() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let blast = PatternRequest {
            area: PatternArea::Circle {
                x: 10.0,
                y: 8.0,
                radius: 4.5,
            },
            asset: GRATE.to_owned(),
            color: None,
            outlined: true,
        };
        assert_eq!(workspace.add_pattern(&map, &blast), Ok("S0".to_owned()));
        let description = workspace.describe(&map).unwrap();
        assert!(
            description.contains("from (5.5, 3.5) to (14.5, 12.5), tint ffffffff, outlined"),
            "{description}"
        );
        let too_large = PatternRequest {
            area: PatternArea::Circle {
                x: 10.0,
                y: 8.0,
                radius: 9.0,
            },
            ..blast.clone()
        };
        assert!(matches!(
            workspace.add_pattern(&map, &too_large),
            Err(AppError::Map(
                dd_domain::MapError::PatternOutsideGrid { .. }
            ))
        ));
        let flat = PatternRequest {
            area: PatternArea::Circle {
                x: 10.0,
                y: 8.0,
                radius: 0.0,
            },
            ..blast
        };
        assert!(matches!(
            workspace.add_pattern(&map, &flat),
            Err(AppError::FreeShape(_))
        ));
    }

    #[test]
    fn doors_far_from_every_wall_are_refused() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace
            .add_room(&map, &room(rectangle(0, 0, 8, 8)))
            .unwrap();
        let error = workspace
            .add_door(&map, &door_at("R0", 4.0, 4.0))
            .unwrap_err();
        assert!(matches!(error, AppError::NoWallNearby { distance, .. } if distance > 3.9));
    }

    #[test]
    fn props_take_their_footprint_from_the_catalog() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert_eq!(
            workspace.place_prop(&map, &crate_at(5.0, 5.0, -90.0)),
            Ok("P0".to_owned())
        );
        assert!(workspace.describe(&map).unwrap().contains(
            "P0: Container_Large_Metal_Crate_Green at (5, 5), 270 degrees, 2 x 1.5 cells"
        ));
        workspace
            .place_prop(&map, &crate_at(12.0, 5.0, 37.5))
            .unwrap();
        assert!(
            workspace
                .describe(&map)
                .unwrap()
                .contains("P1: Container_Large_Metal_Crate_Green at (12, 5), 37.5 degrees")
        );
        assert!(matches!(
            workspace.place_prop(&map, &crate_at(5.0, 5.0, f64::NAN)),
            Err(AppError::Prop(PropError::InvalidRotation(_)))
        ));
    }

    #[test]
    fn props_can_be_removed_to_be_placed_again() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace
            .place_prop(&map, &crate_at(5.0, 5.0, 90.0))
            .unwrap();
        assert_eq!(workspace.remove_prop(&map, "P0"), Ok(()));
        assert!(!workspace.describe(&map).unwrap().contains("P0"));
        assert!(matches!(
            workspace.remove_prop(&map, "P0"),
            Err(AppError::Map(_))
        ));
        assert!(matches!(
            workspace.remove_prop(&map, "R0"),
            Err(AppError::MalformedId { kind: "prop", .. })
        ));
    }

    #[test]
    fn floors_can_be_repainted_room_by_room() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace
            .add_room(&map, &room(rectangle(0, 0, 6, 4)))
            .unwrap();
        let grating = FloorRequest {
            shape: rectangle(3, 0, 3, 4),
            tileset: FLOOR.to_owned(),
        };
        assert_eq!(workspace.paint_floor(&map, &grating), Ok(12));
        let wall_as_floor = FloorRequest {
            tileset: WALL.to_owned(),
            ..grating
        };
        assert!(matches!(
            workspace.paint_floor(&map, &wall_as_floor),
            Err(AppError::WrongAssetKind { .. })
        ));
    }

    #[test]
    fn paths_run_through_their_points() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let belt = PathRequest {
            asset: BELT.to_owned(),
            points: vec![(1.0, 2.0), (6.5, 2.0), (6.5, 5.0)],
            width: Some(1.5),
            closed: false,
            sharp: false,
        };
        assert_eq!(workspace.add_path(&map, &belt), Ok("T0".to_owned()));
        let stripes = PathRequest {
            sharp: true,
            ..belt.clone()
        };
        workspace.add_path(&map, &stripes).unwrap();
        assert!(
            workspace
                .describe(&map)
                .unwrap()
                .contains("1.5 cells wide, sharp corners")
        );
        let description = workspace.describe(&map).unwrap();
        assert!(
            description.contains(
                "T0: Path_Conveyor_Belt_Large through (1, 2) - (6.5, 2) - (6.5, 5), 1.5 cells wide"
            ),
            "{description}"
        );
        let crate_path = PathRequest {
            asset: CRATE.to_owned(),
            ..belt
        };
        assert!(matches!(
            workspace.add_path(&map, &crate_path),
            Err(AppError::WrongAssetKind { .. })
        ));
    }

    #[test]
    fn the_ambient_light_dims_the_whole_map() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert_eq!(workspace.set_ambient_light(&map, "404858"), Ok(0xff40_4858));
        assert!(
            workspace
                .describe(&map)
                .unwrap()
                .contains("Ambient light: ff404858")
        );
        assert_eq!(
            workspace.set_ambient_light(&map, "dark"),
            Err(AppError::InvalidColor("dark".to_owned()))
        );
    }

    #[test]
    fn the_background_is_a_terrain_texture() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert_eq!(workspace.set_background(&map, VOID), Ok(()));
        assert!(
            workspace
                .describe(&map)
                .unwrap()
                .contains("Background: terrain_void")
        );
        assert!(matches!(
            workspace.set_background(&map, FLOOR),
            Err(AppError::WrongAssetKind { .. })
        ));
    }

    fn light_at(x: f64, y: f64) -> LightRequest {
        LightRequest {
            asset: LIGHT.to_owned(),
            x,
            y,
            range: None,
            intensity: None,
            color: None,
        }
    }

    #[test]
    fn lights_take_warm_defaults() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert_eq!(
            workspace.add_light(&map, &light_at(4.0, 3.0)),
            Ok("L0".to_owned())
        );
        let tuned = LightRequest {
            range: Some(2.5),
            intensity: Some(1.6),
            color: Some("#80beff".to_owned()),
            ..light_at(8.0, 3.0)
        };
        assert_eq!(workspace.add_light(&map, &tuned), Ok("L1".to_owned()));
        let description = workspace.describe(&map).unwrap();
        assert!(
            description.contains(
                "L0: Effect_Caged_Light_Cast at (4, 3), range 5 cells, intensity 1, color ffeccd8b"
            ),
            "{description}"
        );
        assert!(
            description.contains("L1: Effect_Caged_Light_Cast at (8, 3), range 2.5 cells, intensity 1.6, color ff80beff"),
            "{description}"
        );
    }

    #[test]
    fn lights_need_a_light_asset_and_a_color() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let wall = LightRequest {
            asset: WALL.to_owned(),
            ..light_at(4.0, 3.0)
        };
        assert!(matches!(
            workspace.add_light(&map, &wall),
            Err(AppError::WrongAssetKind { .. })
        ));
        let red = LightRequest {
            color: Some("red".to_owned()),
            ..light_at(4.0, 3.0)
        };
        assert_eq!(
            workspace.add_light(&map, &red),
            Err(AppError::InvalidColor("red".to_owned()))
        );
    }

    #[test]
    fn patterns_cover_an_area_with_an_optional_tint() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let grate = PatternRequest {
            area: PatternArea::Grid(rectangle(11, 16, 1, 1)),
            asset: GRATE.to_owned(),
            color: None,
            outlined: false,
        };
        assert_eq!(workspace.add_pattern(&map, &grate), Ok("S0".to_owned()));
        let tinted = PatternRequest {
            color: Some("7f80beff".to_owned()),
            ..grate.clone()
        };
        assert_eq!(workspace.add_pattern(&map, &tinted), Ok("S1".to_owned()));
        let description = workspace.describe(&map).unwrap();
        assert!(
            description.contains(
                "S0: Tileset_AncientGothic_Quarters_Sewer_Open from (11, 16) to (12, 17), tint ffffffff"
            ),
            "{description}"
        );
        assert!(description.contains("tint 7f80beff"), "{description}");
        let floor_as_pattern = PatternRequest {
            asset: FLOOR.to_owned(),
            ..grate
        };
        assert!(matches!(
            workspace.add_pattern(&map, &floor_as_pattern),
            Err(AppError::WrongAssetKind { .. })
        ));
    }

    #[test]
    fn validation_lists_design_warnings() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert_eq!(
            workspace.validate(&map),
            Ok(vec!["the map has no room nor cave".to_owned()])
        );
    }

    #[test]
    fn previews_come_from_the_renderer() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert_eq!(
            workspace.preview_png(&map, 24),
            Ok(24_u32.to_be_bytes().to_vec())
        );
    }

    #[test]
    fn saving_passes_the_known_door_radii() {
        let (mut workspace, exports) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace
            .add_room(&map, &room(rectangle(0, 0, 6, 4)))
            .unwrap();
        workspace
            .save(&map, Path::new("out/keep.dungeondraft_map"))
            .unwrap();
        let exports = exports.lock().unwrap();
        let (destination, facts, template, rooms) = &exports[0];
        assert_eq!(destination, Path::new("out/keep.dungeondraft_map"));
        assert_eq!(facts.portal_radii.get(DOOR), Some(&154.0));
        assert_eq!(facts.packs.len(), 1);
        assert_eq!(facts.packs[0].name, "LORE Grimdorx A 3.09");
        assert_eq!(*template, None);
        assert_eq!(*rooms, 1);
    }

    #[test]
    fn doors_without_a_known_radius_take_half_their_width() {
        let (mut workspace, exports) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace
            .add_room(&map, &room(rectangle(0, 0, 10, 6)))
            .unwrap();
        let roller = DoorRequest {
            asset: ROLLER.to_owned(),
            ..door_at("R0", 5.0, 6.0)
        };
        workspace.add_door(&map, &roller).unwrap();
        workspace
            .save(&map, Path::new("out/garage.dungeondraft_map"))
            .unwrap();
        let exports = exports.lock().unwrap();
        let radius = exports[0].1.portal_radii[ROLLER];
        assert!((radius - 3.82 * 128.0).abs() < 1e-9, "{radius}");
    }

    #[test]
    fn opened_maps_are_saved_over_their_own_file() {
        let (mut workspace, exports) = workspace();
        let source = Path::new("in/den.dungeondraft_map");
        let opened = workspace.open_map(source).unwrap();
        assert_eq!(
            opened,
            super::OpenReport {
                id: "map-1".to_owned(),
                warnings: vec!["crate footprint Some((2.0, 1.5))".to_owned()],
            }
        );
        assert!(workspace.describe("map-1").unwrap().contains("R0:"));
        workspace
            .save("map-1", Path::new("out/den.dungeondraft_map"))
            .unwrap();
        let exports = exports.lock().unwrap();
        assert_eq!(exports[0].2.as_deref(), Some(source));
    }
}
