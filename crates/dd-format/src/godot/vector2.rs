use super::number::{format_real, parse_real};
use super::{CodecError, GodotLiteral};

/// A 2D vector in pixels, as stored by Godot.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl GodotLiteral for Vector2 {
    const TYPE_NAME: &'static str = "Vector2";

    fn from_components(components: &[&str]) -> Result<Self, CodecError> {
        let [x, y] = components else {
            return Err(CodecError::WrongComponentCount {
                type_name: Self::TYPE_NAME,
                expected: 2,
                found: components.len(),
            });
        };
        Ok(Self::new(parse_real(x)?, parse_real(y)?))
    }

    fn to_components(&self) -> Vec<String> {
        vec![format_real(self.x), format_real(self.y)]
    }
}

#[cfg(test)]
mod tests {
    use super::Vector2;
    use crate::godot::{CodecError, GodotLiteral, NumberKind};

    #[test]
    fn parses_integer_and_real_components() {
        assert_eq!(
            Vector2::parse_literal("Vector2( 4480, 2560 )"),
            Ok(Vector2::new(4480.0, 2560.0))
        );
        assert_eq!(
            Vector2::parse_literal("Vector2( 5483.15, -6326.94 )"),
            Ok(Vector2::new(5483.15, -6326.94))
        );
    }

    #[test]
    fn writes_like_dungeondraft() {
        assert_eq!(Vector2::new(0.0, -1.0).to_literal(), "Vector2( 0, -1 )");
        let light = Vector2::new(f64::from(5483.15_f32), f64::from(6326.94_f32));
        assert_eq!(light.to_literal(), "Vector2( 5483.15, 6326.94 )");
    }

    #[test]
    fn requires_exactly_two_components() {
        assert_eq!(
            Vector2::parse_literal("Vector2( 1, 2, 3 )"),
            Err(CodecError::WrongComponentCount {
                type_name: "Vector2",
                expected: 2,
                found: 3,
            })
        );
    }

    #[test]
    fn reports_the_invalid_component() {
        assert_eq!(
            Vector2::parse_literal("Vector2( 1, y )"),
            Err(CodecError::InvalidNumber {
                expected: NumberKind::Real,
                token: "y".to_owned(),
            })
        );
    }
}
