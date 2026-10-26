use dd_domain::Light as DomainLight;

use super::pixels::CELL_SIZE_PX;
use crate::godot::Vector2;
use crate::model::{Color, Light, NodeId};

/// A Dungeondraft light for `light`.
pub fn build_light(light: &DomainLight, node_id: NodeId) -> Light {
    Light {
        position: Vector2::new(light.center.x * CELL_SIZE_PX, light.center.y * CELL_SIZE_PX),
        rotation: 0.0,
        range: light.glow.range(),
        intensity: light.glow.intensity(),
        color: Color(light.glow.color()),
        texture: light.asset.as_str().to_owned(),
        shadows: true,
        node_id,
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPosition, Glow, Light};

    use super::build_light;
    use crate::model::{MapFile, NodeId};

    fn as_written(light: &crate::model::Light) -> String {
        crate::json::to_godot_json(&serde_json::to_value(light).unwrap())
    }

    #[test]
    fn rebuilds_a_light_of_the_second_fixture() {
        let fixture =
            MapFile::parse(include_str!("../../../../fixtures/test2.dungeondraft_map")).unwrap();
        let expected = fixture.world().levels["0"]
            .lights
            .iter()
            .find(|light| light.node_id == NodeId(0x51))
            .map(|light| (**light).clone())
            .unwrap();
        let light = Light {
            asset: AssetPath::new(
                "res://packs/LOREGD01/textures/lights/Effect_Church_Window_Elaborate_Tilt.webp",
            )
            .unwrap(),
            center: CellPosition::new(5483.15 / 256.0, 6326.94 / 256.0),
            glow: Glow::new(0.9, 1.0, 0xffec_cd8b).unwrap(),
        };
        assert_eq!(
            as_written(&build_light(&light, NodeId(0x51))),
            as_written(&expected)
        );
    }
}
