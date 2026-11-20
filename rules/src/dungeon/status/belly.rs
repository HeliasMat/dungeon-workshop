//! The belly of who walks.

use super::*;

/// The leader's belly, in 16.16 fixed point: whole points and a fraction.
#[derive(Clone, Copy, Debug)]
pub struct Belly {
    pub(super) value: i32,
    pub(super) max: i32,
}

impl Belly {
    pub fn full() -> Self {
        Self { value: 100 << 16, max: 100 << 16 }
    }

    /// The fixed-point value, for save files.
    pub fn raw(&self) -> i32 {
        self.value
    }

    pub(crate) fn from_raw(value: i32) -> Self {
        let max = Self::full().max;
        Self { value: value.clamp(0, max), max }
    }

    /// Whole points left, rounded up as displayed in game.
    pub fn points(&self) -> i32 {
        (self.value + 0xFFFF) >> 16
    }

    pub fn max_points(&self) -> i32 {
        self.max >> 16
    }

    /// The Hunger Seed's effect.
    pub fn empty(&mut self) {
        self.value = 0;
    }

    pub(crate) fn eat(&mut self, points: i32) {
        self.value = (self.value + (points << 16)).min(self.max);
    }

    /// That much belly gone at once.
    pub(crate) fn lose(&mut self, points: i32) {
        self.value = (self.value - (points << 16)).max(0);
    }

    /// A real meal: that much belly back. What does not fit in the belly
    /// stretches it, by up to `bigger` points (never past 200), and the
    /// belly ends full.
    pub(crate) fn eat_food(&mut self, points: i32, bigger: i32) {
        const LARGEST: i32 = 200 << 16;
        let spare = (self.value + (points << 16) - self.max).max(0);
        let stretch = spare.min(bigger.max(0) << 16);
        self.max = (self.max + stretch).min(LARGEST).max(self.max);
        self.eat(points);
    }

    /// One turn passes: the belly empties, then the monster starves.
    pub(crate) fn end_turn(&mut self, stats: &mut Stats, constants: &Constants) {
        // (What it holds makes it slower or faster.)
        let lost = constants.belly_lost_per_turn * crate::dungeon::held::belly_quarters(stats) / 4;
        // (Energy Saver: one step down the hunger table, three quarters.)
        let lost = match crate::dungeon::iq::has(stats, crate::dungeon::iq::ENERGY_SAVER) {
            true => lost * 3 / 4,
            false => lost,
        };
        self.value = (self.value - lost).max(0);
        if self.value == 0 {
            stats.hp -= 1;
        }
    }
}
