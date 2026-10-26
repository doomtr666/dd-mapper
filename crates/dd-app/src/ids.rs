//! The ids shown to users: `map-1`, `R0`, `D0`, `P0`, as on the preview labels.

use dd_domain::{DoorId, LightId, PartitionId, PatternId, PropId, RoomId, WallRef};

use crate::AppError;

pub fn room_label(id: RoomId) -> String {
    format!("R{}", id.0)
}

pub fn door_label(id: DoorId) -> String {
    format!("D{}", id.0)
}

pub fn prop_label(id: PropId) -> String {
    format!("P{}", id.0)
}

pub fn light_label(id: LightId) -> String {
    format!("L{}", id.0)
}

pub fn pattern_label(id: PatternId) -> String {
    format!("S{}", id.0)
}

pub fn partition_label(id: PartitionId) -> String {
    format!("W{}", id.0)
}

/// What carries walls that doors can go in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallOwner {
    Room(RoomId),
    Partition(PartitionId),
}

/// The id of what carries a wall: `R0` for a room, `W0` for a free-standing wall.
pub fn wall_owner(wall: WallRef) -> String {
    match wall {
        WallRef::Room { room, .. } => room_label(room),
        WallRef::Partition(id) => partition_label(id),
    }
}

/// # Errors
/// Fails unless `text` reads `R<number>` or `W<number>`.
pub fn parse_wall_owner(text: &str) -> Result<WallOwner, AppError> {
    let trimmed = text.trim();
    let number = |prefix: &str| trimmed.strip_prefix(prefix)?.parse().ok();
    if let Some(room) = number("R") {
        return Ok(WallOwner::Room(RoomId(room)));
    }
    if let Some(wall) = number("W") {
        return Ok(WallOwner::Partition(PartitionId(wall)));
    }
    Err(AppError::MalformedId {
        kind: "room or wall",
        id: text.to_owned(),
        example: "R0` or `W0".to_owned(),
    })
}

pub fn map_label(number: u32) -> String {
    format!("map-{number}")
}

/// # Errors
/// Fails unless `text` reads `map-<number>`.
pub fn parse_map(text: &str) -> Result<u32, AppError> {
    parse_prefixed(text, "map-", "map")
}

/// # Errors
/// Fails unless `text` reads `P<number>`.
pub fn parse_prop(text: &str) -> Result<PropId, AppError> {
    parse_prefixed(text, "P", "prop").map(PropId)
}

fn parse_prefixed(text: &str, prefix: &str, kind: &'static str) -> Result<u32, AppError> {
    text.trim()
        .strip_prefix(prefix)
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| AppError::MalformedId {
            kind,
            id: text.to_owned(),
            example: format!("{prefix}0"),
        })
}

#[cfg(test)]
mod tests {
    use dd_domain::RoomId;

    use dd_domain::PartitionId;

    use super::{WallOwner, parse_map, parse_wall_owner, room_label};
    use crate::AppError;

    #[test]
    fn room_ids_round_trip() {
        assert_eq!(
            parse_wall_owner(&room_label(RoomId(12))),
            Ok(WallOwner::Room(RoomId(12)))
        );
        assert_eq!(parse_map(" map-3 "), Ok(3));
    }

    #[test]
    fn doors_go_in_rooms_or_free_walls() {
        assert_eq!(parse_wall_owner("R2"), Ok(WallOwner::Room(RoomId(2))));
        assert_eq!(
            parse_wall_owner("W5"),
            Ok(WallOwner::Partition(PartitionId(5)))
        );
        assert!(matches!(
            parse_wall_owner("D1"),
            Err(AppError::MalformedId {
                kind: "room or wall",
                ..
            })
        ));
    }

    #[test]
    fn malformed_ids_show_an_example() {
        assert_eq!(
            parse_map("my map"),
            Err(AppError::MalformedId {
                kind: "map",
                id: "my map".to_owned(),
                example: "map-0".to_owned()
            })
        );
    }
}
