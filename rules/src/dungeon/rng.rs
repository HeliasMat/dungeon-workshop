//! Dungeon PRNG: the 32-bit LCG used by the original dungeon mode, as
//! documented by the pmdsky-debug project.

const LCG_MULTIPLIER: u32 = 0x5D58_8B65;
const LCG_INCREMENT: u32 = 1;

#[derive(Clone, Debug)]
pub struct DungeonRng {
    state: u32,
}

impl DungeonRng {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Current state; `DungeonRng::new(state)` resumes the same sequence.
    pub fn state(&self) -> u32 {
        self.state
    }

    pub(crate) fn rand16(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
        self.state >> 16
    }

    /// Uniform in `[0, n)`.
    pub fn rand_int(&mut self, n: i32) -> i32 {
        ((self.rand16() as i64 * n as i64) >> 16) as i32
    }

    /// Uniform in `[min(a, b), max(a, b))`; returns `a` when both are equal.
    pub(crate) fn rand_range(&mut self, a: i32, b: i32) -> i32 {
        if a == b {
            a
        } else if a < b {
            self.rand_int(b - a) + a
        } else {
            self.rand_int(a - b) + b
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_bounds() {
        let mut a = DungeonRng::new(1234);
        let mut b = DungeonRng::new(1234);
        for _ in 0..10_000 {
            let v = a.rand_range(3, 11);
            assert_eq!(v, b.rand_range(3, 11));
            assert!((3..11).contains(&v));
        }
    }
}
