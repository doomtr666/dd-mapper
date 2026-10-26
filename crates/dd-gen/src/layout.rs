use dd_domain::{CellPoint, CellPosition};

/// A rectangular area of the layout, such as a room, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Where a door goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorSpot {
    /// On the building outline, such as an entrance.
    Outline(CellPosition),
    /// On partition `index` of the layout.
    Partition { index: usize, at: CellPosition },
}

/// A building: its outline, the partitions splitting it, the doors joining
/// the parts, and the resulting areas.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub outline: Vec<CellPoint>,
    pub partitions: Vec<Vec<CellPoint>>,
    pub doors: Vec<DoorSpot>,
    pub areas: Vec<Area>,
}

impl Area {
    #[must_use]
    pub fn contains(&self, position: CellPosition) -> bool {
        let (left, top) = (f64::from(self.x), f64::from(self.y));
        let right = left + f64::from(self.width);
        let bottom = top + f64::from(self.height);
        position.x > left && position.x < right && position.y > top && position.y < bottom
    }

    /// The bottom-right corner.
    #[must_use]
    pub fn far_corner(&self) -> (i32, i32) {
        (
            self.x + self.width.cast_signed(),
            self.y + self.height.cast_signed(),
        )
    }

    /// The four corners, clockwise on screen from the top-left one.
    #[must_use]
    pub fn corners(&self) -> Vec<CellPoint> {
        let (right, bottom) = self.far_corner();
        vec![
            CellPoint::new(self.x, self.y),
            CellPoint::new(right, self.y),
            CellPoint::new(right, bottom),
            CellPoint::new(self.x, bottom),
        ]
    }

    #[must_use]
    pub fn cells(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}
