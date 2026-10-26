//! Assets used in maps saved by Dungeondraft, with what the maps tell about
//! them (door radius). Complements pack indexes, which say nothing of metrics.

use std::path::PathBuf;

use dd_catalog::{Asset, AssetMetrics, AssetSource, SourceError};
use dd_domain::AssetPath;
use dd_format::model::{Level, MapFile};

/// Harvests the assets of `.dungeondraft_map` files.
#[derive(Debug, Clone)]
pub struct MapHarvester {
    maps: Vec<PathBuf>,
}

impl MapHarvester {
    #[must_use]
    pub fn new(maps: Vec<PathBuf>) -> Self {
        Self { maps }
    }
}

impl AssetSource for MapHarvester {
    fn assets(&self) -> Result<Vec<Asset>, SourceError> {
        let mut assets = Vec::new();
        for path in &self.maps {
            let failure = |reason: String| SourceError {
                source_name: path.display().to_string(),
                reason,
            };
            let text = std::fs::read_to_string(path).map_err(|error| failure(error.to_string()))?;
            let map = MapFile::parse(&text).map_err(|error| failure(error.to_string()))?;
            assets.extend(harvest(&map));
        }
        Ok(assets)
    }
}

/// Every asset `map` uses: tilesets, walls, doors (with radius), objects,
/// lights, patterns and terrain textures.
#[must_use]
pub fn harvest(map: &MapFile) -> Vec<Asset> {
    map.world()
        .levels
        .values()
        .flat_map(|level| level_assets(level))
        .collect()
}

fn level_assets(level: &Level) -> Vec<Asset> {
    let terrain = &level.terrain;
    let textures = level
        .tiles
        .lookup
        .values()
        .chain(level.walls.iter().map(|wall| &wall.texture))
        .chain(level.objects.iter().map(|object| &object.texture))
        .chain(level.lights.iter().map(|light| &light.texture))
        .chain(level.patterns.iter().map(|pattern| &pattern.texture))
        .chain([
            &terrain.texture_1,
            &terrain.texture_2,
            &terrain.texture_3,
            &terrain.texture_4,
        ]);
    let mut assets: Vec<Asset> = textures
        .filter_map(|texture| asset_at(texture, AssetMetrics::default()))
        .collect();
    let portals = level.walls.iter().flat_map(|wall| wall.portals.iter());
    assets.extend(portals.filter_map(|portal| {
        let metrics = AssetMetrics {
            portal_radius: Some(portal.radius),
            ..AssetMetrics::default()
        };
        asset_at(&portal.texture, metrics)
    }));
    assets
}

fn asset_at(texture: &str, metrics: AssetMetrics) -> Option<Asset> {
    let mut asset = Asset::from_path(AssetPath::new(texture).ok()?)?;
    asset.metrics = metrics;
    Some(asset)
}

#[cfg(test)]
mod tests {
    use dd_catalog::{Asset, AssetKind};
    use dd_format::model::MapFile;

    use super::harvest;

    fn harvested(fixture: &str) -> Vec<Asset> {
        harvest(&MapFile::parse(fixture).unwrap())
    }

    fn find<'a>(assets: &'a [Asset], name: &str) -> &'a Asset {
        assets
            .iter()
            .find(|asset| asset.name == name)
            .unwrap_or_else(|| panic!("{name} not harvested"))
    }

    #[test]
    fn harvests_walls_doors_and_tilesets() {
        let assets = harvested(include_str!("../../../fixtures/test.dungeondraft_map"));
        assert_eq!(
            find(&assets, "Wall_Underhive_Reinforced").kind,
            AssetKind::Wall
        );
        let door = find(&assets, "Door_Underhive_Industrial");
        assert_eq!(door.kind, AssetKind::Portal);
        assert_eq!(door.metrics.portal_radius, Some(154.0));
        let tilesets = assets
            .iter()
            .filter(|asset| asset.kind == AssetKind::Tileset)
            .count();
        assert_eq!(tilesets, 25);
        assert_eq!(find(&assets, "terrain_dirt").kind, AssetKind::Terrain);
    }

    #[test]
    fn harvests_objects_lights_and_patterns() {
        let assets = harvested(include_str!("../../../fixtures/test2.dungeondraft_map"));
        assert_eq!(
            find(&assets, "Decor_Aquila_Ancient").kind,
            AssetKind::Object
        );
        assert_eq!(
            find(&assets, "Effect_Caged_Light_Cast").kind,
            AssetKind::Light
        );
        assert_eq!(
            find(&assets, "Tileset_AncientNecromunda_Fan_Off").kind,
            AssetKind::Pattern
        );
        assert_eq!(find(&assets, "Tileset_Sulphur_Rough_Gray").pack, "LORENT01");
    }

    #[test]
    fn harvests_default_doors_with_their_radius() {
        let assets = harvested(include_str!(
            "../../../fixtures/doors_and_free_walls.dungeondraft_map"
        ));
        let door = find(&assets, "door_00");
        assert_eq!(
            (door.pack.as_str(), door.metrics.portal_radius),
            ("default", Some(128.0))
        );
    }
}
