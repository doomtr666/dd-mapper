use thiserror::Error;

/// A grid vertex, in cells from the top-left corner; `y` grows downward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPoint {
    pub x: i32,
    pub y: i32,
}

impl CellPoint {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A grid square, by column and row from the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell {
    pub column: u32,
    pub row: u32,
}

impl Cell {
    #[must_use]
    pub const fn new(column: u32, row: u32) -> Self {
        Self { column, row }
    }
}

/// A position in cells, possibly between grid vertices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellPosition {
    pub x: f64,
    pub y: f64,
}

impl CellPosition {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Size of the map, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GridError {
    #[error("a map needs at least one cell, got {width}x{height}")]
    Empty { width: u32, height: u32 },
}

impl GridSize {
    /// # Errors
    /// Fails when either dimension is zero.
    pub fn new(width: u32, height: u32) -> Result<Self, GridError> {
        if width == 0 || height == 0 {
            return Err(GridError::Empty { width, height });
        }
        Ok(Self { width, height })
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    /// Whether the position lies on or inside the map border.
    #[must_use]
    pub fn contains_position(self, position: CellPosition) -> bool {
        let within = |coordinate: f64, limit: u32| (0.0..=f64::from(limit)).contains(&coordinate);
        within(position.x, self.width) && within(position.y, self.height)
    }

    /// Whether the grid vertex lies on or inside the map border.
    #[must_use]
    pub fn contains(self, point: CellPoint) -> bool {
        let within = |coordinate: i32, limit: u32| {
            u32::try_from(coordinate).is_ok_and(|coordinate| coordinate <= limit)
        };
        within(point.x, self.width) && within(point.y, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::{CellPoint, GridError, GridSize};

    #[test]
    fn rejects_an_empty_grid() {
        assert_eq!(
            GridSize::new(0, 20),
            Err(GridError::Empty {
                width: 0,
                height: 20
            })
        );
    }

    #[test]
    fn contains_every_vertex_up_to_the_border() {
        let grid = GridSize::new(35, 20).unwrap();
        assert!(grid.contains(CellPoint::new(0, 0)));
        assert!(grid.contains(CellPoint::new(35, 20)));
        assert!(!grid.contains(CellPoint::new(36, 0)));
        assert!(!grid.contains(CellPoint::new(0, -1)));
    }
}
