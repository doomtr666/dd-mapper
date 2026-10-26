use std::collections::HashMap;
use std::path::Path;

use dd_catalog::PackInfo;
use dd_domain::Map;
use thiserror::Error;

/// An adapter failed; the message comes from it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct PortError(pub String);

/// Size in cells of an object texture, when known.
pub type FootprintOf<'a> = &'a dyn Fn(&str) -> Option<(f64, f64)>;

/// A map read from a file, and what could not be kept.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenedMap {
    pub map: Map,
    pub warnings: Vec<String>,
}

/// What the catalog knows that a saved file needs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SaveFacts {
    /// Door and window radii, in editor pixels, by asset path.
    pub portal_radii: HashMap<String, f64>,
    /// Every pack whose `pack.json` was read, to declare the ones the map uses.
    pub packs: Vec<PackInfo>,
}

/// Reads and writes maps in the editor's file format.
pub trait MapFiles: Send + Sync {
    /// # Errors
    /// Fails when the file cannot be read or understood.
    fn open(&self, source: &Path, footprint_of: FootprintOf) -> Result<OpenedMap, PortError>;

    /// Writes `map`, over `template` when given so that what the domain
    /// does not model is kept.
    ///
    /// # Errors
    /// Fails when the file cannot be produced or written.
    fn save(
        &self,
        map: &Map,
        facts: &SaveFacts,
        template: Option<&Path>,
        destination: &Path,
    ) -> Result<(), PortError>;
}

/// Draws a schematic preview of a map.
pub trait PreviewRenderer: Send + Sync {
    /// # Errors
    /// Fails when the preview cannot be drawn, typically when it is too large.
    fn render_png(&self, map: &Map, pixels_per_cell: u32) -> Result<Vec<u8>, PortError>;
}
