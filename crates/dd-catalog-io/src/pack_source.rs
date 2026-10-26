//! Assets listed in the index of a Dungeondraft pack, or of `Dungeondraft.pck`
//! for the editor's default assets. Only names and metadata are read.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::PathBuf;

use dd_catalog::{Asset, AssetKind, AssetSource, CellSize, SourceError};
use dd_domain::AssetPath;
use serde::Deserialize;
use thiserror::Error;

use dd_format::export::CELL_SIZE_PX;

use crate::image_size::{HEADER_BYTES, PixelSize, image_size};
use crate::pck::{PckEntry, PckError, PckIndex, read_metadata, read_prefix};

const DEFAULT_TAGS: &str = "res://data/default.dungeondraft_tags";
/// Godot keeps the imported version of default textures beside this marker.
const IMPORT_SUFFIX: &str = ".import";

/// Reads the assets of one `.dungeondraft_pack` or `Dungeondraft.pck`.
#[derive(Debug, Clone)]
pub struct PackIndexSource {
    archive: PathBuf,
}

#[derive(Debug, Error)]
pub enum PackError {
    #[error(transparent)]
    Pck(#[from] PckError),
    #[error("cannot open the pack: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid {file}: {reason}")]
    InvalidMetadata { file: String, reason: String },
    #[error("pack {pack} does not allow third-party mapping software to read it")]
    ReadingNotAllowed { pack: String },
}

impl PackIndexSource {
    #[must_use]
    pub fn new(archive: impl Into<PathBuf>) -> Self {
        Self {
            archive: archive.into(),
        }
    }
}

impl AssetSource for PackIndexSource {
    fn assets(&self) -> Result<Vec<Asset>, SourceError> {
        let read = || -> Result<Vec<Asset>, PackError> {
            let mut archive = BufReader::new(File::open(&self.archive)?);
            assets_from_archive(&mut archive)
        };
        read().map_err(|error| SourceError {
            source_name: self.archive.display().to_string(),
            reason: error.to_string(),
        })
    }
}

/// The assets of a pack archive, tagged with the pack's own tags and sets.
///
/// # Errors
/// Fails when the archive or its metadata cannot be read, or when the pack
/// does not allow third-party mapping software to read it.
pub fn assets_from_archive<R: Read + Seek>(archive: &mut R) -> Result<Vec<Asset>, PackError> {
    let index = PckIndex::read(archive)?;
    let manifest = read_manifest(archive, &index)?;
    if let Some(manifest) = &manifest
        && !manifest.allow_reading
    {
        return Err(PackError::ReadingNotAllowed {
            pack: manifest.id.clone(),
        });
    }
    let layout = PackLayout::of(manifest.as_ref());
    let tags = read_tags(archive, &index, &layout)?;
    let entries_by_path: HashMap<&str, &PckEntry> = index
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let mut assets = Vec::new();
    for entry in &index.entries {
        let Some(mut asset) = texture_asset(&entry.path) else {
            continue;
        };
        if let Some(labels) = tags.get(asset.path.as_str()) {
            asset.tags.extend(labels.iter().cloned());
        }
        if has_footprint(asset.kind) {
            asset.metrics.size = texture_size(archive, entry, &entries_by_path)?.map(to_cells);
        }
        assets.push(asset);
    }
    Ok(assets)
}

#[derive(Debug, Deserialize)]
struct Manifest {
    id: String,
    #[serde(rename = "allow_3rd_party_mapping_software_to_read")]
    allow_reading: bool,
}

#[derive(Debug, Default, Deserialize)]
struct TagsFile {
    #[serde(default)]
    tags: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    sets: BTreeMap<String, Vec<String>>,
}

/// Where a pack keeps its files: packs live under `res://packs/<id>/`,
/// Dungeondraft's own assets directly under `res://`.
struct PackLayout {
    root: String,
    tags_file: String,
}

impl PackLayout {
    fn of(manifest: Option<&Manifest>) -> Self {
        match manifest {
            Some(manifest) => {
                let root = format!("res://packs/{}/", manifest.id);
                let tags_file = format!("{root}data/default.dungeondraft_tags");
                Self { root, tags_file }
            }
            None => Self {
                root: "res://".to_owned(),
                tags_file: DEFAULT_TAGS.to_owned(),
            },
        }
    }

    /// Tag files list paths relative to the pack root, or absolute.
    fn absolute(&self, path: &str) -> String {
        if path.starts_with("res://") {
            path.to_owned()
        } else {
            format!("{}{path}", self.root)
        }
    }
}

fn read_manifest<R: Read + Seek>(
    archive: &mut R,
    index: &PckIndex,
) -> Result<Option<Manifest>, PackError> {
    let entry = index
        .entries
        .iter()
        .find(|entry| is_pack_manifest(&entry.path));
    entry.map(|entry| read_json(archive, entry)).transpose()
}

/// `res://packs/<id>/pack.json`.
fn is_pack_manifest(path: &str) -> bool {
    path.strip_prefix("res://packs/")
        .and_then(|inside| inside.split_once('/'))
        .is_some_and(|(_, file)| file == "pack.json")
}

/// Tags of each asset path: its own tags plus the sets containing them.
fn read_tags<R: Read + Seek>(
    archive: &mut R,
    index: &PckIndex,
    layout: &PackLayout,
) -> Result<HashMap<String, BTreeSet<String>>, PackError> {
    let file: TagsFile = match index.find(&layout.tags_file) {
        Some(entry) => read_json(archive, entry)?,
        None => TagsFile::default(),
    };
    let mut sets_of_tag: HashMap<&str, Vec<&str>> = HashMap::new();
    for (set, tags) in &file.sets {
        for tag in tags {
            sets_of_tag.entry(tag).or_default().push(set);
        }
    }
    let mut labels_by_path: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (tag, paths) in &file.tags {
        let sets = sets_of_tag.get(tag.as_str()).into_iter().flatten();
        let labels: Vec<String> = std::iter::once(tag.as_str())
            .chain(sets.copied())
            .map(str::to_owned)
            .collect();
        for path in paths {
            labels_by_path
                .entry(layout.absolute(path))
                .or_default()
                .extend(labels.iter().cloned());
        }
    }
    Ok(labels_by_path)
}

fn read_json<R: Read + Seek, T: for<'de> Deserialize<'de>>(
    archive: &mut R,
    entry: &PckEntry,
) -> Result<T, PackError> {
    let bytes = read_metadata(archive, entry)?;
    serde_json::from_slice(&bytes).map_err(|error| PackError::InvalidMetadata {
        file: entry.path.clone(),
        reason: error.to_string(),
    })
}

/// Kinds placed as a single picture, whose image size is their footprint.
fn has_footprint(kind: AssetKind) -> bool {
    matches!(
        kind,
        AssetKind::Object | AssetKind::Portal | AssetKind::Light
    )
}

/// Image size of the texture, read from its header. Default assets are
/// `.import` files naming the Godot stream texture that holds the image.
fn texture_size<R: Read + Seek>(
    archive: &mut R,
    entry: &PckEntry,
    entries_by_path: &HashMap<&str, &PckEntry>,
) -> Result<Option<PixelSize>, PackError> {
    let texture = if entry.path.ends_with(IMPORT_SUFFIX) {
        let import = read_metadata(archive, entry)?;
        let imported = imported_texture(&String::from_utf8_lossy(&import))
            .and_then(|path| entries_by_path.get(path.as_str()).copied());
        let Some(imported) = imported else {
            return Ok(None);
        };
        imported
    } else {
        entry
    };
    Ok(image_size(&read_prefix(archive, texture, HEADER_BYTES)?))
}

/// The `path="..."` line of a Godot `.import` file.
fn imported_texture(import: &str) -> Option<String> {
    import.lines().find_map(|line| {
        line.strip_prefix("path=\"")
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_owned)
    })
}

/// Dungeondraft draws textures at 256 pixels per cell.
fn to_cells(size: PixelSize) -> CellSize {
    CellSize {
        width: f64::from(size.width) / CELL_SIZE_PX,
        height: f64::from(size.height) / CELL_SIZE_PX,
    }
}

fn texture_asset(entry_path: &str) -> Option<Asset> {
    let path = entry_path.strip_suffix(IMPORT_SUFFIX).unwrap_or(entry_path);
    AssetPath::new(path).ok().and_then(Asset::from_path)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use dd_catalog::{Asset, AssetKind};

    use super::{PackError, assets_from_archive};
    use crate::pck::tests::build_pck;

    const MANIFEST: &[u8] =
        br#"{"name": "LORE Grimdorx A 3.09", "id": "LOREGD01", "allow_3rd_party_mapping_software_to_read": true}"#;

    fn read(files: &[(&str, &[u8])]) -> Result<Vec<Asset>, PackError> {
        assets_from_archive(&mut Cursor::new(build_pck(files)))
    }

    fn summary(assets: &[Asset]) -> Vec<(String, AssetKind, String)> {
        assets
            .iter()
            .map(|asset| (asset.name.clone(), asset.kind, asset.pack.clone()))
            .collect()
    }

    #[test]
    fn lists_the_textures_of_a_pack() {
        let assets = read(&[
            ("res://packs/LOREGD01/pack.json", MANIFEST),
            (
                "res://packs/LOREGD01/textures/walls/Wall_Underhive_Industrial.webp",
                b"",
            ),
            (
                "res://packs/LOREGD01/textures/objects/Crates/Crate_Green.webp",
                b"",
            ),
            ("res://packs/LOREGD01/thumbnails/abc.png", b""),
            ("res://packs/LOREGD01/preview.png", b""),
        ])
        .unwrap();
        assert_eq!(
            summary(&assets),
            [
                (
                    "Wall_Underhive_Industrial".to_owned(),
                    AssetKind::Wall,
                    "LOREGD01".to_owned()
                ),
                (
                    "Crate_Green".to_owned(),
                    AssetKind::Object,
                    "LOREGD01".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn tags_assets_with_their_tags_and_sets() {
        let tags = br#"{
            "tags": {"[LORE] Containers - Crates": ["textures/objects/Crates/Crate_Green.webp"]},
            "sets": {"LORE Grimdorx A": ["[LORE] Containers - Crates"]}
        }"#;
        let assets = read(&[
            ("res://packs/LOREGD01/pack.json", MANIFEST),
            ("res://packs/LOREGD01/data/default.dungeondraft_tags", tags),
            (
                "res://packs/LOREGD01/textures/objects/Crates/Crate_Green.webp",
                b"",
            ),
        ])
        .unwrap();
        let tags: Vec<&str> = assets[0].tags.iter().map(String::as_str).collect();
        assert_eq!(tags, ["LORE Grimdorx A", "[LORE] Containers - Crates"]);
    }

    #[test]
    fn reads_the_default_assets_of_dungeondraft() {
        let tags =
            br#"{"tags": {"Crate": ["res://textures/objects/crates/crate_01.png"]}, "sets": {}}"#;
        let assets = read(&[
            ("res://data/default.dungeondraft_tags", tags),
            ("res://textures/walls/battlements.png.import", b""),
            ("res://textures/objects/crates/crate_01.png.import", b""),
            ("res://scripts/editor.gdc", b""),
        ])
        .unwrap();
        assert_eq!(
            assets
                .iter()
                .map(|asset| asset.path.as_str())
                .collect::<Vec<_>>(),
            [
                "res://textures/walls/battlements.png",
                "res://textures/objects/crates/crate_01.png"
            ]
        );
        assert_eq!(assets[0].pack, "default");
        assert!(assets[1].tags.contains("Crate"));
    }

    /// A WebP header announcing a 512 x 256 picture.
    fn webp_512_by_256() -> Vec<u8> {
        let mut header = b"RIFF\0\0\0\0WEBPVP8X\0\0\0\0\0\0\0\0".to_vec();
        header.extend([0xff, 0x01, 0x00, 0xff, 0x00, 0x00]);
        header
    }

    #[test]
    fn sizes_objects_portals_and_lights_from_their_image_header() {
        let header = webp_512_by_256();
        let assets = read(&[
            ("res://packs/LOREGD01/pack.json", MANIFEST),
            ("res://packs/LOREGD01/textures/objects/Crate.webp", &header),
            ("res://packs/LOREGD01/textures/portals/Door.webp", &header),
            ("res://packs/LOREGD01/textures/walls/Wall.webp", &header),
            (
                "res://packs/LOREGD01/textures/objects/Broken.webp",
                b"not an image",
            ),
        ])
        .unwrap();
        let sizes: Vec<Option<(f64, f64)>> = assets
            .iter()
            .map(|asset| asset.metrics.size.map(|size| (size.width, size.height)))
            .collect();
        assert_eq!(sizes, [Some((2.0, 1.0)), Some((2.0, 1.0)), None, None]);
    }

    #[test]
    fn sizes_default_assets_through_their_import_file() {
        let import = br#"[remap]
importer="texture"
path="res://.import/crate_01.png-0123.stex"
"#;
        let mut stream_texture = b"GDST".to_vec();
        for field in [128_u16, 0, 64, 0] {
            stream_texture.extend(field.to_le_bytes());
        }
        let assets = read(&[
            ("res://textures/objects/crate_01.png.import", import),
            ("res://.import/crate_01.png-0123.stex", &stream_texture),
        ])
        .unwrap();
        let size = assets[0].metrics.size.unwrap();
        assert_eq!((size.width, size.height), (0.5, 0.25));
    }

    #[test]
    fn respects_packs_forbidding_third_party_reading() {
        let manifest = br#"{"id": "SECRET01", "allow_3rd_party_mapping_software_to_read": false}"#;
        let error = read(&[("res://packs/SECRET01/pack.json", manifest)]).unwrap_err();
        assert!(
            matches!(&error, PackError::ReadingNotAllowed { pack } if pack == "SECRET01"),
            "{error:?}"
        );
    }

    #[test]
    fn reports_invalid_metadata() {
        let error = read(&[("res://packs/X/pack.json", b"not json")]).unwrap_err();
        assert!(
            matches!(&error, PackError::InvalidMetadata { file, .. } if file == "res://packs/X/pack.json"),
            "{error:?}"
        );
    }
}
