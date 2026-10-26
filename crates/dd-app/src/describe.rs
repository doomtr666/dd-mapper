//! A plain-text summary of a map, matching the preview labels.

use dd_domain::{
    AssetPath, Door, DoorId, Light, LightId, Map, Partition, PartitionId, Pattern, PatternId,
    Polygon, Prop, PropId, Room, RoomId, Track, TrackId,
};

use crate::ids::{
    door_label, light_label, partition_label, pattern_label, prop_label, room_label, track_label,
    wall_owner,
};

/// Lists the rooms, doors and props of `map`, named as on the preview.
pub fn describe(map_label: &str, map: &Map) -> String {
    let grid = map.grid();
    let mut text = format!("{map_label}: {} x {} cells\n", grid.width(), grid.height());
    if let Some(background) = map.background() {
        text.push_str(&background_line(background));
    }
    if let Some(argb) = map.ambient_light() {
        text.push_str(&ambient_line(argb));
    }
    if map.cave().floor_samples() > 0 {
        text.push_str(&cave_line(map));
    }
    if map.rooms().next().is_none() {
        text.push_str("No room yet.\n");
    } else {
        text.push_str("Rooms:\n");
        for (id, room) in map.rooms() {
            text.push_str(&room_line(id, room));
        }
    }
    if map.partitions().next().is_some() {
        text.push_str("Walls:\n");
        for (id, partition) in map.partitions() {
            text.push_str(&partition_line(id, partition));
        }
    }
    if map.doors().next().is_some() {
        text.push_str("Doors:\n");
        for (id, door) in map.doors() {
            text.push_str(&door_line(map, id, door));
        }
    }
    if map.patterns().next().is_some() {
        text.push_str("Patterns:\n");
        for (id, pattern) in map.patterns() {
            text.push_str(&pattern_line(id, pattern));
        }
    }
    if map.tracks().next().is_some() {
        text.push_str("Paths:\n");
        for (id, track) in map.tracks() {
            text.push_str(&track_line(id, track));
        }
    }
    if map.lights().next().is_some() {
        text.push_str("Lights:\n");
        for (id, light) in map.lights() {
            text.push_str(&light_line(id, light));
        }
    }
    if map.props().next().is_some() {
        text.push_str("Props:\n");
        for (id, prop) in map.props() {
            text.push_str(&prop_line(id, prop));
        }
    }
    text
}

/// Sixteen quarter-cell samples make a cell.
const SAMPLES_PER_CELL_AREA: f64 = 16.0;

fn cave_line(map: &Map) -> String {
    let samples = u32::try_from(map.cave().floor_samples()).unwrap_or(u32::MAX);
    let cells = (f64::from(samples) / SAMPLES_PER_CELL_AREA).round();
    let entrances = if map.cave().has_entrance() {
        ", with entrances"
    } else {
        ""
    };
    format!("Cave: about {cells} cells of floor{entrances}\n")
}

fn ambient_line(argb: u32) -> String {
    format!("Ambient light: {argb:08x}\n")
}

fn background_line(background: &AssetPath) -> String {
    format!(
        "Background: {}
",
        asset_name(background)
    )
}

fn room_line(id: RoomId, room: &Room) -> String {
    format!(
        "- {}: {}, {} cells of floor, {}, walls {}
",
        room_label(id),
        bounds_text(room.outline()),
        floor_area(room),
        pillar_count(room.pillars().len()),
        asset_name(room.wall()),
    )
}

/// `from (left, top) to (right, bottom)`.
fn bounds_text(polygon: &Polygon) -> String {
    let corners = polygon.vertices();
    let xs = corners.iter().map(|corner| corner.x);
    let ys = corners.iter().map(|corner| corner.y);
    let (left, right) = (xs.clone().min(), xs.max());
    let (top, bottom) = (ys.clone().min(), ys.max());
    format!(
        "from ({}, {}) to ({}, {})",
        left.unwrap_or_default(),
        top.unwrap_or_default(),
        right.unwrap_or_default(),
        bottom.unwrap_or_default()
    )
}

/// Area inside the outline and outside the pillars, in cells.
fn floor_area(room: &Room) -> f64 {
    let pillars: i64 = room
        .pillars()
        .iter()
        .map(dd_domain::Polygon::double_area)
        .sum();
    let doubled = room.outline().double_area() - pillars;
    #[expect(clippy::cast_precision_loss, reason = "map areas stay far below 2^52")]
    let area = doubled as f64 / 2.0;
    area
}

fn partition_line(id: PartitionId, partition: &Partition) -> String {
    let points: Vec<String> = partition
        .points()
        .iter()
        .map(|point| format!("({}, {})", point.x, point.y))
        .collect();
    format!(
        "- {}: {}, {}\n",
        partition_label(id),
        points.join(" - "),
        asset_name(partition.wall())
    )
}

fn pillar_count(count: usize) -> String {
    match count {
        0 => "no pillar".to_owned(),
        1 => "1 pillar".to_owned(),
        _ => format!("{count} pillars"),
    }
}

fn door_line(map: &Map, id: DoorId, door: &Door) -> String {
    let at = map
        .door_position(id)
        .map(|position| format!(" at ({}, {})", position.x, position.y))
        .unwrap_or_default();
    format!(
        "- {}: on {}{at}, {}\n",
        door_label(id),
        wall_owner(door.position.wall),
        asset_name(&door.asset)
    )
}

fn pattern_line(id: PatternId, pattern: &Pattern) -> String {
    format!(
        "- {}: {} {}, tint {:08x}
",
        pattern_label(id),
        asset_name(&pattern.asset),
        bounds_text(&pattern.area),
        pattern.tint,
    )
}

fn track_line(id: TrackId, track: &Track) -> String {
    let points: Vec<String> = track
        .points()
        .iter()
        .map(|point| format!("({}, {})", hundredths(point.x), hundredths(point.y)))
        .collect();
    let closed = if track.closed() { ", closed" } else { "" };
    format!(
        "- {}: {} through {}, {} cells wide{closed}\n",
        track_label(id),
        asset_name(track.asset()),
        points.join(" - "),
        hundredths(track.width()),
    )
}

fn light_line(id: LightId, light: &Light) -> String {
    format!(
        "- {}: {} at ({}, {}), range {} cells, intensity {}, color {:08x}\n",
        light_label(id),
        asset_name(&light.asset),
        light.center.x,
        light.center.y,
        hundredths(light.glow.range()),
        hundredths(light.glow.intensity()),
        light.glow.color(),
    )
}

fn prop_line(id: PropId, prop: &Prop) -> String {
    let tint = prop
        .tint
        .map_or_else(String::new, |argb| format!(", tint {argb:08x}"));
    format!(
        "- {}: {} at ({}, {}), {} degrees, {} x {} cells{tint}\n",
        prop_label(id),
        asset_name(&prop.asset),
        prop.center.x,
        prop.center.y,
        prop.rotation.degrees(),
        hundredths(prop.footprint.width()),
        hundredths(prop.footprint.height()),
    )
}

/// Two decimals are plenty to place things on a grid of cells.
fn hundredths(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// The file name of an asset, without folders nor extension.
fn asset_name(asset: &AssetPath) -> &str {
    let file = asset.as_str().rsplit('/').next().unwrap_or_default();
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use dd_domain::{
        AssetPath, CellPoint, CellPosition, Door, Footprint, GridSize, Map, Polygon, Prop,
        QuarterTurn, Room, WallLoop, WallPosition, WallRef,
    };

    use super::describe;

    fn asset(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    #[test]
    fn summarizes_rooms_doors_and_props() {
        let mut map = Map::new(GridSize::new(30, 20).unwrap());
        let mut room = Room::new(
            Polygon::rectangle(CellPoint::new(2, 2), 14, 10).unwrap(),
            asset("res://packs/X/textures/walls/Wall_Steel.webp"),
        );
        room.add_pillar(Polygon::rectangle(CellPoint::new(5, 5), 1, 1).unwrap())
            .unwrap();
        let room = map
            .add_room(
                room,
                &asset("res://packs/X/textures/tilesets/Floor_Grid.webp"),
            )
            .unwrap()
            .id;
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
            asset: asset("res://packs/X/textures/portals/Door_Steel.webp"),
        })
        .unwrap();
        map.add_prop(Prop {
            asset: asset("res://packs/X/textures/objects/Crate.webp"),
            center: CellPosition::new(12.5, 3.5),
            rotation: QuarterTurn::Deg90,
            footprint: Footprint::new(2.0, 1.5).unwrap(),
            tint: None,
        })
        .unwrap();

        assert_eq!(
            describe("map-1", &map),
            "map-1: 30 x 20 cells\n\
             Rooms:\n\
             - R0: from (2, 2) to (16, 12), 139 cells of floor, 1 pillar, walls Wall_Steel\n\
             Doors:\n\
             - D0: on R0 at (9, 12), Door_Steel\n\
             Props:\n\
             - P0: Crate at (12.5, 3.5), 90 degrees, 2 x 1.5 cells\n"
        );
    }

    #[test]
    fn rounds_prop_sizes_to_hundredths() {
        let mut map = Map::new(GridSize::new(10, 10).unwrap());
        map.add_prop(Prop {
            asset: asset("res://packs/X/textures/objects/Console.webp"),
            center: CellPosition::new(8.0, 3.5),
            rotation: QuarterTurn::Deg0,
            footprint: Footprint::new(0.656_25, 0.8125).unwrap(),
            tint: None,
        })
        .unwrap();
        assert!(
            describe("map-1", &map)
                .contains("P0: Console at (8, 3.5), 0 degrees, 0.66 x 0.81 cells")
        );
    }

    #[test]
    fn says_when_the_map_is_empty() {
        let map = Map::new(GridSize::new(5, 5).unwrap());
        assert_eq!(
            describe("map-2", &map),
            "map-2: 5 x 5 cells\nNo room yet.\n"
        );
    }
}
