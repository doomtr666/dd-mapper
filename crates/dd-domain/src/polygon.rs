use std::ops::Range;

use thiserror::Error;

use crate::{Cell, CellPoint, CellPosition, geometry};

/// A simple polygon on grid vertices, oriented clockwise on screen (y down).
///
/// Construction normalizes the input: collinear and repeated vertices are
/// dropped and a counter-clockwise ring is reversed, keeping its first vertex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polygon {
    vertices: Vec<CellPoint>,
}

/// A straight wall segment between two grid vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub start: CellPoint,
    pub end: CellPoint,
}

/// How far, in cells, a point may stray from a segment and still be on it.
const LOCATE_TOLERANCE: f64 = 1e-9;

impl Segment {
    /// The point at fraction `t` of the segment, from its start.
    #[must_use]
    pub fn point_at(&self, t: f64) -> CellPosition {
        let (start, (dx, dy)) = self.start_and_delta();
        CellPosition::new(start.x + dx * t, start.y + dy * t)
    }

    /// The fraction `t` in `]0, 1[` at which `position` lies on the segment.
    #[must_use]
    pub fn locate(&self, position: CellPosition) -> Option<f64> {
        self.locate_within(position, LOCATE_TOLERANCE)
    }

    /// Like [`Segment::locate`], accepting points up to `tolerance` cells away.
    #[must_use]
    pub fn locate_within(&self, position: CellPosition, tolerance: f64) -> Option<f64> {
        let (start, (dx, dy)) = self.start_and_delta();
        let (px, py) = (position.x - start.x, position.y - start.y);
        let length_squared = dx * dx + dy * dy;
        let distance_to_line = (dx * py - dy * px).abs() / length_squared.sqrt();
        if distance_to_line > tolerance {
            return None;
        }
        let t = (px * dx + py * dy) / length_squared;
        (t > LOCATE_TOLERANCE && t < 1.0 - LOCATE_TOLERANCE).then_some(t)
    }

    fn start_and_delta(&self) -> (CellPosition, (f64, f64)) {
        let start = CellPosition::new(f64::from(self.start.x), f64::from(self.start.y));
        let end = CellPosition::new(f64::from(self.end.x), f64::from(self.end.y));
        (start, (end.x - start.x, end.y - start.y))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolygonError {
    #[error("a polygon needs at least 3 distinct, non-collinear vertices, got {count}")]
    TooFewVertices { count: usize },
    #[error("the polygon crosses or touches itself")]
    SelfIntersecting,
}

const MIN_VERTICES: usize = 3;

impl Polygon {
    /// # Errors
    /// Fails when the vertices do not describe a simple polygon with an area.
    pub fn new(vertices: Vec<CellPoint>) -> Result<Self, PolygonError> {
        let vertices = without_redundant_vertices(vertices);
        if vertices.len() < MIN_VERTICES {
            return Err(PolygonError::TooFewVertices {
                count: vertices.len(),
            });
        }
        if touches_itself(&vertices) {
            return Err(PolygonError::SelfIntersecting);
        }
        Ok(Self {
            vertices: oriented_clockwise(vertices),
        })
    }

    /// An axis-aligned rectangle, starting at its bottom-right corner
    /// like Dungeondraft's room tool.
    ///
    /// # Errors
    /// Fails when the width or the height is zero.
    pub fn rectangle(top_left: CellPoint, width: u32, height: u32) -> Result<Self, PolygonError> {
        let right = top_left.x.saturating_add(saturating_i32(width));
        let bottom = top_left.y.saturating_add(saturating_i32(height));
        Self::new(vec![
            CellPoint::new(right, bottom),
            CellPoint::new(top_left.x, bottom),
            top_left,
            CellPoint::new(right, top_left.y),
        ])
    }

    #[must_use]
    pub fn vertices(&self) -> &[CellPoint] {
        &self.vertices
    }

    /// Twice the area, in square cells: exact and always positive.
    #[must_use]
    pub fn double_area(&self) -> i64 {
        geometry::double_signed_area(&self.vertices)
    }

    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.vertices.len()
    }

    /// The segment from vertex `index` to the next one (wrapping around).
    #[must_use]
    pub fn segment(&self, index: usize) -> Option<Segment> {
        let start = *self.vertices.get(index)?;
        let end = self.vertices[(index + 1) % self.vertices.len()];
        Some(Segment { start, end })
    }

    pub fn segments(&self) -> impl Iterator<Item = Segment> + '_ {
        (0..self.segment_count()).filter_map(|index| self.segment(index))
    }

    /// Whether `point` lies inside the polygon, not on its boundary.
    #[must_use]
    pub fn strictly_contains(&self, point: CellPoint) -> bool {
        geometry::strictly_inside(point, &self.vertices)
    }

    /// Cells overlapping the polygon's bounding box, row by row.
    pub fn bounding_cells(&self) -> impl Iterator<Item = Cell> {
        let columns = cell_span(self.vertices.iter().map(|vertex| vertex.x));
        let rows = cell_span(self.vertices.iter().map(|vertex| vertex.y));
        rows.flat_map(move |row| columns.clone().map(move |column| Cell::new(column, row)))
    }

    /// Whether a fractional position lies inside the polygon, boundary excluded.
    #[must_use]
    pub fn contains_position(&self, position: CellPosition) -> bool {
        let on_vertex = self.vertices.iter().any(|vertex| {
            (f64::from(vertex.x) - position.x).abs() <= LOCATE_TOLERANCE
                && (f64::from(vertex.y) - position.y).abs() <= LOCATE_TOLERANCE
        });
        let on_edge = self
            .segments()
            .any(|segment| segment.locate(position).is_some());
        if on_vertex || on_edge {
            return false;
        }
        let crossings = self
            .segments()
            .filter(|segment| ray_crosses(*segment, position))
            .count();
        crossings % 2 == 1
    }

    /// Whether the center of `cell` lies strictly inside the polygon.
    #[must_use]
    pub fn contains_cell_center(&self, cell: Cell) -> bool {
        let (Ok(column), Ok(row)) = (i32::try_from(cell.column), i32::try_from(cell.row)) else {
            return false;
        };
        let doubled =
            |point: CellPoint| CellPoint::new(point.x.saturating_mul(2), point.y.saturating_mul(2));
        let ring: Vec<CellPoint> = self.vertices.iter().copied().map(doubled).collect();
        let center = CellPoint::new(
            column.saturating_mul(2).saturating_add(1),
            row.saturating_mul(2).saturating_add(1),
        );
        geometry::strictly_inside(center, &ring)
    }

    /// Whether the interior of `cell` shares some area with the polygon.
    #[must_use]
    pub fn overlaps_cell(&self, cell: Cell) -> bool {
        self.contains_cell_center(cell) || self.cuts_cell(cell)
    }

    /// Whether the whole of `cell` lies inside the polygon.
    #[must_use]
    pub fn covers_cell(&self, cell: Cell) -> bool {
        self.contains_cell_center(cell) && !self.cuts_cell(cell)
    }

    /// Whether an edge runs through the interior of `cell`. Edges join grid
    /// vertices, so such an edge always properly crosses one of the cell diagonals.
    fn cuts_cell(&self, cell: Cell) -> bool {
        let Some(diagonals) = cell_diagonals(cell) else {
            return false;
        };
        self.segments().any(|edge| {
            diagonals
                .iter()
                .any(|diagonal| geometry::segments_cross_properly(edge, *diagonal))
        })
    }

    /// Whether `other` lies inside this polygon without touching its boundary.
    #[must_use]
    pub fn strictly_contains_polygon(&self, other: &Polygon) -> bool {
        other
            .vertices
            .iter()
            .all(|vertex| self.strictly_contains(*vertex))
            && !self.boundaries_touch(other)
    }

    /// Whether the two polygons share at least one point, boundary included.
    #[must_use]
    pub fn touches_or_overlaps(&self, other: &Polygon) -> bool {
        self.boundaries_touch(other)
            || self.strictly_contains(other.vertices[0])
            || other.strictly_contains(self.vertices[0])
    }

    fn boundaries_touch(&self, other: &Polygon) -> bool {
        self.segments().any(|own| {
            other
                .segments()
                .any(|theirs| geometry::segments_touch(own, theirs))
        })
    }
}

/// Repeatedly drops a vertex aligned with its neighbours; this also removes
/// repeated vertices and the tips of zero-width spikes.
fn without_redundant_vertices(mut ring: Vec<CellPoint>) -> Vec<CellPoint> {
    while ring.len() >= MIN_VERTICES {
        let Some(redundant) = (0..ring.len()).find(|&index| is_aligned(&ring, index)) else {
            break;
        };
        ring.remove(redundant);
    }
    ring
}

fn is_aligned(ring: &[CellPoint], index: usize) -> bool {
    let previous = ring[(index + ring.len() - 1) % ring.len()];
    let next = ring[(index + 1) % ring.len()];
    geometry::cross(previous, ring[index], next) == 0
}

/// Adjacent segments only share their common vertex once aligned vertices are
/// gone, so only non-adjacent segments need checking.
fn touches_itself(ring: &[CellPoint]) -> bool {
    let segments = geometry::ring_segments(ring);
    let last = segments.len() - 1;
    (0..segments.len()).any(|first| {
        (first + 2..segments.len())
            .filter(|&second| !(first == 0 && second == last))
            .any(|second| geometry::segments_touch(segments[first], segments[second]))
    })
}

fn oriented_clockwise(mut ring: Vec<CellPoint>) -> Vec<CellPoint> {
    if geometry::double_signed_area(&ring) < 0 {
        ring[1..].reverse();
    }
    ring
}

/// Whether the horizontal ray from `position` towards +x crosses `segment`.
fn ray_crosses(segment: Segment, position: CellPosition) -> bool {
    let (start_x, start_y) = (f64::from(segment.start.x), f64::from(segment.start.y));
    let (end_x, end_y) = (f64::from(segment.end.x), f64::from(segment.end.y));
    if (start_y > position.y) == (end_y > position.y) {
        return false;
    }
    let crossing_x = start_x + (position.y - start_y) * (end_x - start_x) / (end_y - start_y);
    position.x < crossing_x
}

fn cell_diagonals(cell: Cell) -> Option<[Segment; 2]> {
    let left = i32::try_from(cell.column).ok()?;
    let top = i32::try_from(cell.row).ok()?;
    let (right, bottom) = (left.checked_add(1)?, top.checked_add(1)?);
    Some([
        Segment {
            start: CellPoint::new(left, top),
            end: CellPoint::new(right, bottom),
        },
        Segment {
            start: CellPoint::new(right, top),
            end: CellPoint::new(left, bottom),
        },
    ])
}

/// Cell indices between the smallest and largest coordinate, clamped at zero.
fn cell_span(coordinates: impl Iterator<Item = i32>) -> Range<u32> {
    let (low, high) = coordinates.fold((i32::MAX, i32::MIN), |(low, high), coordinate| {
        (low.min(coordinate), high.max(coordinate))
    });
    let to_index = |coordinate: i32| u32::try_from(coordinate).unwrap_or(0);
    to_index(low)..to_index(high)
}

fn saturating_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{Polygon, PolygonError, Segment};
    use crate::{Cell, CellPoint, CellPosition};

    fn points(coordinates: &[(i32, i32)]) -> Vec<CellPoint> {
        coordinates
            .iter()
            .map(|&(x, y)| CellPoint::new(x, y))
            .collect()
    }

    fn polygon(coordinates: &[(i32, i32)]) -> Polygon {
        Polygon::new(points(coordinates)).expect("valid polygon")
    }

    #[test]
    fn rectangle_starts_bottom_right_like_dungeondraft() {
        let room = Polygon::rectangle(CellPoint::new(13, 5), 17, 11).unwrap();
        assert_eq!(
            room.vertices(),
            points(&[(30, 16), (13, 16), (13, 5), (30, 5)])
        );
    }

    #[test]
    fn rectangle_needs_an_area() {
        assert_eq!(
            Polygon::rectangle(CellPoint::new(0, 0), 0, 3),
            Err(PolygonError::TooFewVertices { count: 2 })
        );
    }

    #[test]
    fn needs_three_vertices() {
        assert_eq!(
            Polygon::new(points(&[(0, 0), (4, 0)])),
            Err(PolygonError::TooFewVertices { count: 2 })
        );
    }

    #[test]
    fn keeps_a_clockwise_ring_unchanged() {
        let clockwise = [(30, 16), (13, 16), (13, 5), (30, 5)];
        assert_eq!(polygon(&clockwise).vertices(), points(&clockwise));
    }

    #[test]
    fn reverses_a_counter_clockwise_ring_from_its_first_vertex() {
        let counter_clockwise = polygon(&[(30, 16), (30, 5), (13, 5), (13, 16)]);
        assert_eq!(
            counter_clockwise.vertices(),
            points(&[(30, 16), (13, 16), (13, 5), (30, 5)])
        );
    }

    #[test]
    fn drops_collinear_and_repeated_vertices() {
        let square = polygon(&[(0, 0), (2, 0), (4, 0), (4, 4), (4, 4), (0, 4)]);
        assert_eq!(square.vertices(), points(&[(0, 0), (4, 0), (4, 4), (0, 4)]));
    }

    #[test]
    fn rejects_flat_rings() {
        assert_eq!(
            Polygon::new(points(&[(0, 0), (2, 0), (4, 0), (2, 0)])),
            Err(PolygonError::TooFewVertices { count: 2 })
        );
    }

    #[test]
    fn rejects_crossing_edges() {
        assert_eq!(
            Polygon::new(points(&[(0, 0), (4, 4), (4, 0), (0, 4)])),
            Err(PolygonError::SelfIntersecting)
        );
    }

    #[test]
    fn rejects_a_ring_touching_itself() {
        let pinched = points(&[(0, 0), (4, 0), (2, 2), (4, 4), (0, 4), (2, 2)]);
        assert_eq!(Polygon::new(pinched), Err(PolygonError::SelfIntersecting));
    }

    #[test]
    fn points_along_a_segment_round_trip() {
        let segment = Segment {
            start: CellPoint::new(8, 4),
            end: CellPoint::new(0, 4),
        };
        assert_eq!(segment.point_at(0.625), CellPosition::new(3.0, 4.0));
        assert_eq!(segment.locate(CellPosition::new(3.0, 4.0)), Some(0.625));
    }

    #[test]
    fn locates_points_near_the_segment_within_a_tolerance() {
        let segment = Segment {
            start: CellPoint::new(0, 0),
            end: CellPoint::new(4, 0),
        };
        let near = CellPosition::new(1.0, 0.004);
        assert_eq!(segment.locate(near), None);
        assert_eq!(segment.locate_within(near, 0.01), Some(0.25));
    }

    #[test]
    fn locates_only_points_strictly_inside_the_segment() {
        let segment = Segment {
            start: CellPoint::new(0, 0),
            end: CellPoint::new(4, 4),
        };
        assert_eq!(segment.locate(CellPosition::new(1.0, 1.0)), Some(0.25));
        assert_eq!(segment.locate(CellPosition::new(1.0, 1.5)), None);
        assert_eq!(segment.locate(CellPosition::new(0.0, 0.0)), None);
        assert_eq!(segment.locate(CellPosition::new(5.0, 5.0)), None);
    }

    #[test]
    fn segments_wrap_around() {
        let room = Polygon::rectangle(CellPoint::new(13, 5), 17, 11).unwrap();
        assert_eq!(room.segment_count(), 4);
        assert_eq!(
            room.segment(1),
            Some(Segment {
                start: CellPoint::new(13, 16),
                end: CellPoint::new(13, 5)
            })
        );
        assert_eq!(
            room.segment(3),
            Some(Segment {
                start: CellPoint::new(30, 5),
                end: CellPoint::new(30, 16)
            })
        );
        assert_eq!(room.segment(4), None);
    }

    #[test]
    fn contains_the_centers_of_cells_inside_its_area() {
        let triangle = polygon(&[(0, 0), (4, 0), (0, 4)]);
        assert!(triangle.contains_cell_center(Cell::new(0, 0)));
        assert!(triangle.contains_cell_center(Cell::new(2, 0)));
        assert!(!triangle.contains_cell_center(Cell::new(3, 0)));
        assert!(!triangle.contains_cell_center(Cell::new(1, 2)));
        assert!(!triangle.contains_cell_center(Cell::new(4, 4)));
    }

    #[test]
    fn overlaps_cells_cut_by_its_edges() {
        let triangle = polygon(&[(0, 0), (4, 0), (0, 4)]);
        assert!(triangle.overlaps_cell(Cell::new(0, 0)));
        assert!(triangle.overlaps_cell(Cell::new(3, 0)));
        assert!(triangle.overlaps_cell(Cell::new(1, 2)));
        assert!(!triangle.overlaps_cell(Cell::new(2, 2)));
        assert!(!triangle.overlaps_cell(Cell::new(4, 0)));
    }

    #[test]
    fn covers_only_cells_entirely_inside() {
        let triangle = polygon(&[(0, 0), (4, 0), (0, 4)]);
        assert!(triangle.covers_cell(Cell::new(0, 0)));
        assert!(triangle.covers_cell(Cell::new(2, 0)));
        assert!(!triangle.covers_cell(Cell::new(3, 0)));
        assert!(!triangle.covers_cell(Cell::new(1, 2)));
        assert!(!triangle.covers_cell(Cell::new(5, 5)));
    }

    #[test]
    fn contains_fractional_positions_inside_only() {
        let l_shape = polygon(&[(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)]);
        assert!(l_shape.contains_position(CellPosition::new(1.5, 3.5)));
        assert!(l_shape.contains_position(CellPosition::new(3.9, 1.9)));
        assert!(!l_shape.contains_position(CellPosition::new(3.0, 3.0)));
        assert!(!l_shape.contains_position(CellPosition::new(2.0, 3.0)));
        assert!(!l_shape.contains_position(CellPosition::new(-0.1, 1.0)));
    }

    #[test]
    fn strictly_contains_interior_points_of_a_concave_polygon() {
        let l_shape = polygon(&[(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)]);
        assert!(l_shape.strictly_contains(CellPoint::new(1, 3)));
        assert!(l_shape.strictly_contains(CellPoint::new(3, 1)));
        assert!(!l_shape.strictly_contains(CellPoint::new(3, 3)));
        assert!(!l_shape.strictly_contains(CellPoint::new(2, 3)));
        assert!(!l_shape.strictly_contains(CellPoint::new(0, 0)));
        assert!(!l_shape.strictly_contains(CellPoint::new(9, 1)));
    }
}
