//! Helpers shared by the fixture-based acceptance tests.
#![allow(
    dead_code,
    reason = "each test binary compiles this module but uses only part of it"
)]

use std::collections::HashMap;
use std::path::Path;

use dd_format::model::{MapFile, NodeId};

/// Maps saved by Dungeondraft 1.2.0.1: the source of truth of the format.
pub const FIXTURES: [&str; 4] = [
    "test.dungeondraft_map",
    "test2.dungeondraft_map",
    "doors_and_free_walls.dungeondraft_map",
    "cave_and_paths.dungeondraft_map",
];

pub fn read_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// Points at the first differing line, instead of dumping megabytes of JSON.
pub fn assert_same_text(fixture: &str, actual: &str, expected: &str) {
    if actual == expected {
        return;
    }
    let first_difference = actual
        .lines()
        .zip(expected.lines())
        .enumerate()
        .find(|(_, (actual_line, expected_line))| actual_line != expected_line);
    match first_difference {
        Some((index, (actual_line, expected_line))) => panic!(
            "{fixture}: line {} differs\n  actual:   {actual_line:.200}\n  expected: {expected_line:.200}",
            index + 1
        ),
        None => panic!(
            "{fixture}: same lines but lengths differ ({} vs {} bytes)",
            actual.len(),
            expected.len()
        ),
    }
}

/// Renumbers node ids by order of appearance, so that maps built with
/// different id histories (Dungeondraft never reuses deleted ids) compare equal.
pub fn with_sequential_node_ids(mut map: MapFile) -> MapFile {
    let mut renumbered: HashMap<NodeId, NodeId> = HashMap::new();
    let mut next = 0;
    let mut renumber = |id: NodeId| {
        *renumbered.entry(id).or_insert_with(|| {
            next += 1;
            NodeId(next - 1)
        })
    };
    let world = map.world_mut();
    for level in world.levels.values_mut() {
        for wall in &mut level.walls {
            wall.node_id = renumber(wall.node_id);
            for portal in &mut wall.portals {
                portal.node_id = renumber(portal.node_id);
                portal.wall_id = renumber(portal.wall_id);
            }
        }
        let shape_walls: Vec<NodeId> = level.shapes.walls.iter().map(|id| renumber(*id)).collect();
        level.shapes.walls = shape_walls;
    }
    world.next_node_id = NodeId(next);
    map
}
