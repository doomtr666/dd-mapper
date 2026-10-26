//! Exact integer geometry primitives on grid vertices.

use crate::{CellPoint, Segment};

/// Cross product of `origin → a` and `origin → b`.
/// Positive when `b` is clockwise from `a` on screen (y down).
pub fn cross(origin: CellPoint, a: CellPoint, b: CellPoint) -> i64 {
    let (ax, ay) = offset(origin, a);
    let (bx, by) = offset(origin, b);
    ax * by - ay * bx
}

/// Twice the signed area (shoelace); positive for a clockwise ring on screen.
pub fn double_signed_area(ring: &[CellPoint]) -> i64 {
    let next_vertices = ring.iter().cycle().skip(1);
    ring.iter()
        .zip(next_vertices)
        .map(|(current, next)| {
            i64::from(current.x) * i64::from(next.y) - i64::from(next.x) * i64::from(current.y)
        })
        .sum()
}

/// Whether the segments cross at a single point inside both of them.
pub fn segments_cross_properly(first: Segment, second: Segment) -> bool {
    strictly_opposite(sides(first, second)) && strictly_opposite(sides(second, first))
}

/// Whether two segments share at least one point.
pub fn segments_touch(first: Segment, second: Segment) -> bool {
    segments_cross_properly(first, second)
        || lies_on(first.start, second)
        || lies_on(first.end, second)
        || lies_on(second.start, first)
        || lies_on(second.end, first)
}

/// Whether two segments lie on the same line and share a positive length.
pub fn segments_overlap_collinearly(first: Segment, second: Segment) -> bool {
    let collinear = cross(first.start, first.end, second.start) == 0
        && cross(first.start, first.end, second.end) == 0;
    if !collinear {
        return false;
    }
    let along_first = |point: CellPoint| dot(first.start, first.end, point);
    let first_span = (0, along_first(first.end));
    let (second_from, second_to) = (along_first(second.start), along_first(second.end));
    let low = first_span.0.max(second_from.min(second_to));
    let high = first_span.1.min(second_from.max(second_to));
    low < high
}

pub fn lies_on(point: CellPoint, segment: Segment) -> bool {
    cross(segment.start, segment.end, point) == 0
        && within(point.x, segment.start.x, segment.end.x)
        && within(point.y, segment.start.y, segment.end.y)
}

/// Even-odd ray casting; `false` for points on the boundary.
pub fn strictly_inside(point: CellPoint, ring: &[CellPoint]) -> bool {
    let segments = ring_segments(ring);
    if segments.iter().any(|segment| lies_on(point, *segment)) {
        return false;
    }
    let crossings = segments
        .iter()
        .filter(|segment| ray_crosses(point, **segment))
        .count();
    crossings % 2 == 1
}

pub fn ring_segments(ring: &[CellPoint]) -> Vec<Segment> {
    let next_vertices = ring.iter().cycle().skip(1);
    ring.iter()
        .zip(next_vertices)
        .map(|(start, end)| Segment {
            start: *start,
            end: *end,
        })
        .collect()
}

/// Dot product of `origin → direction_end` and `origin → point`.
fn dot(origin: CellPoint, direction_end: CellPoint, point: CellPoint) -> i64 {
    let (dx, dy) = offset(origin, direction_end);
    let (px, py) = offset(origin, point);
    dx * px + dy * py
}

fn offset(from: CellPoint, to: CellPoint) -> (i64, i64) {
    (
        i64::from(to.x) - i64::from(from.x),
        i64::from(to.y) - i64::from(from.y),
    )
}

/// Sides of `segment`'s endpoints relative to the line through `reference`.
fn sides(segment: Segment, reference: Segment) -> (i64, i64) {
    (
        cross(reference.start, reference.end, segment.start).signum(),
        cross(reference.start, reference.end, segment.end).signum(),
    )
}

fn strictly_opposite((first, second): (i64, i64)) -> bool {
    first * second < 0
}

fn within(value: i32, bound: i32, other_bound: i32) -> bool {
    value >= bound.min(other_bound) && value <= bound.max(other_bound)
}

/// Whether the horizontal ray from `point` towards +x crosses `segment`.
fn ray_crosses(point: CellPoint, segment: Segment) -> bool {
    let straddles = (segment.start.y > point.y) != (segment.end.y > point.y);
    if !straddles {
        return false;
    }
    let goes_down = segment.end.y > segment.start.y;
    (cross(segment.start, segment.end, point) > 0) == goes_down
}
