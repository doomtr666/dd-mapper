use dd_format::godot::{
    GodotLiteral, PoolByteArray, PoolIntArray, PoolVector2Array, Vector2, format_real,
};
use proptest::prelude::*;

const SIGNIFICANT_DIGITS_TOLERANCE: f64 = 5e-6;

fn finite_real() -> impl Strategy<Value = f64> {
    any::<f64>().prop_filter("Godot literals hold finite reals", |value| {
        value.is_finite()
    })
}

fn map_coordinate() -> impl Strategy<Value = f64> {
    -100_000.0..100_000.0_f64
}

fn parse_formatted_real(value: f64) -> f64 {
    Vector2::parse_literal(&Vector2::new(value, 0.0).to_literal())
        .expect("formatted reals are parseable")
        .x
}

proptest! {
    #[test]
    fn real_survives_with_six_significant_digits(value in finite_real()) {
        let reparsed = parse_formatted_real(value);
        prop_assert!((reparsed - value).abs() <= value.abs() * SIGNIFICANT_DIGITS_TOLERANCE);
    }

    #[test]
    fn real_formatting_is_idempotent(value in finite_real()) {
        let formatted = format_real(value);
        prop_assert_eq!(format_real(parse_formatted_real(value)), formatted);
    }

    #[test]
    fn int_array_round_trips_exactly(values in prop::collection::vec(any::<i32>(), 0..64)) {
        let cells = PoolIntArray(values);
        prop_assert_eq!(PoolIntArray::parse_literal(&cells.to_literal()), Ok(cells));
    }

    #[test]
    fn byte_array_round_trips_exactly(values in prop::collection::vec(any::<u8>(), 0..64)) {
        let bitmap = PoolByteArray(values);
        prop_assert_eq!(PoolByteArray::parse_literal(&bitmap.to_literal()), Ok(bitmap));
    }

    #[test]
    fn vector2_array_literal_is_stable(
        coordinates in prop::collection::vec((map_coordinate(), map_coordinate()), 0..32)
    ) {
        let points = PoolVector2Array(
            coordinates.into_iter().map(|(x, y)| Vector2::new(x, y)).collect(),
        );
        let literal = points.to_literal();
        let reparsed = PoolVector2Array::parse_literal(&literal).expect("written literals are parseable");
        prop_assert_eq!(reparsed.to_literal(), literal);
    }
}
