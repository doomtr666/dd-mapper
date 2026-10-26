//! Acceptance tests of the typed map model against Dungeondraft fixtures.

mod common;

use dd_format::Preserved;
use dd_format::godot::Vector2;
use dd_format::model::{Level, MapFile, NodeId};
use serde::Serialize;

fn parse_fixture(fixture: &str) -> MapFile {
    MapFile::parse(&common::read_fixture(fixture))
        .unwrap_or_else(|error| panic!("{fixture}: {error}"))
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

fn ground_level(map: &MapFile) -> &Level {
    &map.world().levels["0"]
}

#[test]
fn fixtures_survive_a_typed_round_trip_byte_for_byte() {
    for fixture in common::FIXTURES {
        let original = common::read_fixture(fixture);
        let rewritten = parse_fixture(fixture).to_text().expect("serializable map");
        common::assert_same_text(fixture, &rewritten, &original);
    }
}

/// A record rebuilt without its original JSON must still be written exactly
/// as Dungeondraft wrote it: the type models every field, in Dungeondraft's order.
fn assert_rebuilt_from_scratch<T: Serialize + Clone>(fixture: &str, record: &Preserved<T>) {
    let from_scratch = Preserved::new(T::clone(record));
    let expected = serde_json::to_string(record).expect("serializable record");
    let actual = serde_json::to_string(&from_scratch).expect("serializable record");
    assert_eq!(actual, expected, "{fixture}: record not fully modelled");
}

#[test]
fn typed_records_model_every_field_in_dungeondraft_order() {
    for fixture in common::FIXTURES {
        let map = parse_fixture(fixture);
        let level = ground_level(&map);
        assert_rebuilt_from_scratch(fixture, &level.shapes);
        assert_rebuilt_from_scratch(fixture, &level.tiles);
        assert_rebuilt_from_scratch(fixture, &level.cave);
        assert_rebuilt_from_scratch(fixture, &level.terrain);
        for wall in &level.walls {
            assert_rebuilt_from_scratch(fixture, wall);
            for portal in &wall.portals {
                assert_rebuilt_from_scratch(fixture, portal);
            }
        }
        for object in &level.objects {
            assert_rebuilt_from_scratch(fixture, object);
        }
        for light in &level.lights {
            assert_rebuilt_from_scratch(fixture, light);
        }
        for pattern in &level.patterns {
            assert_rebuilt_from_scratch(fixture, pattern);
        }
    }
}

#[test]
fn decodes_the_door_of_the_first_fixture() {
    let map = parse_fixture("test.dungeondraft_map");
    assert_eq!((map.world().width, map.world().height), (35, 20));
    assert_eq!(map.world().next_node_id, NodeId(4));
    let wall = &ground_level(&map).walls[0];
    assert!(wall.closed_loop);
    let door = &wall.portals[0];
    assert_eq!(door.position, Vector2::new(3328.0, 1920.0));
    assert_eq!(door.direction, Vector2::new(0.0, -1.0));
    assert_eq!(door.point_index, 1);
    assert_eq!(door.wall_id, wall.node_id);
    assert_close(door.wall_distance, 1.772_727);
    assert_close(door.radius, 154.0);
}

#[test]
fn decodes_layers_of_the_second_fixture() {
    let map = parse_fixture("test2.dungeondraft_map");
    let level = ground_level(&map);
    assert_eq!(level.shapes.walls.first(), Some(&NodeId(0x10)));
    assert_eq!(level.shapes.polygons.len(), level.walls.len());
    assert_eq!(level.tiles.cells.0.len(), 48 * 27);
    assert_eq!(level.tiles.lookup.len(), 25);
    assert_eq!(level.cave.bitmap.0.len(), 2706);
    assert_eq!(level.terrain.splat.0.len(), 48 * 4 * 27 * 4 * 4);
    assert_eq!(level.objects.len(), 21);
    assert_close(level.lights[0].range, 0.9);
    assert_eq!(level.patterns.len(), 4);
}

#[test]
fn free_walls_have_type_one() {
    let map = parse_fixture("doors_and_free_walls.dungeondraft_map");
    let wall_types: Vec<i32> = ground_level(&map)
        .walls
        .iter()
        .map(|wall| wall.wall_type)
        .collect();
    assert_eq!(wall_types, [1, 1, 0]);
}
