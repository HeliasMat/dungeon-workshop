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
    /// No move of it sealed (the Seal Trap, a sealing item).
    pub(crate) const NO_MOVE_DISABLING: u8 = 8;
    pub(crate) const NO_WEATHER_DAMAGE: u8 = 9;
    /// Struck, it may poison, paralyse or put to sleep the striker.
    pub(crate) const MAY_POISON_PARALYZE_SLEEP_ATTACKERS: u8 = 11;
    pub(crate) const NO_SLEEP: u8 = 10;
    /// Struck, it may blind the striker, seal one of its moves, or (low on
    /// HP) warp away.
    pub(crate) const MAY_BLINKER_ATTACKERS: u8 = 23;
    pub(crate) const MAY_SEAL_ATTACKER_MOVES: u8 = 24;
    pub(crate) const MAY_WARP_WHEN_ATTACKED: u8 = 27;
    pub(crate) const HALVED_PHYSICAL_DAMAGE: u8 = 30;
    pub(crate) const HALVED_SPECIAL_DAMAGE: u8 = 31;
    pub(crate) const COUNTER_PHYSICAL_DAMAGE: u8 = 32;
    /// Now and then a status move used on it goes back, as by Magic Coat.
    pub(crate) const MAY_BOUNCE_STATUS_MOVES: u8 = 33;
    /// Now and then it holds on with 1 HP.
    pub(crate) const MAY_ENDURE: u8 = 34;
    pub(crate) const COUNTER_25_PCT_PHYSICAL_DAMAGE: u8 = 35;
    pub(crate) const LONG_TOSS: u8 = 36;
    /// Now and then a move used on it goes back, as by Mirror Move.
    pub(crate) const MAY_BOUNCE_MOVES: u8 = 37;
    pub(crate) const NO_STAT_DROPS: u8 = 38;
    /// Hit by a move, it takes a type that resists it, as by Conversion 2.
    pub(crate) const CONVERSION_2_WHEN_HIT: u8 = 39;
    /// Its blows go through Reflect and Light Screen (and the items that
    /// halve physical or special damage).
    pub(crate) const BYPASS_REFLECT_LIGHT_SCREEN: u8 = 68;
    /// Its Normal and Fighting moves hit Ghosts; its Psychic moves Darks.
    pub(crate) const SCRAPPY: u8 = 69;
    pub(crate) const MIRACLE_EYE: u8 = 70;
    pub(crate) const RESTORE_PP_ON_NEW_FLOORS: u8 = 71;
    pub(crate) const RESTORE_HP_ON_NEW_FLOORS: u8 = 72;
    /// Its HP come back faster as turns pass.
    pub(crate) const INCREASED_HP_RECOVERY: u8 = 73;
    /// More PP for each of its moves; more max HP (`MAX_HP_BOOST`).
    pub(crate) const MAX_PP_BOOST: u8 = 74;
    pub(crate) const MAX_HP_BOOST_10: u8 = 76;
    pub(crate) const MAX_HP_BOOST_20: u8 = 77;
    pub(crate) const MAX_HP_BOOST_30: u8 = 78;
    pub(crate) const EXP_BOOST: u8 = 79;
    /// Hurt, it earns experience, as by a Joy Ribbon; now and then a PP
    /// back.
    pub(crate) const EXP_FROM_DAMAGE: u8 = 80;
    pub(crate) const MAY_RESTORE_PP_FROM_DAMAGE: u8 = 81;
    /// Now and then a move of it costs no PP.
    pub(crate) const MAY_NOT_CONSUME_PP: u8 = 82;
    /// Nothing thrown hits it; or it bounces back.
    pub(crate) const THROWN_ITEM_PROTECTION: u8 = 83;
    pub(crate) const BOUNCE_THROWN_ITEMS: u8 = 84;
    /// Its moves on itself reach the whole team.
    pub(crate) const EXTEND_SELF_EFFECTS_TO_TEAM: u8 = 85;
    /// Its draining moves give twice the HP back.
    pub(crate) const HP_DRAIN_RECOVERY_BOOST: u8 = 87;
    /// Its Pressure costs its strikers more PP.
    pub(crate) const PRESSURE_BOOST: u8 = 88;
    pub(crate) const NO_STATUS: u8 = 89;
    pub(crate) const HALVED_DAMAGE: u8 = 90;
    pub(crate) const DAMAGE_BOOST_50_PCT: u8 = 91;
    /// Each turn it takes its teammates' poison away, and HP for it.
    pub(crate) const ABSORB_TEAMMATE_POISON: u8 = 92;
    /// Apples, berries (and the other things eaten) give it HP too.
    pub(crate) const RECOVER_HP_FROM_APPLES_AND_BERRIES: u8 = 93;
    /// Kecleon shops are likelier on the floors of its team.
    pub(crate) const MORE_KECLEON_SHOPS: u8 = 94;
    pub(crate) const NO_FRIENDLY_FIRE: u8 = 96;
    /// Pickup finds more; beaten foes now and then drop Poké.
    pub(crate) const PICKUP_BOOST: u8 = 97;
    pub(crate) const MORE_MONEY_DROPS: u8 = 98;
    /// In water, its HP come back and its status problems go.
    pub(crate) const RECOVER_HP_FROM_WATERY_TERRAIN: u8 = 100;
    pub(crate) const HEAL_STATUS_FROM_WATERY_TERRAIN: u8 = 101;
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

/// What some exclusive items do in one weather only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InWeather {
    /// No status problem.
    NoStatus,
    /// A speed stage more.
    MovementSpeed,
    /// Two attacks a turn.
    AttackSpeed,
    /// An evasion stage more.
    Evasion,
}

/// The first effect of each, the one of clear weather: then one by weather
/// up to fog, none in snow (the game's tables at EU 0x23541B0, 0x23541B8,
/// 0x23541C0 and 0x2354310, by `enum weather_id`).
const IN_WEATHER: [(InWeather, u8); 4] = [(InWeather::NoStatus, 40), (InWeather::MovementSpeed, 47), (InWeather::AttackSpeed, 54), (InWeather::Evasion, 61)];
/// Clear, sunny, sandstorm, cloudy, rain, hail, fog.
const WEATHERS: u8 = 7;

/// The effects of the exclusive items a monster has the use of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Exclusive {
    pub effects: [u64; 3],
    /// Attack, defense, special attack, special defense raised.
    pub boosts: [i32; 4],
    /// The weather as it feels it, for what works in one weather only
    /// (kept by the caller).
    pub weather: u8,
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

    /// What it does in this weather.
    pub(crate) fn in_weather(&self, what: InWeather, weather: u8) -> bool {
        let first = IN_WEATHER.iter().find(|(kind, _)| *kind == what).map_or(0, |&(_, first)| first);
        weather < WEATHERS && self.has(first + weather)
    }

    /// No status problem: always, or in the weather it feels.
    pub(crate) fn no_status(&self) -> bool {
        self.has(effect::NO_STATUS) || self.in_weather(InWeather::NoStatus, self.weather)
    }

    /// The max HP it adds (the game's 10, twice, three times, read from
    /// its arm9).
    pub(crate) fn max_hp_boost(&self) -> i32 {
        const MAX_HP_BOOST: i32 = 10;
        [(effect::MAX_HP_BOOST_10, 1), (effect::MAX_HP_BOOST_20, 2), (effect::MAX_HP_BOOST_30, 3)]
            .iter()
            .filter(|&&(which, _)| self.has(which))
            .map(|&(_, times)| times * MAX_HP_BOOST)
            .sum()
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

    #[test]
    fn the_weather_effects_work_in_their_weather_only() {
        use crate::dungeon::weather::{CLEAR, FOG, RAIN, SANDSTORM, SNOW, SUNNY};
        // (Effect, the weather it works in.)
        let table = [
            (40, CLEAR, InWeather::NoStatus),
            (41, SUNNY, InWeather::NoStatus),
            (46, FOG, InWeather::NoStatus),
            (48, SUNNY, InWeather::MovementSpeed),
            (51, RAIN, InWeather::MovementSpeed),
            (55, SUNNY, InWeather::AttackSpeed),
            (58, RAIN, InWeather::AttackSpeed),
            (63, SANDSTORM, InWeather::Evasion),
            (67, FOG, InWeather::Evasion),
        ];
        for (effect, weather, what) in table {
            let mut held = Exclusive::default();
            held.effects[effect as usize / 64] |= 1 << (effect % 64);
            for other in [CLEAR, SUNNY, SANDSTORM, RAIN, FOG, SNOW] {
                assert_eq!(held.in_weather(what, other), other == weather, "effect {effect} in weather {other}");
            }
            let kinds = [InWeather::NoStatus, InWeather::MovementSpeed, InWeather::AttackSpeed, InWeather::Evasion];
            assert!(kinds.iter().filter(|&&kind| kind != what).all(|&kind| !held.in_weather(kind, weather)));
            held.weather = weather;
            assert_eq!(held.no_status(), what == InWeather::NoStatus);
        }
    }
}
