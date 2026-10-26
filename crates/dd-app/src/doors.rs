//! Placing a door on the wall point nearest to a requested position.

use dd_domain::{
    CellPosition, Partition, PartitionId, Room, RoomId, Segment, WallPosition, WallRef,
};

/// Requests farther than this from every wall of the room are refused, in cells.
pub const MAX_SNAP_DISTANCE: f64 = 1.5;

/// A wall position, and how far it is from the requested point, in cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapped {
    pub position: WallPosition,
    pub distance: f64,
}

/// A wall that can hold doors, with its segments.
pub type DoorWall = (WallRef, Vec<Segment>);

/// The walls of room `id`: its outline and pillars.
pub fn room_walls(id: RoomId, room: &Room) -> Vec<DoorWall> {
    room.wall_loops()
        .map(|(wall, ring)| (WallRef::Room { room: id, wall }, ring.segments().collect()))
        .collect()
}

/// The walls of free-standing wall `id`: just itself.
pub fn partition_walls(id: PartitionId, partition: &Partition) -> Vec<DoorWall> {
    vec![(WallRef::Partition(id), partition.segments().collect())]
}

/// The position on `walls` nearest to `point` where a door of
/// `half_width` cells fits without crossing a corner.
pub fn nearest_wall_position(
    walls: &[DoorWall],
    point: CellPosition,
    half_width: f64,
) -> Option<Snapped> {
    walls
        .iter()
        .flat_map(|(wall, segments)| {
            let wall = *wall;
            segments.iter().enumerate().map(move |(segment, line)| {
                let line = *line;
                let (t, distance) = closest_point(line, point, half_width);
                Snapped {
                    position: WallPosition { wall, segment, t },
                    distance,
                }
            })
        })
        .min_by(|first, second| first.distance.total_cmp(&second.distance))
}

/// Smallest share of a segment kept clear at each end.
const MIN_END_MARGIN: f64 = 0.01;

/// Fraction along `line` closest to `point`, kept `half_width` away from the
/// segment ends, and the distance from `point` to it.
fn closest_point(line: Segment, point: CellPosition, half_width: f64) -> (f64, f64) {
    let start = line.point_at(0.0);
    let end = line.point_at(1.0);
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length = dx.hypot(dy);
    let margin = (half_width / length).clamp(MIN_END_MARGIN, 0.5);
    let projection = ((point.x - start.x) * dx + (point.y - start.y) * dy) / (length * length);
    let t = projection.clamp(margin, 1.0 - margin);
    let snapped = line.point_at(t);
    (t, (snapped.x - point.x).hypot(snapped.y - point.y))
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPoint, CellPosition, Polygon, Room, RoomId, WallLoop, WallRef};

    use super::{DoorWall, nearest_wall_position, room_walls};

    fn walls() -> Vec<DoorWall> {
        room_walls(RoomId(0), &room_with_pillar())
    }

    fn room_with_pillar() -> Room {
        let mut room = Room::new(
            Polygon::rectangle(CellPoint::new(0, 0), 6, 4).unwrap(),
            AssetPath::new("wall.webp").unwrap(),
        );
        room.add_pillar(Polygon::rectangle(CellPoint::new(3, 1), 1, 1).unwrap())
            .unwrap();
        room
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    #[test]
    fn projects_onto_the_nearest_wall() {
        let snapped = nearest_wall_position(&walls(), CellPosition::new(2.2, 4.3), 0.5).unwrap();
        assert_eq!(
            (snapped.position.wall, snapped.position.segment),
            (
                WallRef::Room {
                    room: RoomId(0),
                    wall: WallLoop::Outline
                },
                0
            )
        );
        assert_close(snapped.position.t, (6.0 - 2.2) / 6.0);
        assert_close(snapped.distance, 0.3);
    }

    #[test]
    fn keeps_the_door_clear_of_corners() {
        let snapped = nearest_wall_position(&walls(), CellPosition::new(0.0, 0.0), 0.5).unwrap();
        assert_close(snapped.distance, 0.5);
        let t = snapped.position.t;
        assert!(t > 0.0 && t < 1.0, "{t}");
    }

    #[test]
    fn can_snap_to_a_pillar() {
        let snapped = nearest_wall_position(&walls(), CellPosition::new(3.5, 2.1), 0.3).unwrap();
        assert_eq!(
            snapped.position.wall,
            WallRef::Room {
                room: RoomId(0),
                wall: WallLoop::Pillar(0)
            }
        );
        assert_close(snapped.distance, 0.1);
    }
}
