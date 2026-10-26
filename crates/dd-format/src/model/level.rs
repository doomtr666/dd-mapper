use serde::{Deserialize, Serialize};

use super::{Cave, Light, Object, Pattern, Shapes, Terrain, Tiles, Wall};
use crate::Preserved;

/// One floor of the map. Sections not modelled here are preserved as is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Level {
    pub shapes: Preserved<Shapes>,
    pub tiles: Preserved<Tiles>,
    pub patterns: Vec<Preserved<Pattern>>,
    pub walls: Vec<Preserved<Wall>>,
    pub cave: Preserved<Cave>,
    pub terrain: Preserved<Terrain>,
    pub objects: Vec<Preserved<Object>>,
    pub lights: Vec<Preserved<Light>>,
}
