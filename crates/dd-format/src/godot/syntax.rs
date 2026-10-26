//! Constructor syntax shared by all literals: `TypeName( a, b, c )`.

use super::CodecError;

/// Returns the trimmed, comma-separated components of `type_name( ... )`.
pub fn split_constructor<'a>(
    literal: &'a str,
    type_name: &'static str,
) -> Result<Vec<&'a str>, CodecError> {
    let arguments =
        constructor_arguments(literal, type_name).ok_or_else(|| CodecError::UnexpectedLiteral {
            expected: type_name,
            found: literal.to_owned(),
        })?;
    Ok(split_arguments(arguments))
}

/// Writes `type_name( a, b )` with Godot's spacing; empty gives `type_name(  )`.
pub fn join_constructor(type_name: &str, components: &[String]) -> String {
    format!("{type_name}( {} )", components.join(", "))
}

fn constructor_arguments<'a>(literal: &'a str, type_name: &str) -> Option<&'a str> {
    literal
        .trim()
        .strip_prefix(type_name)?
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')
}

fn split_arguments(arguments: &str) -> Vec<&str> {
    let arguments = arguments.trim();
    if arguments.is_empty() {
        return Vec::new();
    }
    arguments.split(',').map(str::trim).collect()
}

#[cfg(test)]
mod tests {
    use super::{join_constructor, split_constructor};
    use crate::godot::CodecError;

    #[test]
    fn splits_components_written_by_godot() {
        assert_eq!(
            split_constructor("Vector2( 4480, 2560 )", "Vector2"),
            Ok(vec!["4480", "2560"])
        );
    }

    #[test]
    fn tolerates_missing_or_extra_whitespace() {
        assert_eq!(
            split_constructor(" Vector2 (1,2 )", "Vector2"),
            Ok(vec!["1", "2"])
        );
    }

    #[test]
    fn empty_constructor_has_no_components() {
        assert_eq!(
            split_constructor("PoolIntArray(  )", "PoolIntArray"),
            Ok(vec![])
        );
        assert_eq!(
            split_constructor("PoolIntArray()", "PoolIntArray"),
            Ok(vec![])
        );
    }

    #[test]
    fn rejects_another_type_name() {
        assert_eq!(
            split_constructor("PoolVector2Array( 1, 2 )", "Vector2"),
            Err(unexpected("Vector2", "PoolVector2Array( 1, 2 )"))
        );
        assert_eq!(
            split_constructor("Vector2i( 1, 2 )", "Vector2"),
            Err(unexpected("Vector2", "Vector2i( 1, 2 )"))
        );
    }

    #[test]
    fn rejects_unbalanced_or_trailing_text() {
        assert_eq!(
            split_constructor("Vector2( 1, 2", "Vector2"),
            Err(unexpected("Vector2", "Vector2( 1, 2"))
        );
        assert_eq!(
            split_constructor("Vector2( 1, 2 ) x", "Vector2"),
            Err(unexpected("Vector2", "Vector2( 1, 2 ) x"))
        );
    }

    #[test]
    fn joins_components_with_godot_spacing() {
        let components = vec!["4480".to_owned(), "2560".to_owned()];
        assert_eq!(
            join_constructor("Vector2", &components),
            "Vector2( 4480, 2560 )"
        );
    }

    #[test]
    fn joins_empty_constructor_with_two_spaces() {
        assert_eq!(join_constructor("PoolIntArray", &[]), "PoolIntArray(  )");
    }

    fn unexpected(expected: &'static str, found: &str) -> CodecError {
        CodecError::UnexpectedLiteral {
            expected,
            found: found.to_owned(),
        }
    }
}
