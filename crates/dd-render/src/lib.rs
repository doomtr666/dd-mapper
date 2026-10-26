//! Schematic preview of a map, for a reader (human or LLM) to check a layout:
//! flat floors colored by tileset, walls, doors, props as boxes, labels.
//! No pack texture is used.

mod canvas;
mod font;
mod palette;
mod scene;

use thiserror::Error;

pub use scene::render;

/// Largest preview side, in pixels.
pub const MAX_SIDE_PX: u32 = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewOptions {
    pub pixels_per_cell: u32,
    pub grid: bool,
    /// Draws `R<n>`, `D<n>` and `P<n>` ids on rooms, doors and props.
    pub labels: bool,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            pixels_per_cell: 32,
            grid: true,
            labels: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RenderError {
    #[error(
        "a {width} x {height} px preview exceeds {MAX_SIDE_PX} px per side; lower pixels_per_cell"
    )]
    TooLarge { width: u64, height: u64 },
    #[error("cannot encode the preview as PNG: {0}")]
    Encoding(String),
}

/// A rendered preview.
#[derive(Debug, Clone)]
pub struct Preview {
    pub(crate) pixmap: tiny_skia::Pixmap,
}

impl Preview {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Color of a pixel, or `None` outside the image.
    #[must_use]
    pub fn rgb(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        let pixel = self.pixmap.pixel(x, y)?.demultiply();
        Some([pixel.red(), pixel.green(), pixel.blue()])
    }

    /// Raw RGBA bytes, row by row.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// # Errors
    /// Fails when PNG encoding fails.
    pub fn to_png(&self) -> Result<Vec<u8>, RenderError> {
        self.pixmap
            .encode_png()
            .map_err(|error| RenderError::Encoding(error.to_string()))
    }
}
