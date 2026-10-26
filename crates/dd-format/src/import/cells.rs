//! From Dungeondraft pixels to domain cells.

use dd_domain::{CellPoint, CellPosition};

use crate::export::CELL_SIZE_PX;
use crate::godot::{PoolVector2Array, Vector2};

/// How far from a grid vertex a point may be and still count as on it, in cells.
const GRID_TOLERANCE: f64 = 1e-3;

pub fn position(pixel: Vector2) -> CellPosition {
    CellPosition::new(pixel.x / CELL_SIZE_PX, pixel.y / CELL_SIZE_PX)
}

/// The grid vertices nearest to `points`, and whether all of them were on the grid.
pub fn grid_points(points: &PoolVector2Array) -> (Vec<CellPoint>, bool) {
    let mut on_grid = true;
    let vertices = points
        .0
        .iter()
        .map(|pixel| {
            let (vertex, exact) = grid_point(*pixel);
            on_grid &= exact;
            vertex
        })
        .collect();
    (vertices, on_grid)
}

fn grid_point(pixel: Vector2) -> (CellPoint, bool) {
    let cells = position(pixel);
    let (x, y) = (cells.x.round(), cells.y.round());
    let exact = (cells.x - x).abs() < GRID_TOLERANCE && (cells.y - y).abs() < GRID_TOLERANCE;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "map coordinates fit in i32"
    )]
    let vertex = CellPoint::new(x as i32, y as i32);
    (vertex, exact)
}

#[cfg(test)]
mod tests {
    use dd_domain::CellPoint;

    use super::grid_points;
    use crate::godot::{GodotLiteral, PoolVector2Array};

    #[test]
    fn rounds_points_to_the_grid_and_says_when_it_had_to() {
        let exact =
            PoolVector2Array::parse_literal("PoolVector2Array( 3328, 1280, 0, 256 )").unwrap();
        assert_eq!(
            grid_points(&exact),
            (vec![CellPoint::new(13, 5), CellPoint::new(0, 1)], true)
        );
        let off = PoolVector2Array::parse_literal("PoolVector2Array( 3300, 1280 )").unwrap();
        assert_eq!(grid_points(&off), (vec![CellPoint::new(13, 5)], false));
    }
}
