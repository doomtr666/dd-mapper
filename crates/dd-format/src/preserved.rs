//! Typed views over Dungeondraft JSON objects that never lose unknown fields.

use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

/// A typed value decoded from a JSON object, remembering the original object.
///
/// When written back, the typed fields overwrite their original keys in place
/// and every key the type does not model is kept, in its original position.
#[derive(Debug, Clone, PartialEq)]
pub struct Preserved<T> {
    value: T,
    original: Map<String, Value>,
}

impl<T> Preserved<T> {
    /// Wraps a value created from scratch: its keys follow the type's field order.
    pub fn new(value: T) -> Self {
        Self {
            value,
            original: Map::new(),
        }
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: Serialize> Preserved<T> {
    fn merged_object(&self) -> Result<Map<String, Value>, serde_json::Error> {
        let Value::Object(typed_fields) = serde_json::to_value(&self.value)? else {
            return Err(serde::ser::Error::custom(
                "a preserved value must serialize to a JSON object",
            ));
        };
        let mut merged = self.original.clone();
        merged.extend(typed_fields);
        Ok(merged)
    }
}

impl<T> Deref for Preserved<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Preserved<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: Serialize> Serialize for Preserved<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.merged_object()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Preserved<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let original = Value::Object(Map::deserialize(deserializer)?);
        let value = T::deserialize(&original).map_err(serde::de::Error::custom)?;
        let Value::Object(original) = original else {
            unreachable!("built as a JSON object just above")
        };
        Ok(Self { value, original })
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::Preserved;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Door {
        position: i32,
        closed: bool,
    }

    fn decode(value: Value) -> Preserved<Door> {
        serde_json::from_value(value).expect("valid door")
    }

    fn encode(door: &Preserved<Door>) -> String {
        serde_json::to_string(door).expect("serializable door")
    }

    #[test]
    fn exposes_the_typed_fields() {
        let door = decode(json!({ "position": 3, "closed": true }));
        assert_eq!(door.position, 3);
        assert!(door.closed);
    }

    #[test]
    fn keeps_unknown_fields_in_their_original_position() {
        let original = r#"{"texture":"door.webp","position":3,"radius":154,"closed":true}"#;
        let door = decode(serde_json::from_str(original).unwrap());
        assert_eq!(encode(&door), original);
    }

    #[test]
    fn overwrites_modified_fields_in_place() {
        let mut door = decode(json!({ "closed": true, "texture": "door.webp", "position": 3 }));
        door.position = 7;
        assert_eq!(
            encode(&door),
            r#"{"closed":true,"texture":"door.webp","position":7}"#
        );
    }

    #[test]
    fn new_values_follow_the_field_order() {
        let door = Preserved::new(Door {
            position: 1,
            closed: false,
        });
        assert_eq!(encode(&door), r#"{"position":1,"closed":false}"#);
    }

    #[test]
    fn rejects_non_objects() {
        let error = serde_json::from_value::<Preserved<Door>>(json!([1, 2])).unwrap_err();
        assert!(error.to_string().contains("expected a map"), "{error}");
    }
}
