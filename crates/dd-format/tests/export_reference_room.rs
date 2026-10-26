//! Acceptance test: the room of `test.dungeondraft_map`, described in the
//! domain, is exported exactly as Dungeondraft saved it.

mod common;

use std::collections::HashMap;

use dd_domain::{
    AssetPath, CellPoint, Door, GridSize, Map, Polygon, Room, WallLoop, WallPosition, WallRef,
};
use dd_format::export::{ExportSettings, export_map};
use dd_format::model::MapFile;

const FLOOR: &str = "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp";
const WALL: &str = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Reinforced.webp";
const DOOR: &str = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp";
const DOOR_RADIUS: f64 = 154.0;

fn asset(path: &str) -> AssetPath {
    AssetPath::new(path).expect("valid asset path")
}

fn reference_room_map() -> Map {
    let mut map = Map::new(GridSize::new(35, 20).unwrap());
    let outline = Polygon::rectangle(CellPoint::new(13, 5), 17, 11).unwrap();
    let room = map
        .add_room(Room::new(outline, asset(WALL)), &asset(FLOOR))
        .unwrap()
        .id;
    let position = WallPosition {
        wall: WallRef::Room {
            room,
            wall: WallLoop::Outline,
        },
        segment: 1,
        t: 0.772_727,
    };
    let door = Door {
        position,
        asset: asset(DOOR),
    };
    map.add_door(door).unwrap();
    map
}

fn settings() -> ExportSettings {
    ExportSettings {
        portal_radii: HashMap::from([(DOOR.to_owned(), DOOR_RADIUS)]),
        ..ExportSettings::default()
    }
}

#[test]
fn regenerates_the_reference_room_byte_for_byte() {
    let original = common::read_fixture("test.dungeondraft_map");
    let template = MapFile::parse(&original).unwrap();

    let exported = export_map(&reference_room_map(), &template, &settings()).unwrap();

    let expected = common::with_sequential_node_ids(template)
        .to_text()
        .unwrap();
    let actual = common::with_sequential_node_ids(exported)
        .to_text()
        .unwrap();
    common::assert_same_text("test.dungeondraft_map", &actual, &expected);
}
