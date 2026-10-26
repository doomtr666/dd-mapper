//! Properties every generated layout must have.

use std::collections::{HashSet, VecDeque};

use dd_domain::{CellPoint, CellPosition, Polygon, Segment};
use dd_gen::{BspParams, DoorSpot, HabParams, Layout, bsp, hab_block, ship_deck};
use proptest::prelude::*;

const ON_LINE: f64 = 1e-9;

fn bsp_params() -> impl Strategy<Value = BspParams> {
    (8_u32..40, 8_u32..30, 2_u32..5, any::<u64>()).prop_map(|(width, height, min_room, seed)| {
        BspParams {
            width,
            height,
            min_room,
            seed,
        }
    })
}

fn hab_params() -> impl Strategy<Value = HabParams> {
    (16_u32..48, 8_u32..20, 3_u32..6, any::<u64>()).prop_map(|(width, height, min_room, seed)| {
        HabParams {
            width,
            height,
            min_room,
            seed,
        }
    })
}

fn segments(points: &[CellPoint]) -> Vec<Segment> {
    points
        .windows(2)
        .map(|pair| Segment {
            start: pair[0],
            end: pair[1],
        })
        .collect()
}

/// The two sides of a door: points half a cell away, across its wall.
fn sides(segment: Segment, at: CellPosition) -> [CellPosition; 2] {
    let (dx, dy) = (
        f64::from(segment.end.x - segment.start.x),
        f64::from(segment.end.y - segment.start.y),
    );
    let length = dx.hypot(dy);
    let (nx, ny) = (-dy / length * 0.5, dx / length * 0.5);
    [
        CellPosition::new(at.x + nx, at.y + ny),
        CellPosition::new(at.x - nx, at.y - ny),
    ]
}

fn door_segment(layout: &Layout, index: usize, at: CellPosition) -> Option<Segment> {
    segments(&layout.partitions[index])
        .into_iter()
        .find(|segment| segment.locate_within(at, ON_LINE).is_some())
}

/// Areas reachable from the first one through partition doors.
fn reachable(layout: &Layout) -> HashSet<usize> {
    let area_at =
        |position: CellPosition| layout.areas.iter().position(|area| area.contains(position));
    let mut links: Vec<(usize, usize)> = Vec::new();
    for door in &layout.doors {
        if let DoorSpot::Partition { index, at } = *door
            && let Some(segment) = door_segment(layout, index, at)
            && let [Some(first), Some(second)] = sides(segment, at).map(area_at)
        {
            links.push((first, second));
        }
    }
    let mut seen = HashSet::from([0]);
    let mut queue = VecDeque::from([0]);
    while let Some(current) = queue.pop_front() {
        for &(first, second) in &links {
            let next = if first == current {
                second
            } else if second == current {
                first
            } else {
                continue;
            };
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    seen
}

/// Areas cover `cells` cells, every door sits on its partition and every
/// area can be reached.
fn assert_sound(layout: &Layout, cells: u64) -> Result<(), TestCaseError> {
    let tiled: u64 = layout.areas.iter().map(dd_gen::Area::cells).sum();
    prop_assert_eq!(tiled, cells);
    for door in &layout.doors {
        if let DoorSpot::Partition { index, at } = *door {
            prop_assert!(door_segment(layout, index, at).is_some(), "{door:?}");
        }
    }
    prop_assert_eq!(reachable(layout).len(), layout.areas.len());
    Ok(())
}

/// Doors stand at least `clearance` cells from every partition corner.
fn assert_clear_of_junctions(layout: &Layout, clearance: f64) -> Result<(), TestCaseError> {
    let corners: Vec<CellPosition> = layout
        .partitions
        .iter()
        .flatten()
        .map(|point| CellPosition::new(f64::from(point.x), f64::from(point.y)))
        .collect();
    for door in &layout.doors {
        if let DoorSpot::Partition { at, .. } = *door {
            for corner in &corners {
                let distance = (corner.x - at.x).hypot(corner.y - at.y);
                prop_assert!(distance >= clearance, "{door:?} near {corner:?}");
            }
        }
    }
    Ok(())
}

fn entrances(layout: &Layout) -> usize {
    layout
        .doors
        .iter()
        .filter(|door| matches!(door, DoorSpot::Outline(_)))
        .count()
}

proptest! {
    #[test]
    fn bsp_layouts_are_sound(params in bsp_params()) {
        let layout = bsp(params);
        assert_sound(&layout, u64::from(params.width - 2) * u64::from(params.height - 2))?;
        prop_assert_eq!(layout.partitions.len() + 1, layout.areas.len());
        prop_assert_eq!(entrances(&layout), 1);
    }

    #[test]
    fn bsp_rooms_respect_the_minimum(params in bsp_params()) {
        for area in &bsp(params).areas {
            prop_assert!(area.width >= params.min_room.min(params.width - 2));
            prop_assert!(area.height >= params.min_room.min(params.height - 2));
        }
    }

    /// From a minimum room side of 3, every wall has a cell at least one
    /// cell away from the walls it meets; tighter rooms may not.
    #[test]
    fn bsp_doors_stay_clear_of_partition_junctions(params in bsp_params()) {
        let clearance = if params.min_room >= 3 { 1.0 } else { 0.5 };
        assert_clear_of_junctions(&bsp(params), clearance)?;
    }

    #[test]
    fn hab_blocks_are_sound(params in hab_params()) {
        let layout = hab_block(params);
        assert_sound(&layout, u64::from(params.width - 2) * u64::from(params.height - 2))?;
        assert_clear_of_junctions(&layout, 1.0)?;
        prop_assert_eq!(entrances(&layout), 2);
    }

    #[test]
    fn hab_rooms_open_on_the_corridor_and_respect_the_minimum(params in hab_params()) {
        let layout = hab_block(params);
        let corridor = layout.areas[0];
        prop_assert_eq!(corridor.height, 2);
        for room in &layout.areas[1..] {
            prop_assert!(room.width >= params.min_room);
        }
        let room_doors = layout
            .doors
            .iter()
            .filter(|door| matches!(door, DoorSpot::Partition { .. }))
            .count();
        prop_assert_eq!(room_doors, layout.areas.len() - 1);
    }

    #[test]
    fn ship_decks_have_a_bow_and_one_entrance(params in hab_params()) {
        let layout = ship_deck(params);
        prop_assert_eq!(layout.outline.len(), 5);
        let hull = Polygon::new(layout.outline.clone());
        prop_assert!(hull.is_ok(), "{:?}", layout.outline);
        prop_assert_eq!(entrances(&layout), 1);
        let rooms_area: u64 = layout.areas.iter().map(dd_gen::Area::cells).sum();
        prop_assert!(i64::try_from(rooms_area * 2).unwrap() < hull.unwrap().double_area());
        prop_assert_eq!(reachable(&layout).len(), layout.areas.len());
    }

    #[test]
    fn the_same_seed_gives_the_same_layout(params in bsp_params(), hab in hab_params()) {
        prop_assert_eq!(bsp(params), bsp(params));
        prop_assert_eq!(hab_block(hab), hab_block(hab));
        prop_assert_eq!(ship_deck(hab), ship_deck(hab));
    }
}
