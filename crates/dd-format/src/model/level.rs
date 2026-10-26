use serde::{Deserialize, Serialize};

use super::{Cave, Color, Light, Object, Pattern, Shapes, Terrain, Tiles, Wall};
use crate::Preserved;

/// One floor of the map. Sections not modelled here are preserved as is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Level {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Preserved<Environment>>,
    pub shapes: Preserved<Shapes>,
    pub tiles: Preserved<Tiles>,
    pub patterns: Vec<Preserved<Pattern>>,
    pub walls: Vec<Preserved<Wall>>,
    pub cave: Preserved<Cave>,
    pub terrain: Preserved<Terrain>,
    pub objects: Vec<Preserved<Object>>,
    pub lights: Vec<Preserved<Light>>,
}

/// Lighting of a level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    pub baked_lighting: bool,
    /// Light everywhere, before lights add theirs.
    pub ambient_light: Color,
}
