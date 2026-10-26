//! Ground painted with terrain textures blending into each other: scorched
//! earth around a blast, ash drifts, slag, mud, worn paths.

use dd_catalog::AssetKind;

use crate::assets::resolve;
use crate::caves::margin;
use crate::{AppError, NaturalShape, Workspace};

/// Narrowest fade at the edge, in cells.
const SHARPEST_FALLOFF: f64 = 0.05;

/// Paints a terrain texture over a shape, fading out at its edge.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainRequest {
    /// Asset path of a terrain texture.
    pub asset: String,
    pub shape: NaturalShape,
    /// Width of the fade at the edge, in cells.
    pub falloff: f64,
    /// From 0 (smooth) to 1 (very jagged) edges for chambers and tunnels.
    pub roughness: f64,
    /// The same seed gives the same edges.
    pub seed: u64,
}

impl Workspace {
    /// Paints a terrain texture; returns how many quarter-cell samples changed.
    ///
    /// # Errors
    /// Fails on an unknown map or texture, an invalid shape, or a fifth texture:
    /// a map blends four at most, the background included.
    pub fn paint_terrain(
        &mut self,
        map: &str,
        request: &TerrainRequest,
    ) -> Result<usize, AppError> {
        let texture = resolve(self.catalog(), &request.asset, AssetKind::Terrain)?
            .path
            .clone();
        let inside = margin(&request.shape, request.roughness, request.seed)?;
        let falloff = request.falloff.max(SHARPEST_FALLOFF);
        let ground = self.map_mut(map)?.terrain_mut();
        let slot = ground.slot_for(&texture).ok_or(AppError::TerrainFull)?;
        ground.set_texture(slot, texture);
        Ok(ground.paint(slot, |position| 0.5 + inside(position) / falloff))
    }
}

#[cfg(test)]
mod tests {
    use dd_domain::Terrain;

    use super::TerrainRequest;
    use crate::assets::tests::{FLOOR, VOID};
    use crate::workspace::tests::workspace;
    use crate::{AppError, NaturalShape, Workspace};

    fn scorch(asset: &str) -> TerrainRequest {
        TerrainRequest {
            asset: asset.to_owned(),
            shape: NaturalShape::Chamber {
                x: 10.0,
                y: 8.0,
                radius: 4.0,
            },
            falloff: 2.0,
            roughness: 0.0,
            seed: 1,
        }
    }

    /// The weight of the texture painted at `(x, y)`, in the second slot.
    fn painted(workspace: &Workspace, map: &str, x: f64, y: f64) -> u8 {
        let (_, found) = workspace.map(map).unwrap();
        let column = (0..found.terrain().columns())
            .find(|&column| Terrain::position(column, 0).x >= x)
            .unwrap();
        let row = (0..found.terrain().rows())
            .find(|&row| Terrain::position(0, row).y >= y)
            .unwrap();
        found.terrain().weights(column, row)[1]
    }

    #[test]
    fn paints_and_fades_out_at_the_edge() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        assert!(workspace.paint_terrain(&map, &scorch(VOID)).unwrap() > 0);
        assert_eq!(painted(&workspace, &map, 10.0, 8.0), 255);
        let edge = painted(&workspace, &map, 14.0, 8.0);
        assert!((100..160).contains(&edge), "{edge}");
        assert_eq!(painted(&workspace, &map, 20.0, 8.0), 0);
        let description = workspace.describe(&map).unwrap();
        assert!(
            description.contains("Terrain: terrain_void"),
            "{description}"
        );
    }

    #[test]
    fn needs_a_terrain_texture() {
        let (mut workspace, _) = workspace();
        let map = workspace.new_map(30, 20).unwrap();
        let wrong = TerrainRequest {
            asset: FLOOR.to_owned(),
            ..scorch(VOID)
        };
        assert!(matches!(
            workspace.paint_terrain(&map, &wrong),
            Err(AppError::WrongAssetKind { .. })
        ));
    }
}
