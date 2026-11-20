//! Where the game keeps its numbers: by pmdsky-debug symbol name (see
//! `crate::sym`), in the order EU, NA, JP.
//!
//! The few written as numbers are not named in pmdsky-debug: found in the
//! code that reads them (or, for NA and JP, as far from a named neighbour
//! as in the EU binary).

use super::*;
use crate::sym;

/// Addresses of the balance constants, in the order EU, NA, JP (from
/// pmdsky-debug: `ORAN_BERRY_HP_RESTORATION`, `SITRUS_BERRY_HP_RESTORATION`,
/// `POISON_DAMAGE`, `POISON_DAMAGE_COOLDOWN`, `SLEEP_TURN_RANGE`,
/// `PITFALL_TRAP_DAMAGE`).
pub(super) const ORAN_BERRY_HP: [u32; 3] = sym::ORAN_BERRY_HP_RESTORATION.address;

pub(super) const SITRUS_BERRY_HP: [u32; 3] = sym::SITRUS_BERRY_HP_RESTORATION.address;

pub(super) const POISON_DAMAGE: [u32; 3] = sym::POISON_DAMAGE.address;

pub(super) const POISON_COOLDOWN: [u32; 3] = sym::POISON_DAMAGE_COOLDOWN.address;

/// `BAD_POISON_DAMAGE_COOLDOWN`, `BAD_POISON_DAMAGE_TABLE` (30 values).
pub(super) const BAD_POISON_COOLDOWN: [u32; 3] = sym::BAD_POISON_DAMAGE_COOLDOWN.address;

pub(super) const BAD_POISON_DAMAGE: [u32; 3] = sym::BAD_POISON_DAMAGE_TABLE.address;

/// `LEECH_SEED_HP_DRAIN`, `LEECH_SEED_DAMAGE_COOLDOWN`,
/// `LEECH_SEED_TURN_RANGE`, `YAWN_TURN_RANGE`, `TAUNT_TURN_RANGE`.
pub(super) const LEECH_SEED_DRAIN: [u32; 3] = sym::LEECH_SEED_HP_DRAIN.address;

pub(super) const LEECH_SEED_COOLDOWN: [u32; 3] = sym::LEECH_SEED_DAMAGE_COOLDOWN.address;

pub(super) const LEECH_SEED_TURN_RANGE: [u32; 3] = sym::LEECH_SEED_TURN_RANGE.address;

pub(super) const YAWN_TURN_RANGE: [u32; 3] = sym::YAWN_TURN_RANGE.address;

pub(super) const TAUNT_TURN_RANGE: [u32; 3] = sym::TAUNT_TURN_RANGE.address;

/// `SURE_SHOT_TURN_RANGE`, `INFATUATED_TURN_RANGE`, `INVISIBLE_TURN_RANGE`,
/// `MAGIC_COAT_TURN_RANGE`, `SNATCH_TURN_RANGE`.
pub(super) const SURE_SHOT_TURN_RANGE: [u32; 3] = sym::SURE_SHOT_TURN_RANGE.address;

pub(super) const INFATUATED_TURN_RANGE: [u32; 3] = sym::INFATUATED_TURN_RANGE.address;

pub(super) const INVISIBLE_TURN_RANGE: [u32; 3] = sym::INVISIBLE_TURN_RANGE.address;

pub(super) const MAGIC_COAT_TURN_RANGE: [u32; 3] = sym::MAGIC_COAT_TURN_RANGE.address;

pub(super) const SNATCH_TURN_RANGE: [u32; 3] = sym::SNATCH_TURN_RANGE.address;

/// The chances of `status::ROM_SECONDARY_EFFECTS`, in its order.
pub(super) const SECONDARY_CHANCES: [[u32; 3]; 32] = [
    sym::WATERFALL_CRINGE_CHANCE.address,                   // WATERFALL_CRINGE_CHANCE
    sym::AURORA_BEAM_LOWER_ATTACK_CHANCE.address,           // AURORA_BEAM_LOWER_ATTACK_CHANCE
    sym::IRON_TAIL_LOWER_DEFENSE_CHANCE.address,            // IRON_TAIL_LOWER_DEFENSE_CHANCE
    sym::TWINEEDLE_POISON_CHANCE.address,                   // TWINEEDLE_POISON_CHANCE
    sym::CRUNCH_LOWER_DEFENSE_CHANCE.address,               // CRUNCH_LOWER_DEFENSE_CHANCE
    sym::EARTH_POWER_LOWER_SPECIAL_DEFENSE_CHANCE.address,  // EARTH_POWER_LOWER_SPECIAL_DEFENSE_CHANCE
    sym::POISON_TAIL_POISON_CHANCE.address,                 // POISON_TAIL_POISON_CHANCE
    sym::THUNDERSHOCK_PARALYZE_CHANCE.address,              // THUNDERSHOCK_PARALYZE_CHANCE
    sym::BOUNCE_PARALYZE_CHANCE.address,                    // BOUNCE_PARALYZE_CHANCE
    sym::HEADBUTT_CRINGE_CHANCE.address,                    // HEADBUTT_CRINGE_CHANCE
    sym::FIRE_FANG_CRINGE_CHANCE.address,                   // FIRE_FANG_CRINGE_CHANCE
    sym::SACRED_FIRE_BURN_CHANCE.address,                   // SACRED_FIRE_BURN_CHANCE
    sym::OCTAZOOKA_LOWER_ACCURACY_CHANCE.address,           // OCTAZOOKA_LOWER_ACCURACY_CHANCE
    sym::SEED_FLARE_LOWER_SPECIAL_DEFENSE_CHANCE.address,   // SEED_FLARE_LOWER_SPECIAL_DEFENSE_CHANCE
    sym::FLASH_CANNON_LOWER_SPECIAL_DEFENSE_CHANCE.address, // FLASH_CANNON_LOWER_SPECIAL_DEFENSE_CHANCE
    sym::ICE_FANG_FREEZE_CHANCE.address,                    // ICE_FANG_FREEZE_CHANCE
    sym::THUNDER_FANG_PARALYZE_CHANCE.address,              // THUNDER_FANG_PARALYZE_CHANCE
    sym::SKY_ATTACK_CRINGE_CHANCE.address,                  // SKY_ATTACK_CRINGE_CHANCE
    sym::ICE_FANG_CRINGE_CHANCE.address,                    // ICE_FANG_CRINGE_CHANCE
    sym::BLAZE_KICK_BURN_CHANCE.address,                    // BLAZE_KICK_BURN_CHANCE
    sym::SNORE_CRINGE_CHANCE.address,                       // SNORE_CRINGE_CHANCE
    sym::ACID_PSYCHIC_LOWER_DEFENSE_CHANCE.address,         // ACID_PSYCHIC_LOWER_DEFENSE_CHANCE
    sym::ACID_PSYCHIC_LOWER_DEFENSE_CHANCE.address,         // ACID_PSYCHIC_LOWER_DEFENSE_CHANCE
    sym::BUG_BUZZ_LOWER_SPECIAL_DEFENSE_CHANCE.address,     // BUG_BUZZ_LOWER_SPECIAL_DEFENSE_CHANCE
    sym::SHADOW_BALL_LOWER_SPECIAL_DEFENSE_CHANCE.address,  // SHADOW_BALL_LOWER_SPECIAL_DEFENSE_CHANCE
    sym::BUBBLE_LOWER_SPEED_CHANCE.address,                 // BUBBLE_LOWER_SPEED_CHANCE
    sym::POISON_FANG_POISON_CHANCE.address,                 // POISON_FANG_POISON_CHANCE
    sym::THUNDERBOLT_PARALYZE_CHANCE.address,               // THUNDERBOLT_PARALYZE_CHANCE
    sym::TWISTER_CRINGE_CHANCE.address,                     // TWISTER_CRINGE_CHANCE
    sym::FAKE_OUT_CRINGE_CHANCE.address,                    // FAKE_OUT_CRINGE_CHANCE
    sym::THUNDER_FANG_CRINGE_CHANCE.address,                // THUNDER_FANG_CRINGE_CHANCE
    sym::FLARE_BLITZ_BURN_CHANCE.address,                   // FLARE_BLITZ_BURN_CHANCE
];

pub(super) const TRI_ATTACK_CHANCE: [u32; 3] = sym::TRI_ATTACK_STATUS_CHANCE.address;

/// `SPORT_CONDITION_TURN_RANGE`, `SLEEPLESSNESS_TURN_RANGE`.
pub(super) const SPORT_TURN_RANGE: [u32; 3] = sym::SPORT_CONDITION_TURN_RANGE.address;

pub(super) const SLEEPLESS_TURN_RANGE: [u32; 3] = sym::SLEEPLESSNESS_TURN_RANGE.address;

/// `MAGNET_RISE_TURN_RANGE`, `EMBARGO_TURN_RANGE`.
pub(super) const MAGNET_RISE_TURN_RANGE: [u32; 3] = sym::MAGNET_RISE_TURN_RANGE.address;

pub(super) const EMBARGO_TURN_RANGE: [u32; 3] = sym::EMBARGO_TURN_RANGE.address;

/// `ENCORE_TURN_RANGE`.
pub(super) const ENCORE_TURN_RANGE: [u32; 3] = sym::ENCORE_TURN_RANGE.address;

/// `CROSS_EYED_TURN_RANGE`.
pub(super) const CROSS_EYED_TURN_RANGE: [u32; 3] = sym::CROSS_EYED_TURN_RANGE.address;

/// `NIGHTMARE_TURN_RANGE`, `HEAL_BLOCK_TURN_RANGE`.
pub(super) const NIGHTMARE_TURN_RANGE: [u32; 3] = sym::NIGHTMARE_TURN_RANGE.address;

pub(super) const HEAL_BLOCK_TURN_RANGE: [u32; 3] = sym::HEAL_BLOCK_TURN_RANGE.address;

/// `METRONOME_TABLE`: 168 `wildcard_move_desc` of 8 bytes, the move first.
pub(super) const METRONOME_TABLE: [u32; 3] = sym::METRONOME_TABLE.address;

/// `SET_REFLECT_STATUS_TURN_RANGE`, `MIRROR_COAT_TURN_RANGE`,
/// `PERISH_SONG_TURN_RANGE`.
pub(super) const SET_REFLECT_STATUS_TURN_RANGE: [u32; 3] = sym::SET_REFLECT_STATUS_TURN_RANGE.address;

pub(super) const MIRROR_COAT_TURN_RANGE: [u32; 3] = sym::MIRROR_COAT_TURN_RANGE.address;

pub(super) const PERISH_SONG_TURN_RANGE: [u32; 3] = sym::PERISH_SONG_TURN_RANGE.address;

/// `SET_DAMAGE_TURN_RANGE`, `SET_DAMAGE_STATUS_DAMAGE`,
/// `MIRROR_MOVE_TURN_RANGE`, `IMPRISON_TURN_RANGE`.
pub(super) const SET_DAMAGE_TURN_RANGE: [u32; 3] = sym::SET_DAMAGE_TURN_RANGE.address;

pub(super) const SET_DAMAGE_STATUS_DAMAGE: [u32; 3] = sym::SET_DAMAGE_STATUS_DAMAGE.address;

pub(super) const MIRROR_MOVE_TURN_RANGE: [u32; 3] = sym::MIRROR_MOVE_TURN_RANGE.address;

pub(super) const IMPRISON_TURN_RANGE: [u32; 3] = sym::IMPRISON_TURN_RANGE.address;

/// `WRAP_TURN_RANGE`.
pub(super) const WRAP_TURN_RANGE: [u32; 3] = sym::WRAP_TURN_RANGE.address;

/// The tables of `DoMoveReturn`, `DoMoveFrustration`, `DoMoveMagnitude`
/// and `DoMoveVacuumCut` (EU, read in their code; the others as far from
/// `SET_DAMAGE_STATUS_DAMAGE` as it).
pub(super) const RETURN_DAMAGE: [u32; 3] = [0x22C_5564, 0x22C_5564 - 0x958, 0x22C_5564 + 0xD90];

pub(super) const FRUSTRATION_DAMAGE: [u32; 3] = [0x22C_5594, 0x22C_5594 - 0x958, 0x22C_5594 + 0xD90];

pub(super) const MAGNITUDE_DAMAGE: [u32; 3] = [0x22C_527C, 0x22C_527C - 0x958, 0x22C_527C + 0xD90];

pub(super) const VACUUM_CUT_DAMAGE: [u32; 3] = [0x22C_4EC4, 0x22C_4EC4 - 0x958, 0x22C_4EC4 + 0xD90];

/// What `EndSleepClassStatus` takes from a monster waking from a nightmare.
pub(super) const NIGHTMARE_DAMAGE: [u32; 3] = [0x22C_4F00, 0x22C_4F00 - 0x958, 0x22C_4F00 + 0xD90];

/// What it heals, waking from Rest (`EndSleepClassStatus`).
pub(super) const NAPPING_HEAL: [u32; 3] = [0x22C_4EF8, 0x22C_4EF8 - 0x958, 0x22C_4EF8 + 0xD90];

/// `DESTINY_BOND_TURN_RANGE`.
pub(super) const DESTINY_BOND_TURN_RANGE: [u32; 3] = sym::DESTINY_BOND_TURN_RANGE.address;

/// The turn range `TryTransform` reads (EU 0x22C5154, not named in
/// pmdsky-debug).
pub(super) const TRANSFORM_TURN_RANGE: [u32; 3] = [0x22C_5154, 0x22C_5154 - 0x958, 0x22C_5154 + 0xD90];

/// `IQ_GUMMI_GAIN_TABLE`, `GUMMI_BELLY_RESTORE_TABLE`, `WONDER_GUMMI_IQ_GAIN`
/// (arm9) and the chance `ApplyGrimyFoodEffect` reads (overlay 10).
pub(super) const IQ_GUMMI_GAIN_TABLE: [u32; 3] = sym::IQ_GUMMI_GAIN_TABLE.address;

pub(super) const GUMMI_BELLY_RESTORE_TABLE: [u32; 3] = sym::GUMMI_BELLY_RESTORE_TABLE.address;

pub(super) const WONDER_GUMMI_IQ_GAIN: [u32; 3] = sym::WONDER_GUMMI_IQ_GAIN.address;

pub(super) const GRIMY_FOOD_CHANCE: [u32; 3] = [0x22C_4FD8, 0x22C_4FD8 - 0x958, 0x22C_4FD8 + 0xD90];

/// What `ApplyGummiBoostsDungeonMode` reads for a stat boost (overlay 10),
/// and `GUMMI_STAT_BOOST` (arm9).
pub(super) const GUMMI_BOOST_CHANCE: [u32; 3] = [0x22C_4E24, 0x22C_4E24 - 0x958, 0x22C_4E24 + 0xD90];

pub(super) const GUMMI_ALL_STATS_ODDS: [u32; 3] = [0x22C_4F3C, 0x22C_4F3C - 0x958, 0x22C_4F3C + 0xD90];

pub(super) const GUMMI_STAT_BOOST: [u32; 3] = sym::GUMMI_STAT_BOOST.address;

/// `EXP_ELITE_EXP_BOOST`.
pub(super) const EXP_ELITE_EXP_BOOST: [u32; 3] = sym::EXP_ELITE_EXP_BOOST.address;

/// `FACADE_DAMAGE_MULTIPLIER` and the tables of `DoMoveReversal`,
/// `DoMoveWaterSpout`, `DoMoveEruption`, `DoMoveDamageHpDependent`.
pub(super) const FACADE_DAMAGE_MULTIPLIER: [u32; 3] = sym::FACADE_DAMAGE_MULTIPLIER.address;

pub(super) const REVERSAL_DAMAGE_MULT_TABLE: [u32; 3] = sym::REVERSAL_DAMAGE_MULT_TABLE.address;

pub(super) const WATER_SPOUT_DAMAGE_MULT_TABLE: [u32; 3] = sym::WATER_SPOUT_DAMAGE_MULT_TABLE.address;

pub(super) const WRING_OUT_DAMAGE_MULT_TABLE: [u32; 3] = sym::WRING_OUT_DAMAGE_MULT_TABLE.address;

pub(super) const ERUPTION_DAMAGE_MULT_TABLE: [u32; 3] = sym::ERUPTION_DAMAGE_MULT_TABLE.address;

/// `WHIRLPOOL_CONSTRICTION_CHANCE`, `CONSTRICTION_TURN_RANGE`,
/// `CONSTRICTION_DAMAGE` and the turns between its hurts (read where the
/// status hurts).
pub(super) const CONSTRICTION_CHANCE: [u32; 3] = sym::WHIRLPOOL_CONSTRICTION_CHANCE.address;

pub(super) const CONSTRICTION_TURN_RANGE: [u32; 3] = sym::CONSTRICTION_TURN_RANGE.address;

pub(super) const CONSTRICTION_DAMAGE: [u32; 3] = sym::CONSTRICTION_DAMAGE.address;

pub(super) const CONSTRICTION_COOLDOWN: [u32; 3] = [0x22C_4DAC, 0x22C_4DAC - 0x958, 0x22C_4DAC + 0xD90];

pub(super) const LAST_RESORT_DAMAGE_MULT_TABLE: [u32; 3] = sym::LAST_RESORT_DAMAGE_MULT_TABLE.address;

pub(super) const NATURAL_GIFT_ITEM_TABLE: [u32; 3] = sym::NATURAL_GIFT_ITEM_TABLE.address;

pub(super) const SPEED_BOOST_TURNS: [u32; 3] = sym::SPEED_BOOST_TURNS.address;

pub(super) const WISE_HEALER_PERCENTAGE: [u32; 3] = sym::WISE_HEALER_PERCENTAGE.address;

pub(super) const INTIMIDATOR_ACTIVATION_CHANCE: [u32; 3] = sym::INTIMIDATOR_ACTIVATION_CHANCE.address;

pub(super) const POWER_PITCHER_DAMAGE_MULTIPLIER: [u32; 3] = sym::POWER_PITCHER_DAMAGE_MULTIPLIER.address;

pub(super) const ARENA_TRAP_ACTIVATION_CHANCE: [u32; 3] = sym::ARENA_TRAP_ACTIVATION_CHANCE.address;

pub(super) const SHADOW_TAG_ACTIVATION_CHANCE: [u32; 3] = sym::SHADOW_TAG_ACTIVATION_CHANCE.address;

pub(super) const LINK_BELLY: [u32; 3] = [0x22C_53B4, 0x22C_53B4 - 0x958, 0x22C_53B4 + 0xD90];

pub(super) const EXCLUSIVE_REACTION_CHANCE: [u32; 3] = [0x22C_4F0C, 0x22C_4F0C - 0x958, 0x22C_4F0C + 0xD90];

pub(super) const EXCLUSIVE_ITEM_EXP_BOOST: [u32; 3] = sym::EXCLUSIVE_ITEM_EXP_BOOST.address;

pub(super) const SECRET_POWER_EFFECT_CHANCE: [u32; 3] = sym::SECRET_POWER_EFFECT_CHANCE.address;

/// How long Hyper Beam's user rests (unnamed: what `UseMove` reads).
pub(super) const RECHARGE_TURN_RANGE: [u32; 3] = [0x22C_506C, 0x22C_506C - 0x958, 0x22C_506C + 0xD90];

/// `ROLLOUT_DAMAGE_MULT_TABLE` (overlay 29).
pub(super) const ROLLOUT_DAMAGE_MULT_TABLE: [u32; 3] = sym::ROLLOUT_DAMAGE_MULT_TABLE.address;

pub(super) const VITAL_THROW_TURN_RANGE: [u32; 3] = sym::VITAL_THROW_TURN_RANGE.address;

/// `DECOY_TURN_RANGE`, `CONVERSION2_TURN_RANGE`.
pub(super) const DECOY_TURN_RANGE: [u32; 3] = sym::DECOY_TURN_RANGE.address;

pub(super) const CONVERSION2_TURN_RANGE: [u32; 3] = sym::CONVERSION2_TURN_RANGE.address;

/// `CONVERSION2_TYPE_TABLE`, the table `GetConversion2ConvertToType`
/// reads (arm9).
pub(super) const CONVERSION2_TYPES: [u32; 3] = sym::CONVERSION2_TYPE_TABLE.address;

/// `ME_FIRST_MULTIPLIER` (1.5, 8 fraction bits).
pub(super) const ME_FIRST_MULTIPLIER: [u32; 3] = sym::ME_FIRST_MULTIPLIER.address;

pub(super) const SLEEP_TURN_RANGE: [u32; 3] = sym::SLEEP_TURN_RANGE.address;

pub(super) const PITFALL_TRAP_DAMAGE: [u32; 3] = sym::PITFALL_TRAP_DAMAGE.address;

pub(super) const BELLY_LOST_PER_TURN: [u32; 3] = sym::BELLY_LOST_PER_TURN.address;

/// `OFFENSIVE_STAT_STAGE_MULTIPLIERS` and `DEFENSIVE_STAT_STAGE_MULTIPLIERS`.
pub(super) const OFFENSIVE_STAGES: [u32; 3] = sym::OFFENSIVE_STAT_STAGE_MULTIPLIERS.address;

pub(super) const DEFENSIVE_STAGES: [u32; 3] = sym::DEFENSIVE_STAT_STAGE_MULTIPLIERS.address;

/// `MALE_ACCURACY_STAGE_MULTIPLIERS` and `MALE_EVASION_STAGE_MULTIPLIERS`.
pub(super) const ACCURACY_STAGES: [u32; 3] = sym::MALE_ACCURACY_STAGE_MULTIPLIERS.address;

pub(super) const EVASION_STAGES: [u32; 3] = sym::MALE_EVASION_STAGE_MULTIPLIERS.address;

/// `BURN_DAMAGE`, `BURN_DAMAGE_COOLDOWN`, `BURN_DAMAGE_MULTIPLIER` and the
/// `*_TURN_RANGE` pairs of each status.
/// The `*_DAMAGE_MULTIPLIER` of each charged move, in the order of
/// `CHARGED_MOVES`.
pub(super) const CHARGED_MULTIPLIERS: [[u32; 3]; CHARGED_MOVES.len()] = [
    sym::SOLARBEAM_DAMAGE_MULTIPLIER.address,
    sym::SKY_ATTACK_DAMAGE_MULTIPLIER.address,
    sym::RAZOR_WIND_DAMAGE_MULTIPLIER.address,
    sym::FOCUS_PUNCH_DAMAGE_MULTIPLIER.address,
    sym::SKULL_BASH_DAMAGE_MULTIPLIER.address,
    sym::FLY_DAMAGE_MULTIPLIER.address,
    sym::DIG_DAMAGE_MULTIPLIER.address,
    sym::DIVE_DAMAGE_MULTIPLIER.address,
    sym::BOUNCE_DAMAGE_MULTIPLIER.address,
    sym::SHADOW_FORCE_DAMAGE_MULTIPLIER.address,
];

pub(super) const SPEED_BOOST_TURN_RANGE: [u32; 3] = sym::SPEED_BOOST_TURN_RANGE.address;

pub(super) const SPEED_LOWER_TURN_RANGE: [u32; 3] = sym::SPEED_LOWER_TURN_RANGE.address;

/// `METEOR_MASH_BOOST_ATTACK_CHANCE`, `METAL_CLAW_BOOST_ATTACK_CHANCE`,
/// `STEEL_WING_BOOST_DEFENSE_CHANCE`, `SILVER_WIND_BOOST_CHANCE`.
pub(super) const METEOR_MASH_CHANCE: [u32; 3] = sym::METEOR_MASH_BOOST_ATTACK_CHANCE.address;

pub(super) const METAL_CLAW_CHANCE: [u32; 3] = sym::METAL_CLAW_BOOST_ATTACK_CHANCE.address;

pub(super) const STEEL_WING_CHANCE: [u32; 3] = sym::STEEL_WING_BOOST_DEFENSE_CHANCE.address;

pub(super) const SILVER_WIND_CHANCE: [u32; 3] = sym::SILVER_WIND_BOOST_CHANCE.address;

/// The `*_HP_RESTORATION_TABLE`s of Morning Sun, Synthesis, Moonlight and
/// Roost (indexed by weather; clear comes first), then
/// `HEAL_ORDER_HP_RESTORATION`.
pub(super) const HEAL_AMOUNTS: [[u32; 3]; 5] = [
    sym::MORNING_SUN_HP_RESTORATION_TABLE.address,
    sym::SYNTHESIS_HP_RESTORATION_TABLE.address,
    sym::MOONLIGHT_HP_RESTORATION_TABLE.address,
    sym::ROOST_HP_RESTORATION_TABLE.address,
    sym::HEAL_ORDER_HP_RESTORATION.address,
];

/// `MIST_BALL_LOWER_SPECIAL_ATTACK_CHANCE`,
/// `CHARGE_BEAM_BOOST_SPECIAL_ATTACK_CHANCE`,
/// `LUSTER_PURGE_LOWER_SPECIAL_DEFENSE_CHANCE`.
pub(super) const MIST_BALL_CHANCE: [u32; 3] = sym::MIST_BALL_LOWER_SPECIAL_ATTACK_CHANCE.address;

pub(super) const CHARGE_BEAM_CHANCE: [u32; 3] = sym::CHARGE_BEAM_BOOST_SPECIAL_ATTACK_CHANCE.address;

pub(super) const LUSTER_PURGE_CHANCE: [u32; 3] = sym::LUSTER_PURGE_LOWER_SPECIAL_DEFENSE_CHANCE.address;

pub(super) const REFLECT_TURN_RANGE: [u32; 3] = sym::REFLECT_TURN_RANGE.address;

pub(super) const LIGHT_SCREEN_TURN_RANGE: [u32; 3] = sym::LIGHT_SCREEN_TURN_RANGE.address;

/// `WEATHER_MOVE_TURN_COUNT`, `BAD_WEATHER_DAMAGE`, `WEATHER_BALL_TYPE_TABLE`
/// and `WEATHER_BALL_DAMAGE_MULT_TABLE`.
pub(super) const WEATHER_MOVE_TURNS: [u32; 3] = sym::WEATHER_MOVE_TURN_COUNT.address;

pub(super) const BAD_WEATHER_DAMAGE: [u32; 3] = sym::BAD_WEATHER_DAMAGE.address;

pub(super) const AI_REGULAR_ATTACK_WEIGHTS: [u32; 3] = sym::AI_REGULAR_ATTACK_WEIGHTS.address;

pub(super) const ORAN_BERRY_FULL_HP_BOOST: [u32; 3] = sym::ORAN_BERRY_FULL_HP_BOOST.address;

pub(super) const SITRUS_BERRY_FULL_HP_BOOST: [u32; 3] = sym::SITRUS_BERRY_FULL_HP_BOOST.address;

pub(super) const LIFE_SEED_HP_BOOST: [u32; 3] = sym::LIFE_SEED_HP_BOOST.address;

pub(super) const OREN_BERRY_DAMAGE: [u32; 3] = sym::OREN_BERRY_DAMAGE.address;

/// Protein, Calcium, Iron, Zinc.
pub(super) const VITAMIN_BOOSTS: [[u32; 3]; 4] =
    [sym::PROTEIN_STAT_BOOST.address, sym::CALCIUM_STAT_BOOST.address, sym::IRON_STAT_BOOST.address, sym::ZINC_STAT_BOOST.address];

pub(super) const THROWN_ITEM_HIT_CHANCE: [u32; 3] = sym::THROWN_ITEM_HIT_CHANCE.address;

/// By item id, from the Stick (1) to the Rare Fossil (10): `STICK_POWER`,
/// `IRON_THORN_POWER`, `SILVER_SPIKE_POWER`, `GOLD_FANG_POWER`,
/// `CACNEA_SPIKE_POWER`, `CORSOLA_TWIG_POWER`, `GRAVELEROCK_DAMAGE`,
/// `GEO_PEBBLE_DAMAGE`, `GOLD_THORN_POWER`, `RARE_FOSSIL_DAMAGE`.
pub(super) const THROWN_POWERS: [[u32; 3]; 10] = [
    sym::STICK_POWER.address,
    sym::IRON_THORN_POWER.address,
    sym::SILVER_SPIKE_POWER.address,
    sym::GOLD_FANG_POWER.address,
    sym::CACNEA_SPIKE_POWER.address,
    sym::CORSOLA_TWIG_POWER.address,
    sym::GRAVELEROCK_DAMAGE.address,
    sym::GEO_PEBBLE_DAMAGE.address,
    sym::GOLD_THORN_POWER.address,
    sym::RARE_FOSSIL_DAMAGE.address,
];

pub(super) const EATEN_BLAST_SEED_DAMAGE: [u32; 3] = sym::EATEN_BLAST_SEED_DAMAGE_NORMAL.address;

pub(super) const THROWN_BLAST_SEED_DAMAGE: [u32; 3] = sym::THROWN_BLAST_SEED_DAMAGE_NORMAL.address;

pub(super) const SPAWN_COOLDOWN: [u32; 3] = sym::SPAWN_COOLDOWN.address;
pub(super) const SPAWN_COOLDOWN_THIEF_ALERT: [u32; 3] = sym::SPAWN_COOLDOWN_THIEF_ALERT.address;
pub(super) const FEMALE_ACCURACY_STAGES: [u32; 3] = sym::FEMALE_ACCURACY_STAGE_MULTIPLIERS.address;
pub(super) const FEMALE_EVASION_STAGES: [u32; 3] = sym::FEMALE_EVASION_STAGE_MULTIPLIERS.address;
/// The jobs' tables, in the arm9.
pub(super) const RANK_UP_TABLE: [u32; 3] = sym::RANK_UP_TABLE.address;
pub(super) const MISSION_RANK_POINTS: [u32; 3] = sym::MISSION_RANK_POINTS.address;
pub(super) const REWARD_WEIGHTS: [u32; 3] = sym::DEFAULT_MISSION_REWARD_TYPE_WEIGHTS.address;
pub(super) const OUTLAW_REWARD_WEIGHTS: [u32; 3] = sym::OUTLAW_MISSION_REWARD_TYPE_WEIGHTS.address;
pub(super) const MISSION_FLOOR_RANKS: [u32; 3] = sym::MISSION_FLOOR_RANKS_PTRS.address;

pub(super) const SPAWN_CAP_NO_MONSTER_HOUSE: [u32; 3] = sym::SPAWN_CAP_NO_MONSTER_HOUSE.address;

pub(super) const SPAWN_CAP_WITH_MONSTER_HOUSE: [u32; 3] = sym::SPAWN_CAP_WITH_MONSTER_HOUSE.address;

pub(super) const WEATHER_BALL_TYPES: [u32; 3] = sym::WEATHER_BALL_TYPE_TABLE.address;

pub(super) const WEATHER_BALL_MULTIPLIERS: [u32; 3] = sym::WEATHER_BALL_DAMAGE_MULT_TABLE.address;

/// The damage of a monster blown into an obstacle and the Endure turn
/// range (both unnamed in pmdsky-debug), `REST_TURN_RANGE`,
/// `CURSE_TURN_RANGE`, `CURSE_DAMAGE_COOLDOWN`.
pub(super) const BLOWBACK_DAMAGE: [u32; 3] = [0x22C_4EC8, 0x22C_4570, 0x22C_5C58];

pub(super) const ENDURE_TURN_RANGE: [u32; 3] = [0x22C_514C, 0x22C_47F4, 0x22C_5EDC];

pub(super) const REST_TURN_RANGE: [u32; 3] = sym::REST_TURN_RANGE.address;

pub(super) const CURSE_TURN_RANGE: [u32; 3] = sym::CURSE_TURN_RANGE.address;

pub(super) const CURSE_COOLDOWN: [u32; 3] = sym::CURSE_DAMAGE_COOLDOWN.address;

/// `WISH_BONUS_REGEN`, `WISH_TURN_RANGE`, `INGRAIN_BONUS_REGEN`,
/// `INGRAIN_BONUS_REGEN_COOLDOWN`, `INGRAIN_TURN_RANGE`,
/// `AQUA_RING_BONUS_REGEN`, `AQUA_RING_HEALING_COUNTDOWN`,
/// `AQUA_RING_TURN_RANGE`.
pub(super) const WISH_BONUS_REGEN: [u32; 3] = sym::WISH_BONUS_REGEN.address;

pub(super) const WISH_TURN_RANGE: [u32; 3] = sym::WISH_TURN_RANGE.address;

pub(super) const INGRAIN_REGEN: [u32; 3] = sym::INGRAIN_BONUS_REGEN.address;

pub(super) const INGRAIN_COOLDOWN: [u32; 3] = sym::INGRAIN_BONUS_REGEN_COOLDOWN.address;

pub(super) const INGRAIN_TURN_RANGE: [u32; 3] = sym::INGRAIN_TURN_RANGE.address;

pub(super) const AQUA_RING_REGEN: [u32; 3] = sym::AQUA_RING_BONUS_REGEN.address;

pub(super) const AQUA_RING_COOLDOWN: [u32; 3] = sym::AQUA_RING_HEALING_COUNTDOWN.address;

pub(super) const AQUA_RING_TURN_RANGE: [u32; 3] = sym::AQUA_RING_TURN_RANGE.address;

/// The damage of the Chestnut Trap, of the Spiked Tile and of Stealth Rock
/// (unnamed in pmdsky-debug), and `EXPLOSION_FIXED_DAMAGES` (overlay 29).
pub(super) const CHESTNUT_DAMAGE: [u32; 3] = [0x22C_4F20, 0x22C_45C8, 0x22C_5CB0];

pub(super) const SPIKED_TILE_DAMAGE: [u32; 3] = [0x22C_4D70, 0x22C_4418, 0x22C_5B00];

pub(super) const STEALTH_ROCK_DAMAGE: [u32; 3] = [0x22C_4F24, 0x22C_45CC, 0x22C_5CB4];

pub(super) const EXPLOSION_DAMAGES: [u32; 3] = sym::EXPLOSION_FIXED_DAMAGES.address;

pub(super) const PROTECT_TURN_RANGE: [u32; 3] = sym::PROTECT_TURN_RANGE.address;

pub(super) const SAFEGUARD_TURN_RANGE: [u32; 3] = sym::SAFEGUARD_TURN_RANGE.address;

pub(super) const MIST_TURN_RANGE: [u32; 3] = sym::MIST_TURN_RANGE.address;

/// What `TryInflictLuckyChantStatus` reads (EU; the others as far from
/// Safeguard's).
pub(super) const LUCKY_CHANT_TURN_RANGE: [u32; 3] = [0x22C_50C0, 0x22C_4768, 0x22C_5E50];

/// `SONICBOOM_FIXED_DAMAGE` and `DRAGON_RAGE_FIXED_DAMAGE`.
pub(super) const SONICBOOM_DAMAGE: [u32; 3] = sym::SONICBOOM_FIXED_DAMAGE.address;

pub(super) const DRAGON_RAGE_DAMAGE: [u32; 3] = sym::DRAGON_RAGE_FIXED_DAMAGE.address;

pub(super) const BURN_DAMAGE: [u32; 3] = sym::BURN_DAMAGE.address;

pub(super) const BURN_COOLDOWN: [u32; 3] = sym::BURN_DAMAGE_COOLDOWN.address;

pub(super) const BURN_MULTIPLIER: [u32; 3] = sym::BURN_DAMAGE_MULTIPLIER.address;

pub(super) const FROZEN_TURN_RANGE: [u32; 3] = sym::FROZEN_TURN_RANGE.address;

pub(super) const BURN_TURN_RANGE: [u32; 3] = sym::BURN_TURN_RANGE.address;

pub(super) const CRINGE_TURN_RANGE: [u32; 3] = sym::CRINGE_TURN_RANGE.address;

pub(super) const CONFUSED_TURN_RANGE: [u32; 3] = sym::CONFUSED_TURN_RANGE.address;

pub(super) const PARALYSIS_TURN_RANGE: [u32; 3] = sym::PARALYSIS_TURN_RANGE.address;

/// `POWER_BAND_STAT_BOOST`, `SPECIAL_BAND_STAT_BOOST`,
/// `DEF_SCARF_STAT_BOOST`, `ZINC_BAND_STAT_BOOST`, `MUNCH_BELT_STAT_BOOST`
/// (in the ARM9 binary).
pub(super) const HELD_BOOSTS: [[u32; 3]; 5] = [
    sym::POWER_BAND_STAT_BOOST.address,
    sym::SPECIAL_BAND_STAT_BOOST.address,
    sym::DEF_SCARF_STAT_BOOST.address,
    sym::ZINC_BAND_STAT_BOOST.address,
    sym::MUNCH_BELT_STAT_BOOST.address,
];

pub(super) const SCOPE_LENS_CRIT: [u32; 3] = sym::SCOPE_LENS_CRIT_RATE_BOOST.address;

pub(super) const DETECT_BAND_DROP: [u32; 3] = sym::DETECT_BAND_MOVE_ACCURACY_DROP.address;

pub(super) const QUICK_DODGER_DROP: [u32; 3] = sym::QUICK_DODGER_MOVE_ACCURACY_DROP.address;

pub(super) const TYPE_ADVANTAGE_CRIT: [u32; 3] = sym::TYPE_ADVANTAGE_MASTER_CRIT_RATE.address;

/// The IQ tables of the ARM9 binary: `IQ_SKILLS` (69 32-bit values),
/// `IQ_GROUP_SKILLS` (16 groups of 25 bytes), `IQ_SKILL_RESTRICTIONS` (69
/// 16-bit values), `IQ_GUMMI_GAIN_TABLE` and `GUMMI_BELLY_RESTORE_TABLE`
/// (18 by 18 16-bit values), `WONDER_GUMMI_IQ_GAIN`.
pub(super) const IQ_SKILLS: [u32; 3] = sym::IQ_SKILLS.address;

pub(super) const IQ_GROUP_SKILLS: [u32; 3] = sym::IQ_GROUP_SKILLS.address;

pub(super) const IQ_SKILL_RESTRICTIONS: [u32; 3] = sym::IQ_SKILL_RESTRICTIONS.address;

pub(super) const IQ_GUMMI_GAIN: [u32; 3] = sym::IQ_GUMMI_GAIN_TABLE.address;

pub(super) const GUMMI_BELLY_RESTORE: [u32; 3] = sym::GUMMI_BELLY_RESTORE_TABLE.address;

pub(super) const WONDER_GUMMI_IQ: [u32; 3] = sym::WONDER_GUMMI_IQ_GAIN.address;

/// The region of a dump, as the order of the tables above.
/// `RECRUITMENT_LEVEL_BOOST_TABLE` of overlay 10: what the recruiting
/// leader's level (0 to 101) adds to a recruit rate, in 1000ths.
pub(super) const RECRUIT_LEVEL_BOOSTS: [u32; 3] = sym::RECRUITMENT_LEVEL_BOOST_TABLE.address;

/// `EXCLUSIVE_ITEM_EFFECT_DATA`, `EXCLUSIVE_ITEM_STAT_BOOST_DATA` (arm9).
pub(super) const EXCLUSIVE_ITEM_EFFECT_DATA: [u32; 3] = sym::EXCLUSIVE_ITEM_EFFECT_DATA.address;

pub(super) const EXCLUSIVE_ITEM_STAT_BOOST_DATA: [u32; 3] = sym::EXCLUSIVE_ITEM_STAT_BOOST_DATA.address;

/// `SENTRY_DUTY_MONSTER_IDS`: the 102 visitors of sentry duty.
pub(super) const SENTRY_DUTY_MONSTER_IDS: [u32; 3] = sym::SENTRY_DUTY_MONSTER_IDS.address;

/// `RECYCLE_SHOP_ITEM_LIST`: the Recycle Shop's 24 trades.
pub(super) const RECYCLE_SHOP_ITEM_LIST: [u32; 3] = sym::RECYCLE_SHOP_ITEM_LIST.address;

/// `MONEY_QUANTITY_TABLE`, in the order EU, NA, JP: the amounts a pile of
/// Poké can hold, by quantity code.
pub(super) const MONEY_QUANTITY_TABLE: [u32; 3] = sym::MONEY_QUANTITY_TABLE.address;

/// `SPECIAL_EPISODE_MAIN_CHARACTERS`, in the order EU, NA, JP: the ten
/// characters the special episodes are played with, ten numbers each. Of
/// those: the monster first, up to four moves from the third on, the level
/// eighth (read off the table: the levels and the moves are the
/// characters' own; what the other numbers are was not looked into).
pub(super) const SPECIAL_EPISODE_MAIN_CHARACTERS: [u32; 3] = sym::SPECIAL_EPISODE_MAIN_CHARACTERS.address;
