use super::*;

fn open_room() -> Floor {
    let mut floor = Floor::new_walled();
    for y in 5..10 {
        for x in 5..12 {
            floor.set(Pos::new(x, y), Tile::Floor { room: Some(0) });
        }
    }
    floor
}

#[test]
fn chases_then_attacks() {
    let floor = open_room();
    let mut rng = DungeonRng::new(1);
    let target = Pos::new(10, 8);
    let mut goal = None;
    let far = enemy_action(&floor, &mut rng, Pos::new(6, 6), target, |_| false, Mobility::Normal, Roaming { facing: (0, 1), goal: &mut goal });
    assert_eq!(far, Action::Move(1, 1));
    let near = enemy_action(&floor, &mut rng, Pos::new(9, 7), target, |_| false, Mobility::Normal, Roaming { facing: (0, 1), goal: &mut goal });
    assert_eq!(near, Action::Attack(1, 1));
}

#[test]
fn hunting_no_one_it_heads_for_an_exit_then_goes_on_down_the_hallway() {
    // The room, a hallway leaving it to the right from (11, 7).
    let mut floor = open_room();
    for x in 12..30 {
        floor.set(Pos::new(x, 7), Tile::Floor { room: None });
    }
    let mut rng = DungeonRng::new(3);
    let mut goal = None;
    let mut at = Pos::new(6, 6);
    let mut facing = (0, 1);
    for _ in 0..12 {
        if let Action::Move(dx, dy) = roam(&floor, &mut rng, at, |_| false, Mobility::Normal, Roaming { facing, goal: &mut goal }) {
            (at, facing) = (Pos::new(at.x + dx, at.y + dy), (dx, dy));
        }
    }
    // (Out of the room, it keeps going the way it faces.)
    assert!(at.x > 12 && at.y == 7, "{at:?}");
    assert_eq!(facing, (1, 0));
}

fn fighter(level: i32, attack: i32, defense: i32, types: (u8, u8)) -> Fighter {
    Fighter {
        stats: Stats {
            level,
            max_hp: 50,
            hp: 50,
            attack,
            defense,
            sp_attack: attack,
            sp_defense: defense,
            types,
            abilities: [0, 0],
            held: 0,
            iq: 1,
            iq_group: 0,
            iq_skills: 0,
            female: false,
        },
        attack_stage: 10,
        defense_stage: 10,
        attack_multiplier: FIXED_ONE,
        defense_multiplier: FIXED_ONE,
        sp_attack_stage: 10,
        sp_defense_stage: 10,
        sp_attack_multiplier: FIXED_ONE,
        sp_defense_multiplier: FIXED_ONE,
        halves_physical: false,
        halves_special: false,
        team_member: true,
        burned: false,
        ailing: false,
        focused: false,
        lucky: false,
        reaches: (false, false),
    }
}

fn chart() -> TypeChart {
    let mut matchups = [[matchup::NEUTRAL; TYPE_COUNT]; TYPE_COUNT];
    matchups[3][2] = matchup::SUPER_EFFECTIVE;
    matchups[3][9] = matchup::SUPER_EFFECTIVE;
    matchups[3][4] = matchup::NOT_VERY_EFFECTIVE;
    matchups[3][5] = matchup::IMMUNE;
    TypeChart {
        matchups,
        multipliers: [0x80, 0xB5, 0x100, 0x166],
        erratic: [0x40, 0x80, 0x100, 0x1B3],
        combinator: [[0, 0, 0, 1], [0, 1, 1, 2], [0, 1, 2, 3], [1, 2, 3, 3]],
        ln_table: Vec::new(),
    }
}

/// The sum of many blows, to compare rules through the random spread.
fn total_of(attacker: &Fighter, defender: &Fighter, blow: impl Fn(&mut DungeonRng, &Fighter, &Fighter) -> Damage) -> i32 {
    let mut rng = DungeonRng::new(7);
    (0..300).map(|_| blow(&mut rng, attacker, defender).amount).sum()
}

/// A physical, typeless, non-critical blow of power 6.
fn plain_blow(rng: &mut DungeonRng, attacker: &Fighter, defender: &Fighter) -> Damage {
    calc_damage(rng, &Constants::default(), None, attacker, defender, 0, 6, 0, FIXED_ONE)
}

#[test]
fn damage_grows_with_level_power_and_attack() {
    let constants = Constants::default();
    let mut rng = DungeonRng::new(99);
    let (attacker, defender) = (fighter(10, 20, 20, (1, 0)), fighter(10, 20, 20, (1, 0)));
    for _ in 0..200 {
        let damage = plain_blow(&mut rng, &attacker, &defender);
        assert!(!damage.critical && damage.matchup == matchup::NEUTRAL);
        assert!((4..=20).contains(&damage.amount), "{damage:?}");
    }
    let plain = total_of(&attacker, &defender, plain_blow);
    assert!(total_of(&fighter(30, 20, 20, (1, 0)), &defender, plain_blow) > plain);
    assert!(total_of(&fighter(10, 40, 20, (1, 0)), &defender, plain_blow) > plain);
    assert!(total_of(&attacker, &fighter(10, 20, 40, (1, 0)), plain_blow) < plain);
    let stronger = |rng: &mut DungeonRng, a: &Fighter, d: &Fighter| calc_damage(rng, &constants, None, a, d, 0, 12, 0, FIXED_ONE);
    assert!(total_of(&attacker, &defender, stronger) > plain);
    // Foes of the team hit a little softer.
    assert!(total_of(&Fighter { team_member: false, ..attacker }, &defender, plain_blow) < plain);
    // Even the weakest blow does something; nothing passes the cap.
    let weakest = calc_damage(&mut rng, &constants, None, &fighter(1, 1, 255, (1, 0)), &defender, 0, 1, 0, FIXED_ONE);
    assert!(weakest.amount >= 1);
    let strongest = calc_damage(&mut rng, &constants, None, &fighter(100, 255, 1, (1, 0)), &defender, 0, 200, 0, 4 * FIXED_ONE);
    assert_eq!(strongest.amount, 999);
}

#[test]
fn accuracy_stages_and_weather_change_hits() {
    let mut rng = DungeonRng::new(3);
    let hits = |rng: &mut DungeonRng, accuracy, acc, eva| (0..2000).filter(|_| move_hit_check(rng, accuracy, (acc, eva))).count();
    // At neutral stages, the plain percentage; above 100 never misses.
    assert_eq!(hits(&mut rng, 125, 0, 20), 2000);
    assert_eq!(hits(&mut rng, 100, 10, 10), 2000);
    let plain = hits(&mut rng, 60, 10, 10);
    assert!((1100..1300).contains(&plain), "{plain}");
    // The roll is drawn even for a sure hit.
    let state = rng.state();
    move_hit_check(&mut rng, 125, (10, 10));
    assert_ne!(rng.state(), state);
    // Four stages down halve the aim; four stages of evasion halve it again.
    let lowered = hits(&mut rng, 100, 6, 10);
    assert!((900..1100).contains(&lowered), "{lowered}");
    let dodged = hits(&mut rng, 100, 6, 14);
    assert!((400..600).contains(&dodged), "{dodged}");
    assert!(hits(&mut rng, 60, 14, 10) > plain);
    // Stages out of range are clamped.
    assert_eq!(hits(&mut rng, 100, 99, -5), 2000);
    // Thunder in the rain and Blizzard in hail never miss; the sun makes
    // Thunder less accurate.
    let thunder = MoveSlot { id: 64, power: 12, accuracy: 60, type_id: 5, category: 1, pp: 5, max_pp: 5, ..MoveSlot::fallback_regular_attack() };
    let constants = Constants::default();
    let mut landed = |slot: &MoveSlot, weather| (0..1000).filter(|_| move_hits(&mut rng, &constants, slot, (10, 10), weather, (false, false))).count();
    assert_eq!(landed(&thunder, weather::RAIN), 1000);
    let clear = landed(&thunder, weather::CLEAR);
    assert!((540..660).contains(&clear), "{clear}");
    assert!(landed(&thunder, weather::SUNNY) < clear - 150);
    let blizzard = MoveSlot { id: 270, ..thunder };
    assert_eq!(landed(&blizzard, weather::HAIL), 1000);
    assert!(landed(&blizzard, weather::RAIN) < 700);
}

#[test]
fn stages_follow_quarter_steps() {
    assert_eq!(stage_factor(10), 1.0);
    assert_eq!(stage_factor(14), 2.0);
    assert_eq!(stage_factor(6), 0.5);
    assert_eq!(stage_factor(99), stage_factor(20));
    assert!(stage_factor(0) > 0.0);
}

#[test]
fn type_matchups_stack_over_both_types() {
    let chart = chart();
    let user = fighter(5, 10, 10, (1, 0)).stats;
    let target = |types| fighter(5, 10, 10, types).stats;
    let effects = |attacker: &Stats, types, attack_type| type_effects(Some(&chart), attacker, &target(types), attack_type, 10, (false, false));
    let (strong, weak, half) = (0x166 as f64 / 256.0, 0xB5 as f64 / 256.0, 0.5);
    assert_eq!(effects(&user, (2, 0), 3), (strong, matchup::SUPER_EFFECTIVE));
    assert_eq!(effects(&user, (2, 9), 3), (strong * strong, matchup::SUPER_EFFECTIVE));
    assert_eq!(effects(&user, (2, 4), 3), (strong * weak, matchup::NEUTRAL));
    // An immunity is a multiplier like the others, from the chart.
    assert_eq!(effects(&user, (5, 2), 3), (half * strong, matchup::NOT_VERY_EFFECTIVE));
    assert_eq!(effects(&user, (5, 0), 3), (half, matchup::IMMUNE));
    assert_eq!(effects(&user, (2, 0), 18), (1.0, matchup::NEUTRAL));
    assert!(chart.immune(3, &target((5, 2))) && !chart.immune(3, &target((2, 0))));
    assert!(chart.immune(1, &target((14, 0))) && !chart.immune(2, &target((14, 0))));
    // A Scrappy-like exclusive item: Normal moves reach Ghosts.
    let reached = type_effects(Some(&chart), &user, &target((14, 0)), 1, 10, (true, false));
    assert_eq!(reached, (1.0, matchup::NEUTRAL));
    // Without a chart everything is neutral.
    assert_eq!(type_effects(None, &user, &target((2, 0)), 3, 10, (false, false)), (1.0, matchup::NEUTRAL));
}

#[test]
fn types_weather_and_abilities_change_damage() {
    let chart = &chart();
    let constants = &Constants::default();
    let blow = |attack_type: u8, weather: u8| {
        move |rng: &mut DungeonRng, a: &Fighter, d: &Fighter| calc_damage_with(rng, constants, Some(chart), a, d, attack_type, 6, 0, FIXED_ONE, false, weather)
    };
    // A Fighting-type attacker: none of the moves below are its own type.
    let (attacker, defender) = (fighter(20, 40, 30, (7, 0)), fighter(20, 40, 30, (1, 0)));
    let ratio = |a: &Fighter, d: &Fighter, attack_type, weather| {
        total_of(a, d, blow(attack_type, weather)) as f64 / total_of(&attacker, &defender, blow(3, weather::CLEAR)) as f64
    };
    // Super effective does more than neutral, not very effective less.
    assert!(ratio(&attacker, &fighter(20, 40, 30, (2, 0)), 3, weather::CLEAR) > 1.3);
    assert!(ratio(&attacker, &fighter(20, 40, 30, (4, 0)), 3, weather::CLEAR) < 0.8);
    // The same-type bonus.
    assert!((ratio(&fighter(20, 40, 30, (3, 0)), &defender, 3, weather::CLEAR) - 1.5).abs() < 0.05);
    // Sun helps Fire and hinders Water, rain the other way round.
    assert!((ratio(&attacker, &defender, 2, weather::SUNNY) - 1.5).abs() < 0.05);
    assert!((ratio(&attacker, &defender, 3, weather::SUNNY) - 0.5).abs() < 0.05);
    assert!((ratio(&attacker, &defender, 3, weather::RAIN) - 1.5).abs() < 0.05);
    assert!(ratio(&attacker, &defender, 5, weather::CLOUDY) < 1.0);
    assert!((ratio(&attacker, &defender, 1, weather::CLOUDY) - 1.0).abs() < 0.05);
    // Levitate takes Ground moves in whole; Mold Breaker gets through.
    let floating = Fighter { stats: Stats { abilities: [crate::dungeon::ability::LEVITATE, 0], ..defender.stats }, ..defender };
    let mut rng = DungeonRng::new(1);
    let ground = calc_damage(&mut rng, constants, Some(chart), &attacker, &floating, 9, 6, 0, FIXED_ONE);
    assert_eq!((ground.amount, ground.matchup), (0, matchup::IMMUNE));
    let breaker = Fighter { stats: Stats { abilities: [crate::dungeon::ability::MOLD_BREAKER, 0], ..attacker.stats }, ..attacker };
    assert!(calc_damage(&mut rng, constants, Some(chart), &breaker, &floating, 9, 6, 0, FIXED_ONE).amount > 0);
    // A pinch ability wakes up at a quarter of its HP.
    let blazing = Fighter { stats: Stats { abilities: [crate::dungeon::ability::BLAZE, 0], ..attacker.stats }, ..attacker };
    let cornered = Fighter { stats: Stats { hp: 10, ..blazing.stats }, ..blazing };
    assert!(total_of(&cornered, &defender, blow(2, weather::CLEAR)) > total_of(&blazing, &defender, blow(2, weather::CLEAR)));
}

#[test]
fn stages_criticals_and_multipliers_change_damage() {
    let constants = &Constants::default();
    let blow = |crit: i32, mult: i32, special: bool| {
        move |rng: &mut DungeonRng, a: &Fighter, d: &Fighter| {
            let damage = calc_damage_with(rng, constants, None, a, d, 0, 8, crit, mult, special, weather::CLEAR);
            assert!(damage.amount >= 1);
            assert_eq!(damage.critical, crit >= 100);
            damage
        }
    };
    let (attacker, defender) = (fighter(20, 40, 30, (1, 0)), fighter(20, 40, 30, (1, 0)));
    let total = |a: &Fighter, d: &Fighter| total_of(a, d, blow(0, FIXED_ONE, false));
    let special = |a: &Fighter, d: &Fighter| total_of(a, d, blow(0, FIXED_ONE, true));
    let plain = total(&attacker, &defender);
    assert!(total(&Fighter { attack_stage: 14, ..attacker }, &defender) > plain);
    assert!(total(&attacker, &Fighter { defense_stage: 14, ..defender }) < plain);
    // Special moves use the special stats and their own stages.
    assert_eq!(special(&attacker, &defender), plain);
    let strong = Fighter { stats: Stats { sp_attack: 90, ..attacker.stats }, ..attacker };
    assert!(special(&strong, &defender) > plain);
    assert_eq!(total(&strong, &defender), plain);
    assert!(special(&Fighter { sp_attack_stage: 14, ..attacker }, &defender) > plain);
    assert_eq!(special(&Fighter { attack_stage: 14, ..attacker }, &defender), plain);
    // Light Screen halves special damage only, Reflect physical damage.
    let ratio = |value: i32| value as f64 / plain as f64;
    let screened = Fighter { halves_special: true, ..defender };
    assert!((ratio(special(&attacker, &screened)) - 0.5).abs() < 0.03);
    assert_eq!(total(&attacker, &screened), plain);
    assert!((ratio(total(&attacker, &Fighter { halves_physical: true, ..defender })) - 0.5).abs() < 0.03);
    // A burn weakens physical blows only.
    assert!((ratio(total(&Fighter { burned: true, ..attacker }, &defender)) - 0.5).abs() < 0.03);
    assert_eq!(special(&Fighter { burned: true, ..attacker }, &defender), plain);
    // Charm lowers the attack, Screech the defense.
    assert!(total(&Fighter { attack_multiplier: 0x80, ..attacker }, &defender) < plain);
    assert!(total(&attacker, &Fighter { defense_multiplier: 0x40, ..defender }) > plain);
    // A sure critical hit is x1.5, a doubled multiplier x2.
    assert!((ratio(total_of(&attacker, &defender, blow(100, FIXED_ONE, false))) - 1.5).abs() < 0.02);
    assert!((ratio(total_of(&attacker, &defender, blow(0, 2 * FIXED_ONE, false))) - 2.0).abs() < 0.02);
    // Armor keeps criticals off.
    let armored = Fighter { stats: Stats { abilities: [crate::dungeon::ability::SHELL_ARMOR, 0], ..defender.stats }, ..defender };
    let mut rng = DungeonRng::new(5);
    assert!(!calc_damage(&mut rng, constants, None, &Fighter { focused: true, ..attacker }, &armored, 0, 8, 100, FIXED_ONE).critical);
    // A band adds to the stat it raises.
    let banded = Fighter { stats: Stats { held: crate::dungeon::held::POWER_BAND, ..attacker.stats }, ..attacker };
    assert!(total(&banded, &defender) > plain);
    assert_eq!(special(&banded, &defender), plain);
}

#[test]
fn ranges_pick_the_right_tiles() {
    let mut floor = open_room();
    let from = Pos::new(6, 7);
    let enemies = [Pos::new(7, 7), Pos::new(7, 8), Pos::new(9, 7), Pos::new(6, 5)];
    let targets = |floor: &Floor, range| targets_in_range(floor, from, (1, 0), range, &enemies);
    assert_eq!(targets(&floor, range::FRONT), vec![Pos::new(7, 7)]);
    assert_eq!(targets(&floor, range::FRONT_AND_SIDES).len(), 2);
    assert_eq!(targets(&floor, range::ROOM).len(), 4);
    // A line stops at the first enemy, and at walls.
    assert_eq!(targets(&floor, range::FRONT_10), vec![Pos::new(7, 7)]);
    floor.set(Pos::new(7, 7), Tile::Wall);
    assert!(targets(&floor, range::FRONT_10).is_empty());
    // From a hallway, a room-wide move reaches whoever is in sight.
    floor.set(from, Tile::Floor { room: None });
    floor.hallway_sight = 2;
    assert_eq!(targets(&floor, range::ROOM).len(), 3);
    floor.hallway_sight = 1;
    assert_eq!(targets(&floor, range::ROOM).len(), 2);
}

#[test]
fn the_fairy_type_fares_as_in_the_later_games() {
    const DRAGON: u8 = 15;
    let chart = chart();
    let dragon = fighter(10, 20, 20, (DRAGON, 0)).stats;
    let fairy = fighter(10, 20, 20, (FAIRY, 0)).stats;
    assert_eq!(chart.overall(FAIRY, &dragon), matchup::SUPER_EFFECTIVE);
    assert!(chart.immune(DRAGON, &fairy));
    // Poison on a Fairy, Fairy on Steel; its name and its word.
    assert_eq!(chart.matchup(8, FAIRY), matchup::SUPER_EFFECTIVE);
    assert_eq!(chart.matchup(FAIRY, 17), matchup::NOT_VERY_EFFECTIVE);
    assert_eq!(chart.matchup(FAIRY, 1), matchup::NEUTRAL);
    assert_eq!((type_name(FAIRY), type_named("fairy"), type_named("Electric")), ("Fairy", Some(FAIRY), Some(5)));
    // The game's own matchups are left as they were.
    assert_eq!(chart.matchup(3, 2), matchup::SUPER_EFFECTIVE);
}
