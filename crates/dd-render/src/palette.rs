//! Colors of the schematic preview.

use dd_domain::AssetPath;

pub type Rgb = [u8; 3];

pub const VOID: Rgb = [30, 30, 34];
pub const WALL: Rgb = [12, 12, 12];
pub const PILLAR: Rgb = [70, 64, 58];
pub const DOOR: Rgb = [235, 140, 30];
pub const LIGHT: Rgb = [255, 214, 90];
/// Light ranges tint what they cross.
pub const LIGHT_RANGE_ALPHA: u8 = 140;
pub const PROP_FILL: Rgb = [90, 140, 220];
pub const PROP_OUTLINE: Rgb = [25, 60, 140];
pub const LABEL_TEXT: Rgb = [255, 255, 255];
pub const LABEL_BACKGROUND: Rgb = [20, 20, 20];
/// Grid lines darken what they cross.
pub const GRID_ALPHA: u8 = 50;

/// Pastel colors, so that floors stay readable under walls and labels.
const FLOORS: [Rgb; 8] = [
    [196, 176, 134],
    [146, 178, 200],
    [170, 196, 140],
    [206, 156, 156],
    [168, 156, 208],
    [208, 196, 132],
    [140, 196, 176],
    [196, 164, 206],
];

/// Patterns darken the floor under them, in a color of their own.
pub const PATTERN_ALPHA: u8 = 150;

/// A darker color per pattern texture, the same on every run.
pub fn pattern_color(texture: &AssetPath) -> Rgb {
    floor_color(texture).map(|channel| channel / 2)
}

/// A color per tileset, the same on every run.
pub fn floor_color(tileset: &AssetPath) -> Rgb {
    let index = fnv1a(tileset.as_str().as_bytes()) % FLOORS.len() as u64;
    FLOORS[usize::try_from(index).expect("palette index fits in usize")]
}

/// FNV-1a: a tiny, stable hash (std's hasher may change between releases).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use dd_domain::AssetPath;

    use super::floor_color;

    fn color(path: &str) -> [u8; 3] {
        floor_color(&AssetPath::new(path).unwrap())
    }

    #[test]
    fn a_tileset_always_gets_the_same_color() {
        assert_eq!(color("a.webp"), color("a.webp"));
    }

    #[test]
    fn tilesets_spread_over_the_palette() {
        let colors: std::collections::HashSet<[u8; 3]> = (0..32)
            .map(|index| color(&format!("tileset_{index}.webp")))
            .collect();
        assert!(colors.len() >= 6, "{colors:?}");
    }
}
