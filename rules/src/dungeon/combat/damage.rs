//! The damage formula: this project's own, in the spirit of the games.
//!
//! A blow's base damage grows with the attacker's level and with its
//! attack plus the move's power, against the target's defense. Then multipliers
//! pile up: type matchups, the same-type bonus, abilities, the weather,
//! screens, a critical hit, and a random spread of 85 % to 100 %.

use super::*;
use crate::dungeon::ability::{self, has, shields};
use crate::dungeon::{held, iq};

/// A monster as the damage calculation sees it.
#[derive(Clone, Copy, Debug)]
pub struct Fighter {
    pub stats: Stats,
    /// Stat stages, from 0 to 20 (10 neutral).
    pub attack_stage: i32,
    pub defense_stage: i32,
    /// Stat multipliers (Charm, Screech...), in 8-bit fixed point.
    pub attack_multiplier: i32,
    pub defense_multiplier: i32,
    /// The same for the special stats.
    pub sp_attack_stage: i32,
    pub sp_defense_stage: i32,
    pub sp_attack_multiplier: i32,
    pub sp_defense_multiplier: i32,
    /// Reflect halves physical damage taken, Light Screen special damage.
    pub halves_physical: bool,
    pub halves_special: bool,
    /// Attackers outside the team deal less damage.
    pub team_member: bool,
    /// Burned attackers deal less physical damage.
    pub burned: bool,
    /// It has a status problem: Guts and Marvel Scale come into play.
    pub ailing: bool,
    /// Its blows are all critical hits.
    pub focused: bool,
    /// No critical hit lands on it (Lucky Chant).
    pub lucky: bool,
    /// Its Normal and Fighting moves reach Ghosts, its Psychic moves
    /// Darks (exclusive items).
    pub reaches: (bool, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Damage {
    pub amount: i32,
    /// Overall type matchup, see [`matchup`].
    pub matchup: u8,
    pub critical: bool,
}

/// The move's power adds to the attack (powers run from a few units to a
/// few tens, so the attacker's own strength matters as much); this brings
/// the result to the scale of HP.
const BLOW_SCALE: f64 = 1.6;
/// Every blow does at least this much on top of the formula.
const FLAT_DAMAGE: f64 = 2.0;
/// The most a single blow deals.
const MAX_DAMAGE: f64 = 999.0;
/// What foes of the team deal, as a share of what a team member would.
const OUTSIDER_SHARE: f64 = 0.8;

const SAME_TYPE_BONUS: f64 = 1.5;
const ADAPTABILITY_BONUS: f64 = 2.0;
const CRITICAL_BONUS: f64 = 1.5;
const SNIPER_BONUS: f64 = 2.0;
/// Reflect, Light Screen, Thick Fat, Heatproof and a burn each halve.
const HALVED: f64 = 0.5;
/// Sun on Fire, rain on Water; the other way round they are halved.
const WEATHER_BONUS: f64 = 1.5;
/// Clouds dampen every move but Normal ones.
const CLOUDY_DAMPING: f64 = 0.8;
/// Solid Rock and Filter on a super-effective move.
const SOFTENED: f64 = 0.75;
/// The pinch abilities, Technician, Guts, Hustle, the power abilities and
/// Marvel Scale.
const ABILITY_BONUS: f64 = 1.5;
/// Dry Skin on a Fire move.
const DRY_SKIN_PENALTY: f64 = 1.25;
/// Intimidate on a physical attacker.
const INTIMIDATED: f64 = 0.75;
/// A sandstorm hardens Rock types against special moves.
const SANDSTORM_SP_DEFENSE: f64 = 1.5;
/// Critical hit chance with Type-Advantage Master on a super-effective move.
const TYPE_ADVANTAGE_CRIT: i32 = 40;
/// The random spread: from this percentage up to 100.
const SPREAD_LOW: i32 = 85;

pub(super) const TYPE_NORMAL: u8 = 1;
pub(super) const TYPE_FIRE: u8 = 2;
pub(super) const TYPE_WATER: u8 = 3;
pub(super) const TYPE_GRASS: u8 = 4;
pub(super) const TYPE_ELECTRIC: u8 = 5;
pub(super) const TYPE_ICE: u8 = 6;
pub(super) const TYPE_GROUND: u8 = 9;
pub(super) const TYPE_BUG: u8 = 12;
pub(super) const TYPE_ROCK: u8 = 13;

/// What a stat stage (0 to 20, 10 neutral) multiplies the stat by: a
/// quarter more for each stage up, as much less for each stage down.
pub(super) fn stage_factor(stage: i32) -> f64 {
    let from_neutral = (stage.clamp(0, STAGE_COUNT as i32 - 1) - 10) as f64;
    if from_neutral >= 0.0 { (4.0 + from_neutral) / 4.0 } else { 4.0 / (4.0 - from_neutral) }
}

/// A multiplier in 8-bit fixed point, as a plain number.
fn fixed(multiplier: i32) -> f64 {
    multiplier as f64 / FIXED_ONE as f64
}

/// The base damage before any multiplier: it grows with the level, and
/// with the attack plus the move's power against the defense.
fn base_damage(level: i32, power: i32, attack: f64, defense: f64) -> f64 {
    let level_factor = 2.0 * level.max(1) as f64 / 5.0 + 2.0;
    level_factor * (attack + power.max(0) as f64) / defense.max(1.0) * BLOW_SCALE + FLAT_DAMAGE
}

/// One side's stat as a blow uses it: the raw stat, its stage, its
/// multiplier (8-bit fixed point).
#[derive(Clone, Copy)]
struct Stat {
    value: i32,
    stage: i32,
    multiplier: i32,
}

impl Stat {
    fn worth(self) -> f64 {
        self.value.max(0) as f64 * stage_factor(self.stage) * fixed(self.multiplier)
    }
}

/// The attack and the defense a blow pits against each other, stages,
/// held items and abilities counted.
fn attack_and_defense(attacker: &Fighter, defender: &Fighter, special: bool, weather: u8) -> (f64, f64) {
    let (a, d) = (&attacker.stats, &defender.stats);
    let mut attack = match special {
        true => Stat { value: a.sp_attack, stage: attacker.sp_attack_stage, multiplier: attacker.sp_attack_multiplier },
        false => Stat { value: a.attack, stage: attacker.attack_stage, multiplier: attacker.attack_multiplier },
    };
    let mut defense = match special {
        true => Stat { value: d.sp_defense, stage: defender.sp_defense_stage, multiplier: defender.sp_defense_multiplier },
        false => Stat { value: d.defense, stage: defender.defense_stage, multiplier: defender.defense_multiplier },
    };
    // Download aims at the weaker of the target's two defenses.
    let weaker_is_special = d.sp_defense <= d.defense;
    if has(a, ability::DOWNLOAD) && special == weaker_is_special {
        attack.stage += 1;
    }
    // IQ skills: Aggressor and Defender trade attack for defense; Counter
    // Basher leaves its bearer open to physical blows.
    attack.stage += iq::has(a, iq::AGGRESSOR) as i32 - iq::has(a, iq::DEFENDER) as i32;
    defense.stage += iq::has(d, iq::DEFENDER) as i32 - iq::has(d, iq::AGGRESSOR) as i32;
    if !special && iq::has(d, iq::COUNTER_BASHER) {
        defense.stage -= 1;
    }
    // Unaware looks past the other side's changes.
    if has(a, ability::UNAWARE) {
        (defense.stage, defense.multiplier) = (10, FIXED_ONE);
    } else if shields(a, d, ability::UNAWARE) {
        (attack.stage, attack.multiplier) = (10, FIXED_ONE);
    }
    let attack = (attack.worth() + held::stat_boost(a, special as usize) as f64) * attack_abilities(attacker, defender, special, weather);
    let defense = (defense.worth() + held::stat_boost(d, 2 + special as usize) as f64) * defense_abilities(attacker, defender, special, weather);
    (attack.clamp(0.0, MAX_DAMAGE), defense.max(1.0))
}

/// What abilities (and the weather through them) multiply the attack by.
fn attack_abilities(attacker: &Fighter, defender: &Fighter, special: bool, weather: u8) -> f64 {
    let (a, d) = (&attacker.stats, &defender.stats);
    let boosts = match special {
        true => vec![has(a, ability::SOLAR_POWER) && weather == weather::SUNNY],
        false => vec![has(a, ability::GUTS) && attacker.ailing, has(a, ability::HUGE_POWER) || has(a, ability::PURE_POWER), has(a, ability::HUSTLE)],
    };
    let mut factor = boosts.into_iter().filter(|&on| on).fold(1.0, |factor, _| factor * ABILITY_BONUS);
    if !special && shields(a, d, ability::INTIMIDATE) {
        factor *= INTIMIDATED;
    }
    factor
}

/// What abilities and the weather multiply the defense by.
fn defense_abilities(attacker: &Fighter, defender: &Fighter, special: bool, weather: u8) -> f64 {
    let d = &defender.stats;
    let rock = d.types.0 == TYPE_ROCK || d.types.1 == TYPE_ROCK;
    match special {
        true if rock && weather == weather::SANDSTORM => SANDSTORM_SP_DEFENSE,
        false if defender.ailing && shields(&attacker.stats, d, ability::MARVEL_SCALE) => ABILITY_BONUS,
        _ => 1.0,
    }
}

/// What the type matchups make of a move against both of the defender's
/// types, and the overall matchup. Without a chart, everything is neutral.
fn matchup_factor(chart: Option<&TypeChart>, attacker: &Stats, defender: &Stats, attack_type: u8, (ghosts, darks): (bool, bool)) -> (f64, u8) {
    let Some(chart) = chart else {
        return (1.0, matchup::NEUTRAL);
    };
    const TYPE_NORMAL: u8 = 1;
    const TYPE_FIGHTING: u8 = 7;
    const TYPE_PSYCHIC: u8 = 11;
    const TYPE_GHOST: u8 = 14;
    const TYPE_DARK: u8 = 16;
    let erratic = iq::has(attacker, iq::ERRATIC_PLAYER);
    let one = |kind: u8| {
        let reached =
            (ghosts && kind == TYPE_GHOST && matches!(attack_type, TYPE_NORMAL | TYPE_FIGHTING)) || (darks && kind == TYPE_DARK && attack_type == TYPE_PSYCHIC);
        let value = match chart.matchup(attack_type, kind) {
            matchup::IMMUNE if reached || ability::scrappy(attacker, attack_type, kind) => matchup::NEUTRAL,
            value => value,
        };
        let table = if erratic && chart.erratic[value as usize] != 0 { &chart.erratic } else { &chart.multipliers };
        let factor = if value == matchup::NEUTRAL && !erratic { 1.0 } else { fixed(table[value as usize]) };
        (factor, value)
    };
    let ((first, first_value), (second, second_value)) = (one(defender.types.0), one(defender.types.1));
    (first * second, chart.combinator[first_value as usize][second_value as usize])
}

/// The type matchups, then what abilities do with the move's type: the
/// damage multiplier and the overall matchup.
pub(super) fn type_effects(chart: Option<&TypeChart>, attacker: &Stats, defender: &Stats, attack_type: u8, power: i32, reaches: (bool, bool)) -> (f64, u8) {
    let (mut factor, overall) = matchup_factor(chart, attacker, defender, attack_type, reaches);
    let shielded = |which: u8| shields(attacker, defender, which);
    // Flash Fire and Levitate take the move in whole.
    if (attack_type == TYPE_FIRE && shielded(ability::FLASH_FIRE)) || (attack_type == TYPE_GROUND && shielded(ability::LEVITATE)) {
        return (0.0, matchup::IMMUNE);
    }
    // Wonder Guard lets only super-effective typed moves through.
    if shielded(ability::WONDER_GUARD) && attack_type != 0 && overall != matchup::SUPER_EFFECTIVE {
        factor = 0.0;
    }
    if overall == matchup::NOT_VERY_EFFECTIVE && has(attacker, ability::TINTED_LENS) {
        factor *= ability::TINTED_LENS_BOOST;
    }
    if overall == matchup::SUPER_EFFECTIVE && (shielded(ability::SOLID_ROCK) || shielded(ability::FILTER)) {
        factor *= SOFTENED;
    }
    if power <= ability::TECHNICIAN_POWER && has(attacker, ability::TECHNICIAN) {
        factor *= ABILITY_BONUS;
    }
    let halving = [(TYPE_FIRE, ability::THICK_FAT), (TYPE_ICE, ability::THICK_FAT), (TYPE_FIRE, ability::HEATPROOF)];
    for (kind, which) in halving {
        if attack_type == kind && shielded(which) {
            factor *= HALVED;
        }
    }
    if attack_type == TYPE_FIRE && shielded(ability::DRY_SKIN) {
        factor *= DRY_SKIN_PENALTY;
    }
    let pinch = [(TYPE_WATER, ability::TORRENT), (TYPE_GRASS, ability::OVERGROW), (TYPE_BUG, ability::SWARM), (TYPE_FIRE, ability::BLAZE)];
    if ability::in_a_pinch(attacker) && pinch.iter().any(|&(kind, which)| attack_type == kind && has(attacker, which)) {
        factor *= ABILITY_BONUS;
    }
    (factor, overall)
}

/// The bonus for a move of one of its user's own types (never for the
/// typeless regular attack).
fn same_type_bonus(attacker: &Stats, attack_type: u8) -> f64 {
    let own = attack_type != 0 && (attacker.types.0 == attack_type || attacker.types.1 == attack_type);
    match (own, has(attacker, ability::ADAPTABILITY)) {
        (false, _) => 1.0,
        (true, true) => ADAPTABILITY_BONUS,
        (true, false) => SAME_TYPE_BONUS,
    }
}

/// What the weather does to a move of this type.
pub(super) fn weather_factor(weather: u8, attack_type: u8) -> f64 {
    match (weather, attack_type) {
        (weather::SUNNY, TYPE_FIRE) | (weather::RAIN, TYPE_WATER) => WEATHER_BONUS,
        (weather::SUNNY, TYPE_WATER) | (weather::RAIN, TYPE_FIRE) | (weather::FOG, TYPE_ELECTRIC) => HALVED,
        (weather::CLOUDY, kind) if kind != TYPE_NORMAL => CLOUDY_DAMPING,
        _ => 1.0,
    }
}

/// The chance in percent of a critical hit, or `None` when none can land.
fn critical_chance(attacker: &Fighter, defender: &Fighter, crit_chance: i32, overall: u8) -> Option<i32> {
    let (a, d) = (&attacker.stats, &defender.stats);
    let armored = shields(a, d, ability::BATTLE_ARMOR) || shields(a, d, ability::SHELL_ARMOR);
    if armored || defender.lucky || iq::has(d, iq::CRITICAL_DODGER) {
        return None;
    }
    if attacker.focused {
        return Some(100);
    }
    if overall == matchup::SUPER_EFFECTIVE && iq::has(a, iq::TYPE_ADVANTAGE_MASTER) {
        return Some(TYPE_ADVANTAGE_CRIT.max(crit_chance));
    }
    let boosts = [
        (held::holds(a, held::SCOPE_LENS) || iq::has(a, iq::SHARPSHOOTER), held::CRIT_BOOST),
        (has(a, ability::SUPER_LUCK), ability::SUPER_LUCK_CRIT),
        (held::holds(d, held::PATSY_BAND), held::CRIT_BOOST),
    ];
    Some(crit_chance + boosts.iter().filter(|(on, _)| *on).map(|(_, by)| by).sum::<i32>())
}

/// [`calc_damage_with`] for a physical move in clear weather.
#[cfg(test)]
pub(crate) fn calc_damage(
    rng: &mut DungeonRng,
    constants: &Constants,
    chart: Option<&TypeChart>,
    attacker: &Fighter,
    defender: &Fighter,
    attack_type: u8,
    power: i32,
    crit_chance: i32,
    damage_mult: i32,
) -> Damage {
    calc_damage_with(rng, constants, chart, attacker, defender, attack_type, power, crit_chance, damage_mult, false, weather::CLEAR)
}

/// The damage of one blow. `crit_chance` is the move's, in percent;
/// `damage_mult` is a multiplier in 8-bit fixed point ([`FIXED_ONE`] for
/// none). `special` moves use the special attack and the special defense.
/// Two rolls are drawn each time: the critical hit, then the spread.
pub fn calc_damage_with(
    rng: &mut DungeonRng,
    _constants: &Constants,
    chart: Option<&TypeChart>,
    attacker: &Fighter,
    defender: &Fighter,
    attack_type: u8,
    power: i32,
    crit_chance: i32,
    damage_mult: i32,
    special: bool,
    weather: u8,
) -> Damage {
    let (a, d) = (&attacker.stats, &defender.stats);
    let attack_type = ability::attack_type(a, attack_type);
    let (attack, defense) = attack_and_defense(attacker, defender, special, weather);
    let mut base = base_damage(a.level, power, attack, defense);
    if !attacker.team_member {
        base *= OUTSIDER_SHARE;
    }
    let (by_type, matchup) = type_effects(chart, a, d, attack_type, power, attacker.reaches);
    let mut factor = by_type * same_type_bonus(a, attack_type) * weather_factor(weather, attack_type) * fixed(damage_mult);
    if attacker.burned && !special {
        factor *= HALVED;
    }
    if (special && defender.halves_special) || (!special && defender.halves_physical) {
        factor *= HALVED;
    }
    let crit_roll = rng.rand_int(100);
    let critical = factor > 0.0 && critical_chance(attacker, defender, crit_chance, matchup).is_some_and(|chance| crit_roll < chance);
    if critical {
        factor *= if has(a, ability::SNIPER) { SNIPER_BONUS } else { CRITICAL_BONUS };
    }
    let spread = (SPREAD_LOW + rng.rand_int(101 - SPREAD_LOW)) as f64 / 100.0;
    let amount = match factor > 0.0 {
        true => (base.min(MAX_DAMAGE) * factor * spread).round().clamp(1.0, MAX_DAMAGE) as i32,
        false => 0,
    };
    Damage { amount, matchup, critical }
}
