use super::*;

/// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
#[test]
#[ignore]
fn real_rom_money_amounts_grow_with_their_code() {
    let rom = crate::nds::test_rom();
    let table = load_money_table(&rom).unwrap();
    println!("money: {:?} ... {:?}", &table[..8], &table[96..]);
    assert_eq!(table.len(), 100);
    assert!(table[1] >= 1 && table.windows(2).skip(1).all(|pair| pair[0] <= pair[1]));
    assert!(table[99] <= 99_999);
}

/// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
#[test]
#[ignore]
fn real_rom_recruit_boosts_grow_with_the_level() {
    let rom = crate::nds::test_rom();
    let boosts = load_recruit_boosts(&rom).unwrap();
    println!("recruit boosts by level: {boosts:?}");
    assert!(boosts.len() == 102 && boosts[100] > boosts[5] && boosts.iter().all(|&boost| (-1000..=1000).contains(&boost)));
}

#[test]
#[ignore]
fn real_rom_constants_are_sane() {
    use rules::dungeon::status::NEUTRAL_STAGE;
    let rom = crate::nds::test_rom();
    let c = load_constants(&rom).unwrap();
    println!("{c:?}");
    assert!((1..=999).contains(&c.oran_berry_hp) && (1..=999).contains(&c.sitrus_berry_hp));
    println!(
        "full HP boosts {} / {}, life seed {}, oren {}, vitamins {:?}",
        c.oran_berry_full_hp_boost, c.sitrus_berry_full_hp_boost, c.life_seed_hp_boost, c.oren_berry_damage, c.vitamin_boosts
    );
    println!("thrown: {} % to hit, {:?}", c.thrown_hit_chance, c.thrown_powers);
    assert!((50..=100).contains(&c.thrown_hit_chance));
    println!("blast seed: {:?}", c.blast_seed_damage);
    assert!((1..=200).contains(&c.blast_seed_damage.0));
    assert!((1..=200).contains(&c.blast_seed_damage.1));
    assert!(c.thrown_powers[1..].iter().all(|power| (1..=200).contains(power)));
    assert!((0..=10).contains(&c.oran_berry_full_hp_boost));
    assert!((1..=10).contains(&c.sitrus_berry_full_hp_boost));
    assert!((1..=10).contains(&c.life_seed_hp_boost) && (1..=50).contains(&c.oren_berry_damage));
    assert!(c.vitamin_boosts.iter().all(|boost| (1..=10).contains(boost)));
    assert!((1..=50).contains(&c.poison_damage) && (1..=100).contains(&c.poison_cooldown));
    assert!((1..=100).contains(&c.bad_poison_cooldown) && c.bad_poison_damage.iter().all(|&damage| (1..=50).contains(&damage)));
    println!(
        "leech seed {} every {} for {:?}, yawn {:?}, taunt {:?}",
        c.leech_seed_drain, c.leech_seed_cooldown, c.leech_seed_turns, c.yawn_turns, c.taunt_turns
    );
    assert!((1..=50).contains(&c.leech_seed_drain) && (1..=20).contains(&c.leech_seed_cooldown));
    assert!(c.yawn_turns.0 >= 1 && c.taunt_turns.0 >= 1 && c.leech_seed_turns.0 >= 1);
    // pmdsky-debug's documented values.
    assert_eq!((c.sure_shot_turns, c.infatuated_turns, c.invisible_turns), ((10, 12), (4, 6), (20, 30)));
    // (Magic Coat: (10, 14) in pmdsky-debug's notes, (10, 12) in the EU
    // dump.)
    assert!(c.magic_coat_turns.0 == 10 && c.snatch_turns == (3, 5));
    assert!(c.sport_turns.0 == 10 && c.sleepless_turns.0 >= 1);
    println!("magnet rise {:?}, embargo {:?}", c.magnet_rise_turns, c.embargo_turns);
    assert!(c.magnet_rise_turns.0 >= 1 && c.embargo_turns.0 >= 1 && c.encore_turns.0 >= 1 && c.cross_eyed_turns.0 >= 1);
    println!("nightmare {:?}, heal block {:?}, metronome {:?}", c.nightmare_turns, c.heal_block_turns, &c.metronome[0][..8]);
    assert!(c.nightmare_turns == (4, 8) && c.heal_block_turns == (4, 10));
    println!("counter {:?}, mirror coat {:?}, perish {:?}", c.counter_turns, c.mirror_coat_turns, c.perish_turns);
    assert!(c.counter_turns == (6, 12) && c.mirror_coat_turns.0 >= 1 && c.perish_turns.0 >= 1);
    println!("set damage {:?} {}, mirror move {:?}, imprison {:?}", c.set_damage_turns, c.set_damage, c.mirror_move_turns, c.imprison_turns);
    assert!(c.set_damage > 0 && c.set_damage_turns.0 >= 1 && c.mirror_move_turns.0 >= 1 && c.imprison_turns.0 >= 1);
    println!("wrap {:?}", c.wrap_turns);
    println!("return {:?}, frustration {:?}, magnitude {:?}, vacuum cut {}", c.return_damage, c.frustration_damage, c.magnitude_damage, c.vacuum_cut_damage);
    assert!(c.return_damage[0] == (50, 5) && c.magnitude_damage[6] == 40 && c.vacuum_cut_damage > 0 && c.nightmare_damage == 8 && c.napping_heal == 999);
    println!("destiny bond {:?}, transform {:?}", c.destiny_bond_turns, c.transform_turns);
    println!("gummi iq {:?}, belly {:?}, wonder {}, grimy {}", c.gummi_iq[2], c.gummi_belly[2], c.wonder_gummi_iq, c.grimy_food_chance);
    assert!(c.gummi_iq[2][2] == 5 && c.gummi_belly[2][2] == 30 && c.wonder_gummi_iq == 15 && (1..=100).contains(&c.grimy_food_chance));
    assert!(c.gummi_boost_chance == 25 && c.gummi_all_stats_odds == 16 && c.gummi_stat_boost == 1);
    println!("exp elite {}", c.exp_elite_boost);
    assert!((1..=200).contains(&c.exp_elite_boost));
    assert_eq!(c.me_first, 384);
    println!("rollout {:?}, last resort {:?}, recharge {:?}", c.rollout_mult, c.last_resort_mult, c.recharge_turns);
    assert_eq!(c.recharge_turns, (1, 1));
    assert!(c.vital_throw_turns.0 >= 1);
    assert!((1..=100).contains(&c.secret_power_chance) && (1..=200).contains(&c.exclusive_exp_boost));
    assert_eq!(c.exclusive_reaction_chance, 30);
    assert_eq!(c.link_belly, [1, 1, 1, 2, 4]);
    println!("speed boost every {}", c.speed_boost_every);
    assert!(c.speed_boost_every > 0);
    println!("arena trap {}, shadow tag {}", c.arena_trap_chance, c.shadow_tag_chance);
    assert!((1..=100).contains(&c.arena_trap_chance) && (1..=100).contains(&c.shadow_tag_chance));
    assert!(c.wise_healer_percent == 15 && c.intimidator_chance == 32 && c.power_pitcher == 384);
    let exclusive = load_exclusive_items(&rom).unwrap();
    // The Prism Ruff is Eevee's (species 133).
    println!("exclusive: {} items, Prism Ruff {:?} {:?}, boosts {:?}", exclusive.kinds.len(), exclusive.kinds[0], exclusive.effects[0], &exclusive.boosts[..4]);
    assert!(exclusive.kinds.len() == 956 && exclusive.kinds[0] == (10, 133) && exclusive.for_monster(444, 133, 0, (1, 0)));
    println!("natural gift {:?}", &c.natural_gift[0][..4]);
    assert!(c.natural_gift[0][0][0] > 0 && c.natural_gift.iter().flatten().all(|entry| entry[1] < 18));
    assert!(c.rollout_mult[0] >= 0x100 && c.rollout_mult[4] > c.rollout_mult[0]);
    println!("constriction {} % {:?} turns, {} HP every {}", c.constriction_chance, c.constriction_turns, c.constriction_damage, c.constriction_cooldown);
    assert!((1..=100).contains(&c.constriction_chance) && c.constriction_turns.0 >= 1 && c.constriction_damage > 0 && c.constriction_cooldown > 0);
    println!(
        "facade {}, reversal {:?}, water spout {:?}, eruption {:?}, wring out {:?}",
        c.facade_mult, c.reversal_mult, c.water_spout_mult, c.eruption_mult, c.wring_out_mult
    );
    assert!(c.facade_mult == 0x200 && c.reversal_mult[0] > c.reversal_mult[3] && c.eruption_mult[3] > c.eruption_mult[0]);
    println!("decoy {:?}, conversion 2 {:?} {:?}", c.decoy_turns, c.conversion2_turns, c.conversion2_types);
    assert!(c.decoy_turns == (20, 50) && c.conversion2_turns.0 == 10 && c.conversion2_types[1] == 14);
    // (The Oran Berry, 70, and the Apple, 109, which is food.)
    assert!(c.small_food[1] >> (70 - 64) & 1 == 1 && c.small_food[1] >> (109 - 64) & 1 == 0);
    assert!(c.destiny_bond_turns.0 >= 1 && c.transform_turns.0 >= 1 && c.transform_turns.1 < 200);
    assert!(c.wrap_turns.0 >= 1);
    assert!(c.metronome.iter().flatten().all(|&id| (1..560).contains(&id)));
    println!("secondary chances {:?}, Tri Attack {}", c.secondary_chances, c.tri_attack_chance);
    assert!(c.secondary_chances.iter().chain([&c.tri_attack_chance]).all(|&chance| (1..=100).contains(&chance)));
    assert!(c.sleep_turns.0 >= 1 && c.sleep_turns.0 <= c.sleep_turns.1);
    // pmdsky-debug documents these two values.
    assert_eq!(c.pitfall_trap_damage, 5);
    assert_eq!(c.belly_lost_per_turn, 0x199A);
    println!("accuracy {:x?}\nevasion {:x?}", c.accuracy_stages, c.evasion_stages);
    assert!((1..=200).contains(&c.sonicboom_damage) && (1..=200).contains(&c.dragon_rage_damage));
    for chance in [c.meteor_mash_chance, c.metal_claw_chance, c.steel_wing_chance, c.silver_wind_chance].into_iter().chain([
        c.mist_ball_chance,
        c.charge_beam_chance,
        c.luster_purge_chance,
    ]) {
        assert!((1..=100).contains(&chance), "{chance}");
    }
    println!("heal {:?} {} weather ball {:?} {:x?}", c.heal_tables, c.heal_order_hp, c.weather_ball_types, c.weather_ball_multipliers);
    assert!(c.heal_tables.iter().flatten().all(|amount| (1..=999).contains(amount)));
    assert!((1..=999).contains(&c.heal_order_hp));
    println!("blowback {} endure {:?} rest {:?} curse {:?} every {}", c.blowback_damage, c.endure_turns, c.rest_turns, c.curse_turns, c.curse_cooldown);
    println!("wish {} {:?}", c.wish_bonus_regen, c.wish_turns);
    // pmdsky-debug documents these values.
    assert_eq!((c.ingrain_regen, c.ingrain_cooldown, c.ingrain_turns), (10, 5, (15, 30)));
    assert_eq!((c.aqua_ring_regen, c.aqua_ring_cooldown, c.aqua_ring_turns), (8, 4, (8, 16)));
    assert_eq!(c.wish_turns, (10, 12));
    assert!(c.wish_bonus_regen != 0 && c.wish_bonus_regen.abs() < 500);
    println!(
        "traps: chestnut {} spikes {} stealth rock {} explosions {:?}",
        c.chestnut_damage, c.spiked_tile_damage, c.stealth_rock_damage, c.explosion_damages
    );
    for damage in [c.chestnut_damage, c.spiked_tile_damage, c.stealth_rock_damage] {
        assert!((1..=100).contains(&damage), "{damage}");
    }
    // The bigger explosion hurts at least as much as the smaller one.
    assert!(c.explosion_damages[1] >= 1 && c.explosion_damages[2] >= c.explosion_damages[1]);
    assert!(c.explosion_damages[2] <= 999);
    assert!((1..=100).contains(&c.blowback_damage));
    // pmdsky-debug documents these ranges and this cooldown.
    assert_eq!((c.rest_turns, c.curse_turns, c.curse_cooldown), ((1, 4), (3, 9), 3));
    assert!(c.endure_turns.0 >= 1 && c.endure_turns.0 <= c.endure_turns.1);
    assert!(c.endure_turns.1 < 100, "{:?}", c.endure_turns);
    // Weather moves last long; bad weather hurts a little.
    assert_eq!(c.weather_move_turns, 3000);
    assert!((1..=50).contains(&c.bad_weather_damage));
    // The more moves a monster knows, the more it keeps to the regular
    // attack; with none it has nothing else.
    println!("regular attack weights: {:?}", c.ai_regular_attack_weights);
    // New enemies keep coming, but not every turn, and fewer when a
    // Monster House already fills the floor.
    println!("spawns: every {} turns, caps {} / {}", c.spawn_cooldown, c.spawn_cap, c.spawn_cap_monster_house);
    assert!((10..=200).contains(&c.spawn_cooldown));
    assert!((1..=16).contains(&c.spawn_cap));
    assert!((1..=c.spawn_cap).contains(&c.spawn_cap_monster_house));
    assert_eq!(c.ai_regular_attack_weights[0], 100);
    assert!(c.ai_regular_attack_weights[1..].windows(2).all(|w| w[0] < w[1]));
    // Weather Ball is Normal in clear weather, Fire in the sun, Water
    // in the rain, and stronger in any real weather.
    assert_eq!((c.weather_ball_types[0], c.weather_ball_types[1], c.weather_ball_types[4]), (1, 2, 3));
    assert!(c.weather_ball_types.iter().all(|&kind| (1..18).contains(&kind)));
    assert_eq!(c.weather_ball_multipliers[0], 0x100);
    assert!(c.weather_ball_multipliers.iter().all(|m| (0x100..=0x400).contains(m)));
    for turns in [c.protect_turns, c.safeguard_turns, c.mist_turns, c.lucky_chant_turns, c.reflect_turns, c.light_screen_turns] {
        assert!(turns.0 >= 1 && turns.0 <= turns.1 && turns.1 < 200, "{turns:?}");
    }
    // Charged moves hit harder than a plain move.
    assert!(c.charged_multipliers.iter().all(|m| (0x100..=0x400).contains(m)), "{c:x?}");
    for turns in [c.speed_boost_turns, c.speed_lower_turns] {
        assert!(turns.0 >= 1 && turns.0 <= turns.1 && turns.1 < 200, "{turns:?}");
    }
    // Burns hurt a little, now and then, and weaken attacks.
    assert!((1..=50).contains(&c.burn_damage) && (1..=100).contains(&c.burn_cooldown));
    assert!(c.burn_multiplier > 0 && c.burn_multiplier < 0x100, "{:#x}", c.burn_multiplier);
    for turns in [c.burn_turns, c.paralysis_turns, c.frozen_turns, c.cringe_turns, c.confused_turns] {
        assert!(turns.0 >= 1 && turns.0 <= turns.1 && turns.1 < 200, "{turns:?}");
    }
    // Accuracy rises with its stage; a higher evasion stage lowers the
    // chance of being hit.
    // The neutral evasion multiplier is slightly above 1 in the game.
    assert_eq!((c.accuracy_stages[10], c.evasion_stages[10]), (0x100, 0x107));
    assert!(c.accuracy_stages.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(c.evasion_stages.windows(2).all(|pair| pair[0] >= pair[1]));
    // Stage 10 is neutral, and a higher stage never lowers the stat.
    for table in [c.offensive_stages, c.defensive_stages] {
        assert_eq!(table[NEUTRAL_STAGE as usize], 0x100);
        assert!(table.windows(2).all(|pair| pair[0] <= pair[1]), "{table:x?}");
        assert!(table[0] > 0 && table[0] < 0x100 && table[20] > 0x100);
    }
}

/// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
#[test]
#[ignore]
fn real_rom_type_chart_is_sane() {
    let rom = crate::nds::test_rom();
    let chart = load_type_chart(&rom).unwrap();
    // Combining two matchups: neutral changes nothing, opposite
    // matchups cancel out, and an immunity only weakens a super
    // effective hit.
    assert_eq!(chart.combinator, [[0, 0, 0, 1], [0, 1, 1, 2], [0, 1, 2, 3], [1, 2, 3, 3]]);
    // ln(x) with 12 fraction bits.
    assert_eq!(chart.ln_table.len(), 0x800);
    for x in 1..0x800usize {
        let expected = (x as f64).ln() * 4096.0;
        assert!((chart.ln_table[x] as f64 - expected).abs() <= 2.0, "ln({x})");
    }
    println!("multipliers {:x?}", chart.multipliers);
    let [immune, weak, neutral, strong] = chart.multipliers;
    assert_eq!(neutral, 0x100);
    assert!(immune < weak && weak < neutral && neutral < strong);
    // Type 0 is "none": its row and column are all neutral.
    assert!(chart.matchups[0].iter().all(|&m| m == 2));
    assert!(chart.matchups.iter().all(|row| row[0] == 2));
    // Well-known matchups, by type id: water (3) beats fire (2), fire
    // is resisted by water, electric (5) does nothing to ground (9).
    assert_eq!(chart.matchups[3][2], 3);
    assert_eq!(chart.matchups[2][3], 1);
    assert_eq!(chart.matchups[5][9], 0);
}
