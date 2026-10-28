//! Merging rooms the way Dungeondraft's room tool does.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::core::single::SingleIntOverlay;
use i_overlay::i_float::int::point::IntPoint;
use thiserror::Error;

use crate::{CellPoint, Polygon, Room, geometry};

/// A room as `i_overlay` contours: its outline then its pillars, read even-odd.
type Contours = Vec<Vec<IntPoint>>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MergeError {
    #[error("the merged rooms do not form a single room")]
    NotOneRoom,
    #[error("the merged walls would touch themselves; move the room so walls meet along a length")]
    InvalidWalls,
}

/// Whether two rooms share some area or a wall length, and so must merge.
/// Rooms meeting at a single point stay apart.
#[must_use]
pub fn joins(first: &Room, second: &Room) -> bool {
    share_area(first, second) || share_wall_length(first, second)
}

/// The union of `rooms`, walled with the first room's texture.
/// Enclosed empty areas become pillars.
///
/// # Errors
/// Fails when the union is not one room with valid walls.
pub fn merge(rooms: &[&Room]) -> Result<Room, MergeError> {
    let Some((first, others)) = rooms.split_first() else {
        return Err(MergeError::NotOneRoom);
    };
    let mut union: Vec<Contours> = vec![contours(first)];
    for room in others {
        union = union.overlay(&contours(room), OverlayRule::Union, FillRule::EvenOdd);
    }
    let [shape] = union.as_slice() else {
        return Err(MergeError::NotOneRoom);
    };
    room_from_shape(shape, first)
}

fn share_area(first: &Room, second: &Room) -> bool {
    let common =
        contours(first).overlay(&contours(second), OverlayRule::Intersect, FillRule::EvenOdd);
    !common.is_empty()
}

fn share_wall_length(first: &Room, second: &Room) -> bool {
    first.wall_loops().any(|(_, own)| {
        second
            .wall_loops()
            .any(|(_, theirs)| rings_share_length(own, theirs))
    })
}

fn rings_share_length(first: &Polygon, second: &Polygon) -> bool {
    first.segments().any(|own| {
        second
            .segments()
            .any(|theirs| geometry::segments_overlap_collinearly(own, theirs))
    })
}

fn contours(room: &Room) -> Contours {
    room.wall_loops()
        .map(|(_, ring)| {
            ring.vertices()
                .iter()
                .map(|vertex| IntPoint::new(vertex.x, vertex.y))
                .collect()
        })
        .collect()
}

/// Builds a room from an outer contour followed by its holes.
fn room_from_shape(shape: &Contours, template: &Room) -> Result<Room, MergeError> {
    let (outline, holes) = shape.split_first().ok_or(MergeError::NotOneRoom)?;
    let mut room = Room::new(ring(outline)?, template.wall().clone());
    for hole in holes {
        room.add_pillar(ring(hole)?)
            .map_err(|_| MergeError::InvalidWalls)?;
    }
    Ok(room)
}

fn ring(contour: &[IntPoint]) -> Result<Polygon, MergeError> {
    let vertices = contour
        .iter()
        .map(|point| CellPoint::new(point.x, point.y))
        .collect();
    Polygon::new(vertices).map_err(|_| MergeError::InvalidWalls)
}

#[cfg(test)]
mod tests {
    use super::{joins, merge};
    use crate::{AssetPath, CellPoint, Polygon, Room};

    fn polygon(coordinates: &[(i32, i32)]) -> Polygon {
        Polygon::new(
            coordinates
                .iter()
                .map(|&(x, y)| CellPoint::new(x, y))
                .collect(),
        )
        .unwrap()
    }

    fn rectangle(x: i32, y: i32, width: u32, height: u32) -> Polygon {
        Polygon::rectangle(CellPoint::new(x, y), width, height).unwrap()
    }

    fn room(outline: Polygon, wall: &str) -> Room {
        Room::new(outline, AssetPath::new(wall).unwrap())
    }

    fn square_room(x: i32, y: i32, side: u32) -> Room {
        room(rectangle(x, y, side, side), "wall.webp")
    }

    /// A U open at the bottom, around the empty area (2, 2)-(4, 6).
    fn u_shaped_room() -> Room {
        let outline = polygon(&[
            (0, 0),
            (6, 0),
            (6, 6),
            (4, 6),
            (4, 2),
            (2, 2),
            (2, 6),
            (0, 6),
        ]);
        room(outline, "u.webp")
    }

    #[test]
    fn rooms_sharing_area_or_a_wall_length_join() {
        let origin = square_room(0, 0, 4);
        assert!(joins(&origin, &square_room(2, 2, 4)));
        assert!(joins(&origin, &square_room(4, 0, 4)));
        assert!(joins(&origin, &room(rectangle(4, 3, 2, 3), "wall.webp")));
        assert!(joins(&origin, &square_room(1, 1, 1)));
    }

    #[test]
    fn rooms_apart_or_meeting_at_points_do_not_join() {
        let origin = square_room(0, 0, 4);
        assert!(!joins(&origin, &square_room(6, 0, 2)));
        assert!(!joins(&origin, &square_room(4, 4, 2)));
        let closing_tip = room(polygon(&[(2, 6), (4, 6), (3, 8)]), "wall.webp");
        assert!(!joins(&u_shaped_room(), &closing_tip));
    }

    #[test]
    fn overlapping_rooms_merge_into_one_outline() {
        let merged = merge(&[&square_room(0, 0, 4), &square_room(2, 0, 4)]).unwrap();
        let mut corners = merged.outline().vertices().to_vec();
        corners.sort_by_key(|corner| (corner.x, corner.y));
        assert_eq!(
            corners,
            [(0, 0), (0, 4), (6, 0), (6, 4)].map(|(x, y)| CellPoint::new(x, y))
        );
        assert_eq!(merged.pillars(), &[] as &[Polygon]);
    }

    #[test]
    fn keeps_the_wall_texture_of_the_first_room() {
        let merged = merge(&[&u_shaped_room(), &square_room(0, 5, 2)]).unwrap();
        assert_eq!(merged.wall().as_str(), "u.webp");
    }

    #[test]
    fn enclosed_empty_areas_become_pillars() {
        let bar = room(rectangle(0, 6, 6, 2), "wall.webp");
        let merged = merge(&[&u_shaped_room(), &bar]).unwrap();
        assert_eq!(merged.outline().double_area(), 2 * 6 * 8);
        assert_eq!(merged.pillars().len(), 1);
        assert_eq!(merged.pillars()[0].double_area(), 2 * 2 * 4);
    }

    #[test]
    fn covering_a_pillar_removes_it() {
        let mut hall = square_room(0, 0, 10);
        hall.add_pillar(rectangle(4, 4, 1, 1)).unwrap();
        let merged = merge(&[&hall, &square_room(3, 3, 3)]).unwrap();
        assert_eq!(merged.outline().double_area(), 2 * 10 * 10);
        assert_eq!(merged.pillars(), &[] as &[Polygon]);
    }

    #[test]
    fn rooms_apart_cannot_merge() {
        assert_eq!(
            merge(&[&square_room(0, 0, 2), &square_room(5, 5, 2)]),
            Err(super::MergeError::NotOneRoom)
        );
    }
}
