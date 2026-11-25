use super::*;

#[test]
fn seeds_and_drinks_do_their_thing() {
    let mut rng = DungeonRng::new(9);
    let mut eat = |item: u16, stats: &mut Stats, statuses: &mut Statuses, belly: &mut Belly| eat_item(item, &mut rng, &RULES, stats, statuses, belly, &mut []);
    let (mut stats, mut statuses, mut belly) = (monster(), Statuses::default(), Belly::full());
    let before = stats;
    assert_eq!(eat(item::LIFE_SEED, &mut stats, &mut statuses, &mut belly), ItemUse::Done);
    assert_eq!(stats.max_hp, before.max_hp + 3);
    eat(item::PROTEIN, &mut stats, &mut statuses, &mut belly);
    eat(item::ZINC, &mut stats, &mut statuses, &mut belly);
    assert_eq!((stats.attack, stats.sp_defense), (before.attack + 3, before.sp_defense + 3));
    // A Sitrus Berry heals, or raises max HP when there is nothing to heal.
    stats.hp = stats.max_hp;
    eat(item::SITRUS_BERRY, &mut stats, &mut statuses, &mut belly);
    assert_eq!((stats.hp, stats.max_hp), (before.max_hp + 5, before.max_hp + 5));
    stats.hp = 1;
    eat(item::SITRUS_BERRY, &mut stats, &mut statuses, &mut belly);
    assert_eq!((stats.hp, stats.max_hp), (before.max_hp + 5, before.max_hp + 5));
    // Statuses given and cured.
    eat(item::SLEEP_SEED, &mut stats, &mut statuses, &mut belly);
    assert!(statuses.asleep());
    eat(item::CHESTO_BERRY, &mut stats, &mut statuses, &mut belly);
    assert!(!statuses.asleep());
    eat(item::TOTTER_SEED, &mut stats, &mut statuses, &mut belly);
    assert!(statuses.confused());
    eat(item::VIOLENT_SEED, &mut stats, &mut statuses, &mut belly);
    assert_eq!((statuses.attack_stage, statuses.sp_attack_stage), (20, 20));
    eat(item::HUNGER_SEED, &mut stats, &mut statuses, &mut belly);
    assert_eq!(belly.points(), 0);
    eat(item::HUGE_APPLE, &mut stats, &mut statuses, &mut belly);
    assert_eq!(belly.points(), belly.max_points());
    // Left to the caller, and items without an effect are kept.
    assert_eq!(eat(item::WARP_SEED, &mut stats, &mut statuses, &mut belly), ItemUse::Warp);
    assert_eq!(eat(item::JOY_SEED, &mut stats, &mut statuses, &mut belly), ItemUse::Levels(1));
    assert_eq!(eat(1, &mut stats, &mut statuses, &mut belly), ItemUse::Nothing);
}

pub(crate) const RULES: Constants = Constants {
    oran_berry_hp: 100,
    sitrus_berry_hp: 100,
    oran_berry_full_hp_boost: 0,
    sitrus_berry_full_hp_boost: 2,
    life_seed_hp_boost: 3,
    thrown_hit_chance: 90,
    blast_seed_damage: (45, 35),
    thrown_powers: [0, 2, 6, 10, 14, 4, 8, 20, 10, 20, 40],
    oren_berry_damage: 10,
    vitamin_boosts: [3; 4],
    poison_damage: 4,
    poison_cooldown: 3,
    bad_poison_cooldown: 2,
    bad_poison_damage: [6; 30],
    leech_seed_drain: 4,
    leech_seed_cooldown: 2,
    leech_seed_turns: (5, 7),
    yawn_turns: (2, 2),
    taunt_turns: (4, 6),
    sure_shot_turns: (10, 12),
    infatuated_turns: (4, 6),
    invisible_turns: (20, 30),
    magic_coat_turns: (10, 14),
    snatch_turns: (3, 5),
    sport_turns: (10, 12),
    sleepless_turns: (5, 7),
    magnet_rise_turns: (10, 12),
    embargo_turns: (10, 12),
    encore_turns: (3, 5),
    cross_eyed_turns: (6, 10),
    nightmare_turns: (4, 7),
    heal_block_turns: (4, 9),
    metronome: [[0; 28]; 6],
    counter_turns: (6, 11),
    mirror_coat_turns: (6, 11),
    perish_turns: (3, 3),
    set_damage_turns: (10, 12),
    set_damage: 30,
    mirror_move_turns: (10, 12),
    imprison_turns: (6, 10),
    wrap_turns: (3, 5),
    return_damage: [(1000, 25); 12],
    frustration_damage: [(1000, 25); 12],
    magnitude_damage: [20; 7],
    vacuum_cut_damage: 20,
    nightmare_damage: 8,
    napping_heal: 999,
    destiny_bond_turns: (5, 9),
    transform_turns: (10, 12),
    gummi_iq: [[3; 18]; 18],
    gummi_belly: [[15; 18]; 18],
    wonder_gummi_iq: 15,
    gummi_boost_chance: 25,
    gummi_all_stats_odds: 16,
    gummi_stat_boost: 1,
    small_food: [0; 10],
    exp_elite_boost: 50,
    me_first: 384,
    facade_mult: 0x200,
    reversal_mult: [512, 384, 256, 256],
    water_spout_mult: [25, 51, 128, 256],
    eruption_mult: [25, 51, 128, 256],
    wring_out_mult: [25, 51, 128, 256],
    constriction_chance: 10,
    constriction_turns: (3, 6),
    constriction_damage: 5,
    constriction_cooldown: 2,
    last_resort_mult: [256, 384, 512, 512],
    recharge_turns: (1, 1),
    secret_power_chance: 30,
    exclusive_exp_boost: 50,
    exclusive_reaction_chance: 30,
    wise_healer_percent: 15,
    intimidator_chance: 32,
    power_pitcher: 384,
    arena_trap_chance: 12,
    shadow_tag_chance: 12,
    natural_gift: [[[0; 3]; 17]; 2],
    rollout_mult: [256, 281, 307, 332, 358, 384, 409, 435, 460, 486],
    vital_throw_turns: (15, 20),
    decoy_turns: (20, 50),
    conversion2_turns: (10, 10),
    conversion2_types: [0; 18],
    grimy_food_chance: 50,
    secondary_chances: [20; 32],
    tri_attack_chance: 20,
    sleep_turns: (3, 6),
    pitfall_trap_damage: 5,
    belly_lost_per_turn: 0x199A,
    link_belly: [1, 1, 1, 2, 4],
    offensive_stages: [0; STAGE_COUNT],
    defensive_stages: [0; STAGE_COUNT],
    accuracy_stages: [0; STAGE_COUNT],
    female_accuracy_stages: [0; STAGE_COUNT],
    female_evasion_stages: [0; STAGE_COUNT],
    spawn_cooldown_alert: 0,
    evasion_stages: [0; STAGE_COUNT],
    charged_multipliers: [0x200; CHARGED_MOVES.len()],
    speed_boost_turns: (3, 5),
    speed_boost_every: 250,
    speed_lower_turns: (3, 5),
    meteor_mash_chance: 20,
    metal_claw_chance: 10,
    steel_wing_chance: 10,
    silver_wind_chance: 20,
    mist_ball_chance: 50,
    charge_beam_chance: 70,
    luster_purge_chance: 50,
    reflect_turns: (5, 6),
    light_screen_turns: (5, 6),
    heal_tables: [[50, 80, 30, 30, 10, 10, 30, 10]; 4],
    heal_order_hp: 10,
    regen_speed: 200,
    wish_bonus_regen: -100,
    wish_turns: (10, 12),
    ingrain_regen: 10,
    ingrain_cooldown: 5,
    ingrain_turns: (15, 30),
    aqua_ring_regen: 8,
    aqua_ring_cooldown: 4,
    aqua_ring_turns: (8, 16),
    chestnut_damage: 10,
    spiked_tile_damage: 10,
    stealth_rock_damage: 15,
    explosion_damages: [0, 40, 80],
    blowback_damage: 5,
    endure_turns: (2, 3),
    rest_turns: (1, 4),
    curse_turns: (3, 9),
    curse_cooldown: 3,
    weather_move_turns: 3000,
    bad_weather_damage: 3,
    ai_regular_attack_weights: [100, 20, 30, 40, 50],
    spawn_cooldown: 36,
    spawn_cap: 15,
    spawn_cap_monster_house: 4,
    weather_ball_types: [1, 2, 13, 1, 3, 6, 1, 6],
    weather_ball_multipliers: [0x100, 0x200, 0x200, 0x100, 0x200, 0x200, 0x100, 0x200],
    protect_turns: (2, 3),
    safeguard_turns: (5, 6),
    mist_turns: (5, 6),
    lucky_chant_turns: (5, 6),
    sonicboom_damage: 20,
    dragon_rage_damage: 30,
    burn_damage: 5,
    burn_cooldown: 2,
    burn_multiplier: 0xCC,
    burn_turns: (3, 5),
    paralysis_turns: (3, 5),
    frozen_turns: (3, 5),
    cringe_turns: (1, 1),
    confused_turns: (3, 5),
    held_boosts: [0; 5],
    scope_lens_crit: 0,
    detect_band_drop: 0,
    quick_dodger_drop: 0,
    type_advantage_crit: 0,
};

#[test]
fn deep_sleep_lasts_until_a_hit() {
    let mut stats = monster();
    let mut statuses = Statuses { sleep_turns: DEEP_SLEEP, ..Statuses::default() };
    for _ in 0..300 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert!(statuses.asleep());
    statuses.struck();
    assert!(!statuses.asleep());
    // An ordinary sleep wears off by itself and survives hits.
    statuses.sleep_turns = 3;
    statuses.struck();
    assert!(statuses.asleep());
    for _ in 0..3 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert!(!statuses.asleep());
}

#[test]
fn stat_stages_stay_in_range() {
    let mut rng = DungeonRng::new(4);
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    apply_effect(MoveEffect::UserAttack(2), &mut rng, &RULES, &mut stats, &mut statuses);
    apply_effect(MoveEffect::TargetDefense(-1), &mut rng, &RULES, &mut stats, &mut statuses);
    let fighter = statuses.fighter(&stats, true);
    assert_eq!((fighter.attack_stage, fighter.defense_stage), (12, 9));
    for _ in 0..30 {
        apply_effect(MoveEffect::TargetAttack(-2), &mut rng, &RULES, &mut stats, &mut statuses);
    }
    assert_eq!(statuses.attack_stage, 0);
    // Curing statuses keeps the stages; leaving the floor resets them.
    statuses.cure_all();
    assert_eq!(statuses.attack_stage, 0);
    statuses.reset_stages();
    assert_eq!(statuses.attack_stage, NEUTRAL_STAGE);
}

#[test]
fn status_problems_block_replace_and_wear_off() {
    let mut rng = DungeonRng::new(9);
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    statuses.poison();
    statuses.burn(&mut rng, &RULES);
    assert!(statuses.burn_turns > 0 && !statuses.poisoned);
    assert!(statuses.fighter(&stats, true).burned);
    // The burn hurts every second turn while it lasts.
    let hp = stats.hp;
    statuses.end_turn(&mut stats, &RULES);
    statuses.end_turn(&mut stats, &RULES);
    assert_eq!(stats.hp, hp - 5);
    // Paralysis replaces the burn and only prevents attacking.
    statuses.paralyze(&mut rng, &RULES);
    assert!(statuses.burn_turns == 0 && statuses.cannot_attack() && !statuses.cannot_act());
    statuses.freeze(&mut rng, &RULES);
    assert!(statuses.cannot_act());
    statuses.cringe(&mut rng, &RULES);
    statuses.confuse(&mut rng, &RULES);
    assert!(statuses.confused() && statuses.cringe_turns == 0);
    for _ in 0..10 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert!(!statuses.cannot_attack() && !statuses.confused());
    statuses.paralyze(&mut rng, &RULES);
    statuses.cure_all();
    assert!(!statuses.cannot_attack());
}

#[test]
fn fixed_damage_and_strike_counts() {
    let mut rng = DungeonRng::new(5);
    let user = Stats {
        level: 20,
        max_hp: 60,
        hp: 60,
        attack: 30,
        defense: 30,
        sp_attack: 30,
        sp_defense: 30,
        types: (1, 0),
        abilities: [0, 0],
        held: 0,
        iq: 1,
        iq_group: 0,
        iq_skills: 0,
        female: false,
    };
    let target = Stats { hp: 31, ..user };
    let mut fixed = |id| fixed_damage(&MoveSlot { id, ..slot() }, &mut rng, &RULES, None, &user, &target);
    // A one-hit KO.
    assert_eq!(fixed(150), Some(9999));
    assert_eq!(fixed(152), Some(20));
    assert_eq!(fixed(341), Some(30));
    assert_eq!(fixed(170), Some(20));
    assert_eq!(fixed(216), Some(20));
    assert_eq!(fixed(20), Some(15));
    assert_eq!(fixed(1), None);
    for _ in 0..200 {
        let psywave = fixed(108).unwrap();
        assert!((10..30).contains(&psywave), "{psywave}");
    }
    assert!(is_fixed_damage(20) && !is_fixed_damage(1));
    assert_eq!(strike_count(&mut rng, 1), 1);
    assert_eq!(strike_count(&mut rng, 3), 3);
    let counts: Vec<i32> = (0..300).map(|_| strike_count(&mut rng, 0)).collect();
    assert!(counts.iter().all(|count| (2..=5).contains(count)));
    assert!(counts.contains(&2) && counts.contains(&5));
}

#[test]
fn speed_stages_follow_their_counters() {
    let mut rng = DungeonRng::new(11);
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    assert_eq!(statuses.speed_stage(), 1);
    assert!(statuses.boost_speed(&mut rng, &RULES));
    assert!(statuses.boost_speed(&mut rng, &RULES));
    assert!(statuses.boost_speed(&mut rng, &RULES));
    assert_eq!(statuses.speed_stage(), 4);
    // Already at the maximum.
    assert!(!statuses.boost_speed(&mut rng, &RULES));
    assert!(statuses.lower_speed(&mut rng, &RULES));
    assert_eq!(statuses.speed_stage(), 3);
    // Paralysis costs a stage while it lasts.
    statuses.paralyze(&mut rng, &RULES);
    assert_eq!(statuses.speed_stage(), 2);
    for _ in 0..10 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert_eq!(statuses.speed_stage(), 1);
    assert!(statuses.lower_speed(&mut rng, &RULES));
    assert_eq!(statuses.speed_stage(), 0);
    assert!(!statuses.lower_speed(&mut rng, &RULES));
    // Charged moves.
    assert_eq!(charged_multiplier(&RULES, 151), Some(0x200));
    assert_eq!(charged_multiplier(&RULES, 1), None);
    statuses.charging = Some(8);
    assert!(statuses.out_of_reach());
    statuses.charging = Some(151);
    assert!(!statuses.out_of_reach());
}

#[test]
fn multipliers_heals_and_protections() {
    let mut rng = DungeonRng::new(21);
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    let mut apply = |effect, stats: &mut Stats, statuses: &mut Statuses| apply_effect(effect, &mut rng, &RULES, stats, statuses);
    // Charm halves the attack, Screech quarters the defense; they stack.
    assert!(apply(MoveEffect::AttackMultiplier(0x80), &mut stats, &mut statuses));
    assert!(apply(MoveEffect::AttackMultiplier(0x80), &mut stats, &mut statuses));
    assert!(apply(MoveEffect::DefenseMultiplier(0x40), &mut stats, &mut statuses));
    let fighter = statuses.fighter(&stats, true);
    assert_eq!((fighter.attack_multiplier, fighter.defense_multiplier), (0x40, 0x40));
    for _ in 0..10 {
        apply(MoveEffect::AttackMultiplier(0x40), &mut stats, &mut statuses);
    }
    assert_eq!(statuses.attack_multiplier, 2);
    // Snow speeds Ice types up by a stage.
    let ice = Stats { types: (6, 0), ..stats };
    assert_eq!(statuses.speed_stage_in(&ice, weather::SNOW), 2);
    assert_eq!(statuses.speed_stage_in(&ice, weather::CLEAR), 1);
    assert_eq!(statuses.speed_stage_in(&stats, weather::SNOW), 1);
    // Exclusive items: a stage faster in the rain (51), status problems
    // kept off in the sun (41).
    let mut held = statuses;
    held.exclusive.effects[0] |= 1 << 51 | 1 << 41;
    assert_eq!((held.speed_stage_in(&stats, weather::RAIN), held.speed_stage_in(&stats, weather::SUNNY)), (2, 1));
    held.exclusive.weather = weather::SUNNY;
    assert!(!apply(MoveEffect::Poison, &mut stats, &mut held) && !held.poisoned);
    held.exclusive.weather = weather::CLEAR;
    assert!(apply(MoveEffect::Poison, &mut stats, &mut held) && held.poisoned);
    assert_eq!(RULES.heal_amount(0, weather::SUNNY), 80);
    assert_eq!(RULES.heal_amount(4, weather::SUNNY), 10);
    assert_eq!(RULES.weather_ball(weather::RAIN), (3, 0x200));
    assert_eq!(move_effect(12), Some(MoveEffect::Weather(weather::RAIN)));
    // Combined stage changes.
    apply(MoveEffect::BulkUp, &mut stats, &mut statuses);
    apply(MoveEffect::Swagger, &mut stats, &mut statuses);
    assert_eq!((statuses.attack_stage, statuses.defense_stage), (13, 11));
    assert!(statuses.confused());
    apply(MoveEffect::DragonDance, &mut stats, &mut statuses);
    assert_eq!((statuses.attack_stage, statuses.speed_stage()), (14, 2));
    // Heals: a quarter of the max HP, or a fixed amount.
    stats.hp = 1;
    apply(MoveEffect::HealQuarter, &mut stats, &mut statuses);
    assert_eq!(stats.hp, 1 + 30 / 4);
    apply(MoveEffect::HealBy(4), &mut stats, &mut statuses);
    assert_eq!(stats.hp, 18);
    apply(MoveEffect::HealFlat(2), &mut stats, &mut statuses);
    assert_eq!(stats.hp, 20);
    stats.hp = 18;
    apply(MoveEffect::HealBy(0), &mut stats, &mut statuses);
    assert_eq!(stats.hp, 30);
    apply(MoveEffect::CureStatus, &mut stats, &mut statuses);
    assert!(!statuses.confused());
    // Safeguard stops status problems, Mist stat drops; each replaces
    // the other and wears off.
    apply(MoveEffect::Safeguard, &mut stats, &mut statuses);
    assert!(!apply(MoveEffect::Poison, &mut stats, &mut statuses) && !statuses.poisoned);
    assert!(apply(MoveEffect::TargetDefense(-1), &mut stats, &mut statuses));
    apply(MoveEffect::Mist, &mut stats, &mut statuses);
    assert!(!apply(MoveEffect::TargetDefense(-1), &mut stats, &mut statuses));
    assert!(!apply(MoveEffect::AttackMultiplier(0x80), &mut stats, &mut statuses));
    assert!(apply(MoveEffect::Poison, &mut stats, &mut statuses));
    apply(MoveEffect::Protect, &mut stats, &mut statuses);
    assert!(statuses.protected());
    for _ in 0..5 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert!(!statuses.protected() && statuses.shield.is_none());
    statuses.reset_stages();
    assert_eq!(statuses.attack_multiplier, 0x100);
    // Special stats have their own stages, multipliers and screen.
    apply(MoveEffect::CalmMind, &mut stats, &mut statuses);
    apply(MoveEffect::TargetSpDefense(-3), &mut stats, &mut statuses);
    apply(MoveEffect::Memento, &mut stats, &mut statuses);
    apply(MoveEffect::LightScreen, &mut stats, &mut statuses);
    let fighter = statuses.fighter(&stats, true);
    assert_eq!((fighter.sp_attack_stage, fighter.sp_defense_stage), (11, 8));
    assert_eq!((fighter.attack_multiplier, fighter.sp_attack_multiplier), (0x40, 0x40));
    assert!(fighter.halves_special && !fighter.halves_physical);
    apply(MoveEffect::Reflect, &mut stats, &mut statuses);
    let fighter = statuses.fighter(&stats, true);
    assert!(fighter.halves_physical && !fighter.halves_special);
    assert_eq!(secondary_effect(309, &RULES), Some((50, MoveEffect::TargetSpAttack(-1))));
    assert_eq!(user_effect(538, &RULES), Some((100, MoveEffect::UserSpAttack(-2))));
    // Natural regeneration: 30 max HP against a speed of 200 is 3 HP
    // in 20 turns; Wish's bonus changes the speed.
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    stats.hp = 1;
    for _ in 0..20 {
        statuses.regenerate(&mut stats, &RULES);
    }
    assert_eq!(stats.hp, 4);
    apply(MoveEffect::Wish, &mut stats, &mut statuses);
    for _ in 0..10 {
        statuses.regenerate(&mut stats, &RULES);
    }
    assert_eq!(stats.hp, 7);
    stats.hp = 30;
    statuses.regenerate(&mut stats, &RULES);
    assert_eq!(stats.hp, 30);
    // No regeneration without the species' speed (no ROM).
    let mut still = Statuses::default();
    stats.hp = 1;
    still.regenerate(&mut stats, &Constants::default());
    assert_eq!(stats.hp, 1);
    // Ingrain roots and heals 10 HP every fifth turn; Aqua Ring heals 8
    // every fourth.
    let mut statuses = Statuses::default();
    apply(MoveEffect::Ingrain, &mut stats, &mut statuses);
    assert!(statuses.rooted());
    for _ in 0..5 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert_eq!(stats.hp, 11);
    let mut statuses = Statuses::default();
    apply(MoveEffect::AquaRing, &mut stats, &mut statuses);
    for _ in 0..4 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert_eq!(stats.hp, 19);
    assert!(!statuses.rooted());
    // Endure leaves 1 HP; a curse takes a quarter of the max HP every
    // third turn; Rest heals, cures and puts to sleep.
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    apply(MoveEffect::Endure, &mut stats, &mut statuses);
    stats.hp = -12;
    statuses.endure(&mut stats);
    assert_eq!(stats.hp, 1);
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    stats.hp = -12;
    statuses.endure(&mut stats);
    assert_eq!(stats.hp, -12);
    stats.hp = 30;
    apply(MoveEffect::Cursed, &mut stats, &mut statuses);
    for _ in 0..3 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert_eq!(stats.hp, 30 - 7);
    apply(MoveEffect::Burn, &mut stats, &mut statuses);
    // Rest: a nap, which heals and cures as it ends.
    apply(MoveEffect::Rest, &mut stats, &mut statuses);
    assert!(statuses.asleep() && statuses.napping);
    while statuses.asleep() {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert!(stats.hp == 30 && statuses.burn_turns == 0 && statuses.cursed_turns == 0, "{} {}", stats.hp, statuses.burn_turns);
    let mut statuses = Statuses::default();
    apply(MoveEffect::CurseSelf, &mut stats, &mut statuses);
    assert_eq!((statuses.attack_stage, statuses.defense_stage), (11, 11));
    assert_eq!(statuses.speed_stage(), 0);
    assert_eq!(move_effect(187), Some(MoveEffect::Warp));
    // Effects on the user of a damaging move.
    assert_eq!(user_effect(243, &RULES), Some((100, MoveEffect::Superpower)));
    assert_eq!(user_effect(103, &RULES), Some((20, MoveEffect::UserAttack(1))));
    assert_eq!(user_effect(1, &RULES), None);
}

#[test]
fn the_other_traps_do_their_thing() {
    let mut rng = DungeonRng::new(8);
    let spring =
        |kind, rng: &mut DungeonRng, stats: &mut Stats, statuses: &mut Statuses, moves: &mut [MoveSlot]| spring_trap(kind, rng, &RULES, stats, statuses, moves);
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    let mut moves = [slot(), slot()];
    assert_eq!(spring(trap::GUST, &mut rng, &mut stats, &mut statuses, &mut moves), TrapOutcome::Blow);
    assert_eq!(spring(trap::SELFDESTRUCT, &mut rng, &mut stats, &mut statuses, &mut moves), TrapOutcome::Explode(1));
    assert_eq!(spring(trap::EXPLOSION, &mut rng, &mut stats, &mut statuses, &mut moves), TrapOutcome::Explode(2));
    assert_eq!(spring(trap::GRIMY, &mut rng, &mut stats, &mut statuses, &mut moves), TrapOutcome::GrimyFood);
    spring(trap::SPIN, &mut rng, &mut stats, &mut statuses, &mut moves);
    assert!(statuses.confused());
    spring(trap::SLOW, &mut rng, &mut stats, &mut statuses, &mut moves);
    assert_eq!(statuses.speed_stage(), 0);
    // Fixed damage; Stealth Rock spares Rock types.
    stats.hp = 30;
    spring(trap::CHESTNUT, &mut rng, &mut stats, &mut statuses, &mut moves);
    spring(trap::SPIKED_TILE, &mut rng, &mut stats, &mut statuses, &mut moves);
    assert_eq!(stats.hp, 10);
    spring(trap::STEALTH_ROCK, &mut rng, &mut stats, &mut statuses, &mut moves);
    assert_eq!(stats.hp, -5);
    let mut rock = Stats { hp: 30, types: (13, 0), ..stats };
    spring(trap::STEALTH_ROCK, &mut rng, &mut rock, &mut statuses, &mut moves);
    assert_eq!(rock.hp, 30);
    // The Seal Trap seals one more move each time, and a sealed move
    // cannot be used.
    spring(trap::SEAL, &mut rng, &mut stats, &mut statuses, &mut moves);
    assert_eq!(moves.iter().filter(|slot| slot.sealed).count(), 1);
    assert_eq!(moves.iter().filter(|slot| slot.usable()).count(), 1);
    spring(trap::SEAL, &mut rng, &mut stats, &mut statuses, &mut moves);
    spring(trap::SEAL, &mut rng, &mut stats, &mut statuses, &mut moves);
    assert!(moves.iter().all(|slot| slot.sealed && !slot.usable()));
    // Explosions: half the HP for the team, a fixed amount for others,
    // half of either for Fire types.
    let victim = Stats { hp: 30, ..monster() };
    assert_eq!(explosion_damage(&RULES, 1, &victim, true), 15);
    assert_eq!(explosion_damage(&RULES, 1, &victim, false), 40);
    assert_eq!(explosion_damage(&RULES, 2, &victim, false), 80);
    let fire = Stats { types: (2, 0), ..victim };
    assert_eq!(explosion_damage(&RULES, 2, &fire, true), 7);
    assert_eq!(explosion_damage(&RULES, 2, &fire, false), 40);
    assert_eq!(explosion_damage(&RULES, 1, &Stats { hp: 1, ..victim }, true), 1);
}

#[test]
fn recoil_and_drain_go_by_the_damage_dealt() {
    let mut user = Stats {
        level: 5,
        max_hp: 80,
        hp: 50,
        attack: 10,
        defense: 10,
        sp_attack: 10,
        sp_defense: 10,
        types: (1, 0),
        abilities: [0, 0],
        held: 0,
        iq: 1,
        iq_group: 0,
        iq_skills: 0,
        female: false,
    };
    assert_eq!(Aftermath::Recoil.apply(&mut user, 30), 10);
    assert_eq!(Aftermath::Drain.apply(&mut user, 31), -15);
    assert_eq!(Aftermath::Drain.apply(&mut user, 1), -1);
    user.hp = 79;
    assert_eq!(Aftermath::Drain.apply(&mut user, 60), -1);
    let mut tiny = Stats { max_hp: 5, hp: 5, ..user };
    assert_eq!(Aftermath::Recoil.apply(&mut tiny, 3), 1);
    assert_eq!(aftermath(116), Some(Aftermath::Recoil));
    assert_eq!(secondary_effect(53, &RULES), Some((10, MoveEffect::Burn)));
    assert_eq!(secondary_effect(1, &RULES), None);
    // Iron Tail's, and both of Ice Fang's, with the extracted chances.
    assert_eq!(secondary_effects(1, &RULES), vec![(20, MoveEffect::TargetDefense(-1))]);
    assert_eq!(secondary_effects(461, &RULES).len(), 2);
}

pub(crate) fn slot() -> MoveSlot {
    MoveSlot {
        id: 1,
        power: 40,
        accuracy: 100,
        accuracy2: 125,
        strikes: 1,
        pp: 10,
        max_pp: 10,
        type_id: 1,
        category: 0,
        sealed: false,
        crit_chance: 0,
        range: 0,
        target: 0,
        ai_weight: 0,
        ai_target_range: 0,
        ai_chance: 0,
        linked: false,
        talented: false,
        off: false,
    }
}

pub(crate) fn monster() -> Stats {
    Stats {
        level: 5,
        max_hp: 30,
        hp: 20,
        attack: 10,
        defense: 10,
        sp_attack: 10,
        sp_defense: 10,
        types: (1, 0),
        abilities: [0, 0],
        held: 0,
        iq: 1,
        iq_group: 0,
        iq_skills: 0,
        female: false,
    }
}

#[test]
fn some_moves_hit_harder_by_state() {
    let (calm, weak) = (Statuses::default(), Stats { hp: 5, ..monster() });
    let burnt = Statuses { burn_turns: 3, ..Statuses::default() };
    let power = |id, user: (&Stats, &Statuses), target: &Stats| power_multiplier(id, 10, 10, &RULES, (user.0, user.1, false), (target, &calm));
    // Facade, burnt or not; Brine on a target at a sixth of its HP.
    assert_eq!((power(67, (&monster(), &burnt), &monster()), power(67, (&monster(), &calm), &monster())), (RULES.facade_mult, 0x100));
    assert_eq!(power(469, (&monster(), &calm), &weak), 0x200);
    // Flail at a sixth of its HP, Eruption at two thirds.
    assert_eq!(power(121, (&weak, &calm), &monster()), RULES.reversal_mult[0]);
    assert_eq!(power(278, (&monster(), &calm), &monster()), RULES.eruption_mult[2]);
}

#[test]
fn yawn_puts_to_sleep_and_leech_seed_drains() {
    let mut stats = Stats { hp: 50, max_hp: 50, ..monster() };
    let mut statuses = Statuses::default();
    let mut rng = DungeonRng::new(1);
    assert!(apply_effect(MoveEffect::Yawn, &mut rng, &RULES, &mut stats, &mut statuses));
    statuses.end_turn(&mut stats, &RULES);
    assert!(!statuses.asleep());
    // (A duration counts the turn it starts on.)
    (0..2).for_each(|_| statuses.end_turn(&mut stats, &RULES));
    assert!(statuses.asleep());
    apply_effect(MoveEffect::LeechSeed, &mut rng, &RULES, &mut stats, &mut statuses);
    let before = stats.hp;
    statuses.end_turn(&mut stats, &RULES);
    statuses.end_turn(&mut stats, &RULES);
    assert_eq!(stats.hp, before - 4);
    // Stockpile twice, then Swallow: half the HP back.
    let (mut low, mut store) = (Stats { hp: 10, max_hp: 50, ..monster() }, Statuses::default());
    assert!(!apply_effect(MoveEffect::Swallow, &mut rng, &RULES, &mut low, &mut store));
    apply_effect(MoveEffect::Stockpile, &mut rng, &RULES, &mut low, &mut store);
    apply_effect(MoveEffect::Stockpile, &mut rng, &RULES, &mut low, &mut store);
    apply_effect(MoveEffect::Swallow, &mut rng, &RULES, &mut low, &mut store);
    assert!(low.hp == 35 && store.stockpile == 0);
    // Belly Drum: half the HP for the highest attack.
    assert!(apply_effect(MoveEffect::BellyDrum, &mut rng, &RULES, &mut stats, &mut statuses));
    assert_eq!(statuses.attack_stage, STAGE_COUNT as i32 - 1);
}

#[test]
fn bad_poison_hurts_on_its_table_and_stops_regeneration() {
    let mut stats = Stats { hp: 50, max_hp: 50, ..monster() };
    let mut statuses = Statuses::default();
    let mut rng = DungeonRng::new(1);
    apply_effect(MoveEffect::BadlyPoison, &mut rng, &RULES, &mut stats, &mut statuses);
    assert!(statuses.poisoned && statuses.badly_poisoned);
    for _ in 0..4 {
        statuses.end_turn(&mut stats, &RULES);
    }
    // Two ticks of 6 in four turns.
    assert_eq!(stats.hp, 38);
    statuses.regenerate(&mut stats, &RULES);
    assert_eq!(stats.hp, 38);
}

#[test]
fn poison_ticks_on_its_cooldown_until_cured() {
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    statuses.poison();
    for _ in 0..6 {
        statuses.end_turn(&mut stats, &RULES);
    }
    assert_eq!(stats.hp, 20 - 2 * 4);
    let mut belly = Belly::full();
    let pecha = item::PECHA_BERRY;
    assert!(use_item(pecha, &RULES, &mut stats, &mut statuses, &mut belly, &mut []));
    statuses.end_turn(&mut stats, &RULES);
    statuses.end_turn(&mut stats, &RULES);
    statuses.end_turn(&mut stats, &RULES);
    assert_eq!(stats.hp, 12);
}

#[test]
fn sleep_lasts_within_the_range_and_wears_off() {
    let mut rng = DungeonRng::new(11);
    for _ in 0..200 {
        let (mut stats, mut statuses) = (monster(), Statuses::default());
        statuses.fall_asleep(&mut rng, &RULES);
        assert!((3..=6).contains(&statuses.sleep_turns));
        for _ in 0..6 {
            statuses.end_turn(&mut stats, &RULES);
        }
        assert!(!statuses.asleep());
    }
}

#[test]
fn berries_heal_up_to_max_and_unknown_items_are_kept() {
    let (mut stats, mut statuses, mut belly) = (monster(), Statuses::default(), Belly::full());
    let oran = item::ORAN_BERRY;
    assert!(use_item(oran, &RULES, &mut stats, &mut statuses, &mut belly, &mut []));
    assert_eq!(stats.hp, 30);
    assert!(!use_item(9999, &RULES, &mut stats, &mut statuses, &mut belly, &mut []));
    // A Max Elixir refills every move.
    let mut moves = [slot(); 2];
    moves[0].pp = 0;
    moves[1].pp = 3;
    let elixir = item::MAX_ELIXIR;
    assert!(use_item(elixir, &RULES, &mut stats, &mut statuses, &mut belly, &mut moves));
    assert!(moves.iter().all(|m| m.pp == m.max_pp));
}

#[test]
fn traps_apply_their_effect() {
    let mut rng = DungeonRng::new(2);
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    let mut moves = [slot(); 3];
    let mut spring = |kind, stats: &mut Stats, statuses: &mut Statuses, moves: &mut [MoveSlot]| spring_trap(kind, &mut rng, &RULES, stats, statuses, moves);
    assert_eq!(spring(trap::PITFALL, &mut stats, &mut statuses, &mut moves), TrapOutcome::Fall);
    assert_eq!(stats.hp, 15);
    assert_eq!(spring(trap::WARP, &mut stats, &mut statuses, &mut moves), TrapOutcome::Warp);
    spring(trap::POISON, &mut stats, &mut statuses, &mut moves);
    spring(trap::SLUMBER, &mut stats, &mut statuses, &mut moves);
    assert!(statuses.poisoned && statuses.asleep());
    spring(trap::PP_ZERO, &mut stats, &mut statuses, &mut moves);
    assert_eq!(moves.iter().filter(|m| m.pp == 0).count(), 1);

    // Toxic Spikes: half a Spiked Tile's damage, and poison. Mud: one
    // stat down 2 stages. Wonder Tile: stages back to normal.
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    spring(trap::TOXIC_SPIKES, &mut stats, &mut statuses, &mut moves);
    assert!(stats.hp == 15 && statuses.poisoned);
    let stages = |statuses: &Statuses| statuses.attack_stage + statuses.defense_stage + statuses.sp_attack_stage + statuses.sp_defense_stage;
    spring(trap::MUD, &mut stats, &mut statuses, &mut moves);
    assert_eq!(stages(&statuses), 4 * NEUTRAL_STAGE - 2);
    spring(trap::WONDER_TILE, &mut stats, &mut statuses, &mut moves);
    assert_eq!(stages(&statuses), 4 * NEUTRAL_STAGE);
    for _ in 0..50 {
        let summoned = spring(trap::SUMMON, &mut stats, &mut statuses, &mut moves);
        assert!(matches!(summoned, TrapOutcome::Summon(2..=3, false)));
        let grudging = spring(trap::GRUDGE, &mut stats, &mut statuses, &mut moves);
        assert!(matches!(grudging, TrapOutcome::Summon(3..=4, true)));
    }
    let pokemon = spring(trap::POKEMON, &mut stats, &mut statuses, &mut moves);
    assert_eq!(pokemon, TrapOutcome::ItemsToMonsters);
    // A Random Trap always ends as some other trap.
    for _ in 0..200 {
        let (mut stats, mut statuses) = (monster(), Statuses::default());
        spring(trap::RANDOM, &mut stats, &mut statuses, &mut moves);
    }
    // Safeguard keeps the status traps off, as it keeps moves.
    let (mut stats, mut statuses) = (monster(), Statuses::default());
    statuses.shield = Some(Shield::Safeguard);
    statuses.shield_turns = 10;
    spring(trap::SLUMBER, &mut stats, &mut statuses, &mut moves);
    spring(trap::POISON, &mut stats, &mut statuses, &mut moves);
    assert!(!statuses.asleep() && !statuses.poisoned);
}

#[test]
fn speed_boosts_and_drops_cancel_out() {
    let mut rng = DungeonRng::new(3);
    let mut statuses = Statuses::default();
    assert!(statuses.lower_speed(&mut rng, &RULES));
    assert_eq!(statuses.speed_changes(), (0, 1));
    // A boost undoes the drop rather than running beside it.
    assert!(statuses.boost_speed(&mut rng, &RULES));
    assert_eq!((statuses.speed_changes(), statuses.speed_stage()), ((0, 0), 1));
    // Trick Room: an ordinary monster speeds up, a fast one slows down.
    let (mut stats, mut quick) = (monster(), Statuses::default());
    apply_effect(MoveEffect::TrickRoom, &mut rng, &RULES, &mut stats, &mut quick);
    assert_eq!(quick.speed_stage(), 2);
    apply_effect(MoveEffect::TrickRoom, &mut rng, &RULES, &mut stats, &mut quick);
    assert_eq!(quick.speed_stage(), 1);
}

#[test]
fn a_new_dose_keeps_the_longer_status() {
    let mut rng = DungeonRng::new(5);
    let mut statuses = Statuses { confused_turns: 40, ..Statuses::default() };
    statuses.confuse(&mut rng, &RULES);
    assert_eq!(statuses.confused_turns, 40);
    // One-hit KOs fail on a target of a higher level.
    let user = monster();
    let target = Stats { level: user.level + 1, ..user };
    assert_eq!(fixed_damage(&MoveSlot { id: 150, ..slot() }, &mut rng, &RULES, None, &user, &target), Some(0));
    // Heal Block keeps healing away.
    let (mut stats, mut statuses) = (Stats { hp: 5, ..monster() }, Statuses { heal_block_turns: 3, ..Statuses::default() });
    assert!(!apply_effect(MoveEffect::HealHalf, &mut rng, &RULES, &mut stats, &mut statuses));
    assert_eq!(stats.hp, 5);
}

#[test]
fn the_lookalike_items_do_their_own_thing() {
    let mut rng = DungeonRng::new(4);
    let (mut stats, mut statuses, mut belly) = (monster(), Statuses::default(), Belly::full());
    let mut eat = |item: u16, stats: &mut Stats, statuses: &mut Statuses| eat_item(item, &mut rng, &RULES, stats, statuses, &mut belly, &mut []);
    assert_eq!(eat(item::VIA_SEED, &mut stats, &mut statuses), ItemUse::Warp);
    assert_eq!(eat(item::DOUGH_SEED, &mut stats, &mut statuses), ItemUse::MorePoke);
    assert_eq!(eat(item::MIX_ELIXIR, &mut stats, &mut statuses), ItemUse::Fussy(item::MIX_ELIXIR));
    assert!(fussy_eaters(item::GRAVELYROCK).contains(&185));
    eat(item::SLIP_SEED, &mut stats, &mut statuses);
    eat(item::DROPEYE_SEED, &mut stats, &mut statuses);
    assert!(statuses.slipping && statuses.dropeye);
    eat(item::GONE_PEBBLE, &mut stats, &mut statuses);
    assert_eq!(statuses.shield, Some(Shield::Endure));
    eat(item::WANDER_GUMMI, &mut stats, &mut statuses);
    assert!(statuses.accuracy_stage < NEUTRAL_STAGE);
    assert_eq!(eat(item::REVISER_SEED, &mut stats, &mut statuses), ItemUse::Done);
    assert_eq!(eat(item::KEY, &mut stats, &mut statuses), ItemUse::Nothing);
    statuses.leave_floor();
    assert!(!statuses.slipping && !statuses.dropeye);
}
