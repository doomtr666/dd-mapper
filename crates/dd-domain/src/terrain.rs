use crate::{AssetPath, CellPosition, GridSize};

/// Samples per cell side.
const SAMPLES_PER_CELL: u32 = 4;
/// Dungeondraft blends up to four terrain textures.
pub const TERRAIN_SLOTS: usize = 4;
/// Weights of a sample add up to this.
const FULL: u8 = 255;

/// The ground around and under rooms: up to four textures blended with
/// weights sampled every quarter cell. Sample `(column, row)` stands at the
/// center of its quarter cell. The first texture is the background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terrain {
    columns: u32,
    rows: u32,
    textures: [Option<AssetPath>; TERRAIN_SLOTS],
    weights: Vec<[u8; TERRAIN_SLOTS]>,
}

impl Terrain {
    /// The first texture everywhere, none chosen yet.
    #[must_use]
    pub fn new(grid: GridSize) -> Self {
        let columns = grid.width() * SAMPLES_PER_CELL;
        let rows = grid.height() * SAMPLES_PER_CELL;
        Self {
            columns,
            rows,
            textures: Default::default(),
            weights: vec![[FULL, 0, 0, 0]; (columns * rows) as usize],
        }
    }

    #[must_use]
    pub fn columns(&self) -> u32 {
        self.columns
    }

    #[must_use]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Where sample `(column, row)` stands, in cells.
    #[must_use]
    pub fn position(column: u32, row: u32) -> CellPosition {
        let cells = |sample: u32| (f64::from(sample) + 0.5) / f64::from(SAMPLES_PER_CELL);
        CellPosition::new(cells(column), cells(row))
    }

    #[must_use]
    pub fn texture(&self, slot: usize) -> Option<&AssetPath> {
        self.textures.get(slot)?.as_ref()
    }

    pub fn set_texture(&mut self, slot: usize, texture: AssetPath) {
        if let Some(chosen) = self.textures.get_mut(slot) {
            *chosen = Some(texture);
        }
    }

    /// The slot holding `texture`, or else the first free one after the background.
    #[must_use]
    pub fn slot_for(&self, texture: &AssetPath) -> Option<usize> {
        let holding = self
            .textures
            .iter()
            .position(|chosen| chosen.as_ref() == Some(texture));
        holding.or_else(|| (1..TERRAIN_SLOTS).find(|&slot| self.textures[slot].is_none()))
    }

    /// The chosen textures that show somewhere.
    pub fn used_textures(&self) -> impl Iterator<Item = &AssetPath> {
        self.textures
            .iter()
            .enumerate()
            .filter(|(slot, _)| self.weights.iter().any(|weights| weights[*slot] > 0))
            .filter_map(|(_, texture)| texture.as_ref())
    }

    #[must_use]
    pub fn weights(&self, column: u32, row: u32) -> [u8; TERRAIN_SLOTS] {
        self.index(column, row)
            .map_or([FULL, 0, 0, 0], |index| self.weights[index])
    }

    /// Sets one sample, as read from a file.
    pub fn set_weights(&mut self, column: u32, row: u32, weights: [u8; TERRAIN_SLOTS]) {
        if let Some(index) = self.index(column, row) {
            self.weights[index] = weights;
        }
    }

    /// Blends `slot` in by `strength(position)`, from 0 (unchanged) to 1 (only
    /// that texture); returns how many samples changed.
    pub fn paint(&mut self, slot: usize, strength: impl Fn(CellPosition) -> f64) -> usize {
        if slot >= TERRAIN_SLOTS {
            return 0;
        }
        let mut changed = 0;
        for row in 0..self.rows {
            for column in 0..self.columns {
                let amount = strength(Self::position(column, row)).clamp(0.0, 1.0);
                if amount <= 0.0 {
                    continue;
                }
                let Some(index) = self.index(column, row) else {
                    continue;
                };
                let blended = blend(self.weights[index], slot, amount);
                if blended != self.weights[index] {
                    self.weights[index] = blended;
                    changed += 1;
                }
            }
        }
        changed
    }

    fn index(&self, column: u32, row: u32) -> Option<usize> {
        (column < self.columns && row < self.rows).then(|| (row * self.columns + column) as usize)
    }
}

/// `weights` moved toward `slot` alone by `amount`, still adding up to 255.
fn blend(weights: [u8; TERRAIN_SLOTS], slot: usize, amount: f64) -> [u8; TERRAIN_SLOTS] {
    let mut blended = [0_u8; TERRAIN_SLOTS];
    for (index, weight) in weights.iter().enumerate() {
        if index != slot {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a weight scaled down stays within 0 to 255"
            )]
            let kept = (f64::from(*weight) * (1.0 - amount)).round() as u8;
            blended[index] = kept;
        }
    }
    let others: u32 = blended.iter().map(|weight| u32::from(*weight)).sum();
    blended[slot] = u8::try_from(u32::from(FULL).saturating_sub(others)).unwrap_or(FULL);
    blended
}

#[cfg(test)]
mod tests {
    use super::Terrain;
    use crate::{AssetPath, CellPosition, GridSize};

    fn terrain() -> Terrain {
        Terrain::new(GridSize::new(10, 8).unwrap())
    }

    fn asset(name: &str) -> AssetPath {
        AssetPath::new(name).unwrap()
    }

    #[test]
    fn samples_the_center_of_every_quarter_cell() {
        let ground = terrain();
        assert_eq!((ground.columns(), ground.rows()), (40, 32));
        assert_eq!(Terrain::position(0, 0), CellPosition::new(0.125, 0.125));
        assert_eq!(Terrain::position(4, 2), CellPosition::new(1.125, 0.625));
        assert_eq!(ground.weights(3, 3), [255, 0, 0, 0]);
    }

    #[test]
    fn hands_out_free_slots_after_the_background() {
        let mut ground = terrain();
        assert_eq!(ground.slot_for(&asset("ash.png")), Some(1));
        ground.set_texture(1, asset("ash.png"));
        assert_eq!(ground.slot_for(&asset("ash.png")), Some(1));
        ground.set_texture(2, asset("slag.png"));
        ground.set_texture(3, asset("rock.png"));
        assert_eq!(ground.slot_for(&asset("sand.png")), None);
        assert_eq!(ground.texture(2), Some(&asset("slag.png")));
    }

    #[test]
    fn only_textures_showing_somewhere_are_used() {
        let mut ground = terrain();
        ground.set_texture(0, asset("rock.png"));
        ground.set_texture(1, asset("ash.png"));
        ground.set_texture(2, asset("slag.png"));
        ground.paint(2, |position| if position.x < 2.0 { 0.6 } else { 0.0 });
        let used: Vec<&str> = ground.used_textures().map(AssetPath::as_str).collect();
        assert_eq!(used, ["rock.png", "slag.png"]);
    }

    #[test]
    fn paints_by_strength_keeping_weights_whole() {
        let mut ground = terrain();
        let changed = ground.paint(2, |position| if position.x < 1.0 { 1.0 } else { 0.5 });
        assert_eq!(changed, 40 * 32);
        assert_eq!(ground.weights(0, 0), [0, 0, 255, 0]);
        let half = ground.weights(20, 10);
        assert_eq!(
            half.iter().map(|weight| u32::from(*weight)).sum::<u32>(),
            255
        );
        assert!((127..=128).contains(&half[2]), "{half:?}");
        assert_eq!(ground.paint(2, |_| 0.0), 0);
    }
}
