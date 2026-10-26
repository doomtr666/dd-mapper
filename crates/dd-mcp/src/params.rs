//! Tool parameters, as the model sends them. Positions and sizes are in cells.

use dd_app::{
    DEFAULT_MARGIN, DEFAULT_MIN_ROOM, DoorRequest, FloorRequest, Generator, LayoutRequest,
    LightRequest, PatternRequest, PropRequest, RoomRequest, ScatterRequest, Shape, WallRequest,
};
use dd_catalog::{AssetKind, AssetQuery};
use schemars::JsonSchema;
use serde::Deserialize;

/// Default number of catalog results.
const DEFAULT_SEARCH_LIMIT: usize = 20;
/// Default preview scale, in pixels per cell.
pub const DEFAULT_PIXELS_PER_CELL: u32 = 32;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapNewParams {
    /// Width in cells (one cell is 5 feet, 256 pixels in Dungeondraft).
    pub width: u32,
    /// Height in cells.
    pub height: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapOpenParams {
    /// A `.dungeondraft_map` file saved by Dungeondraft.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapIdParams {
    /// Id returned by `map_new`, such as `map-1`.
    pub map_id: String,
}

/// An area whose corners sit on grid vertices.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShapeParams {
    /// Axis-aligned rectangle from its top-left corner.
    Rectangle {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    /// Any simple polygon, corners in order, as `[x, y]` pairs.
    Polygon { points: Vec<[i32; 2]> },
}

impl From<ShapeParams> for Shape {
    fn from(shape: ShapeParams) -> Self {
        match shape {
            ShapeParams::Rectangle {
                x,
                y,
                width,
                height,
            } => Self::Rectangle {
                x,
                y,
                width,
                height,
            },
            ShapeParams::Polygon { points } => {
                Self::Polygon(points.into_iter().map(|[x, y]| (x, y)).collect())
            }
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RoomAddParams {
    pub map_id: String,
    pub shape: ShapeParams,
    /// Solid blocks inside the room, walled all around.
    #[serde(default)]
    pub pillars: Vec<ShapeParams>,
    /// Asset path of a floor tileset (`catalog_search` kind `tileset`).
    pub floor: String,
    /// Asset path of a wall texture (`catalog_search` kind `wall`).
    pub wall: String,
}

impl RoomAddParams {
    pub fn into_request(self) -> (String, RoomRequest) {
        let request = RoomRequest {
            shape: self.shape.into(),
            pillars: self.pillars.into_iter().map(Shape::from).collect(),
            floor: self.floor,
            wall: self.wall,
        };
        (self.map_id, request)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WallAddParams {
    pub map_id: String,
    /// Corners of the wall, in order, as `[x, y]` pairs on grid vertices.
    /// The wall does not close: use it to split a room into parts.
    pub points: Vec<[i32; 2]>,
    /// Asset path of a wall texture (`catalog_search` kind `wall`).
    pub wall: String,
}

impl WallAddParams {
    pub fn into_request(self) -> (String, WallRequest) {
        let request = WallRequest {
            points: self.points.into_iter().map(|[x, y]| (x, y)).collect(),
            wall: self.wall,
        };
        (self.map_id, request)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DoorAddParams {
    pub map_id: String,
    /// Where the door goes: a room id such as `R0` (its outline or pillars)
    /// or a wall id such as `W0` from `wall_add`.
    pub on: String,
    /// The door goes on the wall point nearest to (x, y).
    pub x: f64,
    pub y: f64,
    /// Asset path of a door or window (`catalog_search` kind `portal`, not `object`).
    pub asset: String,
}

impl DoorAddParams {
    pub fn into_request(self) -> (String, DoorRequest) {
        let request = DoorRequest {
            on: self.on,
            x: self.x,
            y: self.y,
            asset: self.asset,
        };
        (self.map_id, request)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PatternAddParams {
    pub map_id: String,
    /// The area to cover, on grid vertices.
    pub shape: ShapeParams,
    /// Asset path of a pattern texture (`catalog_search` kind `pattern`).
    pub asset: String,
    /// `RRGGBB` or `AARRGGBB` tint; none keeps the texture as is.
    pub color: Option<String>,
}

impl PatternAddParams {
    pub fn into_request(self) -> (String, PatternRequest) {
        let request = PatternRequest {
            shape: self.shape.into(),
            asset: self.asset,
            color: self.color,
        };
        (self.map_id, request)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LightAddParams {
    pub map_id: String,
    /// Asset path of a light texture (`catalog_search` kind `light`).
    pub asset: String,
    /// Center of the light, in cells.
    pub x: f64,
    pub y: f64,
    /// Radius of the glow in cells, 5 by default.
    pub range: Option<f64>,
    /// Brightness, 1 by default; 1.5 to 2 for strong sources.
    pub intensity: Option<f64>,
    /// `RRGGBB` or `AARRGGBB`, warm white `ffeccd8b` by default.
    pub color: Option<String>,
}

impl LightAddParams {
    pub fn into_request(self) -> (String, LightRequest) {
        let request = LightRequest {
            asset: self.asset,
            x: self.x,
            y: self.y,
            range: self.range,
            intensity: self.intensity,
            color: self.color,
        };
        (self.map_id, request)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PropPlaceParams {
    pub map_id: String,
    /// Asset path of an object (`catalog_search` kind `object`).
    pub asset: String,
    /// Center of the prop, in cells.
    pub x: f64,
    pub y: f64,
    /// Clockwise rotation in degrees: 0, 90, 180 or 270.
    #[serde(default)]
    pub rotation: i32,
}

impl PropPlaceParams {
    pub fn into_request(self) -> (String, PropRequest) {
        let request = PropRequest {
            asset: self.asset,
            x: self.x,
            y: self.y,
            rotation_degrees: self.rotation,
        };
        (self.map_id, request)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapPreviewParams {
    pub map_id: String,
    /// Scale of the image, 32 by default; lower it for large maps.
    pub pixels_per_cell: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapSaveParams {
    pub map_id: String,
    /// Destination `.dungeondraft_map` file.
    pub path: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KindParams {
    Tileset,
    Wall,
    Portal,
    Object,
    Light,
    Pattern,
    Terrain,
    Path,
}

impl From<KindParams> for AssetKind {
    fn from(kind: KindParams) -> Self {
        match kind {
            KindParams::Tileset => Self::Tileset,
            KindParams::Wall => Self::Wall,
            KindParams::Portal => Self::Portal,
            KindParams::Object => Self::Object,
            KindParams::Light => Self::Light,
            KindParams::Pattern => Self::Pattern,
            KindParams::Terrain => Self::Terrain,
            KindParams::Path => Self::Path,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CatalogSearchParams {
    /// Words found in asset names, folders or tags, such as `crate metal`.
    #[serde(default)]
    pub query: String,
    pub kind: Option<KindParams>,
    /// Tags every result must carry.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Pack id such as `LOREGD01`, or `default` for Dungeondraft's own assets.
    pub pack: Option<String>,
    /// At most this many results, 20 by default.
    pub limit: Option<usize>,
}

impl From<CatalogSearchParams> for AssetQuery {
    fn from(params: CatalogSearchParams) -> Self {
        Self {
            text: params.query,
            kind: params.kind.map(AssetKind::from),
            tags: params.tags,
            pack: params.pack,
            limit: params.limit.unwrap_or(DEFAULT_SEARCH_LIMIT),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GeneratorParams {
    /// Rooms split at random: manufactorum, bunker, administratum offices.
    Bsp,
    /// Cells on both sides of a 2-cell corridor, entered at both ends.
    HabBlock,
    /// A hab block with a pointed bow, entered from the stern.
    ShipDeck,
}

impl From<GeneratorParams> for Generator {
    fn from(generator: GeneratorParams) -> Self {
        match generator {
            GeneratorParams::Bsp => Self::Bsp,
            GeneratorParams::HabBlock => Self::HabBlock,
            GeneratorParams::ShipDeck => Self::ShipDeck,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LayoutGenerateParams {
    pub generator: GeneratorParams,
    /// Map width in cells.
    pub width: u32,
    /// Map height in cells.
    pub height: u32,
    /// Narrowest room in cells, 3 by default.
    pub min_room: Option<u32>,
    /// Free cells around the building, 1 by default: 4 leaves room for engines
    /// behind a ship deck.
    pub margin: Option<u32>,
    /// The same seed draws the same building; change it for another one.
    #[serde(default)]
    pub seed: u64,
    /// Asset path of a floor tileset (`catalog_search` kind `tileset`).
    pub floor: String,
    /// Asset path of a wall texture (`catalog_search` kind `wall`).
    pub wall: String,
    /// Asset path of a door (`catalog_search` kind `portal`).
    pub door: String,
}

impl From<LayoutGenerateParams> for LayoutRequest {
    fn from(params: LayoutGenerateParams) -> Self {
        Self {
            generator: params.generator.into(),
            width: params.width,
            height: params.height,
            min_room: params.min_room.unwrap_or(DEFAULT_MIN_ROOM),
            margin: params.margin.unwrap_or(DEFAULT_MARGIN),
            seed: params.seed,
            floor: params.floor,
            wall: params.wall,
            door: params.door,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PropRemoveParams {
    pub map_id: String,
    /// Id of the prop, such as `P3`.
    pub prop_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PropScatterParams {
    pub map_id: String,
    /// Asset paths of small objects to pick from at random (`catalog_search` kind
    /// `object`): debris, cans, papers, cables, bones...
    pub assets: Vec<String>,
    /// The area to strew, on grid vertices; props stay inside rooms.
    pub shape: ShapeParams,
    /// How many props to place.
    pub count: u32,
    /// The same seed strews the same way; change it for another layout.
    #[serde(default)]
    pub seed: u64,
}

impl PropScatterParams {
    pub fn into_request(self) -> (String, ScatterRequest) {
        let request = ScatterRequest {
            assets: self.assets,
            area: self.shape.into(),
            count: self.count,
            seed: self.seed,
        };
        (self.map_id, request)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapBackgroundParams {
    pub map_id: String,
    /// Asset path of a terrain texture (`catalog_search` kind `terrain`).
    pub asset: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FloorPaintParams {
    pub map_id: String,
    /// The area to repaint, on grid vertices, such as one room.
    pub shape: ShapeParams,
    /// Asset path of a floor tileset (`catalog_search` kind `tileset`).
    pub tileset: String,
}

impl FloorPaintParams {
    pub fn into_request(self) -> (String, FloorRequest) {
        let request = FloorRequest {
            shape: self.shape.into(),
            tileset: self.tileset,
        };
        (self.map_id, request)
    }
}
