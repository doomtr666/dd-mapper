use std::collections::BTreeMap;

use thiserror::Error;

use crate::{
    AssetPath, CaveGrid, Cell, CellPoint, CellPosition, GridSize, Light, MergeError, Partition,
    Pattern, Polygon, Prop, Room, Segment, Track, WallLoop, joins, merge,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoomId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DoorId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PropId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartitionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LightId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PatternId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrackId(pub u32);

/// A wall that can hold doors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallRef {
    /// The outline or a pillar of a room.
    Room { room: RoomId, wall: WallLoop },
    /// A free-standing wall.
    Partition(PartitionId),
}

/// A point on a wall: segment `segment` of `wall`, at fraction `t` of its length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallPosition {
    pub wall: WallRef,
    pub segment: usize,
    pub t: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Door {
    pub position: WallPosition,
    pub asset: AssetPath,
}

/// Outcome of [`Map::add_room`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedRoom {
    /// The room now holding the new walls: new, or the oldest merged room.
    pub id: RoomId,
    /// Rooms merged into `id`, which no longer exist.
    pub absorbed: Vec<RoomId>,
    /// Doors whose wall disappeared in the merge.
    pub removed_doors: Vec<DoorId>,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum MapError {
    #[error(transparent)]
    Merge(#[from] MergeError),
    #[error("room vertex ({}, {}) lies outside the map", vertex.x, vertex.y)]
    RoomOutsideGrid { vertex: CellPoint },
    #[error("there is no room {0:?}")]
    UnknownRoom(RoomId),
    #[error("room {room:?} has no wall loop {wall:?}")]
    UnknownWall { room: RoomId, wall: WallLoop },
    #[error("segment {segment} does not exist, this wall has {count} segments")]
    UnknownSegment { segment: usize, count: usize },
    #[error("a door position must be strictly between 0 and 1 along its segment, got {t}")]
    PositionOffSegment { t: f64 },
    #[error("there is no wall {0:?}")]
    UnknownPartition(PartitionId),
    #[error("wall point ({}, {}) lies outside the map", point.x, point.y)]
    PartitionOutsideGrid { point: CellPoint },
    #[error("pattern vertex ({}, {}) lies outside the map", vertex.x, vertex.y)]
    PatternOutsideGrid { vertex: CellPoint },
    #[error("light center ({}, {}) lies outside the map", center.x, center.y)]
    LightOutsideGrid { center: CellPosition },
    #[error("prop center ({}, {}) lies outside the map", center.x, center.y)]
    PropOutsideGrid { center: CellPosition },
    #[error("floor vertex ({}, {}) lies outside the map", vertex.x, vertex.y)]
    FloorOutsideGrid { vertex: CellPoint },
    #[error("the path lies wholly outside the map")]
    TrackOutsideGrid,
    #[error("there is no prop {0:?}")]
    UnknownProp(PropId),
}

/// The whole map: its grid, rooms, free-standing walls, doors, props and floor tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    grid: GridSize,
    rooms: Vec<(RoomId, Room)>,
    doors: Vec<(DoorId, Door)>,
    floor: BTreeMap<Cell, AssetPath>,
    props: Vec<(PropId, Prop)>,
    partitions: Vec<(PartitionId, Partition)>,
    lights: Vec<(LightId, Light)>,
    patterns: Vec<(PatternId, Pattern)>,
    tracks: Vec<(TrackId, Track)>,
    cave: CaveGrid,
    /// Terrain texture covering the whole ground, under rooms.
    background: Option<AssetPath>,
    /// ARGB light everywhere, before lights add theirs.
    ambient_light: Option<u32>,
    next_room_id: u32,
    next_door_id: u32,
    next_prop_id: u32,
    next_partition_id: u32,
    next_light_id: u32,
    next_pattern_id: u32,
    next_track_id: u32,
}

impl Map {
    #[must_use]
    pub fn new(grid: GridSize) -> Self {
        Self {
            grid,
            rooms: Vec::new(),
            doors: Vec::new(),
            floor: BTreeMap::new(),
            props: Vec::new(),
            partitions: Vec::new(),
            lights: Vec::new(),
            patterns: Vec::new(),
            tracks: Vec::new(),
            cave: CaveGrid::new(grid),
            background: None,
            ambient_light: None,
            next_room_id: 0,
            next_door_id: 0,
            next_prop_id: 0,
            next_partition_id: 0,
            next_light_id: 0,
            next_pattern_id: 0,
            next_track_id: 0,
        }
    }

    #[must_use]
    pub fn grid(&self) -> GridSize {
        self.grid
    }

    /// Adds a room and paints its floor cells with the `floor` tileset,
    /// over any floor already there.
    ///
    /// Like Dungeondraft's room tool, a room sharing area or a wall length with
    /// existing rooms merges with them; their doors follow the remaining walls.
    ///
    /// # Errors
    /// Fails, leaving the map unchanged, when the room leaves the grid
    /// or cannot merge into a valid room.
    pub fn add_room(&mut self, room: Room, floor: &AssetPath) -> Result<AddedRoom, MapError> {
        let floor_cells: Vec<Cell> = room.floor_cells().collect();
        let added = self.add_walls(room)?;
        for cell in floor_cells {
            self.paint_floor(cell, floor);
        }
        Ok(added)
    }

    /// Adds a room's walls, merging like [`Map::add_room`], without painting its floor.
    ///
    /// # Errors
    /// Fails, leaving the map unchanged, when the room leaves the grid
    /// or cannot merge into a valid room.
    pub fn add_walls(&mut self, room: Room) -> Result<AddedRoom, MapError> {
        self.check_inside_grid(&room)?;
        let joined: Vec<RoomId> = self
            .rooms
            .iter()
            .filter(|(_, existing)| joins(existing, &room))
            .map(|(id, _)| *id)
            .collect();
        match joined.split_first() {
            None => Ok(self.insert_room(room)),
            Some((kept, absorbed)) => self.merge_into(*kept, absorbed, &room),
        }
    }

    /// Paints one cell with a floor tileset.
    pub fn paint_floor(&mut self, cell: Cell, tileset: &AssetPath) {
        self.floor.insert(cell, tileset.clone());
    }

    /// Paints every cell the area overlaps with a floor tileset, walls or not;
    /// returns how many.
    ///
    /// # Errors
    /// Fails when a vertex of the area lies outside the map.
    pub fn paint_area(&mut self, area: &Polygon, tileset: &AssetPath) -> Result<usize, MapError> {
        if let Some(vertex) = self.first_outside_grid(area.vertices()) {
            return Err(MapError::FloorOutsideGrid { vertex });
        }
        let cells: Vec<Cell> = area
            .bounding_cells()
            .filter(|cell| area.overlaps_cell(*cell))
            .collect();
        for cell in &cells {
            self.paint_floor(*cell, tileset);
        }
        Ok(cells.len())
    }

    /// Where the door stands on its wall.
    #[must_use]
    pub fn door_position(&self, id: DoorId) -> Option<CellPosition> {
        let (_, door) = self.doors.iter().find(|(candidate, _)| *candidate == id)?;
        self.door_point(door)
    }

    /// # Errors
    /// Fails when the door does not sit strictly inside a segment of an existing wall.
    pub fn add_door(&mut self, door: Door) -> Result<DoorId, MapError> {
        self.check_door_position(door.position)?;
        let id = DoorId(allocate(&mut self.next_door_id));
        self.doors.push((id, door));
        Ok(id)
    }

    #[must_use]
    pub fn room(&self, id: RoomId) -> Option<&Room> {
        self.rooms
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, room)| room)
    }

    fn first_outside_grid(&self, points: &[CellPoint]) -> Option<CellPoint> {
        points
            .iter()
            .copied()
            .find(|point| !self.grid.contains(*point))
    }

    fn check_inside_grid(&self, room: &Room) -> Result<(), MapError> {
        match self.first_outside_grid(room.outline().vertices()) {
            Some(vertex) => Err(MapError::RoomOutsideGrid { vertex }),
            None => Ok(()),
        }
    }

    fn insert_room(&mut self, room: Room) -> AddedRoom {
        let id = RoomId(allocate(&mut self.next_room_id));
        self.rooms.push((id, room));
        AddedRoom {
            id,
            absorbed: Vec::new(),
            removed_doors: Vec::new(),
        }
    }

    /// Merges `added` with rooms `kept` and `absorbed` into room `kept`.
    /// Everything is computed before the map changes, so errors leave it intact.
    fn merge_into(
        &mut self,
        kept: RoomId,
        absorbed: &[RoomId],
        added: &Room,
    ) -> Result<AddedRoom, MapError> {
        let merged_ids: Vec<RoomId> = std::iter::once(kept)
            .chain(absorbed.iter().copied())
            .collect();
        let parts: Vec<&Room> = merged_ids
            .iter()
            .filter_map(|id| self.room(*id))
            .chain(std::iter::once(added))
            .collect();
        let merged = merge(&parts)?;
        let relocations = self.relocate_doors(&merged_ids, &merged);

        self.rooms.retain(|(id, _)| !absorbed.contains(id));
        if let Some((_, room)) = self.rooms.iter_mut().find(|(id, _)| *id == kept) {
            *room = merged;
        }
        let removed_doors = self.apply_relocations(relocations);
        Ok(AddedRoom {
            id: kept,
            absorbed: absorbed.to_vec(),
            removed_doors,
        })
    }

    /// New wall positions, on `merged`, of the doors of `rooms`;
    /// `None` when their wall is gone.
    fn relocate_doors(
        &self,
        rooms: &[RoomId],
        merged: &Room,
    ) -> Vec<(DoorId, Option<WallPosition>)> {
        let kept = rooms[0];
        self.doors
            .iter()
            .filter(|(_, door)| {
                matches!(door.position.wall, WallRef::Room { room, .. } if rooms.contains(&room))
            })
            .map(|(id, door)| {
                let position = self
                    .door_point(door)
                    .and_then(|point| locate_on_walls(kept, merged, point));
                (*id, position)
            })
            .collect()
    }

    fn apply_relocations(
        &mut self,
        relocations: Vec<(DoorId, Option<WallPosition>)>,
    ) -> Vec<DoorId> {
        let mut removed = Vec::new();
        for (id, position) in relocations {
            match position {
                Some(position) => self.move_door(id, position),
                None => removed.push(id),
            }
        }
        self.doors.retain(|(id, _)| !removed.contains(id));
        removed
    }

    fn move_door(&mut self, id: DoorId, position: WallPosition) {
        if let Some((_, door)) = self
            .doors
            .iter_mut()
            .find(|(candidate, _)| *candidate == id)
        {
            door.position = position;
        }
    }

    fn door_point(&self, door: &Door) -> Option<CellPosition> {
        let segment = self.wall_segment(&door.position)?;
        Some(segment.point_at(door.position.t))
    }

    /// The wall segment a position lies on, if that wall exists.
    #[must_use]
    pub fn wall_segment(&self, position: &WallPosition) -> Option<Segment> {
        match position.wall {
            WallRef::Room { room, wall } => {
                self.room(room)?.wall_loop(wall)?.segment(position.segment)
            }
            WallRef::Partition(id) => self.partition(id)?.segment(position.segment),
        }
    }

    fn segment_count(&self, wall: WallRef) -> Result<usize, MapError> {
        match wall {
            WallRef::Room { room, wall } => {
                let ring = self
                    .room(room)
                    .ok_or(MapError::UnknownRoom(room))?
                    .wall_loop(wall)
                    .ok_or(MapError::UnknownWall { room, wall })?;
                Ok(ring.segment_count())
            }
            WallRef::Partition(id) => self
                .partition(id)
                .map(Partition::segment_count)
                .ok_or(MapError::UnknownPartition(id)),
        }
    }

    fn check_door_position(&self, position: WallPosition) -> Result<(), MapError> {
        let count = self.segment_count(position.wall)?;
        if position.segment >= count {
            return Err(MapError::UnknownSegment {
                segment: position.segment,
                count,
            });
        }
        let strictly_inside_segment = position.t > 0.0 && position.t < 1.0;
        if !strictly_inside_segment {
            return Err(MapError::PositionOffSegment { t: position.t });
        }
        Ok(())
    }

    pub fn rooms(&self) -> impl Iterator<Item = (RoomId, &Room)> {
        self.rooms.iter().map(|(id, room)| (*id, room))
    }

    pub fn doors(&self) -> impl Iterator<Item = (DoorId, &Door)> {
        self.doors.iter().map(|(id, door)| (*id, door))
    }

    /// # Errors
    /// Fails when the prop's center lies outside the map.
    pub fn add_prop(&mut self, prop: Prop) -> Result<PropId, MapError> {
        if !self.grid.contains_position(prop.center) {
            return Err(MapError::PropOutsideGrid {
                center: prop.center,
            });
        }
        let id = PropId(allocate(&mut self.next_prop_id));
        self.props.push((id, prop));
        Ok(id)
    }

    /// # Errors
    /// Fails when there is no such prop.
    pub fn remove_prop(&mut self, id: PropId) -> Result<Prop, MapError> {
        let index = self
            .props
            .iter()
            .position(|(candidate, _)| *candidate == id)
            .ok_or(MapError::UnknownProp(id))?;
        Ok(self.props.remove(index).1)
    }

    pub fn props(&self) -> impl Iterator<Item = (PropId, &Prop)> {
        self.props.iter().map(|(id, prop)| (*id, prop))
    }

    /// Adds a free-standing wall, which merging rooms leave untouched.
    ///
    /// # Errors
    /// Fails when a point of the wall lies outside the map.
    pub fn add_partition(&mut self, partition: Partition) -> Result<PartitionId, MapError> {
        if let Some(point) = self.first_outside_grid(partition.points()) {
            return Err(MapError::PartitionOutsideGrid { point });
        }
        let id = PartitionId(allocate(&mut self.next_partition_id));
        self.partitions.push((id, partition));
        Ok(id)
    }

    #[must_use]
    pub fn partition(&self, id: PartitionId) -> Option<&Partition> {
        self.partitions
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, partition)| partition)
    }

    pub fn partitions(&self) -> impl Iterator<Item = (PartitionId, &Partition)> {
        self.partitions
            .iter()
            .map(|(id, partition)| (*id, partition))
    }

    /// # Errors
    /// Fails when the light's center lies outside the map.
    pub fn add_light(&mut self, light: Light) -> Result<LightId, MapError> {
        if !self.grid.contains_position(light.center) {
            return Err(MapError::LightOutsideGrid {
                center: light.center,
            });
        }
        let id = LightId(allocate(&mut self.next_light_id));
        self.lights.push((id, light));
        Ok(id)
    }

    pub fn lights(&self) -> impl Iterator<Item = (LightId, &Light)> {
        self.lights.iter().map(|(id, light)| (*id, light))
    }

    /// # Errors
    /// Fails when a vertex of the pattern lies outside the map.
    pub fn add_pattern(&mut self, pattern: Pattern) -> Result<PatternId, MapError> {
        if let Some(vertex) = self.first_outside_grid(pattern.area.vertices()) {
            return Err(MapError::PatternOutsideGrid { vertex });
        }
        let id = PatternId(allocate(&mut self.next_pattern_id));
        self.patterns.push((id, pattern));
        Ok(id)
    }

    pub fn patterns(&self) -> impl Iterator<Item = (PatternId, &Pattern)> {
        self.patterns.iter().map(|(id, pattern)| (*id, pattern))
    }

    #[must_use]
    pub fn cave(&self) -> &CaveGrid {
        &self.cave
    }

    /// Caves are dug freely: no wall or room depends on them.
    pub fn cave_mut(&mut self) -> &mut CaveGrid {
        &mut self.cave
    }

    pub fn set_ambient_light(&mut self, argb: u32) {
        self.ambient_light = Some(argb);
    }

    #[must_use]
    pub fn ambient_light(&self) -> Option<u32> {
        self.ambient_light
    }

    pub fn set_background(&mut self, terrain: AssetPath) {
        self.background = Some(terrain);
    }

    #[must_use]
    pub fn background(&self) -> Option<&AssetPath> {
        self.background.as_ref()
    }

    /// Adds a track; like in Dungeondraft, it may run off the edges of the map.
    ///
    /// # Errors
    /// Fails when no point of the track lies on the map.
    pub fn add_track(&mut self, track: Track) -> Result<TrackId, MapError> {
        if !track
            .points()
            .iter()
            .any(|point| self.grid.contains_position(*point))
        {
            return Err(MapError::TrackOutsideGrid);
        }
        let id = TrackId(allocate(&mut self.next_track_id));
        self.tracks.push((id, track));
        Ok(id)
    }

    pub fn tracks(&self) -> impl Iterator<Item = (TrackId, &Track)> {
        self.tracks.iter().map(|(id, track)| (*id, track))
    }

    /// Every asset the map uses, once per use.
    pub fn assets(&self) -> impl Iterator<Item = &AssetPath> {
        let rooms = self.rooms.iter().map(|(_, room)| room.wall());
        let partitions = self
            .partitions
            .iter()
            .map(|(_, partition)| partition.wall());
        let doors = self.doors.iter().map(|(_, door)| &door.asset);
        let props = self.props.iter().map(|(_, prop)| &prop.asset);
        let lights = self.lights.iter().map(|(_, light)| &light.asset);
        let patterns = self.patterns.iter().map(|(_, pattern)| &pattern.asset);
        let tracks = self.tracks.iter().map(|(_, track)| track.asset());
        self.floor
            .values()
            .chain(rooms)
            .chain(partitions)
            .chain(doors)
            .chain(props)
            .chain(lights)
            .chain(patterns)
            .chain(tracks)
            .chain(self.background.iter())
    }

    /// Floor tileset of every painted cell, column by column.
    pub fn floor(&self) -> impl Iterator<Item = (Cell, &AssetPath)> {
        self.floor.iter().map(|(cell, tileset)| (*cell, tileset))
    }
}

/// The wall position of `point` on room `id`, if a wall of the room passes there.
fn locate_on_walls(id: RoomId, room: &Room, point: CellPosition) -> Option<WallPosition> {
    room.wall_loops().find_map(|(wall, ring)| {
        ring.segments().enumerate().find_map(|(segment, line)| {
            line.locate(point).map(|t| WallPosition {
                wall: WallRef::Room { room: id, wall },
                segment,
                t,
            })
        })
    })
}

fn allocate(next_id: &mut u32) -> u32 {
    let id = *next_id;
    *next_id += 1;
    id
}

#[cfg(test)]
mod tests {
    use super::{
        AddedRoom, Door, DoorId, LightId, Map, MapError, PartitionId, PatternId, PropId, RoomId,
        TrackId, WallPosition, WallRef,
    };
    use crate::{
        AssetPath, Cell, CellPoint, CellPosition, Footprint, Glow, GridSize, Light, Partition,
        Pattern, Polygon, Prop, QuarterTurn, Room, Track, WallLoop,
    };

    fn map() -> Map {
        Map::new(GridSize::new(35, 20).unwrap())
    }

    fn room(x: i32, y: i32, width: u32, height: u32) -> Room {
        Room::new(
            Polygon::rectangle(CellPoint::new(x, y), width, height).unwrap(),
            AssetPath::new("wall.webp").unwrap(),
        )
    }

    fn floor() -> AssetPath {
        AssetPath::new("floor.webp").unwrap()
    }

    fn door(room: RoomId, wall: WallLoop, segment: usize, t: f64) -> Door {
        Door {
            position: WallPosition {
                wall: WallRef::Room { room, wall },
                segment,
                t,
            },
            asset: AssetPath::new("door.webp").unwrap(),
        }
    }

    #[test]
    fn rooms_apart_get_distinct_ids() {
        let mut map = map();
        let first = map.add_room(room(13, 5, 17, 11), &floor()).unwrap();
        let second = map.add_room(room(0, 0, 5, 5), &floor()).unwrap();
        assert_ne!(first.id, second.id);
        assert!(second.absorbed.is_empty() && second.removed_doors.is_empty());
        assert_eq!(map.room(second.id), Some(&room(0, 0, 5, 5)));
        assert_eq!(map.rooms().count(), 2);
    }

    fn tileset(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    #[test]
    fn overlapping_rooms_merge_into_the_oldest() {
        let mut map = map();
        let first = map
            .add_room(room(0, 0, 4, 4), &tileset("a.webp"))
            .unwrap()
            .id;
        let added = map.add_room(room(2, 0, 4, 4), &tileset("b.webp")).unwrap();
        assert_eq!(
            added,
            AddedRoom {
                id: first,
                absorbed: vec![],
                removed_doors: vec![]
            }
        );
        assert_eq!(map.rooms().count(), 1);
        assert_eq!(map.room(first).unwrap().outline().double_area(), 2 * 6 * 4);
    }

    #[test]
    fn each_part_keeps_its_floor_and_the_newest_wins_on_overlap() {
        let mut map = map();
        map.add_room(room(0, 0, 4, 1), &tileset("a.webp")).unwrap();
        map.add_room(room(2, 0, 4, 1), &tileset("b.webp")).unwrap();
        let floor: Vec<&str> = map.floor().map(|(_, tile)| tile.as_str()).collect();
        assert_eq!(
            floor,
            ["a.webp", "a.webp", "b.webp", "b.webp", "b.webp", "b.webp"]
        );
    }

    #[test]
    fn a_room_bridging_two_rooms_merges_all_three() {
        let mut map = map();
        let left = map.add_room(room(0, 0, 2, 2), &floor()).unwrap().id;
        let right = map.add_room(room(4, 0, 2, 2), &floor()).unwrap().id;
        let added = map.add_room(room(1, 0, 4, 1), &floor()).unwrap();
        assert_eq!((added.id, added.absorbed), (left, vec![right]));
        assert_eq!(map.rooms().map(|(id, _)| id).collect::<Vec<_>>(), [left]);
    }

    #[test]
    fn doors_on_remaining_walls_follow_the_merge() {
        let mut map = map();
        let hall = map.add_room(room(0, 0, 4, 4), &floor()).unwrap().id;
        let entrance = map
            .add_door(door(hall, WallLoop::Outline, 0, 0.25))
            .unwrap();
        assert_eq!(
            map.door_position(entrance),
            Some(CellPosition::new(3.0, 4.0))
        );
        let added = map.add_room(room(4, 0, 4, 4), &floor()).unwrap();
        assert!(added.removed_doors.is_empty());
        assert_eq!(
            map.door_position(entrance),
            Some(CellPosition::new(3.0, 4.0))
        );
    }

    #[test]
    fn doors_on_vanished_walls_are_removed() {
        let mut map = map();
        let hall = map.add_room(room(0, 0, 4, 4), &floor()).unwrap().id;
        let inner = map.add_door(door(hall, WallLoop::Outline, 3, 0.5)).unwrap();
        let added = map.add_room(room(4, 0, 4, 4), &floor()).unwrap();
        assert_eq!(added.removed_doors, [inner]);
        assert_eq!(map.doors().count(), 0);
    }

    #[test]
    fn a_failed_merge_leaves_the_map_unchanged() {
        let mut map = map();
        let u_shape = [
            (0, 0),
            (6, 0),
            (6, 6),
            (4, 6),
            (4, 2),
            (2, 2),
            (2, 6),
            (0, 6),
        ]
        .into_iter()
        .map(|(x, y)| CellPoint::new(x, y))
        .collect();
        let u_room = Room::new(Polygon::new(u_shape).unwrap(), tileset("wall.webp"));
        map.add_room(u_room, &floor()).unwrap();
        let before = map.clone();
        let pinching_bar = room(0, 6, 4, 2);
        let result = map.add_room(pinching_bar, &tileset("other.webp"));
        assert!(matches!(result, Err(MapError::Merge(_))), "{result:?}");
        assert_eq!(map, before);
    }

    #[test]
    fn paints_the_floor_of_new_rooms() {
        let mut map = map();
        map.add_room(room(1, 2, 2, 1), &floor()).unwrap();
        let painted: Vec<(Cell, &AssetPath)> = map.floor().collect();
        let tileset = floor();
        assert_eq!(
            painted,
            [(Cell::new(1, 2), &tileset), (Cell::new(2, 2), &tileset)]
        );
    }

    fn prop_at(x: f64, y: f64) -> Prop {
        Prop {
            asset: AssetPath::new("crate.webp").unwrap(),
            center: CellPosition::new(x, y),
            rotation: QuarterTurn::Deg0,
            footprint: Footprint::new(1.0, 1.0).unwrap(),
            tint: None,
        }
    }

    #[test]
    fn props_get_distinct_ids() {
        let mut map = map();
        let first = map.add_prop(prop_at(3.5, 4.5)).unwrap();
        let second = map.add_prop(prop_at(35.0, 20.0)).unwrap();
        assert_eq!((first, second), (PropId(0), PropId(1)));
        assert_eq!(map.props().count(), 2);
    }

    #[test]
    fn removed_props_leave_their_id_unused() {
        let mut map = map();
        let first = map.add_prop(prop_at(3.5, 4.5)).unwrap();
        map.add_prop(prop_at(5.5, 4.5)).unwrap();
        assert_eq!(map.remove_prop(first), Ok(prop_at(3.5, 4.5)));
        assert_eq!(map.remove_prop(first), Err(MapError::UnknownProp(first)));
        assert_eq!(map.add_prop(prop_at(3.5, 4.5)), Ok(PropId(2)));
        assert_eq!(map.props().count(), 2);
    }

    #[test]
    fn areas_repaint_the_floor_cells_they_cover() {
        let mut map = map();
        let plain = AssetPath::new("plain.webp").unwrap();
        let grate = AssetPath::new("grate.webp").unwrap();
        map.add_room(room(0, 0, 4, 3), &plain).unwrap();
        let area = Polygon::rectangle(CellPoint::new(2, 1), 2, 2).unwrap();
        assert_eq!(map.paint_area(&area, &grate), Ok(4));
        let grated: Vec<Cell> = map
            .floor()
            .filter(|(_, tileset)| **tileset == grate)
            .map(|(cell, _)| cell)
            .collect();
        assert_eq!(
            grated,
            [
                Cell::new(2, 1),
                Cell::new(2, 2),
                Cell::new(3, 1),
                Cell::new(3, 2)
            ]
        );
        assert_eq!(map.floor().count(), 12);
        let outside = Polygon::rectangle(CellPoint::new(34, 0), 2, 2).unwrap();
        assert!(matches!(
            map.paint_area(&outside, &grate),
            Err(MapError::FloorOutsideGrid { vertex }) if vertex.x == 36
        ));
    }

    #[test]
    fn lists_every_asset_it_uses() {
        let mut map = map();
        let path = |name: &str| AssetPath::new(name).unwrap();
        map.add_room(room(0, 0, 6, 4), &path("floor.webp")).unwrap();
        let wall_ref = WallRef::Room {
            room: RoomId(0),
            wall: WallLoop::Outline,
        };
        let door = Door {
            position: WallPosition {
                wall: wall_ref,
                segment: 0,
                t: 0.5,
            },
            asset: path("door.webp"),
        };
        map.add_door(door).unwrap();
        map.add_partition(partition(&[(3, 0), (3, 4)])).unwrap();
        map.add_prop(prop_at(1.5, 1.5)).unwrap();
        map.set_background(path("void.png"));
        let mut assets: Vec<&str> = map.assets().map(AssetPath::as_str).collect();
        assets.sort_unstable();
        assets.dedup();
        assert_eq!(
            assets,
            [
                "crate.webp",
                "door.webp",
                "floor.webp",
                "void.png",
                "wall.webp"
            ]
        );
    }

    fn belt(points: &[(f64, f64)]) -> Track {
        let points = points
            .iter()
            .map(|&(x, y)| CellPosition::new(x, y))
            .collect();
        Track::new(AssetPath::new("belt.webp").unwrap(), points, 1.0, false).unwrap()
    }

    #[test]
    fn tracks_get_distinct_ids_and_may_leave_the_map() {
        let mut map = map();
        assert_eq!(
            map.add_track(belt(&[(1.0, 1.0), (5.0, 1.0)])),
            Ok(TrackId(0))
        );
        assert_eq!(
            map.add_track(belt(&[(1.0, 2.0), (40.0, 2.0)])),
            Ok(TrackId(1))
        );
        assert_eq!(map.tracks().count(), 2);
        assert_eq!(
            map.add_track(belt(&[(40.0, 1.0), (50.0, 1.0)])),
            Err(MapError::TrackOutsideGrid)
        );
        assert!(map.assets().any(|asset| asset.as_str() == "belt.webp"));
    }

    #[test]
    fn caves_cover_the_whole_map() {
        let mut map = map();
        assert!(map.cave().is_empty());
        assert_eq!((map.cave().columns(), map.cave().rows()), (143, 83));
        map.cave_mut()
            .dig(|position| position.x < 3.0 && position.y < 3.0, true);
        assert!(map.cave().is_floor_at(CellPosition::new(1.0, 1.0)));
    }

    #[test]
    fn the_ambient_light_is_unset_until_chosen() {
        let mut map = map();
        assert_eq!(map.ambient_light(), None);
        map.set_ambient_light(0xff40_4858);
        assert_eq!(map.ambient_light(), Some(0xff40_4858));
    }

    #[test]
    fn the_background_is_unset_until_chosen() {
        let mut map = map();
        assert_eq!(map.background(), None);
        let void = AssetPath::new("res://textures/terrain/terrain_void.png").unwrap();
        map.set_background(void.clone());
        assert_eq!(map.background(), Some(&void));
    }

    #[test]
    fn rejects_props_outside_the_map() {
        assert_eq!(
            map().add_prop(prop_at(-0.5, 3.0)),
            Err(MapError::PropOutsideGrid {
                center: CellPosition::new(-0.5, 3.0)
            })
        );
    }

    fn partition(coordinates: &[(i32, i32)]) -> Partition {
        let points = coordinates
            .iter()
            .map(|&(x, y)| CellPoint::new(x, y))
            .collect();
        Partition::new(points, AssetPath::new("wall.webp").unwrap()).unwrap()
    }

    fn door_in_partition(id: PartitionId, segment: usize, t: f64) -> Door {
        Door {
            position: WallPosition {
                wall: WallRef::Partition(id),
                segment,
                t,
            },
            asset: AssetPath::new("door.webp").unwrap(),
        }
    }

    #[test]
    fn partitions_get_ids_and_hold_doors() {
        let mut map = map();
        map.add_room(room(0, 0, 12, 8), &floor()).unwrap();
        let wall = map.add_partition(partition(&[(6, 0), (6, 8)])).unwrap();
        assert_eq!(wall, PartitionId(0));
        assert_eq!(map.partition(wall), Some(&partition(&[(6, 0), (6, 8)])));
        let door = map.add_door(door_in_partition(wall, 0, 0.5)).unwrap();
        assert_eq!(map.door_position(door), Some(CellPosition::new(6.0, 4.0)));
        assert_eq!(map.partitions().count(), 1);
    }

    #[test]
    fn partition_doors_need_an_existing_wall_and_segment() {
        let mut map = map();
        let wall = map.add_partition(partition(&[(6, 0), (6, 8)])).unwrap();
        assert_eq!(
            map.add_door(door_in_partition(PartitionId(9), 0, 0.5)),
            Err(MapError::UnknownPartition(PartitionId(9)))
        );
        assert_eq!(
            map.add_door(door_in_partition(wall, 1, 0.5)),
            Err(MapError::UnknownSegment {
                segment: 1,
                count: 1
            })
        );
    }

    #[test]
    fn rejects_partitions_beyond_the_border() {
        assert_eq!(
            map().add_partition(partition(&[(30, 5), (36, 5)])),
            Err(MapError::PartitionOutsideGrid {
                point: CellPoint::new(36, 5)
            })
        );
    }

    #[test]
    fn merging_rooms_keeps_partition_doors() {
        let mut map = map();
        map.add_room(room(0, 0, 12, 8), &floor()).unwrap();
        let wall = map.add_partition(partition(&[(6, 0), (6, 8)])).unwrap();
        let door = map.add_door(door_in_partition(wall, 0, 0.5)).unwrap();
        let added = map.add_room(room(12, 0, 4, 4), &floor()).unwrap();
        assert!(added.removed_doors.is_empty());
        assert_eq!(map.door_position(door), Some(CellPosition::new(6.0, 4.0)));
    }

    fn light_at(x: f64, y: f64) -> Light {
        Light {
            asset: AssetPath::new("light.webp").unwrap(),
            center: CellPosition::new(x, y),
            glow: Glow::new(4.0, 1.0, 0xffec_cd8b).unwrap(),
        }
    }

    #[test]
    fn lights_get_ids_inside_the_map() {
        let mut map = map();
        assert_eq!(map.add_light(light_at(3.5, 4.5)), Ok(LightId(0)));
        assert_eq!(map.add_light(light_at(9.0, 9.0)), Ok(LightId(1)));
        assert_eq!(map.lights().count(), 2);
        assert_eq!(
            map.add_light(light_at(40.0, 1.0)),
            Err(MapError::LightOutsideGrid {
                center: CellPosition::new(40.0, 1.0)
            })
        );
    }

    fn grate(x: i32, y: i32) -> Pattern {
        Pattern {
            asset: AssetPath::new("grate.webp").unwrap(),
            area: Polygon::rectangle(CellPoint::new(x, y), 1, 1).unwrap(),
            tint: 0xffff_ffff,
        }
    }

    #[test]
    fn patterns_get_ids_inside_the_map() {
        let mut map = map();
        assert_eq!(map.add_pattern(grate(2, 2)), Ok(PatternId(0)));
        assert_eq!(map.add_pattern(grate(5, 2)), Ok(PatternId(1)));
        assert_eq!(map.patterns().count(), 2);
        assert_eq!(
            map.add_pattern(grate(35, 2)),
            Err(MapError::PatternOutsideGrid {
                vertex: CellPoint::new(36, 3)
            })
        );
    }

    #[test]
    fn walls_can_be_added_without_painting_the_floor() {
        let mut map = map();
        map.add_walls(room(1, 1, 3, 2)).unwrap();
        assert_eq!(map.rooms().count(), 1);
        assert_eq!(map.floor().count(), 0);
        let tileset = floor();
        map.paint_floor(Cell::new(2, 1), &tileset);
        assert_eq!(
            map.floor().collect::<Vec<_>>(),
            [(Cell::new(2, 1), &tileset)]
        );
    }

    #[test]
    fn rejects_rooms_beyond_the_border() {
        assert_eq!(
            map().add_room(room(30, 5, 6, 2), &floor()),
            Err(MapError::RoomOutsideGrid {
                vertex: CellPoint::new(36, 7)
            })
        );
    }

    #[test]
    fn places_the_door_of_the_reference_map() {
        let mut map = map();
        let hall = map.add_room(room(13, 5, 17, 11), &floor()).unwrap().id;
        let placed = door(hall, WallLoop::Outline, 1, 0.772_727);
        let id = map.add_door(placed.clone()).unwrap();
        assert_eq!(map.doors().collect::<Vec<_>>(), [(id, &placed)]);
    }

    #[test]
    fn doors_get_distinct_ids() {
        let mut map = map();
        let hall = map.add_room(room(13, 5, 17, 11), &floor()).unwrap().id;
        let first: DoorId = map.add_door(door(hall, WallLoop::Outline, 0, 0.5)).unwrap();
        let second = map.add_door(door(hall, WallLoop::Outline, 2, 0.5)).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn doors_need_an_existing_room_wall_and_segment() {
        let mut map = map();
        let hall = map.add_room(room(13, 5, 17, 11), &floor()).unwrap().id;
        let missing_room = RoomId(99);
        assert_eq!(
            map.add_door(door(missing_room, WallLoop::Outline, 0, 0.5)),
            Err(MapError::UnknownRoom(missing_room))
        );
        assert_eq!(
            map.add_door(door(hall, WallLoop::Pillar(0), 0, 0.5)),
            Err(MapError::UnknownWall {
                room: hall,
                wall: WallLoop::Pillar(0)
            })
        );
        assert_eq!(
            map.add_door(door(hall, WallLoop::Outline, 4, 0.5)),
            Err(MapError::UnknownSegment {
                segment: 4,
                count: 4
            })
        );
    }

    #[test]
    fn doors_sit_strictly_inside_their_segment() {
        let mut map = map();
        let hall = map.add_room(room(13, 5, 17, 11), &floor()).unwrap().id;
        for t in [0.0, 1.0, -0.2, 1.5, f64::NAN] {
            let error = map
                .add_door(door(hall, WallLoop::Outline, 0, t))
                .unwrap_err();
            assert!(
                matches!(error, MapError::PositionOffSegment { .. }),
                "{t}: {error:?}"
            );
        }
        assert_eq!(map.doors().count(), 0);
    }
}
