//! Types against types.

use super::*;

/// Types 0 (none) to 17; type 18, "neutral", never has a matchup.
pub const TYPE_COUNT: usize = 18;
/// The Fairy type, after the game's own (`rule_options::RuleOptions::fairy`):
/// not in the game's table, its matchups are the later games'.
pub const FAIRY: u8 = 19;

/// The types' names by number, as own files write them (0: none, 18: the
/// game's "neutral"), and their colours where they are shown.
pub const TYPE_NAMES: [&str; 20] = [
    "None", "Normal", "Fire", "Water", "Grass", "Electric", "Ice", "Fighting", "Poison", "Ground", "Flying", "Psychic", "Bug", "Rock", "Ghost", "Dragon",
    "Dark", "Steel", "Neutral", "Fairy",
];
pub const TYPE_COLOURS: [[u8; 3]; 20] = [
    [136, 136, 136],
    [168, 168, 120],
    [240, 128, 48],
    [104, 144, 240],
    [120, 200, 80],
    [248, 208, 48],
    [152, 216, 216],
    [192, 48, 40],
    [160, 64, 160],
    [224, 192, 104],
    [168, 144, 240],
    [248, 88, 136],
    [168, 184, 32],
    [184, 160, 56],
    [112, 88, 152],
    [112, 56, 248],
    [112, 88, 72],
    [184, 184, 208],
    [136, 136, 136],
    [238, 153, 172],
];

/// A type's name (none for a number past them).
pub fn type_name(kind: u8) -> &'static str {
    TYPE_NAMES.get(kind as usize).copied().unwrap_or("?")
}

/// The type of a name, as own files write it (`fairy`, `Electric`, `electric`).
pub fn type_named(said: &str) -> Option<u8> {
    let said = crate::own::plain(said);
    TYPE_NAMES.iter().position(|name| crate::own::plain(name) == said).map(|kind| kind as u8)
}

/// The Fairy type's matchups, attacking (`true`) or attacked, with that
/// other type (none: neutral), from the later games' chart.
fn fairy_matchup(attacking: bool, other: u8) -> Option<u8> {
    const FIRE: u8 = 2;
    const FIGHTING: u8 = 7;
    const POISON: u8 = 8;
    const BUG: u8 = 12;
    const DRAGON: u8 = 15;
    const DARK: u8 = 16;
    const STEEL: u8 = 17;
    Some(match (attacking, other) {
        (true, FIGHTING | DRAGON | DARK) => matchup::SUPER_EFFECTIVE,
        (true, FIRE | POISON | STEEL) => matchup::NOT_VERY_EFFECTIVE,
        (false, POISON | STEEL) => matchup::SUPER_EFFECTIVE,
        (false, FIGHTING | BUG | DARK) => matchup::NOT_VERY_EFFECTIVE,
        (false, DRAGON) => matchup::IMMUNE,
        _ => return None,
    })
}

/// How well a type fares against another, worst first.
pub mod matchup {
    pub const IMMUNE: u8 = 0;
    pub const NOT_VERY_EFFECTIVE: u8 = 1;
    pub const NEUTRAL: u8 = 2;
    pub const SUPER_EFFECTIVE: u8 = 3;
}

/// The type chart and its damage multipliers, as the extracted data gives
/// them.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TypeChart {
    /// `matchups[attack type][target type]`, see [`matchup`].
    pub matchups: [[u8; TYPE_COUNT]; TYPE_COUNT],
    /// Multiplier of each matchup, in 8-bit fixed point.
    pub multipliers: [i32; 4],
    /// The same for an attacker with the Erratic Player IQ skill.
    #[serde(default)]
    pub erratic: [i32; 4],
    /// The overall matchup against a dual-type target, from the matchups
    /// against each of its types.
    pub combinator: [[u8; 4]; 4],
    /// The game's natural logarithm table, ln(x) with 12 fraction bits by
    /// x (the damage formula's; empty without the game's data).
    pub ln_table: Vec<i16>,
}

impl TypeChart {
    /// Whether a move of this type cannot affect the target at all: Normal
    /// and Fighting moves on a Ghost, or an immunity of either type.
    pub fn immune(&self, attack: u8, target: &Stats) -> bool {
        const NORMAL: u8 = 1;
        const FIGHTING: u8 = 7;
        const GHOST: u8 = 14;
        let types = [target.types.0, target.types.1];
        (types.contains(&GHOST) && matches!(attack, NORMAL | FIGHTING)) || types.iter().any(|&kind| self.matchup(attack, kind) == matchup::IMMUNE)
    }

    /// How a move of this type fares overall against someone.
    pub fn overall(&self, attack: u8, target: &Stats) -> u8 {
        self.combinator[self.matchup(attack, target.types.0) as usize][self.matchup(attack, target.types.1) as usize]
    }

    pub(super) fn matchup(&self, attack: u8, target: u8) -> u8 {
        let fairy = match (attack, target) {
            (FAIRY, FAIRY) => Some(matchup::NEUTRAL),
            (FAIRY, other) => Some(fairy_matchup(true, other).unwrap_or(matchup::NEUTRAL)),
            (other, FAIRY) => Some(fairy_matchup(false, other).unwrap_or(matchup::NEUTRAL)),
            _ => None,
        };
        if let Some(value) = fairy {
            return value;
        }
        match self.matchups.get(attack as usize).and_then(|row| row.get(target as usize)) {
            Some(&value) => value,
            None => matchup::NEUTRAL,
        }
    }
}
