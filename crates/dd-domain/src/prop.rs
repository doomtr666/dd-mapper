use thiserror::Error;

use crate::{AssetPath, CellPosition};

/// Below this, a sine or cosine counts as zero, so quarter turns stay exact.
const TRIG_EPSILON: f64 = 1e-9;
const FULL_TURN: f64 = 360.0;

/// Rotation of a prop in degrees, clockwise on screen, in `[0, 360[`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation(f64);

/// Size of a prop before rotation, in cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Footprint {
    width: f64,
    height: f64,
}

/// A placed asset: furniture, crate, decoration...
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub asset: AssetPath,
    pub center: CellPosition,
    pub rotation: Rotation,
    pub footprint: Footprint,
    /// ARGB custom color, for assets Dungeondraft lets color.
    pub tint: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum PropError {
    #[error("a rotation needs a finite number of degrees, got {0}")]
    InvalidRotation(f64),
    #[error("a footprint needs a positive width and height, got {width} x {height}")]
    InvalidFootprint { width: f64, height: f64 },
}

impl Rotation {
    pub const NONE: Self = Self(0.0);
    pub const QUARTER: Self = Self(90.0);
    pub const HALF: Self = Self(180.0);
    pub const THREE_QUARTERS: Self = Self(270.0);

    /// Any angle, negative or beyond a full turn.
    ///
    /// # Errors
    /// Fails when `degrees` is not finite.
    pub fn from_degrees(degrees: f64) -> Result<Self, PropError> {
        if !degrees.is_finite() {
            return Err(PropError::InvalidRotation(degrees));
        }
        let turned = degrees.rem_euclid(FULL_TURN);
        Ok(Self(if turned >= FULL_TURN { 0.0 } else { turned }))
    }

    #[must_use]
    pub const fn degrees(self) -> f64 {
        self.0
    }

    /// Sine and cosine, with the values of quarter turns made exact.
    fn sin_cos(self) -> (f64, f64) {
        let exact = |value: f64| {
            if value.abs() < TRIG_EPSILON {
                0.0
            } else {
                value
            }
        };
        let (sin, cos) = self.0.to_radians().sin_cos();
        (exact(sin), exact(cos))
    }
}

impl Footprint {
    /// # Errors
    /// Fails unless both sides are finite and positive.
    pub fn new(width: f64, height: f64) -> Result<Self, PropError> {
        let positive = |side: f64| side.is_finite() && side > 0.0;
        if !(positive(width) && positive(height)) {
            return Err(PropError::InvalidFootprint { width, height });
        }
        Ok(Self { width, height })
    }

    #[must_use]
    pub const fn width(self) -> f64 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> f64 {
        self.height
    }

    /// The upright box holding the footprint once turned: sides swap on 90
    /// and 270 degrees, and grow in between.
    #[must_use]
    pub fn turned(self, rotation: Rotation) -> Self {
        let (sin, cos) = rotation.sin_cos();
        Self {
            width: (self.width * cos).abs() + (self.height * sin).abs(),
            height: (self.width * sin).abs() + (self.height * cos).abs(),
        }
    }
}

impl Prop {
    /// Top-left and bottom-right corners of the upright box holding the prop.
    #[must_use]
    pub fn bounds(&self) -> (CellPosition, CellPosition) {
        let turned = self.footprint.turned(self.rotation);
        let (half_width, half_height) = (turned.width / 2.0, turned.height / 2.0);
        (
            CellPosition::new(self.center.x - half_width, self.center.y - half_height),
            CellPosition::new(self.center.x + half_width, self.center.y + half_height),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Footprint, Prop, PropError, Rotation};
    use crate::{AssetPath, CellPosition};

    #[test]
    fn rotations_wrap_into_one_turn() {
        let turn = |value: f64| Rotation::from_degrees(value).unwrap();
        assert_eq!(turn(0.0), Rotation::NONE);
        assert_eq!(turn(-90.0), Rotation::THREE_QUARTERS);
        assert_eq!(turn(540.0), Rotation::HALF);
        assert_eq!(turn(37.5), Rotation(37.5));
        assert_eq!(turn(-1e-20), Rotation::NONE);
        assert_eq!(
            Rotation::from_degrees(f64::NAN).unwrap_err().to_string(),
            PropError::InvalidRotation(f64::NAN).to_string()
        );
    }

    #[test]
    fn footprints_need_positive_sides() {
        assert!(Footprint::new(1.8, 1.5).is_ok());
        for (width, height) in [
            (0.0, 1.0),
            (1.0, -2.0),
            (f64::NAN, 1.0),
            (f64::INFINITY, 1.0),
        ] {
            assert!(Footprint::new(width, height).is_err(), "{width} x {height}");
        }
    }

    #[test]
    fn quarter_turns_swap_the_sides_exactly() {
        let footprint = Footprint::new(2.0, 1.0).unwrap();
        assert_eq!(footprint.turned(Rotation::HALF), footprint);
        assert_eq!(
            footprint.turned(Rotation::THREE_QUARTERS),
            Footprint::new(1.0, 2.0).unwrap()
        );
    }

    #[test]
    fn other_turns_take_a_larger_box() {
        let turned = Footprint::new(2.0, 1.0)
            .unwrap()
            .turned(Rotation::from_degrees(45.0).unwrap());
        let side = 3.0 * std::f64::consts::FRAC_1_SQRT_2;
        assert!((turned.width() - side).abs() < 1e-12, "{turned:?}");
        assert!((turned.height() - side).abs() < 1e-12, "{turned:?}");
    }

    #[test]
    fn bounds_follow_the_rotation() {
        let prop = Prop {
            asset: AssetPath::new("crate.webp").unwrap(),
            center: CellPosition::new(5.0, 5.0),
            rotation: Rotation::QUARTER,
            footprint: Footprint::new(2.0, 1.0).unwrap(),
            tint: None,
        };
        assert_eq!(
            prop.bounds(),
            (CellPosition::new(4.5, 4.0), CellPosition::new(5.5, 6.0))
        );
    }
}
