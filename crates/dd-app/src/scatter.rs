//! Clutter strewn at random over an area: debris, cans, papers, cables...

use dd_catalog::AssetKind;
use dd_domain::{AssetPath, CellPosition, Footprint, Map, Polygon, Prop, QuarterTurn};
use dd_gen::Random;

use crate::assets::resolve;
use crate::ids::prop_label;
use crate::validate::{boxes_overlap, crosses_box, crosses_walls};
use crate::workspace::footprint;
use crate::{AppError, Shape, Workspace};

/// Random spots tried for each prop before giving up.
const ATTEMPTS_PER_PROP: u32 = 60;
/// Free space kept around door centers, in cells.
const DOORWAY_CLEARANCE: f64 = 1.0;
/// Positions are drawn on a grid this fine, per cell.
const STEPS_PER_CELL: u32 = 100;

/// Props picked among `assets` and strewn over `area`, inside rooms, clear of
/// walls, doorways and other props.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterRequest {
    pub assets: Vec<String>,
    pub area: Shape,
    /// How many props to place.
    pub count: u32,
    /// The same seed strews the same way.
    pub seed: u64,
}

/// What a scatter placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterReport {
    pub props: Vec<String>,
    /// Props asked for that found no free spot.
    pub missing: u32,
}

impl Workspace {
    /// # Errors
    /// Fails on an unknown map, an invalid area, no asset, or assets that are
    /// not objects.
    pub fn scatter(
        &mut self,
        map: &str,
        request: &ScatterRequest,
    ) -> Result<ScatterReport, AppError> {
        let palette = self.palette(&request.assets)?;
        let area = request.area.to_polygon()?;
        let target = self.map_mut(map)?;
        let mut random = Random::new(request.seed);
        let mut props = Vec::new();
        for _ in 0..request.count {
            let spot = (0..ATTEMPTS_PER_PROP)
                .map(|_| candidate(&palette, &area, &mut random))
                .find(|prop| fits(target, &area, prop));
            if let Some(prop) = spot {
                props.push(prop_label(target.add_prop(prop)?));
            }
        }
        let placed = u32::try_from(props.len()).unwrap_or(request.count);
        Ok(ScatterReport {
            props,
            missing: request.count - placed,
        })
    }

    /// The objects to pick from, with their footprints.
    fn palette(&self, assets: &[String]) -> Result<Vec<(AssetPath, Footprint)>, AppError> {
        if assets.is_empty() {
            return Err(AppError::NothingToScatter);
        }
        assets
            .iter()
            .map(|path| {
                let asset = resolve(self.catalog(), path, AssetKind::Object)?;
                Ok((asset.path.clone(), footprint(asset)?))
            })
            .collect()
    }
}

/// A random prop of the palette, turned at random, centered in the area's bounds.
fn candidate(palette: &[(AssetPath, Footprint)], area: &Polygon, random: &mut Random) -> Prop {
    let last = u32::try_from(palette.len() - 1).unwrap_or(0);
    let (asset, footprint) = &palette[random.between(0, last) as usize];
    let turns = random.between(0, 3).cast_signed();
    let (low, high) = bounds(area);
    Prop {
        asset: asset.clone(),
        center: CellPosition::new(
            between(low.x, high.x, random),
            between(low.y, high.y, random),
        ),
        rotation: QuarterTurn::from_degrees(turns * 90).unwrap_or(QuarterTurn::Deg0),
        footprint: *footprint,
    }
}

/// Inside the area and a room, crossing no wall, prop or doorway.
fn fits(map: &Map, area: &Polygon, prop: &Prop) -> bool {
    let (low, high) = prop.bounds();
    let corners = [
        low,
        CellPosition::new(high.x, low.y),
        high,
        CellPosition::new(low.x, high.y),
    ];
    let in_area = corners.iter().all(|corner| area.contains_position(*corner));
    let in_room = map.rooms().any(|(_, room)| {
        room.outline().contains_position(prop.center) && !crosses_walls(room, prop)
    });
    in_area
        && in_room
        && !map.partitions().any(|(_, wall)| {
            wall.segments()
                .any(|segment| crosses_box(segment, low, high))
        })
        && !map
            .props()
            .any(|(_, other)| boxes_overlap((low, high), other.bounds()))
        && map.doors().all(|(id, _)| {
            map.door_position(id)
                .is_none_or(|door| distance_to_box(door, low, high) > DOORWAY_CLEARANCE)
        })
}

fn bounds(area: &Polygon) -> (CellPosition, CellPosition) {
    let xs = area.vertices().iter().map(|vertex| f64::from(vertex.x));
    let ys = area.vertices().iter().map(|vertex| f64::from(vertex.y));
    (
        CellPosition::new(
            xs.clone().fold(f64::INFINITY, f64::min),
            ys.clone().fold(f64::INFINITY, f64::min),
        ),
        CellPosition::new(
            xs.fold(f64::NEG_INFINITY, f64::max),
            ys.fold(f64::NEG_INFINITY, f64::max),
        ),
    )
}

/// A random value from `low` to `high`, on a grid of hundredths of a cell.
fn between(low: f64, high: f64, random: &mut Random) -> f64 {
    let steps = random.between(0, STEPS_PER_CELL);
    low + (high - low) * f64::from(steps) / f64::from(STEPS_PER_CELL)
}

fn distance_to_box(point: CellPosition, low: CellPosition, high: CellPosition) -> f64 {
    let dx = (low.x - point.x).max(point.x - high.x).max(0.0);
    let dy = (low.y - point.y).max(point.y - high.y).max(0.0);
    dx.hypot(dy)
}

#[cfg(test)]
mod tests {
    use super::ScatterRequest;
    use crate::assets::tests::{CAN, CRATE, DOOR, FLOOR, WALL};
    use crate::workspace::tests::workspace;
    use crate::{AppError, DoorRequest, RoomRequest, Shape, WallRequest, Workspace};

    fn rectangle(x: i32, y: i32, width: u32, height: u32) -> Shape {
        Shape::Rectangle {
            x,
            y,
            width,
            height,
        }
    }

    /// A 12 x 8 room split at x = 6, with a door in the split and one outside.
    fn split_room() -> (Workspace, String) {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(20, 12).unwrap();
        let room = RoomRequest {
            shape: rectangle(1, 1, 12, 8),
            pillars: vec![rectangle(3, 3, 1, 1)],
            floor: FLOOR.to_owned(),
            wall: WALL.to_owned(),
        };
        workspace.add_room(&map, &room).unwrap();
        let split = WallRequest {
            points: vec![(7, 1), (7, 9)],
            wall: WALL.to_owned(),
        };
        workspace.add_wall(&map, &split).unwrap();
        for (on, x, y) in [("W0", 7.0, 5.0), ("R0", 4.0, 9.0)] {
            let door = DoorRequest {
                on: on.to_owned(),
                x,
                y,
                asset: DOOR.to_owned(),
            };
            workspace.add_door(&map, &door).unwrap();
        }
        (workspace, map)
    }

    fn strew(count: u32, seed: u64) -> ScatterRequest {
        ScatterRequest {
            assets: vec![CAN.to_owned(), CRATE.to_owned()],
            area: rectangle(0, 0, 20, 12),
            count,
            seed,
        }
    }

    #[test]
    fn strews_props_inside_rooms_clear_of_everything() {
        let (mut workspace, map) = split_room();
        let report = workspace.scatter(&map, &strew(25, 3)).unwrap();
        assert_eq!(report.props.len() + report.missing as usize, 25);
        assert!(report.props.len() >= 20, "{report:?}");
        assert_eq!(workspace.validate(&map), Ok(vec![]));
        let description = workspace.describe(&map).unwrap();
        assert!(description.contains("Debris_Can_Empty"), "{description}");
    }

    #[test]
    fn keeps_doorways_and_partitions_free() {
        let (mut workspace, map) = split_room();
        workspace.scatter(&map, &strew(60, 9)).unwrap();
        for (center, half_width, _) in workspace.prop_extents(&map) {
            let (left, right) = (center.0 - half_width, center.0 + half_width);
            assert!(right <= 7.0 || left >= 7.0, "{center:?} crosses W0");
            for door in [(7.0, 5.0), (4.0, 9.0)] {
                let distance = (center.0 - door.0).hypot(center.1 - door.1);
                assert!(distance > 1.0, "{center:?} blocks the door at {door:?}");
            }
        }
    }

    #[test]
    fn stays_within_the_area() {
        let (mut workspace, map) = split_room();
        let corner = ScatterRequest {
            area: rectangle(8, 1, 5, 4),
            ..strew(8, 1)
        };
        workspace.scatter(&map, &corner).unwrap();
        for (center, half_width, half_height) in workspace.prop_extents(&map) {
            assert!(center.0 - half_width >= 8.0 && center.0 + half_width <= 13.0);
            assert!(center.1 - half_height >= 1.0 && center.1 + half_height <= 5.0);
        }
    }

    #[test]
    fn the_same_seed_strews_the_same_way() {
        let (mut first, map) = split_room();
        let (mut second, _) = split_room();
        first.scatter(&map, &strew(12, 5)).unwrap();
        second.scatter(&map, &strew(12, 5)).unwrap();
        assert_eq!(first.describe(&map), second.describe(&map));
    }

    #[test]
    fn reports_what_found_no_room() {
        let (mut workspace, map) = split_room();
        let crowded = ScatterRequest {
            assets: vec![CRATE.to_owned()],
            area: rectangle(8, 1, 5, 4),
            ..strew(40, 2)
        };
        let report = workspace.scatter(&map, &crowded).unwrap();
        assert!(report.missing > 30, "{report:?}");
    }

    #[test]
    fn needs_objects() {
        let (mut workspace, map) = split_room();
        let walls = ScatterRequest {
            assets: vec![WALL.to_owned()],
            ..strew(3, 1)
        };
        assert!(matches!(
            workspace.scatter(&map, &walls),
            Err(AppError::WrongAssetKind { .. })
        ));
        let nothing = ScatterRequest {
            assets: vec![],
            ..strew(3, 1)
        };
        assert_eq!(
            workspace.scatter(&map, &nothing),
            Err(AppError::NothingToScatter)
        );
    }

    impl Workspace {
        /// Center and half sizes of every prop.
        fn prop_extents(&self, map: &str) -> Vec<((f64, f64), f64, f64)> {
            self.map(map)
                .unwrap()
                .1
                .props()
                .map(|(_, prop)| {
                    let (low, high) = prop.bounds();
                    (
                        (prop.center.x, prop.center.y),
                        (high.x - low.x) / 2.0,
                        (high.y - low.y) / 2.0,
                    )
                })
                .collect()
        }
    }
}
