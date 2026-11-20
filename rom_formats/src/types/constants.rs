//! The rules' numbers, gathered.

use super::*;

pub fn load_constants(rom: &NdsRom) -> Result<Constants, RomError> {
    let region = match rom.game_code().as_bytes().get(3) {
        Some(b'P') => 0,
        Some(b'E') => 1,
        Some(b'J') => 2,
        _ => return Err(RomError::Truncated),
    };
    let (base, overlay) = rom.overlay(OVERLAY_ID).ok_or(RomError::Truncated)?;
    // All of them are 16-bit values.
    let read = |address: u32| -> Result<i32, RomError> {
        let off = address.checked_sub(base).ok_or(RomError::Truncated)? as usize;
        let b = overlay.get(off..off + 2).ok_or(RomError::Truncated)?;
        Ok(i16::from_le_bytes([b[0], b[1]]) as i32)
    };
    // `BELLY_LOST_PER_TURN` lives in overlay 29 and is 32 bits wide.
    let (base29, overlay29) = rom.overlay(29).ok_or(RomError::Truncated)?;
    let off = BELLY_LOST_PER_TURN[region].checked_sub(base29).ok_or(RomError::Truncated)? as usize;
    let b = overlay29.get(off..off + 4).ok_or(RomError::Truncated)?;
    let belly_lost_per_turn = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    // 21 stages of 32-bit multipliers each.
    let stages = |address: u32| -> Result<[i32; STAGE_COUNT], RomError> {
        let off = address.checked_sub(base).ok_or(RomError::Truncated)? as usize;
        let table = overlay.get(off..off + STAGE_COUNT * 4).ok_or(RomError::Truncated)?;
        Ok(std::array::from_fn(|stage| {
            let b = &table[stage * 4..stage * 4 + 4];
            i32::from_le_bytes([b[0], b[1], b[2], b[3]])
        }))
    };
    let range = |address: u32| Ok::<_, RomError>((read(address)?, read(address + 2)?));
    // (Four 32-bit numbers, all small.)
    let quads = |address: u32| Ok::<_, RomError>([read(address)?, read(address + 4)?, read(address + 8)?, read(address + 12)?]);
    Ok(Constants {
        offensive_stages: stages(OFFENSIVE_STAGES[region])?,
        defensive_stages: stages(DEFENSIVE_STAGES[region])?,
        accuracy_stages: stages(ACCURACY_STAGES[region])?,
        evasion_stages: stages(EVASION_STAGES[region])?,
        female_accuracy_stages: stages(FEMALE_ACCURACY_STAGES[region])?,
        female_evasion_stages: stages(FEMALE_EVASION_STAGES[region])?,
        charged_multipliers: {
            let mut multipliers = [0; CHARGED_MOVES.len()];
            for (multiplier, addresses) in multipliers.iter_mut().zip(CHARGED_MULTIPLIERS) {
                *multiplier = read(addresses[region])?;
            }
            multipliers
        },
        speed_boost_turns: range(SPEED_BOOST_TURN_RANGE[region])?,
        speed_lower_turns: range(SPEED_LOWER_TURN_RANGE[region])?,
        meteor_mash_chance: read(METEOR_MASH_CHANCE[region])?,
        metal_claw_chance: read(METAL_CLAW_CHANCE[region])?,
        steel_wing_chance: read(STEEL_WING_CHANCE[region])?,
        silver_wind_chance: read(SILVER_WIND_CHANCE[region])?,
        mist_ball_chance: read(MIST_BALL_CHANCE[region])?,
        charge_beam_chance: read(CHARGE_BEAM_CHANCE[region])?,
        luster_purge_chance: read(LUSTER_PURGE_CHANCE[region])?,
        reflect_turns: range(REFLECT_TURN_RANGE[region])?,
        light_screen_turns: range(LIGHT_SCREEN_TURN_RANGE[region])?,
        heal_tables: {
            // One 16-bit amount per weather.
            let mut tables = [[0; 8]; 4];
            for (table, addresses) in tables.iter_mut().zip(HEAL_AMOUNTS) {
                for (weather, amount) in table.iter_mut().enumerate() {
                    *amount = read(addresses[region] + weather as u32 * 2)?;
                }
            }
            tables
        },
        heal_order_hp: read(HEAL_AMOUNTS[4][region])?,
        // Set from the leader's species by the caller.
        regen_speed: 0,
        wish_bonus_regen: read(WISH_BONUS_REGEN[region])?,
        wish_turns: range(WISH_TURN_RANGE[region])?,
        ingrain_regen: read(INGRAIN_REGEN[region])?,
        ingrain_cooldown: read(INGRAIN_COOLDOWN[region])?,
        ingrain_turns: range(INGRAIN_TURN_RANGE[region])?,
        aqua_ring_regen: read(AQUA_RING_REGEN[region])?,
        aqua_ring_cooldown: read(AQUA_RING_COOLDOWN[region])?,
        aqua_ring_turns: range(AQUA_RING_TURN_RANGE[region])?,
        chestnut_damage: read(CHESTNUT_DAMAGE[region])?,
        spiked_tile_damage: read(SPIKED_TILE_DAMAGE[region])?,
        stealth_rock_damage: read(STEALTH_ROCK_DAMAGE[region])?,
        explosion_damages: {
            // Three 32-bit values, by radius.
            let off = EXPLOSION_DAMAGES[region].checked_sub(base29).ok_or(RomError::Truncated)?;
            let table = overlay29.get(off as usize..off as usize + 12).ok_or(RomError::Truncated)?;
            std::array::from_fn(|radius| {
                let b = &table[radius * 4..radius * 4 + 4];
                i32::from_le_bytes([b[0], b[1], b[2], b[3]])
            })
        },
        blowback_damage: read(BLOWBACK_DAMAGE[region])?,
        endure_turns: range(ENDURE_TURN_RANGE[region])?,
        rest_turns: range(REST_TURN_RANGE[region])?,
        curse_turns: range(CURSE_TURN_RANGE[region])?,
        curse_cooldown: read(CURSE_COOLDOWN[region])?,
        weather_move_turns: read(WEATHER_MOVE_TURNS[region])?,
        bad_weather_damage: read(BAD_WEATHER_DAMAGE[region])?,
        held_boosts: {
            let mut boosts = [0; 5];
            for (boost, addresses) in boosts.iter_mut().zip(HELD_BOOSTS) {
                *boost = arm9_values(rom, addresses[region], 1, 2)?[0];
            }
            boosts
        },
        scope_lens_crit: read(SCOPE_LENS_CRIT[region])?,
        detect_band_drop: read(DETECT_BAND_DROP[region])?,
        quick_dodger_drop: read(QUICK_DODGER_DROP[region])?,
        type_advantage_crit: read(TYPE_ADVANTAGE_CRIT[region])?,
        oran_berry_full_hp_boost: read(ORAN_BERRY_FULL_HP_BOOST[region])?,
        sitrus_berry_full_hp_boost: read(SITRUS_BERRY_FULL_HP_BOOST[region])?,
        life_seed_hp_boost: read(LIFE_SEED_HP_BOOST[region])?,
        oren_berry_damage: read(OREN_BERRY_DAMAGE[region])?,
        vitamin_boosts: [
            read(VITAMIN_BOOSTS[0][region])?,
            read(VITAMIN_BOOSTS[1][region])?,
            read(VITAMIN_BOOSTS[2][region])?,
            read(VITAMIN_BOOSTS[3][region])?,
        ],
        thrown_hit_chance: read(THROWN_ITEM_HIT_CHANCE[region])?,
        thrown_powers: {
            let mut powers = [0; 11];
            for (power, addresses) in powers[1..].iter_mut().zip(THROWN_POWERS) {
                *power = read(addresses[region])?;
            }
            powers
        },
        blast_seed_damage: (read(EATEN_BLAST_SEED_DAMAGE[region])?, read(THROWN_BLAST_SEED_DAMAGE[region])?),
        spawn_cooldown: read(SPAWN_COOLDOWN[region])?,
        spawn_cooldown_alert: read(SPAWN_COOLDOWN_THIEF_ALERT[region])?,
        spawn_cap: read(SPAWN_CAP_NO_MONSTER_HOUSE[region])?,
        spawn_cap_monster_house: read(SPAWN_CAP_WITH_MONSTER_HOUSE[region])?,
        ai_regular_attack_weights: {
            let mut weights = [0; 5];
            for (known, weight) in weights.iter_mut().enumerate() {
                *weight = read(AI_REGULAR_ATTACK_WEIGHTS[region] + known as u32 * 2)?;
            }
            weights
        },
        weather_ball_types: {
            let off = WEATHER_BALL_TYPES[region].checked_sub(base).ok_or(RomError::Truncated)?;
            let table = overlay.get(off as usize..off as usize + 8).ok_or(RomError::Truncated)?;
            std::array::from_fn(|weather| table[weather])
        },
        weather_ball_multipliers: {
            // 32-bit values; they fit in 16 bits.
            let mut multipliers = [0; 8];
            for (weather, multiplier) in multipliers.iter_mut().enumerate() {
                *multiplier = read(WEATHER_BALL_MULTIPLIERS[region] + weather as u32 * 4)?;
            }
            multipliers
        },
        protect_turns: range(PROTECT_TURN_RANGE[region])?,
        safeguard_turns: range(SAFEGUARD_TURN_RANGE[region])?,
        mist_turns: range(MIST_TURN_RANGE[region])?,
        lucky_chant_turns: range(LUCKY_CHANT_TURN_RANGE[region])?,
        sonicboom_damage: read(SONICBOOM_DAMAGE[region])?,
        dragon_rage_damage: read(DRAGON_RAGE_DAMAGE[region])?,
        burn_damage: read(BURN_DAMAGE[region])?,
        burn_cooldown: read(BURN_COOLDOWN[region])?,
        burn_multiplier: read(BURN_MULTIPLIER[region])?,
        burn_turns: range(BURN_TURN_RANGE[region])?,
        paralysis_turns: range(PARALYSIS_TURN_RANGE[region])?,
        frozen_turns: range(FROZEN_TURN_RANGE[region])?,
        cringe_turns: range(CRINGE_TURN_RANGE[region])?,
        confused_turns: range(CONFUSED_TURN_RANGE[region])?,
        belly_lost_per_turn,
        oran_berry_hp: read(ORAN_BERRY_HP[region])?,
        sitrus_berry_hp: read(SITRUS_BERRY_HP[region])?,
        poison_damage: read(POISON_DAMAGE[region])?,
        poison_cooldown: read(POISON_COOLDOWN[region])?,
        bad_poison_cooldown: read(BAD_POISON_COOLDOWN[region])?,
        leech_seed_drain: read(LEECH_SEED_DRAIN[region])?,
        leech_seed_cooldown: read(LEECH_SEED_COOLDOWN[region])?,
        leech_seed_turns: range(LEECH_SEED_TURN_RANGE[region])?,
        yawn_turns: range(YAWN_TURN_RANGE[region])?,
        taunt_turns: range(TAUNT_TURN_RANGE[region])?,
        sure_shot_turns: range(SURE_SHOT_TURN_RANGE[region])?,
        infatuated_turns: range(INFATUATED_TURN_RANGE[region])?,
        invisible_turns: range(INVISIBLE_TURN_RANGE[region])?,
        magic_coat_turns: range(MAGIC_COAT_TURN_RANGE[region])?,
        snatch_turns: range(SNATCH_TURN_RANGE[region])?,
        sport_turns: range(SPORT_TURN_RANGE[region])?,
        sleepless_turns: range(SLEEPLESS_TURN_RANGE[region])?,
        magnet_rise_turns: range(MAGNET_RISE_TURN_RANGE[region])?,
        embargo_turns: range(EMBARGO_TURN_RANGE[region])?,
        encore_turns: range(ENCORE_TURN_RANGE[region])?,
        cross_eyed_turns: range(CROSS_EYED_TURN_RANGE[region])?,
        nightmare_turns: range(NIGHTMARE_TURN_RANGE[region])?,
        heal_block_turns: range(HEAL_BLOCK_TURN_RANGE[region])?,
        counter_turns: range(SET_REFLECT_STATUS_TURN_RANGE[region])?,
        mirror_coat_turns: range(MIRROR_COAT_TURN_RANGE[region])?,
        perish_turns: range(PERISH_SONG_TURN_RANGE[region])?,
        set_damage_turns: range(SET_DAMAGE_TURN_RANGE[region])?,
        set_damage: read(SET_DAMAGE_STATUS_DAMAGE[region])?,
        mirror_move_turns: range(MIRROR_MOVE_TURN_RANGE[region])?,
        imprison_turns: range(IMPRISON_TURN_RANGE[region])?,
        wrap_turns: range(WRAP_TURN_RANGE[region])?,
        return_damage: {
            let mut table = [(-1, 1); 12];
            for (at, entry) in table.iter_mut().enumerate() {
                *entry = (read(RETURN_DAMAGE[region] + at as u32 * 4)?, read(RETURN_DAMAGE[region] + at as u32 * 4 + 2)?);
            }
            table
        },
        frustration_damage: {
            let mut table = [(-1, 1); 12];
            for (at, entry) in table.iter_mut().enumerate() {
                *entry = (read(FRUSTRATION_DAMAGE[region] + at as u32 * 4)?, read(FRUSTRATION_DAMAGE[region] + at as u32 * 4 + 2)?);
            }
            table
        },
        magnitude_damage: {
            let mut table = [0; 7];
            for (at, damage) in table.iter_mut().enumerate() {
                *damage = read(MAGNITUDE_DAMAGE[region] + at as u32 * 2)?;
            }
            table
        },
        vacuum_cut_damage: read(VACUUM_CUT_DAMAGE[region])?,
        nightmare_damage: read(NIGHTMARE_DAMAGE[region])?,
        napping_heal: read(NAPPING_HEAL[region])?,
        destiny_bond_turns: range(DESTINY_BOND_TURN_RANGE[region])?,
        transform_turns: range(TRANSFORM_TURN_RANGE[region])?,
        gummi_iq: square18(arm9_values(rom, IQ_GUMMI_GAIN_TABLE[region], 18 * 18, 2)?),
        gummi_belly: square18(arm9_values(rom, GUMMI_BELLY_RESTORE_TABLE[region], 18 * 18, 2)?),
        wonder_gummi_iq: arm9_values(rom, WONDER_GUMMI_IQ_GAIN[region], 1, 2)?[0],
        grimy_food_chance: read(GRIMY_FOOD_CHANCE[region])?,
        gummi_boost_chance: read(GUMMI_BOOST_CHANCE[region])?,
        gummi_all_stats_odds: read(GUMMI_ALL_STATS_ODDS[region])?,
        gummi_stat_boost: arm9_values(rom, GUMMI_STAT_BOOST[region], 1, 2)?[0],
        exp_elite_boost: read(EXP_ELITE_EXP_BOOST[region])?,
        me_first: read(ME_FIRST_MULTIPLIER[region])?,
        facade_mult: read(FACADE_DAMAGE_MULTIPLIER[region])?,
        reversal_mult: quads(REVERSAL_DAMAGE_MULT_TABLE[region])?,
        water_spout_mult: quads(WATER_SPOUT_DAMAGE_MULT_TABLE[region])?,
        eruption_mult: quads(ERUPTION_DAMAGE_MULT_TABLE[region])?,
        wring_out_mult: quads(WRING_OUT_DAMAGE_MULT_TABLE[region])?,
        constriction_chance: read(CONSTRICTION_CHANCE[region])?,
        constriction_turns: range(CONSTRICTION_TURN_RANGE[region])?,
        constriction_damage: read(CONSTRICTION_DAMAGE[region])?,
        constriction_cooldown: read(CONSTRICTION_COOLDOWN[region])?,
        recharge_turns: range(RECHARGE_TURN_RANGE[region])?,
        secret_power_chance: read(SECRET_POWER_EFFECT_CHANCE[region])?,
        exclusive_exp_boost: read(EXCLUSIVE_ITEM_EXP_BOOST[region])?,
        exclusive_reaction_chance: read(EXCLUSIVE_REACTION_CHANCE[region])?,
        speed_boost_every: read(SPEED_BOOST_TURNS[region])?,
        wise_healer_percent: read(WISE_HEALER_PERCENTAGE[region])?,
        intimidator_chance: read(INTIMIDATOR_ACTIVATION_CHANCE[region])?,
        power_pitcher: read(POWER_PITCHER_DAMAGE_MULTIPLIER[region])?,
        arena_trap_chance: read(ARENA_TRAP_ACTIVATION_CHANCE[region])?,
        shadow_tag_chance: read(SHADOW_TAG_ACTIVATION_CHANCE[region])?,
        link_belly: {
            let at = LINK_BELLY[region];
            [read(at)?, read(at + 4)?, read(at + 8)?, read(at + 12)?, read(at + 16)?]
        },
        natural_gift: {
            // 34 entries of 6 bytes: item, type, padding, power added.
            let entry = |at: u32| -> Result<[i32; 3], RomError> {
                let off = at.checked_sub(base).ok_or(RomError::Truncated)? as usize;
                let b = overlay.get(off..off + 6).ok_or(RomError::Truncated)?;
                Ok([u16::from_le_bytes([b[0], b[1]]) as i32, b[2] as i32, i16::from_le_bytes([b[4], b[5]]) as i32])
            };
            let mut halves = [[[0; 3]; 17]; 2];
            for index in 0..34 {
                halves[index / 17][index % 17] = entry(NATURAL_GIFT_ITEM_TABLE[region] + index as u32 * 6)?;
            }
            halves
        },
        last_resort_mult: quads(LAST_RESORT_DAMAGE_MULT_TABLE[region])?,
        rollout_mult: {
            let off = ROLLOUT_DAMAGE_MULT_TABLE[region].checked_sub(base29).ok_or(RomError::Truncated)? as usize;
            let table = overlay29.get(off..off + 40).ok_or(RomError::Truncated)?;
            std::array::from_fn(|hit| i32::from_le_bytes([table[hit * 4], table[hit * 4 + 1], table[hit * 4 + 2], table[hit * 4 + 3]]))
        },
        vital_throw_turns: range(VITAL_THROW_TURN_RANGE[region])?,
        decoy_turns: range(DECOY_TURN_RANGE[region])?,
        conversion2_turns: range(CONVERSION2_TURN_RANGE[region])?,
        conversion2_types: {
            let kinds = arm9_values(rom, CONVERSION2_TYPES[region], 18, 1)?;
            std::array::from_fn(|kind| kinds.get(kind).copied().unwrap_or(0) as u8)
        },
        small_food: {
            const BERRIES_SEEDS_VITAMINS: u8 = 2;
            let mut bits = [0u64; 10];
            if let Some(items) = rom.file("BALANCE/item_p.bin").and_then(|file| crate::items::ItemData::parse(file).ok()) {
                for id in 0..640 {
                    if items.category(id) == Some(BERRIES_SEEDS_VITAMINS) {
                        bits[id / 64] |= 1 << (id % 64);
                    }
                }
            }
            bits
        },
        metronome: {
            let mut table = [[0; 28]; 6];
            for index in 0..168 {
                table[index / 28][index % 28] = read(METRONOME_TABLE[region] + index as u32 * 8)? as u16;
            }
            table
        },
        secondary_chances: {
            let mut chances = [0; 32];
            for (chance, addresses) in chances.iter_mut().zip(SECONDARY_CHANCES) {
                *chance = read(addresses[region])?;
            }
            chances
        },
        tri_attack_chance: read(TRI_ATTACK_CHANCE[region])?,
        bad_poison_damage: {
            let mut table = [0; 30];
            for (tick, damage) in table.iter_mut().enumerate() {
                *damage = read(BAD_POISON_DAMAGE[region] + tick as u32 * 2)?;
            }
            table
        },
        sleep_turns: (read(SLEEP_TURN_RANGE[region])?, read(SLEEP_TURN_RANGE[region] + 2)?),
        pitfall_trap_damage: read(PITFALL_TRAP_DAMAGE[region])?,
    })
}
