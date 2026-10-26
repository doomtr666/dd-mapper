//! Colors written by users: `RRGGBB` or `AARRGGBB`, with an optional `#`.

use crate::AppError;

const RGB_DIGITS: usize = 6;
const ARGB_DIGITS: usize = 8;
/// Full alpha, for colors given without one.
const OPAQUE: u32 = 0xff00_0000;

/// ARGB value of `text`; six digits mean an opaque color.
///
/// # Errors
/// Fails unless `text` holds 6 or 8 hexadecimal digits.
pub fn parse_color(text: &str) -> Result<u32, AppError> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    let hexadecimal = digits.chars().all(|digit| digit.is_ascii_hexdigit());
    let value = u32::from_str_radix(digits, 16).ok().filter(|_| hexadecimal);
    match (digits.len(), value) {
        (RGB_DIGITS, Some(rgb)) => Ok(OPAQUE | rgb),
        (ARGB_DIGITS, Some(argb)) => Ok(argb),
        _ => Err(AppError::InvalidColor(text.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_color;
    use crate::AppError;

    #[test]
    fn reads_argb_and_opaque_rgb() {
        assert_eq!(parse_color("ffeccd8b"), Ok(0xffec_cd8b));
        assert_eq!(parse_color("#ECCD8B"), Ok(0xffec_cd8b));
        assert_eq!(parse_color("7f80beff"), Ok(0x7f80_beff));
    }

    #[test]
    fn rejects_other_texts() {
        for text in ["", "red", "eccd8", "+fffffff", "#ffeccd8b00"] {
            assert_eq!(
                parse_color(text),
                Err(AppError::InvalidColor(text.to_owned())),
                "{text}"
            );
        }
    }
}
