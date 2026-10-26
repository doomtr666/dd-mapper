use std::collections::BTreeSet;

use dd_domain::AssetPath;

use crate::AssetKind;

/// Pack id of the assets shipped with Dungeondraft (`res://textures/...`).
pub const DEFAULT_PACK: &str = "default";

/// Footprint of an asset, in cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellSize {
    pub width: f64,
    pub height: f64,
}

/// Where an asset is meant to stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Against a wall, its back along the wall.
    Wall,
    /// Free-standing, centered on its position.
    Center,
}

/// Facts the pack itself does not tell, gathered from maps or by hand.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssetMetrics {
    pub size: Option<CellSize>,
    pub anchor: Option<Anchor>,
    /// Half-width of a door or window, in pixels.
    pub portal_radius: Option<f64>,
}

impl AssetMetrics {
    /// Takes every fact known by `newer`, keeping ours otherwise.
    pub fn update(&mut self, newer: &AssetMetrics) {
        self.size = newer.size.or(self.size);
        self.anchor = newer.anchor.or(self.anchor);
        self.portal_radius = newer.portal_radius.or(self.portal_radius);
    }
}

/// One usable asset: a texture of a pack, with what we know about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub path: AssetPath,
    pub kind: AssetKind,
    /// Pack id (`LOREGD01`), or [`DEFAULT_PACK`].
    pub pack: String,
    /// File name without extension (`Wall_Underhive_Industrial`).
    pub name: String,
    /// Folders between the kind folder and the file (`[LORE] Containers`).
    pub folders: Vec<String>,
    pub tags: BTreeSet<String>,
    pub metrics: AssetMetrics,
}

impl Asset {
    /// Describes the asset at `path`, or `None` when the path is not
    /// a `textures/<kind folder>/` texture.
    #[must_use]
    pub fn from_path(path: AssetPath) -> Option<Self> {
        let (pack, inside_textures) = split_pack(path.as_str())?;
        let mut segments: Vec<&str> = inside_textures.split('/').collect();
        let file = segments.pop()?;
        let (kind_folder, folders) = segments.split_first()?;
        let kind = AssetKind::from_folder(kind_folder)?;
        let name = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        Some(Self {
            kind,
            pack: pack.to_owned(),
            name: name.to_owned(),
            folders: folders.iter().map(|folder| (*folder).to_owned()).collect(),
            tags: BTreeSet::new(),
            metrics: AssetMetrics::default(),
            path,
        })
    }
}

/// Splits `res://packs/<pack>/textures/<rest>` or `res://textures/<rest>`
/// into the pack id and `<rest>`.
fn split_pack(path: &str) -> Option<(&str, &str)> {
    if let Some(rest) = path.strip_prefix("res://textures/") {
        return Some((DEFAULT_PACK, rest));
    }
    let (pack, inside_pack) = path.strip_prefix("res://packs/")?.split_once('/')?;
    Some((pack, inside_pack.strip_prefix("textures/")?))
}

#[cfg(test)]
mod tests {
    use dd_domain::AssetPath;

    use super::{Anchor, Asset, AssetMetrics, CellSize};
    use crate::AssetKind;

    fn asset(path: &str) -> Option<Asset> {
        Asset::from_path(AssetPath::new(path).unwrap())
    }

    #[test]
    fn describes_a_pack_asset_from_its_path() {
        let crate_asset = asset(
            "res://packs/LOREGD01/textures/objects/[LORE] Containers/Crates/Container_Large_Metal_Crate_Green.webp",
        )
        .unwrap();
        assert_eq!(crate_asset.kind, AssetKind::Object);
        assert_eq!(crate_asset.pack, "LOREGD01");
        assert_eq!(crate_asset.name, "Container_Large_Metal_Crate_Green");
        assert_eq!(crate_asset.folders, ["[LORE] Containers", "Crates"]);
        assert!(crate_asset.tags.is_empty());
        assert_eq!(crate_asset.metrics, AssetMetrics::default());
    }

    #[test]
    fn describes_a_default_asset() {
        let wall = asset("res://textures/walls/battlements.png").unwrap();
        assert_eq!(
            (wall.kind, wall.pack.as_str(), wall.name.as_str()),
            (AssetKind::Wall, "default", "battlements")
        );
    }

    #[test]
    fn ignores_files_outside_asset_folders() {
        assert_eq!(asset("res://packs/LOREGD01/thumbnails/abc.png"), None);
        assert_eq!(asset("res://textures/roofs/slate.png"), None);
        assert_eq!(asset("res://packs/LOREGD01/pack.json"), None);
    }

    #[test]
    fn newer_metrics_override_known_ones() {
        let mut metrics = AssetMetrics {
            size: Some(CellSize {
                width: 1.0,
                height: 1.0,
            }),
            anchor: Some(Anchor::Center),
            portal_radius: None,
        };
        metrics.update(&AssetMetrics {
            anchor: Some(Anchor::Wall),
            portal_radius: Some(154.0),
            ..AssetMetrics::default()
        });
        assert_eq!(
            metrics,
            AssetMetrics {
                size: Some(CellSize {
                    width: 1.0,
                    height: 1.0
                }),
                anchor: Some(Anchor::Wall),
                portal_radius: Some(154.0),
            }
        );
    }
}
