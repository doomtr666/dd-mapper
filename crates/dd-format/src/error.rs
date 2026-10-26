use thiserror::Error;

/// Failure to read or write a `.dungeondraft_map` file.
#[derive(Debug, Error)]
pub enum FormatError {
    #[error("invalid Dungeondraft map JSON: {0}")]
    Json(#[from] serde_json::Error),
}
