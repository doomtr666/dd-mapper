//! Pure map model, expressed in grid cells. No IO, no serialization,
//! no knowledge of any map file format.

mod asset;
mod cave;
mod geometry;
mod grid;
mod light;
mod map;
mod partition;
mod pattern;
mod polygon;
mod prop;
mod room;
mod terrain;
mod track;
mod union;

pub use asset::{AssetPath, AssetPathError};
pub use cave::CaveGrid;
pub use grid::{Cell, CellPoint, CellPosition, GridError, GridSize};
pub use light::{Glow, Light, LightError};
pub use map::{
    AddedRoom, Door, DoorId, LightId, Map, MapError, PartitionId, PatternId, PropId, RoomId,
    TrackId, WallPosition, WallRef,
};
pub use partition::{Partition, PartitionError};
pub use pattern::{FreeShape, FreeShapeError, Pattern};
pub use polygon::{Polygon, PolygonError, Segment};
pub use prop::{Footprint, Prop, PropError, Rotation};
pub use room::{Room, RoomError, WallLoop};
pub use terrain::{TERRAIN_SLOTS, Terrain};
pub use track::{Track, TrackError};
pub use union::{MergeError, joins, merge};
