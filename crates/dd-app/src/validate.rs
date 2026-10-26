//! Design checks: what is legal but probably a mistake.

use dd_domain::{CellPosition, Map, Prop, Room, Segment, WallRef};

use crate::ids::{prop_label, room_label};

/// Warnings about `map`, empty when nothing looks wrong.
pub fn validate(map: &Map) -> Vec<String> {
    if map.rooms().next().is_none() && map.cave().floor_samples() == 0 {
        return vec!["the map has no room nor cave".to_owned()];
    }
    let mut warnings = rooms_without_doors(map);
    warnings.extend(misplaced_props(map));
    warnings.extend(overlapping_props(map));
    warnings
}

fn rooms_without_doors(map: &Map) -> Vec<String> {
    map.rooms()
        .filter(|(id, _)| {
            !map.doors().any(
                |(_, door)| matches!(door.position.wall, WallRef::Room { room, .. } if room == *id),
            )
        })
        .map(|(id, _)| format!("room {} has no door", room_label(id)))
        .collect()
}

fn misplaced_props(map: &Map) -> Vec<String> {
    map.props()
        .filter_map(|(id, prop)| {
            if map.rooms().any(|(_, room)| crosses_walls(room, prop)) {
                return Some(format!(
                    "prop {} crosses a wall or a pillar",
                    prop_label(id)
                ));
            }
            let in_room = map
                .rooms()
                .any(|(_, room)| room.outline().contains_position(prop.center));
            let on_open_ground = map.background().is_some();
            (!in_room && !on_open_ground && !map.cave().is_floor_at(prop.center))
                .then(|| format!("prop {} is outside every room and cave", prop_label(id)))
        })
        .collect()
}

pub(crate) fn crosses_walls(room: &Room, prop: &Prop) -> bool {
    let (top_left, bottom_right) = prop.bounds();
    let in_pillar = room
        .pillars()
        .iter()
        .any(|pillar| pillar.contains_position(prop.center));
    in_pillar
        || room.wall_loops().any(|(_, ring)| {
            ring.segments()
                .any(|segment| crosses_box(segment, top_left, bottom_right))
        })
}

/// Whether `segment` runs through the open inside of a box. The segment is
/// clipped to the closed box (Liang-Barsky); a wall merely running along a side
/// leaves the middle of the clipped part on that side.
pub(crate) fn crosses_box(
    segment: Segment,
    top_left: CellPosition,
    bottom_right: CellPosition,
) -> bool {
    let start = segment.point_at(0.0);
    let end = segment.point_at(1.0);
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let limits = [
        (-dx, start.x - top_left.x),
        (dx, bottom_right.x - start.x),
        (-dy, start.y - top_left.y),
        (dy, bottom_right.y - start.y),
    ];
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    for (direction, room_left) in limits {
        if direction.abs() < f64::EPSILON {
            if room_left < 0.0 {
                return false;
            }
            continue;
        }
        let crossing = room_left / direction;
        if direction < 0.0 {
            enter = enter.max(crossing);
        } else {
            leave = leave.min(crossing);
        }
    }
    if enter >= leave {
        return false;
    }
    let middle = segment.point_at(f64::midpoint(enter, leave));
    top_left.x < middle.x
        && middle.x < bottom_right.x
        && top_left.y < middle.y
        && middle.y < bottom_right.y
}

fn overlapping_props(map: &Map) -> Vec<String> {
    let props: Vec<_> = map.props().collect();
    let mut warnings = Vec::new();
    for (index, (first_id, first)) in props.iter().enumerate() {
        for (second_id, second) in &props[index + 1..] {
            if boxes_overlap(first.bounds(), second.bounds()) {
                warnings.push(format!(
                    "props {} and {} overlap",
                    prop_label(*first_id),
                    prop_label(*second_id)
                ));
            }
        }
    }
    warnings
}

pub(crate) fn boxes_overlap(
    (first_min, first_max): (CellPosition, CellPosition),
    (second_min, second_max): (CellPosition, CellPosition),
) -> bool {
    first_min.x < second_max.x
        && second_min.x < first_max.x
        && first_min.y < second_max.y
        && second_min.y < first_max.y
}

#[cfg(test)]
mod tests {
    use dd_domain::{
        AssetPath, CellPoint, CellPosition, Door, Footprint, GridSize, Map, Polygon, Prop, Room,
        RoomId, Rotation, WallLoop, WallPosition, WallRef,
    };

    use super::validate;

    fn asset(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    fn map_with_room() -> (Map, RoomId) {
        let mut map = Map::new(GridSize::new(20, 20).unwrap());
        let mut room = Room::new(
            Polygon::rectangle(CellPoint::new(0, 0), 6, 4).unwrap(),
            asset("wall.webp"),
        );
        room.add_pillar(Polygon::rectangle(CellPoint::new(3, 1), 1, 1).unwrap())
            .unwrap();
        let id = map.add_room(room, &asset("floor.webp")).unwrap().id;
        (map, id)
    }

    fn add_door(map: &mut Map, room: RoomId) {
        let position = WallPosition {
            wall: WallRef::Room {
                room,
                wall: WallLoop::Outline,
            },
            segment: 0,
            t: 0.5,
        };
        map.add_door(Door {
            position,
            asset: asset("door.webp"),
        })
        .unwrap();
    }

    fn add_prop(map: &mut Map, x: f64, y: f64) {
        map.add_prop(Prop {
            asset: asset("crate.webp"),
            center: CellPosition::new(x, y),
            rotation: Rotation::NONE,
            footprint: Footprint::new(1.0, 1.0).unwrap(),
            tint: None,
        })
        .unwrap();
    }

    #[test]
    fn a_furnished_room_with_a_door_is_fine() {
        let (mut map, room) = map_with_room();
        add_door(&mut map, room);
        add_prop(&mut map, 1.5, 1.5);
        assert_eq!(validate(&map), Vec::<String>::new());
    }

    #[test]
    fn warns_about_an_empty_map() {
        let map = Map::new(GridSize::new(5, 5).unwrap());
        assert_eq!(validate(&map), ["the map has no room nor cave"]);
    }

    #[test]
    fn warns_about_rooms_without_doors() {
        let (map, _) = map_with_room();
        assert_eq!(validate(&map), ["room R0 has no door"]);
    }

    #[test]
    fn warns_about_props_outside_rooms_or_across_walls() {
        let (mut map, room) = map_with_room();
        add_door(&mut map, room);
        add_prop(&mut map, 10.5, 10.5);
        add_prop(&mut map, 5.8, 2.0);
        add_prop(&mut map, 3.5, 2.2);
        assert_eq!(
            validate(&map),
            [
                "prop P0 is outside every room and cave",
                "prop P1 crosses a wall or a pillar",
                "prop P2 crosses a wall or a pillar",
            ]
        );
    }

    #[test]
    fn warns_about_cave_props_across_room_walls() {
        let (mut map, room) = map_with_room();
        add_door(&mut map, room);
        map.cave_mut().dig(|position| position.x < 12.0, true);
        add_prop(&mut map, 9.0, 2.0);
        add_prop(&mut map, 6.3, 2.0);
        assert_eq!(validate(&map), ["prop P1 crosses a wall or a pillar"]);
    }

    #[test]
    fn props_on_open_ground_are_fine_once_the_ground_is_chosen() {
        let (mut map, room) = map_with_room();
        add_door(&mut map, room);
        add_prop(&mut map, 10.5, 10.5);
        assert_eq!(validate(&map), ["prop P0 is outside every room and cave"]);
        map.set_background(asset("rockrete.png"));
        assert_eq!(validate(&map), Vec::<String>::new());
    }

    #[test]
    fn warns_about_overlapping_props() {
        let (mut map, room) = map_with_room();
        add_door(&mut map, room);
        add_prop(&mut map, 1.5, 1.5);
        add_prop(&mut map, 2.0, 1.5);
        assert_eq!(validate(&map), ["props P0 and P1 overlap"]);
    }
}
