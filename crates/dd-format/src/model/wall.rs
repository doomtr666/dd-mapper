use serde::{Deserialize, Serialize};

use super::{Color, NodeId};
use crate::Preserved;
use crate::godot::{PoolVector2Array, Vector2};

/// A wall polyline in pixels; rooms are closed loops.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    pub points: PoolVector2Array,
    pub texture: String,
    pub color: Color,
    #[serde(rename = "loop")]
    pub closed_loop: bool,
    /// `0` for walls drawn with the room tool, `1` for free walls.
    #[serde(rename = "type")]
    pub wall_type: i32,
    pub joint: i32,
    pub normalize_uv: bool,
    pub shadow: bool,
    pub node_id: NodeId,
    pub portals: Vec<Preserved<Portal>>,
}

/// A door or window placed on a wall segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portal {
    pub position: Vector2,
    /// Radians, `atan2` of `direction`.
    pub rotation: f64,
    pub scale: Vector2,
    /// Unit vector of the wall segment the portal sits on.
    pub direction: Vector2,
    pub texture: String,
    pub radius: f64,
    /// Index of the segment's starting point in the wall.
    pub point_index: u32,
    pub wall_id: NodeId,
    /// `point_index + t`, with `t` in `[0, 1]` the fraction along the segment.
    pub wall_distance: f64,
    pub closed: bool,
    pub node_id: NodeId,
}
