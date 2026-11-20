//! The chance of `scatter`, of weaving and of the floors made: drawn from a
//! seed (SplitMix64), so a place is built the same each time.

/// Numbers drawn one after the other from a seed.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// From the words of a seed (`seed`, a line's number...).
    pub fn new(words: &[u64]) -> Rng {
        let mut rng = Rng { state: 0x9e37_79b9_7f4a_7c15 };
        for word in words {
            rng.state ^= *word;
            rng.next_u64();
        }
        rng
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    /// In [0, 1).
    pub fn random(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// So many in [0, 1).
    pub fn randoms(&mut self, count: usize) -> Vec<f64> {
        (0..count).map(|_| self.random()).collect()
    }

    /// In [0, n); 0 if n is.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }

    /// In [low, high); low if there is nothing between.
    pub fn integers(&mut self, low: i64, high: i64) -> i64 {
        low + self.below((high - low).max(0) as usize) as i64
    }

    /// In [low, high], both in.
    pub fn between(&mut self, low: usize, high: usize) -> usize {
        low + self.below(high - low + 1)
    }

    pub fn choice<'a, T>(&mut self, among: &'a [T]) -> &'a T {
        &among[self.below(among.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_draws_the_same() {
        let drawn = |words: &[u64]| {
            let mut rng = Rng::new(words);
            (rng.randoms(4), rng.integers(3, 9))
        };
        assert_eq!(drawn(&[7, 2]), drawn(&[7, 2]));
        assert_ne!(drawn(&[7, 2]), drawn(&[7, 3]));
        let (randoms, integer) = drawn(&[1]);
        assert!(randoms.iter().all(|value| (0.0..1.0).contains(value)) && (3..9).contains(&integer));
        assert_eq!(Rng::new(&[1]).integers(4, 4), 4);
    }
}
