//! Exclusive items: kept in the bag, they work for the monsters they are
//! made for (`IsExclusiveItemIdForMonster`), raising their stats and giving
//! them an effect (`enum exclusive_item_effect_id`).

use serde::{Deserialize, Serialize};

/// The first exclusive item (the Prism Ruff), and one past the last.
pub const FIRST: u16 = 444;
pub const END: u16 = 1400;

/// The effects this engine knows (pmdsky-debug's `EXCLUSIVE_EFF_*`).
pub mod effect {
    pub(crate) const NO_PARALYSIS: u8 = 1;
    pub(crate) const NO_CONFUSION: u8 = 2;
    pub(crate) const NO_INFATUATION: u8 = 3;
    pub(crate) const NO_FREEZE: u8 = 4;
    pub(crate) const NO_CRITICAL_HITS: u8 = 5;
    pub(crate) const HALVED_EXPLOSION_DAMAGE: u8 = 6;
    pub(crate) const NO_EXPLOSION_DAMAGE: u8 = 7;
    pub(crate) const NO_WEATHER_DAMAGE: u8 = 9;
    /// Struck, it may poison, paralyse or put to sleep the striker.
    pub(crate) const MAY_POISON_PARALYZE_SLEEP_ATTACKERS: u8 = 11;
    pub(crate) const NO_SLEEP: u8 = 10;
    pub(crate) const HALVED_PHYSICAL_DAMAGE: u8 = 30;
    pub(crate) const HALVED_SPECIAL_DAMAGE: u8 = 31;
    pub(crate) const COUNTER_PHYSICAL_DAMAGE: u8 = 32;
    pub(crate) const COUNTER_25_PCT_PHYSICAL_DAMAGE: u8 = 35;
    pub(crate) const LONG_TOSS: u8 = 36;
    pub(crate) const NO_STAT_DROPS: u8 = 38;
    pub(crate) const RESTORE_PP_ON_NEW_FLOORS: u8 = 71;
    pub(crate) const RESTORE_HP_ON_NEW_FLOORS: u8 = 72;
    pub(crate) const EXP_BOOST: u8 = 79;
    pub(crate) const NO_STATUS: u8 = 89;
    pub(crate) const HALVED_DAMAGE: u8 = 90;
    pub(crate) const DAMAGE_BOOST_50_PCT: u8 = 91;
    pub(crate) const NO_FRIENDLY_FIRE: u8 = 96;
    /// No damage from moves of a type: Fire (102) to Steel (113), in
    /// `NO_DAMAGE_TYPES`' order; absorbed, healing: Fire (114) to Steel
    /// (128), in `ABSORB_TYPES`' order.
    pub(crate) const NO_FIRE_DAMAGE: u8 = 102;
    pub(crate) const ABSORB_FIRE_DAMAGE: u8 = 114;
    pub(crate) const LAST: u8 = 129;
}

/// What the effects that answer a blow do (the game's table at
/// EU 0x23541DC, each at the chance [`crate::dungeon::status::Constants`]
/// `exclusive_reaction_chance`): to the striker, or to the one struck.
pub(crate) fn reactions() -> [(u8, crate::dungeon::status::MoveEffect, bool); 14] {
    use crate::dungeon::status::MoveEffect::*;
    [
        (13, Sleep, false),
        (14, Nightmare, false),
        (15, Burn, false),
        (16, Paralyze, false),
        (17, Confuse, false),
        (18, Infatuate, false),
        (19, Freeze, false),
        (20, Immobilize, false),
        (21, Constrict, false),
        (22, Cringe, false),
        (25, Invisify, true),
        (26, UserSpeedUp, true),
        (28, PerishSong, false),
        (29, TargetSpeedDown, false),
    ]
}

/// The types of `NO_*_DAMAGE` and of `ABSORB_*_DAMAGE`, in their order
/// (`enum type_id`).
const NO_DAMAGE_TYPES: [u8; 12] = [2, 3, 4, 5, 7, 9, 10, 11, 14, 15, 16, 17];
const ABSORB_TYPES: [u8; 15] = [2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17];

/// The game's tables of exclusive items, from the first on.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExclusiveItems {
    /// Who each is for (`item_s_p.bin`): its kind (1 to 4: a type; 9, 10:
    /// a species; otherwise a family) and that type, species or family.
    pub kinds: Vec<(u8, i16)>,
    /// Its effect and its stat boosts' row (`EXCLUSIVE_ITEM_EFFECT_DATA`).
    pub effects: Vec<(u8, u8)>,
    /// Attack, defense, special attack, special defense raised, by row
    /// (`EXCLUSIVE_ITEM_STAT_BOOST_DATA`).
    pub boosts: Vec<[i8; 4]>,
}

impl ExclusiveItems {
    fn entry(&self, item: u16) -> Option<usize> {
        (FIRST..END).contains(&item).then(|| (item - FIRST) as usize).filter(|&at| at < self.kinds.len())
    }

    /// Whether the item works for this monster (its species, family and
    /// types).
    pub fn for_monster(&self, item: u16, species: u16, family: u16, types: (u8, u8)) -> bool {
        // (Of the second half of the monster table, the first's: the
        // other gender.)
        const FORMS: u16 = 600;
        let species = if species >= FORMS { species - FORMS } else { species };
        let Some(&(kind, param)) = self.entry(item).and_then(|at| self.kinds.get(at)) else {
            return false;
        };
        match kind {
            0 => false,
            1..=4 => param as u8 == types.0 || param as u8 == types.1,
            9 | 10 => param as u16 == species,
            _ => param as u16 == family,
        }
    }

    /// What the item does, and what it raises (attack, defense, special
    /// attack, special defense).
    pub fn effect(&self, item: u16) -> Option<(u8, [i32; 4])> {
        let &(effect, row) = self.entry(item).and_then(|at| self.effects.get(at))?;
        let boosts = self.boosts.get(row as usize).map_or([0; 4], |row| row.map(i32::from));
        Some((effect, boosts))
    }
}

/// The effects of the exclusive items a monster has the use of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Exclusive {
    pub effects: [u64; 3],
    /// Attack, defense, special attack, special defense raised.
    pub boosts: [i32; 4],
}

impl Exclusive {
    /// Of the items in the bag, those for this monster.
    pub fn of(items: &ExclusiveItems, bag: impl IntoIterator<Item = u16>, species: u16, family: u16, types: (u8, u8)) -> Self {
        let mut exclusive = Self::default();
        for item in bag {
            if !items.for_monster(item, species, family, types) {
                continue;
            }
            if let Some((effect, boosts)) = items.effect(item) {
                if effect != 0 && effect < effect::LAST {
                    exclusive.effects[effect as usize / 64] |= 1 << (effect % 64);
                }
                for (total, boost) in exclusive.boosts.iter_mut().zip(boosts) {
                    *total += boost;
                }
            }
        }
        exclusive
    }

    pub fn has(&self, effect: u8) -> bool {
        self.effects.get(effect as usize / 64).is_some_and(|bits| bits & 1 << (effect % 64) != 0)
    }

    /// Damage of a move of this type it takes: none, or absorbed (healing),
    /// or as it is.
    pub(crate) fn shields_type(&self, kind: u8) -> Option<bool> {
        let none = NO_DAMAGE_TYPES.iter().position(|&t| t == kind).is_some_and(|at| self.has(effect::NO_FIRE_DAMAGE + at as u8));
        let absorbed = ABSORB_TYPES.iter().position(|&t| t == kind).is_some_and(|at| self.has(effect::ABSORB_FIRE_DAMAGE + at as u8));
        match (absorbed, none) {
            (true, _) => Some(true),
            (false, true) => Some(false),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_works_for_its_monster_only() {
        let items = ExclusiveItems {
            kinds: vec![(10, 133), (1, 10)],
            effects: vec![(effect::NO_SLEEP, 1), (effect::ABSORB_FIRE_DAMAGE, 0)],
            boosts: vec![[0; 4], [1, 2, 3, 4]],
        };
        let eevee = Exclusive::of(&items, [FIRST, FIRST + 1], 133, 0, (1, 0));
        assert!(eevee.has(effect::NO_SLEEP) && eevee.boosts == [1, 2, 3, 4] && !eevee.has(effect::ABSORB_FIRE_DAMAGE));
        let fire = Exclusive::of(&items, [FIRST, FIRST + 1], 4, 0, (10, 0));
        assert_eq!(fire.shields_type(2), Some(true));
    }
}
