use std::f64::consts::{FRAC_PI_2, PI};

use dd_domain::{Prop, QuarterTurn};

use super::pixels::CELL_SIZE_PX;
use crate::godot::Vector2;
use crate::model::{NodeId, Object};

/// Layer Dungeondraft puts new objects on ("User Layer 1").
const DEFAULT_OBJECT_LAYER: i32 = 100;

/// A Dungeondraft object for `prop`.
pub fn build_object(prop: &Prop, node_id: NodeId) -> Object {
    Object {
        position: Vector2::new(prop.center.x * CELL_SIZE_PX, prop.center.y * CELL_SIZE_PX),
        rotation: radians(prop.rotation),
        scale: Vector2::new(1.0, 1.0),
        mirror: false,
        texture: prop.asset.as_str().to_owned(),
        layer: DEFAULT_OBJECT_LAYER,
        shadow: true,
        block_light: false,
        node_id,
    }
}

/// Radians, clockwise on screen, in `[-π, π[` like Dungeondraft writes them.
fn radians(rotation: QuarterTurn) -> f64 {
    match rotation {
        QuarterTurn::Deg0 => 0.0,
        QuarterTurn::Deg90 => FRAC_PI_2,
        QuarterTurn::Deg180 => -PI,
        QuarterTurn::Deg270 => -FRAC_PI_2,
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::{AssetPath, CellPosition, Footprint, Prop, QuarterTurn};

    use super::build_object;
    use crate::model::{MapFile, NodeId};

    fn prop(center: CellPosition, rotation: QuarterTurn) -> Prop {
        Prop {
            asset: AssetPath::new(
                "res://packs/LOREGD01/textures/objects/[LORE] Buildings/[LORE] Buildings - Greebles/Decor_Aquila_Ancient.webp",
            )
            .unwrap(),
            center,
            rotation,
            footprint: Footprint::new(1.0, 1.0).unwrap(),
        }
    }

    fn as_written(object: &crate::model::Object) -> String {
        crate::json::to_godot_json(&serde_json::to_value(object).unwrap())
    }

    #[test]
    fn rebuilds_an_object_of_the_second_fixture() {
        let fixture =
            MapFile::parse(include_str!("../../../../fixtures/test2.dungeondraft_map")).unwrap();
        let expected = fixture.world().levels["0"]
            .objects
            .iter()
            .find(|object| object.node_id == NodeId(0x3d))
            .map(|object| (**object).clone())
            .unwrap();
        let object = build_object(
            &prop(CellPosition::new(37.5, 23.5), QuarterTurn::Deg90),
            NodeId(0x3d),
        );
        assert_eq!(as_written(&object), as_written(&expected));
    }

    #[test]
    fn writes_rotations_like_dungeondraft() {
        let center = CellPosition::new(1.0, 1.0);
        let rotations: Vec<String> = [
            QuarterTurn::Deg0,
            QuarterTurn::Deg90,
            QuarterTurn::Deg180,
            QuarterTurn::Deg270,
        ]
        .into_iter()
        .map(|turn| {
            let rotation = build_object(&prop(center, turn), NodeId(0)).rotation;
            crate::json::to_godot_json(&serde_json::json!(rotation))
        })
        .collect();
        assert_eq!(rotations, ["0", "1.570796", "-3.141593", "-1.570796"]);
    }
}
