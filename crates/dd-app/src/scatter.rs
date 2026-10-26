//! Clutter strewn at random over an area, or lined along its walls: debris,
//! cans, papers, cables, crates, lockers...

use dd_catalog::AssetKind;
use dd_domain::{AssetPath, CellPosition, Footprint, Map, Polygon, Prop, QuarterTurn, Segment};
use dd_gen::Random;

use crate::assets::resolve;
use crate::color::parse_color;
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
/// Space left between a wall and the props lined along it, in cells.
const WALL_GAP: f64 = 0.05;
/// Props whose longest side is shorter than this barely show on a map, in cells.
const VISIBLE_SIDE: f64 = 0.35;

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
    /// Line the props along the walls, backs to the wall, instead of
    /// strewing them anywhere.
    pub along_walls: bool,
    /// `RRGGBB` or `AARRGGBB` custom color given to every prop.
    pub color: Option<String>,
}

/// What a scatter placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterReport {
    pub props: Vec<String>,
    /// Props asked for that found no free spot.
    pub missing: u32,
    /// Names of the assets too small to show on a map.
    pub tiny: Vec<String>,
}

/// An object to pick, and the space it takes.
struct Piece {
    asset: AssetPath,
    name: String,
    footprint: Footprint,
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
        let tint = request.color.as_deref().map(parse_color).transpose()?;
        let area = request.area.to_polygon()?;
        let target = self.map_mut(map)?;
        let walls = if request.along_walls {
            straight_walls(target)
        } else {
            Vec::new()
        };
        let mut random = Random::new(request.seed);
        let mut props = Vec::new();
        for _ in 0..request.count {
            let spot = (0..ATTEMPTS_PER_PROP)
                .filter_map(|_| {
                    let piece = pick(&palette, &mut random);
                    if request.along_walls {
                        against_wall(piece, &walls, &mut random)
                    } else {
                        Some(anywhere(piece, &area, &mut random))
                    }
                })
                .find(|prop| fits(target, &area, prop));
            if let Some(prop) = spot {
                props.push(prop_label(target.add_prop(Prop { tint, ..prop })?));
            }
        }
        let placed = u32::try_from(props.len()).unwrap_or(request.count);
        Ok(ScatterReport {
            props,
            missing: request.count - placed,
            tiny: tiny_names(&palette),
        })
    }

    /// The objects to pick from, with their footprints.
    fn palette(&self, assets: &[String]) -> Result<Vec<Piece>, AppError> {
        if assets.is_empty() {
            return Err(AppError::NothingToScatter);
        }
        assets
            .iter()
            .map(|path| {
                let asset = resolve(self.catalog(), path, AssetKind::Object)?;
                Ok(Piece {
                    asset: asset.path.clone(),
                    name: asset.name.clone(),
                    footprint: footprint(asset)?,
                })
            })
            .collect()
    }
}

fn pick<'p>(palette: &'p [Piece], random: &mut Random) -> &'p Piece {
    let last = u32::try_from(palette.len() - 1).unwrap_or(0);
    &palette[random.between(0, last) as usize]
}

fn tiny_names(palette: &[Piece]) -> Vec<String> {
    palette
        .iter()
        .filter(|piece| piece.footprint.width().max(piece.footprint.height()) < VISIBLE_SIDE)
        .map(|piece| piece.name.clone())
        .collect()
}

/// The piece turned at random, centered anywhere in the area's bounds.
fn anywhere(piece: &Piece, area: &Polygon, random: &mut Random) -> Prop {
    let turns = random.between(0, 3).cast_signed();
    let (low, high) = bounds(area);
    Prop {
        asset: piece.asset.clone(),
        center: CellPosition::new(
            between(low.x, high.x, random),
            between(low.y, high.y, random),
        ),
        rotation: QuarterTurn::from_degrees(turns * 90).unwrap_or(QuarterTurn::Deg0),
        footprint: piece.footprint,
        tint: None,
    }
}

/// The piece against a random side of a random wall, facing away from it.
fn against_wall(piece: &Piece, walls: &[Segment], random: &mut Random) -> Option<Prop> {
    let last = u32::try_from(walls.len().checked_sub(1)?).ok()?;
    let wall = walls[random.between(0, last) as usize];
    let point = wall.point_at(between(0.0, 1.0, random));
    let (dx, dy) = (
        (wall.end.x - wall.start.x).signum(),
        (wall.end.y - wall.start.y).signum(),
    );
    let side = if random.coin() { 1 } else { -1 };
    let (nx, ny) = (-dy * side, dx * side);
    let rotation = facing(nx, ny);
    let turned = piece.footprint.turned(rotation);
    let depth = if nx == 0 {
        turned.height()
    } else {
        turned.width()
    };
    let offset = depth / 2.0 + WALL_GAP;
    Some(Prop {
        asset: piece.asset.clone(),
        center: CellPosition::new(
            point.x + f64::from(nx) * offset,
            point.y + f64::from(ny) * offset,
        ),
        rotation,
        footprint: piece.footprint,
        tint: None,
    })
}

/// The turn making a prop face `(nx, ny)`: LORE art faces down at 0 degrees.
fn facing(nx: i32, ny: i32) -> QuarterTurn {
    match (nx, ny) {
        (0, 1) => QuarterTurn::Deg0,
        (-1, 0) => QuarterTurn::Deg90,
        (0, -1) => QuarterTurn::Deg180,
        _ => QuarterTurn::Deg270,
    }
}

/// Every horizontal or vertical wall: rooms, pillars and partitions.
fn straight_walls(map: &Map) -> Vec<Segment> {
    let mut walls: Vec<Segment> = map
        .rooms()
        .flat_map(|(_, room)| {
            room.wall_loops()
                .flat_map(|(_, ring)| ring.segments())
                .collect::<Vec<_>>()
        })
        .collect();
    walls.extend(
        map.partitions()
            .flat_map(|(_, partition)| partition.segments().collect::<Vec<_>>()),
    );
    walls.retain(|wall| wall.start.x == wall.end.x || wall.start.y == wall.end.y);
    walls
}

/// Inside the area and a room or a cave, crossing no wall, prop or doorway.
fn fits(map: &Map, area: &Polygon, prop: &Prop) -> bool {
    let (low, high) = prop.bounds();
    let corners = [
        low,
        CellPosition::new(high.x, low.y),
        high,
        CellPosition::new(low.x, high.y),
    ];
    let in_area = corners.iter().all(|corner| area.contains_position(*corner));
    let in_room = map
        .rooms()
        .any(|(_, room)| room.outline().contains_position(prop.center));
    let in_cave = corners
        .iter()
        .chain([&prop.center])
        .all(|point| map.cave().is_floor_at(*point));
    let clear_of_rooms = !map.rooms().any(|(_, room)| crosses_walls(room, prop));
    in_area
        && (in_room || in_cave)
        && clear_of_rooms
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
    use dd_domain::QuarterTurn;

    use super::ScatterRequest;
    use crate::assets::tests::{CAN, CRATE, DOOR, FLOOR, WALL};
    use crate::workspace::tests::workspace;
    use crate::{
        AppError, CaveRequest, CaveShape, DoorRequest, RoomRequest, Shape, WallRequest, Workspace,
    };

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
            along_walls: false,
            color: None,
        }
    }

    #[test]
    fn colors_every_strewn_prop() {
        let (mut workspace, map) = split_room();
        let green = ScatterRequest {
            color: Some("5aff3c".to_owned()),
            ..strew(6, 2)
        };
        workspace.scatter(&map, &green).unwrap();
        let (_, found) = workspace.map(&map).unwrap();
        assert!(
            found
                .props()
                .all(|(_, prop)| prop.tint == Some(0xff5a_ff3c))
        );
    }

    /// Room walls x = 1, 13 and y = 1, 9; the split x = 7; the pillar's sides.
    fn gap_to_walls((low, high): ((f64, f64), (f64, f64))) -> f64 {
        let verticals = [1.0, 3.0, 4.0, 7.0, 13.0];
        let horizontals = [1.0, 3.0, 4.0, 9.0];
        let gaps_x = verticals
            .iter()
            .flat_map(|x| [(low.0 - x).abs(), (high.0 - x).abs()]);
        let gaps_y = horizontals
            .iter()
            .flat_map(|y| [(low.1 - y).abs(), (high.1 - y).abs()]);
        gaps_x.chain(gaps_y).fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn lines_walls_with_props_facing_the_room() {
        let (mut workspace, map) = split_room();
        let along = ScatterRequest {
            assets: vec![CRATE.to_owned()],
            along_walls: true,
            ..strew(12, 8)
        };
        let report = workspace.scatter(&map, &along).unwrap();
        assert!(report.props.len() >= 8, "{report:?}");
        assert_eq!(workspace.validate(&map), Ok(vec![]));
        let (_, found) = workspace.map(&map).unwrap();
        for (_, prop) in found.props() {
            let (low, high) = prop.bounds();
            let gap = gap_to_walls(((low.x, low.y), (high.x, high.y)));
            assert!(gap < 0.2, "{prop:?} stands {gap} from the walls");
            if (low.y - 1.0).abs() < 0.2 {
                assert_eq!(
                    prop.rotation,
                    QuarterTurn::Deg0,
                    "{prop:?} under the top wall"
                );
            }
        }
    }

    #[test]
    fn strews_cave_floors_too() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let chamber = CaveRequest {
            shape: CaveShape::Chamber {
                x: 15.0,
                y: 10.0,
                radius: 5.0,
            },
            fill: false,
            roughness: 0.0,
            seed: 1,
        };
        workspace.carve_cave(&map, &chamber).unwrap();
        let report = workspace.scatter(&map, &strew(10, 6)).unwrap();
        assert_eq!(report.props.len(), 10, "{report:?}");
        let (_, found) = workspace.map(&map).unwrap();
        for (_, prop) in found.props() {
            let (low, high) = prop.bounds();
            for corner in [low, high, prop.center] {
                assert!(found.cave().is_floor_at(corner), "{prop:?}");
            }
        }
        assert_eq!(workspace.validate(&map), Ok(vec![]));
    }

    #[test]
    fn cave_props_keep_off_the_walls_of_rooms_built_inside() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let hall = CaveRequest {
            shape: CaveShape::Chamber {
                x: 15.0,
                y: 10.0,
                radius: 7.0,
            },
            fill: false,
            roughness: 0.0,
            seed: 1,
        };
        workspace.carve_cave(&map, &hall).unwrap();
        let outpost = RoomRequest {
            shape: rectangle(13, 8, 4, 4),
            pillars: vec![],
            floor: FLOOR.to_owned(),
            wall: WALL.to_owned(),
        };
        workspace.add_room(&map, &outpost).unwrap();
        workspace.scatter(&map, &strew(60, 4)).unwrap();
        let (_, found) = workspace.map(&map).unwrap();
        let (_, room) = found.rooms().next().unwrap();
        for (_, prop) in found.props() {
            assert!(!crate::validate::crosses_walls(room, prop), "{prop:?}");
        }
    }

    #[test]
    fn flags_assets_too_small_to_show() {
        let (mut workspace, map) = split_room();
        let report = workspace.scatter(&map, &strew(4, 1)).unwrap();
        assert_eq!(report.tiny, ["Debris_Can_Empty"]);
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
