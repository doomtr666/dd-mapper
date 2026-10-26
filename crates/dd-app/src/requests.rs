//! Requests, in cells, as adapters receive them.

use dd_domain::{CellPoint, Polygon, PolygonError};

/// An area of the grid, given by its corners on grid vertices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Rectangle {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    Polygon(Vec<(i32, i32)>),
}

impl Shape {
    /// # Errors
    /// Fails when the shape is not a simple polygon with an area.
    pub fn to_polygon(&self) -> Result<Polygon, PolygonError> {
        match self {
            Self::Rectangle {
                x,
                y,
                width,
                height,
            } => Polygon::rectangle(CellPoint::new(*x, *y), *width, *height),
            Self::Polygon(corners) => {
                Polygon::new(corners.iter().map(|&(x, y)| CellPoint::new(x, y)).collect())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomRequest {
    pub shape: Shape,
    pub pillars: Vec<Shape>,
    /// Asset path of the floor tileset.
    pub floor: String,
    /// Asset path of the wall texture.
    pub wall: String,
}

/// A texture drawn along a line through `points`: conveyor belts, cables,
/// pipes, trails. Points need not sit on grid vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct PathRequest {
    pub asset: String,
    pub points: Vec<(f64, f64)>,
    /// In cells; one cell when unset.
    pub width: Option<f64>,
    /// Back to the first point, such as a conveyor loop.
    pub closed: bool,
    /// Straight segments and sharp corners, for hazard stripes, fences and
    /// pipes; paths curve through their points otherwise.
    pub sharp: bool,
}

/// A floor tileset painted over an area, such as one room of a building.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorRequest {
    pub shape: Shape,
    /// Asset path of the floor tileset.
    pub tileset: String,
}

/// A free-standing wall through `points`, such as a partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallRequest {
    pub points: Vec<(i32, i32)>,
    /// Asset path of the wall texture.
    pub wall: String,
}

/// A door in the walls of `on` (a room `R0` or a wall `W0`),
/// at the wall point nearest to `(x, y)`.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorRequest {
    pub on: String,
    pub x: f64,
    pub y: f64,
    pub asset: String,
}

/// A light centered on `(x, y)`; unset settings take warm, room-sized defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct LightRequest {
    pub asset: String,
    pub x: f64,
    pub y: f64,
    /// Radius in cells.
    pub range: Option<f64>,
    pub intensity: Option<f64>,
    /// `RRGGBB` or `AARRGGBB`.
    pub color: Option<String>,
}

/// A texture painted over an area of the floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternRequest {
    pub shape: Shape,
    pub asset: String,
    /// `RRGGBB` or `AARRGGBB` tint; none keeps the texture as is.
    pub color: Option<String>,
}

/// A prop centered on `(x, y)`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropRequest {
    pub asset: String,
    pub x: f64,
    pub y: f64,
    pub rotation_degrees: f64,
    /// `RRGGBB` or `AARRGGBB` custom color, for assets Dungeondraft lets color.
    pub color: Option<String>,
}
