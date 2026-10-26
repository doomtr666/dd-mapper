//! Wires the format, render and catalog crates into a [`dd_app::Workspace`],
//! from the options every binary shares.

pub mod adapters;
mod config;

pub use config::{Config, SOURCE_OPTIONS};
