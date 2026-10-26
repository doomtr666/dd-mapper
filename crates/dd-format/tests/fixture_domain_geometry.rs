//! Acceptance test: the domain's polygon normalization matches Dungeondraft.

mod common;

use dd_domain::{CellPoint, Polygon};
use dd_format::godot::Vector2;
use dd_format::model::MapFile;

const ROOM_WALL_TYPE: i32 = 0;
const CELL_SIZE_PX: f64 = 256.0;

fn to_cell_point(pixel: Vector2) -> CellPoint {
    let to_cells = |coordinate: f64| {
        let cells = coordinate / CELL_SIZE_PX;
        assert!(cells.fract() == 0.0, "{coordinate} px is not on the grid");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "checked to be a whole number"
        )]
        let whole = cells as i32;
        whole
    };
    CellPoint::new(to_cells(pixel.x), to_cells(pixel.y))
}

#[test]
fn room_walls_are_already_normalized_domain_polygons() {
    let mut checked = 0;
    for fixture in common::FIXTURES {
        let map = MapFile::parse(&common::read_fixture(fixture)).expect("valid fixture");
        let room_walls = map.world().levels["0"]
            .walls
            .iter()
            .filter(|wall| wall.wall_type == ROOM_WALL_TYPE);
        for wall in room_walls {
            let ring: Vec<CellPoint> = wall.points.0.iter().copied().map(to_cell_point).collect();
            let polygon = Polygon::new(ring.clone())
                .unwrap_or_else(|error| panic!("{fixture}: wall {:?}: {error}", wall.node_id));
            assert_eq!(
                polygon.vertices(),
                ring,
                "{fixture}: wall {:?}",
                wall.node_id
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 1 + 16 + 1);
}
