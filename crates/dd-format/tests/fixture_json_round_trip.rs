//! Acceptance test: the Godot JSON writer reproduces Dungeondraft files byte for byte.

mod common;

use dd_format::json::to_godot_json;
use serde_json::Value;

#[test]
fn fixtures_are_rewritten_byte_for_byte() {
    for fixture in common::FIXTURES {
        let original = common::read_fixture(fixture);
        let document: Value = serde_json::from_str(&original).expect("fixtures are valid JSON");
        let rewritten = to_godot_json(&document) + "\n";
        common::assert_same_text(fixture, &rewritten, &original);
    }
}
