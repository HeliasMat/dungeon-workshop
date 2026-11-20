//! What a change to a monster shows: a rule changes it inside `watched`
//! and the numbers over it, the effects its statuses play and the jingle of
//! a level come out as sights.

use crate::dungeon::Statuses;

use super::monster::MonsterId;
use super::{Monster, Run, Sight};

/// The jingle of a level gained (the team's: foes do not grow).
const LEVEL_UP_JINGLE: u16 = 7;

/// What a monster's statuses were, to see them change.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Looks {
    offence: (i32, i32),
    defence: (i32, i32),
    aim: (i32, i32),
    speed: (usize, usize),
    paralyzed: bool,
    confused: bool,
}

impl Looks {
    fn of(status: &Statuses) -> Self {
        Self {
            offence: (status.attack_stage, status.sp_attack_stage),
            defence: (status.defense_stage, status.sp_defense_stage),
            aim: (status.accuracy_stage, status.evasion_stage),
            speed: status.speed_changes(),
            paralyzed: status.paralysis_turns > 0,
            confused: status.confused(),
        }
    }

    /// The effects played on such a change: stats up or down (arrows, a
    /// sound by stat), speed, paralysis, confusion, by their numbers in the
    /// effect data; where there are two, both.
    fn effects(self, now: Self) -> &'static [u16] {
        let rose = |a: (i32, i32), b: (i32, i32)| b.0 > a.0 || b.1 > a.1;
        if rose(self.offence, now.offence) {
            &[425, 2]
        } else if rose(now.offence, self.offence) {
            &[404, 403]
        } else if rose(self.defence, now.defence) {
            &[2, 400]
        } else if rose(now.defence, self.defence) {
            &[399, 401]
        } else if rose(self.aim, now.aim) {
            &[396, 13]
        } else if rose(now.aim, self.aim) {
            &[397, 14]
        } else if now.speed.0 > self.speed.0 {
            &[395]
        } else if now.speed.1 > self.speed.1 {
            &[2]
        } else if now.paralyzed && !self.paralyzed {
            &[423]
        } else if now.confused && !self.confused {
            // (A question mark.)
            &[324]
        } else {
            &[]
        }
    }
}

/// A monster before a change.
pub(crate) struct Watch {
    hp: i32,
    level: i32,
    looks: Looks,
}

impl Watch {
    pub(crate) fn of(monster: &Monster) -> Self {
        Self { hp: monster.stats.hp.max(0), level: monster.stats.level, looks: Looks::of(&monster.status) }
    }

    /// What it became shows: all a change did, as one number.
    pub(crate) fn show(self, monster: &Monster, sights: &mut Vec<Sight>) {
        let lost = self.hp - monster.stats.hp.max(0);
        if lost != 0 {
            sights.push(Sight::Number(monster.id, lost));
        }
        if monster.stats.level > self.level {
            sights.push(Sight::Jingle(LEVEL_UP_JINGLE));
        }
        if monster.alive() {
            let effects = self.looks.effects(Looks::of(&monster.status));
            sights.extend(effects.iter().map(|&effect| Sight::Effect(effect, monster.at)));
        }
    }
}

/// Changes a monster, what it does to it being shown.
pub(crate) fn watched<R>(monster: &mut Monster, sights: &mut Vec<Sight>, change: impl FnOnce(&mut Monster) -> R) -> R {
    let watch = Watch::of(monster);
    let result = change(monster);
    watch.show(monster, sights);
    result
}

impl Run {
    /// Changes a monster on the floor, what it does to it being shown. The
    /// way for any rule to touch HP, stages or levels. `None`: no such
    /// monster.
    pub(crate) fn change<R>(&mut self, who: MonsterId, change: impl FnOnce(&mut Monster) -> R) -> Option<R> {
        let monster = self.dungeon.as_mut()?.monster_mut(who)?;
        Some(watched(monster, &mut self.sights, change))
    }

    /// A monster loses HP.
    pub(crate) fn hurt(&mut self, who: MonsterId, amount: i32) {
        self.change(who, |monster| monster.stats.hp -= amount);
    }
}
