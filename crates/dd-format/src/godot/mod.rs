//! Codec for the Godot 3 variant literals Dungeondraft embeds in JSON strings,
//! such as `"Vector2( 4480, 2560 )"` or `"PoolIntArray( -1, 12 )"`.

mod error;
mod literal;
mod number;
mod pool;
mod serde_literal;
mod syntax;
mod vector2;

pub use error::{CodecError, NumberKind};
pub use literal::GodotLiteral;
pub use number::format_real;
pub(crate) use number::trim_fraction_zeros;
pub use pool::{PoolByteArray, PoolIntArray, PoolVector2Array};
pub use vector2::Vector2;
