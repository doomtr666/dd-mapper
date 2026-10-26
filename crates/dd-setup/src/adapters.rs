//! Adapters plugging the format and render crates into the application ports.

use std::collections::HashMap;
use std::path::Path;

use dd_app::{FootprintOf, MapFiles, OpenedMap, PortError, PreviewRenderer};
use dd_domain::Map;
use dd_format::FormatError;
use dd_format::export::{ExportSettings, export_map};
use dd_format::import::import_map;
use dd_format::model::MapFile;
use dd_render::{PreviewOptions, render};

/// A map saved by Dungeondraft 1.2, used as the skeleton of new maps.
const DEFAULT_TEMPLATE: &str = include_str!("../templates/default.dungeondraft_map");

/// Reads and writes `.dungeondraft_map` files.
pub struct DungeondraftFiles {
    default_template: MapFile,
}

impl DungeondraftFiles {
    /// New maps are written over `default_template`.
    #[must_use]
    pub fn new(default_template: MapFile) -> Self {
        Self { default_template }
    }

    /// # Errors
    /// Fails if the embedded template cannot be parsed.
    pub fn with_default_template() -> Result<Self, FormatError> {
        Ok(Self::new(MapFile::parse(DEFAULT_TEMPLATE)?))
    }
}

impl MapFiles for DungeondraftFiles {
    fn open(&self, source: &Path, footprint_of: FootprintOf) -> Result<OpenedMap, PortError> {
        let file = read_map_file(source)?;
        let imported = import_map(&file, footprint_of)
            .map_err(|error| PortError(format!("cannot import {}: {error}", source.display())))?;
        Ok(OpenedMap {
            map: imported.map,
            warnings: imported.warnings,
        })
    }

    fn save(
        &self,
        map: &Map,
        portal_radii: &HashMap<String, f64>,
        template: Option<&Path>,
        destination: &Path,
    ) -> Result<(), PortError> {
        let template = match template {
            Some(origin) => read_map_file(origin)?,
            None => self.default_template.clone(),
        };
        let settings = ExportSettings {
            portal_radii: portal_radii.clone(),
            ..ExportSettings::default()
        };
        let exported =
            export_map(map, &template, &settings).map_err(|error| PortError(error.to_string()))?;
        let text = exported
            .to_text()
            .map_err(|error| PortError(error.to_string()))?;
        std::fs::write(destination, text)
            .map_err(|error| PortError(format!("cannot write {}: {error}", destination.display())))
    }
}

fn read_map_file(path: &Path) -> Result<MapFile, PortError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| PortError(format!("cannot read {}: {error}", path.display())))?;
    MapFile::parse(&text).map_err(|error| PortError(format!("{}: {error}", path.display())))
}

/// Draws schematic previews with `dd-render`.
pub struct SchematicRenderer;

impl PreviewRenderer for SchematicRenderer {
    fn render_png(&self, map: &Map, pixels_per_cell: u32) -> Result<Vec<u8>, PortError> {
        let options = PreviewOptions {
            pixels_per_cell,
            ..PreviewOptions::default()
        };
        render(map, &options)
            .and_then(|preview| preview.to_png())
            .map_err(|error| PortError(error.to_string()))
    }
}
