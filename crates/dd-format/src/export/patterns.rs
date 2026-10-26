use dd_domain::Pattern as DomainPattern;

use super::pixels::CELL_SIZE_PX;
use crate::godot::{PoolVector2Array, Vector2};
use crate::model::{Color, NodeId, Pattern};

/// Layer Dungeondraft puts new patterns on ("User Layer 1").
const DEFAULT_PATTERN_LAYER: i32 = 100;

/// A Dungeondraft pattern for `pattern`.
pub fn build_pattern(pattern: &DomainPattern, node_id: NodeId) -> Pattern {
    Pattern {
        position: Vector2::default(),
        shape_rotation: 0.0,
        scale: Vector2::new(1.0, 1.0),
        points: PoolVector2Array(
            pattern
                .area
                .points()
                .iter()
                .map(|point| Vector2::new(point.x * CELL_SIZE_PX, point.y * CELL_SIZE_PX))
                .collect(),
        ),
        layer: DEFAULT_PATTERN_LAYER,
        color: Color(pattern.tint),
        outline: pattern.outlined,
        texture: pattern.asset.as_str().to_owned(),
        rotation: 0.0,
        node_id,
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPoint, CellPosition, FreeShape, Pattern, Polygon};

    use super::build_pattern;
    use crate::model::{MapFile, NodeId};

    fn as_written(pattern: &crate::model::Pattern) -> String {
        crate::json::to_godot_json(&serde_json::to_value(pattern).unwrap())
    }

    #[test]
    fn rebuilds_a_pattern_of_the_second_fixture() {
        let fixture =
            MapFile::parse(include_str!("../../../../fixtures/test2.dungeondraft_map")).unwrap();
        let expected = fixture.world().levels["0"]
            .patterns
            .iter()
            .find(|pattern| pattern.node_id == NodeId(0x37))
            .map(|pattern| (**pattern).clone())
            .unwrap();
        let corners = [(11, 16), (12, 16), (12, 17), (11, 17)]
            .into_iter()
            .map(|(x, y)| CellPoint::new(x, y))
            .collect();
        let pattern = Pattern {
            asset: AssetPath::new(
                "res://packs/LOREGD01/textures/patterns/normal/Artifcial/Tileset_AncientGothic_Quarters_Sewer_Open.webp",
            )
            .unwrap(),
            area: FreeShape::from(&Polygon::new(corners).unwrap()),
            tint: 0xffff_ffff,
            outlined: false,
        };
        assert_eq!(
            as_written(&build_pattern(&pattern, NodeId(0x37))),
            as_written(&expected)
        );
    }

    #[test]
    fn writes_free_outlines_in_pixels_and_the_outline_flag() {
        let pattern = Pattern {
            asset: AssetPath::new("res://textures/patterns/void.png").unwrap(),
            area: FreeShape::circle(CellPosition::new(2.0, 2.0), 0.5).unwrap(),
            tint: 0xffff_ffff,
            outlined: true,
        };
        let written = build_pattern(&pattern, NodeId(1));
        assert!(written.outline);
        let first = written.points.0[0];
        assert!((first.x - 640.0).abs() < 1e-9 && (first.y - 512.0).abs() < 1e-9);
    }
}
