//! Items a monster holds (one each), and what they do while held. Item
//! numbers follow the item list of the extracted data. The stat items, the
//! Scope Lens and the Patsy Band are this project's own rules (see
//! `stat_boost`, `CRIT_BOOST`); the others follow the well-known
//! rules of the games.

use super::combat::Stats;
use super::status::MoveEffect;

pub(crate) const HEAL_RIBBON: u16 = 17;
pub(crate) const TWIST_BAND: u16 = 18;
pub(crate) const SCOPE_LENS: u16 = 19;
pub(crate) const PATSY_BAND: u16 = 20;
pub(crate) const PERSIM_BAND: u16 = 25;
pub(crate) const POWER_BAND: u16 = 26;
pub(crate) const PECHA_SCARF: u16 = 27;
pub(crate) const INSOMNISCOPE: u16 = 28;
pub(crate) const TIGHT_BELT: u16 = 30;
pub(crate) const DIET_RIBBON: u16 = 34;
pub(crate) const DEF_SCARF: u16 = 37;
pub(crate) const STAMINA_BAND: u16 = 38;
pub(crate) const SPECIAL_BAND: u16 = 40;
pub(crate) const ZINC_BAND: u16 = 41;
pub(crate) const DETECT_BAND: u16 = 42;
pub(crate) const LOCKON_SPECS: u16 = 49;
pub(crate) const MUNCH_BELT: u16 = 50;
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
/// blows, and a Patsy Band to the blows its holder takes.
pub(crate) const CRIT_BOOST: i32 = 12;

/// What a stat item adds to its holder's stat (0 attack, 1 special
/// attack, 2 defense, 3 special defense): a band or a scarf raises one
/// stat well, the Munch Belt both attacks a little.
pub(crate) fn stat_boost(stats: &Stats, stat: usize) -> i32 {
    const BAND: i32 = 8;
    const BELT: i32 = 5;
    match (working(stats), stat) {
        (POWER_BAND, 0) | (SPECIAL_BAND, 1) | (DEF_SCARF, 2) | (ZINC_BAND, 3) => BAND,
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

/// How much faster the holder recovers HP (Heal Ribbon: twice).
pub(crate) fn regen_times(stats: &Stats) -> i32 {
    if holds(stats, HEAL_RIBBON) { 2 } else { 1 }
}
