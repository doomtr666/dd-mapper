use dd_domain::{CellPoint, Polygon};
use proptest::prelude::*;

fn grid_point() -> impl Strategy<Value = CellPoint> {
    (0..12, 0..12).prop_map(|(x, y)| CellPoint::new(x, y))
}

fn rings() -> impl Strategy<Value = Vec<CellPoint>> {
    prop::collection::vec(grid_point(), 0..9)
}

fn has_aligned_vertex(polygon: &Polygon) -> bool {
    let ring = polygon.vertices();
    (0..ring.len()).any(|index| {
        let previous = ring[(index + ring.len() - 1) % ring.len()];
        let next = ring[(index + 1) % ring.len()];
        let current = ring[index];
        let cross = (current.x - previous.x) * (next.y - previous.y)
            - (current.y - previous.y) * (next.x - previous.x);
        cross == 0
    })
}

proptest! {
    #[test]
    fn accepted_polygons_are_clockwise_and_minimal(ring in rings()) {
        if let Ok(polygon) = Polygon::new(ring.clone()) {
            prop_assert!(polygon.double_area() > 0);
            prop_assert!(!has_aligned_vertex(&polygon));
            prop_assert!(polygon.vertices().iter().all(|vertex| ring.contains(vertex)));
        }
    }

    #[test]
    fn construction_is_idempotent(ring in rings()) {
        if let Ok(polygon) = Polygon::new(ring) {
            prop_assert_eq!(Polygon::new(polygon.vertices().to_vec()), Ok(polygon));
        }
    }

    #[test]
    fn rectangles_cover_width_times_height(
        x in 0..50, y in 0..50, width in 1_u32..50, height in 1_u32..50
    ) {
        let rectangle = Polygon::rectangle(CellPoint::new(x, y), width, height).unwrap();
        prop_assert_eq!(rectangle.double_area(), 2 * i64::from(width) * i64::from(height));
        let bottom_right = CellPoint::new(x + width.cast_signed(), y + height.cast_signed());
        prop_assert_eq!(rectangle.vertices()[0], bottom_right);
    }
}
