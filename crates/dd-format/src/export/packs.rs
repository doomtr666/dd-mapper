//! The packs a map needs, declared in its header so that Dungeondraft loads them.

use std::collections::BTreeSet;

use serde_json::json;

use super::{ExportSettings, PackManifest};
use crate::Preserved;
use crate::model::{Header, PackEntry};

const PACK_PREFIX: &str = "res://packs/";
const DEFAULT_PREFIX: &str = "res://textures/";

/// Adds the packs of the `used` assets that the header does not declare yet,
/// and flags the use of default assets.
pub fn declare_packs(header: &mut Header, used: &[&str], settings: &ExportSettings) {
    let packs: BTreeSet<&str> = used.iter().filter_map(|asset| pack_id(asset)).collect();
    let missing: Vec<&str> = packs
        .into_iter()
        .filter(|id| header.asset_manifest.iter().all(|pack| pack.id != *id))
        .collect();
    if !missing.is_empty() {
        for id in missing {
            let entry = pack_entry(id, settings.packs.get(id));
            header.asset_manifest.push(Preserved::new(entry));
        }
        header
            .asset_manifest
            .sort_by(|first, second| first.id.cmp(&second.id));
    }
    if used.iter().any(|asset| asset.starts_with(DEFAULT_PREFIX)) {
        header.uses_default_assets = true;
    }
}

/// `LOREGD01` in `res://packs/LOREGD01/textures/...`.
fn pack_id(path: &str) -> Option<&str> {
    path.strip_prefix(PACK_PREFIX)?.split('/').next()
}

/// An entry like the ones Dungeondraft writes; named after its id when the
/// pack is unknown.
fn pack_entry(id: &str, manifest: Option<&PackManifest>) -> PackEntry {
    let text = |field: fn(&PackManifest) -> &String| manifest.map(field).cloned();
    PackEntry {
        name: text(|pack| &pack.name).unwrap_or_else(|| id.to_owned()),
        id: id.to_owned(),
        version: text(|pack| &pack.version).unwrap_or_default(),
        author: text(|pack| &pack.author).unwrap_or_default(),
        keywords: text(|pack| &pack.keywords).unwrap_or_default(),
        allow_reading: false,
        custom_color_overrides: json!({
            "enabled": false,
            "min_redness": 0,
            "min_saturation": 0,
            "red_tolerance": 0,
        }),
    }
}
