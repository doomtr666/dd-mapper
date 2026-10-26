use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::hex::parse_hex_digits;

const ARGB_DIGITS: usize = 8;

/// A color as Dungeondraft stores it: ARGB, written `"ffeccd8b"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color(pub u32);

impl Color {
    pub const WHITE: Self = Self(0xffff_ffff);
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:0ARGB_DIGITS$x}", self.0))
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let argb = (text.len() == ARGB_DIGITS)
            .then(|| parse_hex_digits(&text))
            .flatten();
        argb.map(Self).ok_or_else(|| {
            D::Error::custom(format!(
                "invalid ARGB color `{text}`, expected 8 hex digits"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Color;

    #[test]
    fn reads_and_writes_eight_lowercase_hex_digits() {
        assert_eq!(
            serde_json::from_value::<Color>(json!("ffeccd8b")).unwrap(),
            Color(0xffec_cd8b)
        );
        assert_eq!(
            serde_json::to_value(Color(0x7f00_0000)).unwrap(),
            json!("7f000000")
        );
        assert_eq!(
            serde_json::to_value(Color::WHITE).unwrap(),
            json!("ffffffff")
        );
    }

    #[test]
    fn rejects_colors_without_alpha_or_with_bad_digits() {
        for text in ["eccd8b", "ffeccd8g", "+fffffff"] {
            let error = serde_json::from_value::<Color>(json!(text)).unwrap_err();
            assert!(error.to_string().contains("ARGB color"), "{text}: {error}");
        }
    }
}
