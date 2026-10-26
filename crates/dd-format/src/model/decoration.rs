use serde::{Deserialize, Serialize};

use super::{Color, NodeId};
use crate::godot::{PoolVector2Array, Vector2};

/// A placed asset (prop, furniture, decal, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub position: Vector2,
    pub rotation: f64,
    pub scale: Vector2,
    pub mirror: bool,
    pub texture: String,
    pub layer: i32,
    pub shadow: bool,
    pub block_light: bool,
    /// Only written for colored objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_color: Option<Color>,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub position: Vector2,
    pub rotation: f64,
    /// Radius in cells.
    pub range: f64,
    pub intensity: f64,
    pub color: Color,
    pub texture: String,
    pub shadows: bool,
    pub node_id: NodeId,
}

/// A textured polygon painted on a layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub position: Vector2,
    pub shape_rotation: f64,
    pub scale: Vector2,
    pub points: PoolVector2Array,
    pub layer: i32,
    pub color: Color,
    pub outline: bool,
    pub texture: String,
    pub rotation: f64,
    pub node_id: NodeId,
}

/// A texture drawn along a line, such as a conveyor belt or a cable. Points
/// are relative to `position`, and Dungeondraft repeats the last one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors the path record Dungeondraft writes, flag for flag"
)]
pub struct Path {
    pub position: Vector2,
    pub rotation: f64,
    pub scale: Vector2,
    pub edit_points: PoolVector2Array,
    pub smoothness: f64,
    pub texture: String,
    /// In pixels.
    pub width: f64,
    pub layer: i32,
    pub fade_in: bool,
    pub fade_out: bool,
    pub grow: bool,
    pub shrink: bool,
    pub block_light: bool,
    #[serde(rename = "loop")]
    pub closed_loop: bool,
    pub node_id: NodeId,
}
