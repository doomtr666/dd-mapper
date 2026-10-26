use dd_domain::{CellPosition, Track};

use super::pixels::CELL_SIZE_PX;
use crate::godot::{PoolVector2Array, Vector2};
use crate::model::{NodeId, Path};

/// Layer Dungeondraft puts new paths on ("User Layer 1").
const DEFAULT_PATH_LAYER: i32 = 100;
/// Dungeondraft's default curve through the points.
const DEFAULT_SMOOTHNESS: f64 = 1.0;

/// A Dungeondraft path for `track`: points relative to the first one, the
/// last one repeated as Dungeondraft writes it.
pub fn build_path(track: &Track, node_id: NodeId) -> Path {
    let points = track.points();
    let origin = points[0];
    let relative = |point: &CellPosition| {
        Vector2::new(
            (point.x - origin.x) * CELL_SIZE_PX,
            (point.y - origin.y) * CELL_SIZE_PX,
        )
    };
    let mut edit_points: Vec<Vector2> = points.iter().map(relative).collect();
    edit_points.extend(points.last().map(relative));
    Path {
        position: Vector2::new(origin.x * CELL_SIZE_PX, origin.y * CELL_SIZE_PX),
        rotation: 0.0,
        scale: Vector2::new(1.0, 1.0),
        edit_points: PoolVector2Array(edit_points),
        smoothness: DEFAULT_SMOOTHNESS,
        texture: track.asset().as_str().to_owned(),
        width: track.width() * CELL_SIZE_PX,
        layer: DEFAULT_PATH_LAYER,
        fade_in: false,
        fade_out: false,
        grow: false,
        shrink: false,
        block_light: false,
        closed_loop: track.closed(),
        node_id,
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPosition, Track};

    use super::build_path;
    use crate::model::{MapFile, NodeId};

    fn as_written(path: &crate::model::Path) -> String {
        crate::json::to_godot_json(&serde_json::to_value(path).unwrap())
    }

    fn sample_path(node_id: NodeId) -> String {
        let fixture = MapFile::parse(include_str!(
            "../../../../fixtures/cave_and_paths.dungeondraft_map"
        ))
        .unwrap();
        let path = fixture.world().levels["0"]
            .paths
            .iter()
            .find(|path| path.node_id == node_id)
            .map(|path| (**path).clone())
            .unwrap();
        as_written(&path)
    }

    #[test]
    fn rebuilds_the_rope_of_the_sample() {
        let rope = Track::new(
            AssetPath::new("res://textures/paths/rope_(thick).png").unwrap(),
            vec![CellPosition::new(32.0, 21.0), CellPosition::new(34.0, 19.0)],
            48.0 / 256.0,
            false,
        )
        .unwrap();
        assert_eq!(
            as_written(&build_path(&rope, NodeId(0xe7))),
            sample_path(NodeId(0xe7))
        );
    }
}
