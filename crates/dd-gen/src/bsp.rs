//! Binary space partitioning of a rectangular building into rooms.

use dd_domain::{CellPoint, CellPosition};

use crate::{Area, DoorSpot, Layout, Random};

/// A room this much longer than wide is split across its length.
const ELONGATED: f64 = 1.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BspParams {
    /// Map size, in cells.
    pub width: u32,
    pub height: u32,
    /// Smallest room side, in cells.
    pub min_room: u32,
    /// Cells left free around the building, for engines or a courtyard.
    pub margin: u32,
    /// Keep a large hall, half the building, between two wings of rooms.
    pub hall: bool,
    pub seed: u64,
}

/// Splits the building recursively; every split is a partition with one door,
/// so every room can be reached. An entrance opens in the outline.
#[must_use]
pub fn bsp(params: BspParams) -> Layout {
    let margin = params.margin;
    let building = Area {
        x: margin.cast_signed(),
        y: margin.cast_signed(),
        width: params.width.saturating_sub(2 * margin).max(1),
        height: params.height.saturating_sub(2 * margin).max(1),
    };
    let mut builder = Builder {
        random: Random::new(params.seed),
        min_room: params.min_room.max(1),
        partitions: Vec::new(),
        doors: Vec::new(),
        areas: Vec::new(),
        hall: None,
    };
    if params.hall {
        builder.split_around_hall(building);
    } else {
        builder.split(building);
    }
    let entrance = builder.entrance(building);
    builder.doors.push(entrance);
    Layout {
        outline: building.corners(),
        partitions: builder.partitions,
        doors: builder.doors,
        areas: builder.areas,
    }
}

/// What fills one side of a cut.
#[derive(Debug, Clone, Copy)]
enum Fill {
    /// Rooms split further.
    Rooms,
    /// One large room left whole.
    Hall,
    /// The hall on the first side of this cut, rooms on the other.
    HallThenRooms(Cut),
}

/// A straight cut through an area, `offset` cells from its left or top side.
#[derive(Debug, Clone, Copy)]
enum Cut {
    Vertical(u32),
    Horizontal(u32),
}

impl Cut {
    /// The two halves, and the partition between them.
    fn apply(self, area: Area) -> (Area, Area, [CellPoint; 2]) {
        let (right, bottom) = area.far_corner();
        match self {
            Self::Vertical(offset) => {
                let x = area.x + offset.cast_signed();
                let first = Area {
                    width: offset,
                    ..area
                };
                let second = Area {
                    x,
                    width: area.width - offset,
                    ..area
                };
                (
                    first,
                    second,
                    [CellPoint::new(x, area.y), CellPoint::new(x, bottom)],
                )
            }
            Self::Horizontal(offset) => {
                let y = area.y + offset.cast_signed();
                let first = Area {
                    height: offset,
                    ..area
                };
                let second = Area {
                    y,
                    height: area.height - offset,
                    ..area
                };
                (
                    first,
                    second,
                    [CellPoint::new(area.x, y), CellPoint::new(right, y)],
                )
            }
        }
    }
}

struct Builder {
    random: Random,
    min_room: u32,
    partitions: Vec<Vec<CellPoint>>,
    doors: Vec<DoorSpot>,
    areas: Vec<Area>,
    hall: Option<Area>,
}

impl Builder {
    fn split(&mut self, area: Area) {
        let Some(cut) = self.choose_cut(area) else {
            self.areas.push(area);
            return;
        };
        self.divide(area, cut, [Fill::Rooms, Fill::Rooms]);
    }

    /// A wing of rooms, the hall, and another wing, across the building's length.
    /// Buildings too short for wings are split as usual.
    fn split_around_hall(&mut self, building: Area) {
        let vertical = building.width >= building.height;
        let length = if vertical {
            building.width
        } else {
            building.height
        };
        if length < 4 * self.min_room {
            self.split(building);
            return;
        }
        let wing = length / 4;
        let cut = |offset| {
            if vertical {
                Cut::Vertical(offset)
            } else {
                Cut::Horizontal(offset)
            }
        };
        let hall = Fill::HallThenRooms(cut(length - 2 * wing));
        self.divide(building, cut(wing), [Fill::Rooms, hall]);
    }

    /// Cuts `area` with a partition holding one door, and fills both sides.
    fn divide(&mut self, area: Area, cut: Cut, [first_fill, second_fill]: [Fill; 2]) {
        let (first, second, line) = cut.apply(area);
        let index = self.partitions.len();
        self.partitions.push(line.to_vec());
        self.fill(first, first_fill);
        self.fill(second, second_fill);
        let at = self.door_on(line);
        self.doors.push(DoorSpot::Partition { index, at });
    }

    fn fill(&mut self, area: Area, fill: Fill) {
        match fill {
            Fill::Rooms => self.split(area),
            Fill::Hall => {
                self.hall = Some(area);
                self.areas.push(area);
            }
            Fill::HallThenRooms(cut) => self.divide(area, cut, [Fill::Hall, Fill::Rooms]),
        }
    }

    fn choose_cut(&mut self, area: Area) -> Option<Cut> {
        let can_cut_across = area.width >= 2 * self.min_room;
        let can_cut_along = area.height >= 2 * self.min_room;
        let vertical = match (can_cut_across, can_cut_along) {
            (false, false) => return None,
            (true, false) => true,
            (false, true) => false,
            (true, true) => self.prefers_vertical(area),
        };
        let size = if vertical { area.width } else { area.height };
        let offset = self.random.between(self.min_room, size - self.min_room);
        Some(if vertical {
            Cut::Vertical(offset)
        } else {
            Cut::Horizontal(offset)
        })
    }

    /// Long areas are cut across their length; square ones either way.
    fn prefers_vertical(&mut self, area: Area) -> bool {
        let (width, height) = (f64::from(area.width), f64::from(area.height));
        if width >= height * ELONGATED {
            true
        } else if height >= width * ELONGATED {
            false
        } else {
            self.random.coin()
        }
    }

    /// The middle of a cell along `line`, as far as possible from the
    /// partitions meeting it, picked at random among the clear ones.
    fn door_on(&mut self, [start, end]: [CellPoint; 2]) -> CellPosition {
        let junctions: Vec<CellPoint> = self
            .partitions
            .iter()
            .flatten()
            .copied()
            .filter(|point| on_line(*point, start, end))
            .collect();
        let candidates = cell_centers(start, end);
        let clearance = |at: &CellPosition| {
            junctions
                .iter()
                .map(|junction| distance(*junction, *at))
                .fold(f64::INFINITY, f64::min)
        };
        let clear: Vec<CellPosition> = candidates
            .iter()
            .copied()
            .filter(|at| clearance(at) >= 1.0)
            .collect();
        if clear.is_empty() {
            return candidates
                .into_iter()
                .max_by(|first, second| clearance(first).total_cmp(&clearance(second)))
                .expect("a partition is at least one cell long");
        }
        let last = u32::try_from(clear.len() - 1).unwrap_or(u32::MAX);
        let pick = usize::try_from(self.random.between(0, last)).unwrap_or(0);
        clear[pick]
    }

    /// An entrance in the middle of the hall, or of a room, along the bottom
    /// of the building.
    fn entrance(&mut self, building: Area) -> DoorSpot {
        let (_, bottom) = building.far_corner();
        if let Some(hall) = self.hall
            && hall.far_corner().1 == bottom
        {
            let middle = f64::from(hall.x) + f64::from(hall.width) / 2.0;
            return DoorSpot::Outline(CellPosition::new(middle, f64::from(bottom)));
        }
        let along_bottom: Vec<Area> = self
            .areas
            .iter()
            .copied()
            .filter(|area| area.far_corner().1 == bottom)
            .collect();
        let last = u32::try_from(along_bottom.len().saturating_sub(1)).unwrap_or(0);
        let pick = usize::try_from(self.random.between(0, last)).unwrap_or(0);
        let room = along_bottom.get(pick).copied().unwrap_or(building);
        let middle = f64::from(room.x) + f64::from(room.width) / 2.0;
        DoorSpot::Outline(CellPosition::new(middle, f64::from(bottom)))
    }
}

fn on_line(point: CellPoint, start: CellPoint, end: CellPoint) -> bool {
    let within =
        |value: i32, bound: i32, other: i32| value >= bound.min(other) && value <= bound.max(other);
    let collinear =
        (end.x - start.x) * (point.y - start.y) == (end.y - start.y) * (point.x - start.x);
    collinear && within(point.x, start.x, end.x) && within(point.y, start.y, end.y)
}

/// Centers of the cells a straight, axis-aligned line runs along.
fn cell_centers(start: CellPoint, end: CellPoint) -> Vec<CellPosition> {
    let (dx, dy) = ((end.x - start.x).signum(), (end.y - start.y).signum());
    let length = (end.x - start.x).abs().max((end.y - start.y).abs());
    (0..length)
        .map(|step| {
            let along = f64::from(step) + 0.5;
            CellPosition::new(
                f64::from(start.x) + f64::from(dx) * along,
                f64::from(start.y) + f64::from(dy) * along,
            )
        })
        .collect()
}

fn distance(point: CellPoint, at: CellPosition) -> f64 {
    (f64::from(point.x) - at.x).hypot(f64::from(point.y) - at.y)
}
