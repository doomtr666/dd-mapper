//! Asset references checked against the catalog.

use dd_catalog::{Asset, AssetKind, AssetQuery, Catalog};

use crate::AppError;

const SUGGESTIONS: usize = 3;

/// The catalog asset at `path`, which must be of `kind`.
///
/// # Errors
/// Fails with close matches when the path is unknown, or when the asset
/// is of another kind.
pub fn resolve<'c>(
    catalog: &'c Catalog,
    path: &str,
    kind: AssetKind,
) -> Result<&'c Asset, AppError> {
    let Some(asset) = catalog.get(path) else {
        return Err(AppError::UnknownAsset {
            path: path.to_owned(),
            kind,
            suggestions: suggestions(catalog, path, kind),
        });
    };
    if asset.kind != kind {
        return Err(AppError::WrongAssetKind {
            path: path.to_owned(),
            expected: kind,
            actual: asset.kind,
        });
    }
    Ok(asset)
}

/// `path | kind | size | tags`, sizes rounded to hundredths of a cell.
#[must_use]
pub fn asset_line(asset: &Asset) -> String {
    let size = asset.metrics.size.map_or_else(String::new, |size| {
        format!(
            " | {} x {} cells",
            hundredths(size.width),
            hundredths(size.height)
        )
    });
    let tags = if asset.tags.is_empty() {
        String::new()
    } else {
        format!(
            " | tags: {}",
            asset.tags.iter().cloned().collect::<Vec<_>>().join(", ")
        )
    };
    format!("{} | {}{size}{tags}", asset.path.as_str(), asset.kind)
}

/// Two decimals are plenty to place props.
fn hundredths(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Assets of `kind` sharing the most specific word of the requested file name.
fn suggestions(catalog: &Catalog, path: &str, kind: AssetKind) -> Vec<String> {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let mut words: Vec<&str> = stem
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.len() > 2)
        .collect();
    words.sort_by_key(|word| std::cmp::Reverse(word.len()));
    words
        .into_iter()
        .map(|word| {
            catalog.search(&AssetQuery {
                text: word.to_owned(),
                kind: Some(kind),
                limit: SUGGESTIONS,
                ..AssetQuery::default()
            })
        })
        .find(|matches| !matches.is_empty())
        .map(|matches| {
            matches
                .iter()
                .map(|asset| asset.path.as_str().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) mod tests {
    use dd_catalog::{Asset, AssetKind, AssetMetrics, Catalog, CellSize, PackInfo};
    use dd_domain::AssetPath;

    use super::{asset_line, resolve};
    use crate::AppError;

    pub const FLOOR: &str =
        "res://packs/LOREGD01/textures/tilesets/simple/Tileset_RT_Punched_Floor.webp";
    pub const WALL: &str = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp";
    pub const DOOR: &str = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp";
    pub const LIGHT: &str = "res://packs/LOREGD01/textures/lights/Effect_Caged_Light_Cast.webp";
    pub const GRATE: &str = "res://packs/LOREGD01/textures/patterns/normal/Artifcial/Tileset_AncientGothic_Quarters_Sewer_Open.webp";
    pub const CRATE: &str = "res://packs/LOREGD01/textures/objects/[LORE] Containers/Container_Large_Metal_Crate_Green.webp";

    /// A catalog with one asset of each kind the use cases need.
    pub const VOID: &str = "res://textures/terrain/terrain_void.png";
    pub const ROLLER: &str = "res://packs/LOREAP01/textures/portals/Door_Roller_Bay.webp";
    pub const BELT: &str =
        "res://packs/LOREGD01/textures/paths/Artificial/Path_Conveyor_Belt_Large.webp";
    pub const CAN: &str =
        "res://packs/LOREGD01/textures/objects/[LORE] Clutter/Debris_Can_Empty.webp";
    /// Tagged `Colorable`: Dungeondraft draws it red until it gets a color.
    pub const ARMCHAIR: &str = "res://textures/objects/furniture/chairs/arm_chair_01.png";

    pub fn small_catalog() -> Catalog {
        let mut catalog = Catalog::default();
        for (path, metrics) in [
            (FLOOR, AssetMetrics::default()),
            (WALL, AssetMetrics::default()),
            (LIGHT, AssetMetrics::default()),
            (GRATE, AssetMetrics::default()),
            (VOID, AssetMetrics::default()),
            (BELT, AssetMetrics::default()),
            (
                ROLLER,
                AssetMetrics {
                    size: Some(CellSize {
                        width: 3.82,
                        height: 0.57,
                    }),
                    ..AssetMetrics::default()
                },
            ),
            (
                CAN,
                AssetMetrics {
                    size: Some(CellSize {
                        width: 0.2,
                        height: 0.3,
                    }),
                    ..AssetMetrics::default()
                },
            ),
            (
                DOOR,
                AssetMetrics {
                    portal_radius: Some(154.0),
                    ..AssetMetrics::default()
                },
            ),
            (
                CRATE,
                AssetMetrics {
                    size: Some(CellSize {
                        width: 2.0,
                        height: 1.5,
                    }),
                    ..AssetMetrics::default()
                },
            ),
        ] {
            let mut asset = Asset::from_path(AssetPath::new(path).unwrap()).unwrap();
            asset.metrics = metrics;
            catalog.add(asset);
        }
        let mut armchair = Asset::from_path(AssetPath::new(ARMCHAIR).unwrap()).unwrap();
        armchair.tags.insert("Colorable".to_owned());
        catalog.add(armchair);
        catalog.add_pack(PackInfo {
            id: "LOREGD01".to_owned(),
            name: "LORE Grimdorx A 3.09".to_owned(),
            version: "3.9".to_owned(),
            author: "LORE".to_owned(),
            keywords: String::new(),
        });
        catalog
    }

    #[test]
    fn finds_an_asset_of_the_requested_kind() {
        let catalog = small_catalog();
        let wall = resolve(&catalog, WALL, AssetKind::Wall).unwrap();
        assert_eq!(wall.name, "Wall_Underhive_Industrial");
    }

    #[test]
    fn rejects_an_asset_of_another_kind() {
        assert_eq!(
            resolve(&small_catalog(), WALL, AssetKind::Tileset).unwrap_err(),
            AppError::WrongAssetKind {
                path: WALL.to_owned(),
                expected: AssetKind::Tileset,
                actual: AssetKind::Wall
            }
        );
    }

    #[test]
    fn suggests_close_assets_for_a_misspelled_path() {
        let typo = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrail.webp";
        let AppError::UnknownAsset { suggestions, .. } =
            resolve(&small_catalog(), typo, AssetKind::Wall).unwrap_err()
        else {
            panic!("expected an unknown asset error");
        };
        assert_eq!(suggestions, [WALL]);
    }

    #[test]
    fn lines_show_path_kind_size_and_tags() {
        let catalog = small_catalog();
        let mut crate_asset = catalog.get(CRATE).unwrap().clone();
        assert_eq!(
            asset_line(&crate_asset),
            format!("{CRATE} | objects | 2 x 1.5 cells")
        );
        crate_asset.tags = ["container".to_owned(), "metal".to_owned()].into();
        assert!(asset_line(&crate_asset).ends_with("cells | tags: container, metal"));
    }
}
