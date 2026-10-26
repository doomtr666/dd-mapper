use std::collections::BTreeMap;

use dd_domain::{Cell, GridSize, Map};

use crate::godot::PoolIntArray;
use crate::model::{Color, Tiles};

const EMPTY_CELL: i32 = -1;

/// Writes the map's floor layer as Dungeondraft tiles.
///
/// The template's `lookup` is kept, so existing indices stay valid;
/// tilesets missing from it are appended.
pub fn build_tiles(map: &Map, mut lookup: BTreeMap<u32, String>) -> Tiles {
    let grid = map.grid();
    let cell_count = cell_count(grid);
    let mut cells = vec![EMPTY_CELL; cell_count];
    for (cell, tileset) in map.floor() {
        if let Some(index) = cell_index(grid, cell) {
            cells[index] = tileset_index(&mut lookup, tileset.as_str());
        }
    }
    Tiles {
        cells: PoolIntArray(cells),
        colors: vec![Color::WHITE; cell_count],
        lookup,
    }
}

/// The lookup key of `path`, appended after the last key when missing.
fn tileset_index(lookup: &mut BTreeMap<u32, String>, path: &str) -> i32 {
    let existing = lookup
        .iter()
        .find(|(_, known)| known.as_str() == path)
        .map(|(key, _)| *key);
    let key = existing.unwrap_or_else(|| {
        let next = lookup.keys().next_back().map_or(0, |last| last + 1);
        lookup.insert(next, path.to_owned());
        next
    });
    i32::try_from(key).expect("tileset lookups stay far below 2^31 entries")
}

fn cell_count(grid: GridSize) -> usize {
    to_usize(grid.width()) * to_usize(grid.height())
}

/// Row-major index of `cell`, or `None` outside the grid.
fn cell_index(grid: GridSize, cell: Cell) -> Option<usize> {
    let inside = cell.column < grid.width() && cell.row < grid.height();
    inside.then(|| to_usize(cell.row) * to_usize(grid.width()) + to_usize(cell.column))
}

fn to_usize(value: u32) -> usize {
    usize::try_from(value).expect("map dimensions fit in memory")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dd_domain::{AssetPath, CellPoint, GridSize, Map, Polygon, Room};

    use super::build_tiles;
    use crate::model::Color;

    fn room(x: i32, y: i32) -> Room {
        Room::new(
            Polygon::rectangle(CellPoint::new(x, y), 2, 1).unwrap(),
            AssetPath::new("wall.webp").unwrap(),
        )
    }

    fn tileset(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    fn lookup() -> BTreeMap<u32, String> {
        BTreeMap::from([(0, "grass.webp".to_owned()), (4, "stone.webp".to_owned())])
    }

    #[test]
    fn paints_floor_cells_row_by_row() {
        let mut map = Map::new(GridSize::new(4, 3).unwrap());
        map.add_room(room(1, 1), &tileset("stone.webp")).unwrap();
        let tiles = build_tiles(&map, lookup());
        assert_eq!(
            tiles.cells.0,
            [-1, -1, -1, -1, -1, 4, 4, -1, -1, -1, -1, -1]
        );
        assert_eq!(tiles.colors, vec![Color::WHITE; 12]);
        assert_eq!(tiles.lookup, lookup());
    }

    #[test]
    fn appends_unknown_tilesets_to_the_lookup() {
        let mut map = Map::new(GridSize::new(4, 3).unwrap());
        map.add_room(room(0, 0), &tileset("metal.webp")).unwrap();
        map.add_room(room(2, 2), &tileset("metal.webp")).unwrap();
        let tiles = build_tiles(&map, lookup());
        assert_eq!(tiles.lookup.get(&5).map(String::as_str), Some("metal.webp"));
        assert_eq!(tiles.lookup.len(), 3);
        assert_eq!(tiles.cells.0, [5, 5, -1, -1, -1, -1, -1, -1, -1, -1, 5, 5]);
    }
}
