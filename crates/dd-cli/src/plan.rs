//! Map plans: a TOML file describing a map, built through the use cases.
//!
//! ```toml
//! [style]                      # defaults for rooms, walls and doors
//! floor = "res://packs/.../Tileset_RT_Punched_Floor.webp"
//! wall = "res://packs/.../Wall_Underhive_Industrial.webp"
//! door = "res://packs/.../Door_Underhive_Industrial.webp"
//! background = "res://textures/terrain/terrain_void.png"   # the ground around rooms
//! ambient = "404858"           # light everywhere; dim it so that lights matter
//!
//! [map]                        # a blank map...
//! width = 30
//! height = 20
//!
//! # [generate]                 # ...or a generated building
//! # generator = "hab_block"    # bsp, hab_block or ship_deck
//! # width = 30
//! # height = 14
//! # seed = 7
//! # min_room = 3
//! # margin = 1                 # free cells around the building
//! # hall = false               # bsp: a large hall between two wings of rooms
//!
//! [[cave_network]]             # caverns grown over a rectangle of cells
//! x = 1
//! y = 1
//! width = 20
//! height = 18
//! density = 0.47               # share of starting rock: lower is more open
//! seed = 3
//!
//! [[cave]]                     # dig (or fill = true) a chamber, tunnel, rectangle or polygon
//! shape = { type = "chamber", x = 24, y = 10, radius = 3 }
//! # shape = { type = "tunnel", points = [[17, 10], [24, 10]], width = 2 }
//! roughness = 0.4              # 0 smooth to 1 jagged walls
//! seed = 1
//!
//! [[cave_entrance]]            # walls left open where a path comes in
//! shape = { type = "chamber", x = 27, y = 10, radius = 1 }
//!
//! [[terrain]]                  # ground painted over the background, fading out
//! asset = "res://packs/.../Tileset_Ash.webp"
//! shape = { type = "chamber", x = 20, y = 14, radius = 8 }
//! falloff = 2                  # cells of fade at the edge
//! roughness = 0.4
//!
//! [[room]]                     # R0, R1... in order
//! shape = { type = "rectangle", x = 1, y = 1, width = 12, height = 8 }
//! pillars = [{ type = "rectangle", x = 6, y = 4, width = 1, height = 1 }]
//!
//! [[wall]]                     # W0, W1...
//! points = [[6, 1], [6, 9]]
//!
//! [[door]]                     # on a room (R0) or a wall (W0)
//! on = "W0"
//! at = [6, 2.5]
//!
//! [[prop]]
//! asset = "res://packs/.../Container_Large_Metal_Crate_Green.webp"
//! at = [3, 3]
//! rotation = 90
//! # color = "5aff3c"          # custom color, for assets Dungeondraft lets color
//!
//! [[light]]
//! asset = "res://packs/.../Effect_Caged_Light_Cast.webp"
//! at = [4, 4]
//! range = 5
//! color = "80beff"
//!
//! [[floor]]                    # another floor for part of the map
//! shape = { type = "rectangle", x = 7, y = 1, width = 6, height = 8 }
//! tileset = "res://packs/.../Floor_SpaceShip_Grate_Worn.webp"
//!
//! [[path]]                     # conveyor belts, cables, pipes, trails
//! asset = "res://packs/.../Path_Conveyor_Belt_Large.webp"
//! points = [[2.5, 7], [12, 7], [12, 3.5]]   # cells, anywhere, even off the map
//! width = 1.5                  # cells, 1 by default
//! closed = false
//! sharp = false                # true: straight segments, for stripes, fences, pipes
//!
//! [[scatter]]                  # clutter strewn at random inside rooms
//! assets = ["res://packs/.../Debris_Can_Empty.webp"]
//! shape = { type = "rectangle", x = 1, y = 1, width = 12, height = 8 }
//! count = 15
//! seed = 3
//! along_walls = false          # true lines them along the walls, facing the room
//! outdoors = false             # true strews open ground too: streets, plazas
//!
//! [[pattern]]
//! asset = "res://packs/.../Tileset_AncientGothic_Quarters_Sewer_Open.webp"
//! shape = { type = "polygon", points = [[8, 2], [10, 2], [10, 4]] }
//! ```

use anyhow::Context as _;
use dd_app::{
    CaveNetwork, CaveRequest, DEFAULT_MARGIN, DEFAULT_MIN_ROOM, DoorRequest, FloorRequest,
    Generator, LayoutRequest, LightRequest, NaturalShape, PathRequest, PatternRequest, PropRequest,
    RoomRequest, ScatterRequest, Shape, TerrainRequest, WallRequest, Workspace,
};
use serde::Deserialize;

/// A map plan, as read from TOML.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    #[serde(default)]
    style: Style,
    map: Option<MapSize>,
    generate: Option<GeneratePlan>,
    #[serde(default)]
    room: Vec<RoomPlan>,
    #[serde(default)]
    wall: Vec<WallPlan>,
    #[serde(default)]
    door: Vec<DoorPlan>,
    #[serde(default)]
    prop: Vec<PropPlan>,
    #[serde(default)]
    light: Vec<LightPlan>,
    #[serde(default)]
    pattern: Vec<PatternPlan>,
    #[serde(default)]
    scatter: Vec<ScatterPlan>,
    #[serde(default)]
    floor: Vec<FloorPlan>,
    #[serde(default)]
    path: Vec<PathPlan>,
    #[serde(default)]
    cave_network: Vec<CaveNetworkPlan>,
    #[serde(default)]
    cave: Vec<CavePlan>,
    #[serde(default)]
    cave_entrance: Vec<CaveEntrancePlan>,
    #[serde(default)]
    terrain: Vec<TerrainPlan>,
}

/// Width of the fade at the edge of painted terrain when none is given, in cells.
const DEFAULT_FALLOFF: f64 = 1.5;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerrainPlan {
    asset: String,
    shape: NaturalShapePlan,
    falloff: Option<f64>,
    roughness: Option<f64>,
    #[serde(default)]
    seed: u64,
}

/// Roughness of dug walls when none is given.
const DEFAULT_ROUGHNESS: f64 = 0.4;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum NaturalShapePlan {
    Chamber {
        x: f64,
        y: f64,
        radius: f64,
    },
    Tunnel {
        points: Vec<[f64; 2]>,
        width: f64,
    },
    Rectangle {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    Polygon {
        points: Vec<[i32; 2]>,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaveNetworkPlan {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    density: Option<f64>,
    #[serde(default)]
    seed: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CavePlan {
    shape: NaturalShapePlan,
    #[serde(default)]
    fill: bool,
    roughness: Option<f64>,
    #[serde(default)]
    seed: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaveEntrancePlan {
    shape: NaturalShapePlan,
}

/// Assets used where an element names none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Style {
    floor: Option<String>,
    wall: Option<String>,
    door: Option<String>,
    /// Terrain texture covering the whole ground.
    background: Option<String>,
    /// `RRGGBB` or `AARRGGBB` light everywhere, before lights add theirs.
    ambient: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct MapSize {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GeneratorName {
    Bsp,
    HabBlock,
    ShipDeck,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratePlan {
    generator: GeneratorName,
    width: u32,
    height: u32,
    #[serde(default)]
    seed: u64,
    min_room: Option<u32>,
    margin: Option<u32>,
    #[serde(default)]
    hall: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ShapePlan {
    Rectangle {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    Polygon {
        points: Vec<[i32; 2]>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomPlan {
    shape: ShapePlan,
    #[serde(default)]
    pillars: Vec<ShapePlan>,
    floor: Option<String>,
    wall: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct WallPlan {
    points: Vec<[i32; 2]>,
    wall: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct DoorPlan {
    on: String,
    at: [f64; 2],
    asset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PropPlan {
    asset: String,
    at: [f64; 2],
    /// Clockwise degrees, any angle.
    #[serde(default)]
    rotation: f64,
    color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct LightPlan {
    asset: String,
    at: [f64; 2],
    range: Option<f64>,
    intensity: Option<f64>,
    color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatternPlan {
    asset: String,
    shape: ShapePlan,
    color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PathPlan {
    asset: String,
    points: Vec<[f64; 2]>,
    width: Option<f64>,
    #[serde(default)]
    closed: bool,
    #[serde(default)]
    sharp: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct FloorPlan {
    shape: ShapePlan,
    tileset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScatterPlan {
    assets: Vec<String>,
    shape: ShapePlan,
    count: u32,
    #[serde(default)]
    seed: u64,
    #[serde(default)]
    along_walls: bool,
    color: Option<String>,
    #[serde(default)]
    outdoors: bool,
}

impl Plan {
    /// # Errors
    /// Fails on invalid TOML or unknown keys.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(text)?)
    }

    /// Builds the planned map in `workspace` and returns its id.
    ///
    /// # Errors
    /// Fails on the first element that cannot be added, naming it.
    pub fn build(&self, workspace: &mut Workspace) -> anyhow::Result<String> {
        let map = self.create_map(workspace)?;
        self.dig_caves(workspace, &map)?;
        if let Some(background) = &self.style.background {
            workspace
                .set_background(&map, background)
                .context("[style] background")?;
        }
        if let Some(ambient) = &self.style.ambient {
            workspace
                .set_ambient_light(&map, ambient)
                .context("[style] ambient")?;
        }
        self.add_structure(workspace, &map)?;
        self.furnish(workspace, &map)?;
        Ok(map)
    }

    /// Cave networks, dug or filled shapes, entrances, then painted terrain.
    fn dig_caves(&self, workspace: &mut Workspace, map: &str) -> anyhow::Result<()> {
        each(&self.cave_network, "cave_network", |network| {
            workspace.grow_caves(map, &network.network())?;
            Ok(())
        })?;
        each(&self.cave, "cave", |cave| {
            workspace.carve_cave(map, &cave.request())?;
            Ok(())
        })?;
        each(&self.cave_entrance, "cave_entrance", |entrance| {
            workspace.open_cave_entrance(map, &(&entrance.shape).into())?;
            Ok(())
        })?;
        each(&self.terrain, "terrain", |terrain| {
            workspace.paint_terrain(map, &terrain.request())?;
            Ok(())
        })
    }

    /// Rooms, walls, doors and repainted floors.
    fn add_structure(&self, workspace: &mut Workspace, map: &str) -> anyhow::Result<()> {
        let style = &self.style;
        each(&self.room, "room", |room| {
            workspace.add_room(map, &room.request(style)?)?;
            Ok(())
        })?;
        each(&self.wall, "wall", |wall| {
            workspace.add_wall(map, &wall.request(style)?)?;
            Ok(())
        })?;
        each(&self.door, "door", |door| {
            workspace.add_door(map, &door.request(style)?)?;
            Ok(())
        })?;
        each(&self.floor, "floor", |floor| {
            workspace.paint_floor(map, &floor.request())?;
            Ok(())
        })
    }

    /// Floor patterns, paths, props, clutter and lights.
    fn furnish(&self, workspace: &mut Workspace, map: &str) -> anyhow::Result<()> {
        each(&self.pattern, "pattern", |pattern| {
            workspace.add_pattern(map, &pattern.request())?;
            Ok(())
        })?;
        each(&self.path, "path", |path| {
            workspace.add_path(map, &path.request())?;
            Ok(())
        })?;
        each(&self.prop, "prop", |prop| {
            workspace.place_prop(map, &prop.request())?;
            Ok(())
        })?;
        each(&self.scatter, "scatter", |scatter| {
            workspace.scatter(map, &scatter.request())?;
            Ok(())
        })?;
        each(&self.light, "light", |light| {
            workspace.add_light(map, &light.request())?;
            Ok(())
        })
    }

    fn create_map(&self, workspace: &mut Workspace) -> anyhow::Result<String> {
        match (&self.map, &self.generate) {
            (Some(size), None) => Ok(workspace.new_map(size.width, size.height)?),
            (None, Some(generate)) => {
                let request = generate.request(&self.style)?;
                let report = workspace.generate(&request).context("[generate]")?;
                Ok(report.map)
            }
            _ => anyhow::bail!("a plan needs either [map] or [generate], not both"),
        }
    }
}

/// Adds every item, naming the first one that fails: `[[door]] 2: ...`.
fn each<T>(
    items: &[T],
    table: &str,
    mut add: impl FnMut(&T) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    for (index, item) in items.iter().enumerate() {
        add(item).with_context(|| format!("[[{table}]] {}", index + 1))?;
    }
    Ok(())
}

/// The element's own asset, or the style's.
fn styled(own: Option<&String>, style: Option<&String>, what: &str) -> anyhow::Result<String> {
    own.or(style)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no {what}: set {what} here or in [style]"))
}

fn points(points: &[[i32; 2]]) -> Vec<(i32, i32)> {
    points.iter().map(|&[x, y]| (x, y)).collect()
}

impl From<GeneratorName> for Generator {
    fn from(name: GeneratorName) -> Self {
        match name {
            GeneratorName::Bsp => Self::Bsp,
            GeneratorName::HabBlock => Self::HabBlock,
            GeneratorName::ShipDeck => Self::ShipDeck,
        }
    }
}

impl From<&ShapePlan> for Shape {
    fn from(shape: &ShapePlan) -> Self {
        match shape {
            &ShapePlan::Rectangle {
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
            ShapePlan::Polygon { points: corners } => Self::Polygon(points(corners)),
        }
    }
}

impl GeneratePlan {
    fn request(&self, style: &Style) -> anyhow::Result<LayoutRequest> {
        Ok(LayoutRequest {
            generator: self.generator.into(),
            width: self.width,
            height: self.height,
            min_room: self.min_room.unwrap_or(DEFAULT_MIN_ROOM),
            margin: self.margin.unwrap_or(DEFAULT_MARGIN),
            hall: self.hall,
            seed: self.seed,
            floor: styled(None, style.floor.as_ref(), "floor").context("[generate]")?,
            wall: styled(None, style.wall.as_ref(), "wall").context("[generate]")?,
            door: styled(None, style.door.as_ref(), "door").context("[generate]")?,
        })
    }
}

impl RoomPlan {
    fn request(&self, style: &Style) -> anyhow::Result<RoomRequest> {
        Ok(RoomRequest {
            shape: (&self.shape).into(),
            pillars: self.pillars.iter().map(Shape::from).collect(),
            floor: styled(self.floor.as_ref(), style.floor.as_ref(), "floor")?,
            wall: styled(self.wall.as_ref(), style.wall.as_ref(), "wall")?,
        })
    }
}

impl WallPlan {
    fn request(&self, style: &Style) -> anyhow::Result<WallRequest> {
        Ok(WallRequest {
            points: points(&self.points),
            wall: styled(self.wall.as_ref(), style.wall.as_ref(), "wall")?,
        })
    }
}

impl DoorPlan {
    fn request(&self, style: &Style) -> anyhow::Result<DoorRequest> {
        let [x, y] = self.at;
        Ok(DoorRequest {
            on: self.on.clone(),
            x,
            y,
            asset: styled(self.asset.as_ref(), style.door.as_ref(), "door")?,
        })
    }
}

impl PropPlan {
    fn request(&self) -> PropRequest {
        let [x, y] = self.at;
        PropRequest {
            asset: self.asset.clone(),
            x,
            y,
            rotation_degrees: self.rotation,
            color: self.color.clone(),
        }
    }
}

impl LightPlan {
    fn request(&self) -> LightRequest {
        let [x, y] = self.at;
        LightRequest {
            asset: self.asset.clone(),
            x,
            y,
            range: self.range,
            intensity: self.intensity,
            color: self.color.clone(),
        }
    }
}

impl From<&NaturalShapePlan> for NaturalShape {
    fn from(shape: &NaturalShapePlan) -> Self {
        match shape {
            &NaturalShapePlan::Chamber { x, y, radius } => Self::Chamber { x, y, radius },
            NaturalShapePlan::Tunnel { points, width } => Self::Tunnel {
                points: points.iter().map(|&[x, y]| (x, y)).collect(),
                width: *width,
            },
            &NaturalShapePlan::Rectangle {
                x,
                y,
                width,
                height,
            } => Self::Area(Shape::Rectangle {
                x,
                y,
                width,
                height,
            }),
            NaturalShapePlan::Polygon { points: corners } => {
                Self::Area(Shape::Polygon(points(corners)))
            }
        }
    }
}

impl CaveNetworkPlan {
    fn network(&self) -> CaveNetwork {
        CaveNetwork {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            density: self.density,
            seed: self.seed,
        }
    }
}

impl TerrainPlan {
    fn request(&self) -> TerrainRequest {
        TerrainRequest {
            asset: self.asset.clone(),
            shape: (&self.shape).into(),
            falloff: self.falloff.unwrap_or(DEFAULT_FALLOFF),
            roughness: self.roughness.unwrap_or(DEFAULT_ROUGHNESS),
            seed: self.seed,
        }
    }
}

impl CavePlan {
    fn request(&self) -> CaveRequest {
        CaveRequest {
            shape: (&self.shape).into(),
            fill: self.fill,
            roughness: self.roughness.unwrap_or(DEFAULT_ROUGHNESS),
            seed: self.seed,
        }
    }
}

impl PathPlan {
    fn request(&self) -> PathRequest {
        PathRequest {
            asset: self.asset.clone(),
            points: self.points.iter().map(|&[x, y]| (x, y)).collect(),
            width: self.width,
            closed: self.closed,
            sharp: self.sharp,
        }
    }
}

impl FloorPlan {
    fn request(&self) -> FloorRequest {
        FloorRequest {
            shape: (&self.shape).into(),
            tileset: self.tileset.clone(),
        }
    }
}

impl ScatterPlan {
    fn request(&self) -> ScatterRequest {
        ScatterRequest {
            assets: self.assets.clone(),
            area: (&self.shape).into(),
            count: self.count,
            seed: self.seed,
            along_walls: self.along_walls,
            color: self.color.clone(),
            outdoors: self.outdoors,
        }
    }
}

impl PatternPlan {
    fn request(&self) -> PatternRequest {
        PatternRequest {
            shape: (&self.shape).into(),
            asset: self.asset.clone(),
            color: self.color.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use dd_app::Workspace;
    use dd_setup::Config;

    use super::Plan;

    const STYLE: &str = r#"
[style]
floor = "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp"
wall = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp"
door = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp"
background = "res://textures/terrain/terrain_void.png"
ambient = "404858"
"#;

    fn workspace() -> Workspace {
        let catalog = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/catalog.toml");
        Config {
            catalog: Some(catalog),
            ..Config::default()
        }
        .build_workspace()
        .unwrap()
    }

    fn build(plan: &str) -> anyhow::Result<(Workspace, String)> {
        let mut workspace = workspace();
        let map = Plan::parse(&format!("{STYLE}{plan}"))?.build(&mut workspace)?;
        Ok((workspace, map))
    }

    /// Why `plan` is refused, with its context.
    fn failure(plan: &str) -> String {
        match build(plan) {
            Ok(_) => panic!("the plan was accepted"),
            Err(error) => format!("{error:#}"),
        }
    }

    #[test]
    fn builds_every_element_of_a_blank_map() {
        let (workspace, map) = build(
            r#"
[map]
width = 20
height = 12

[[room]]
shape = { type = "rectangle", x = 1, y = 1, width = 12, height = 8 }
pillars = [{ type = "rectangle", x = 3, y = 6, width = 1, height = 1 }]

[[wall]]
points = [[6, 1], [6, 9]]

[[door]]
on = "W0"
at = [6, 2.5]

[[door]]
on = "R0"
at = [9.5, 9]

[[prop]]
asset = "res://packs/LOREGD01/textures/objects/[LORE] Containers/Container_Large_Metal_Crate_Green.webp"
at = [3, 3]
rotation = 90

[[light]]
asset = "res://packs/LOREGD01/textures/lights/Effect_Caged_Light_Cast.webp"
at = [9, 4]
range = 4
color = "80beff"

[[pattern]]
asset = "res://packs/LOREGD01/textures/patterns/normal/Artifcial/Tileset_AncientGothic_Quarters_Sewer_Open.webp"
shape = { type = "polygon", points = [[8, 6], [10, 6], [10, 8]] }
"#,
        )
        .unwrap();
        assert_eq!(map, "map-1");
        let description = workspace.describe(&map).unwrap();
        for expected in [
            "1 pillar",
            "W0: (6, 1) - (6, 9)",
            "D0: on W0 at (6, 2.5)",
            "D1: on R0 at (9.5, 9)",
            "P0: Container_Large_Metal_Crate_Green at (3, 3), 90 degrees",
            "L0: Effect_Caged_Light_Cast at (9, 4), range 4 cells",
            "S0: Tileset_AncientGothic_Quarters_Sewer_Open",
        ] {
            assert!(description.contains(expected), "{expected}\n{description}");
        }
    }

    #[test]
    fn furnishes_a_generated_building() {
        let (workspace, map) = build(
            r#"
[generate]
generator = "ship_deck"
width = 30
height = 14
seed = 7

[[light]]
asset = "res://packs/LOREGD01/textures/lights/Effect_Caged_Light_Cast.webp"
at = [3, 7]

[[floor]]
shape = { type = "rectangle", x = 1, y = 6, width = 28, height = 2 }
tileset = "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp"

[[scatter]]
assets = ["res://packs/LOREGD01/textures/objects/[LORE] Clutter/Debris_Can_Empty.webp"]
shape = { type = "rectangle", x = 0, y = 0, width = 30, height = 14 }
count = 10
seed = 4

[[path]]
asset = "res://packs/LOREGD01/textures/paths/Artificial/Path_Conveyor_Belt_Large.webp"
points = [[2.5, 7], [12, 7], [12, 3.5]]
width = 1.5

[[scatter]]
assets = ["res://packs/LOREGD01/textures/objects/[LORE] Containers/Container_Large_Metal_Crate_Green.webp"]
shape = { type = "rectangle", x = 0, y = 0, width = 30, height = 14 }
count = 3
seed = 2
along_walls = true
"#,
        )
        .unwrap();
        let description = workspace.describe(&map).unwrap();
        assert!(
            description.contains("Background: terrain_void"),
            "{description}"
        );
        assert!(
            description.contains("Ambient light: ff404858"),
            "{description}"
        );
        assert!(description.contains("W0:"), "{description}");
        assert!(description.contains("L0:"), "{description}");
        assert!(
            description.contains("P9: Debris_Can_Empty"),
            "{description}"
        );
        assert!(
            description.contains("P12: Container_Large_Metal_Crate_Green"),
            "{description}"
        );
        assert!(
            description.contains("T0: Path_Conveyor_Belt_Large through (2.5, 7) - (12, 7)"),
            "{description}"
        );
        assert_eq!(workspace.validate(&map), Ok(vec![]));
    }

    #[test]
    fn digs_caves() {
        let (workspace, map) = build(
            r#"
[map]
width = 30
height = 20

[[cave_network]]
x = 1
y = 1
width = 18
height = 18
seed = 3

[[cave]]
shape = { type = "chamber", x = 24, y = 10, radius = 3 }
roughness = 0.4
seed = 1

[[cave]]
shape = { type = "tunnel", points = [[17, 10], [24, 10]], width = 2 }

[[cave_entrance]]
shape = { type = "chamber", x = 27, y = 10, radius = 1 }

[[terrain]]
asset = "res://textures/terrain/terrain_void.png"
shape = { type = "chamber", x = 24, y = 10, radius = 5 }
falloff = 2
roughness = 0.3
"#,
        )
        .unwrap();
        let description = workspace.describe(&map).unwrap();
        assert!(description.contains("Cave: about"), "{description}");
        assert!(description.contains("with entrances"), "{description}");
        assert!(
            description.contains("Terrain: terrain_void"),
            "{description}"
        );
    }

    #[test]
    fn needs_either_a_blank_map_or_a_generator() {
        let neither = failure("");
        assert!(neither.contains("[map] or [generate]"), "{neither}");
        let both = failure(
            "[map]\nwidth = 9\nheight = 9\n[generate]\ngenerator = \"bsp\"\nwidth = 9\nheight = 9\n",
        );
        assert!(both.contains("[map] or [generate]"), "{both}");
    }

    #[test]
    fn rejects_unknown_keys() {
        let error = failure("[map]\nwidth = 9\nheight = 9\ndepth = 3\n");
        assert!(error.contains("depth"), "{error}");
    }

    #[test]
    fn names_the_element_that_fails() {
        let text = failure(
            r#"
[map]
width = 20
height = 12

[[room]]
shape = { type = "rectangle", x = 1, y = 1, width = 8, height = 8 }

[[door]]
on = "R0"
at = [5, 5]
"#,
        );
        assert!(text.starts_with("[[door]] 1: the nearest wall"), "{text}");
    }

    #[test]
    fn rooms_without_floor_or_style_are_refused() {
        let mut workspace = workspace();
        let plan = Plan::parse(
            "[map]\nwidth = 9\nheight = 9\n[[room]]\nshape = { type = \"rectangle\", x = 1, y = 1, width = 3, height = 3 }\n",
        )
        .unwrap();
        let error = format!("{:#}", plan.build(&mut workspace).unwrap_err());
        assert!(error.contains("[[room]] 1: no floor"), "{error}");
    }
}
