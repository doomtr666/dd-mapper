//! Typed model of the `.dungeondraft_map` file, as Dungeondraft writes it.
//!
//! Field order matters: records created from scratch are written in
//! declaration order, which mirrors Dungeondraft's.

mod color;
mod decoration;
mod ground;
mod header;
mod hex;
mod level;
mod map_file;
mod node_id;
mod wall;

pub use color::Color;
pub use decoration::{Light, Object, Path, Pattern};
pub use ground::{Cave, Shapes, Terrain, Tiles};
pub use header::{Header, PackEntry};
pub use level::{Environment, Level};
pub use map_file::{MapFile, World};
pub use node_id::{NodeId, decimal as decimal_node_ids};
pub use wall::{Portal, Wall};
