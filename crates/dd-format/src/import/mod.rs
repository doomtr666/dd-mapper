//! Reads a `.dungeondraft_map` into the domain, warning about what it cannot keep.

mod cells;
mod content;
mod doors;
mod walls;

use dd_domain::{GridError, GridSize, Map};
use thiserror::Error;

use crate::model::{MapFile, World};

const GROUND_LEVEL: &str = "0";

/// Size in cells of an object texture, when known.
pub type FootprintOf<'a> = &'a dyn Fn(&str) -> Option<(f64, f64)>;

/// A domain map read from a file, and what was lost or approximated on the way.
#[derive(Debug, Clone)]
pub struct ImportedMap {
    pub map: Map,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImportError {
    #[error("the map has no ground level \"0\"")]
    MissingGroundLevel,
    #[error(transparent)]
    Grid(#[from] GridError),
}

/// Imports level 0 of `file`.
///
/// # Errors
/// Fails when the map has no ground level or an empty grid.
pub fn import_map(file: &MapFile, footprint_of: FootprintOf) -> Result<ImportedMap, ImportError> {
    let world = file.world();
    let level = world
        .levels
        .get(GROUND_LEVEL)
        .ok_or(ImportError::MissingGroundLevel)?;
    let mut importer = Importer {
        map: Map::new(GridSize::new(world.width, world.height)?),
        warnings: other_levels(world),
    };
    walls::import_walls(&mut importer, level);
    doors::import_doors(&mut importer, level);
    content::import_floor(&mut importer, level);
    content::import_props(&mut importer, level, footprint_of);
    content::import_lights(&mut importer, level);
    content::import_patterns(&mut importer, level);
    content::import_tracks(&mut importer, level);
    content::import_cave(&mut importer, level);
    Ok(ImportedMap {
        map: importer.map,
        warnings: importer.warnings,
    })
}

/// The map being built and the warnings gathered so far.
struct Importer {
    map: Map,
    warnings: Vec<String>,
}

impl Importer {
    fn warn(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

fn other_levels(world: &World) -> Vec<String> {
    let others: Vec<&str> = world
        .levels
        .keys()
        .map(String::as_str)
        .filter(|level| *level != GROUND_LEVEL)
        .collect();
    if others.is_empty() {
        return Vec::new();
    }
    vec![format!(
        "only level 0 is imported; levels {} stay in the file untouched",
        others.join(", ")
    )]
}
