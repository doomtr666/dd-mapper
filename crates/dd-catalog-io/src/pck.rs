//! Reads the file index of a Godot 3 PCK archive (`.dungeondraft_pack`,
//! `Dungeondraft.pck`) without touching the files it contains.
//!
//! Layout: `GDPC`, format version, engine version (3 x u32), 16 reserved u32,
//! file count, then per file: path length, path (zero-padded), offset, size, MD5.

use std::io::{self, Read, Seek, SeekFrom};

use thiserror::Error;

const MAGIC: &[u8; 4] = b"GDPC";
const SUPPORTED_FORMAT: u32 = 1;
const ENGINE_VERSION_FIELDS: usize = 3;
const RESERVED_FIELDS: usize = 16;
const MD5_BYTES: usize = 16;
/// Metadata files (pack.json, tags) are small; anything bigger is corrupt data.
pub const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
/// Godot paths are short; anything bigger is corrupt data.
const MAX_PATH_BYTES: u32 = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PckEntry {
    pub path: String,
    /// From the start of the archive.
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PckIndex {
    pub entries: Vec<PckEntry>,
}

#[derive(Debug, Error)]
pub enum PckError {
    #[error("not a Godot PCK archive")]
    NotAPck,
    #[error("PCK format {0} is not supported, only Godot 3 archives (format 1) are")]
    UnsupportedFormat(u32),
    #[error("corrupt PCK index: {0}")]
    Corrupt(&'static str),
    #[error("cannot read the PCK archive: {0}")]
    Io(#[from] io::Error),
}

impl PckIndex {
    /// Reads the index at the start of `archive`.
    ///
    /// # Errors
    /// Fails when the data is not a readable Godot 3 PCK index.
    pub fn read(archive: &mut impl Read) -> Result<Self, PckError> {
        let mut magic = [0_u8; 4];
        archive.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(PckError::NotAPck);
        }
        let format = read_u32(archive)?;
        if format != SUPPORTED_FORMAT {
            return Err(PckError::UnsupportedFormat(format));
        }
        skip(archive, 4 * (ENGINE_VERSION_FIELDS + RESERVED_FIELDS))?;
        let count = read_u32(archive)?;
        let entries = (0..count)
            .map(|_| read_entry(archive))
            .collect::<Result<_, _>>()?;
        Ok(Self { entries })
    }

    #[must_use]
    pub fn find(&self, path: &str) -> Option<&PckEntry> {
        self.entries.iter().find(|entry| entry.path == path)
    }
}

/// Reads one small file of the archive, such as `pack.json`.
///
/// # Errors
/// Fails when the file is larger than [`MAX_METADATA_BYTES`] or cannot be read.
pub fn read_metadata<R: Read + Seek>(
    archive: &mut R,
    entry: &PckEntry,
) -> Result<Vec<u8>, PckError> {
    if entry.size > MAX_METADATA_BYTES {
        return Err(PckError::Corrupt("metadata file too large"));
    }
    let size = usize::try_from(entry.size).map_err(|_| PckError::Corrupt("file too large"))?;
    archive.seek(SeekFrom::Start(entry.offset))?;
    let mut content = vec![0; size];
    archive.read_exact(&mut content)?;
    Ok(content)
}

fn read_entry(archive: &mut impl Read) -> Result<PckEntry, PckError> {
    let length = read_u32(archive)?;
    if length > MAX_PATH_BYTES {
        return Err(PckError::Corrupt("path too long"));
    }
    let mut path =
        vec![0; usize::try_from(length).map_err(|_| PckError::Corrupt("path too long"))?];
    archive.read_exact(&mut path)?;
    let path = String::from_utf8(path).map_err(|_| PckError::Corrupt("path is not UTF-8"))?;
    let offset = read_u64(archive)?;
    let size = read_u64(archive)?;
    skip(archive, MD5_BYTES)?;
    Ok(PckEntry {
        path: path.trim_end_matches('\0').to_owned(),
        offset,
        size,
    })
}

fn read_u32(archive: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0_u8; 4];
    archive.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(archive: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0_u8; 8];
    archive.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn skip(archive: &mut impl Read, bytes: usize) -> io::Result<()> {
    let wanted = u64::try_from(bytes).unwrap_or(u64::MAX);
    let skipped = io::copy(&mut archive.take(wanted), &mut io::sink())?;
    if skipped < wanted {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(())
}

/// Reads at most the first `length` bytes of a file of the archive.
///
/// # Errors
/// Fails when the archive cannot be read.
pub fn read_prefix<R: Read + Seek>(
    archive: &mut R,
    entry: &PckEntry,
    length: u64,
) -> Result<Vec<u8>, PckError> {
    archive.seek(SeekFrom::Start(entry.offset))?;
    let mut bytes = Vec::new();
    archive
        .by_ref()
        .take(length.min(entry.size))
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::Cursor;

    use super::{PckEntry, PckError, PckIndex, read_metadata, read_prefix};

    /// Builds a Godot 3 PCK archive holding `files`.
    pub fn build_pck(files: &[(&str, &[u8])]) -> Vec<u8> {
        let padded = |path: &str| {
            let mut bytes = path.as_bytes().to_vec();
            bytes.resize(path.len().div_ceil(4) * 4, 0);
            bytes
        };
        let index_size: usize = 4 * 21
            + 4
            + files
                .iter()
                .map(|(path, _)| 4 + padded(path).len() + 8 + 8 + 16)
                .sum::<usize>();
        let mut archive = b"GDPC".to_vec();
        for field in [1_u32, 3, 4, 2] {
            archive.extend(field.to_le_bytes());
        }
        archive.extend([0_u8; 64]);
        archive.extend(u32::try_from(files.len()).unwrap().to_le_bytes());
        let mut offset = index_size;
        for (path, content) in files {
            let path = padded(path);
            archive.extend(u32::try_from(path.len()).unwrap().to_le_bytes());
            archive.extend(&path);
            archive.extend(u64::try_from(offset).unwrap().to_le_bytes());
            archive.extend(u64::try_from(content.len()).unwrap().to_le_bytes());
            archive.extend([0_u8; 16]);
            offset += content.len();
        }
        for (_, content) in files {
            archive.extend(*content);
        }
        archive
    }

    #[test]
    fn lists_every_file_with_its_location() {
        let archive = build_pck(&[("res://packs/X/pack.json", b"{}"), ("res://a.webp", b"abc")]);
        let index = PckIndex::read(&mut Cursor::new(&archive)).unwrap();
        let paths: Vec<&str> = index
            .entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths, ["res://packs/X/pack.json", "res://a.webp"]);
        assert_eq!(index.entries[1].size, 3);
    }

    #[test]
    fn reads_a_small_file() {
        let archive = build_pck(&[("res://packs/X/pack.json", br#"{"id": "X"}"#)]);
        let mut cursor = Cursor::new(&archive);
        let index = PckIndex::read(&mut cursor).unwrap();
        let entry = index.find("res://packs/X/pack.json").unwrap();
        assert_eq!(
            read_metadata(&mut cursor, entry).unwrap(),
            br#"{"id": "X"}"#
        );
    }

    #[test]
    fn reads_only_the_start_of_a_file() {
        let archive = build_pck(&[("res://a.webp", b"RIFF0123WEBP"), ("res://b", b"xy")]);
        let mut cursor = Cursor::new(&archive);
        let index = PckIndex::read(&mut cursor).unwrap();
        assert_eq!(
            read_prefix(&mut cursor, &index.entries[0], 4).unwrap(),
            b"RIFF"
        );
        assert_eq!(
            read_prefix(&mut cursor, &index.entries[1], 30).unwrap(),
            b"xy"
        );
    }

    #[test]
    fn refuses_oversized_metadata() {
        let entry = PckEntry {
            path: "huge".to_owned(),
            offset: 0,
            size: super::MAX_METADATA_BYTES + 1,
        };
        let error = read_metadata(&mut Cursor::new(Vec::new()), &entry).unwrap_err();
        assert!(matches!(error, PckError::Corrupt(_)), "{error:?}");
    }

    #[test]
    fn rejects_other_files_and_godot_4_archives() {
        let error = PckIndex::read(&mut Cursor::new(b"PK\x03\x04 a zip".to_vec())).unwrap_err();
        assert!(matches!(error, PckError::NotAPck), "{error:?}");
        let mut godot_4 = build_pck(&[]);
        godot_4[4] = 2;
        let error = PckIndex::read(&mut Cursor::new(godot_4)).unwrap_err();
        assert!(matches!(error, PckError::UnsupportedFormat(2)), "{error:?}");
    }

    #[test]
    fn reports_truncated_indexes() {
        let mut archive = build_pck(&[("res://a.webp", b"abc")]);
        archive.truncate(100);
        let error = PckIndex::read(&mut Cursor::new(archive)).unwrap_err();
        assert!(matches!(error, PckError::Io(_)), "{error:?}");
    }
}
