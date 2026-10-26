//! Cave and terrain layers, reset to empty for the map size.

use dd_domain::GridSize;

use crate::godot::PoolByteArray;
use crate::model::{Cave, Terrain};

/// Cave and terrain bitmaps have 4 sub-cells per cell side.
const SUBCELLS_PER_CELL: usize = 4;
/// The cave bitmap has a 3 sub-cell border.
const CAVE_BORDER: usize = 3;
/// 100 % of `texture_1`, none of the others.
const FIRST_TEXTURE_ONLY: [u8; 4] = [255, 0, 0, 0];

/// An empty cave of the right size, keeping the template's colors and texture.
pub fn blank_cave(grid: GridSize, template: &Cave) -> Cave {
    let bytes = cave_bitmap_bytes(grid);
    Cave {
        bitmap: PoolByteArray(vec![0; bytes]),
        entrance_bitmap: PoolByteArray(vec![0; bytes]),
        ..template.clone()
    }
}

/// A terrain fully covered by `texture_1`, keeping the template's textures.
pub fn blank_terrain(grid: GridSize, template: &Terrain) -> Terrain {
    let pixels = subcells(grid.width()) * subcells(grid.height());
    Terrain {
        splat: PoolByteArray(FIRST_TEXTURE_ONLY.repeat(pixels)),
        ..template.clone()
    }
}

/// One bit per sub-cell, border included, rounded up to whole bytes.
fn cave_bitmap_bytes(grid: GridSize) -> usize {
    let columns = subcells(grid.width()) + CAVE_BORDER;
    let rows = subcells(grid.height()) + CAVE_BORDER;
    (columns * rows).div_ceil(8)
}

fn subcells(cells: u32) -> usize {
    usize::try_from(cells).expect("map dimensions fit in memory") * SUBCELLS_PER_CELL
}

#[cfg(test)]
mod tests {
    use dd_domain::GridSize;

    use super::{blank_cave, blank_terrain};
    use crate::godot::PoolByteArray;
    use crate::model::{Cave, Color, Terrain};

    fn template_cave() -> Cave {
        Cave {
            bitmap: PoolByteArray(vec![1, 2]),
            ground_color: Color(0xff7f_7e71),
            wall_color: Color(0xff7f_7e70),
            entrance_bitmap: PoolByteArray(vec![3]),
            texture: "floor.png".to_owned(),
        }
    }

    fn template_terrain() -> Terrain {
        Terrain {
            enabled: true,
            expand_slots: false,
            smooth_blending: false,
            texture_1: "sulphur.webp".to_owned(),
            texture_2: "gravel.png".to_owned(),
            texture_3: "sand.png".to_owned(),
            texture_4: "snow.png".to_owned(),
            splat: PoolByteArray(vec![0, 255, 0, 0]),
        }
    }

    #[test]
    fn cave_bitmaps_match_both_fixture_sizes() {
        for (width, height, bytes) in [(35, 20, 1484), (48, 27, 2706)] {
            let cave = blank_cave(GridSize::new(width, height).unwrap(), &template_cave());
            assert_eq!(cave.bitmap.0, vec![0; bytes]);
            assert_eq!(cave.entrance_bitmap.0, vec![0; bytes]);
            assert_eq!(cave.ground_color, Color(0xff7f_7e71));
            assert_eq!(cave.texture, "floor.png");
        }
    }

    #[test]
    fn terrain_is_all_first_texture() {
        let terrain = blank_terrain(GridSize::new(35, 20).unwrap(), &template_terrain());
        assert_eq!(terrain.splat.0.len(), 35 * 4 * 20 * 4 * 4);
        assert!(
            terrain
                .splat
                .0
                .chunks(4)
                .all(|pixel| pixel == [255, 0, 0, 0])
        );
        assert_eq!(terrain.texture_1, "sulphur.webp");
    }
}
