//! Hand-written asset notes, `catalog.toml`:
//!
//! ```toml
//! [[asset]]
//! path = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp"
//! tags = ["door", "industrial"]
//! size = [1.2, 0.3]       # width and height, in cells
//! anchor = "wall"         # or "center"
//! portal_radius = 154     # pixels
//! ```

use std::path::PathBuf;

use dd_catalog::{Anchor, Asset, AssetMetrics, AssetSource, CellSize, SourceError};
use dd_domain::AssetPath;
use serde::Deserialize;
use thiserror::Error;

/// Reads a `catalog.toml` file.
#[derive(Debug, Clone)]
pub struct TomlCatalogSource {
    file: PathBuf,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TomlCatalogError {
    #[error("{0}")]
    Syntax(String),
    #[error("`{path}` is not a texture of an asset folder (tilesets, walls, portals, objects...)")]
    NotAnAsset { path: String },
}

impl TomlCatalogSource {
    #[must_use]
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self { file: file.into() }
    }
}

impl AssetSource for TomlCatalogSource {
    fn assets(&self) -> Result<Vec<Asset>, SourceError> {
        let failure = |reason: String| SourceError {
            source_name: self.file.display().to_string(),
            reason,
        };
        let text =
            std::fs::read_to_string(&self.file).map_err(|error| failure(error.to_string()))?;
        assets_from_toml(&text).map_err(|error| failure(error.to_string()))
    }
}

/// Parses the content of a `catalog.toml` file.
///
/// # Errors
/// Fails on invalid TOML, unknown keys, or paths that are not assets.
pub fn assets_from_toml(text: &str) -> Result<Vec<Asset>, TomlCatalogError> {
    let file: CatalogFile =
        toml::from_str(text).map_err(|error| TomlCatalogError::Syntax(error.to_string()))?;
    file.asset.into_iter().map(Entry::into_asset).collect()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogFile {
    #[serde(default)]
    asset: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    #[serde(default)]
    tags: Vec<String>,
    /// Width and height, in cells.
    size: Option<[f64; 2]>,
    anchor: Option<AnchorName>,
    portal_radius: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AnchorName {
    Wall,
    Center,
}

impl From<AnchorName> for Anchor {
    fn from(name: AnchorName) -> Self {
        match name {
            AnchorName::Wall => Self::Wall,
            AnchorName::Center => Self::Center,
        }
    }
}

impl Entry {
    fn into_asset(self) -> Result<Asset, TomlCatalogError> {
        let not_an_asset = || TomlCatalogError::NotAnAsset {
            path: self.path.clone(),
        };
        let path = AssetPath::new(self.path.clone()).map_err(|_| not_an_asset())?;
        let mut asset = Asset::from_path(path).ok_or_else(not_an_asset)?;
        asset.tags = self.tags.into_iter().collect();
        asset.metrics = AssetMetrics {
            size: self.size.map(|[width, height]| CellSize { width, height }),
            anchor: self.anchor.map(Anchor::from),
            portal_radius: self.portal_radius,
        };
        Ok(asset)
    }
}

#[cfg(test)]
mod tests {
    use dd_catalog::{Anchor, AssetKind, CellSize};

    use super::{TomlCatalogError, assets_from_toml};

    #[test]
    fn reads_tags_and_metrics() {
        let assets = assets_from_toml(
            r#"
            [[asset]]
            path = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp"
            tags = ["door", "industrial"]
            size = [1.2, 0.3]
            anchor = "wall"
            portal_radius = 154

            [[asset]]
            path = "res://textures/objects/furniture/table_01.png"
            anchor = "center"
            "#,
        )
        .unwrap();
        let door = &assets[0];
        assert_eq!(door.kind, AssetKind::Portal);
        assert_eq!(door.tags.len(), 2);
        assert_eq!(
            door.metrics.size,
            Some(CellSize {
                width: 1.2,
                height: 0.3
            })
        );
        assert_eq!(door.metrics.anchor, Some(Anchor::Wall));
        assert_eq!(door.metrics.portal_radius, Some(154.0));
        assert_eq!(assets[1].metrics.anchor, Some(Anchor::Center));
    }

    #[test]
    fn an_empty_file_has_no_assets() {
        assert_eq!(assets_from_toml(""), Ok(vec![]));
    }

    #[test]
    fn rejects_misspelled_keys() {
        let error = assets_from_toml(
            r#"
            [[asset]]
            path = "res://textures/walls/stone.png"
            portal_raduis = 154
            "#,
        )
        .unwrap_err();
        assert!(
            matches!(&error, TomlCatalogError::Syntax(message) if message.contains("portal_raduis")),
            "{error:?}"
        );
    }

    #[test]
    fn rejects_paths_outside_asset_folders() {
        let path = "res://packs/LOREGD01/thumbnails/abc.png";
        let error = assets_from_toml(&format!("[[asset]]\npath = \"{path}\"\n")).unwrap_err();
        assert_eq!(
            error,
            TomlCatalogError::NotAnAsset {
                path: path.to_owned()
            }
        );
    }
}
