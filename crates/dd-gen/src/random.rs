//! A small seeded generator (SplitMix64): same seed, same layout, everywhere.

#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
}

impl Random {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    /// A number in `low..=high`; `low` when the range is empty.
    pub fn between(&mut self, low: u32, high: u32) -> u32 {
        if high <= low {
            return low;
        }
        let span = u64::from(high - low) + 1;
        let offset = u32::try_from(self.next_u64() % span).unwrap_or(0);
        low + offset
    }

    pub fn coin(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::Random;

    #[test]
    fn matches_the_splitmix64_reference_sequence() {
        let mut random = Random::new(1_234_567);
        assert_eq!(random.next_u64(), 6_457_827_717_110_365_317);
        assert_eq!(random.next_u64(), 3_203_168_211_198_807_973);
    }

    #[test]
    fn stays_within_bounds() {
        let mut random = Random::new(7);
        for _ in 0..1000 {
            let value = random.between(3, 6);
            assert!((3..=6).contains(&value), "{value}");
        }
        assert_eq!(random.between(5, 2), 5);
    }
}
