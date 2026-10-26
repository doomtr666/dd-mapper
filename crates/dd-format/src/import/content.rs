//! Floor tiles, props, lights and patterns.

use std::collections::BTreeSet;

use dd_domain::{AssetPath, Cell, Footprint, Glow, Light, Pattern, Polygon, Prop, QuarterTurn};

use super::cells::{grid_points, position};
use super::{FootprintOf, Importer};
use crate::model::{Level, Light as LightRecord, Object, Pattern as PatternRecord};

/// Rotations further than this from a quarter turn are reported, in degrees.
const QUARTER_TURN_TOLERANCE: f64 = 0.5;
const QUARTER_TURN: f64 = 90.0;
/// Footprint of objects whose size is unknown, in cells.
const UNKNOWN_SIDE: f64 = 1.0;

pub fn import_floor(importer: &mut Importer, level: &Level) {
    let width = importer.map.grid().width();
    let mut missing = BTreeSet::new();
    for (index, tile) in (0_u32..).zip(&level.tiles.cells.0) {
        let Ok(key) = u32::try_from(*tile) else {
            continue;
        };
        let tileset = level
            .tiles
            .lookup
            .get(&key)
            .and_then(|path| AssetPath::new(path.clone()).ok());
        match tileset {
            Some(tileset) => {
                let cell = Cell::new(index % width, index / width);
                importer.map.paint_floor(cell, &tileset);
            }
            None => {
                missing.insert(key);
            }
        }
    }
    for key in missing {
        importer.warn(format!("floor tiles {key} have no tileset in the lookup"));
    }
}

pub fn import_props(importer: &mut Importer, level: &Level, footprint_of: FootprintOf) {
    for object in &level.objects {
        if let Err(error) = import_prop(importer, object, footprint_of) {
            importer.warn(format!("prop {} is skipped: {error}", object.node_id.0));
        }
    }
}

fn import_prop(
    importer: &mut Importer,
    object: &Object,
    footprint_of: FootprintOf,
) -> Result<(), String> {
    let rotation = quarter_turn(importer, object)?;
    if object.mirror {
        importer.warn(format!(
            "prop {} is mirrored; mirroring is not kept",
            object.node_id.0
        ));
    }
    let (width, height) = footprint_of(&object.texture).unwrap_or((UNKNOWN_SIDE, UNKNOWN_SIDE));
    let footprint = Footprint::new(width * object.scale.x.abs(), height * object.scale.y.abs())
        .map_err(|error| error.to_string())?;
    let prop = Prop {
        asset: AssetPath::new(object.texture.clone()).map_err(|error| error.to_string())?,
        center: position(object.position),
        rotation,
        footprint,
    };
    importer
        .map
        .add_prop(prop)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// The quarter turn nearest to the object's rotation.
fn quarter_turn(importer: &mut Importer, object: &Object) -> Result<QuarterTurn, String> {
    let degrees = object.rotation.to_degrees();
    let quarters = (degrees / QUARTER_TURN).round();
    if (degrees - quarters * QUARTER_TURN).abs() > QUARTER_TURN_TOLERANCE {
        importer.warn(format!(
            "prop {} is turned by {degrees:.0} degrees; it was rounded to a quarter turn",
            object.node_id.0
        ));
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a few quarter turns at most"
    )]
    let quarter_degrees = (quarters * QUARTER_TURN) as i32;
    QuarterTurn::from_degrees(quarter_degrees).map_err(|error| error.to_string())
}

pub fn import_lights(importer: &mut Importer, level: &Level) {
    for light in &level.lights {
        if let Err(error) = import_light(importer, light) {
            importer.warn(format!("light {} is skipped: {error}", light.node_id.0));
        }
    }
}

fn import_light(importer: &mut Importer, light: &LightRecord) -> Result<(), String> {
    let glow = Glow::new(light.range, light.intensity, light.color.0)
        .map_err(|error| error.to_string())?;
    let asset = AssetPath::new(light.texture.clone()).map_err(|error| error.to_string())?;
    importer
        .map
        .add_light(Light {
            asset,
            center: position(light.position),
            glow,
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn import_patterns(importer: &mut Importer, level: &Level) {
    for pattern in &level.patterns {
        if let Err(error) = import_pattern(importer, pattern) {
            importer.warn(format!("pattern {} is skipped: {error}", pattern.node_id.0));
        }
    }
}

fn import_pattern(importer: &mut Importer, pattern: &PatternRecord) -> Result<(), String> {
    let (corners, on_grid) = grid_points(&pattern.points);
    let transformed = pattern.rotation != 0.0
        || pattern.shape_rotation != 0.0
        || pattern.position.x != 0.0
        || pattern.position.y != 0.0;
    if !on_grid || transformed {
        importer.warn(format!(
            "pattern {} is moved, turned or off the grid; only its outline on the grid is kept",
            pattern.node_id.0
        ));
    }
    let area = Polygon::new(corners).map_err(|error| error.to_string())?;
    let asset = AssetPath::new(pattern.texture.clone()).map_err(|error| error.to_string())?;
    importer
        .map
        .add_pattern(Pattern {
            asset,
            area,
            tint: pattern.color.0,
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}
