//! Writes a map exercising pillars and oblique walls, to check in Dungeondraft
//! that exported floor shapes are accepted.
//!
//! Usage: `cargo run -p dd-format --example export_experiment -- <template> <output>`
//! Also writes a schematic preview next to the output, as `<output>.png`.

use std::collections::HashMap;
use std::error::Error;

use dd_domain::{
    AssetPath, CellPoint, CellPosition, Door, Footprint, GridSize, Map, Polygon, Prop, QuarterTurn,
    Room, WallLoop, WallPosition, WallRef,
};
use dd_format::export::{ExportSettings, export_map};
use dd_format::model::MapFile;

const FLOOR: &str = "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp";
const WALL: &str = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp";
const DOOR: &str = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp";
const DOOR_RADIUS: f64 = 154.0;
const CRATE: &str = "res://packs/LOREGD01/textures/objects/[LORE] Containers/Container_Large_Metal_Crate_Green.webp";
const CORRIDOR_FLOOR: &str =
    "res://packs/LOREGD01/textures/tilesets/simple/Tileset_Tight_Panels.webp";

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    let (Some(template_path), Some(output_path)) = (arguments.next(), arguments.next()) else {
        return Err("usage: export_experiment <template> <output>".into());
    };
    let template = MapFile::parse(&std::fs::read_to_string(template_path)?)?;
    let settings = ExportSettings {
        portal_radii: HashMap::from([(DOOR.to_owned(), DOOR_RADIUS)]),
        ..ExportSettings::default()
    };
    let map = experiment_map()?;
    let exported = export_map(&map, &template, &settings)?;
    std::fs::write(&output_path, exported.to_text()?)?;
    let preview = dd_render::render(&map, &dd_render::PreviewOptions::default())?;
    std::fs::write(format!("{output_path}.png"), preview.to_png()?)?;
    println!("written {output_path} and its preview");
    Ok(())
}

fn experiment_map() -> Result<Map, Box<dyn Error>> {
    let mut map = Map::new(GridSize::new(30, 20)?);
    let (floor, wall) = (AssetPath::new(FLOOR)?, AssetPath::new(WALL)?);

    let mut hall = Room::new(
        Polygon::rectangle(CellPoint::new(2, 2), 14, 10)?,
        wall.clone(),
    );
    hall.add_pillar(Polygon::rectangle(CellPoint::new(5, 5), 1, 1)?)?;
    hall.add_pillar(Polygon::rectangle(CellPoint::new(10, 6), 2, 2)?)?;
    let hall = map.add_room(hall, &floor)?.id;
    add_door(&mut map, hall, 0)?;

    let slanted = [(20, 2), (28, 2), (28, 10), (24, 14), (20, 14)]
        .into_iter()
        .map(|(x, y)| CellPoint::new(x, y))
        .collect();
    let slanted = map
        .add_room(Room::new(Polygon::new(slanted)?, wall.clone()), &floor)?
        .id;
    add_door(&mut map, slanted, 2)?;

    add_merged_wing(&mut map, &floor, &wall)?;
    add_crates(&mut map)?;
    Ok(map)
}

/// Two large Grimdorx crates (452 x 390 px), the second one turned a quarter.
fn add_crates(map: &mut Map) -> Result<(), Box<dyn Error>> {
    let footprint = Footprint::new(452.0 / 256.0, 390.0 / 256.0)?;
    for (x, rotation) in [(12.5, QuarterTurn::Deg0), (14.5, QuarterTurn::Deg90)] {
        map.add_prop(Prop {
            asset: AssetPath::new(CRATE)?,
            center: CellPosition::new(x, 3.5),
            rotation,
            footprint,
        })?;
    }
    Ok(())
}

/// Two rooms joined by a corridor with another floor: all three merge,
/// one door survives on a remaining wall, the other one is removed.
fn add_merged_wing(
    map: &mut Map,
    floor: &AssetPath,
    wall: &AssetPath,
) -> Result<(), Box<dyn Error>> {
    let left = Room::new(
        Polygon::rectangle(CellPoint::new(2, 14), 5, 5)?,
        wall.clone(),
    );
    let left = map.add_room(left, floor)?.id;
    add_door_at(map, left, 3, 0.2)?;
    add_door_at(map, left, 3, 0.5)?;
    let right = Room::new(
        Polygon::rectangle(CellPoint::new(11, 14), 5, 5)?,
        wall.clone(),
    );
    map.add_room(right, floor)?;
    let corridor = Room::new(
        Polygon::rectangle(CellPoint::new(7, 16), 4, 1)?,
        wall.clone(),
    );
    let added = map.add_room(corridor, &AssetPath::new(CORRIDOR_FLOOR)?)?;
    println!(
        "corridor merged into {:?}, absorbed {:?}, removed doors {:?}",
        added.id, added.absorbed, added.removed_doors
    );
    Ok(())
}

fn add_door(map: &mut Map, room: dd_domain::RoomId, segment: usize) -> Result<(), Box<dyn Error>> {
    add_door_at(map, room, segment, 0.5)
}

fn add_door_at(
    map: &mut Map,
    room: dd_domain::RoomId,
    segment: usize,
    t: f64,
) -> Result<(), Box<dyn Error>> {
    let position = WallPosition {
        wall: WallRef::Room {
            room,
            wall: WallLoop::Outline,
        },
        segment,
        t,
    };
    let asset = AssetPath::new(DOOR)?;
    map.add_door(Door { position, asset })?;
    Ok(())
}
