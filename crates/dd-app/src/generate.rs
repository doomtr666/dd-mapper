//! Buildings drawn by a generator: the outline, its partitions and doors at once.

use dd_catalog::{AssetKind, Catalog};
use dd_gen::{BspParams, DoorSpot, HabParams, Layout, bsp, hab_block, ship_deck};

use crate::assets::resolve;
use crate::{AppError, DoorRequest, RoomRequest, Shape, WallRequest, Workspace};

/// Narrowest room when none is asked for, in cells: 15 feet.
pub const DEFAULT_MIN_ROOM: u32 = 3;

/// The kinds of building a generator draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generator {
    /// Rooms split at random, such as a manufactorum or a bunker.
    Bsp,
    /// Cells on both sides of a corridor, entered at both ends.
    HabBlock,
    /// A hab block with a pointed bow, entered from the stern.
    ShipDeck,
}

/// A building filling a new map, and the assets it is drawn with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutRequest {
    pub generator: Generator,
    /// Map size, in cells; the building leaves a one-cell margin.
    pub width: u32,
    pub height: u32,
    /// Narrowest room, in cells.
    pub min_room: u32,
    /// The same seed draws the same building.
    pub seed: u64,
    /// Asset path of the floor tileset.
    pub floor: String,
    /// Asset path of the wall texture, outline and partitions alike.
    pub wall: String,
    /// Asset path of every door.
    pub door: String,
}

/// What a generator drew.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutReport {
    pub map: String,
    /// The room holding the outline.
    pub room: String,
    /// Areas the partitions split the building into.
    pub areas: usize,
    pub walls: usize,
    pub doors: usize,
}

impl Generator {
    /// The smallest map, in cells, it draws a building with rooms in.
    const fn smallest_map(self) -> (u32, u32) {
        match self {
            Self::Bsp => (3, 3),
            Self::HabBlock | Self::ShipDeck => (8, 6),
        }
    }
}

impl LayoutRequest {
    fn layout(&self) -> Layout {
        let (width, height, min_room, seed) = (self.width, self.height, self.min_room, self.seed);
        match self.generator {
            Generator::Bsp => bsp(BspParams {
                width,
                height,
                min_room,
                seed,
            }),
            Generator::HabBlock | Generator::ShipDeck => {
                let params = HabParams {
                    width,
                    height,
                    min_room,
                    seed,
                };
                if self.generator == Generator::HabBlock {
                    hab_block(params)
                } else {
                    ship_deck(params)
                }
            }
        }
    }

    /// Fails unless the map is large enough and the assets are of the right kinds.
    fn check(&self, catalog: &Catalog) -> Result<(), AppError> {
        let (width, height) = self.generator.smallest_map();
        if self.width < width || self.height < height {
            return Err(AppError::MapTooSmall { width, height });
        }
        resolve(catalog, &self.floor, AssetKind::Tileset)?;
        resolve(catalog, &self.wall, AssetKind::Wall)?;
        resolve(catalog, &self.door, AssetKind::Portal)?;
        Ok(())
    }
}

impl Workspace {
    /// Draws a building on a new map.
    ///
    /// # Errors
    /// Fails when the map is too small for the generator, or on unknown assets;
    /// no map is created then.
    pub fn generate(&mut self, request: &LayoutRequest) -> Result<LayoutReport, AppError> {
        request.check(self.catalog())?;
        let layout = request.layout();
        let map = self.new_map(request.width, request.height)?;
        let outline = RoomRequest {
            shape: Shape::Polygon(
                layout
                    .outline
                    .iter()
                    .map(|point| (point.x, point.y))
                    .collect(),
            ),
            pillars: Vec::new(),
            floor: request.floor.clone(),
            wall: request.wall.clone(),
        };
        let room = self.add_room(&map, &outline)?.id;
        let walls = self.add_partitions(&map, &layout, &request.wall)?;
        for door in &layout.doors {
            let (on, at) = match *door {
                DoorSpot::Outline(at) => (room.clone(), at),
                DoorSpot::Partition { index, at } => (walls[index].clone(), at),
            };
            let door = DoorRequest {
                on,
                x: at.x,
                y: at.y,
                asset: request.door.clone(),
            };
            self.add_door(&map, &door)?;
        }
        Ok(LayoutReport {
            map,
            room,
            areas: layout.areas.len(),
            walls: walls.len(),
            doors: layout.doors.len(),
        })
    }

    /// Adds the partitions of `layout`, returning their ids in order.
    fn add_partitions(
        &mut self,
        map: &str,
        layout: &Layout,
        wall: &str,
    ) -> Result<Vec<String>, AppError> {
        layout
            .partitions
            .iter()
            .map(|points| {
                let request = WallRequest {
                    points: points.iter().map(|point| (point.x, point.y)).collect(),
                    wall: wall.to_owned(),
                };
                self.add_wall(map, &request)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Generator, LayoutRequest};
    use crate::AppError;
    use crate::assets::tests::{CRATE, DOOR, FLOOR, WALL};
    use crate::workspace::tests::workspace;

    fn request(generator: Generator) -> LayoutRequest {
        LayoutRequest {
            generator,
            width: 30,
            height: 14,
            min_room: 3,
            seed: 42,
            floor: FLOOR.to_owned(),
            wall: WALL.to_owned(),
            door: DOOR.to_owned(),
        }
    }

    #[test]
    fn every_generator_draws_a_sound_building() {
        for generator in [Generator::Bsp, Generator::HabBlock, Generator::ShipDeck] {
            let (mut workspace, _) = workspace();
            let report = workspace.generate(&request(generator)).unwrap();
            assert_eq!((report.map.as_str(), report.room.as_str()), ("map-1", "R0"));
            assert!(report.areas > 1, "{generator:?}: {report:?}");
            assert!(
                report.walls >= report.areas - 1,
                "{generator:?}: {report:?}"
            );
            let entrances = if generator == Generator::HabBlock {
                2
            } else {
                1
            };
            assert_eq!(report.doors, report.areas - 1 + entrances, "{generator:?}");
            assert_eq!(workspace.validate(&report.map), Ok(vec![]), "{generator:?}");
        }
    }

    #[test]
    fn the_same_seed_draws_the_same_building() {
        let (mut workspace, _) = workspace();
        let first = workspace.generate(&request(Generator::Bsp)).unwrap();
        let second = workspace.generate(&request(Generator::Bsp)).unwrap();
        assert_eq!(
            workspace
                .describe(&first.map)
                .unwrap()
                .replace("map-1", "map-2"),
            workspace.describe(&second.map).unwrap()
        );
    }

    #[test]
    fn maps_too_small_for_the_generator_are_refused() {
        let (mut workspace, _) = workspace();
        let tiny = LayoutRequest {
            height: 5,
            ..request(Generator::HabBlock)
        };
        assert_eq!(
            workspace.generate(&tiny),
            Err(AppError::MapTooSmall {
                width: 8,
                height: 6
            })
        );
    }

    #[test]
    fn wrong_assets_are_refused_before_any_map_is_created() {
        let (mut workspace, _) = workspace();
        let crate_door = LayoutRequest {
            door: CRATE.to_owned(),
            ..request(Generator::Bsp)
        };
        assert!(matches!(
            workspace.generate(&crate_door),
            Err(AppError::WrongAssetKind { .. })
        ));
        assert_eq!(workspace.new_map(10, 10), Ok("map-1".to_owned()));
    }
}
