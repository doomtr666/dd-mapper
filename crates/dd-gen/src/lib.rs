//! Seeded generators producing building layouts on the grid: an outline,
//! partitions splitting it into areas, and where doors go. No assets.

mod bsp;
mod cave;
mod hab;
mod layout;
mod random;

pub use bsp::{BspParams, bsp};
pub use cave::{CaveLayout, CaveParams, cave};
pub use hab::{HabParams, hab_block, ship_deck};
pub use layout::{Area, DoorSpot, Layout};
pub use random::Random;
