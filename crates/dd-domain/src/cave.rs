use crate::{CellPosition, GridSize};

/// Samples per cell side.
const SAMPLES_PER_CELL: u32 = 4;
/// The grid overhangs the map by one sample on each side, plus the closing vertex.
const EXTRA_SAMPLES: u32 = 3;

/// Where caves are dug, sampled every quarter of a cell. Sample `(i, j)`
/// stands at `((i - 1) / 4, (j - 1) / 4)` cells, so the samples overhang the
/// map by a quarter cell on every side. Entrances are cave edges left open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaveGrid {
    columns: u32,
    rows: u32,
    floor: Vec<bool>,
    entrance: Vec<bool>,
}

impl CaveGrid {
    /// No cave at all.
    #[must_use]
    pub fn new(grid: GridSize) -> Self {
        let columns = grid.width() * SAMPLES_PER_CELL + EXTRA_SAMPLES;
        let rows = grid.height() * SAMPLES_PER_CELL + EXTRA_SAMPLES;
        let samples = (columns * rows) as usize;
        Self {
            columns,
            rows,
            floor: vec![false; samples],
            entrance: vec![false; samples],
        }
    }

    #[must_use]
    pub fn columns(&self) -> u32 {
        self.columns
    }

    #[must_use]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Where sample `(column, row)` stands, in cells.
    #[must_use]
    pub fn position(column: u32, row: u32) -> CellPosition {
        let cells = |sample: u32| (f64::from(sample) - 1.0) / f64::from(SAMPLES_PER_CELL);
        CellPosition::new(cells(column), cells(row))
    }

    #[must_use]
    pub fn is_floor(&self, column: u32, row: u32) -> bool {
        self.index(column, row)
            .is_some_and(|index| self.floor[index])
    }

    #[must_use]
    pub fn is_entrance(&self, column: u32, row: u32) -> bool {
        self.index(column, row)
            .is_some_and(|index| self.entrance[index])
    }

    /// Whether the sample nearest to `position` is cave floor.
    #[must_use]
    pub fn is_floor_at(&self, position: CellPosition) -> bool {
        let sample = |cells: f64| {
            let nearest = (cells * f64::from(SAMPLES_PER_CELL) + 1.0).round();
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "checked to lie within the grid"
            )]
            (nearest >= 0.0).then_some(nearest as u32)
        };
        match (sample(position.x), sample(position.y)) {
            (Some(column), Some(row)) => self.is_floor(column, row),
            _ => false,
        }
    }

    /// How many samples are cave floor; sixteen make a cell.
    #[must_use]
    pub fn floor_samples(&self) -> usize {
        self.floor.iter().filter(|floor| **floor).count()
    }

    #[must_use]
    pub fn has_entrance(&self) -> bool {
        self.entrance.contains(&true)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.floor.contains(&true) && !self.entrance.contains(&true)
    }

    /// Digs (`floor` true) or fills every sample where `inside` holds;
    /// returns how many samples changed.
    pub fn dig(&mut self, inside: impl Fn(CellPosition) -> bool, floor: bool) -> usize {
        let indices = self.matching(inside);
        update(&mut self.floor, &indices, floor)
    }

    /// Opens (or closes) the cave walls where `inside` holds; returns how many
    /// samples changed.
    pub fn mark_entrance(&mut self, inside: impl Fn(CellPosition) -> bool, open: bool) -> usize {
        let indices = self.matching(inside);
        update(&mut self.entrance, &indices, open)
    }

    /// Sets one sample, as read from a file.
    pub fn set(&mut self, column: u32, row: u32, floor: bool, entrance: bool) {
        if let Some(index) = self.index(column, row) {
            self.floor[index] = floor;
            self.entrance[index] = entrance;
        }
    }

    fn index(&self, column: u32, row: u32) -> Option<usize> {
        (column < self.columns && row < self.rows).then(|| (row * self.columns + column) as usize)
    }

    /// Indices of the samples standing where `inside` holds.
    fn matching(&self, inside: impl Fn(CellPosition) -> bool) -> Vec<usize> {
        (0..self.rows)
            .flat_map(|row| (0..self.columns).map(move |column| (column, row)))
            .filter(|&(column, row)| inside(Self::position(column, row)))
            .filter_map(|(column, row)| self.index(column, row))
            .collect()
    }
}

/// Sets `samples[index]` to `value` at every index; returns how many changed.
fn update(samples: &mut [bool], indices: &[usize], value: bool) -> usize {
    let mut changed = 0;
    for &index in indices {
        if samples[index] != value {
            samples[index] = value;
            changed += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::CaveGrid;
    use crate::{CellPosition, GridSize};

    fn grid() -> CaveGrid {
        CaveGrid::new(GridSize::new(48, 27).unwrap())
    }

    #[test]
    fn samples_every_quarter_cell_with_an_overhang() {
        let cave = grid();
        assert_eq!((cave.columns(), cave.rows()), (195, 111));
        assert_eq!(CaveGrid::position(1, 1), CellPosition::new(0.0, 0.0));
        assert_eq!(CaveGrid::position(0, 0), CellPosition::new(-0.25, -0.25));
        assert_eq!(CaveGrid::position(193, 109), CellPosition::new(48.0, 27.0));
        assert!(cave.is_empty());
    }

    #[test]
    fn digs_and_fills_where_asked() {
        let mut cave = grid();
        let disc = |position: CellPosition| (position.x - 10.0).hypot(position.y - 5.0) <= 1.0;
        let dug = cave.dig(disc, true);
        assert!(dug > 40 && dug < 60, "{dug}");
        assert!(cave.is_floor(41, 21));
        assert!(cave.is_floor_at(CellPosition::new(10.3, 5.2)));
        assert!(!cave.is_floor_at(CellPosition::new(12.0, 5.0)));
        assert!(!cave.is_empty());
        assert_eq!(cave.floor_samples(), dug);
        assert!(!cave.has_entrance());
        assert_eq!(cave.dig(disc, true), 0);
        assert_eq!(cave.dig(disc, false), dug);
        assert!(cave.is_empty());
    }

    #[test]
    fn entrances_are_marked_apart_from_the_floor() {
        let mut cave = grid();
        let spot = |position: CellPosition| position == CellPosition::new(2.0, 3.0);
        assert_eq!(cave.mark_entrance(spot, true), 1);
        assert!(cave.is_entrance(9, 13));
        assert!(!cave.is_floor(9, 13));
        cave.set(9, 13, true, false);
        assert!(cave.is_floor(9, 13) && !cave.is_entrance(9, 13));
    }
}
