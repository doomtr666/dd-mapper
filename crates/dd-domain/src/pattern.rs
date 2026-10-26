use crate::{AssetPath, Polygon};

/// A texture painted over an area of the floor: grates, fans, markings...
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub asset: AssetPath,
    pub area: Polygon,
    /// ARGB tint; opaque white keeps the texture as is.
    pub tint: u32,
}
