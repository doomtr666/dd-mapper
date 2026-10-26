use dd_domain::CellPoint;

use crate::godot::{PoolVector2Array, Vector2};

/// Dungeondraft draws one grid cell as 256 pixels.
pub const CELL_SIZE_PX: f64 = 256.0;

pub fn point_to_pixels(point: CellPoint) -> Vector2 {
    Vector2::new(
        f64::from(point.x) * CELL_SIZE_PX,
        f64::from(point.y) * CELL_SIZE_PX,
    )
}

pub fn ring_to_pixels<'a>(ring: impl Iterator<Item = &'a CellPoint>) -> PoolVector2Array {
    PoolVector2Array(ring.copied().map(point_to_pixels).collect())
}

#[cfg(test)]
mod tests {
    use dd_domain::CellPoint;

    use super::{point_to_pixels, ring_to_pixels};
    use crate::godot::Vector2;

    #[test]
    fn one_cell_is_256_pixels() {
        assert_eq!(
            point_to_pixels(CellPoint::new(13, 5)),
            Vector2::new(3328.0, 1280.0)
        );
    }

    #[test]
    fn converts_rings_in_order() {
        let ring = [CellPoint::new(30, 16), CellPoint::new(13, 16)];
        assert_eq!(
            ring_to_pixels(ring.iter()).0,
            [Vector2::new(7680.0, 4096.0), Vector2::new(3328.0, 4096.0)]
        );
    }
}
