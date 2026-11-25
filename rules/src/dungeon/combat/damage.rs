//! The damage formula: the game's own, as pmdsky-debug documents its
//! `CalcDamage` and the decompilation shows it, in floating point where the
//! game counts in fixed point (the same to a unit or so).
//!
//! The attack A and the defense D are the stats through their stages (the
//! game's tables), multipliers, held items and abilities; P is the move's
//! power through the same attack stage; L the attacker's level. The base is
//!
//! `153/256 (A + P) - D/2 + 50 ln(10 [L + (A - D)/8 + 50]) - 311`,
//!
//! divided by 85/64 for those outside the team, kept between 1 and 999.
//! Then multipliers pile up: type matchups, the same-type bonus, abilities,
//! the weather, a burn, screens, a critical hit, the move's own, and a
//! random spread of 7/8 to 9/8.

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
    /// Burned attackers deal less damage.
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
    /// Plus beside a Minus, or Minus beside a Plus: its special attack is
    /// worth half again (see `ability::cheer`).
    pub cheered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Damage {
    pub amount: i32,
    /// Overall type matchup, see [`matchup`].
    pub matchup: u8,
    pub critical: bool,
}

/// The game's damage formula constants (pmdsky-debug's `DAMAGE_FORMULA_*`).
const AT_PREFACTOR: f64 = 153.0 / 256.0;
const DEF_PREFACTOR: f64 = -0.5;
const FLV_DEFICIT_DIVISOR: f64 = 8.0;
const FLV_SHIFT: f64 = 50.0;
const LN_ARG_PREFACTOR: f64 = 10.0;
const LN_PREFACTOR: f64 = 50.0;
const CONSTANT_SHIFT: f64 = -311.0;
/// What the base of a blow from outside the team is divided by.
const NON_TEAM_DIVISOR: f64 = 85.0 / 64.0;
/// The base of a blow, and a blow, at most; the attack, at most.
const MAX_DAMAGE: f64 = 999.0;
const MAX_ATTACK: i32 = 999;
/// The natural logarithm's domain in the game's table.
const LN_DOMAIN: (i32, i32) = (1, 0x7FF);

const SAME_TYPE_BONUS: f64 = 1.5;
const ADAPTABILITY_BONUS: f64 = 2.0;
const CRITICAL_BONUS: f64 = 1.5;
const SNIPER_BONUS: f64 = 2.0;
/// Reflect, Light Screen, Thick Fat, Heatproof, the weather against a
/// type, Fog on Electric moves each halve.
const HALVED: f64 = 0.5;
/// A burn, on any blow (0xCC in 8-bit fixed point).
const BURNED: f64 = 204.0 / 256.0;
/// Sun on Fire, rain on Water.
const WEATHER_BONUS: f64 = 1.5;
/// Clouds dampen every move but Normal ones.
const CLOUDY_DAMPING: f64 = 0.75;
/// Solid Rock and Filter on a super-effective move.
const SOFTENED: f64 = 0.75;
/// Technician; Dry Skin on a Fire move.
const BOOSTED: f64 = 1.5;
/// The pinch abilities.
const PINCH_BONUS: f64 = 2.0;
/// Marvel Scale's defense, as a fraction.
const MARVEL_SCALE: (i32, i32) = (3, 2);
/// The stages Solar Power adds to the special attack in the sun, and a
/// sandstorm to a Rock type's special defense.
const SOLAR_POWER_STAGES: i32 = 2;
const SANDSTORM_SP_DEFENSE_STAGES: i32 = 2;
/// Huge Power and Pure Power's chance in 100 to work on a blow.
const HUGE_POWER_CHANCE: i32 = 33;
/// The critical hit chance with Type-Advantage Master on a super-effective
/// move, and a sure one (Focus Energy).
const TYPE_ADVANTAGE_CRIT: i32 = 40;
const SURE_CRIT: i32 = 999;
/// The random spread, in 16-bit fixed point: from 0xE000 for 0x4000.
const SPREAD_LOW: i32 = 0xE000;
const SPREAD_WIDTH: i32 = 0x4000;
/// The neutral stage.
const NEUTRAL: i32 = 10;

pub(super) const TYPE_NORMAL: u8 = 1;
pub(super) const TYPE_FIRE: u8 = 2;
pub(super) const TYPE_WATER: u8 = 3;
pub(super) const TYPE_GRASS: u8 = 4;
pub(super) const TYPE_ELECTRIC: u8 = 5;
pub(super) const TYPE_ICE: u8 = 6;
pub(super) const TYPE_GROUND: u8 = 9;
pub(super) const TYPE_BUG: u8 = 12;
pub(super) const TYPE_ROCK: u8 = 13;

/// The multiplier of a stage (0 to 20, 10 neutral) in a table of the game,
/// in 8-bit fixed point. Without the game's data (an empty table), a
/// quarter more for each stage up, as much less for each stage down.
pub(crate) fn stage_multiplier(table: &[i32; STAGE_COUNT], stage: i32) -> i32 {
    let stage = stage.clamp(0, STAGE_COUNT as i32 - 1);
    if table[NEUTRAL as usize] != 0 {
        return table[stage as usize];
    }
    let from_neutral = stage - NEUTRAL;
    match from_neutral >= 0 {
        true => FIXED_ONE * (4 + from_neutral) / 4,
        false => FIXED_ONE * 4 / (4 - from_neutral),
    }
}

/// `a` times `b`, `b` in 8-bit fixed point, as the game multiplies them.
fn times(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> 8) as i32
}

/// A multiplier in 8-bit fixed point, as a plain number.
fn fixed(multiplier: i32) -> f64 {
    multiplier as f64 / FIXED_ONE as f64
}

/// The natural logarithm as the game takes it: of a whole number clamped
/// to its table's domain, from the table where the data has it.
fn game_ln(chart: Option<&TypeChart>, x: f64) -> f64 {
    let x = (x + 0.5).floor().clamp(LN_DOMAIN.0 as f64, LN_DOMAIN.1 as f64);
    match chart.and_then(|chart| chart.ln_table.get(x as usize)) {
        Some(&value) if value != 0 => value as f64 / 4096.0,
        _ => x.ln(),
    }
}

/// The game's base damage, before any multiplier: from the attack and the
/// power (both through their stages), the defense and the level.
pub(crate) fn base_damage(chart: Option<&TypeChart>, level: i32, attack: i32, power: f64, defense: i32, team_member: bool) -> f64 {
    let at = power + attack as f64;
    let flv = level as f64 + (attack - defense) as f64 / FLV_DEFICIT_DIVISOR;
    let ln = game_ln(chart, LN_ARG_PREFACTOR * (flv + FLV_SHIFT));
    let mut base = AT_PREFACTOR * at + DEF_PREFACTOR * defense as f64 + LN_PREFACTOR * ln + CONSTANT_SHIFT;
    if !team_member {
        base /= NON_TEAM_DIVISOR;
    }
    base.clamp(1.0, MAX_DAMAGE)
}

/// One side's stat as a blow uses it: the raw stat, its stage, its
/// multiplier (8-bit fixed point).
#[derive(Clone, Copy)]
struct Stat {
    value: i32,
    stage: i32,
    multiplier: i32,
}

/// The attack, the power through the attack's stage (a plain number) and
/// the defense a blow pits against each other, stages, held items and
/// abilities counted. `huge`: Huge Power's roll came up.
fn attack_and_defense(constants: &Constants, attacker: &Fighter, defender: &Fighter, power: i32, special: bool, weather: u8, huge: bool) -> (i32, f64, i32) {
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
    // Solar Power in the sun; a sandstorm hardens Rock types against
    // special moves.
    if special && has(a, ability::SOLAR_POWER) && weather == weather::SUNNY {
        attack.stage += SOLAR_POWER_STAGES;
    }
    let rock = d.types.0 == TYPE_ROCK || d.types.1 == TYPE_ROCK;
    if special && rock && weather == weather::SANDSTORM {
        defense.stage += SANDSTORM_SP_DEFENSE_STAGES;
    }
    // Unaware looks past the other side's changes.
    if has(a, ability::UNAWARE) {
        (defense.stage, defense.multiplier) = (NEUTRAL, FIXED_ONE);
    } else if shields(a, d, ability::UNAWARE) {
        (attack.stage, attack.multiplier) = (NEUTRAL, FIXED_ONE);
    }
    let attack_stage = stage_multiplier(&constants.offensive_stages, attack.stage);
    let defense_stage = stage_multiplier(&constants.defensive_stages, defense.stage);
    let through = |value: i32, stage: i32, multiplier: i32| times(times(value.max(0) << 8, stage), multiplier);
    let mut attack_value = (through(attack.value, attack_stage, attack.multiplier) >> 8) + held::stat_boost(a, special as usize);
    let mut defense_value = (through(defense.value, defense_stage, defense.multiplier) >> 8) + held::stat_boost(d, 2 + special as usize);
    let power = through(power, attack_stage, attack.multiplier) as f64 / FIXED_ONE as f64;
    let (numerator, denominator) = attack_abilities(attacker, defender, special, huge);
    attack_value = attack_value * numerator / denominator;
    if !special && defender.ailing && shields(a, d, ability::MARVEL_SCALE) {
        defense_value = defense_value * MARVEL_SCALE.0 / MARVEL_SCALE.1;
    }
    (attack_value.clamp(0, MAX_ATTACK), power, defense_value)
}

/// What abilities multiply the attack by, as a fraction: Guts, Huge Power
/// (on its roll), Hustle, Plus and Minus, and the target's Intimidate.
fn attack_abilities(attacker: &Fighter, defender: &Fighter, special: bool, huge: bool) -> (i32, i32) {
    let (a, d) = (&attacker.stats, &defender.stats);
    let boosts = [
        (!special && has(a, ability::GUTS) && attacker.ailing, (2, 1)),
        (!special && huge && (has(a, ability::HUGE_POWER) || has(a, ability::PURE_POWER)), (3, 2)),
        (!special && has(a, ability::HUSTLE), (3, 2)),
        (special && attacker.cheered, (15, 10)),
        (!special && shields(a, d, ability::INTIMIDATE), (4, 5)),
    ];
    boosts.iter().filter(|(on, _)| *on).fold((1, 1), |(n, d), (_, (by_n, by_d))| (n * by_n, d * by_d))
}

/// What the type matchups make of a move against both of the defender's
/// types, and the overall matchup. Without a chart, everything is neutral.
/// Erratic Player on either side changes the multipliers, on all but the
/// regular attack; the attacker's even the neutral one.
fn matchup_factor(chart: Option<&TypeChart>, attacker: &Stats, defender: &Stats, attack_type: u8, (ghosts, darks): (bool, bool), regular: bool) -> (f64, u8) {
    let Some(chart) = chart else {
        return (1.0, matchup::NEUTRAL);
    };
    const TYPE_NORMAL: u8 = 1;
    const TYPE_FIGHTING: u8 = 7;
    const TYPE_PSYCHIC: u8 = 11;
    const TYPE_GHOST: u8 = 14;
    const TYPE_DARK: u8 = 16;
    let erratic = !regular && iq::has(attacker, iq::ERRATIC_PLAYER);
    let erratic_table = (erratic || (!regular && iq::has(defender, iq::ERRATIC_PLAYER))) && chart.erratic != [0; 4];
    let one = |kind: u8| {
        let reached =
            (ghosts && kind == TYPE_GHOST && matches!(attack_type, TYPE_NORMAL | TYPE_FIGHTING)) || (darks && kind == TYPE_DARK && attack_type == TYPE_PSYCHIC);
        let value = match chart.matchup(attack_type, kind) {
            matchup::IMMUNE if reached || ability::scrappy(attacker, attack_type, kind) => matchup::NEUTRAL,
            value => value,
        };
        let table = if erratic_table { &chart.erratic } else { &chart.multipliers };
        let factor = if value == matchup::NEUTRAL && !erratic { 1.0 } else { fixed(table[value as usize]) };
        (factor, value)
    };
    let ((first, first_value), (second, second_value)) = (one(defender.types.0), one(defender.types.1));
    (first * second, chart.combinator[first_value as usize][second_value as usize])
}

/// The type matchups, then what abilities do with the move's type: the
/// damage multiplier and the overall matchup. `regular`: the regular
/// attack, which Technician and Erratic Player leave alone.
pub(super) fn type_effects(
    chart: Option<&TypeChart>,
    attacker: &Stats,
    defender: &Stats,
    attack_type: u8,
    power: i32,
    reaches: (bool, bool),
    regular: bool,
) -> (f64, u8) {
    let (mut factor, overall) = matchup_factor(chart, attacker, defender, attack_type, reaches, regular);
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
        factor *= fixed(ability::TINTED_LENS_BOOST);
    }
    if overall == matchup::SUPER_EFFECTIVE && (shielded(ability::SOLID_ROCK) || shielded(ability::FILTER)) {
        factor *= SOFTENED;
    }
    if !regular && power <= ability::TECHNICIAN_POWER && has(attacker, ability::TECHNICIAN) {
        factor *= BOOSTED;
    }
    let halving = [(TYPE_FIRE, ability::THICK_FAT), (TYPE_ICE, ability::THICK_FAT), (TYPE_FIRE, ability::HEATPROOF)];
    for (kind, which) in halving {
        if attack_type == kind && shielded(which) {
            factor *= HALVED;
        }
    }
    if attack_type == TYPE_FIRE && shielded(ability::DRY_SKIN) {
        factor *= BOOSTED;
    }
    let pinch = [(TYPE_WATER, ability::TORRENT), (TYPE_GRASS, ability::OVERGROW), (TYPE_BUG, ability::SWARM), (TYPE_FIRE, ability::BLAZE)];
    if ability::in_a_pinch(attacker) && pinch.iter().any(|&(kind, which)| attack_type == kind && has(attacker, which)) {
        factor *= PINCH_BONUS;
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
/// Females keep the move's chance; the others have half again.
fn critical_chance(attacker: &Fighter, defender: &Fighter, crit_chance: i32, overall: u8) -> Option<i32> {
    let (a, d) = (&attacker.stats, &defender.stats);
    let armored = shields(a, d, ability::BATTLE_ARMOR) || shields(a, d, ability::SHELL_ARMOR);
    if armored || defender.lucky || iq::has(d, iq::CRITICAL_DODGER) {
        return None;
    }
    if attacker.focused {
        return Some(SURE_CRIT);
    }
    if overall == matchup::SUPER_EFFECTIVE && iq::has(a, iq::TYPE_ADVANTAGE_MASTER) {
        return Some(TYPE_ADVANTAGE_CRIT);
    }
    let chance = if a.female { crit_chance } else { crit_chance + crit_chance / 2 };
    let boosts = [
        (held::holds(a, held::SCOPE_LENS) || iq::has(a, iq::SHARPSHOOTER), held::CRIT_BOOST),
        (has(a, ability::SUPER_LUCK), ability::SUPER_LUCK_CRIT),
        (held::holds(d, held::PATSY_BAND), held::CRIT_BOOST),
    ];
    Some(chance + boosts.iter().filter(|(on, _)| *on).map(|(_, by)| by).sum::<i32>())
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
/// none). `special` moves use the special attack and the special defense;
/// the typeless move (type 0) is the regular attack. Three rolls are drawn
/// each time, as in the game: Huge Power's, the critical hit, the spread.
pub fn calc_damage_with(
    rng: &mut DungeonRng,
    constants: &Constants,
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
    let regular = attack_type == 0;
    let attack_type = ability::attack_type(a, attack_type);
    let huge = rng.rand_int(100) < HUGE_POWER_CHANCE;
    let (attack, stepped_power, defense) = attack_and_defense(constants, attacker, defender, power, special, weather, huge);
    let base = base_damage(chart, a.level.max(1), attack, stepped_power, defense, attacker.team_member);
    let (by_type, matchup) = type_effects(chart, a, d, attack_type, power, attacker.reaches, regular);
    let mut factor = by_type;
    if attacker.burned {
        factor *= BURNED;
    }
    if factor > 0.0 {
        factor *= same_type_bonus(a, attack_type);
    }
    factor *= weather_factor(weather, attack_type);
    if (special && defender.halves_special) || (!special && defender.halves_physical) {
        factor *= HALVED;
    }
    let crit_roll = rng.rand_int(100);
    let critical = factor > 0.0 && critical_chance(attacker, defender, crit_chance, matchup).is_some_and(|chance| crit_roll < chance);
    if critical {
        factor *= if has(a, ability::SNIPER) { SNIPER_BONUS } else { CRITICAL_BONUS };
    }
    let spread = (SPREAD_LOW + rng.rand_int(SPREAD_WIDTH)) as f64 / 65536.0;
    let amount = match factor > 0.0 {
        true => (base * factor * fixed(damage_mult) * spread + 0.5).floor().clamp(1.0, MAX_DAMAGE) as i32,
        false => 0,
    };
    Damage { amount, matchup, critical }
}
