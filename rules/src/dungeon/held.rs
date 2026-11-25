//! Items a monster holds (one each), and what they do while held. Item
//! numbers follow the item list of the extracted data. The stat items, the
//! Scope Lens and the Patsy Band add the game's figures (see `stat_boost`,
//! `CRIT_BOOST`); the others follow the well-known rules of the games.
//! (The Gold Ribbon and the Plain Ribbon do nothing, as in the game: the
//! first sells well.)

use super::combat::Stats;
use super::status::MoveEffect;

pub(crate) const NO_SLIP_CAP: u16 = 13;
pub(crate) const Y_RAY_SPECS: u16 = 14;
pub(crate) const GAGGLE_SPECS: u16 = 15;
pub(crate) const MOBILE_SCARF: u16 = 16;
pub(crate) const HEAL_RIBBON: u16 = 17;
pub(crate) const TWIST_BAND: u16 = 18;
pub(crate) const SCOPE_LENS: u16 = 19;
pub(crate) const PATSY_BAND: u16 = 20;
pub(crate) const NO_STICK_CAP: u16 = 21;
pub(crate) const PIERCE_BAND: u16 = 22;
pub(crate) const JOY_RIBBON: u16 = 23;
pub(crate) const X_RAY_SPECS: u16 = 24;
pub(crate) const PERSIM_BAND: u16 = 25;
pub(crate) const POWER_BAND: u16 = 26;
pub(crate) const PECHA_SCARF: u16 = 27;
pub(crate) const INSOMNISCOPE: u16 = 28;
pub(crate) const WARP_SCARF: u16 = 29;
pub(crate) const TIGHT_BELT: u16 = 30;
pub(crate) const SNEAK_SCARF: u16 = 31;
pub(crate) const DIET_RIBBON: u16 = 34;
pub(crate) const RACKET_BAND: u16 = 36;
pub(crate) const DEF_SCARF: u16 = 37;
pub(crate) const STAMINA_BAND: u16 = 38;
pub(crate) const SPECIAL_BAND: u16 = 40;
pub(crate) const ZINC_BAND: u16 = 41;
pub(crate) const DETECT_BAND: u16 = 42;
pub(crate) const DODGE_SCARF: u16 = 44;
pub(crate) const BOUNCE_BAND: u16 = 45;
pub(crate) const CURVE_BAND: u16 = 46;
pub(crate) const WHIFF_SPECS: u16 = 47;
pub(crate) const NO_AIM_SCOPE: u16 = 48;
pub(crate) const LOCKON_SPECS: u16 = 49;
pub(crate) const MUNCH_BELT: u16 = 50;
pub(crate) const PASS_SCARF: u16 = 51;
pub(crate) const WEATHER_BAND: u16 = 52;
pub(crate) const FRIEND_BOW: u16 = 53;

/// The items that are held rather than eaten or thrown (the No-Slip Cap
/// to the Wonder Chest).
pub(crate) fn is_held_item(item: u16) -> bool {
    (13..=67).contains(&item)
}

/// Whether a monster holds an item.
pub fn holds(stats: &Stats, item: u16) -> bool {
    item != 0 && working(stats) == item
}

/// The item held, as far as it works: none for a Klutz.
fn working(stats: &Stats) -> u16 {
    match super::ability::has(stats, super::ability::KLUTZ) {
        true => 0,
        false => stats.held,
    }
}

/// Critical hit chance, in percent, a Scope Lens adds to its holder's
/// blows, and a Patsy Band to the blows its holder takes (the game's).
pub(crate) const CRIT_BOOST: i32 = 15;

/// What a stat item adds to its holder's stat (0 attack, 1 special
/// attack, 2 defense, 3 special defense), after its stage: a band raises
/// an attack, a scarf or the Zinc Band a defense, the Munch Belt both
/// attacks. The game's figures (`POWER_BAND_STAT_BOOST` and its like).
pub(crate) fn stat_boost(stats: &Stats, stat: usize) -> i32 {
    const BAND: i32 = 12;
    const SCARF: i32 = 8;
    const BELT: i32 = 8;
    match (working(stats), stat) {
        (POWER_BAND, 0) | (SPECIAL_BAND, 1) => BAND,
        (DEF_SCARF, 2) | (ZINC_BAND, 3) => SCARF,
        (MUNCH_BELT, 0 | 1) => BELT,
        _ => 0,
    }
}

/// Whether a held item keeps an effect off its holder.
pub(crate) fn keeps_off(stats: &Stats, effect: MoveEffect) -> bool {
    use MoveEffect::*;
    match effect {
        Poison | BadlyPoison => holds(stats, PECHA_SCARF),
        Sleep => holds(stats, INSOMNISCOPE),
        Confuse => holds(stats, PERSIM_BAND),
        TargetAttack(by) | TargetDefense(by) | TargetAccuracy(by) | TargetEvasion(by) | TargetSpAttack(by) | TargetSpDefense(by) => {
            by < 0 && holds(stats, TWIST_BAND)
        }
        TargetSpeedDown | Tickle | Memento => holds(stats, TWIST_BAND),
        _ => false,
    }
}

/// How fast the holder's belly empties, in quarters of the usual (Tight
/// Belt: not at all; Stamina Band: half; Munch Belt: twice).
pub(crate) fn belly_quarters(stats: &Stats) -> i32 {
    match working(stats) {
        TIGHT_BELT => 0,
        STAMINA_BAND | DIET_RIBBON => 2,
        MUNCH_BELT => 8,
        _ => 4,
    }
}

/// The turns a Warp Scarf or a No-Slip Cap counts up to; its chance to
/// act each turn, in 1000ths for each turn counted (this project's own
/// figures: the game keeps a table of its own).
const TICK_TOP: i32 = 19;
const TICK_PER_MILLE: i32 = 5;

/// A turn counted for a Warp Scarf or a No-Slip Cap: whether it acts now
/// (its count starts again then).
pub(crate) fn ticks(count: &mut i32, rng: &mut super::DungeonRng) -> bool {
    *count = (*count + 1).min(TICK_TOP);
    let acts = rng.rand_int(1000) < *count * TICK_PER_MILLE;
    if acts {
        *count = 0;
    }
    acts
}

/// How much faster the holder recovers HP (Heal Ribbon: twice).
pub(crate) fn regen_times(stats: &Stats) -> i32 {
    if holds(stats, HEAL_RIBBON) { 2 } else { 1 }
}
