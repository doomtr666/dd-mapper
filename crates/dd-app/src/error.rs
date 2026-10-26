use dd_catalog::AssetKind;
use dd_domain::{
    GridError, LightError, MapError, PartitionError, PolygonError, PropError, RoomError,
};
use thiserror::Error;

use crate::PortError;

/// What went wrong, worded so that the caller knows how to fix the request.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AppError {
    #[error("there is no map `{0}`; create one with map_new")]
    UnknownMap(String),
    #[error("`{id}` is not a {kind} id; expected something like `{example}`")]
    MalformedId {
        kind: &'static str,
        id: String,
        example: String,
    },
    #[error("{}", unknown_asset(path, *kind, suggestions))]
    UnknownAsset {
        path: String,
        kind: AssetKind,
        suggestions: Vec<String>,
    },
    #[error("`{path}` is a {actual} asset, but a {expected} asset is needed here")]
    WrongAssetKind {
        path: String,
        expected: AssetKind,
        actual: AssetKind,
    },
    #[error(
        "the nearest wall of {owner} is {distance:.1} cells away from ({x}, {y}); \
         give a point on one of its walls"
    )]
    NoWallNearby {
        owner: String,
        x: f64,
        y: f64,
        distance: f64,
    },
    #[error("`{0}` is not a color; write RRGGBB or AARRGGBB in hexadecimal, such as ffeccd8b")]
    InvalidColor(String),
    #[error("this generator needs a map of at least {width} x {height} cells")]
    MapTooSmall { width: u32, height: u32 },
    #[error("give at least one object asset to scatter")]
    NothingToScatter,
    #[error(transparent)]
    Grid(#[from] GridError),
    #[error(transparent)]
    Light(#[from] LightError),
    #[error(transparent)]
    Polygon(#[from] PolygonError),
    #[error(transparent)]
    Room(#[from] RoomError),
    #[error(transparent)]
    Partition(#[from] PartitionError),
    #[error(transparent)]
    Map(#[from] MapError),
    #[error(transparent)]
    Prop(#[from] PropError),
    #[error(transparent)]
    Port(#[from] PortError),
}

fn unknown_asset(path: &str, kind: AssetKind, suggestions: &[String]) -> String {
    let hint = if suggestions.is_empty() {
        String::new()
    } else {
        format!(" Did you mean: {}?", suggestions.join(", "))
    };
    format!("no {kind} asset `{path}` in the catalog.{hint} Use catalog_search to find assets.")
}
