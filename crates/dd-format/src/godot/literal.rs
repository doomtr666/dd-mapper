use super::CodecError;
use super::syntax::{join_constructor, split_constructor};

/// A Godot variant literal written as `TypeName( component, ... )`.
pub trait GodotLiteral: Sized {
    const TYPE_NAME: &'static str;

    /// Builds the value from its raw, trimmed components.
    ///
    /// # Errors
    /// Fails when a component is not a valid number or the count is wrong.
    fn from_components(components: &[&str]) -> Result<Self, CodecError>;

    fn to_components(&self) -> Vec<String>;

    /// Parses a literal such as `Vector2( 4480, 2560 )`.
    ///
    /// # Errors
    /// Fails when the text is not a well-formed `TYPE_NAME` literal.
    fn parse_literal(literal: &str) -> Result<Self, CodecError> {
        let components = split_constructor(literal, Self::TYPE_NAME)?;
        Self::from_components(&components)
    }

    /// Writes the literal exactly as Dungeondraft does.
    fn to_literal(&self) -> String {
        join_constructor(Self::TYPE_NAME, &self.to_components())
    }
}
