//! Use cases for designing maps, shared by every adapter (MCP server, CLI).
//! Inputs are in cells; errors say what to do next.

mod assets;
mod color;
mod describe;
mod doors;
mod error;
mod generate;
mod ids;
mod ports;
mod requests;
mod validate;
mod workspace;

pub use assets::asset_line;
pub use error::AppError;
pub use generate::{DEFAULT_MIN_ROOM, Generator, LayoutReport, LayoutRequest};
pub use ports::{FootprintOf, MapFiles, OpenedMap, PortError, PreviewRenderer};
pub use requests::{
    DoorRequest, LightRequest, PatternRequest, PropRequest, RoomRequest, Shape, WallRequest,
};
pub use workspace::{OpenReport, RoomReport, Workspace};
