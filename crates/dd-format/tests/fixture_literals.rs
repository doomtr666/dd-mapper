//! Acceptance test: every Godot literal Dungeondraft wrote in the fixtures
//! must be decoded and re-encoded byte for byte.

mod common;

use dd_format::godot::{GodotLiteral, PoolByteArray, PoolIntArray, PoolVector2Array, Vector2};
use serde_json::Value;

type Reencoder = fn(&str) -> String;

const SUPPORTED_LITERALS: [(&str, Reencoder); 4] = [
    ("Vector2", reencode::<Vector2>),
    ("PoolVector2Array", reencode::<PoolVector2Array>),
    ("PoolIntArray", reencode::<PoolIntArray>),
    ("PoolByteArray", reencode::<PoolByteArray>),
];

fn reencode<T: GodotLiteral>(literal: &str) -> String {
    T::parse_literal(literal)
        .unwrap_or_else(|error| panic!("cannot decode `{literal}`: {error}"))
        .to_literal()
}

fn load_fixture(name: &str) -> Value {
    serde_json::from_str(&common::read_fixture(name)).expect("fixtures are valid JSON")
}

fn collect_strings<'a>(value: &'a Value, strings: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => strings.push(text),
        Value::Array(items) => items.iter().for_each(|item| collect_strings(item, strings)),
        Value::Object(fields) => fields
            .values()
            .for_each(|field| collect_strings(field, strings)),
        _ => {}
    }
}

/// Returns `Name` for strings shaped like `Name( ... )`.
fn literal_type_name(text: &str) -> Option<&str> {
    let (name, _) = text.split_once('(')?;
    let starts_uppercase = name.starts_with(|first: char| first.is_ascii_uppercase());
    let is_identifier = name
        .chars()
        .all(|character| character.is_ascii_alphanumeric());
    (starts_uppercase && is_identifier && text.ends_with(')')).then_some(name)
}

fn assert_all_literals_round_trip(fixture: &str) -> usize {
    let document = load_fixture(fixture);
    let mut strings = Vec::new();
    collect_strings(&document, &mut strings);
    let mut checked = 0;
    for text in strings {
        let Some(type_name) = literal_type_name(text) else {
            continue;
        };
        let (_, reencoder) = SUPPORTED_LITERALS
            .iter()
            .find(|(supported, _)| *supported == type_name)
            .unwrap_or_else(|| panic!("{fixture}: unsupported Godot literal `{type_name}`"));
        assert_eq!(
            reencoder(text),
            text,
            "{fixture}: literal changed on round trip"
        );
        checked += 1;
    }
    checked
}

#[test]
fn fixture_literals_round_trip_byte_for_byte() {
    for fixture in common::FIXTURES {
        let checked = assert_all_literals_round_trip(fixture);
        assert!(checked >= 10, "{fixture}: only {checked} literals found");
    }
}
