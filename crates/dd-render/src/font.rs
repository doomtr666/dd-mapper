//! A 5 x 7 bitmap font, enough for ids such as `R0`, `D12`, `P3`, `W1` or `L2`.

const GLYPH_HEIGHT: usize = 7;
const GLYPH_WIDTH: u32 = 5;
/// Glyph width plus one column of spacing.
pub const ADVANCE: u32 = GLYPH_WIDTH + 1;
pub const HEIGHT: u32 = 7;

/// Each row is 5 bits, the leftmost dot in the highest bit.
type Glyph = [u8; GLYPH_HEIGHT];

fn glyph(character: char) -> Option<Glyph> {
    let rows = match character {
        '0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        '1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
        '2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        '3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
        '4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        '5' => [0x1f, 0x10, 0x1e, 0x01, 0x01, 0x11, 0x0e],
        '6' => [0x06, 0x08, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        '7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        '9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x02, 0x0c],
        'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0a],
        _ => return None,
    };
    Some(rows)
}

/// Width of `text` in dots, without trailing spacing.
pub fn text_width(text: &str) -> u32 {
    let characters = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
    (characters * ADVANCE).saturating_sub(1)
}

/// Positions `(column, row)` of the lit dots of `text`, from its top-left corner.
/// Unknown characters are left blank.
pub fn lit_dots(text: &str) -> Vec<(u32, u32)> {
    let mut dots = Vec::new();
    for (index, character) in (0_u32..).zip(text.chars()) {
        let Some(rows) = glyph(character) else {
            continue;
        };
        for (row, bits) in (0_u32..).zip(rows) {
            let lit_columns =
                (0..GLYPH_WIDTH).filter(|column| bits & (1 << (GLYPH_WIDTH - 1 - column)) != 0);
            dots.extend(lit_columns.map(|column| (index * ADVANCE + column, row)));
        }
    }
    dots
}

#[cfg(test)]
mod tests {
    use super::{lit_dots, text_width};

    #[test]
    fn measures_text_without_trailing_spacing() {
        assert_eq!(text_width(""), 0);
        assert_eq!(text_width("P"), 5);
        assert_eq!(text_width("P12"), 17);
    }

    #[test]
    fn lights_the_dots_of_each_glyph() {
        let one = lit_dots("1");
        assert_eq!(one.len(), 1 + 2 + 1 + 1 + 1 + 1 + 3);
        assert!(one.contains(&(2, 0)));
        assert!(one.contains(&(1, 1)));
        let shifted = lit_dots("11");
        assert!(shifted.contains(&(6 + 2, 0)));
    }

    #[test]
    fn unknown_characters_are_blank() {
        assert!(lit_dots("?").is_empty());
    }
}
