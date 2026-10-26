//! Doors go back on whichever imported wall passes through them.

use dd_domain::{AssetPath, CellPosition, Door, Map, Segment, WallPosition, WallRef};

use super::Importer;
use super::cells::position;
use crate::model::Level;

/// Dungeondraft stores positions with 6 significant digits.
const ON_WALL_TOLERANCE: f64 = 0.01;

pub fn import_doors(importer: &mut Importer, level: &Level) {
    for portal in level.walls.iter().flat_map(|wall| wall.portals.iter()) {
        let point = position(portal.position);
        let Some(on_wall) = locate(&importer.map, point) else {
            importer.warn(format!(
                "door {} at ({:.2}, {:.2}) is not on an imported wall",
                portal.node_id.0, point.x, point.y
            ));
            continue;
        };
        let added = AssetPath::new(portal.texture.clone())
            .map_err(|error| error.to_string())
            .and_then(|asset| {
                importer
                    .map
                    .add_door(Door {
                        position: on_wall,
                        asset,
                    })
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = added {
            importer.warn(format!("door {} is skipped: {error}", portal.node_id.0));
        }
    }
}

fn locate(map: &Map, point: CellPosition) -> Option<WallPosition> {
    let room_walls = map.rooms().flat_map(|(room, walls)| {
        walls.wall_loops().map(move |(wall, ring)| {
            (
                WallRef::Room { room, wall },
                ring.segments().collect::<Vec<_>>(),
            )
        })
    });
    let partitions = map
        .partitions()
        .map(|(id, partition)| (WallRef::Partition(id), partition.segments().collect()));
    room_walls
        .chain(partitions)
        .find_map(|(wall, segments)| locate_on(wall, &segments, point))
}

fn locate_on(wall: WallRef, segments: &[Segment], point: CellPosition) -> Option<WallPosition> {
    segments.iter().enumerate().find_map(|(segment, line)| {
        line.locate_within(point, ON_WALL_TOLERANCE)
            .map(|t| WallPosition { wall, segment, t })
    })
}
