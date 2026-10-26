use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::hex::parse_hex_digits;

/// Identifier of a map node (wall, door, object, light, ...).
///
/// Written as lowercase hexadecimal text (`"3b"`) everywhere,
/// except in `shapes.walls` where it is a decimal number: see [`decimal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl Serialize for NodeId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:x}", self.0))
    }
}

impl<'de> Deserialize<'de> for NodeId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        parse_hex_digits(&text)
            .map(Self)
            .ok_or_else(|| D::Error::custom(format!("invalid hexadecimal node id `{text}`")))
    }
}

/// Serde adapter for node id lists written as decimal numbers (`shapes.walls`).
pub mod decimal {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::NodeId;

    /// # Errors
    /// Fails when the serializer fails.
    pub fn serialize<S: Serializer>(ids: &[NodeId], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(ids.iter().map(|id| id.0))
    }

    /// # Errors
    /// Fails when an id is not an unsigned 32-bit number.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<NodeId>, D::Error> {
        let ids = Vec::<u32>::deserialize(deserializer)?;
        Ok(ids.into_iter().map(NodeId).collect())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::NodeId;

    #[test]
    fn reads_and_writes_lowercase_hexadecimal() {
        assert_eq!(
            serde_json::from_value::<NodeId>(json!("3b")).unwrap(),
            NodeId(0x3b)
        );
        assert_eq!(serde_json::to_value(NodeId(0x2a)).unwrap(), json!("2a"));
        assert_eq!(serde_json::to_value(NodeId(0)).unwrap(), json!("0"));
    }

    #[test]
    fn rejects_text_that_is_not_hexadecimal() {
        let error = serde_json::from_value::<NodeId>(json!("xyz")).unwrap_err();
        assert!(error.to_string().contains("hexadecimal node id"), "{error}");
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Shapes {
        #[serde(with = "super::decimal")]
        walls: Vec<NodeId>,
    }

    #[test]
    fn shape_walls_are_decimal_numbers() {
        let shapes = Shapes {
            walls: vec![NodeId(0x10), NodeId(0x2a)],
        };
        let encoded = json!({ "walls": [16, 42] });
        assert_eq!(serde_json::to_value(&shapes).unwrap(), encoded);
        assert_eq!(serde_json::from_value::<Shapes>(encoded).unwrap(), shapes);
    }
}
