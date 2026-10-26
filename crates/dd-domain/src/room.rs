use thiserror::Error;

use crate::{AssetPath, Cell, Polygon};

/// One closed wall loop of a room: its outline or one of its pillars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallLoop {
    Outline,
    Pillar(usize),
}

/// A walled area. Pillars are solid blocks inside it, walled all around.
///
/// The floor is not part of the room: like in Dungeondraft, it is a separate
/// layer of the map, so merged rooms keep each part's tileset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Room {
    outline: Polygon,
    pillars: Vec<Polygon>,
    wall: AssetPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RoomError {
    #[error("a pillar must lie strictly inside the room, without touching its walls")]
    PillarNotInside,
    #[error("a pillar cannot touch or overlap pillar {existing}")]
    PillarsOverlap { existing: usize },
}

impl Room {
    #[must_use]
    pub fn new(outline: Polygon, wall: AssetPath) -> Self {
        Self {
            outline,
            pillars: Vec::new(),
            wall,
        }
    }

    /// Adds a pillar and returns its index.
    ///
    /// # Errors
    /// Fails when the pillar is not strictly inside the room
    /// or touches another pillar.
    pub fn add_pillar(&mut self, pillar: Polygon) -> Result<usize, RoomError> {
        if !self.outline.strictly_contains_polygon(&pillar) {
            return Err(RoomError::PillarNotInside);
        }
        if let Some(existing) = self.pillar_touching(&pillar) {
            return Err(RoomError::PillarsOverlap { existing });
        }
        self.pillars.push(pillar);
        Ok(self.pillars.len() - 1)
    }

    fn pillar_touching(&self, candidate: &Polygon) -> Option<usize> {
        self.pillars
            .iter()
            .position(|pillar| pillar.touches_or_overlaps(candidate))
    }

    #[must_use]
    pub fn outline(&self) -> &Polygon {
        &self.outline
    }

    #[must_use]
    pub fn pillars(&self) -> &[Polygon] {
        &self.pillars
    }

    /// Texture of every wall of the room.
    #[must_use]
    pub fn wall(&self) -> &AssetPath {
        &self.wall
    }

    #[must_use]
    pub fn wall_loop(&self, wall: WallLoop) -> Option<&Polygon> {
        match wall {
            WallLoop::Outline => Some(&self.outline),
            WallLoop::Pillar(index) => self.pillars.get(index),
        }
    }

    /// Cells sharing some area with the room: overlapping the outline
    /// and not entirely covered by a pillar.
    pub fn floor_cells(&self) -> impl Iterator<Item = Cell> + '_ {
        self.outline.bounding_cells().filter(|cell| {
            self.outline.overlaps_cell(*cell)
                && !self.pillars.iter().any(|pillar| pillar.covers_cell(*cell))
        })
    }

    /// The outline first, then every pillar.
    pub fn wall_loops(&self) -> impl Iterator<Item = (WallLoop, &Polygon)> {
        let pillars = self
            .pillars
            .iter()
            .enumerate()
            .map(|(index, pillar)| (WallLoop::Pillar(index), pillar));
        std::iter::once((WallLoop::Outline, &self.outline)).chain(pillars)
    }
}

#[cfg(test)]
mod tests {
    use super::{Room, RoomError, WallLoop};
    use crate::{AssetPath, Cell, CellPoint, Polygon};

    fn square(x: i32, y: i32, side: u32) -> Polygon {
        Polygon::rectangle(CellPoint::new(x, y), side, side).unwrap()
    }

    fn wall() -> AssetPath {
        AssetPath::new("wall.webp").unwrap()
    }

    fn hall() -> Room {
        Room::new(square(0, 0, 10), wall())
    }

    #[test]
    fn accepts_a_pillar_inside_the_room() {
        let mut room = hall();
        assert_eq!(room.add_pillar(square(4, 4, 1)), Ok(0));
        assert_eq!(room.add_pillar(square(7, 7, 2)), Ok(1));
        assert_eq!(room.wall_loop(WallLoop::Pillar(1)), Some(&square(7, 7, 2)));
    }

    #[test]
    fn rejects_pillars_touching_crossing_or_outside_the_walls() {
        for pillar in [square(0, 4, 1), square(8, 4, 4), square(20, 20, 1)] {
            assert_eq!(hall().add_pillar(pillar), Err(RoomError::PillarNotInside));
        }
    }

    #[test]
    fn rejects_pillars_overlapping_touching_or_containing_another() {
        for (first, second) in [
            (square(4, 4, 2), square(5, 5, 2)),
            (square(4, 4, 1), square(5, 4, 1)),
            (square(2, 2, 5), square(3, 3, 1)),
            (square(3, 3, 1), square(2, 2, 5)),
        ] {
            let mut room = hall();
            room.add_pillar(first).unwrap();
            assert_eq!(
                room.add_pillar(second),
                Err(RoomError::PillarsOverlap { existing: 0 })
            );
        }
    }

    #[test]
    fn floor_covers_the_cells_of_the_reference_room() {
        let room = Room::new(
            Polygon::rectangle(CellPoint::new(13, 5), 17, 11).unwrap(),
            wall(),
        );
        let cells: Vec<Cell> = room.floor_cells().collect();
        assert_eq!(cells.len(), 17 * 11);
        assert_eq!(cells.first(), Some(&Cell::new(13, 5)));
        assert_eq!(cells.last(), Some(&Cell::new(29, 15)));
    }

    #[test]
    fn floor_includes_cells_cut_by_an_oblique_wall() {
        let slanted = [(20, 2), (28, 2), (28, 10), (24, 14), (20, 14)]
            .into_iter()
            .map(|(x, y)| CellPoint::new(x, y))
            .collect();
        let room = Room::new(Polygon::new(slanted).unwrap(), wall());
        let cells: Vec<Cell> = room.floor_cells().collect();
        assert_eq!(cells.len(), 8 * 12 - 6);
        assert!(cells.contains(&Cell::new(27, 10)));
        assert!(cells.contains(&Cell::new(24, 13)));
        assert!(!cells.contains(&Cell::new(27, 11)));
    }

    #[test]
    fn floor_excludes_pillars() {
        let mut room = hall();
        room.add_pillar(square(4, 4, 1)).unwrap();
        let cells: Vec<Cell> = room.floor_cells().collect();
        assert_eq!(cells.len(), 99);
        assert!(!cells.contains(&Cell::new(4, 4)));
    }

    #[test]
    fn lists_the_outline_then_the_pillars() {
        let mut room = hall();
        room.add_pillar(square(4, 4, 1)).unwrap();
        let loops: Vec<WallLoop> = room.wall_loops().map(|(wall, _)| wall).collect();
        assert_eq!(loops, [WallLoop::Outline, WallLoop::Pillar(0)]);
        assert_eq!(room.wall_loop(WallLoop::Outline), Some(&square(0, 0, 10)));
        assert_eq!(room.wall_loop(WallLoop::Pillar(1)), None);
    }
}
