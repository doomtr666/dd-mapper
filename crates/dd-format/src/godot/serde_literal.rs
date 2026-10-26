//! Serde support for every [`GodotLiteral`]: stored as its JSON string form.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{GodotLiteral, PoolByteArray, PoolIntArray, PoolVector2Array, Vector2};

fn serialize_literal<T: GodotLiteral, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_literal())
}

fn deserialize_literal<'de, T: GodotLiteral, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    let literal = String::deserialize(deserializer)?;
    T::parse_literal(&literal).map_err(D::Error::custom)
}

macro_rules! serde_as_literal {
    ($($literal:ty),+) => {$(
        impl Serialize for $literal {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serialize_literal(self, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $literal {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserialize_literal(deserializer)
            }
        }
    )+};
}

serde_as_literal!(Vector2, PoolVector2Array, PoolIntArray, PoolByteArray);

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::godot::{PoolIntArray, PoolVector2Array, Vector2};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Portal {
        position: Vector2,
        cells: PoolIntArray,
    }

    #[test]
    fn literals_are_json_strings() {
        let portal = Portal {
            position: Vector2::new(3328.0, 1920.0),
            cells: PoolIntArray(vec![-1, 12]),
        };
        let encoded = json!({
            "position": "Vector2( 3328, 1920 )",
            "cells": "PoolIntArray( -1, 12 )"
        });
        assert_eq!(serde_json::to_value(&portal).unwrap(), encoded);
        assert_eq!(serde_json::from_value::<Portal>(encoded).unwrap(), portal);
    }

    #[test]
    fn literal_lists_are_lists_of_strings() {
        let polygons = vec![PoolVector2Array(vec![Vector2::new(1.0, 2.0)])];
        let encoded = json!(["PoolVector2Array( 1, 2 )"]);
        assert_eq!(serde_json::to_value(&polygons).unwrap(), encoded);
        assert_eq!(
            serde_json::from_value::<Vec<PoolVector2Array>>(encoded).unwrap(),
            polygons
        );
    }

    #[test]
    fn invalid_literals_report_the_codec_error() {
        let encoded = json!({ "position": "Vector2( 1 )", "cells": "PoolIntArray(  )" });
        let error = serde_json::from_value::<Portal>(encoded).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("`Vector2` expects exactly 2 components"),
            "{error}"
        );
    }
}
