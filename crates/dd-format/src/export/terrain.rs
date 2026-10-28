//! Terrain as Dungeondraft stores it: four texture slots, and for every
//! quarter-cell sample, row after row, one weight byte per slot.

use dd_domain::{TERRAIN_SLOTS, Terrain as Ground};

use crate::godot::PoolByteArray;
use crate::model::Terrain;

/// Writes the chosen textures and every sample's weights into `terrain`.
pub fn write_terrain(terrain: &mut Terrain, ground: &Ground) {
    let slots = [
        &mut terrain.texture_1,
        &mut terrain.texture_2,
        &mut terrain.texture_3,
        &mut terrain.texture_4,
    ];
    for (slot, texture) in slots.into_iter().enumerate() {
        if let Some(chosen) = ground.texture(slot) {
            chosen.as_str().clone_into(texture);
        }
    }
    let mut bytes = Vec::with_capacity((ground.columns() * ground.rows()) as usize * TERRAIN_SLOTS);
    for row in 0..ground.rows() {
        for column in 0..ground.columns() {
            bytes.extend(ground.weights(column, row));
        }
    }
    terrain.splat = PoolByteArray(bytes);
}

/// The weights of `splat`, sample after sample, or `None` when its size does
/// not fit `ground`.
#[must_use]
pub fn unpack_splat(splat: &PoolByteArray, ground: &Ground) -> Option<Vec<[u8; TERRAIN_SLOTS]>> {
    let samples = (ground.columns() * ground.rows()) as usize;
    if splat.0.len() != samples * TERRAIN_SLOTS {
        return None;
    }
    Some(splat.0.as_chunks::<TERRAIN_SLOTS>().0.to_vec())
}

/// The texture of each slot, empty names left out.
#[must_use]
pub fn slot_textures(terrain: &Terrain) -> [&str; TERRAIN_SLOTS] {
    [
        &terrain.texture_1,
        &terrain.texture_2,
        &terrain.texture_3,
        &terrain.texture_4,
    ]
}
