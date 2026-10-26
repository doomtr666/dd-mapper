use dd_domain::Segment;

use super::pixels::point_to_pixels;
use crate::godot::Vector2;
use crate::model::{NodeId, Portal};

/// Everything needed to place a portal on a wall segment.
pub struct PortalSpec<'a> {
    pub segment: Segment,
    pub segment_index: usize,
    /// Fraction of the segment length, from its start.
    pub t: f64,
    pub wall_id: NodeId,
    pub node_id: NodeId,
    pub texture: &'a str,
    pub radius: f64,
}

pub fn build_portal(spec: &PortalSpec) -> Portal {
    let start = point_to_pixels(spec.segment.start);
    let end = point_to_pixels(spec.segment.end);
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length = dx.hypot(dy);
    let point_index =
        u32::try_from(spec.segment_index).expect("wall loops have far fewer than 2^32 segments");
    Portal {
        position: Vector2::new(start.x + dx * spec.t, start.y + dy * spec.t),
        rotation: dy.atan2(dx),
        scale: Vector2::new(1.0, 1.0),
        direction: Vector2::new(dx / length, dy / length),
        texture: spec.texture.to_owned(),
        radius: spec.radius,
        point_index,
        wall_id: spec.wall_id,
        wall_distance: f64::from(point_index) + spec.t,
        closed: true,
        node_id: spec.node_id,
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use dd_domain::{CellPoint, Segment};

    use super::{PortalSpec, build_portal};
    use crate::godot::Vector2;
    use crate::model::NodeId;

    fn spec(segment: Segment, segment_index: usize, t: f64) -> PortalSpec<'static> {
        PortalSpec {
            segment,
            segment_index,
            t,
            wall_id: NodeId(0),
            node_id: NodeId(3),
            texture: "door.webp",
            radius: 154.0,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-3, "{actual} != {expected}");
    }

    #[test]
    fn places_the_door_of_the_reference_room() {
        let segment = Segment {
            start: CellPoint::new(13, 16),
            end: CellPoint::new(13, 5),
        };
        let portal = build_portal(&spec(segment, 1, 0.772_727));
        assert_close(portal.position.x, 3328.0);
        assert_close(portal.position.y, 1920.0);
        assert_close(portal.rotation, -FRAC_PI_2);
        assert_eq!(portal.direction, Vector2::new(0.0, -1.0));
        assert_eq!(portal.point_index, 1);
        assert_close(portal.wall_distance, 1.772_727);
        assert_eq!((portal.wall_id, portal.node_id), (NodeId(0), NodeId(3)));
        assert_eq!(portal.scale, Vector2::new(1.0, 1.0));
        assert!(portal.closed);
    }

    #[test]
    fn faces_along_a_leftward_segment() {
        let segment = Segment {
            start: CellPoint::new(40, 17),
            end: CellPoint::new(31, 17),
        };
        let portal = build_portal(&spec(segment, 3, 0.5));
        assert_eq!(portal.position, Vector2::new(9088.0, 4352.0));
        assert_close(portal.rotation, PI);
        assert_eq!(portal.direction, Vector2::new(-1.0, 0.0));
        assert_close(portal.wall_distance, 3.5);
    }
}
