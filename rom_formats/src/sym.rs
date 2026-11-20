//! Where the game keeps what the ROM's readers read, by pmdsky-debug
//! symbol name (GPL-3; SkyTemple's patches for a few).
//!
//! Written by `cargo run -p rom --example rom_symbols -- <pmdsky-debug>`,
//! which takes the names the code uses (`sym::NAME`,
//! `symbol("NAME", region)`): do not edit by hand.

/// A symbol of the game's binaries: its address in each release (EU, NA,
/// JP, the order of `NdsRom::region`; 0: unknown) and how many bytes it
/// takes there (0: unknown).
#[derive(Clone, Copy, Debug)]
pub struct Symbol {
    pub name: &'static str,
    pub address: [u32; 3],
    pub length: [u32; 3],
}

/// The address of a symbol in a release, by name; `None` where it is not
/// known there.
pub fn symbol(name: &str, region: usize) -> Option<u32> {
    SYMBOLS.iter().find(|symbol| symbol.name == name).and_then(|symbol| symbol.address.get(region).copied()).filter(|&address| address != 0)
}

/// In overlay10.
pub const ACID_PSYCHIC_LOWER_DEFENSE_CHANCE: Symbol =
    Symbol { name: "ACID_PSYCHIC_LOWER_DEFENSE_CHANCE", address: [0x22C_4F64, 0x22C_460C, 0x22C_5CF4], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const ADVANCED_MENU_DEFAULT_WINDOW_PARAMS: Symbol =
    Symbol { name: "ADVANCED_MENU_DEFAULT_WINDOW_PARAMS", address: [0x209_B3EC, 0x209_AEB0, 0x209_B804], length: [0x0, 0x10, 0x0] };
/// In overlay10.
pub const AI_REGULAR_ATTACK_WEIGHTS: Symbol =
    Symbol { name: "AI_REGULAR_ATTACK_WEIGHTS", address: [0x22C_5258, 0x22C_4900, 0x22C_5FE8], length: [0xA, 0xA, 0xA] };
/// In arm9.
pub const ALERT_BOX_DEFAULT_WINDOW_PARAMS: Symbol =
    Symbol { name: "ALERT_BOX_DEFAULT_WINDOW_PARAMS", address: [0x209_B4D0, 0x209_AF94, 0x209_B8E8], length: [0x0, 0x10, 0x0] };
/// In overlay10.
pub const AQUA_RING_BONUS_REGEN: Symbol = Symbol { name: "AQUA_RING_BONUS_REGEN", address: [0x22C_4F80, 0x22C_4628, 0x22C_5D10], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const AQUA_RING_HEALING_COUNTDOWN: Symbol =
    Symbol { name: "AQUA_RING_HEALING_COUNTDOWN", address: [0x22C_4F78, 0x22C_4620, 0x22C_5D08], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const AQUA_RING_TURN_RANGE: Symbol = Symbol { name: "AQUA_RING_TURN_RANGE", address: [0x22C_50BC, 0x22C_4764, 0x22C_5E4C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const ARENA_TRAP_ACTIVATION_CHANCE: Symbol =
    Symbol { name: "ARENA_TRAP_ACTIVATION_CHANCE", address: [0x22C_4EBC, 0x22C_4564, 0x22C_5C4C], length: [0x2, 0x0, 0x0] };
/// In overlay10.
pub const AURORA_BEAM_LOWER_ATTACK_CHANCE: Symbol =
    Symbol { name: "AURORA_BEAM_LOWER_ATTACK_CHANCE", address: [0x22C_4D80, 0x22C_4428, 0x22C_5B10], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BAD_POISON_DAMAGE_COOLDOWN: Symbol =
    Symbol { name: "BAD_POISON_DAMAGE_COOLDOWN", address: [0x22C_4D6C, 0x22C_4414, 0x22C_5AFC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BAD_POISON_DAMAGE_TABLE: Symbol =
    Symbol { name: "BAD_POISON_DAMAGE_TABLE", address: [0x22C_55F4, 0x22C_4C9C, 0x22C_6384], length: [0x3C, 0x3C, 0x3C] };
/// In overlay10.
pub const BAD_WEATHER_DAMAGE: Symbol = Symbol { name: "BAD_WEATHER_DAMAGE", address: [0x22C_5044, 0x22C_46EC, 0x22C_5DD4], length: [0x2, 0x2, 0x2] };
/// In overlay29.
pub const BELLY_LOST_PER_TURN: Symbol = Symbol { name: "BELLY_LOST_PER_TURN", address: [0x231_14D0, 0x231_0A70, 0x231_1F98], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const BLAZE_KICK_BURN_CHANCE: Symbol = Symbol { name: "BLAZE_KICK_BURN_CHANCE", address: [0x22C_4E98, 0x22C_4540, 0x22C_5C28], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BOUNCE_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "BOUNCE_DAMAGE_MULTIPLIER", address: [0x22C_5144, 0x22C_47EC, 0x22C_5ED4], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const BOUNCE_PARALYZE_CHANCE: Symbol = Symbol { name: "BOUNCE_PARALYZE_CHANCE", address: [0x22C_4DEC, 0x22C_4494, 0x22C_5B7C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BUBBLE_LOWER_SPEED_CHANCE: Symbol =
    Symbol { name: "BUBBLE_LOWER_SPEED_CHANCE", address: [0x22C_4F84, 0x22C_462C, 0x22C_5D14], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BUG_BUZZ_LOWER_SPECIAL_DEFENSE_CHANCE: Symbol =
    Symbol { name: "BUG_BUZZ_LOWER_SPECIAL_DEFENSE_CHANCE", address: [0x22C_4F6C, 0x22C_4614, 0x22C_5CFC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BURN_DAMAGE: Symbol = Symbol { name: "BURN_DAMAGE", address: [0x22C_4E2C, 0x22C_44D4, 0x22C_5BBC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BURN_DAMAGE_COOLDOWN: Symbol = Symbol { name: "BURN_DAMAGE_COOLDOWN", address: [0x22C_4F68, 0x22C_4610, 0x22C_5CF8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const BURN_DAMAGE_MULTIPLIER: Symbol = Symbol { name: "BURN_DAMAGE_MULTIPLIER", address: [0x22C_509C, 0x22C_4744, 0x22C_5E2C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const BURN_TURN_RANGE: Symbol = Symbol { name: "BURN_TURN_RANGE", address: [0x22C_51CC, 0x22C_4874, 0x22C_5F5C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const CACNEA_SPIKE_POWER: Symbol = Symbol { name: "CACNEA_SPIKE_POWER", address: [0x22C_5034, 0x22C_46DC, 0x22C_5DC4], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const CALCIUM_STAT_BOOST: Symbol = Symbol { name: "CALCIUM_STAT_BOOST", address: [0x22C_5024, 0x22C_46CC, 0x22C_5DB4], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const CHARGE_BEAM_BOOST_SPECIAL_ATTACK_CHANCE: Symbol =
    Symbol { name: "CHARGE_BEAM_BOOST_SPECIAL_ATTACK_CHANCE", address: [0x22C_4E48, 0x22C_44F0, 0x22C_5BD8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const CONFUSED_TURN_RANGE: Symbol = Symbol { name: "CONFUSED_TURN_RANGE", address: [0x22C_51E8, 0x22C_4890, 0x22C_5F78], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const CONSTRICTION_DAMAGE: Symbol = Symbol { name: "CONSTRICTION_DAMAGE", address: [0x22C_4DC4, 0x22C_446C, 0x22C_5B54], length: [0x2, 0x0, 0x0] };
/// In overlay10.
pub const CONSTRICTION_TURN_RANGE: Symbol = Symbol { name: "CONSTRICTION_TURN_RANGE", address: [0x22C_51F8, 0x22C_48A0, 0x22C_5F88], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const CONVERSION2_TURN_RANGE: Symbol = Symbol { name: "CONVERSION2_TURN_RANGE", address: [0x22C_51AC, 0x22C_4854, 0x22C_5F3C], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const CONVERSION2_TYPE_TABLE: Symbol = Symbol { name: "CONVERSION2_TYPE_TABLE", address: [0x20A_1EB0, 0x20A_192C, 0x20A_2D00], length: [0x12, 0x0, 0x0] };
/// In overlay10.
pub const CORSOLA_TWIG_POWER: Symbol = Symbol { name: "CORSOLA_TWIG_POWER", address: [0x22C_5030, 0x22C_46D8, 0x22C_5DC0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const CRINGE_TURN_RANGE: Symbol = Symbol { name: "CRINGE_TURN_RANGE", address: [0x22C_51DC, 0x22C_4884, 0x22C_5F6C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const CROSS_EYED_TURN_RANGE: Symbol = Symbol { name: "CROSS_EYED_TURN_RANGE", address: [0x22C_5188, 0x22C_4830, 0x22C_5F18], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const CRUNCH_LOWER_DEFENSE_CHANCE: Symbol =
    Symbol { name: "CRUNCH_LOWER_DEFENSE_CHANCE", address: [0x22C_4DA8, 0x22C_4450, 0x22C_5B38], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const CURSE_DAMAGE_COOLDOWN: Symbol = Symbol { name: "CURSE_DAMAGE_COOLDOWN", address: [0x22C_4E14, 0x22C_44BC, 0x22C_5BA4], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const CURSE_TURN_RANGE: Symbol = Symbol { name: "CURSE_TURN_RANGE", address: [0x22C_50DC, 0x22C_4784, 0x22C_5E6C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const DECOY_TURN_RANGE: Symbol = Symbol { name: "DECOY_TURN_RANGE", address: [0x22C_510C, 0x22C_47B4, 0x22C_5E9C], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const DEFAULT_MISSION_REWARD_TYPE_WEIGHTS: Symbol =
    Symbol { name: "DEFAULT_MISSION_REWARD_TYPE_WEIGHTS", address: [0x20A_42E4, 0x20A_3CE4, 0x20A_50C8], length: [0xE, 0xE, 0xE] };
/// In overlay10.
pub const DEFENSIVE_STAT_STAGE_MULTIPLIERS: Symbol =
    Symbol { name: "DEFENSIVE_STAT_STAGE_MULTIPLIERS", address: [0x22C_5744, 0x22C_4DEC, 0x22C_64D4], length: [0x54, 0x54, 0x54] };
/// In arm9.
pub const DEF_SCARF_STAT_BOOST: Symbol = Symbol { name: "DEF_SCARF_STAT_BOOST", address: [0x20A_1E2C, 0x20A_18A8, 0x20A_2C7C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const DESTINY_BOND_TURN_RANGE: Symbol = Symbol { name: "DESTINY_BOND_TURN_RANGE", address: [0x22C_512C, 0x22C_47D4, 0x22C_5EBC], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const DETECT_BAND_MOVE_ACCURACY_DROP: Symbol =
    Symbol { name: "DETECT_BAND_MOVE_ACCURACY_DROP", address: [0x22C_5100, 0x22C_47A8, 0x22C_5E90], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const DIALOGUE_BOX_DEFAULT_WINDOW_PARAMS: Symbol =
    Symbol { name: "DIALOGUE_BOX_DEFAULT_WINDOW_PARAMS", address: [0x209_B468, 0x209_AF2C, 0x209_B880], length: [0x0, 0x10, 0x0] };
/// In overlay10.
pub const DIG_DAMAGE_MULTIPLIER: Symbol = Symbol { name: "DIG_DAMAGE_MULTIPLIER", address: [0x22C_513C, 0x22C_47E4, 0x22C_5ECC], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const DIVE_DAMAGE_MULTIPLIER: Symbol = Symbol { name: "DIVE_DAMAGE_MULTIPLIER", address: [0x22C_5140, 0x22C_47E8, 0x22C_5ED0], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const DRAGON_RAGE_FIXED_DAMAGE: Symbol =
    Symbol { name: "DRAGON_RAGE_FIXED_DAMAGE", address: [0x22C_502C, 0x22C_46D4, 0x22C_5DBC], length: [0x2, 0x2, 0x2] };
/// In overlay29.
pub const DUNGEON_CONTROLS_MENU_WINDOW_PARAMS: Symbol =
    Symbol { name: "DUNGEON_CONTROLS_MENU_WINDOW_PARAMS", address: [0x235_2200, 0x235_15F4, 0x235_2874], length: [0x10, 0x0, 0x0] };
/// In arm9.
pub const DUNGEON_DATA_LIST: Symbol = Symbol { name: "DUNGEON_DATA_LIST", address: [0x209_E924, 0x209_E3A0, 0x209_F774], length: [0x2D0, 0x2D0, 0x2D0] };
/// In arm9.
pub const DUNGEON_RESTRICTIONS: Symbol = Symbol { name: "DUNGEON_RESTRICTIONS", address: [0x20A_11E8, 0x20A_0C64, 0x20A_2038], length: [0xC00, 0xC00, 0xC00] };
/// In overlay31.
pub const DUNGEON_WINDOW_PARAMS_1: Symbol =
    Symbol { name: "DUNGEON_WINDOW_PARAMS_1", address: [0x238_A9B8, 0x238_9D94, 0x238_B05C], length: [0x10, 0x10, 0x10] };
/// In overlay31.
pub const DUNGEON_WINDOW_PARAMS_2: Symbol =
    Symbol { name: "DUNGEON_WINDOW_PARAMS_2", address: [0x238_A9C8, 0x238_9DA4, 0x238_B06C], length: [0x10, 0x10, 0x10] };
/// In overlay31.
pub const DUNGEON_WINDOW_PARAMS_4: Symbol =
    Symbol { name: "DUNGEON_WINDOW_PARAMS_4", address: [0x238_A9E8, 0x238_9DC4, 0x238_B08C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const EARTH_POWER_LOWER_SPECIAL_DEFENSE_CHANCE: Symbol =
    Symbol { name: "EARTH_POWER_LOWER_SPECIAL_DEFENSE_CHANCE", address: [0x22C_4DD8, 0x22C_4480, 0x22C_5B68], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const EATEN_BLAST_SEED_DAMAGE_NORMAL: Symbol =
    Symbol { name: "EATEN_BLAST_SEED_DAMAGE_NORMAL", address: [0x22C_4E40, 0x22C_44E8, 0x22C_5BD0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const EFFECT_ANIMATION_INFO: Symbol =
    Symbol { name: "EFFECT_ANIMATION_INFO", address: [0x22C_CE84, 0x22C_C52C, 0x22C_DC14], length: [0x4C90, 0x4C90, 0x4C90] };
/// In overlay10.
pub const EMBARGO_TURN_RANGE: Symbol = Symbol { name: "EMBARGO_TURN_RANGE", address: [0x22C_50D0, 0x22C_4778, 0x22C_5E60], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const ENCORE_TURN_RANGE: Symbol = Symbol { name: "ENCORE_TURN_RANGE", address: [0x22C_5130, 0x22C_47D8, 0x22C_5EC0], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const ENTITIES: Symbol = Symbol { name: "ENTITIES", address: [0x20A_8890, 0x20A_7FF0, 0x20A_9438], length: [0x1218, 0x1218, 0x1218] };
/// In overlay10.
pub const ERUPTION_DAMAGE_MULT_TABLE: Symbol =
    Symbol { name: "ERUPTION_DAMAGE_MULT_TABLE", address: [0x22C_531C, 0x22C_49C4, 0x22C_60AC], length: [0x10, 0x10, 0x10] };
/// In arm9.
pub const EVENTS: Symbol = Symbol { name: "EVENTS", address: [0x20A_5BD8, 0x20A_5488, 0x20A_6894], length: [0x1584, 0x1434, 0x1470] };
/// In arm9.
pub const EXCLUSIVE_ITEM_EFFECT_DATA: Symbol =
    Symbol { name: "EXCLUSIVE_ITEM_EFFECT_DATA", address: [0x209_8568, 0x209_8124, 0x209_8418], length: [0x778, 0x778, 0x778] };
/// In overlay10.
pub const EXCLUSIVE_ITEM_EXP_BOOST: Symbol =
    Symbol { name: "EXCLUSIVE_ITEM_EXP_BOOST", address: [0x22C_4EE4, 0x22C_458C, 0x22C_5C74], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const EXCLUSIVE_ITEM_STAT_BOOST_DATA: Symbol =
    Symbol { name: "EXCLUSIVE_ITEM_STAT_BOOST_DATA", address: [0x209_852C, 0x209_80E8, 0x209_83DC], length: [0x3C, 0x3C, 0x3C] };
/// In overlay29.
pub const EXPLOSION_FIXED_DAMAGES: Symbol = Symbol { name: "EXPLOSION_FIXED_DAMAGES", address: [0x235_4300, 0x235_3700, 0x235_4980], length: [0xC, 0x0, 0x0] };
/// In overlay10.
pub const EXP_ELITE_EXP_BOOST: Symbol = Symbol { name: "EXP_ELITE_EXP_BOOST", address: [0x22C_4E00, 0x22C_44A8, 0x22C_5B90], length: [0x2, 0x2, 0x2] };
/// In arm9 (SkyTemple's patches).
pub const EXTRA_DUNGEON_DATA: Symbol = Symbol { name: "EXTRA_DUNGEON_DATA", address: [0x204_EFE0, 0x204_ECA8, 0x204_F000], length: [0x168, 0x168, 0x168] };
/// In overlay10.
pub const FACADE_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "FACADE_DAMAGE_MULTIPLIER", address: [0x22C_5070, 0x22C_4718, 0x22C_5E00], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const FAKE_OUT_CRINGE_CHANCE: Symbol = Symbol { name: "FAKE_OUT_CRINGE_CHANCE", address: [0x22C_4FC8, 0x22C_4670, 0x22C_5D58], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const FEMALE_ACCURACY_STAGE_MULTIPLIERS: Symbol =
    Symbol { name: "FEMALE_ACCURACY_STAGE_MULTIPLIERS", address: [0x22C_5E0C, 0x22C_54B4, 0x22C_6B9C], length: [0x54, 0x54, 0x54] };
/// In overlay10.
pub const FEMALE_EVASION_STAGE_MULTIPLIERS: Symbol =
    Symbol { name: "FEMALE_EVASION_STAGE_MULTIPLIERS", address: [0x22C_5E60, 0x22C_5508, 0x22C_6BF0], length: [0x54, 0x54, 0x54] };
/// In overlay10.
pub const FIRE_FANG_CRINGE_CHANCE: Symbol = Symbol { name: "FIRE_FANG_CRINGE_CHANCE", address: [0x22C_4DF4, 0x22C_449C, 0x22C_5B84], length: [0x2, 0x2, 0x2] };
/// In overlay29.
pub const FIXED_ROOM_ENTITY_SPAWN_TABLE: Symbol =
    Symbol { name: "FIXED_ROOM_ENTITY_SPAWN_TABLE", address: [0x235_0EF8, 0x235_02EC, 0x235_156C], length: [0xC9C, 0xC9C, 0xC9C] };
/// In overlay10.
pub const FIXED_ROOM_MONSTER_SPAWN_STATS_TABLE: Symbol =
    Symbol { name: "FIXED_ROOM_MONSTER_SPAWN_STATS_TABLE", address: [0x22C_6290, 0x22C_5938, 0x22C_7020], length: [0x4A4, 0x4A4, 0x4A4] };
/// In overlay29.
pub const FIXED_ROOM_MONSTER_SPAWN_TABLE: Symbol =
    Symbol { name: "FIXED_ROOM_MONSTER_SPAWN_TABLE", address: [0x235_0B20, 0x234_FF14, 0x235_1194], length: [0x1E0, 0x1E0, 0x1E0] };
/// In overlay10.
pub const FIXED_ROOM_PROPERTIES_TABLE: Symbol =
    Symbol { name: "FIXED_ROOM_PROPERTIES_TABLE", address: [0x22C_75C8, 0x22C_6C70, 0x22C_8358], length: [0xC00, 0xC00, 0xC00] };
/// In overlay10.
pub const FLARE_BLITZ_BURN_CHANCE: Symbol = Symbol { name: "FLARE_BLITZ_BURN_CHANCE", address: [0x22C_4FDC, 0x22C_4684, 0x22C_5D6C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const FLASH_CANNON_LOWER_SPECIAL_DEFENSE_CHANCE: Symbol =
    Symbol { name: "FLASH_CANNON_LOWER_SPECIAL_DEFENSE_CHANCE", address: [0x22C_4E60, 0x22C_4508, 0x22C_5BF0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const FLY_DAMAGE_MULTIPLIER: Symbol = Symbol { name: "FLY_DAMAGE_MULTIPLIER", address: [0x22C_5218, 0x22C_48C0, 0x22C_5FA8], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const FOCUS_PUNCH_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "FOCUS_PUNCH_DAMAGE_MULTIPLIER", address: [0x22C_520C, 0x22C_48B4, 0x22C_5F9C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const FROZEN_TURN_RANGE: Symbol = Symbol { name: "FROZEN_TURN_RANGE", address: [0x22C_51C8, 0x22C_4870, 0x22C_5F58], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const GEO_PEBBLE_DAMAGE: Symbol = Symbol { name: "GEO_PEBBLE_DAMAGE", address: [0x22C_500C, 0x22C_46B4, 0x22C_5D9C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const GOLD_FANG_POWER: Symbol = Symbol { name: "GOLD_FANG_POWER", address: [0x22C_5038, 0x22C_46E0, 0x22C_5DC8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const GOLD_THORN_POWER: Symbol = Symbol { name: "GOLD_THORN_POWER", address: [0x22C_4E28, 0x22C_44D0, 0x22C_5BB8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const GRAVELEROCK_DAMAGE: Symbol = Symbol { name: "GRAVELEROCK_DAMAGE", address: [0x22C_5010, 0x22C_46B8, 0x22C_5DA0], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const GUEST_MONSTER_DATA: Symbol = Symbol { name: "GUEST_MONSTER_DATA", address: [0x20A_2E40, 0x20A_28BC, 0x20A_3C90], length: [0x288, 0x288, 0x288] };
/// In arm9 (SkyTemple's patches).
pub const GUEST_MONSTER_DATA2: Symbol = Symbol { name: "GUEST_MONSTER_DATA2", address: [0x204_F148, 0x204_EE10, 0x204_F168], length: [0x1D0, 0x1D0, 0x1D0] };
/// In arm9.
pub const GUMMI_BELLY_RESTORE_TABLE: Symbol =
    Symbol { name: "GUMMI_BELLY_RESTORE_TABLE", address: [0x20A_2ABC, 0x20A_2538, 0x20A_390C], length: [0x288, 0x288, 0x288] };
/// In arm9.
pub const GUMMI_STAT_BOOST: Symbol = Symbol { name: "GUMMI_STAT_BOOST", address: [0x20A_1E0C, 0x20A_1888, 0x20A_2C5C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const HEADBUTT_CRINGE_CHANCE: Symbol = Symbol { name: "HEADBUTT_CRINGE_CHANCE", address: [0x22C_4DF0, 0x22C_4498, 0x22C_5B80], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const HEAL_BLOCK_TURN_RANGE: Symbol = Symbol { name: "HEAL_BLOCK_TURN_RANGE", address: [0x22C_50CC, 0x22C_4774, 0x22C_5E5C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const HEAL_ORDER_HP_RESTORATION: Symbol =
    Symbol { name: "HEAL_ORDER_HP_RESTORATION", address: [0x22C_4E10, 0x22C_44B8, 0x22C_5BA0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const ICE_FANG_CRINGE_CHANCE: Symbol = Symbol { name: "ICE_FANG_CRINGE_CHANCE", address: [0x22C_4E94, 0x22C_453C, 0x22C_5C24], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const ICE_FANG_FREEZE_CHANCE: Symbol = Symbol { name: "ICE_FANG_FREEZE_CHANCE", address: [0x22C_4E70, 0x22C_4518, 0x22C_5C00], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const IMPRISON_TURN_RANGE: Symbol = Symbol { name: "IMPRISON_TURN_RANGE", address: [0x22C_5074, 0x22C_471C, 0x22C_5E04], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const INFATUATED_TURN_RANGE: Symbol = Symbol { name: "INFATUATED_TURN_RANGE", address: [0x22C_515C, 0x22C_4804, 0x22C_5EEC], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const INGRAIN_BONUS_REGEN: Symbol = Symbol { name: "INGRAIN_BONUS_REGEN", address: [0x22C_4EE8, 0x22C_4590, 0x22C_5C78], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const INGRAIN_BONUS_REGEN_COOLDOWN: Symbol =
    Symbol { name: "INGRAIN_BONUS_REGEN_COOLDOWN", address: [0x22C_4F40, 0x22C_45E8, 0x22C_5CD0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const INGRAIN_TURN_RANGE: Symbol = Symbol { name: "INGRAIN_TURN_RANGE", address: [0x22C_5210, 0x22C_48B8, 0x22C_5FA0], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const INTIMIDATOR_ACTIVATION_CHANCE: Symbol =
    Symbol { name: "INTIMIDATOR_ACTIVATION_CHANCE", address: [0x22C_4F10, 0x22C_45B8, 0x22C_5CA0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const INVENTORY_MENU_DEFAULT_WINDOW_PARAMS: Symbol =
    Symbol { name: "INVENTORY_MENU_DEFAULT_WINDOW_PARAMS", address: [0x22C_4CEC, 0x22C_4394, 0x22C_5A7C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const INVISIBLE_TURN_RANGE: Symbol = Symbol { name: "INVISIBLE_TURN_RANGE", address: [0x22C_5120, 0x22C_47C8, 0x22C_5EB0], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const IQ_GROUP_SKILLS: Symbol = Symbol { name: "IQ_GROUP_SKILLS", address: [0x20A_2314, 0x20A_1D90, 0x20A_3164], length: [0x190, 0x190, 0x190] };
/// In arm9.
pub const IQ_GUMMI_GAIN_TABLE: Symbol = Symbol { name: "IQ_GUMMI_GAIN_TABLE", address: [0x20A_2834, 0x20A_22B0, 0x20A_3684], length: [0x288, 0x288, 0x288] };
/// In arm9.
pub const IQ_SKILLS: Symbol = Symbol { name: "IQ_SKILLS", address: [0x20A_2200, 0x20A_1C7C, 0x20A_3050], length: [0x114, 0x114, 0x114] };
/// In arm9.
pub const IQ_SKILL_RESTRICTIONS: Symbol = Symbol { name: "IQ_SKILL_RESTRICTIONS", address: [0x20A_1FE0, 0x20A_1A5C, 0x20A_2E30], length: [0x8A, 0x8A, 0x8A] };
/// In overlay10.
pub const IRON_STAT_BOOST: Symbol = Symbol { name: "IRON_STAT_BOOST", address: [0x22C_5020, 0x22C_46C8, 0x22C_5DB0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const IRON_TAIL_LOWER_DEFENSE_CHANCE: Symbol =
    Symbol { name: "IRON_TAIL_LOWER_DEFENSE_CHANCE", address: [0x22C_4D98, 0x22C_4440, 0x22C_5B28], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const IRON_THORN_POWER: Symbol = Symbol { name: "IRON_THORN_POWER", address: [0x22C_5040, 0x22C_46E8, 0x22C_5DD0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const ITEM_ANIMATION_INFO: Symbol = Symbol { name: "ITEM_ANIMATION_INFO", address: [0x22C_83DC, 0x22C_7A84, 0x22C_916C], length: [0x15E0, 0x15E0, 0x15E0] };
/// In arm9.
pub const ITEM_TABLES_PTRS_1: Symbol = Symbol { name: "ITEM_TABLES_PTRS_1", address: [0x20B_1264, 0x20B_0948, 0x20B_21BC], length: [0x68, 0x68, 0x68] };
/// In arm9.
pub const KECLEON_SHOP_ITEM_TABLE_LISTS_1: Symbol =
    Symbol { name: "KECLEON_SHOP_ITEM_TABLE_LISTS_1", address: [0x209_8504, 0x209_80C0, 0x209_83B4], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const LAST_RESORT_DAMAGE_MULT_TABLE: Symbol =
    Symbol { name: "LAST_RESORT_DAMAGE_MULT_TABLE", address: [0x22C_529C, 0x22C_4944, 0x22C_602C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const LEECH_SEED_DAMAGE_COOLDOWN: Symbol =
    Symbol { name: "LEECH_SEED_DAMAGE_COOLDOWN", address: [0x22C_5004, 0x22C_46AC, 0x22C_5D94], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const LEECH_SEED_HP_DRAIN: Symbol = Symbol { name: "LEECH_SEED_HP_DRAIN", address: [0x22C_4EE0, 0x22C_4588, 0x22C_5C70], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const LEECH_SEED_TURN_RANGE: Symbol = Symbol { name: "LEECH_SEED_TURN_RANGE", address: [0x22C_50E8, 0x22C_4790, 0x22C_5E78], length: [0x4, 0x4, 0x4] };
/// In overlay11.
pub const LEVEL_TILEMAP_LIST: Symbol = Symbol { name: "LEVEL_TILEMAP_LIST", address: [0x232_185C, 0x232_0D2C, 0x232_2290], length: [0x288, 0x288, 0x288] };
/// In overlay10.
pub const LIFE_SEED_HP_BOOST: Symbol = Symbol { name: "LIFE_SEED_HP_BOOST", address: [0x22C_4E50, 0x22C_44F8, 0x22C_5BE0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const LIGHT_SCREEN_TURN_RANGE: Symbol = Symbol { name: "LIGHT_SCREEN_TURN_RANGE", address: [0x22C_5088, 0x22C_4730, 0x22C_5E18], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const LUSTER_PURGE_LOWER_SPECIAL_DEFENSE_CHANCE: Symbol =
    Symbol { name: "LUSTER_PURGE_LOWER_SPECIAL_DEFENSE_CHANCE", address: [0x22C_4E64, 0x22C_450C, 0x22C_5BF4], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const MAGIC_COAT_TURN_RANGE: Symbol = Symbol { name: "MAGIC_COAT_TURN_RANGE", address: [0x22C_5098, 0x22C_4740, 0x22C_5E28], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MAGNET_RISE_TURN_RANGE: Symbol = Symbol { name: "MAGNET_RISE_TURN_RANGE", address: [0x22C_50D8, 0x22C_4780, 0x22C_5E68], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MALE_ACCURACY_STAGE_MULTIPLIERS: Symbol =
    Symbol { name: "MALE_ACCURACY_STAGE_MULTIPLIERS", address: [0x22C_5D64, 0x22C_540C, 0x22C_6AF4], length: [0x54, 0x54, 0x54] };
/// In overlay10.
pub const MALE_EVASION_STAGE_MULTIPLIERS: Symbol =
    Symbol { name: "MALE_EVASION_STAGE_MULTIPLIERS", address: [0x22C_5DB8, 0x22C_5460, 0x22C_6B48], length: [0x54, 0x54, 0x54] };
/// In arm9.
pub const MAP_MARKER_PLACEMENTS: Symbol =
    Symbol { name: "MAP_MARKER_PLACEMENTS", address: [0x20A_9D70, 0x20A_94D0, 0x20A_A918], length: [0x9B0, 0x9B0, 0x9B0] };
/// In overlay10.
pub const MATCHUP_IMMUNE_MULTIPLIER: Symbol =
    Symbol { name: "MATCHUP_IMMUNE_MULTIPLIER", address: [0x22C_50B0, 0x22C_4758, 0x22C_5E40], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MATCHUP_IMMUNE_MULTIPLIER_ERRATIC_PLAYER: Symbol =
    Symbol { name: "MATCHUP_IMMUNE_MULTIPLIER_ERRATIC_PLAYER", address: [0x22C_5178, 0x22C_4820, 0x22C_5F08], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MATCHUP_NEUTRAL_MULTIPLIER: Symbol =
    Symbol { name: "MATCHUP_NEUTRAL_MULTIPLIER", address: [0x22C_5174, 0x22C_481C, 0x22C_5F04], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MATCHUP_NEUTRAL_MULTIPLIER_ERRATIC_PLAYER: Symbol =
    Symbol { name: "MATCHUP_NEUTRAL_MULTIPLIER_ERRATIC_PLAYER", address: [0x22C_5184, 0x22C_482C, 0x22C_5F14], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER: Symbol =
    Symbol { name: "MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER", address: [0x22C_5168, 0x22C_4810, 0x22C_5EF8], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER: Symbol =
    Symbol { name: "MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER", address: [0x22C_517C, 0x22C_4824, 0x22C_5F0C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MATCHUP_SUPER_EFFECTIVE_MULTIPLIER: Symbol =
    Symbol { name: "MATCHUP_SUPER_EFFECTIVE_MULTIPLIER", address: [0x22C_5170, 0x22C_4818, 0x22C_5F00], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MATCHUP_SUPER_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER: Symbol =
    Symbol { name: "MATCHUP_SUPER_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER", address: [0x22C_50A4, 0x22C_474C, 0x22C_5E34], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const METAL_CLAW_BOOST_ATTACK_CHANCE: Symbol =
    Symbol { name: "METAL_CLAW_BOOST_ATTACK_CHANCE", address: [0x22C_4EAC, 0x22C_4554, 0x22C_5C3C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const METEOR_MASH_BOOST_ATTACK_CHANCE: Symbol =
    Symbol { name: "METEOR_MASH_BOOST_ATTACK_CHANCE", address: [0x22C_4F5C, 0x22C_4604, 0x22C_5CEC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const METRONOME_TABLE: Symbol = Symbol { name: "METRONOME_TABLE", address: [0x22C_6734, 0x22C_5DDC, 0x22C_74C4], length: [0x540, 0x540, 0x540] };
/// In overlay10.
pub const ME_FIRST_MULTIPLIER: Symbol = Symbol { name: "ME_FIRST_MULTIPLIER", address: [0x22C_5068, 0x22C_4710, 0x22C_5DF8], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MIRROR_COAT_TURN_RANGE: Symbol = Symbol { name: "MIRROR_COAT_TURN_RANGE", address: [0x22C_5124, 0x22C_47CC, 0x22C_5EB4], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const MIRROR_MOVE_TURN_RANGE: Symbol = Symbol { name: "MIRROR_MOVE_TURN_RANGE", address: [0x22C_5198, 0x22C_4840, 0x22C_5F28], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const MISSION_FLOOR_RANKS_PTRS: Symbol =
    Symbol { name: "MISSION_FLOOR_RANKS_PTRS", address: [0x20A_1058, 0x20A_0AD4, 0x20A_1EA8], length: [0x190, 0x190, 0x190] };
/// In arm9.
pub const MISSION_RANK_POINTS: Symbol = Symbol { name: "MISSION_RANK_POINTS", address: [0x20A_436C, 0x20A_3D6C, 0x20A_5150], length: [0x40, 0x40, 0x40] };
/// In overlay10.
pub const MIST_BALL_LOWER_SPECIAL_ATTACK_CHANCE: Symbol =
    Symbol { name: "MIST_BALL_LOWER_SPECIAL_ATTACK_CHANCE", address: [0x22C_4E38, 0x22C_44E0, 0x22C_5BC8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const MIST_TURN_RANGE: Symbol = Symbol { name: "MIST_TURN_RANGE", address: [0x22C_5090, 0x22C_4738, 0x22C_5E20], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const MONEY_QUANTITY_TABLE: Symbol = Symbol { name: "MONEY_QUANTITY_TABLE", address: [0x20A_24A4, 0x20A_1F20, 0x20A_32F4], length: [0x190, 0x190, 0x190] };
/// In overlay10.
pub const MOONLIGHT_HP_RESTORATION_TABLE: Symbol =
    Symbol { name: "MOONLIGHT_HP_RESTORATION_TABLE", address: [0x22C_52CC, 0x22C_4974, 0x22C_605C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const MORNING_SUN_HP_RESTORATION_TABLE: Symbol =
    Symbol { name: "MORNING_SUN_HP_RESTORATION_TABLE", address: [0x22C_52DC, 0x22C_4984, 0x22C_606C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const MOVE_ANIMATION_INFO: Symbol = Symbol { name: "MOVE_ANIMATION_INFO", address: [0x22C_99BC, 0x22C_9064, 0x22C_A74C], length: [0x34C8, 0x34C8, 0x34C8] };
/// In arm9.
pub const MUNCH_BELT_STAT_BOOST: Symbol = Symbol { name: "MUNCH_BELT_STAT_BOOST", address: [0x20A_1E00, 0x20A_187C, 0x20A_2C50], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const MUSIC_ID_TABLE: Symbol = Symbol { name: "MUSIC_ID_TABLE", address: [0x22C_5EB4, 0x22C_555C, 0x22C_6C44], length: [0x154, 0x154, 0x154] };
/// In overlay10.
pub const NATURAL_GIFT_ITEM_TABLE: Symbol =
    Symbol { name: "NATURAL_GIFT_ITEM_TABLE", address: [0x22C_5A88, 0x22C_5130, 0x22C_6818], length: [0xCC, 0xCC, 0xCC] };
/// In arm9.
pub const NATURAL_LOG_VALUE_TABLE: Symbol =
    Symbol { name: "NATURAL_LOG_VALUE_TABLE", address: [0x209_17E0, 0x209_1448, 0x209_1730], length: [0x0, 0x1000, 0x0] };
/// In overlay10.
pub const NATURE_POWER_TABLE: Symbol = Symbol { name: "NATURE_POWER_TABLE", address: [0x22C_5798, 0x22C_4E40, 0x22C_6528], length: [0x78, 0x78, 0x78] };
/// In overlay10.
pub const NIGHTMARE_TURN_RANGE: Symbol = Symbol { name: "NIGHTMARE_TURN_RANGE", address: [0x22C_507C, 0x22C_4724, 0x22C_5E0C], length: [0x4, 0x4, 0x4] };
/// In overlay11.
pub const OBJECTS: Symbol = Symbol { name: "OBJECTS", address: [0x231_F8DC, 0x231_EE54, 0x232_03B8], length: [0x1AAC, 0x1A04, 0x1A04] };
/// In overlay10.
pub const OCTAZOOKA_LOWER_ACCURACY_CHANCE: Symbol =
    Symbol { name: "OCTAZOOKA_LOWER_ACCURACY_CHANCE", address: [0x22C_4E58, 0x22C_4500, 0x22C_5BE8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const OFFENSIVE_STAT_STAGE_MULTIPLIERS: Symbol =
    Symbol { name: "OFFENSIVE_STAT_STAGE_MULTIPLIERS", address: [0x22C_56F0, 0x22C_4D98, 0x22C_6480], length: [0x54, 0x54, 0x54] };
/// In overlay10.
pub const ORAN_BERRY_FULL_HP_BOOST: Symbol =
    Symbol { name: "ORAN_BERRY_FULL_HP_BOOST", address: [0x22C_4E4C, 0x22C_44F4, 0x22C_5BDC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const ORAN_BERRY_HP_RESTORATION: Symbol =
    Symbol { name: "ORAN_BERRY_HP_RESTORATION", address: [0x22C_4F44, 0x22C_45EC, 0x22C_5CD4], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const OREN_BERRY_DAMAGE: Symbol = Symbol { name: "OREN_BERRY_DAMAGE", address: [0x22C_4D90, 0x22C_4438, 0x22C_5B20], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const OUTLAW_MISSION_REWARD_TYPE_WEIGHTS: Symbol =
    Symbol { name: "OUTLAW_MISSION_REWARD_TYPE_WEIGHTS", address: [0x20A_42D6, 0x20A_3CD6, 0x20A_50BA], length: [0xE, 0xE, 0xE] };
/// In overlay10.
pub const PARALYSIS_TURN_RANGE: Symbol = Symbol { name: "PARALYSIS_TURN_RANGE", address: [0x22C_51EC, 0x22C_4894, 0x22C_5F7C], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const PARTNER_TALK_KIND_TABLE: Symbol =
    Symbol { name: "PARTNER_TALK_KIND_TABLE", address: [0x209_D268, 0x209_CCE4, 0x209_E0B8], length: [0x58, 0x58, 0x58] };
/// In overlay10.
pub const PERISH_SONG_TURN_RANGE: Symbol = Symbol { name: "PERISH_SONG_TURN_RANGE", address: [0x22C_5128, 0x22C_47D0, 0x22C_5EB8], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const PITFALL_TRAP_DAMAGE: Symbol = Symbol { name: "PITFALL_TRAP_DAMAGE", address: [0x22C_4E3C, 0x22C_44E4, 0x22C_5BCC], length: [0x2, 0x2, 0x2] };
/// In overlay13.
pub const PLAY_OLD_GAME_BOOST: Symbol = Symbol { name: "PLAY_OLD_GAME_BOOST", address: [0x238_CB5C, 0x238_C01C, 0x238_D584], length: [0x8, 0x8, 0x8] };
/// In overlay10.
pub const POISON_DAMAGE: Symbol = Symbol { name: "POISON_DAMAGE", address: [0x22C_4E30, 0x22C_44D8, 0x22C_5BC0], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const POISON_DAMAGE_COOLDOWN: Symbol = Symbol { name: "POISON_DAMAGE_COOLDOWN", address: [0x22C_5000, 0x22C_46A8, 0x22C_5D90], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const POISON_FANG_POISON_CHANCE: Symbol =
    Symbol { name: "POISON_FANG_POISON_CHANCE", address: [0x22C_4F9C, 0x22C_4644, 0x22C_5D2C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const POISON_TAIL_POISON_CHANCE: Symbol =
    Symbol { name: "POISON_TAIL_POISON_CHANCE", address: [0x22C_4DE4, 0x22C_448C, 0x22C_5B74], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const PORTRAIT_LAYOUTS: Symbol = Symbol { name: "PORTRAIT_LAYOUTS", address: [0x209_E598, 0x209_E014, 0x209_F3E8], length: [0xC0, 0xC0, 0xC0] };
/// In arm9.
pub const POWER_BAND_STAT_BOOST: Symbol = Symbol { name: "POWER_BAND_STAT_BOOST", address: [0x20A_1E30, 0x20A_18AC, 0x20A_2C80], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const POWER_PITCHER_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "POWER_PITCHER_DAMAGE_MULTIPLIER", address: [0x22C_5150, 0x22C_47F8, 0x22C_5EE0], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const PROTECT_TURN_RANGE: Symbol = Symbol { name: "PROTECT_TURN_RANGE", address: [0x22C_5110, 0x22C_47B8, 0x22C_5EA0], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const PROTEIN_STAT_BOOST: Symbol = Symbol { name: "PROTEIN_STAT_BOOST", address: [0x22C_4D78, 0x22C_4420, 0x22C_5B08], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const QUICK_DODGER_MOVE_ACCURACY_DROP: Symbol =
    Symbol { name: "QUICK_DODGER_MOVE_ACCURACY_DROP", address: [0x22C_5160, 0x22C_4808, 0x22C_5EF0], length: [0x4, 0x4, 0x4] };
/// In overlay13.
pub const QUIZ_ANSWER_ASSOCIATIONS: Symbol =
    Symbol { name: "QUIZ_ANSWER_ASSOCIATIONS", address: [0x238_DA64, 0x238_CF24, 0x238_E484], length: [0x84, 0x84, 0x84] };
/// In overlay13.
pub const QUIZ_ANSWER_POINTS: Symbol = Symbol { name: "QUIZ_ANSWER_POINTS", address: [0x238_CED0, 0x238_C390, 0x238_D8F8], length: [0xAB0, 0xAB0, 0xAB0] };
/// In overlay13.
pub const QUIZ_ANSWER_STRINGS: Symbol = Symbol { name: "QUIZ_ANSWER_STRINGS", address: [0x238_CD70, 0x238_C230, 0x238_D798], length: [0x15E, 0x15E, 0x15E] };
/// In overlay13.
pub const QUIZ_QUESTION_STRINGS: Symbol = Symbol { name: "QUIZ_QUESTION_STRINGS", address: [0x238_CCEC, 0x238_C1AC, 0x238_D714], length: [0x84, 0x84, 0x84] };
/// In overlay10.
pub const RANDOM_MUSIC_ID_TABLE: Symbol = Symbol { name: "RANDOM_MUSIC_ID_TABLE", address: [0x22C_5B54, 0x22C_51FC, 0x22C_68E4], length: [0xF0, 0xF0, 0xF0] };
/// In arm9.
pub const RANK_UP_TABLE: Symbol = Symbol { name: "RANK_UP_TABLE", address: [0x20A_30C8, 0x20A_2B44, 0x20A_3F18], length: [0xD0, 0xD0, 0xD0] };
/// In overlay10.
pub const RARE_FOSSIL_DAMAGE: Symbol = Symbol { name: "RARE_FOSSIL_DAMAGE", address: [0x22C_5014, 0x22C_46BC, 0x22C_5DA4], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const RAZOR_WIND_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "RAZOR_WIND_DAMAGE_MULTIPLIER", address: [0x22C_5208, 0x22C_48B0, 0x22C_5F98], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const RECRUITMENT_LEVEL_BOOST_TABLE: Symbol =
    Symbol { name: "RECRUITMENT_LEVEL_BOOST_TABLE", address: [0x22C_59BC, 0x22C_5064, 0x22C_674C], length: [0xCC, 0xCC, 0xCC] };
/// In arm9.
pub const RECYCLE_SHOP_ITEM_LIST: Symbol =
    Symbol { name: "RECYCLE_SHOP_ITEM_LIST", address: [0x209_8D10, 0x209_88CC, 0x209_8BC0], length: [0x360, 0x360, 0x360] };
/// In overlay10.
pub const REFLECT_TURN_RANGE: Symbol = Symbol { name: "REFLECT_TURN_RANGE", address: [0x22C_5084, 0x22C_472C, 0x22C_5E14], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const REST_TURN_RANGE: Symbol = Symbol { name: "REST_TURN_RANGE", address: [0x22C_50A0, 0x22C_4748, 0x22C_5E30], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const REVERSAL_DAMAGE_MULT_TABLE: Symbol =
    Symbol { name: "REVERSAL_DAMAGE_MULT_TABLE", address: [0x22C_52EC, 0x22C_4994, 0x22C_607C], length: [0x10, 0x10, 0x10] };
/// In overlay29.
pub const ROLLOUT_DAMAGE_MULT_TABLE: Symbol =
    Symbol { name: "ROLLOUT_DAMAGE_MULT_TABLE", address: [0x235_36FC, 0x235_2AF0, 0x235_3D70], length: [0x0, 0x28, 0x0] };
/// In overlay10.
pub const ROOST_HP_RESTORATION_TABLE: Symbol =
    Symbol { name: "ROOST_HP_RESTORATION_TABLE", address: [0x22C_52BC, 0x22C_4964, 0x22C_604C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const SACRED_FIRE_BURN_CHANCE: Symbol = Symbol { name: "SACRED_FIRE_BURN_CHANCE", address: [0x22C_4DF8, 0x22C_44A0, 0x22C_5B88], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SAFEGUARD_TURN_RANGE: Symbol = Symbol { name: "SAFEGUARD_TURN_RANGE", address: [0x22C_508C, 0x22C_4734, 0x22C_5E1C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SCOPE_LENS_CRIT_RATE_BOOST: Symbol =
    Symbol { name: "SCOPE_LENS_CRIT_RATE_BOOST", address: [0x22C_5048, 0x22C_46F0, 0x22C_5DD8], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const SECONDARY_TERRAIN_TYPES: Symbol =
    Symbol { name: "SECONDARY_TERRAIN_TYPES", address: [0x20A_206C, 0x20A_1AE8, 0x20A_2EBC], length: [0xC8, 0xC8, 0xC8] };
/// In overlay10.
pub const SECRET_POWER_EFFECT_CHANCE: Symbol =
    Symbol { name: "SECRET_POWER_EFFECT_CHANCE", address: [0x22C_4EA4, 0x22C_454C, 0x22C_5C34], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SEED_FLARE_LOWER_SPECIAL_DEFENSE_CHANCE: Symbol =
    Symbol { name: "SEED_FLARE_LOWER_SPECIAL_DEFENSE_CHANCE", address: [0x22C_4E5C, 0x22C_4504, 0x22C_5BEC], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const SENTRY_DUTY_MONSTER_IDS: Symbol =
    Symbol { name: "SENTRY_DUTY_MONSTER_IDS", address: [0x20A_2134, 0x20A_1BB0, 0x20A_2F84], length: [0xCC, 0xCC, 0xCC] };
/// In overlay11.
pub const SETANIMATION_TABLE: Symbol = Symbol { name: "SETANIMATION_TABLE", address: [0x232_23FC, 0x232_18CC, 0x232_2E30], length: [0xA8, 0xA8, 0x0] };
/// In overlay10.
pub const SET_DAMAGE_STATUS_DAMAGE: Symbol =
    Symbol { name: "SET_DAMAGE_STATUS_DAMAGE", address: [0x22C_4EF0, 0x22C_4598, 0x22C_5C80], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SET_DAMAGE_TURN_RANGE: Symbol = Symbol { name: "SET_DAMAGE_TURN_RANGE", address: [0x22C_50FC, 0x22C_47A4, 0x22C_5E8C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SET_REFLECT_STATUS_TURN_RANGE: Symbol =
    Symbol { name: "SET_REFLECT_STATUS_TURN_RANGE", address: [0x22C_50A8, 0x22C_4750, 0x22C_5E38], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SHADOW_BALL_LOWER_SPECIAL_DEFENSE_CHANCE: Symbol =
    Symbol { name: "SHADOW_BALL_LOWER_SPECIAL_DEFENSE_CHANCE", address: [0x22C_4F74, 0x22C_461C, 0x22C_5D04], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SHADOW_FORCE_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "SHADOW_FORCE_DAMAGE_MULTIPLIER", address: [0x22C_5134, 0x22C_47DC, 0x22C_5EC4], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SHADOW_TAG_ACTIVATION_CHANCE: Symbol =
    Symbol { name: "SHADOW_TAG_ACTIVATION_CHANCE", address: [0x22C_4FE4, 0x22C_468C, 0x22C_5D74], length: [0x2, 0x0, 0x0] };
/// In overlay10.
pub const SILVER_SPIKE_POWER: Symbol = Symbol { name: "SILVER_SPIKE_POWER", address: [0x22C_503C, 0x22C_46E4, 0x22C_5DCC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SILVER_WIND_BOOST_CHANCE: Symbol =
    Symbol { name: "SILVER_WIND_BOOST_CHANCE", address: [0x22C_4DE0, 0x22C_4488, 0x22C_5B70], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const SIMPLE_MENU_DEFAULT_WINDOW_PARAMS: Symbol =
    Symbol { name: "SIMPLE_MENU_DEFAULT_WINDOW_PARAMS", address: [0x209_B3DC, 0x209_AEA0, 0x209_B7F4], length: [0x0, 0x10, 0x0] };
/// In overlay10.
pub const SITRUS_BERRY_FULL_HP_BOOST: Symbol =
    Symbol { name: "SITRUS_BERRY_FULL_HP_BOOST", address: [0x22C_4F4C, 0x22C_45F4, 0x22C_5CDC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SITRUS_BERRY_HP_RESTORATION: Symbol =
    Symbol { name: "SITRUS_BERRY_HP_RESTORATION", address: [0x22C_4DD0, 0x22C_4478, 0x22C_5B60], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SKULL_BASH_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "SKULL_BASH_DAMAGE_MULTIPLIER", address: [0x22C_5214, 0x22C_48BC, 0x22C_5FA4], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SKY_ATTACK_CRINGE_CHANCE: Symbol =
    Symbol { name: "SKY_ATTACK_CRINGE_CHANCE", address: [0x22C_4E90, 0x22C_4538, 0x22C_5C20], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SKY_ATTACK_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "SKY_ATTACK_DAMAGE_MULTIPLIER", address: [0x22C_5200, 0x22C_48A8, 0x22C_5F90], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SLEEPLESSNESS_TURN_RANGE: Symbol =
    Symbol { name: "SLEEPLESSNESS_TURN_RANGE", address: [0x22C_5080, 0x22C_4728, 0x22C_5E10], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SLEEP_TURN_RANGE: Symbol = Symbol { name: "SLEEP_TURN_RANGE", address: [0x22C_5078, 0x22C_4720, 0x22C_5E08], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SNATCH_TURN_RANGE: Symbol = Symbol { name: "SNATCH_TURN_RANGE", address: [0x22C_50E0, 0x22C_4788, 0x22C_5E70], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SNORE_CRINGE_CHANCE: Symbol = Symbol { name: "SNORE_CRINGE_CHANCE", address: [0x22C_4F58, 0x22C_4600, 0x22C_5CE8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SOLARBEAM_DAMAGE_MULTIPLIER: Symbol =
    Symbol { name: "SOLARBEAM_DAMAGE_MULTIPLIER", address: [0x22C_51FC, 0x22C_48A4, 0x22C_5F8C], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SONICBOOM_FIXED_DAMAGE: Symbol = Symbol { name: "SONICBOOM_FIXED_DAMAGE", address: [0x22C_4EC0, 0x22C_4568, 0x22C_5C50], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SPAWN_CAP_NO_MONSTER_HOUSE: Symbol =
    Symbol { name: "SPAWN_CAP_NO_MONSTER_HOUSE", address: [0x22C_4D88, 0x22C_4430, 0x22C_5B18], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SPAWN_CAP_WITH_MONSTER_HOUSE: Symbol =
    Symbol { name: "SPAWN_CAP_WITH_MONSTER_HOUSE", address: [0x22C_4FFC, 0x22C_46A4, 0x22C_5D8C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SPAWN_COOLDOWN: Symbol = Symbol { name: "SPAWN_COOLDOWN", address: [0x22C_4E34, 0x22C_44DC, 0x22C_5BC4], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SPAWN_COOLDOWN_THIEF_ALERT: Symbol =
    Symbol { name: "SPAWN_COOLDOWN_THIEF_ALERT", address: [0x22C_4F98, 0x22C_4640, 0x22C_5D28], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const SPECIAL_BAND_STAT_BOOST: Symbol = Symbol { name: "SPECIAL_BAND_STAT_BOOST", address: [0x20A_1DF0, 0x20A_186C, 0x20A_2C40], length: [0x2, 0x2, 0x2] };
/// In arm9.
pub const SPECIAL_EPISODE_MAIN_CHARACTERS: Symbol =
    Symbol { name: "SPECIAL_EPISODE_MAIN_CHARACTERS", address: [0x20A_2D78, 0x20A_27F4, 0x20A_3BC8], length: [0xC8, 0xC8, 0xC8] };
/// In overlay10.
pub const SPECIAL_MONSTER_MOVE_ANIMATION_INFO: Symbol =
    Symbol { name: "SPECIAL_MONSTER_MOVE_ANIMATION_INFO", address: [0x22D_1B14, 0x22D_11BC, 0x22D_28A4], length: [0xADF4, 0xADF4, 0xADF4] };
/// In overlay10.
pub const SPEED_BOOST_TURNS: Symbol = Symbol { name: "SPEED_BOOST_TURNS", address: [0x22C_4FC4, 0x22C_466C, 0x22C_5D54], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SPEED_BOOST_TURN_RANGE: Symbol = Symbol { name: "SPEED_BOOST_TURN_RANGE", address: [0x22C_51E0, 0x22C_4888, 0x22C_5F70], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SPEED_LOWER_TURN_RANGE: Symbol = Symbol { name: "SPEED_LOWER_TURN_RANGE", address: [0x22C_51E4, 0x22C_488C, 0x22C_5F74], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SPORT_CONDITION_TURN_RANGE: Symbol =
    Symbol { name: "SPORT_CONDITION_TURN_RANGE", address: [0x22C_50E4, 0x22C_478C, 0x22C_5E74], length: [0x4, 0x4, 0x4] };
/// In overlay13.
pub const STARTERS_HERO_IDS: Symbol = Symbol { name: "STARTERS_HERO_IDS", address: [0x238_CBF8, 0x238_C0B8, 0x238_D620], length: [0x40, 0x40, 0x40] };
/// In overlay13.
pub const STARTERS_PARTNER_IDS: Symbol = Symbol { name: "STARTERS_PARTNER_IDS", address: [0x238_CBCC, 0x238_C08C, 0x238_D5F4], length: [0x2A, 0x2A, 0x2A] };
/// In overlay13.
pub const STARTERS_STRINGS: Symbol = Symbol { name: "STARTERS_STRINGS", address: [0x238_CC8C, 0x238_C14C, 0x238_D6B4], length: [0x60, 0x60, 0x60] };
/// In overlay10.
pub const STEEL_WING_BOOST_DEFENSE_CHANCE: Symbol =
    Symbol { name: "STEEL_WING_BOOST_DEFENSE_CHANCE", address: [0x22C_4E1C, 0x22C_44C4, 0x22C_5BAC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const STICK_POWER: Symbol = Symbol { name: "STICK_POWER", address: [0x22C_4F7C, 0x22C_4624, 0x22C_5D0C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const SURE_SHOT_TURN_RANGE: Symbol = Symbol { name: "SURE_SHOT_TURN_RANGE", address: [0x22C_50F0, 0x22C_4798, 0x22C_5E80], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const SYNTHESIS_HP_RESTORATION_TABLE: Symbol =
    Symbol { name: "SYNTHESIS_HP_RESTORATION_TABLE", address: [0x22C_52AC, 0x22C_4954, 0x22C_603C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const TAUNT_TURN_RANGE: Symbol = Symbol { name: "TAUNT_TURN_RANGE", address: [0x22C_5114, 0x22C_47BC, 0x22C_5EA4], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const TEAM_SELECTION_MENU_DEFAULT_WINDOW_PARAMS: Symbol =
    Symbol { name: "TEAM_SELECTION_MENU_DEFAULT_WINDOW_PARAMS", address: [0x209_B4F0, 0x209_AFB4, 0x209_B908], length: [0x0, 0x10, 0x0] };
/// In arm9.
pub const TEXT_BOX_DEFAULT_WINDOW_PARAMS: Symbol =
    Symbol { name: "TEXT_BOX_DEFAULT_WINDOW_PARAMS", address: [0x209_B488, 0x209_AF4C, 0x209_B8A0], length: [0x0, 0x10, 0x0] };
/// In overlay10.
pub const THROWN_BLAST_SEED_DAMAGE_NORMAL: Symbol =
    Symbol { name: "THROWN_BLAST_SEED_DAMAGE_NORMAL", address: [0x22C_4F2C, 0x22C_45D4, 0x22C_5CBC], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const THROWN_ITEM_HIT_CHANCE: Symbol = Symbol { name: "THROWN_ITEM_HIT_CHANCE", address: [0x22C_5008, 0x22C_46B0, 0x22C_5D98], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const THUNDERBOLT_PARALYZE_CHANCE: Symbol =
    Symbol { name: "THUNDERBOLT_PARALYZE_CHANCE", address: [0x22C_4FB4, 0x22C_465C, 0x22C_5D44], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const THUNDERSHOCK_PARALYZE_CHANCE: Symbol =
    Symbol { name: "THUNDERSHOCK_PARALYZE_CHANCE", address: [0x22C_4DE8, 0x22C_4490, 0x22C_5B78], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const THUNDER_FANG_CRINGE_CHANCE: Symbol =
    Symbol { name: "THUNDER_FANG_CRINGE_CHANCE", address: [0x22C_4FD0, 0x22C_4678, 0x22C_5D60], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const THUNDER_FANG_PARALYZE_CHANCE: Symbol =
    Symbol { name: "THUNDER_FANG_PARALYZE_CHANCE", address: [0x22C_4E84, 0x22C_452C, 0x22C_5C14], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const TILESET_PROPERTIES: Symbol = Symbol { name: "TILESET_PROPERTIES", address: [0x22C_6C74, 0x22C_631C, 0x22C_7A04], length: [0x954, 0x954, 0x954] };
/// In overlay10.
pub const TRAP_ANIMATION_INFO: Symbol = Symbol { name: "TRAP_ANIMATION_INFO", address: [0x22C_83A8, 0x22C_7A50, 0x22C_9138], length: [0x34, 0x34, 0x34] };
/// In overlay10.
pub const TRI_ATTACK_STATUS_CHANCE: Symbol =
    Symbol { name: "TRI_ATTACK_STATUS_CHANCE", address: [0x22C_4FEC, 0x22C_4694, 0x22C_5D7C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const TWINEEDLE_POISON_CHANCE: Symbol = Symbol { name: "TWINEEDLE_POISON_CHANCE", address: [0x22C_4D9C, 0x22C_4444, 0x22C_5B2C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const TWISTER_CRINGE_CHANCE: Symbol = Symbol { name: "TWISTER_CRINGE_CHANCE", address: [0x22C_4FC0, 0x22C_4668, 0x22C_5D50], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const TYPE_ADVANTAGE_MASTER_CRIT_RATE: Symbol =
    Symbol { name: "TYPE_ADVANTAGE_MASTER_CRIT_RATE", address: [0x22C_4F38, 0x22C_45E0, 0x22C_5CC8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const TYPE_MATCHUP_COMBINATOR_TABLE: Symbol =
    Symbol { name: "TYPE_MATCHUP_COMBINATOR_TABLE", address: [0x22C_566C, 0x22C_4D14, 0x22C_63FC], length: [0x40, 0x40, 0x40] };
/// In overlay10.
pub const TYPE_MATCHUP_TABLE: Symbol = Symbol { name: "TYPE_MATCHUP_TABLE", address: [0x22C_6008, 0x22C_56B0, 0x22C_6D98], length: [0x288, 0x288, 0x288] };
/// In overlay10.
pub const VITAL_THROW_TURN_RANGE: Symbol = Symbol { name: "VITAL_THROW_TURN_RANGE", address: [0x22C_51B4, 0x22C_485C, 0x22C_5F44], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const WATERFALL_CRINGE_CHANCE: Symbol = Symbol { name: "WATERFALL_CRINGE_CHANCE", address: [0x22C_4D7C, 0x22C_4424, 0x22C_5B0C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const WATER_SPOUT_DAMAGE_MULT_TABLE: Symbol =
    Symbol { name: "WATER_SPOUT_DAMAGE_MULT_TABLE", address: [0x22C_52FC, 0x22C_49A4, 0x22C_608C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const WEATHER_BALL_DAMAGE_MULT_TABLE: Symbol =
    Symbol { name: "WEATHER_BALL_DAMAGE_MULT_TABLE", address: [0x22C_54AC, 0x22C_4B54, 0x22C_623C], length: [0x20, 0x20, 0x20] };
/// In overlay10.
pub const WEATHER_BALL_TYPE_TABLE: Symbol = Symbol { name: "WEATHER_BALL_TYPE_TABLE", address: [0x22C_5234, 0x22C_48DC, 0x22C_5FC4], length: [0x8, 0x8, 0x8] };
/// In overlay10.
pub const WEATHER_MOVE_TURN_COUNT: Symbol = Symbol { name: "WEATHER_MOVE_TURN_COUNT", address: [0x22C_4FAC, 0x22C_4654, 0x22C_5D3C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const WHIRLPOOL_CONSTRICTION_CHANCE: Symbol =
    Symbol { name: "WHIRLPOOL_CONSTRICTION_CHANCE", address: [0x22C_4DFC, 0x22C_44A4, 0x22C_5B8C], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const WISE_HEALER_PERCENTAGE: Symbol = Symbol { name: "WISE_HEALER_PERCENTAGE", address: [0x22C_4DC8, 0x22C_4470, 0x22C_5B58], length: [0x2, 0x0, 0x0] };
/// In overlay10.
pub const WISH_BONUS_REGEN: Symbol = Symbol { name: "WISH_BONUS_REGEN", address: [0x22C_5028, 0x22C_46D0, 0x22C_5DB8], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const WISH_TURN_RANGE: Symbol = Symbol { name: "WISH_TURN_RANGE", address: [0x22C_50EC, 0x22C_4794, 0x22C_5E7C], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const WONDER_GUMMI_IQ_GAIN: Symbol = Symbol { name: "WONDER_GUMMI_IQ_GAIN", address: [0x20A_1E14, 0x20A_1890, 0x20A_2C64], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const WRAP_TURN_RANGE: Symbol = Symbol { name: "WRAP_TURN_RANGE", address: [0x22C_5204, 0x22C_48AC, 0x22C_5F94], length: [0x4, 0x4, 0x4] };
/// In overlay10.
pub const WRING_OUT_DAMAGE_MULT_TABLE: Symbol =
    Symbol { name: "WRING_OUT_DAMAGE_MULT_TABLE", address: [0x22C_530C, 0x22C_49B4, 0x22C_609C], length: [0x10, 0x10, 0x10] };
/// In overlay10.
pub const YAWN_TURN_RANGE: Symbol = Symbol { name: "YAWN_TURN_RANGE", address: [0x22C_51B8, 0x22C_4860, 0x22C_5F48], length: [0x4, 0x4, 0x4] };
/// In arm9.
pub const ZINC_BAND_STAT_BOOST: Symbol = Symbol { name: "ZINC_BAND_STAT_BOOST", address: [0x20A_1E38, 0x20A_18B4, 0x20A_2C88], length: [0x2, 0x2, 0x2] };
/// In overlay10.
pub const ZINC_STAT_BOOST: Symbol = Symbol { name: "ZINC_STAT_BOOST", address: [0x22C_501C, 0x22C_46C4, 0x22C_5DAC], length: [0x2, 0x2, 0x2] };

/// Every symbol above.
pub const SYMBOLS: [Symbol; 268] = [
    ACID_PSYCHIC_LOWER_DEFENSE_CHANCE,
    ADVANCED_MENU_DEFAULT_WINDOW_PARAMS,
    AI_REGULAR_ATTACK_WEIGHTS,
    ALERT_BOX_DEFAULT_WINDOW_PARAMS,
    AQUA_RING_BONUS_REGEN,
    AQUA_RING_HEALING_COUNTDOWN,
    AQUA_RING_TURN_RANGE,
    ARENA_TRAP_ACTIVATION_CHANCE,
    AURORA_BEAM_LOWER_ATTACK_CHANCE,
    BAD_POISON_DAMAGE_COOLDOWN,
    BAD_POISON_DAMAGE_TABLE,
    BAD_WEATHER_DAMAGE,
    BELLY_LOST_PER_TURN,
    BLAZE_KICK_BURN_CHANCE,
    BOUNCE_DAMAGE_MULTIPLIER,
    BOUNCE_PARALYZE_CHANCE,
    BUBBLE_LOWER_SPEED_CHANCE,
    BUG_BUZZ_LOWER_SPECIAL_DEFENSE_CHANCE,
    BURN_DAMAGE,
    BURN_DAMAGE_COOLDOWN,
    BURN_DAMAGE_MULTIPLIER,
    BURN_TURN_RANGE,
    CACNEA_SPIKE_POWER,
    CALCIUM_STAT_BOOST,
    CHARGE_BEAM_BOOST_SPECIAL_ATTACK_CHANCE,
    CONFUSED_TURN_RANGE,
    CONSTRICTION_DAMAGE,
    CONSTRICTION_TURN_RANGE,
    CONVERSION2_TURN_RANGE,
    CONVERSION2_TYPE_TABLE,
    CORSOLA_TWIG_POWER,
    CRINGE_TURN_RANGE,
    CROSS_EYED_TURN_RANGE,
    CRUNCH_LOWER_DEFENSE_CHANCE,
    CURSE_DAMAGE_COOLDOWN,
    CURSE_TURN_RANGE,
    DECOY_TURN_RANGE,
    DEFAULT_MISSION_REWARD_TYPE_WEIGHTS,
    DEFENSIVE_STAT_STAGE_MULTIPLIERS,
    DEF_SCARF_STAT_BOOST,
    DESTINY_BOND_TURN_RANGE,
    DETECT_BAND_MOVE_ACCURACY_DROP,
    DIALOGUE_BOX_DEFAULT_WINDOW_PARAMS,
    DIG_DAMAGE_MULTIPLIER,
    DIVE_DAMAGE_MULTIPLIER,
    DRAGON_RAGE_FIXED_DAMAGE,
    DUNGEON_CONTROLS_MENU_WINDOW_PARAMS,
    DUNGEON_DATA_LIST,
    DUNGEON_RESTRICTIONS,
    DUNGEON_WINDOW_PARAMS_1,
    DUNGEON_WINDOW_PARAMS_2,
    DUNGEON_WINDOW_PARAMS_4,
    EARTH_POWER_LOWER_SPECIAL_DEFENSE_CHANCE,
    EATEN_BLAST_SEED_DAMAGE_NORMAL,
    EFFECT_ANIMATION_INFO,
    EMBARGO_TURN_RANGE,
    ENCORE_TURN_RANGE,
    ENTITIES,
    ERUPTION_DAMAGE_MULT_TABLE,
    EVENTS,
    EXCLUSIVE_ITEM_EFFECT_DATA,
    EXCLUSIVE_ITEM_EXP_BOOST,
    EXCLUSIVE_ITEM_STAT_BOOST_DATA,
    EXPLOSION_FIXED_DAMAGES,
    EXP_ELITE_EXP_BOOST,
    EXTRA_DUNGEON_DATA,
    FACADE_DAMAGE_MULTIPLIER,
    FAKE_OUT_CRINGE_CHANCE,
    FEMALE_ACCURACY_STAGE_MULTIPLIERS,
    FEMALE_EVASION_STAGE_MULTIPLIERS,
    FIRE_FANG_CRINGE_CHANCE,
    FIXED_ROOM_ENTITY_SPAWN_TABLE,
    FIXED_ROOM_MONSTER_SPAWN_STATS_TABLE,
    FIXED_ROOM_MONSTER_SPAWN_TABLE,
    FIXED_ROOM_PROPERTIES_TABLE,
    FLARE_BLITZ_BURN_CHANCE,
    FLASH_CANNON_LOWER_SPECIAL_DEFENSE_CHANCE,
    FLY_DAMAGE_MULTIPLIER,
    FOCUS_PUNCH_DAMAGE_MULTIPLIER,
    FROZEN_TURN_RANGE,
    GEO_PEBBLE_DAMAGE,
    GOLD_FANG_POWER,
    GOLD_THORN_POWER,
    GRAVELEROCK_DAMAGE,
    GUEST_MONSTER_DATA,
    GUEST_MONSTER_DATA2,
    GUMMI_BELLY_RESTORE_TABLE,
    GUMMI_STAT_BOOST,
    HEADBUTT_CRINGE_CHANCE,
    HEAL_BLOCK_TURN_RANGE,
    HEAL_ORDER_HP_RESTORATION,
    ICE_FANG_CRINGE_CHANCE,
    ICE_FANG_FREEZE_CHANCE,
    IMPRISON_TURN_RANGE,
    INFATUATED_TURN_RANGE,
    INGRAIN_BONUS_REGEN,
    INGRAIN_BONUS_REGEN_COOLDOWN,
    INGRAIN_TURN_RANGE,
    INTIMIDATOR_ACTIVATION_CHANCE,
    INVENTORY_MENU_DEFAULT_WINDOW_PARAMS,
    INVISIBLE_TURN_RANGE,
    IQ_GROUP_SKILLS,
    IQ_GUMMI_GAIN_TABLE,
    IQ_SKILLS,
    IQ_SKILL_RESTRICTIONS,
    IRON_STAT_BOOST,
    IRON_TAIL_LOWER_DEFENSE_CHANCE,
    IRON_THORN_POWER,
    ITEM_ANIMATION_INFO,
    ITEM_TABLES_PTRS_1,
    KECLEON_SHOP_ITEM_TABLE_LISTS_1,
    LAST_RESORT_DAMAGE_MULT_TABLE,
    LEECH_SEED_DAMAGE_COOLDOWN,
    LEECH_SEED_HP_DRAIN,
    LEECH_SEED_TURN_RANGE,
    LEVEL_TILEMAP_LIST,
    LIFE_SEED_HP_BOOST,
    LIGHT_SCREEN_TURN_RANGE,
    LUSTER_PURGE_LOWER_SPECIAL_DEFENSE_CHANCE,
    MAGIC_COAT_TURN_RANGE,
    MAGNET_RISE_TURN_RANGE,
    MALE_ACCURACY_STAGE_MULTIPLIERS,
    MALE_EVASION_STAGE_MULTIPLIERS,
    MAP_MARKER_PLACEMENTS,
    MATCHUP_IMMUNE_MULTIPLIER,
    MATCHUP_IMMUNE_MULTIPLIER_ERRATIC_PLAYER,
    MATCHUP_NEUTRAL_MULTIPLIER,
    MATCHUP_NEUTRAL_MULTIPLIER_ERRATIC_PLAYER,
    MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER,
    MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER,
    MATCHUP_SUPER_EFFECTIVE_MULTIPLIER,
    MATCHUP_SUPER_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER,
    METAL_CLAW_BOOST_ATTACK_CHANCE,
    METEOR_MASH_BOOST_ATTACK_CHANCE,
    METRONOME_TABLE,
    ME_FIRST_MULTIPLIER,
    MIRROR_COAT_TURN_RANGE,
    MIRROR_MOVE_TURN_RANGE,
    MISSION_FLOOR_RANKS_PTRS,
    MISSION_RANK_POINTS,
    MIST_BALL_LOWER_SPECIAL_ATTACK_CHANCE,
    MIST_TURN_RANGE,
    MONEY_QUANTITY_TABLE,
    MOONLIGHT_HP_RESTORATION_TABLE,
    MORNING_SUN_HP_RESTORATION_TABLE,
    MOVE_ANIMATION_INFO,
    MUNCH_BELT_STAT_BOOST,
    MUSIC_ID_TABLE,
    NATURAL_GIFT_ITEM_TABLE,
    NATURAL_LOG_VALUE_TABLE,
    NATURE_POWER_TABLE,
    NIGHTMARE_TURN_RANGE,
    OBJECTS,
    OCTAZOOKA_LOWER_ACCURACY_CHANCE,
    OFFENSIVE_STAT_STAGE_MULTIPLIERS,
    ORAN_BERRY_FULL_HP_BOOST,
    ORAN_BERRY_HP_RESTORATION,
    OREN_BERRY_DAMAGE,
    OUTLAW_MISSION_REWARD_TYPE_WEIGHTS,
    PARALYSIS_TURN_RANGE,
    PARTNER_TALK_KIND_TABLE,
    PERISH_SONG_TURN_RANGE,
    PITFALL_TRAP_DAMAGE,
    PLAY_OLD_GAME_BOOST,
    POISON_DAMAGE,
    POISON_DAMAGE_COOLDOWN,
    POISON_FANG_POISON_CHANCE,
    POISON_TAIL_POISON_CHANCE,
    PORTRAIT_LAYOUTS,
    POWER_BAND_STAT_BOOST,
    POWER_PITCHER_DAMAGE_MULTIPLIER,
    PROTECT_TURN_RANGE,
    PROTEIN_STAT_BOOST,
    QUICK_DODGER_MOVE_ACCURACY_DROP,
    QUIZ_ANSWER_ASSOCIATIONS,
    QUIZ_ANSWER_POINTS,
    QUIZ_ANSWER_STRINGS,
    QUIZ_QUESTION_STRINGS,
    RANDOM_MUSIC_ID_TABLE,
    RANK_UP_TABLE,
    RARE_FOSSIL_DAMAGE,
    RAZOR_WIND_DAMAGE_MULTIPLIER,
    RECRUITMENT_LEVEL_BOOST_TABLE,
    RECYCLE_SHOP_ITEM_LIST,
    REFLECT_TURN_RANGE,
    REST_TURN_RANGE,
    REVERSAL_DAMAGE_MULT_TABLE,
    ROLLOUT_DAMAGE_MULT_TABLE,
    ROOST_HP_RESTORATION_TABLE,
    SACRED_FIRE_BURN_CHANCE,
    SAFEGUARD_TURN_RANGE,
    SCOPE_LENS_CRIT_RATE_BOOST,
    SECONDARY_TERRAIN_TYPES,
    SECRET_POWER_EFFECT_CHANCE,
    SEED_FLARE_LOWER_SPECIAL_DEFENSE_CHANCE,
    SENTRY_DUTY_MONSTER_IDS,
    SETANIMATION_TABLE,
    SET_DAMAGE_STATUS_DAMAGE,
    SET_DAMAGE_TURN_RANGE,
    SET_REFLECT_STATUS_TURN_RANGE,
    SHADOW_BALL_LOWER_SPECIAL_DEFENSE_CHANCE,
    SHADOW_FORCE_DAMAGE_MULTIPLIER,
    SHADOW_TAG_ACTIVATION_CHANCE,
    SILVER_SPIKE_POWER,
    SILVER_WIND_BOOST_CHANCE,
    SIMPLE_MENU_DEFAULT_WINDOW_PARAMS,
    SITRUS_BERRY_FULL_HP_BOOST,
    SITRUS_BERRY_HP_RESTORATION,
    SKULL_BASH_DAMAGE_MULTIPLIER,
    SKY_ATTACK_CRINGE_CHANCE,
    SKY_ATTACK_DAMAGE_MULTIPLIER,
    SLEEPLESSNESS_TURN_RANGE,
    SLEEP_TURN_RANGE,
    SNATCH_TURN_RANGE,
    SNORE_CRINGE_CHANCE,
    SOLARBEAM_DAMAGE_MULTIPLIER,
    SONICBOOM_FIXED_DAMAGE,
    SPAWN_CAP_NO_MONSTER_HOUSE,
    SPAWN_CAP_WITH_MONSTER_HOUSE,
    SPAWN_COOLDOWN,
    SPAWN_COOLDOWN_THIEF_ALERT,
    SPECIAL_BAND_STAT_BOOST,
    SPECIAL_EPISODE_MAIN_CHARACTERS,
    SPECIAL_MONSTER_MOVE_ANIMATION_INFO,
    SPEED_BOOST_TURNS,
    SPEED_BOOST_TURN_RANGE,
    SPEED_LOWER_TURN_RANGE,
    SPORT_CONDITION_TURN_RANGE,
    STARTERS_HERO_IDS,
    STARTERS_PARTNER_IDS,
    STARTERS_STRINGS,
    STEEL_WING_BOOST_DEFENSE_CHANCE,
    STICK_POWER,
    SURE_SHOT_TURN_RANGE,
    SYNTHESIS_HP_RESTORATION_TABLE,
    TAUNT_TURN_RANGE,
    TEAM_SELECTION_MENU_DEFAULT_WINDOW_PARAMS,
    TEXT_BOX_DEFAULT_WINDOW_PARAMS,
    THROWN_BLAST_SEED_DAMAGE_NORMAL,
    THROWN_ITEM_HIT_CHANCE,
    THUNDERBOLT_PARALYZE_CHANCE,
    THUNDERSHOCK_PARALYZE_CHANCE,
    THUNDER_FANG_CRINGE_CHANCE,
    THUNDER_FANG_PARALYZE_CHANCE,
    TILESET_PROPERTIES,
    TRAP_ANIMATION_INFO,
    TRI_ATTACK_STATUS_CHANCE,
    TWINEEDLE_POISON_CHANCE,
    TWISTER_CRINGE_CHANCE,
    TYPE_ADVANTAGE_MASTER_CRIT_RATE,
    TYPE_MATCHUP_COMBINATOR_TABLE,
    TYPE_MATCHUP_TABLE,
    VITAL_THROW_TURN_RANGE,
    WATERFALL_CRINGE_CHANCE,
    WATER_SPOUT_DAMAGE_MULT_TABLE,
    WEATHER_BALL_DAMAGE_MULT_TABLE,
    WEATHER_BALL_TYPE_TABLE,
    WEATHER_MOVE_TURN_COUNT,
    WHIRLPOOL_CONSTRICTION_CHANCE,
    WISE_HEALER_PERCENTAGE,
    WISH_BONUS_REGEN,
    WISH_TURN_RANGE,
    WONDER_GUMMI_IQ_GAIN,
    WRAP_TURN_RANGE,
    WRING_OUT_DAMAGE_MULT_TABLE,
    YAWN_TURN_RANGE,
    ZINC_BAND_STAT_BOOST,
    ZINC_STAT_BOOST,
];
