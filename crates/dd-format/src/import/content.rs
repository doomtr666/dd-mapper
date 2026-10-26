//! Floor tiles, props, lights, patterns, paths, caves and terrain.

use std::collections::BTreeSet;

use dd_domain::{
    AssetPath, Cell, CellPosition, Footprint, Glow, Light, Pattern, Polygon, Prop, Rotation, Track,
};

use super::cells::{grid_points, position};
use super::{FootprintOf, Importer};
use crate::export::{CELL_SIZE_PX, slot_textures, unpack_cave, unpack_splat};
use crate::godot::Vector2;
use crate::model::{
    Level, Light as LightRecord, Object, Path as PathRecord, Pattern as PatternRecord,
};

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
    let rotation =
        Rotation::from_degrees(object.rotation.to_degrees()).map_err(|error| error.to_string())?;
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
        tint: object.custom_color.map(|color| color.0),
    };
    importer
        .map
        .add_prop(prop)
        .map(|_| ())
        .map_err(|error| error.to_string())
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

pub fn import_tracks(importer: &mut Importer, level: &Level) {
    for path in &level.paths {
        if let Err(error) = import_track(importer, path) {
            importer.warn(format!("path {} is skipped: {error}", path.node_id.0));
        }
    }
}

fn import_track(importer: &mut Importer, path: &PathRecord) -> Result<(), String> {
    let transformed = path.rotation != 0.0 || path.scale != Vector2::new(1.0, 1.0);
    if transformed {
        importer.warn(format!(
            "path {} is turned or scaled; its points are kept as drawn",
            path.node_id.0
        ));
    }
    let mut points: Vec<CellPosition> = path
        .edit_points
        .0
        .iter()
        .map(|point| {
            CellPosition::new(
                (path.position.x + point.x) / CELL_SIZE_PX,
                (path.position.y + point.y) / CELL_SIZE_PX,
            )
        })
        .collect();
    points.dedup();
    let asset = AssetPath::new(path.texture.clone()).map_err(|error| error.to_string())?;
    let track = Track::new(asset, points, path.width / CELL_SIZE_PX, path.closed_loop)
        .map_err(|error| error.to_string())?;
    let track = if path.smoothness > 0.5 {
        track
    } else {
        track.with_sharp_corners()
    };
    importer
        .map
        .add_track(track)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn import_cave(importer: &mut Importer, level: &Level) {
    let grid = importer.map.cave().clone();
    let (Some(floor), Some(entrance)) = (
        unpack_cave(&level.cave.bitmap, &grid),
        unpack_cave(&level.cave.entrance_bitmap, &grid),
    ) else {
        importer.warn("the cave does not fit the map size and is skipped".to_owned());
        return;
    };
    let columns = grid.columns();
    let cave = importer.map.cave_mut();
    for (index, (floor, entrance)) in floor.into_iter().zip(entrance).enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        cave.set(index % columns, index / columns, floor, entrance);
    }
}

pub fn import_terrain(importer: &mut Importer, level: &Level) {
    let Some(weights) = unpack_splat(&level.terrain.splat, importer.map.terrain()) else {
        importer.warn("the terrain does not fit the map size and is skipped".to_owned());
        return;
    };
    let ground = importer.map.terrain_mut();
    for (slot, texture) in slot_textures(&level.terrain).into_iter().enumerate() {
        if let Ok(texture) = AssetPath::new(texture.to_owned()) {
            ground.set_texture(slot, texture);
        }
    }
    let columns = ground.columns();
    for (index, weights) in weights.into_iter().enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        ground.set_weights(index % columns, index / columns, weights);
    }
}
