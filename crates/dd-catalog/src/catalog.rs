use std::collections::BTreeMap;

use crate::{Asset, AssetKind, AssetSource, SourceError};

/// Search criteria; every given criterion must match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetQuery {
    /// Words to find in the name, folders or tags, in any order and case.
    pub text: String,
    pub kind: Option<AssetKind>,
    /// Tags the asset must all carry (case-insensitive).
    pub tags: Vec<String>,
    pub pack: Option<String>,
    /// At most this many results; `0` means no limit.
    pub limit: usize,
}

/// All known assets, by path. Later additions refine earlier ones.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    assets: BTreeMap<String, Asset>,
}

impl Catalog {
    /// Builds a catalog from `sources`, later sources refining earlier ones.
    ///
    /// # Errors
    /// Fails on the first source that cannot be read.
    pub fn from_sources(sources: &[&dyn AssetSource]) -> Result<Self, SourceError> {
        let mut catalog = Self::default();
        for source in sources {
            for asset in source.assets()? {
                catalog.add(asset);
            }
        }
        Ok(catalog)
    }

    /// Adds an asset; if already known, merges its tags and newer metrics.
    pub fn add(&mut self, asset: Asset) {
        match self.assets.get_mut(asset.path.as_str()) {
            Some(known) => {
                known.tags.extend(asset.tags);
                known.metrics.update(&asset.metrics);
            }
            None => {
                self.assets.insert(asset.path.as_str().to_owned(), asset);
            }
        }
    }

    #[must_use]
    pub fn get(&self, path: &str) -> Option<&Asset> {
        self.assets.get(path)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Matching assets, best first: name matches rank above folder or tag matches.
    #[must_use]
    pub fn search(&self, query: &AssetQuery) -> Vec<&Asset> {
        let words = lowercase_words(&query.text);
        let mut ranked: Vec<(usize, &Asset)> = self
            .assets
            .values()
            .filter(|asset| passes_filters(asset, query))
            .filter_map(|asset| relevance(asset, &words).map(|score| (score, asset)))
            .collect();
        ranked.sort_by(|(first_score, first), (second_score, second)| {
            second_score
                .cmp(first_score)
                .then_with(|| first.name.cmp(&second.name))
        });
        let limit = if query.limit == 0 {
            usize::MAX
        } else {
            query.limit
        };
        ranked
            .into_iter()
            .take(limit)
            .map(|(_, asset)| asset)
            .collect()
    }
}

const NAME_MATCH: usize = 2;
const CONTEXT_MATCH: usize = 1;

fn lowercase_words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn passes_filters(asset: &Asset, query: &AssetQuery) -> bool {
    let kind_matches = query.kind.is_none_or(|kind| asset.kind == kind);
    let pack_matches = query.pack.as_ref().is_none_or(|pack| asset.pack == *pack);
    let tags_match = query.tags.iter().all(|wanted| {
        asset
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(wanted))
    });
    kind_matches && pack_matches && tags_match
}

/// Sum of word scores, or `None` when a word matches nowhere.
fn relevance(asset: &Asset, words: &[String]) -> Option<usize> {
    let name = asset.name.to_lowercase();
    let context = asset
        .folders
        .iter()
        .chain(&asset.tags)
        .map(|text| text.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    words
        .iter()
        .map(|word| {
            if name.contains(word.as_str()) {
                Some(NAME_MATCH)
            } else if context.contains(word.as_str()) {
                Some(CONTEXT_MATCH)
            } else {
                None
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use dd_domain::AssetPath;

    use super::{AssetQuery, Catalog};
    use crate::{Asset, AssetKind, AssetMetrics, AssetSource, SourceError};

    fn asset(path: &str, tags: &[&str]) -> Asset {
        let mut asset = Asset::from_path(AssetPath::new(path).unwrap()).unwrap();
        asset.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        asset
    }

    fn sample() -> Catalog {
        let mut catalog = Catalog::default();
        for (path, tags) in [
            (
                "res://packs/LOREGD01/textures/objects/[LORE] Containers/Crates/Container_Large_Metal_Crate_Green.webp",
                &["[LORE] Containers - Crates"][..],
            ),
            (
                "res://packs/LOREGD01/textures/objects/[LORE] Buildings/Decor_Aquila_Ancient.webp",
                &["[LORE] Buildings - Greebles"][..],
            ),
            (
                "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp",
                &[][..],
            ),
            (
                "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp",
                &[][..],
            ),
            (
                "res://textures/objects/furniture/crate_wood.png",
                &["Colorable"][..],
            ),
        ] {
            catalog.add(asset(path, tags));
        }
        catalog
    }

    fn names(results: &[&Asset]) -> Vec<String> {
        results.iter().map(|asset| asset.name.clone()).collect()
    }

    fn query(text: &str) -> AssetQuery {
        AssetQuery {
            text: text.to_owned(),
            ..AssetQuery::default()
        }
    }

    #[test]
    fn every_word_must_match_in_any_case() {
        assert_eq!(
            names(&sample().search(&query("CRATE metal"))),
            ["Container_Large_Metal_Crate_Green"]
        );
    }

    #[test]
    fn name_matches_rank_first() {
        let catalog = sample();
        let results = catalog.search(&query("crate"));
        assert_eq!(
            names(&results),
            ["Container_Large_Metal_Crate_Green", "crate_wood"]
        );
    }

    #[test]
    fn words_can_match_folders_and_tags() {
        assert_eq!(
            names(&sample().search(&query("greebles"))),
            ["Decor_Aquila_Ancient"]
        );
        assert_eq!(
            names(&sample().search(&query("buildings aquila"))),
            ["Decor_Aquila_Ancient"]
        );
    }

    #[test]
    fn filters_by_kind_pack_and_tags() {
        let catalog = sample();
        let walls = catalog.search(&AssetQuery {
            kind: Some(AssetKind::Wall),
            ..query("underhive")
        });
        assert_eq!(names(&walls), ["Wall_Underhive_Industrial"]);
        let default_pack = catalog.search(&AssetQuery {
            pack: Some("default".to_owned()),
            ..AssetQuery::default()
        });
        assert_eq!(names(&default_pack), ["crate_wood"]);
        let colorable = catalog.search(&AssetQuery {
            tags: vec!["colorable".to_owned()],
            ..AssetQuery::default()
        });
        assert_eq!(names(&colorable), ["crate_wood"]);
    }

    #[test]
    fn limits_the_number_of_results() {
        let catalog = sample();
        let results = catalog.search(&AssetQuery {
            limit: 2,
            ..AssetQuery::default()
        });
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn adding_a_known_asset_merges_tags_and_metrics() {
        let mut catalog = sample();
        let path = "res://packs/LOREGD01/textures/portals/Door_Underhive_Industrial.webp";
        let mut harvested = asset(path, &["door"]);
        harvested.metrics = AssetMetrics {
            portal_radius: Some(154.0),
            ..AssetMetrics::default()
        };
        catalog.add(harvested);
        let door = catalog.get(path).unwrap();
        assert_eq!(door.tags, BTreeSet::from(["door".to_owned()]));
        assert_eq!(door.metrics.portal_radius, Some(154.0));
        assert_eq!(catalog.len(), 5);
    }

    struct FixedSource(Vec<Asset>);

    impl AssetSource for FixedSource {
        fn assets(&self) -> Result<Vec<Asset>, SourceError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn later_sources_refine_earlier_ones() {
        let path = "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp";
        let pack = FixedSource(vec![asset(path, &["Walls"])]);
        let notes = FixedSource(vec![asset(path, &["industrial"])]);
        let catalog = Catalog::from_sources(&[&pack, &notes]).unwrap();
        let wall = catalog.get(path).unwrap();
        assert_eq!(wall.tags.len(), 2);
    }
}
