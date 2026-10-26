use thiserror::Error;

use crate::{AssetPath, CellPosition};

/// A light source: a texture cast around its center.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    pub asset: AssetPath,
    pub center: CellPosition,
    pub glow: Glow,
}

/// How far, how strong and what color a light shines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    range: f64,
    intensity: f64,
    color: u32,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LightError {
    #[error("a light needs a positive range in cells, got {0}")]
    InvalidRange(f64),
    #[error("a light needs a positive intensity, got {0}")]
    InvalidIntensity(f64),
}

impl Glow {
    /// `range` in cells; `color` as ARGB.
    ///
    /// # Errors
    /// Fails unless range and intensity are finite and positive.
    pub fn new(range: f64, intensity: f64, color: u32) -> Result<Self, LightError> {
        let positive = |value: f64| value.is_finite() && value > 0.0;
        if !positive(range) {
            return Err(LightError::InvalidRange(range));
        }
        if !positive(intensity) {
            return Err(LightError::InvalidIntensity(intensity));
        }
        Ok(Self {
            range,
            intensity,
            color,
        })
    }

    #[must_use]
    pub const fn range(self) -> f64 {
        self.range
    }

    #[must_use]
    pub const fn intensity(self) -> f64 {
        self.intensity
    }

    /// ARGB.
    #[must_use]
    pub const fn color(self) -> u32 {
        self.color
    }
}

#[cfg(test)]
mod tests {
    use super::{Glow, LightError};

    #[test]
    fn keeps_a_valid_glow() {
        let glow = Glow::new(4.4, 1.6, 0xffec_cd8b).unwrap();
        assert_eq!(
            (glow.range(), glow.intensity(), glow.color()),
            (4.4, 1.6, 0xffec_cd8b)
        );
    }

    #[test]
    fn rejects_empty_or_invalid_glows() {
        assert_eq!(Glow::new(0.0, 1.0, 0), Err(LightError::InvalidRange(0.0)));
        assert_eq!(
            Glow::new(1.0, -1.0, 0),
            Err(LightError::InvalidIntensity(-1.0))
        );
        assert!(Glow::new(f64::NAN, 1.0, 0).is_err());
    }
}
