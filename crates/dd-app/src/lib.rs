//! Use cases for designing maps, shared by every adapter (MCP server, CLI).
//! Inputs are in cells; errors say what to do next.

mod assets;
mod caves;
mod color;
mod describe;
mod doors;
mod error;
mod generate;
mod ids;
mod ports;
mod requests;
mod scatter;
mod validate;
mod workspace;

pub use assets::asset_line;
pub use caves::{CaveNetwork, CaveRequest, CaveShape};
pub use error::AppError;
pub use generate::{DEFAULT_MARGIN, DEFAULT_MIN_ROOM, Generator, LayoutReport, LayoutRequest};
pub use ports::{FootprintOf, MapFiles, OpenedMap, PortError, PreviewRenderer, SaveFacts};
pub use requests::{
    DoorRequest, FloorRequest, LightRequest, PathRequest, PatternRequest, PropRequest, RoomRequest,
    Shape, WallRequest,
};
pub use scatter::{ScatterReport, ScatterRequest};
pub use workspace::{OpenReport, RoomReport, Workspace};
