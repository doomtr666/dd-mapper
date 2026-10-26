//! Numbers inside literals: reals are written like Godot's `rtosfix`
//! (C `%g`, 6 significant digits); integers and bytes in plain decimal.

use super::{CodecError, NumberKind};

const SIGNIFICANT_DIGITS: i32 = 6;
const MANTISSA_DECIMALS: usize = 5;
const MIN_FIXED_EXPONENT: i32 = -4;

/// Formats a real number the way Godot writes it inside variant literals.
///
/// Godot 3 uses C's `%g` with 6 significant digits, and writes `-0` as `0`.
#[must_use]
pub fn format_real(value: f64) -> String {
    if !value.is_finite() {
        return format_non_finite(value);
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    let scientific = format!("{value:.MANTISSA_DECIMALS$e}");
    let (mantissa, exponent) = split_scientific(&scientific);
    if (MIN_FIXED_EXPONENT..SIGNIFICANT_DIGITS).contains(&exponent) {
        format_fixed(value, exponent)
    } else {
        format_scientific(mantissa, exponent)
    }
}

fn format_non_finite(value: f64) -> String {
    let text = if value.is_nan() {
        "nan"
    } else if value > 0.0 {
        "inf"
    } else {
        "-inf"
    };
    text.to_owned()
}

fn split_scientific(scientific: &str) -> (&str, i32) {
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("Rust scientific formatting always contains an exponent");
    let exponent = exponent
        .parse()
        .expect("Rust scientific formatting always writes an integer exponent");
    (mantissa, exponent)
}

fn format_fixed(value: f64, exponent: i32) -> String {
    let decimals = usize::try_from(SIGNIFICANT_DIGITS - 1 - exponent)
        .expect("fixed notation is only used for exponents below the significant digit count");
    trim_fraction_zeros(&format!("{value:.decimals$}")).to_owned()
}

fn format_scientific(mantissa: &str, exponent: i32) -> String {
    let sign = if exponent < 0 { '-' } else { '+' };
    let mantissa = trim_fraction_zeros(mantissa);
    format!("{mantissa}e{sign}{:02}", exponent.abs())
}

/// Parses a finite real number; `inf` and `nan` are rejected.
pub fn parse_real(token: &str) -> Result<f64, CodecError> {
    token
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| invalid_number(NumberKind::Real, token))
}

pub fn parse_int(token: &str) -> Result<i32, CodecError> {
    token
        .parse()
        .map_err(|_| invalid_number(NumberKind::Int, token))
}

pub fn parse_byte(token: &str) -> Result<u8, CodecError> {
    token
        .parse()
        .map_err(|_| invalid_number(NumberKind::Byte, token))
}

fn invalid_number(expected: NumberKind, token: &str) -> CodecError {
    CodecError::InvalidNumber {
        expected,
        token: token.to_owned(),
    }
}

/// Removes trailing fractional zeros, and the dot when nothing is left after it.
pub(crate) fn trim_fraction_zeros(number: &str) -> &str {
    if !number.contains('.') {
        return number;
    }
    number.trim_end_matches('0').trim_end_matches('.')
}

#[cfg(test)]
mod tests {
    use super::{format_real, parse_byte, parse_int, parse_real};
    use crate::godot::{CodecError, NumberKind};

    fn invalid(expected: NumberKind, token: &str) -> CodecError {
        CodecError::InvalidNumber {
            expected,
            token: token.to_owned(),
        }
    }

    #[test]
    fn parses_fixed_and_exponent_reals() {
        assert_eq!(parse_real("5483.15"), Ok(5483.15));
        assert_eq!(parse_real("-4.4"), Ok(-4.4));
        assert_eq!(parse_real("1e+06"), Ok(1_000_000.0));
        assert_eq!(parse_real("1.25e-05"), Ok(0.000_012_5));
    }

    #[test]
    fn rejects_malformed_or_non_finite_reals() {
        for token in ["", "abc", "1,5", "inf", "-inf", "nan", "infinity"] {
            assert_eq!(parse_real(token), Err(invalid(NumberKind::Real, token)));
        }
    }

    #[test]
    fn parses_32_bit_integers() {
        assert_eq!(parse_int("-1"), Ok(-1));
        assert_eq!(parse_int("2147483647"), Ok(i32::MAX));
    }

    #[test]
    fn rejects_non_integers_and_overflow() {
        for token in ["", "1.5", "2147483648", "x"] {
            assert_eq!(parse_int(token), Err(invalid(NumberKind::Int, token)));
        }
    }

    #[test]
    fn parses_bytes() {
        assert_eq!(parse_byte("0"), Ok(0));
        assert_eq!(parse_byte("255"), Ok(255));
    }

    #[test]
    fn rejects_values_outside_byte_range() {
        for token in ["256", "-1", ""] {
            assert_eq!(parse_byte(token), Err(invalid(NumberKind::Byte, token)));
        }
    }

    #[test]
    fn whole_numbers_have_no_decimal_part() {
        assert_eq!(format_real(4480.0), "4480");
        assert_eq!(format_real(-256.0), "-256");
    }

    #[test]
    fn keeps_six_significant_digits() {
        assert_eq!(format_real(f64::from(5483.15_f32)), "5483.15");
        assert_eq!(format_real(10656.5), "10656.5");
        assert_eq!(format_real(std::f64::consts::FRAC_PI_2), "1.5708");
    }

    #[test]
    fn negative_zero_is_written_as_zero() {
        assert_eq!(format_real(-0.0), "0");
    }

    #[test]
    fn large_magnitudes_switch_to_exponent_notation() {
        assert_eq!(format_real(1_000_000.0), "1e+06");
        assert_eq!(format_real(1_234_567.0), "1.23457e+06");
    }

    #[test]
    fn small_magnitudes_switch_to_exponent_notation() {
        assert_eq!(format_real(0.0001), "0.0001");
        assert_eq!(format_real(0.000_012_5), "1.25e-05");
    }

    #[test]
    fn rounding_carry_is_applied_before_choosing_notation() {
        assert_eq!(format_real(999_999.5), "1e+06");
        assert_eq!(format_real(9.999_999), "10");
    }

    #[test]
    fn non_finite_values_are_written_like_printf() {
        assert_eq!(format_real(f64::INFINITY), "inf");
        assert_eq!(format_real(f64::NEG_INFINITY), "-inf");
        assert_eq!(format_real(f64::NAN), "nan");
    }
}
