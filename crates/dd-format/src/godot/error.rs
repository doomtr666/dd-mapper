use std::fmt;

use thiserror::Error;

/// Failure to decode a Godot variant literal.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodecError {
    #[error("expected a `{expected}( ... )` literal, found `{found}`")]
    UnexpectedLiteral {
        expected: &'static str,
        found: String,
    },
    #[error("invalid {expected}: `{token}`")]
    InvalidNumber { expected: NumberKind, token: String },
    #[error("`{type_name}` expects exactly {expected} components, found {found}")]
    WrongComponentCount {
        type_name: &'static str,
        expected: usize,
        found: usize,
    },
    #[error("`{type_name}` expects an even number of components, found {found}")]
    OddComponentCount {
        type_name: &'static str,
        found: usize,
    },
}

/// The kind of number a literal component must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    Real,
    Int,
    Byte,
}

impl fmt::Display for NumberKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            Self::Real => "finite real number",
            Self::Int => "32-bit integer",
            Self::Byte => "byte (0 to 255)",
        };
        formatter.write_str(description)
    }
}
