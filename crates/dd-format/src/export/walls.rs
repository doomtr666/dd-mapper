use dd_domain::{Partition, Polygon};

use super::pixels::ring_to_pixels;
use crate::model::{Color, NodeId, Wall};

/// `type` of walls drawn with the room tool.
const ROOM_WALL_TYPE: i32 = 0;
/// `type` of walls drawn with the wall tool.
const FREE_WALL_TYPE: i32 = 1;
/// `joint` Dungeondraft writes for every wall of the fixtures.
const DEFAULT_JOINT: i32 = 1;

/// A closed room wall following `ring`, without portals yet.
pub fn build_wall(ring: &Polygon, texture: &str, node_id: NodeId) -> Wall {
    Wall {
        points: ring_to_pixels(ring.vertices().iter()),
        texture: texture.to_owned(),
        color: Color::WHITE,
        closed_loop: true,
        wall_type: ROOM_WALL_TYPE,
        joint: DEFAULT_JOINT,
        normalize_uv: true,
        shadow: true,
        node_id,
        portals: Vec::new(),
    }
}

/// An open free-standing wall following `partition`, without portals yet.
pub fn build_free_wall(partition: &Partition, node_id: NodeId) -> Wall {
    Wall {
        points: ring_to_pixels(partition.points().iter()),
        texture: partition.wall().as_str().to_owned(),
        color: Color::WHITE,
        closed_loop: false,
        wall_type: FREE_WALL_TYPE,
        joint: DEFAULT_JOINT,
        normalize_uv: true,
        shadow: true,
        node_id,
        portals: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPoint, Partition, Polygon};

    use super::{build_free_wall, build_wall};
    use crate::godot::{GodotLiteral, PoolVector2Array};
    use crate::model::{Color, NodeId};

    #[test]
    fn builds_a_closed_room_wall_in_pixels() {
        let ring = Polygon::rectangle(CellPoint::new(13, 5), 17, 11).unwrap();
        let wall = build_wall(&ring, "wall.webp", NodeId(7));
        let expected_points = "PoolVector2Array( 7680, 4096, 3328, 4096, 3328, 1280, 7680, 1280 )";
        assert_eq!(
            wall.points,
            PoolVector2Array::parse_literal(expected_points).unwrap()
        );
        assert_eq!(wall.texture, "wall.webp");
        assert_eq!(wall.color, Color::WHITE);
        assert!(wall.closed_loop && wall.normalize_uv && wall.shadow);
        assert_eq!((wall.wall_type, wall.joint), (0, 1));
        assert_eq!(wall.node_id, NodeId(7));
        assert!(wall.portals.is_empty());
    }

    #[test]
    fn builds_an_open_free_wall() {
        let points = vec![
            CellPoint::new(6, 0),
            CellPoint::new(6, 8),
            CellPoint::new(9, 8),
        ];
        let partition = Partition::new(points, AssetPath::new("wall.webp").unwrap()).unwrap();
        let wall = build_free_wall(&partition, NodeId(4));
        assert_eq!(
            wall.points,
            PoolVector2Array::parse_literal("PoolVector2Array( 1536, 0, 1536, 2048, 2304, 2048 )")
                .unwrap()
        );
        assert!(!wall.closed_loop);
        assert_eq!((wall.wall_type, wall.joint), (1, 1));
        assert_eq!(
            (wall.texture.as_str(), wall.node_id),
            ("wall.webp", NodeId(4))
        );
    }
}
