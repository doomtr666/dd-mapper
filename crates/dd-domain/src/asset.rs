use thiserror::Error;

/// Reference to an editor asset (a texture path), opaque to the domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssetPathError {
    #[error("an asset path cannot be blank")]
    Blank,
}

impl AssetPath {
    /// # Errors
    /// Fails when the path is empty or only whitespace.
    pub fn new(path: impl Into<String>) -> Result<Self, AssetPathError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Err(AssetPathError::Blank);
        }
        Ok(Self(path))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetPath, AssetPathError};

    #[test]
    fn keeps_the_path_verbatim() {
        let path = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp";
        assert_eq!(AssetPath::new(path).unwrap().as_str(), path);
    }

    #[test]
    fn rejects_blank_paths() {
        assert_eq!(AssetPath::new(" \t"), Err(AssetPathError::Blank));
    }
}
