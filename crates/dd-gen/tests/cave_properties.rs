//! Properties every grown cave network must have.

use std::collections::{HashSet, VecDeque};

use dd_gen::{CaveLayout, CaveParams, cave};
use proptest::prelude::*;

fn params() -> impl Strategy<Value = CaveParams> {
    (12_u32..48, 10_u32..32, 40_u32..56, any::<u64>()).prop_map(|(width, height, density, seed)| {
        CaveParams {
            width,
            height,
            density: f64::from(density) / 100.0,
            seed,
        }
    })
}

fn floor_cells(layout: &CaveLayout) -> Vec<(i64, i64)> {
    (0..i64::from(layout.height))
        .flat_map(|row| (0..i64::from(layout.width)).map(move |column| (column, row)))
        .filter(|&(column, row)| layout.is_floor(column, row))
        .collect()
}

/// Floor cells reached from the first one, through edges.
fn reached(layout: &CaveLayout, start: (i64, i64)) -> usize {
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some((column, row)) = queue.pop_front() {
        for next in [
            (column + 1, row),
            (column - 1, row),
            (column, row + 1),
            (column, row - 1),
        ] {
            if layout.is_floor(next.0, next.1) && seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    seen.len()
}

proptest! {
    #[test]
    fn every_part_of_the_cave_can_be_reached(params in params()) {
        let layout = cave(params);
        let floor = floor_cells(&layout);
        prop_assert!(!floor.is_empty());
        prop_assert_eq!(reached(&layout, floor[0]), floor.len());
    }

    #[test]
    fn the_border_stays_rock(params in params()) {
        let layout = cave(params);
        let (right, bottom) = (i64::from(params.width) - 1, i64::from(params.height) - 1);
        for (column, row) in floor_cells(&layout) {
            prop_assert!(column > 0 && row > 0 && column < right && row < bottom);
        }
    }

    #[test]
    fn caves_are_neither_solid_nor_hollow(params in params()) {
        let layout = cave(params);
        let cells = f64::from(params.width * params.height);
        let share = f64::from(u32::try_from(floor_cells(&layout).len()).unwrap()) / cells;
        prop_assert!(share > 0.15 && share < 0.8, "{share}");
    }

    #[test]
    fn the_same_seed_grows_the_same_cave(params in params()) {
        prop_assert_eq!(cave(params), cave(params));
    }
}
