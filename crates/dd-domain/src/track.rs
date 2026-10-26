use thiserror::Error;

use crate::{AssetPath, CellPosition};

/// A texture drawn along a line: conveyor belts, cables, pipes, ropes,
/// worn trails. Dungeondraft calls them paths.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    asset: AssetPath,
    points: Vec<CellPosition>,
    width: f64,
    closed: bool,
    /// Curved through its points, as Dungeondraft draws paths by default;
    /// sharp corners suit man-made lines such as hazard stripes.
    smooth: bool,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum TrackError {
    #[error("a path needs at least 2 points, got {0}")]
    TooFewPoints(usize),
    #[error("a path width must be positive, got {0} cells")]
    InvalidWidth(f64),
}

impl Track {
    /// A track through `points`, `width` cells wide, back to its start when `closed`.
    ///
    /// # Errors
    /// Fails with fewer than two points or a width that is not positive.
    pub fn new(
        asset: AssetPath,
        points: Vec<CellPosition>,
        width: f64,
        closed: bool,
    ) -> Result<Self, TrackError> {
        if points.len() < 2 {
            return Err(TrackError::TooFewPoints(points.len()));
        }
        if !(width.is_finite() && width > 0.0) {
            return Err(TrackError::InvalidWidth(width));
        }
        Ok(Self {
            asset,
            points,
            width,
            closed,
            smooth: true,
        })
    }

    /// The same track with straight segments and sharp corners.
    #[must_use]
    pub fn with_sharp_corners(self) -> Self {
        Self {
            smooth: false,
            ..self
        }
    }

    #[must_use]
    pub fn smooth(&self) -> bool {
        self.smooth
    }

    #[must_use]
    pub fn asset(&self) -> &AssetPath {
        &self.asset
    }

    #[must_use]
    pub fn points(&self) -> &[CellPosition] {
        &self.points
    }

    #[must_use]
    pub fn width(&self) -> f64 {
        self.width
    }

    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }
}

#[cfg(test)]
mod tests {
    use super::{Track, TrackError};
    use crate::{AssetPath, CellPosition};

    fn belt() -> AssetPath {
        AssetPath::new(
            "res://packs/LOREGD01/textures/paths/Artificial/Path_Conveyor_Belt_Large.webp",
        )
        .unwrap()
    }

    fn points(coordinates: &[(f64, f64)]) -> Vec<CellPosition> {
        coordinates
            .iter()
            .map(|&(x, y)| CellPosition::new(x, y))
            .collect()
    }

    #[test]
    fn runs_through_its_points() {
        let track = Track::new(belt(), points(&[(1.0, 2.0), (6.5, 2.0)]), 1.5, false).unwrap();
        assert_eq!(track.points(), points(&[(1.0, 2.0), (6.5, 2.0)]));
        assert!((track.width() - 1.5).abs() < f64::EPSILON);
        assert!(!track.closed());
    }

    #[test]
    fn curves_unless_asked_for_sharp_corners() {
        let track = Track::new(belt(), points(&[(1.0, 2.0), (6.5, 2.0)]), 1.5, false).unwrap();
        assert!(track.smooth());
        assert!(!track.with_sharp_corners().smooth());
    }

    #[test]
    fn needs_two_points_and_a_width() {
        assert_eq!(
            Track::new(belt(), points(&[(1.0, 2.0)]), 1.0, false),
            Err(TrackError::TooFewPoints(1))
        );
        assert_eq!(
            Track::new(belt(), points(&[(1.0, 2.0), (3.0, 2.0)]), 0.0, false),
            Err(TrackError::InvalidWidth(0.0))
        );
        assert!(matches!(
            Track::new(belt(), points(&[(1.0, 2.0), (3.0, 2.0)]), f64::NAN, true),
            Err(TrackError::InvalidWidth(_))
        ));
    }
}
