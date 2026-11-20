//! Abilities: a species has two, both at work in dungeons. Their numbers
//! follow the ability list of the extracted data; what they do is this
//! project's own reading of the well-known rules of the games, kept simple.
//! Those that change the damage are applied in [`super::combat`].

use super::combat::Stats;
use super::rng::DungeonRng;
use super::status::MoveEffect;
use super::weather;

pub(crate) const THICK_FAT: u8 = 2;
pub(crate) const SPEED_BOOST: u8 = 11;
pub(crate) const SWIFT_SWIM: u8 = 27;
pub(crate) const OBLIVIOUS: u8 = 41;
pub(crate) const CUTE_CHARM: u8 = 68;
pub(crate) const CHLOROPHYLL: u8 = 74;
pub(crate) const UNBURDEN: u8 = 84;
pub(crate) const SIMPLE: u8 = 97;
pub(crate) const POISON_HEAL: u8 = 114;
pub(crate) const SOUNDPROOF: u8 = 60;
pub(crate) const TRUANT: u8 = 42;
pub(crate) const KLUTZ: u8 = 111;
pub(crate) const EARLY_BIRD: u8 = 47;
pub(crate) const ANGER_POINT: u8 = 79;
pub(crate) const STEADFAST: u8 = 112;
pub(crate) const ARENA_TRAP: u8 = 5;
pub(crate) const SHADOW_TAG: u8 = 10;
pub(crate) const LIQUID_OOZE: u8 = 58;
pub(crate) const MAGIC_GUARD: u8 = 115;
pub(crate) const AIR_LOCK: u8 = 8;
pub(crate) const CLOUD_NINE: u8 = 45;
pub(crate) const DAMP: u8 = 21;
pub(crate) const QUICK_FEET: u8 = 109;
pub(crate) const STICKY_HOLD: u8 = 44;
pub(crate) const LIGHTNINGROD: u8 = 50;
pub(crate) const STORM_DRAIN: u8 = 122;
pub(crate) const RECKLESS: u8 = 92;
pub(crate) const IRON_FIST: u8 = 101;

pub(crate) const RAIN_DISH: u8 = 3;
pub(crate) const DRIZZLE: u8 = 4;
pub(crate) const INTIMIDATE: u8 = 6;
pub(crate) const HYPER_CUTTER: u8 = 9;
pub(crate) const BATTLE_ARMOR: u8 = 12;
pub(crate) const STURDY: u8 = 13;
pub(crate) const CLEAR_BODY: u8 = 15;
pub(crate) const TORRENT: u8 = 16;
pub(crate) const GUTS: u8 = 17;
pub(crate) const ROUGH_SKIN: u8 = 18;
pub(crate) const SHELL_ARMOR: u8 = 19;
pub(crate) const LIMBER: u8 = 22;
pub(crate) const WHITE_SMOKE: u8 = 24;
pub(crate) const OVERGROW: u8 = 26;
pub(crate) const SAND_STREAM: u8 = 28;
pub(crate) const SAND_VEIL: u8 = 29;
pub(crate) const KEEN_EYE: u8 = 30;
pub(crate) const INNER_FOCUS: u8 = 31;
pub(crate) const STATIC: u8 = 32;
pub(crate) const SHED_SKIN: u8 = 33;
pub(crate) const HUGE_POWER: u8 = 34;
pub(crate) const VOLT_ABSORB: u8 = 35;
pub(crate) const WATER_ABSORB: u8 = 36;
pub(crate) const SERENE_GRACE: u8 = 38;
pub(crate) const POISON_POINT: u8 = 39;
pub(crate) const HUSTLE: u8 = 48;
pub(crate) const DROUGHT: u8 = 49;
pub(crate) const COMPOUNDEYES: u8 = 51;
pub(crate) const MARVEL_SCALE: u8 = 52;
pub(crate) const WONDER_GUARD: u8 = 53;
pub(crate) const INSOMNIA: u8 = 54;
pub(crate) const LEVITATE: u8 = 55;
pub(crate) const EFFECT_SPORE: u8 = 61;
pub(crate) const FLAME_BODY: u8 = 62;
pub(crate) const OWN_TEMPO: u8 = 64;
pub(crate) const MAGMA_ARMOR: u8 = 65;
pub(crate) const WATER_VEIL: u8 = 66;
pub(crate) const SWARM: u8 = 67;
pub(crate) const IMMUNITY: u8 = 69;
pub(crate) const BLAZE: u8 = 70;
pub(crate) const FLASH_FIRE: u8 = 72;
pub(crate) const VITAL_SPIRIT: u8 = 73;
pub(crate) const PURE_POWER: u8 = 75;
pub(crate) const SHIELD_DUST: u8 = 76;
pub(crate) const ICE_BODY: u8 = 77;
pub(crate) const TINTED_LENS: u8 = 80;
pub(crate) const HYDRATION: u8 = 81;
pub(crate) const MOLD_BREAKER: u8 = 83;
pub(crate) const DRY_SKIN: u8 = 85;
pub(crate) const SCRAPPY: u8 = 87;
pub(crate) const SUPER_LUCK: u8 = 88;
pub(crate) const SOLAR_POWER: u8 = 90;
pub(crate) const SKILL_LINK: u8 = 91;
pub(crate) const SNIPER: u8 = 93;
pub(crate) const HEATPROOF: u8 = 95;
pub(crate) const DOWNLOAD: u8 = 96;
pub(crate) const TANGLED_FEET: u8 = 98;
pub(crate) const ADAPTABILITY: u8 = 99;
pub(crate) const TECHNICIAN: u8 = 100;
pub(crate) const MOTOR_DRIVE: u8 = 102;
pub(crate) const UNAWARE: u8 = 103;
pub(crate) const NO_GUARD: u8 = 106;
pub(crate) const NORMALIZE: u8 = 107;
pub(crate) const SOLID_ROCK: u8 = 108;
pub(crate) const FILTER: u8 = 110;
pub(crate) const SNOW_CLOAK: u8 = 119;
pub(crate) const SNOW_WARNING: u8 = 120;

/// The sound moves, the moves that hurt their user, the punches (by move
/// number).
pub(crate) const SOUND_MOVES: [u16; 13] = [217, 284, 83, 34, 84, 171, 293, 27, 241, 25, 26, 530, 446];
pub(crate) const RECOIL_MOVES: [u16; 10] = [116, 140, 203, 204, 206, 354, 441, 517, 518, 533];
pub(crate) const PUNCH_MOVES: [u16; 15] = [65, 75, 103, 126, 136, 246, 264, 292, 300, 322, 344, 347, 430, 500, 510];

/// Chance out of 100 that a contact ability (Static, Poison Point, Effect
/// Spore, Flame Body, Cute Charm) strikes back.
const CONTACT_CHANCE: i32 = 15;
/// Chance out of 100 that Shed Skin cures its bearer at the end of a turn.
const SHED_SKIN_CHANCE: i32 = 33;
/// Critical hit chance, in percent, Super Luck adds.
pub(crate) const SUPER_LUCK_CRIT: i32 = 12;
/// The highest move power Technician strengthens.
pub(crate) const TECHNICIAN_POWER: i32 = 6;
/// What Tinted Lens multiplies a not very effective move by.
pub(crate) const TINTED_LENS_BOOST: f64 = 1.5;
/// What Reckless and Iron Fist multiply the moves they favour by.
const FAVOURED_MOVE_BOOST: f64 = 1.25;
/// The share of the damage Rough Skin sends back, in quarters.
const ROUGH_SKIN_QUARTERS: i32 = 1;

const TYPE_NORMAL: u8 = 1;
const TYPE_FIRE: u8 = 2;
const TYPE_WATER: u8 = 3;
const TYPE_ELECTRIC: u8 = 5;
const TYPE_FIGHTING: u8 = 7;
const TYPE_GHOST: u8 = 14;

/// Whether a monster has an ability.
pub fn has(stats: &Stats, ability: u8) -> bool {
    ability != 0 && stats.abilities.contains(&ability)
}

/// Whether a defender's ability works against this attacker: Mold Breaker
/// ignores it.
pub(crate) fn shields(attacker: &Stats, defender: &Stats, ability: u8) -> bool {
    has(defender, ability) && !has(attacker, MOLD_BREAKER)
}

/// What the attacker's ability multiplies a move's damage by, with 8
/// fraction bits (`0x100` for nothing): Reckless favours the moves that
/// hurt their user, Iron Fist the punches.
pub(crate) fn power_boost(attacker: &Stats, move_id: u16) -> i32 {
    let favoured = [(RECKLESS, &RECOIL_MOVES[..]), (IRON_FIST, &PUNCH_MOVES[..])];
    let boost =
        favoured.iter().filter(|(ability, moves)| has(attacker, *ability) && moves.contains(&move_id)).fold(1.0, |boost, _| boost * FAVOURED_MOVE_BOOST);
    (boost * 256.0).round() as i32
}

/// At a quarter of its HP or less: the pinch abilities (Torrent, Overgrow,
/// Swarm, Blaze) wake up.
pub(crate) fn in_a_pinch(stats: &Stats) -> bool {
    stats.hp * 4 <= stats.max_hp
}

/// The type a move strikes with: always Normal under Normalize.
pub(crate) fn attack_type(attacker: &Stats, attack_type: u8) -> u8 {
    match has(attacker, NORMALIZE) {
        true => TYPE_NORMAL,
        false => attack_type,
    }
}

/// Whether Scrappy lets a Normal or Fighting move strike a Ghost.
pub(crate) fn scrappy(attacker: &Stats, attack_type: u8, defender_type: u8) -> bool {
    defender_type == TYPE_GHOST && matches!(attack_type, TYPE_NORMAL | TYPE_FIGHTING) && has(attacker, SCRAPPY)
}

/// The attacker's accuracy stage and the defender's evasion stage once
/// abilities have their say. No Guard on either side makes both neutral.
pub(crate) fn hit_stages(
    attacker: &Stats,
    defender: &Stats,
    (accuracy, evasion): (i32, i32),
    physical: bool,
    weather: u8,
    defender_confused: bool,
) -> (i32, i32) {
    if has(attacker, NO_GUARD) || shields(attacker, defender, NO_GUARD) {
        return (10, 10);
    }
    let accuracy = accuracy + if has(attacker, COMPOUNDEYES) { 2 } else { 0 };
    let in_hail = matches!(weather, weather::HAIL | weather::SNOW);
    let dodges = [
        (weather == weather::SANDSTORM && shields(attacker, defender, SAND_VEIL), 2),
        (in_hail && shields(attacker, defender, SNOW_CLOAK), 2),
        (defender_confused && shields(attacker, defender, TANGLED_FEET), 3),
        // Hustle trades its own aim for strength.
        (physical && has(attacker, HUSTLE), 2),
    ];
    let evasion = evasion + dodges.iter().filter(|(on, _)| *on).map(|(_, by)| by).sum::<i32>();
    (accuracy, evasion)
}

/// Whether an ability keeps an effect off its bearer: the status problems
/// it is immune to, the stat drops it refuses.
pub(crate) fn keeps_off(stats: &Stats, effect: MoveEffect) -> bool {
    use MoveEffect::*;
    let any = |abilities: &[u8]| abilities.iter().any(|&ability| has(stats, ability));
    // Clear Body and White Smoke refuse every stat drop.
    let guarded = any(&[CLEAR_BODY, WHITE_SMOKE]);
    match effect {
        Sleep | Yawn => any(&[INSOMNIA, VITAL_SPIRIT]),
        Paralyze => any(&[LIMBER]),
        Poison | BadlyPoison => any(&[IMMUNITY]),
        Burn => any(&[WATER_VEIL]),
        Freeze => any(&[MAGMA_ARMOR]),
        Confuse => any(&[OWN_TEMPO]),
        Infatuate => any(&[OBLIVIOUS]),
        Cringe => any(&[INNER_FOCUS]),
        TargetAttack(by) => by < 0 && (guarded || any(&[HYPER_CUTTER])),
        TargetAccuracy(by) => by < 0 && (guarded || any(&[KEEN_EYE])),
        TargetDefense(by) | TargetEvasion(by) | TargetSpAttack(by) | TargetSpDefense(by) => by < 0 && guarded,
        TargetSpeedDown | Tickle | Memento => guarded,
        _ => false,
    }
}

/// What a defender's ability does with a move of its type instead of
/// taking its damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Absorbed {
    /// Volt Absorb, Water Absorb, Dry Skin: the damage heals it.
    Heals,
    /// Motor Drive: it gets faster.
    SpeedsUp,
    /// Flash Fire, Lightningrod, Storm Drain: nothing happens.
    Nothing,
}

/// The ability, if any, that takes a move of this type in.
pub(crate) fn absorbs(attacker: &Stats, defender: &Stats, attack_type: u8) -> Option<Absorbed> {
    const TAKERS: [(u8, u8, Absorbed); 7] = [
        (TYPE_ELECTRIC, VOLT_ABSORB, Absorbed::Heals),
        (TYPE_ELECTRIC, MOTOR_DRIVE, Absorbed::SpeedsUp),
        (TYPE_ELECTRIC, LIGHTNINGROD, Absorbed::Nothing),
        (TYPE_WATER, WATER_ABSORB, Absorbed::Heals),
        (TYPE_WATER, DRY_SKIN, Absorbed::Heals),
        (TYPE_WATER, STORM_DRAIN, Absorbed::Nothing),
        (TYPE_FIRE, FLASH_FIRE, Absorbed::Nothing),
    ];
    TAKERS.iter().find(|&&(kind, ability, _)| kind == attack_type && shields(attacker, defender, ability)).map(|&(_, _, absorbed)| absorbed)
}

/// What a defender's ability does to an attacker that touched it (a
/// physical move): the effect inflicted on the attacker, if any. Each
/// contact ability rolls its own chance; the first that lands counts.
pub(crate) fn on_contact(rng: &mut DungeonRng, attacker: &Stats, defender: &Stats) -> Option<MoveEffect> {
    const CONTACT: [u8; 5] = [STATIC, POISON_POINT, EFFECT_SPORE, FLAME_BODY, CUTE_CHARM];
    let mut struck = None;
    for ability in CONTACT {
        if !shields(attacker, defender, ability) || rng.rand_int(100) >= CONTACT_CHANCE {
            continue;
        }
        let effect = match ability {
            STATIC => MoveEffect::Paralyze,
            POISON_POINT => MoveEffect::Poison,
            EFFECT_SPORE => [MoveEffect::Sleep, MoveEffect::Poison, MoveEffect::Paralyze][rng.rand_int(3) as usize],
            FLAME_BODY => MoveEffect::Burn,
            _ => MoveEffect::Infatuate,
        };
        struck = struck.or(Some(effect));
    }
    struck
}

/// Two attacks a turn: Swift Swim in the rain, Chlorophyll in the sun,
/// Unburden with empty hands.
pub(crate) fn attacks_twice(stats: &Stats, weather: u8) -> bool {
    match weather {
        weather::RAIN if has(stats, SWIFT_SWIM) => true,
        weather::SUNNY if has(stats, CHLOROPHYLL) => true,
        _ => stats.held == 0 && has(stats, UNBURDEN),
    }
}

/// What Rough Skin sends back to an attacker that touched its bearer: a
/// share of the damage it dealt.
pub(crate) fn rough_skin(attacker: &Stats, defender: &Stats, damage: i32) -> i32 {
    match shields(attacker, defender, ROUGH_SKIN) {
        true => (damage * ROUGH_SKIN_QUARTERS / 4).max(1),
        false => 0,
    }
}

/// The chance of a move's secondary effect, as abilities make it: Shield
/// Dust on the target stops it, Serene Grace doubles it.
pub(crate) fn secondary_chance(attacker: &Stats, defender: &Stats, chance: i32) -> i32 {
    match (shields(attacker, defender, SHIELD_DUST), has(attacker, SERENE_GRACE)) {
        (true, _) => 0,
        (false, true) => chance * 2,
        (false, false) => chance,
    }
}

/// The weather a monster's ability brings to the floor while it is there
/// (Drizzle, Drought, Sand Stream, Snow Warning).
pub(crate) fn brings_weather(stats: &Stats) -> Option<u8> {
    const BRINGERS: [(u8, u8); 4] = [(DRIZZLE, weather::RAIN), (DROUGHT, weather::SUNNY), (SAND_STREAM, weather::SANDSTORM), (SNOW_WARNING, weather::HAIL)];
    BRINGERS.iter().find(|&&(ability, _)| has(stats, ability)).map(|&(_, kind)| kind)
}

/// Whether a monster's ability spares it the weather's sting.
pub fn weathers(stats: &Stats, weather: u8) -> bool {
    match weather {
        weather::HAIL => has(stats, SNOW_CLOAK) || has(stats, ICE_BODY),
        weather::SANDSTORM => has(stats, SAND_VEIL),
        _ => false,
    }
}

/// What the weather does to a monster through its ability when it stings
/// (every ten turns): HP lost, negative for HP gained. The sun burns Solar
/// Power and Dry Skin; rain feeds Rain Dish and Dry Skin, hail Ice Body.
pub(crate) fn weather_damage(stats: &Stats, weather: u8, damage: i32) -> i32 {
    let any = |abilities: &[u8]| abilities.iter().any(|&ability| has(stats, ability));
    match weather {
        weather::SUNNY if any(&[SOLAR_POWER, DRY_SKIN]) => damage,
        weather::RAIN if any(&[RAIN_DISH, DRY_SKIN]) => -damage,
        weather::HAIL if any(&[ICE_BODY]) => -damage,
        _ => 0,
    }
}

/// Whether Hydration (in the rain) or Shed Skin (now and then) cures its
/// bearer's status problems at the end of this turn.
pub(crate) fn cures(rng: &mut DungeonRng, stats: &Stats, weather: u8) -> bool {
    if weather == weather::RAIN && has(stats, HYDRATION) {
        return true;
    }
    has(stats, SHED_SKIN) && rng.rand_int(100) < SHED_SKIN_CHANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(abilities: [u8; 2]) -> Stats {
        Stats {
            level: 5,
            max_hp: 40,
            hp: 40,
            attack: 10,
            defense: 10,
            sp_attack: 10,
            sp_defense: 10,
            types: (1, 0),
            abilities,
            held: 0,
            iq: 1,
            iq_group: 0,
            iq_skills: 0,
            female: false,
        }
    }

    #[test]
    fn mold_breaker_sets_the_defenders_ability_aside() {
        let (plain, breaker, absorbing) = (with([0, 0]), with([MOLD_BREAKER, 0]), with([0, VOLT_ABSORB]));
        assert_eq!(absorbs(&plain, &absorbing, TYPE_ELECTRIC), Some(Absorbed::Heals));
        assert_eq!(absorbs(&breaker, &absorbing, TYPE_ELECTRIC), None);
        assert_eq!(absorbs(&plain, &absorbing, TYPE_WATER), None);
        assert!(keeps_off(&with([INSOMNIA, 0]), MoveEffect::Sleep));
        assert!(!keeps_off(&with([INSOMNIA, 0]), MoveEffect::Poison));
        assert!(keeps_off(&with([HYPER_CUTTER, 0]), MoveEffect::TargetAttack(-1)));
        assert!(!keeps_off(&with([HYPER_CUTTER, 0]), MoveEffect::TargetAttack(1)));
        assert!(keeps_off(&with([CLEAR_BODY, 0]), MoveEffect::TargetDefense(-2)));
    }

    #[test]
    fn no_guard_and_compoundeyes_change_the_stages() {
        let (plain, eyes, guard) = (with([0, 0]), with([COMPOUNDEYES, 0]), with([NO_GUARD, 0]));
        assert_eq!(hit_stages(&eyes, &plain, (10, 12), false, weather::CLEAR, false), (12, 12));
        assert_eq!(hit_stages(&plain, &guard, (8, 14), false, weather::CLEAR, false), (10, 10));
        let veiled = with([SAND_VEIL, 0]);
        assert_eq!(hit_stages(&plain, &veiled, (10, 10), false, weather::SANDSTORM, false), (10, 12));
        assert_eq!(hit_stages(&plain, &veiled, (10, 10), false, weather::CLEAR, false), (10, 10));
    }

    #[test]
    fn favoured_moves_and_pinches() {
        assert_eq!(power_boost(&with([0, 0]), PUNCH_MOVES[0]), 0x100);
        assert!(power_boost(&with([IRON_FIST, 0]), PUNCH_MOVES[0]) > 0x100);
        assert_eq!(power_boost(&with([IRON_FIST, 0]), RECOIL_MOVES[0]), 0x100);
        let hurt = Stats { hp: 10, ..with([0, 0]) };
        assert!(in_a_pinch(&hurt) && !in_a_pinch(&with([0, 0])));
    }
}
