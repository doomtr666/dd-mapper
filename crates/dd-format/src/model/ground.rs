use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Color, NodeId, decimal_node_ids};
use crate::godot::{PoolByteArray, PoolIntArray, PoolVector2Array};

/// Floor outlines of the rooms drawn with the room tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shapes {
    pub polygons: Vec<PoolVector2Array>,
    #[serde(with = "decimal_node_ids")]
    pub walls: Vec<NodeId>,
}

/// Floor tiles, one entry per cell in row-major order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tiles {
    /// Key into `lookup`, or `-1` for an empty cell.
    pub cells: PoolIntArray,
    pub colors: Vec<Color>,
    pub lookup: BTreeMap<u32, String>,
}

/// Cave bitmaps: 4 sub-cells per cell plus a border.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cave {
    pub bitmap: PoolByteArray,
    pub ground_color: Color,
    pub wall_color: Color,
    pub entrance_bitmap: PoolByteArray,
    pub texture: String,
}

/// Terrain blending: `splat` holds one RGBA weight per sub-cell (4 per cell side).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Terrain {
    pub enabled: bool,
    pub expand_slots: bool,
    pub smooth_blending: bool,
    pub texture_1: String,
    pub texture_2: String,
    pub texture_3: String,
    pub texture_4: String,
    pub splat: PoolByteArray,
}
