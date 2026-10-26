//! Acceptance tests: Dungeondraft maps imported into the domain.

mod common;

use dd_domain::{AssetPath, Cell, CellPosition, Map};
use dd_format::export::{ExportSettings, export_map};
use dd_format::import::{ImportedMap, import_map};
use dd_format::model::MapFile;

fn no_footprints(_: &str) -> Option<(f64, f64)> {
    None
}

fn import(fixture: &str) -> ImportedMap {
    let file = MapFile::parse(&common::read_fixture(fixture)).unwrap();
    import_map(&file, &no_footprints).unwrap()
}

fn door_positions(map: &Map) -> Vec<CellPosition> {
    map.doors()
        .filter_map(|(id, _)| map.door_position(id))
        .collect()
}

fn assert_close(actual: CellPosition, expected: (f64, f64)) {
    let near = (actual.x - expected.0).abs() < 1e-3 && (actual.y - expected.1).abs() < 1e-3;
    assert!(near, "{actual:?} != {expected:?}");
}

#[test]
fn imports_the_reference_room() {
    let imported = import("test.dungeondraft_map");
    assert_eq!(imported.warnings, Vec::<String>::new());
    let map = &imported.map;
    assert_eq!((map.grid().width(), map.grid().height()), (35, 20));
    let rooms: Vec<_> = map.rooms().collect();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].1.outline().double_area(), 2 * 17 * 11);
    let doors = door_positions(map);
    assert_eq!(doors.len(), 1);
    assert_close(doors[0], (13.0, 7.5));
    let floor = AssetPath::new(
        "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp",
    )
    .unwrap();
    let painted: Vec<(Cell, &AssetPath)> = map.floor().collect();
    assert_eq!(painted.len(), 17 * 11);
    assert!(painted.iter().all(|(_, tileset)| **tileset == floor));
}

#[test]
fn imports_pillars_props_lights_and_patterns() {
    let imported = import("test2.dungeondraft_map");
    let map = &imported.map;
    let rooms: Vec<_> = map.rooms().collect();
    assert_eq!(rooms.len(), 1, "{:?}", imported.warnings);
    assert_eq!(rooms[0].1.pillars().len(), 15, "{:?}", imported.warnings);
    assert_eq!(map.props().count(), 21);
    assert_eq!(map.lights().count(), 6);
    assert_eq!(map.patterns().count(), 4);
}

#[test]
fn imports_paths_without_their_repeated_last_point() {
    let map = import("cave_and_paths.dungeondraft_map").map;
    let tracks: Vec<_> = map.tracks().map(|(_, track)| track).collect();
    assert_eq!(tracks.len(), 2);
    let rope = tracks
        .iter()
        .find(|track| track.asset().as_str().contains("rope"))
        .unwrap();
    assert_eq!(
        rope.points(),
        [CellPosition::new(32.0, 21.0), CellPosition::new(34.0, 19.0)]
    );
    assert!((rope.width() - 0.1875).abs() < 1e-9);
    assert!(!rope.closed());
    let blend = tracks
        .iter()
        .find(|track| track.asset().as_str().contains("blender"))
        .unwrap();
    assert_eq!(blend.points().len(), 4);
}

#[test]
fn reexports_the_cave_of_the_sample_bit_for_bit() {
    let template =
        MapFile::parse(&common::read_fixture("cave_and_paths.dungeondraft_map")).unwrap();
    let imported = import_map(&template, &no_footprints).unwrap();
    assert!(!imported.map.cave().is_empty());
    let blank = MapFile::parse(&common::read_fixture("test2.dungeondraft_map")).unwrap();
    let exported = export_map(&imported.map, &blank, &ExportSettings::default()).unwrap();
    let cave = |file: &MapFile| {
        let cave = &file.world().levels["0"].cave;
        (cave.bitmap.clone(), cave.entrance_bitmap.clone())
    };
    assert_eq!(cave(&exported), cave(&template));
}

#[test]
fn reexports_painted_terrain_onto_another_template() {
    let painted = MapFile::parse(&common::read_fixture("test2.dungeondraft_map")).unwrap();
    let imported = import_map(&painted, &no_footprints).unwrap();
    let other = MapFile::parse(&common::read_fixture("test.dungeondraft_map")).unwrap();
    let exported = export_map(&imported.map, &other, &ExportSettings::default()).unwrap();
    let terrain = |file: &MapFile| {
        let terrain = &file.world().levels["0"].terrain;
        (
            terrain.splat.clone(),
            terrain.texture_1.clone(),
            terrain.texture_2.clone(),
        )
    };
    assert_eq!(terrain(&exported), terrain(&painted));
}

#[test]
fn imports_the_custom_colors_of_objects() {
    let map = import("cave_and_paths.dungeondraft_map").map;
    let tints: Vec<u32> = map.props().filter_map(|(_, prop)| prop.tint).collect();
    assert_eq!(tints.len(), 8);
    assert!(tints.contains(&0xff67_9865));
}

#[test]
fn imports_free_walls_as_partitions_holding_their_doors() {
    let imported = import("doors_and_free_walls.dungeondraft_map");
    let map = &imported.map;
    assert_eq!(map.rooms().count(), 1);
    assert_eq!(map.partitions().count(), 2);
    let doors = door_positions(map);
    assert_eq!(doors.len(), 7, "{:?}", imported.warnings);
    assert!(
        doors
            .iter()
            .any(|door| (door.x - 4616.65 / 256.0).abs() < 1e-2)
    );
}

#[test]
fn reports_what_cannot_be_imported() {
    let mut file = MapFile::parse(&common::read_fixture("test.dungeondraft_map")).unwrap();
    let ground = file.world().levels["0"].clone();
    file.world_mut().levels.insert("1".to_owned(), ground);
    let imported = import_map(&file, &no_footprints).unwrap();
    assert_eq!(
        imported.warnings,
        ["only level 0 is imported; levels 1 stay in the file untouched"]
    );
}

#[test]
fn reexports_the_reference_room_byte_for_byte() {
    let original = common::read_fixture("test.dungeondraft_map");
    let template = MapFile::parse(&original).unwrap();
    let imported = import_map(&template, &no_footprints).unwrap();
    let settings = ExportSettings {
        portal_radii: [(
            "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp".to_owned(),
            154.0,
        )]
        .into(),
        ..ExportSettings::default()
    };
    let exported = export_map(&imported.map, &template, &settings).unwrap();
    let expected = common::with_sequential_node_ids(template)
        .to_text()
        .unwrap();
    let actual = common::with_sequential_node_ids(exported)
        .to_text()
        .unwrap();
    common::assert_same_text("test.dungeondraft_map", &actual, &expected);
}

#[test]
#[ignore = "prints the warnings of every fixture, for review"]
fn print_warnings() {
    for fixture in common::FIXTURES {
        println!("{fixture}: {:?}", import(fixture).warnings);
    }
}
