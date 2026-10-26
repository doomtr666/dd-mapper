//! Map plans: a TOML file describing a map, built through the use cases.
//!
//! ```toml
//! [style]                      # defaults for rooms, walls and doors
//! floor = "res://packs/.../Tileset_RT_Punched_Floor.webp"
//! wall = "res://packs/.../Wall_Underhive_Industrial.webp"
//! door = "res://packs/.../Door_Underhive_Industrial.webp"
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
//!
//! [[light]]
//! asset = "res://packs/.../Effect_Caged_Light_Cast.webp"
//! at = [4, 4]
//! range = 5
//! color = "80beff"
//!
//! [[pattern]]
//! asset = "res://packs/.../Tileset_AncientGothic_Quarters_Sewer_Open.webp"
//! shape = { type = "polygon", points = [[8, 2], [10, 2], [10, 4]] }
//! ```

use anyhow::Context as _;
use dd_app::{
    DEFAULT_MIN_ROOM, DoorRequest, Generator, LayoutRequest, LightRequest, PatternRequest,
    PropRequest, RoomRequest, Shape, WallRequest, Workspace,
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
}

/// Assets used where an element names none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Style {
    floor: Option<String>,
    wall: Option<String>,
    door: Option<String>,
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
    /// Degrees: 0, 90, 180 or 270.
    #[serde(default)]
    rotation: i32,
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
        self.add_structure(workspace, &map)?;
        self.furnish(workspace, &map)?;
        Ok(map)
    }

    /// Rooms, walls and doors.
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
        })
    }

    /// Floor patterns, props and lights.
    fn furnish(&self, workspace: &mut Workspace, map: &str) -> anyhow::Result<()> {
        each(&self.pattern, "pattern", |pattern| {
            workspace.add_pattern(map, &pattern.request())?;
            Ok(())
        })?;
        each(&self.prop, "prop", |prop| {
            workspace.place_prop(map, &prop.request())?;
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
"#,
        )
        .unwrap();
        let description = workspace.describe(&map).unwrap();
        assert!(description.contains("W0:"), "{description}");
        assert!(description.contains("L0:"), "{description}");
        assert_eq!(workspace.validate(&map), Ok(vec![]));
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
