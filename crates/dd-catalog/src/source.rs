use thiserror::Error;

use crate::Asset;

/// Where assets come from: pack indexes, saved maps, hand-written files...
pub trait AssetSource {
    /// Every asset this source knows, with the tags and metrics it knows.
    ///
    /// # Errors
    /// Fails when the source cannot be read.
    fn assets(&self) -> Result<Vec<Asset>, SourceError>;
}

/// A source failed; the message says which one and why.
#[derive(Debug, Error)]
#[error("{source_name}: {reason}")]
pub struct SourceError {
    pub source_name: String,
    pub reason: String,
}
