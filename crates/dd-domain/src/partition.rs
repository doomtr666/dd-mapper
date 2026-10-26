use thiserror::Error;

use crate::{AssetPath, CellPoint, Segment};

/// A free-standing wall, such as a partition inside a room: an open polyline
/// on grid vertices. Doors can be set in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    points: Vec<CellPoint>,
    wall: AssetPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PartitionError {
    #[error("a wall needs at least 2 distinct points, got {count}")]
    TooFewPoints { count: usize },
}

impl Partition {
    /// Repeated consecutive points are dropped.
    ///
    /// # Errors
    /// Fails when fewer than two distinct points remain.
    pub fn new(points: Vec<CellPoint>, wall: AssetPath) -> Result<Self, PartitionError> {
        let mut distinct = points;
        distinct.dedup();
        if distinct.len() < 2 {
            return Err(PartitionError::TooFewPoints {
                count: distinct.len(),
            });
        }
        Ok(Self {
            points: distinct,
            wall,
        })
    }

    #[must_use]
    pub fn points(&self) -> &[CellPoint] {
        &self.points
    }

    #[must_use]
    pub fn wall(&self) -> &AssetPath {
        &self.wall
    }

    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.points.len() - 1
    }

    /// The segment from point `index` to the next one; the line does not close.
    #[must_use]
    pub fn segment(&self, index: usize) -> Option<Segment> {
        let start = *self.points.get(index)?;
        let end = *self.points.get(index + 1)?;
        Some(Segment { start, end })
    }

    pub fn segments(&self) -> impl Iterator<Item = Segment> + '_ {
        (0..self.segment_count()).filter_map(|index| self.segment(index))
    }
}

#[cfg(test)]
mod tests {
    use super::{Partition, PartitionError};
    use crate::{AssetPath, CellPoint, Segment};

    fn points(coordinates: &[(i32, i32)]) -> Vec<CellPoint> {
        coordinates
            .iter()
            .map(|&(x, y)| CellPoint::new(x, y))
            .collect()
    }

    fn wall() -> AssetPath {
        AssetPath::new("wall.webp").unwrap()
    }

    #[test]
    fn keeps_an_open_polyline() {
        let partition = Partition::new(points(&[(8, 3), (8, 7), (8, 7), (12, 7)]), wall()).unwrap();
        assert_eq!(partition.points(), points(&[(8, 3), (8, 7), (12, 7)]));
        assert_eq!(partition.segment_count(), 2);
        assert_eq!(
            partition.segment(1),
            Some(Segment {
                start: CellPoint::new(8, 7),
                end: CellPoint::new(12, 7)
            })
        );
        assert_eq!(partition.segment(2), None);
    }

    #[test]
    fn needs_two_distinct_points() {
        assert_eq!(
            Partition::new(points(&[(1, 1), (1, 1)]), wall()),
            Err(PartitionError::TooFewPoints { count: 1 })
        );
    }
}
