use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Level, NodeId};
use crate::json::to_godot_json;
use crate::{FormatError, Preserved};

/// A whole `.dungeondraft_map` file; `header` and `mod` are preserved as is.
#[derive(Debug, Clone, PartialEq)]
pub struct MapFile {
    root: Preserved<MapRoot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MapRoot {
    world: Preserved<World>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    /// In cells.
    pub width: u32,
    /// In cells.
    pub height: u32,
    pub next_node_id: NodeId,
    pub levels: IndexMap<String, Preserved<Level>>,
}

impl MapFile {
    /// Decodes the text of a `.dungeondraft_map` file.
    ///
    /// # Errors
    /// Fails when the text is not JSON or a modelled field is invalid.
    pub fn parse(text: &str) -> Result<Self, FormatError> {
        Ok(Self {
            root: serde_json::from_str(text)?,
        })
    }

    /// Encodes the map exactly as Dungeondraft writes it.
    ///
    /// # Errors
    /// Fails when a modelled value cannot be represented in JSON.
    pub fn to_text(&self) -> Result<String, FormatError> {
        let document = serde_json::to_value(&self.root)?;
        Ok(to_godot_json(&document) + "\n")
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.root.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.root.world
    }
}
