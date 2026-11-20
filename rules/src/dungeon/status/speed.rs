//! How fast a monster goes.
//!
//! Speed is a stage from 0 (half speed) to 4 (four actions a turn), 1 being
//! normal. Each boost and each drop is a counter of its own that runs out
//! by itself; a boost first cancels a running drop, and a drop a running
//! boost, so the two never pile up against each other.

use super::*;

/// How many of these counters are running.
fn running(counters: &[i32]) -> usize {
    counters.iter().filter(|&&turns| turns > 0).count()
}

/// Stops the running counter with the fewest turns left, if one runs.
fn cancel_one(counters: &mut [i32]) -> bool {
    match counters.iter_mut().filter(|turns| **turns > 0).min_by_key(|turns| **turns) {
        Some(turns) => {
            *turns = 0;
            true
        }
        None => false,
    }
}

/// Starts an idle counter for that many turns, if one is idle.
fn start_one(counters: &mut [i32], turns: i32) {
    if let Some(idle) = counters.iter_mut().find(|turns| **turns <= 0) {
        *idle = turns;
    }
}

impl Statuses {
    /// [`Self::speed_stage`] as the floor's weather and the monster's
    /// abilities make it: Ice types are quicker in snow, Quick Feet with a
    /// status problem, and the abilities that strike twice a turn go at
    /// least at that pace.
    pub(crate) fn speed_stage_in(&self, stats: &Stats, weather: u8) -> i32 {
        use crate::dungeon::ability;
        const TYPE_ICE: u8 = 6;
        let snowy_ice = weather == weather::SNOW && (stats.types.0 == TYPE_ICE || stats.types.1 == TYPE_ICE);
        let quick_feet = ability::has(stats, ability::QUICK_FEET) && self.ailing();
        let stage = (self.raw_speed_stage() + snowy_ice as i32 + quick_feet as i32).clamp(0, MAX_SPEED_STAGE);
        if ability::attacks_twice(stats, weather) { stage.max(2) } else { stage }
    }

    pub(crate) fn speed_stage(&self) -> i32 {
        self.raw_speed_stage().clamp(0, MAX_SPEED_STAGE)
    }

    /// How many speed boosts and how many speed drops are running.
    pub(crate) fn speed_changes(&self) -> (usize, usize) {
        (running(&self.speed_up), running(&self.speed_down))
    }

    /// Normal speed, with each boost and drop running; paralysis costs a
    /// stage too.
    pub(super) fn raw_speed_stage(&self) -> i32 {
        let (up, down) = self.speed_changes();
        1 + up as i32 - down as i32 - (self.paralysis_turns > 0) as i32
    }

    /// One stage faster, for a while. Returns whether the speed changed.
    pub(crate) fn boost_speed(&mut self, rng: &mut DungeonRng, constants: &Constants) -> bool {
        let before = self.speed_stage();
        if before >= MAX_SPEED_STAGE {
            return false;
        }
        if !cancel_one(&mut self.speed_down) {
            start_one(&mut self.speed_up, Self::duration(rng, constants.speed_boost_turns));
        }
        self.speed_stage() != before
    }

    /// One stage slower, for a while. Returns whether the speed changed.
    pub(crate) fn lower_speed(&mut self, rng: &mut DungeonRng, constants: &Constants) -> bool {
        let before = self.speed_stage();
        if before <= 0 {
            return false;
        }
        if !cancel_one(&mut self.speed_up) {
            start_one(&mut self.speed_down, Self::duration(rng, constants.speed_lower_turns));
        }
        self.speed_stage() != before
    }

    /// A boost the monster's own body gives (Speed Boost): it lasts the
    /// longest a boost can, with no roll.
    pub(super) fn natural_speed_boost(&mut self, constants: &Constants) {
        if self.speed_stage() < MAX_SPEED_STAGE && !cancel_one(&mut self.speed_down) {
            start_one(&mut self.speed_up, constants.speed_boost_turns.1.max(1));
        }
    }
}
