//! Godot packed arrays: `PoolVector2Array`, `PoolIntArray` and `PoolByteArray`.

use super::number::{parse_byte, parse_int};
use super::{CodecError, GodotLiteral, Vector2};

/// A polygon or polyline: `PoolVector2Array( x1, y1, x2, y2, ... )`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoolVector2Array(pub Vec<Vector2>);

/// Tile cells and other integer grids: `PoolIntArray( -1, 12, ... )`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolIntArray(pub Vec<i32>);

/// Cave bitmaps and terrain splat maps: `PoolByteArray( 255, 0, ... )`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolByteArray(pub Vec<u8>);

impl GodotLiteral for PoolVector2Array {
    const TYPE_NAME: &'static str = "PoolVector2Array";

    fn from_components(components: &[&str]) -> Result<Self, CodecError> {
        if !components.len().is_multiple_of(2) {
            return Err(CodecError::OddComponentCount {
                type_name: Self::TYPE_NAME,
                found: components.len(),
            });
        }
        components
            .chunks_exact(2)
            .map(Vector2::from_components)
            .collect::<Result<_, _>>()
            .map(Self)
    }

    fn to_components(&self) -> Vec<String> {
        self.0.iter().flat_map(Vector2::to_components).collect()
    }
}

impl GodotLiteral for PoolIntArray {
    const TYPE_NAME: &'static str = "PoolIntArray";

    fn from_components(components: &[&str]) -> Result<Self, CodecError> {
        parse_each(components, parse_int).map(Self)
    }

    fn to_components(&self) -> Vec<String> {
        format_each(&self.0)
    }
}

impl GodotLiteral for PoolByteArray {
    const TYPE_NAME: &'static str = "PoolByteArray";

    fn from_components(components: &[&str]) -> Result<Self, CodecError> {
        parse_each(components, parse_byte).map(Self)
    }

    fn to_components(&self) -> Vec<String> {
        format_each(&self.0)
    }
}

fn parse_each<T>(
    components: &[&str],
    parse: fn(&str) -> Result<T, CodecError>,
) -> Result<Vec<T>, CodecError> {
    components
        .iter()
        .map(|component| parse(component))
        .collect()
}

fn format_each<T: ToString>(values: &[T]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::{PoolByteArray, PoolIntArray, PoolVector2Array};
    use crate::godot::{CodecError, GodotLiteral, NumberKind, Vector2};

    #[test]
    fn parses_points_in_pairs() {
        let wall = "PoolVector2Array( 7680, 4096, 3328, 4096, 3328, 1280 )";
        assert_eq!(
            PoolVector2Array::parse_literal(wall),
            Ok(PoolVector2Array(vec![
                Vector2::new(7680.0, 4096.0),
                Vector2::new(3328.0, 4096.0),
                Vector2::new(3328.0, 1280.0),
            ]))
        );
    }

    #[test]
    fn writes_points_flattened() {
        let points = PoolVector2Array(vec![
            Vector2::new(2816.0, 4096.0),
            Vector2::new(3072.5, 0.0),
        ]);
        assert_eq!(
            points.to_literal(),
            "PoolVector2Array( 2816, 4096, 3072.5, 0 )"
        );
    }

    #[test]
    fn rejects_an_unpaired_coordinate() {
        assert_eq!(
            PoolVector2Array::parse_literal("PoolVector2Array( 1, 2, 3 )"),
            Err(CodecError::OddComponentCount {
                type_name: "PoolVector2Array",
                found: 3,
            })
        );
    }

    #[test]
    fn round_trips_integers() {
        let literal = "PoolIntArray( -1, -1, 12, 2147483647 )";
        let cells = PoolIntArray::parse_literal(literal).unwrap();
        assert_eq!(cells, PoolIntArray(vec![-1, -1, 12, i32::MAX]));
        assert_eq!(cells.to_literal(), literal);
    }

    #[test]
    fn round_trips_bytes() {
        let literal = "PoolByteArray( 255, 0, 0, 0 )";
        let splat = PoolByteArray::parse_literal(literal).unwrap();
        assert_eq!(splat, PoolByteArray(vec![255, 0, 0, 0]));
        assert_eq!(splat.to_literal(), literal);
    }

    #[test]
    fn rejects_out_of_range_bytes() {
        assert_eq!(
            PoolByteArray::parse_literal("PoolByteArray( 0, 256 )"),
            Err(CodecError::InvalidNumber {
                expected: NumberKind::Byte,
                token: "256".to_owned(),
            })
        );
    }

    #[test]
    fn empty_arrays_round_trip() {
        assert_eq!(PoolIntArray::default().to_literal(), "PoolIntArray(  )");
        assert_eq!(
            PoolVector2Array::parse_literal("PoolVector2Array(  )"),
            Ok(PoolVector2Array::default())
        );
        assert_eq!(
            PoolByteArray::parse_literal("PoolByteArray(  )"),
            Ok(PoolByteArray::default())
        );
    }
}
