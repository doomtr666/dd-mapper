//! Cave networks grown by a cellular automaton, then reduced to their largest
//! connected pocket so that every part can be reached.

use crate::Random;

/// A cave network to grow over an area of cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaveParams {
    pub width: u32,
    pub height: u32,
    /// Share of rock to start from: 0.45 gives open caverns, 0.55 narrow ones.
    pub density: f64,
    pub seed: u64,
}

/// Which cells of the area are cave floor, row after row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaveLayout {
    pub width: u32,
    pub height: u32,
    pub floor: Vec<bool>,
}

impl CaveLayout {
    #[must_use]
    pub fn is_floor(&self, column: i64, row: i64) -> bool {
        let inside = (0..i64::from(self.width)).contains(&column)
            && (0..i64::from(self.height)).contains(&row);
        inside && {
            let index = row * i64::from(self.width) + column;
            usize::try_from(index).is_ok_and(|index| self.floor[index])
        }
    }
}

/// Smoothing passes of the automaton.
const PASSES: u32 = 5;
/// A cell turns to rock with at least this many rock cells around it, itself included.
const ROCK_NEIGHBOURS: usize = 5;
/// Pockets smaller than this share of the area are grown again, more open.
const SMALLEST_SHARE: f64 = 0.2;
/// Growths tried before keeping the last one.
const ATTEMPTS: u32 = 8;
/// Rock taken off the starting density at each new growth.
const OPENING_STEP: f64 = 0.04;

/// Grows a cave network over the area; its border stays rock.
#[must_use]
pub fn cave(params: CaveParams) -> CaveLayout {
    let mut random = Random::new(params.seed);
    let cells = f64::from(params.width) * f64::from(params.height);
    let mut layout = grow(params, &mut random);
    for attempt in 1..ATTEMPTS {
        let floor =
            u32::try_from(layout.floor.iter().filter(|floor| **floor).count()).unwrap_or(u32::MAX);
        if f64::from(floor) >= cells * SMALLEST_SHARE {
            break;
        }
        let opener = CaveParams {
            density: params.density - OPENING_STEP * f64::from(attempt),
            ..params
        };
        layout = grow(opener, &mut random);
    }
    layout
}

fn grow(params: CaveParams, random: &mut Random) -> CaveLayout {
    let mut layout = CaveLayout {
        width: params.width,
        height: params.height,
        floor: Vec::new(),
    };
    let threshold = (params.density.clamp(0.0, 1.0) * 1000.0).round();
    layout.floor = cells(&layout)
        .map(|(column, row)| {
            !layout.is_border(column, row) && f64::from(random.between(0, 999)) >= threshold
        })
        .collect();
    for _ in 0..PASSES {
        layout.floor = cells(&layout)
            .map(|(column, row)| {
                !layout.is_border(column, row) && layout.rock_around(column, row) < ROCK_NEIGHBOURS
            })
            .collect();
    }
    layout.keep_largest_pocket();
    layout
}

/// Every cell of the area, row after row.
fn cells(layout: &CaveLayout) -> impl Iterator<Item = (i64, i64)> + use<> {
    let (width, height) = (i64::from(layout.width), i64::from(layout.height));
    (0..height).flat_map(move |row| (0..width).map(move |column| (column, row)))
}

impl CaveLayout {
    fn is_border(&self, column: i64, row: i64) -> bool {
        column == 0
            || row == 0
            || column == i64::from(self.width) - 1
            || row == i64::from(self.height) - 1
    }

    /// Rock cells in the 3 x 3 block around a cell; outside counts as rock.
    fn rock_around(&self, column: i64, row: i64) -> usize {
        (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
            .filter(|&(dx, dy)| !self.is_floor(column + dx, row + dy))
            .count()
    }

    /// Fills every pocket of floor but the largest with rock.
    fn keep_largest_pocket(&mut self) {
        let mut pocket_of = vec![usize::MAX; self.floor.len()];
        let mut sizes = Vec::new();
        for (column, row) in cells(self) {
            let index = self.index(column, row);
            if self.floor[index] && pocket_of[index] == usize::MAX {
                sizes.push(self.flood(column, row, sizes.len(), &mut pocket_of));
            }
        }
        let largest = (0..sizes.len()).max_by_key(|&pocket| sizes[pocket]);
        for (index, floor) in self.floor.iter_mut().enumerate() {
            *floor = *floor && Some(pocket_of[index]) == largest;
        }
    }

    /// Marks the pocket holding a cell; returns its size.
    fn flood(&self, column: i64, row: i64, pocket: usize, pocket_of: &mut [usize]) -> usize {
        let mut stack = vec![(column, row)];
        pocket_of[self.index(column, row)] = pocket;
        let mut size = 0;
        while let Some((column, row)) = stack.pop() {
            size += 1;
            for (next_column, next_row) in [
                (column + 1, row),
                (column - 1, row),
                (column, row + 1),
                (column, row - 1),
            ] {
                if self.is_floor(next_column, next_row) {
                    let next = self.index(next_column, next_row);
                    if pocket_of[next] == usize::MAX {
                        pocket_of[next] = pocket;
                        stack.push((next_column, next_row));
                    }
                }
            }
        }
        size
    }

    fn index(&self, column: i64, row: i64) -> usize {
        usize::try_from(row * i64::from(self.width) + column).unwrap_or(0)
    }
}
