//! Room loops become rooms (inner loops their pillars); free walls become partitions.

use dd_domain::{AssetPath, Partition, Polygon, Room};

use super::Importer;
use super::cells::grid_points;
use crate::model::{Level, Wall};

const ROOM_WALL_TYPE: i32 = 0;
const FREE_WALL_TYPE: i32 = 1;

/// A closed room wall, as a polygon on the grid.
struct Loop {
    ring: Polygon,
    texture: AssetPath,
}

pub fn import_walls(importer: &mut Importer, level: &Level) {
    let mut loops = Vec::new();
    for wall in &level.walls {
        match (wall.wall_type, wall.closed_loop) {
            (ROOM_WALL_TYPE, true) => loops.extend(room_loop(importer, wall)),
            (FREE_WALL_TYPE, _) => import_partition(importer, wall),
            _ => importer.warn(format!(
                "wall {} (type {}) is not imported",
                wall.node_id.0, wall.wall_type
            )),
        }
    }
    import_rooms(importer, &loops);
}

fn room_loop(importer: &mut Importer, wall: &Wall) -> Option<Loop> {
    let (corners, on_grid) = grid_points(&wall.points);
    if !on_grid {
        warn_off_grid(importer, wall);
    }
    let ring = Polygon::new(corners)
        .map_err(|error| importer.warn(format!("wall {} is skipped: {error}", wall.node_id.0)))
        .ok()?;
    let texture = asset(importer, wall)?;
    Some(Loop { ring, texture })
}

fn import_partition(importer: &mut Importer, wall: &Wall) {
    let (mut points, on_grid) = grid_points(&wall.points);
    if !on_grid {
        warn_off_grid(importer, wall);
    }
    if wall.closed_loop
        && let Some(first) = points.first().copied()
    {
        points.push(first);
    }
    let Some(texture) = asset(importer, wall) else {
        return;
    };
    let added = Partition::new(points, texture)
        .map_err(|error| error.to_string())
        .and_then(|partition| {
            importer
                .map
                .add_partition(partition)
                .map_err(|error| error.to_string())
        });
    if let Err(error) = added {
        importer.warn(format!("wall {} is skipped: {error}", wall.node_id.0));
    }
}

/// Loops nested an even number of times are rooms; the others are pillars
/// of the innermost loop around them.
fn import_rooms(importer: &mut Importer, loops: &[Loop]) {
    let containers: Vec<Vec<usize>> = (0..loops.len())
        .map(|inner| {
            (0..loops.len())
                .filter(|&outer| outer != inner)
                .filter(|&outer| {
                    loops[outer]
                        .ring
                        .strictly_contains_polygon(&loops[inner].ring)
                })
                .collect()
        })
        .collect();
    let mut rooms: Vec<Option<Room>> = loops
        .iter()
        .zip(&containers)
        .map(|(candidate, around)| {
            around
                .len()
                .is_multiple_of(2)
                .then(|| Room::new(candidate.ring.clone(), candidate.texture.clone()))
        })
        .collect();
    for (index, around) in containers.iter().enumerate() {
        if around.len().is_multiple_of(2) {
            continue;
        }
        let parent = around
            .iter()
            .copied()
            .max_by_key(|outer| containers[*outer].len());
        let pillar = loops[index].ring.clone();
        let added = parent
            .and_then(|parent| rooms[parent].as_mut())
            .map(|room| room.add_pillar(pillar));
        if let Some(Err(error)) = added {
            importer.warn(format!("a pillar is skipped: {error}"));
        }
    }
    for room in rooms.into_iter().flatten() {
        if let Err(error) = importer.map.add_walls(room) {
            importer.warn(format!("a room is skipped: {error}"));
        }
    }
}

fn asset(importer: &mut Importer, wall: &Wall) -> Option<AssetPath> {
    AssetPath::new(wall.texture.clone())
        .map_err(|error| importer.warn(format!("wall {} is skipped: {error}", wall.node_id.0)))
        .ok()
}

fn warn_off_grid(importer: &mut Importer, wall: &Wall) {
    importer.warn(format!(
        "wall {} is off the grid; its corners were moved to the nearest grid vertices",
        wall.node_id.0
    ));
}
