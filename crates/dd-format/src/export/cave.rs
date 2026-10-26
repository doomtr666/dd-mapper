//! Caves as Dungeondraft stores them: one bit per quarter-cell sample, row
//! after row, the lowest bit of each byte first.

use dd_domain::CaveGrid;

use crate::godot::PoolByteArray;
use crate::model::Cave;

/// Writes the cave floor and its entrances into `cave`.
pub fn write_cave(cave: &mut Cave, grid: &CaveGrid) {
    cave.bitmap = pack(grid, CaveGrid::is_floor);
    cave.entrance_bitmap = pack(grid, CaveGrid::is_entrance);
}

/// Reads `bitmap` back, or `None` when its size does not fit `grid`.
#[must_use]
pub fn unpack(bitmap: &PoolByteArray, grid: &CaveGrid) -> Option<Vec<bool>> {
    let samples = (grid.columns() * grid.rows()) as usize;
    if bitmap.0.len() != samples.div_ceil(8) {
        return None;
    }
    Some(
        (0..samples)
            .map(|index| bitmap.0[index / 8] >> (index % 8) & 1 == 1)
            .collect(),
    )
}

fn pack(grid: &CaveGrid, set: fn(&CaveGrid, u32, u32) -> bool) -> PoolByteArray {
    let samples = (grid.columns() * grid.rows()) as usize;
    let mut bytes = vec![0_u8; samples.div_ceil(8)];
    for row in 0..grid.rows() {
        for column in 0..grid.columns() {
            if set(grid, column, row) {
                let index = (row * grid.columns() + column) as usize;
                bytes[index / 8] |= 1 << (index % 8);
            }
        }
    }
    PoolByteArray(bytes)
}

#[cfg(test)]
mod tests {
    use dd_domain::{CaveGrid, CellPosition, GridSize};

    use super::{pack, unpack};

    #[test]
    fn packs_rows_lowest_bit_first() {
        let mut grid = CaveGrid::new(GridSize::new(2, 1).unwrap());
        assert_eq!((grid.columns(), grid.rows()), (11, 7));
        grid.dig(|position| position == CellPosition::new(0.0, 0.0), true);
        grid.dig(|position| position == CellPosition::new(0.5, 0.25), true);
        let bytes = pack(&grid, CaveGrid::is_floor);
        assert_eq!(bytes.0.len(), 10);
        // Sample (1, 1) is bit 12; sample (3, 2) is bit 25.
        assert_eq!(bytes.0[1], 0b0001_0000);
        assert_eq!(bytes.0[3], 0b0000_0010);
        let samples = unpack(&bytes, &grid).unwrap();
        assert_eq!(samples.iter().filter(|sample| **sample).count(), 2);
        assert!(samples[12] && samples[25]);
    }
}
