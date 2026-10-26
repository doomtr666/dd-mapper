//! Catalog of the assets a map can use, fed by [`AssetSource`]s.

mod asset;
mod catalog;
mod kind;
mod source;

pub use asset::{Anchor, Asset, AssetMetrics, CellSize, DEFAULT_PACK};
pub use catalog::{AssetQuery, Catalog};
pub use kind::AssetKind;
pub use source::{AssetSource, SourceError};
