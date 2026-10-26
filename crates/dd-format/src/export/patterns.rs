use dd_domain::Pattern as DomainPattern;

use super::pixels::ring_to_pixels;
use crate::godot::Vector2;
use crate::model::{Color, NodeId, Pattern};

/// Layer Dungeondraft puts new patterns on ("User Layer 1").
const DEFAULT_PATTERN_LAYER: i32 = 100;

/// A Dungeondraft pattern for `pattern`.
pub fn build_pattern(pattern: &DomainPattern, node_id: NodeId) -> Pattern {
    Pattern {
        position: Vector2::default(),
        shape_rotation: 0.0,
        scale: Vector2::new(1.0, 1.0),
        points: ring_to_pixels(pattern.area.vertices().iter()),
        layer: DEFAULT_PATTERN_LAYER,
        color: Color(pattern.tint),
        outline: false,
        texture: pattern.asset.as_str().to_owned(),
        rotation: 0.0,
        node_id,
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPoint, Pattern, Polygon};

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
            area: Polygon::new(corners).unwrap(),
            tint: 0xffff_ffff,
        };
        assert_eq!(
            as_written(&build_pattern(&pattern, NodeId(0x37))),
            as_written(&expected)
        );
    }
}
