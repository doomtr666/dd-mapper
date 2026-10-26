//! Adapters feeding the asset catalog.

mod harvester;
mod image_size;
mod pack_source;
mod pck;
mod toml_source;

pub use harvester::{MapHarvester, harvest};
pub use image_size::{PixelSize, image_size};
pub use pack_source::{PackError, PackIndexSource, assets_from_archive};
pub use pck::{PckEntry, PckError, PckIndex};
pub use toml_source::{TomlCatalogError, TomlCatalogSource, assets_from_toml};
