use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Preserved;

/// The file header; only the packs the map needs are modelled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    /// Whether the map uses Dungeondraft's own assets (`res://textures/...`).
    pub uses_default_assets: bool,
    /// The packs Dungeondraft loads to show the map; their assets are missing otherwise.
    pub asset_manifest: Vec<Preserved<PackEntry>>,
}

/// A pack Dungeondraft must load, as its `pack.json` describes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackEntry {
    pub name: String,
    pub id: String,
    pub version: String,
    pub author: String,
    pub keywords: String,
    #[serde(rename = "allow_3rd_party_mapping_software_to_read")]
    pub allow_reading: bool,
    pub custom_color_overrides: Value,
}
