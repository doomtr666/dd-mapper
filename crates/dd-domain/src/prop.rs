use thiserror::Error;

use crate::{AssetPath, CellPosition};

/// Rotation of a prop, by quarter turns, clockwise on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuarterTurn {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

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
    pub rotation: QuarterTurn,
    pub footprint: Footprint,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum PropError {
    #[error("props turn by quarter turns: use 0, 90, 180 or 270 degrees, got {0}")]
    NotAQuarterTurn(i32),
    #[error("a footprint needs a positive width and height, got {width} x {height}")]
    InvalidFootprint { width: f64, height: f64 },
}

impl QuarterTurn {
    /// Any multiple of 90 degrees, negative or beyond a full turn.
    ///
    /// # Errors
    /// Fails when `degrees` is not a multiple of 90.
    pub fn from_degrees(degrees: i32) -> Result<Self, PropError> {
        match degrees.rem_euclid(360) {
            0 => Ok(Self::Deg0),
            90 => Ok(Self::Deg90),
            180 => Ok(Self::Deg180),
            270 => Ok(Self::Deg270),
            _ => Err(PropError::NotAQuarterTurn(degrees)),
        }
    }

    #[must_use]
    pub const fn degrees(self) -> u16 {
        match self {
            Self::Deg0 => 0,
            Self::Deg90 => 90,
            Self::Deg180 => 180,
            Self::Deg270 => 270,
        }
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

    /// The footprint as seen once turned: sides swap on 90 and 270 degrees.
    #[must_use]
    pub fn turned(self, rotation: QuarterTurn) -> Self {
        match rotation {
            QuarterTurn::Deg0 | QuarterTurn::Deg180 => self,
            QuarterTurn::Deg90 | QuarterTurn::Deg270 => Self {
                width: self.height,
                height: self.width,
            },
        }
    }
}

impl Prop {
    /// Top-left and bottom-right corners of the space the prop takes.
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
    use super::{Footprint, Prop, PropError, QuarterTurn};
    use crate::{AssetPath, CellPosition};

    #[test]
    fn accepts_any_multiple_of_ninety_degrees() {
        assert_eq!(QuarterTurn::from_degrees(0), Ok(QuarterTurn::Deg0));
        assert_eq!(QuarterTurn::from_degrees(90), Ok(QuarterTurn::Deg90));
        assert_eq!(QuarterTurn::from_degrees(-90), Ok(QuarterTurn::Deg270));
        assert_eq!(QuarterTurn::from_degrees(540), Ok(QuarterTurn::Deg180));
        assert_eq!(
            QuarterTurn::from_degrees(45),
            Err(PropError::NotAQuarterTurn(45))
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
    fn quarter_turns_swap_the_sides() {
        let footprint = Footprint::new(2.0, 1.0).unwrap();
        assert_eq!(footprint.turned(QuarterTurn::Deg180), footprint);
        assert_eq!(
            footprint.turned(QuarterTurn::Deg270),
            Footprint::new(1.0, 2.0).unwrap()
        );
    }

    #[test]
    fn bounds_follow_the_rotation() {
        let prop = Prop {
            asset: AssetPath::new("crate.webp").unwrap(),
            center: CellPosition::new(5.0, 5.0),
            rotation: QuarterTurn::Deg90,
            footprint: Footprint::new(2.0, 1.0).unwrap(),
        };
        assert_eq!(
            prop.bounds(),
            (CellPosition::new(4.5, 4.0), CellPosition::new(5.5, 6.0))
        );
    }
}
