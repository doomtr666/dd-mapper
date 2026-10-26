use std::f64::consts::TAU;

use thiserror::Error;

use crate::{AssetPath, CellPosition, Polygon};

/// Points of a circle's outline: smooth at any size a map can hold.
const CIRCLE_POINTS: u32 = 64;
const MIN_POINTS: usize = 3;

/// A texture painted over an area of the floor: grates, fans, markings, a
/// blast cut clean through the ground...
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub asset: AssetPath,
    pub area: FreeShape,
    /// ARGB tint; opaque white keeps the texture as is.
    pub tint: u32,
    /// Dungeondraft draws a dark line along the edge.
    pub outlined: bool,
}

/// A closed outline whose points need not sit on grid vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeShape(Vec<CellPosition>);

#[derive(Debug, Clone, PartialEq, Error)]
pub enum FreeShapeError {
    #[error("a circle needs a positive radius, got {0}")]
    InvalidRadius(f64),
    #[error("an outline needs at least {MIN_POINTS} finite points, got {count}")]
    TooFewPoints { count: usize },
}

impl FreeShape {
    /// # Errors
    /// Fails with fewer than three points, or points that are not finite.
    pub fn new(points: Vec<CellPosition>) -> Result<Self, FreeShapeError> {
        let finite = points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite());
        if points.len() < MIN_POINTS || !finite {
            return Err(FreeShapeError::TooFewPoints {
                count: points.len(),
            });
        }
        Ok(Self(points))
    }

    /// A circle around `center`, drawn with enough points to look round.
    ///
    /// # Errors
    /// Fails unless the radius is finite and positive.
    pub fn circle(center: CellPosition, radius: f64) -> Result<Self, FreeShapeError> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(FreeShapeError::InvalidRadius(radius));
        }
        let points = (0..CIRCLE_POINTS)
            .map(|index| {
                let angle = TAU * f64::from(index) / f64::from(CIRCLE_POINTS);
                CellPosition::new(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                )
            })
            .collect();
        Self::new(points)
    }

    #[must_use]
    pub fn points(&self) -> &[CellPosition] {
        &self.0
    }

    /// Top-left and bottom-right corners of the box around the outline.
    #[must_use]
    pub fn bounds(&self) -> (CellPosition, CellPosition) {
        let fold = |pick: fn(&CellPosition) -> f64, start: f64, keep: fn(f64, f64) -> f64| {
            self.0.iter().map(pick).fold(start, keep)
        };
        (
            CellPosition::new(
                fold(|point| point.x, f64::INFINITY, f64::min),
                fold(|point| point.y, f64::INFINITY, f64::min),
            ),
            CellPosition::new(
                fold(|point| point.x, f64::NEG_INFINITY, f64::max),
                fold(|point| point.y, f64::NEG_INFINITY, f64::max),
            ),
        )
    }
}

impl From<&Polygon> for FreeShape {
    fn from(polygon: &Polygon) -> Self {
        Self(
            polygon
                .vertices()
                .iter()
                .map(|vertex| CellPosition::new(f64::from(vertex.x), f64::from(vertex.y)))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{FreeShape, FreeShapeError};
    use crate::{CellPoint, CellPosition, Polygon};

    #[test]
    fn circles_are_round_and_centered() {
        let center = CellPosition::new(30.0, 24.0);
        let circle = FreeShape::circle(center, 6.5).unwrap();
        assert!(circle.points().len() >= 48);
        for point in circle.points() {
            let distance = (point.x - center.x).hypot(point.y - center.y);
            assert!((distance - 6.5).abs() < 1e-9, "{point:?}");
        }
        let (low, high) = circle.bounds();
        assert!((low.x - 23.5).abs() < 1e-9 && (high.y - 30.5).abs() < 1e-3);
    }

    #[test]
    fn circles_need_a_positive_radius() {
        for radius in [0.0, -1.0, f64::NAN] {
            assert!(matches!(
                FreeShape::circle(CellPosition::new(1.0, 1.0), radius),
                Err(FreeShapeError::InvalidRadius(_))
            ));
        }
    }

    #[test]
    fn outlines_need_three_finite_points() {
        let points = |list: &[(f64, f64)]| {
            list.iter()
                .map(|&(x, y)| CellPosition::new(x, y))
                .collect::<Vec<_>>()
        };
        assert!(FreeShape::new(points(&[(0.0, 0.0), (1.5, 0.0), (0.5, 1.2)])).is_ok());
        assert_eq!(
            FreeShape::new(points(&[(0.0, 0.0), (1.0, 0.0)])),
            Err(FreeShapeError::TooFewPoints { count: 2 })
        );
        assert!(FreeShape::new(points(&[(0.0, 0.0), (f64::NAN, 0.0), (0.5, 1.0)])).is_err());
    }

    #[test]
    fn grid_polygons_keep_their_corners() {
        let polygon = Polygon::rectangle(CellPoint::new(2, 3), 2, 1).unwrap();
        let shape = FreeShape::from(&polygon);
        assert_eq!(shape.points().len(), 4);
        assert_eq!(
            shape.bounds(),
            (CellPosition::new(2.0, 3.0), CellPosition::new(4.0, 4.0))
        );
    }
}
