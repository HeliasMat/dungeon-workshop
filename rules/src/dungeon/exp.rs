//! Experience and level-ups.
//!
//! A defeated monster is worth its species' experience yield, growing with
//! its level; the team shares in it by how the monster was beaten.

use std::sync::Arc;

use super::combat::Stats;

const MAX_LEVEL: i32 = 100;
const MAX_HP: i32 = 999;
const MAX_STAT: i32 = 255;

/// What reaching a level takes and gives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LevelGain {
    /// Total experience needed to be at this level.
    pub total_exp: i32,
    pub hp: i32,
    pub attack: i32,
    pub sp_attack: i32,
    pub defense: i32,
    pub sp_defense: i32,
}

/// What a species of this experience yield is worth at a level: its yield
/// at level 1, an eighth more for each level above.
pub fn exp_value(exp_yield: i32, level: i32) -> i32 {
    exp_yield * (level.max(1) + 7) / 8
}

/// What each team member earns for a defeated monster: two thirds of its
/// value if it only took regular attacks, a third more if a linked move
/// struck it; never nothing.
// (The boosts of items and IQ skills come after: `Run::foe_falls`.)
pub fn defeat_exp(value: i32, hit_by_move: bool, hit_by_link: bool) -> i32 {
    let earned = if hit_by_move { value } else { value * 2 / 3 };
    let earned = if hit_by_link { earned * 4 / 3 } else { earned };
    earned.max(1)
}

/// The experience of a team member.
#[derive(Clone, Debug, Default)]
pub struct Growth {
    pub points: i32,
    /// One entry per level, level 1 first; empty without a ROM, and nobody
    /// levels up then.
    table: Arc<[LevelGain]>,
}

impl Growth {
    /// A monster that has just reached `level`.
    pub fn new(table: Arc<[LevelGain]>, level: i32) -> Self {
        let points = table.get((level - 1).max(0) as usize).map_or(0, |entry| entry.total_exp);
        Self { points, table }
    }

    /// Experience still missing for the next level, if there is one.
    pub(crate) fn to_next(&self, level: i32) -> Option<i32> {
        let next = self.table.get(level.max(1) as usize)?;
        (level < MAX_LEVEL).then_some((next.total_exp - self.points).max(0))
    }

    /// Adds experience and applies the level-ups it brings: each raises the
    /// stats, and current HP by as much as maximum HP. Returns the levels
    /// reached.
    pub fn gain(&mut self, stats: &mut Stats, amount: i32) -> Vec<i32> {
        self.points += amount;
        let mut reached = Vec::new();
        while stats.level < MAX_LEVEL
            && let Some(next) = self.table.get(stats.level as usize)
            && self.points >= next.total_exp
        {
            stats.level += 1;
            let hp = (stats.max_hp + next.hp).min(MAX_HP) - stats.max_hp;
            stats.max_hp += hp;
            stats.hp += hp;
            stats.attack = (stats.attack + next.attack).min(MAX_STAT);
            stats.sp_attack = (stats.sp_attack + next.sp_attack).min(MAX_STAT);
            stats.defense = (stats.defense + next.defense).min(MAX_STAT);
            stats.sp_defense = (stats.sp_defense + next.sp_defense).min(MAX_STAT);
            reached.push(stats.level);
        }
        reached
    }

    /// Goes back down a level (a Doom Seed): the stats lose what that
    /// level gave, the experience is the level's start. False at level 1.
    pub(crate) fn lose_level(&mut self, stats: &mut Stats) -> bool {
        let (Some(was), Some(below)) = (self.table.get((stats.level - 1).max(0) as usize), self.table.get((stats.level - 2).max(0) as usize)) else {
            return false;
        };
        if stats.level <= 1 {
            return false;
        }
        stats.level -= 1;
        stats.max_hp = (stats.max_hp - was.hp).max(1);
        stats.hp = stats.hp.min(stats.max_hp);
        stats.attack = (stats.attack - was.attack).max(1);
        stats.sp_attack = (stats.sp_attack - was.sp_attack).max(1);
        stats.defense = (stats.defense - was.defense).max(1);
        stats.sp_defense = (stats.sp_defense - was.sp_defense).max(1);
        self.points = below.total_exp;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_lost_takes_back_what_it_gave() {
        let mut growth = Growth::new(table(), 5);
        let mut stats = Stats { max_hp: 40, hp: 40, attack: 20, sp_attack: 20, defense: 20, sp_defense: 20, ..stats(5) };
        let before = stats;
        growth.gain(&mut stats, growth.to_next(5).unwrap());
        assert!(stats.level == 6 && growth.lose_level(&mut stats));
        assert_eq!((stats.level, stats.max_hp, stats.attack, stats.defense), (5, before.max_hp, before.attack, before.defense));
        let mut lowest = Stats { level: 1, ..stats };
        assert!(!Growth::new(table(), 1).lose_level(&mut lowest));
    }

    fn table() -> Arc<[LevelGain]> {
        (0..100).map(|level| LevelGain { total_exp: level * level * 10, hp: 3, attack: 2, sp_attack: 1, defense: 1, sp_defense: 0 }).collect()
    }

    fn stats(level: i32) -> Stats {
        Stats {
            level,
            max_hp: 30,
            hp: 12,
            attack: 10,
            defense: 10,
            sp_attack: 10,
            sp_defense: 10,
            types: (1, 0),
            abilities: [0, 0],
            held: 0,
            iq: 1,
            iq_group: 0,
            iq_skills: 0,
            female: false,
        }
    }

    #[test]
    fn exp_grows_with_level_and_with_moves() {
        assert_eq!(exp_value(40, 1), 40);
        assert_eq!(exp_value(40, 9), 80);
        assert_eq!(exp_value(32, 12), 76);
        assert_eq!(exp_value(40, 0), 40);
        // Two thirds without a move, and at least 1.
        assert_eq!(defeat_exp(90, true, false), 90);
        assert_eq!(defeat_exp(90, false, false), 60);
        assert_eq!(defeat_exp(1, false, false), 1);
        // A third more with linked moves.
        assert_eq!(defeat_exp(60, true, true), 80);
    }

    #[test]
    fn levels_come_with_their_stats() {
        let mut stats = stats(5);
        let mut growth = Growth::new(table(), 5);
        // Level 5 starts at 160 points; level 6 takes 250.
        assert_eq!((growth.points, growth.to_next(5)), (160, Some(90)));
        assert!(growth.gain(&mut stats, 89).is_empty());
        assert_eq!(growth.gain(&mut stats, 1), vec![6]);
        assert_eq!((stats.level, stats.max_hp, stats.hp, stats.attack), (6, 33, 15, 12));
        // Enough for several levels at once: 7 (360) and 8 (490).
        assert_eq!(growth.gain(&mut stats, 250), vec![7, 8]);
        assert_eq!((stats.max_hp, stats.hp, stats.sp_defense), (39, 21, 10));
    }

    #[test]
    fn level_100_is_the_last() {
        let mut stats = stats(99);
        let mut growth = Growth::new(table(), 99);
        assert_eq!(growth.gain(&mut stats, 1_000_000), vec![100]);
        assert_eq!(growth.to_next(100), None);
        assert!(growth.gain(&mut stats, 1_000_000).is_empty());
        // Without a table nothing happens.
        let mut blank = Growth::default();
        assert!(blank.gain(&mut stats, 500).is_empty());
        assert_eq!(blank.to_next(5), None);
    }
}
