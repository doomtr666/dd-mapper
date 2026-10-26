use dd_domain::Room;

use super::pixels::ring_to_pixels;
use crate::godot::PoolVector2Array;
use crate::model::{NodeId, Shapes};

/// Floor outlines of the rooms: each outline as walled, each pillar
/// reversed as a hole, as Dungeondraft writes them.
pub fn build_shapes<'a>(rooms: impl Iterator<Item = &'a Room>, wall_ids: Vec<NodeId>) -> Shapes {
    Shapes {
        polygons: rooms.flat_map(floor_polygons).collect(),
        walls: wall_ids,
    }
}

fn floor_polygons(room: &Room) -> impl Iterator<Item = PoolVector2Array> + '_ {
    let outline = ring_to_pixels(room.outline().vertices().iter());
    let holes = room
        .pillars()
        .iter()
        .map(|pillar| ring_to_pixels(pillar.vertices().iter().rev()));
    std::iter::once(outline).chain(holes)
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPoint, Polygon, Room};

    use super::build_shapes;
    use crate::godot::{GodotLiteral, PoolVector2Array};
    use crate::model::NodeId;

    fn literal(text: &str) -> PoolVector2Array {
        PoolVector2Array::parse_literal(text).unwrap()
    }

    #[test]
    fn outlines_keep_the_wall_order_and_pillars_are_reversed() {
        let mut room = Room::new(
            Polygon::rectangle(CellPoint::new(0, 0), 4, 4).unwrap(),
            AssetPath::new("wall.webp").unwrap(),
        );
        room.add_pillar(Polygon::rectangle(CellPoint::new(1, 1), 1, 1).unwrap())
            .unwrap();
        let shapes = build_shapes([&room].into_iter(), vec![NodeId(0), NodeId(1)]);
        assert_eq!(
            shapes.polygons,
            [
                literal("PoolVector2Array( 1024, 1024, 0, 1024, 0, 0, 1024, 0 )"),
                literal("PoolVector2Array( 512, 256, 256, 256, 256, 512, 512, 512 )"),
            ]
        );
        assert_eq!(shapes.walls, [NodeId(0), NodeId(1)]);
    }
}
