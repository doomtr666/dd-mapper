use std::collections::HashSet;

use dd_domain::{AssetPath, CellPoint, Polygon, Room, joins, merge};
use proptest::prelude::*;

fn rectangle_room() -> impl Strategy<Value = (Room, HashSet<(i32, i32)>)> {
    (0..8, 0..8, 1_u32..6, 1_u32..6).prop_map(|(x, y, width, height)| {
        let outline = Polygon::rectangle(CellPoint::new(x, y), width, height).unwrap();
        let room = Room::new(outline, AssetPath::new("wall.webp").unwrap());
        let cells = (x..x + width.cast_signed())
            .flat_map(|column| (y..y + height.cast_signed()).map(move |row| (column, row)))
            .collect();
        (room, cells)
    })
}

fn wall_area(room: &Room) -> i64 {
    let pillars: i64 = room.pillars().iter().map(Polygon::double_area).sum();
    (room.outline().double_area() - pillars) / 2
}

proptest! {
    #[test]
    fn merged_rooms_cover_exactly_the_union_of_their_cells(
        (first, first_cells) in rectangle_room(),
        (second, second_cells) in rectangle_room(),
    ) {
        prop_assume!(joins(&first, &second));
        let merged = merge(&[&first, &second]).expect("joined rectangles merge");
        let union_cells = first_cells.union(&second_cells).count();
        prop_assert_eq!(wall_area(&merged), i64::try_from(union_cells).unwrap());
    }

    #[test]
    fn joining_is_symmetric(
        (first, _) in rectangle_room(),
        (second, _) in rectangle_room(),
    ) {
        prop_assert_eq!(joins(&first, &second), joins(&second, &first));
    }
}
