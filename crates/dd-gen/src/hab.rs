//! Hab blocks: cells on both sides of a central corridor. Ship decks add a bow.

use dd_domain::{CellPoint, CellPosition};

use crate::{Area, DoorSpot, Layout, Random};

/// Corridor width, in cells.
const CORRIDOR: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HabParams {
    /// Map size, in cells.
    pub width: u32,
    pub height: u32,
    /// Narrowest cell, in cells.
    pub min_room: u32,
    /// Cells left free around the building, for engines or a courtyard.
    pub margin: u32,
    pub seed: u64,
}

/// A two-cell-wide corridor along the block, cells on both sides, each with a
/// door on the corridor, and an entrance at both ends of the corridor.
#[must_use]
pub fn hab_block(params: HabParams) -> Layout {
    let block = building(params, 0);
    let mut layout = cells_along_corridor(block, params);
    let middle = corridor_middle(block);
    layout.doors.push(DoorSpot::Outline(CellPosition::new(
        f64::from(block.x),
        middle,
    )));
    let (east, _) = block.far_corner();
    layout.doors.push(DoorSpot::Outline(CellPosition::new(
        f64::from(east),
        middle,
    )));
    layout
}

/// A hab block ending in a pointed bow, entered from the stern.
#[must_use]
pub fn ship_deck(params: HabParams) -> Layout {
    let full = building(params, 0);
    let bow = (full.height / 2).min(full.width / 3).max(1);
    let hull = building(params, bow);
    let mut layout = cells_along_corridor(hull, params);
    let (east, south) = hull.far_corner();
    let corridor_top = corridor_top(hull);
    let corridor_bottom = corridor_top + CORRIDOR.cast_signed();
    layout.partitions.push(vec![
        CellPoint::new(east, hull.y),
        CellPoint::new(east, corridor_top),
    ]);
    layout.partitions.push(vec![
        CellPoint::new(east, corridor_bottom),
        CellPoint::new(east, south),
    ]);
    let apex = CellPoint::new(east + bow.cast_signed(), corridor_top + 1);
    layout.outline = vec![
        CellPoint::new(hull.x, hull.y),
        CellPoint::new(east, hull.y),
        apex,
        CellPoint::new(east, south),
        CellPoint::new(hull.x, south),
    ];
    let middle = corridor_middle(hull);
    layout.doors.push(DoorSpot::Outline(CellPosition::new(
        f64::from(hull.x),
        middle,
    )));
    layout
}

/// The building inside the margin, `bow` cells shorter.
fn building(params: HabParams, bow: u32) -> Area {
    let margin = params.margin;
    Area {
        x: margin.cast_signed(),
        y: margin.cast_signed(),
        width: params.width.saturating_sub(2 * margin + bow).max(1),
        height: params.height.saturating_sub(2 * margin).max(CORRIDOR + 2),
    }
}

fn corridor_top(block: Area) -> i32 {
    block.y + ((block.height - CORRIDOR) / 2).cast_signed()
}

fn corridor_middle(block: Area) -> f64 {
    f64::from(corridor_top(block)) + f64::from(CORRIDOR) / 2.0
}

/// The corridor walls, the cells above and below with their doors,
/// and the dividers between cells. No entrance yet.
fn cells_along_corridor(block: Area, params: HabParams) -> Layout {
    let mut random = Random::new(params.seed);
    let top = corridor_top(block);
    let bottom = top + CORRIDOR.cast_signed();
    let (east, south) = block.far_corner();
    let mut layout = Layout {
        outline: block.corners(),
        partitions: vec![
            vec![CellPoint::new(block.x, top), CellPoint::new(east, top)],
            vec![
                CellPoint::new(block.x, bottom),
                CellPoint::new(east, bottom),
            ],
        ],
        doors: Vec::new(),
        areas: vec![Area {
            y: top,
            height: CORRIDOR,
            ..block
        }],
    };
    let bands = [(block.y, top, 0, top), (bottom, south, 1, bottom)];
    for (band_top, band_bottom, wall, door_y) in bands {
        let band = Band {
            top: band_top,
            bottom: band_bottom,
            wall,
            door_y,
        };
        band.add_cells(&mut layout, block, params.min_room.max(1), &mut random);
    }
    layout
}

/// A row of cells opening on one corridor wall.
struct Band {
    top: i32,
    bottom: i32,
    /// Index of the corridor wall the cells open on.
    wall: usize,
    door_y: i32,
}

impl Band {
    fn add_cells(&self, layout: &mut Layout, block: Area, min_room: u32, random: &mut Random) {
        let mut x = block.x;
        for width in cell_widths(block.width, min_room, random) {
            if x > block.x {
                layout.partitions.push(vec![
                    CellPoint::new(x, self.top),
                    CellPoint::new(x, self.bottom),
                ]);
            }
            let cell = Area {
                x,
                y: self.top,
                width,
                height: (self.bottom - self.top).cast_unsigned(),
            };
            layout.areas.push(cell);
            let door_x = f64::from(x) + f64::from((width - 1) / 2) + 0.5;
            layout.doors.push(DoorSpot::Partition {
                index: self.wall,
                at: CellPosition::new(door_x, f64::from(self.door_y)),
            });
            x += width.cast_signed();
        }
    }
}

/// Widths of cells filling `length`, each between `min_room` and twice that.
fn cell_widths(length: u32, min_room: u32, random: &mut Random) -> Vec<u32> {
    let mut widths = Vec::new();
    let mut remaining = length;
    while remaining >= 2 * min_room {
        let widest = (2 * min_room - 1).min(remaining - min_room);
        let width = random.between(min_room, widest);
        widths.push(width);
        remaining -= width;
    }
    widths.push(remaining);
    widths
}

#[cfg(test)]
mod tests {
    use super::{HabParams, hab_block, ship_deck};

    #[test]
    fn a_zero_minimum_room_is_taken_as_one_cell() {
        let params = HabParams {
            width: 12,
            height: 8,
            min_room: 0,
            margin: 1,
            seed: 7,
        };
        assert!(hab_block(params).areas.iter().all(|area| area.width >= 1));
        assert!(ship_deck(params).areas.iter().all(|area| area.width >= 1));
    }
}
