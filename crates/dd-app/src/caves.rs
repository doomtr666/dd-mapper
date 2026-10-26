//! Caves dug with organic walls: round chambers, winding tunnels, or exact
//! areas, and the openings where paths enter them.

use std::f64::consts::TAU;

use dd_domain::CellPosition;
use dd_gen::{CaveLayout, CaveParams, Random, cave};

use crate::{AppError, Shape, Workspace};

/// Starting rock of networks when none is asked for: open caverns.
const DEFAULT_DENSITY: f64 = 0.47;

/// How far walls stray from the shape at full roughness, as a share of its size.
const MAX_STRAY: f64 = 0.45;
/// Waves summed into rough walls.
const WAVES: u32 = 4;

/// An area with natural edges, in cells: what caves dig and terrain paints.
#[derive(Debug, Clone, PartialEq)]
pub enum NaturalShape {
    /// A chamber around `(x, y)`.
    Chamber { x: f64, y: f64, radius: f64 },
    /// A gallery `width` cells wide along `points`.
    Tunnel { points: Vec<(f64, f64)>, width: f64 },
    /// Exactly an area: straight walls, for mines and cut passages.
    Area(Shape),
}

/// Digs (or fills back) a cave shape.
#[derive(Debug, Clone, PartialEq)]
pub struct CaveRequest {
    pub shape: NaturalShape,
    /// Fill the shape with rock instead of digging it.
    pub fill: bool,
    /// From 0 (smooth) to 1 (very jagged) walls for chambers and tunnels.
    pub roughness: f64,
    /// The same seed gives the same walls.
    pub seed: u64,
}

/// A network of caverns grown over a rectangle of cells, every part reachable.
#[derive(Debug, Clone, PartialEq)]
pub struct CaveNetwork {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Share of rock to start from, 0.47 by default: lower is more open.
    pub density: Option<f64>,
    /// The same seed grows the same network.
    pub seed: u64,
}

impl Workspace {
    /// Grows a cave network over an area, with walls smoothed to a quarter
    /// cell; returns how many samples were dug.
    ///
    /// # Errors
    /// Fails on an unknown map.
    pub fn grow_caves(&mut self, map: &str, network: &CaveNetwork) -> Result<usize, AppError> {
        let layout = cave(CaveParams {
            width: network.width,
            height: network.height,
            density: network.density.unwrap_or(DEFAULT_DENSITY),
            seed: network.seed,
        });
        let origin = (f64::from(network.x), f64::from(network.y));
        Ok(self
            .map_mut(map)?
            .cave_mut()
            .dig(|position| smoothed(&layout, origin, position) >= 0.5, true))
    }

    /// Digs or fills a cave shape; returns how many quarter-cell samples changed.
    ///
    /// # Errors
    /// Fails on an unknown map or an invalid shape.
    pub fn carve_cave(&mut self, map: &str, request: &CaveRequest) -> Result<usize, AppError> {
        let inside = region(&request.shape, request.roughness, request.seed)?;
        Ok(self.map_mut(map)?.cave_mut().dig(inside, !request.fill))
    }

    /// Leaves the cave walls open over a shape, where a path or a door enters;
    /// returns how many samples changed.
    ///
    /// # Errors
    /// Fails on an unknown map or an invalid shape.
    pub fn open_cave_entrance(
        &mut self,
        map: &str,
        shape: &NaturalShape,
    ) -> Result<usize, AppError> {
        let inside = region(shape, 0.0, 0)?;
        Ok(self.map_mut(map)?.cave_mut().mark_entrance(inside, true))
    }
}

type Region = Box<dyn Fn(CellPosition) -> bool>;

/// How far inside a shape a position lies, in cells: positive inside, zero on
/// its edge, negative outside.
pub(crate) type Margin = Box<dyn Fn(CellPosition) -> f64>;

/// The positions inside `shape`, its walls strayed by `roughness`.
fn region(shape: &NaturalShape, roughness: f64, seed: u64) -> Result<Region, AppError> {
    let margin = margin(shape, roughness, seed)?;
    Ok(Box::new(move |position: CellPosition| {
        margin(position) >= 0.0
    }))
}

/// The margin of `shape`, its edges strayed by `roughness`. Areas are exact:
/// one cell in or out.
pub(crate) fn margin(shape: &NaturalShape, roughness: f64, seed: u64) -> Result<Margin, AppError> {
    let stray = roughness.clamp(0.0, 1.0) * MAX_STRAY;
    match shape {
        &NaturalShape::Chamber { x, y, radius } => {
            positive(radius, "a chamber radius")?;
            let wobble = Wobble::around(seed);
            Ok(Box::new(move |position: CellPosition| {
                let (dx, dy) = (position.x - x, position.y - y);
                let reach = radius * (1.0 + stray * wobble.at(dy.atan2(dx)));
                reach - dx.hypot(dy)
            }))
        }
        NaturalShape::Tunnel { points, width } => {
            positive(*width, "a tunnel width")?;
            if points.len() < 2 {
                return Err(AppError::InvalidNaturalShape(
                    "a tunnel needs at least 2 points".to_owned(),
                ));
            }
            let line = points.clone();
            let half = width / 2.0;
            let wobble = Wobble::along(seed);
            Ok(Box::new(move |position: CellPosition| {
                let (distance, along) = nearest_on_line(&line, position);
                half * (1.0 + stray * wobble.at(along)) - distance
            }))
        }
        NaturalShape::Area(shape) => {
            let area = shape.to_polygon()?;
            Ok(Box::new(move |position: CellPosition| {
                if area.contains_position(position) {
                    1.0
                } else {
                    -1.0
                }
            }))
        }
    }
}

/// How much cave floor surrounds `position`, between 0 and 1, blending the
/// four nearest cell centers of the network.
fn smoothed(layout: &CaveLayout, (x, y): (f64, f64), position: CellPosition) -> f64 {
    let (u, v) = (position.x - x - 0.5, position.y - y - 0.5);
    let (column, row) = (u.floor(), v.floor());
    let (fx, fy) = (u - column, v - row);
    #[expect(clippy::cast_possible_truncation, reason = "cells of a map")]
    let (column, row) = (column as i64, row as i64);
    let floor = |dx: i64, dy: i64| {
        if layout.is_floor(column + dx, row + dy) {
            1.0
        } else {
            0.0
        }
    };
    let top = floor(0, 0) * (1.0 - fx) + floor(1, 0) * fx;
    let bottom = floor(0, 1) * (1.0 - fx) + floor(1, 1) * fx;
    top * (1.0 - fy) + bottom * fy
}

fn positive(value: f64, what: &str) -> Result<(), AppError> {
    if value.is_finite() && value > 0.0 {
        return Ok(());
    }
    Err(AppError::InvalidNaturalShape(format!(
        "{what} must be positive, got {value}"
    )))
}

/// Distance from `position` to the polyline, and how far along it the nearest
/// point lies, in cells.
fn nearest_on_line(line: &[(f64, f64)], position: CellPosition) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    let mut travelled = 0.0;
    for pair in line.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let length = dx.hypot(dy);
        let t = if length > 0.0 {
            (((position.x - ax) * dx + (position.y - ay) * dy) / (length * length)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let distance = (ax + t * dx - position.x).hypot(ay + t * dy - position.y);
        if distance < best.0 {
            best = (distance, travelled + t * length);
        }
        travelled += length;
    }
    best
}

/// A smooth random wave between -1 and 1: a few sines of random phases.
struct Wobble {
    waves: Vec<(f64, f64, f64)>,
}

impl Wobble {
    /// Waves closing on themselves around a full turn, for chambers.
    fn around(seed: u64) -> Self {
        Self::new(seed, |index| f64::from(index + 2))
    }

    /// Waves a few cells long, along tunnels.
    fn along(seed: u64) -> Self {
        Self::new(seed, |index| 0.35 * f64::from(index + 1))
    }

    /// Wave `index` has frequency `frequency(index)` and weight `1 / (index + 1)`.
    fn new(seed: u64, frequency: fn(u32) -> f64) -> Self {
        let mut random = Random::new(seed);
        let waves = (0..WAVES)
            .map(|index| {
                let phase = TAU * f64::from(random.between(0, 999)) / 1000.0;
                (frequency(index), phase, 1.0 / f64::from(index + 1))
            })
            .collect();
        Self { waves }
    }

    fn at(&self, t: f64) -> f64 {
        let total: f64 = self.waves.iter().map(|&(_, _, weight)| weight).sum();
        let sum: f64 = self
            .waves
            .iter()
            .map(|&(frequency, phase, weight)| weight * (frequency * t + phase).sin())
            .sum();
        sum / total
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::CellPosition;

    use super::{CaveNetwork, CaveRequest, NaturalShape};
    use crate::assets::tests::CRATE;
    use crate::workspace::tests::workspace;
    use crate::{AppError, PropRequest, Shape, Workspace};

    fn chamber(radius: f64, roughness: f64, seed: u64) -> CaveRequest {
        CaveRequest {
            shape: NaturalShape::Chamber {
                x: 10.0,
                y: 8.0,
                radius,
            },
            fill: false,
            roughness,
            seed,
        }
    }

    fn floor_cells(workspace: &Workspace, map: &str) -> f64 {
        let (_, found) = workspace.map(map).unwrap();
        let cave = found.cave();
        let samples = (0..cave.rows())
            .flat_map(|row| (0..cave.columns()).map(move |column| (column, row)))
            .filter(|&(column, row)| cave.is_floor(column, row))
            .count();
        f64::from(u32::try_from(samples).unwrap()) / 16.0
    }

    fn is_floor(workspace: &Workspace, map: &str, x: f64, y: f64) -> bool {
        let (_, found) = workspace.map(map).unwrap();
        found.cave().is_floor_at(CellPosition::new(x, y))
    }

    #[test]
    fn chambers_have_the_area_of_their_radius() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert!(workspace.carve_cave(&map, &chamber(3.0, 0.0, 1)).unwrap() > 0);
        let area = floor_cells(&workspace, &map);
        assert!((area - 28.3).abs() < 2.0, "{area}");
        assert!(
            workspace
                .describe(&map)
                .unwrap()
                .contains("Cave: about 28 cells of floor")
        );
    }

    #[test]
    fn rough_walls_depend_on_the_seed_only() {
        let carve = |seed| {
            let (mut workspace, _) = workspace();
            let map = workspace.new_map(30, 20).unwrap();
            workspace
                .carve_cave(&map, &chamber(4.0, 0.5, seed))
                .unwrap();
            let (_, found) = workspace.map(&map).unwrap();
            found.cave().clone()
        };
        assert_eq!(carve(7), carve(7));
        assert_ne!(carve(7), carve(8));
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace.carve_cave(&map, &chamber(4.0, 0.5, 7)).unwrap();
        assert!(is_floor(&workspace, &map, 10.0, 8.0));
        let area = floor_cells(&workspace, &map);
        assert!(area > 25.0 && area < 80.0, "{area}");
    }

    #[test]
    fn tunnels_join_their_ends() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let tunnel = CaveRequest {
            shape: NaturalShape::Tunnel {
                points: vec![(2.0, 2.0), (12.0, 4.0), (20.0, 15.0)],
                width: 2.0,
            },
            ..chamber(1.0, 0.4, 3)
        };
        workspace.carve_cave(&map, &tunnel).unwrap();
        for (x, y) in [
            (2.0, 2.0),
            (7.0, 3.0),
            (12.0, 4.0),
            (16.0, 9.5),
            (20.0, 15.0),
        ] {
            assert!(is_floor(&workspace, &map, x, y), "({x}, {y})");
        }
        assert!(!is_floor(&workspace, &map, 2.0, 15.0));
    }

    #[test]
    fn filling_puts_the_rock_back() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let dug = workspace.carve_cave(&map, &chamber(3.0, 0.0, 1)).unwrap();
        let fill = CaveRequest {
            fill: true,
            ..chamber(3.0, 0.0, 1)
        };
        assert_eq!(workspace.carve_cave(&map, &fill), Ok(dug));
        assert!(!workspace.describe(&map).unwrap().contains("Cave:"));
    }

    #[test]
    fn areas_dig_exactly_and_entrances_open_walls() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let mine = CaveRequest {
            shape: NaturalShape::Area(Shape::Rectangle {
                x: 2,
                y: 2,
                width: 4,
                height: 3,
            }),
            ..chamber(1.0, 0.9, 1)
        };
        workspace.carve_cave(&map, &mine).unwrap();
        assert!((floor_cells(&workspace, &map) - 12.0).abs() < 2.5);
        let opening = NaturalShape::Chamber {
            x: 6.0,
            y: 3.5,
            radius: 0.6,
        };
        assert!(workspace.open_cave_entrance(&map, &opening).unwrap() > 0);
        assert!(workspace.describe(&map).unwrap().contains("with entrances"));
    }

    #[test]
    fn props_on_cave_floors_are_in_place() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        workspace.carve_cave(&map, &chamber(3.0, 0.0, 1)).unwrap();
        let boulder = PropRequest {
            asset: CRATE.to_owned(),
            x: 10.0,
            y: 8.0,
            rotation_degrees: 0.0,
            color: None,
        };
        workspace.place_prop(&map, &boulder).unwrap();
        assert_eq!(workspace.validate(&map), Ok(vec![]));
    }

    #[test]
    fn networks_grow_within_their_area() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let network = CaveNetwork {
            x: 2,
            y: 2,
            width: 16,
            height: 12,
            density: None,
            seed: 4,
        };
        assert!(workspace.grow_caves(&map, &network).unwrap() > 0);
        let (_, found) = workspace.map(&map).unwrap();
        let cave = found.cave();
        for row in 0..cave.rows() {
            for column in 0..cave.columns() {
                if cave.is_floor(column, row) {
                    let at = dd_domain::CaveGrid::position(column, row);
                    assert!(
                        at.x > 2.0 && at.x < 18.0 && at.y > 2.0 && at.y < 14.0,
                        "{at:?}"
                    );
                }
            }
        }
        let regrown = {
            let (mut again, _) = crate::workspace::tests::workspace();
            let map = again.new_map(30, 20).unwrap();
            again.grow_caves(&map, &network).unwrap();
            let (_, found) = again.map(&map).unwrap();
            found.cave().clone()
        };
        assert_eq!(&regrown, cave);
    }

    #[test]
    fn shapes_need_a_size() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert!(matches!(
            workspace.carve_cave(&map, &chamber(0.0, 0.0, 1)),
            Err(AppError::InvalidNaturalShape(_))
        ));
    }
}
