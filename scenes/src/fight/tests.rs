//! The rules of a fight, on small tables made by hand.

use super::battle::Battle as Fight;
use super::*;
use rules::dungeon::combat::matchup;
use rules::dungeon::weather;

const TACKLE: u16 = 154;
const EMBER: u16 = 52;
const FURY: u16 = 200;
const SWORDS: u16 = 14;
const HEAL: u16 = 105;
const BLAST: u16 = 155;
const BEAM: u16 = 242;

fn slot(id: u16, power: i32, type_id: u8, category: u8, strikes: i32) -> MoveSlot {
    MoveSlot {
        id,
        power,
        accuracy: 125,
        accuracy2: 125,
        strikes,
        pp: 10,
        max_pp: 10,
        type_id,
        category,
        sealed: false,
        crit_chance: 0,
        range: 0,
        target: 0,
        ai_weight: 0,
        ai_target_range: 0,
        ai_chance: 0,
        linked: false,
    }
}

fn known(slot: MoveSlot, name: &str) -> Move {
    Move { slot, name: name.to_string(), flight: 6.0, flies: false }
}

/// A chart where fire is strong on grass and useless on rock.
fn chart() -> TypeChart {
    let mut matchups = [[matchup::NEUTRAL; 18]; 18];
    matchups[2][4] = matchup::SUPER_EFFECTIVE;
    matchups[2][13] = matchup::IMMUNE;
    let mut combinator = [[matchup::NEUTRAL; 4]; 4];
    combinator[3][2] = matchup::SUPER_EFFECTIVE;
    combinator[2][3] = matchup::SUPER_EFFECTIVE;
    combinator[0] = [matchup::IMMUNE; 4];
    combinator[1][2] = matchup::NOT_VERY_EFFECTIVE;
    TypeChart { matchups, multipliers: [0, 128, 256, 512], erratic: [0; 4], combinator, ln_table: Vec::new() }
}

pub(super) fn rules() -> Rules {
    let mut moves: Vec<Move> = (0..300).map(|id| known(slot(id, 0, 1, 2, 1), "Nothing")).collect();
    moves[TACKLE as usize] = known(slot(TACKLE, 40, 1, 0, 1), "Tackle");
    moves[EMBER as usize] = known(slot(EMBER, 60, 2, 1, 1), "Ember");
    moves[FURY as usize] = known(slot(FURY, 15, 1, 0, 0), "Fury Attack");
    moves[SWORDS as usize] = known(slot(SWORDS, 0, 1, 2, 1), "Swords Dance");
    moves[HEAL as usize] = known(slot(HEAL, 0, 1, 2, 1), "Recover");
    moves[BLAST as usize] = known(slot(BLAST, 0, 1, 2, 1), "Explosion");
    moves[BEAM as usize] = known(slot(BEAM, 150, 1, 1, 1), "Hyper Beam");
    let kind = |name: &str, types, learnset: &[u16]| Kind {
        name: name.to_string(),
        types,
        base: [50, 30, 30, 30, 30],
        growth: Vec::new(),
        movement: 0,
        learnset: learnset.iter().map(|id| (1, *id)).collect(),
    };
    let kinds = [(1, kind("Charmander", (2, 0), &[TACKLE, EMBER])), (2, kind("Chikorita", (4, 0), &[TACKLE])), (3, kind("Geodude", (13, 0), &[TACKLE, FURY]))];
    Rules::new(Default::default(), Some(chart()), moves, kinds.into_iter().collect())
}

pub(super) fn entrant(number: u16, species: u16) -> Entrant {
    Entrant { who: Who::Monster(number), species, level: 10, life: None, moves: Vec::new(), seed: false, waits: false }
}

pub(super) fn setup(mode: Mode, first: Vec<Entrant>, second: Vec<Entrant>) -> Setup {
    Setup { mode, sides: [first, second], winner: None, seed: 7, sky: weather::CLEAR, edges: None, foes_life: 1.0, hardest: 20, direction: None }
}

pub(super) fn fight(setup: Setup) -> Fight {
    Fight::new(&setup, Arc::new(rules()))
}

/// All that happens, a fight left to itself (whoever plays taking `chosen`).
fn played_out(fight: &mut Fight, chosen: impl Fn(&Fight) -> Choice) -> Vec<Sight> {
    let mut seen = Vec::new();
    for _ in 0..2000 {
        if fight.over().is_some() {
            return seen;
        }
        let choice = fight.waiting().then(|| chosen(fight));
        seen.extend(fight.step(choice));
    }
    panic!("the fight does not end");
}

/// A fight of two against two, stated in advance.
fn directed(direction: Direction) -> Fight {
    let mut made = setup(Mode::Together, vec![entrant(1, 1), entrant(2, 2)], vec![entrant(3, 3), entrant(4, 3)]);
    for one in made.sides.iter_mut().flatten() {
        one.life = Some((500, 500));
    }
    made.direction = Some(direction);
    fight(made)
}

fn act(who: u16, power: u16, target: Option<u16>, told: Told) -> Act {
    Act { targets: target.map(Who::Monster).into_iter().collect(), told, ..Act::of(Who::Monster(who), power) }
}

fn acts(all: Vec<Act>) -> Direction {
    Direction { acts: all.into_iter().map(Stated::Act).collect(), ..Direction::default() }
}

/// The blows of what was seen, in their order.
fn blows(seen: &[Sight]) -> Vec<&Blow> {
    seen.iter().filter_map(|sight| if let Sight::Blow(blow) = sight { Some(blow) } else { None }).collect()
}

fn alone(first: u16, second: u16) -> Fight {
    fight(setup(Mode::Alone, vec![entrant(first, first)], vec![entrant(second, second)]))
}

#[test]
fn a_blow_said_critical_with_its_number_and_taken_by_another() {
    let hit = Act { critical: true, slow: true, slowed_hit: Some(30), taken_by: Some(Who::Monster(4)), ..act(1, TACKLE, Some(3), Told::Takes(60)) };
    let mut battle = directed(acts(vec![hit]));
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    let blown = blows(&seen);
    assert_eq!(blown[0].landings[0].target, Who::Monster(4));
    assert_eq!(blown[0].landings[0].strikes[0].taken, 60);
    assert!(blown[0].landings[0].critical);
    let has = |test: &dyn Fn(&Sight) -> bool| seen.iter().any(test);
    assert!(has(&|sight| matches!(sight, Sight::StepsBetween { who, target, .. } if (*who, *target) == (Who::Monster(4), Who::Monster(3)))));
    assert!(has(&|sight| matches!(sight, Sight::StepsBack { .. })));
    assert!(has(&|sight| matches!(sight, Sight::SlowHit(30))));
    assert_eq!(seen.iter().filter(|sight| matches!(sight, Sight::Tempo(_))).count(), 2, "slowed, then as usual");
}

#[test]
fn what_is_stated_between_two_blows_comes_in_its_order() {
    let mut direction = acts(vec![act(1, TACKLE, Some(3), Told::Takes(1))]);
    direction.acts.extend([Stated::Pause(60), Stated::Log("The ground shakes".to_string()), Stated::Passage("dit".to_string())]);
    let mut battle = directed(direction);
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    let at = |test: &dyn Fn(&Sight) -> bool| seen.iter().position(test).expect("seen");
    let (paused, said, passage) =
        (at(&|sight| matches!(sight, Sight::Waits(60))), at(&|sight| matches!(sight, Sight::Says(_))), at(&|sight| matches!(sight, Sight::Passage(_))));
    assert!(paused < said && said < passage);
}

#[test]
fn one_who_waits_is_in_nothing_until_it_is_given_a_place() {
    let mut made = setup(Mode::Together, vec![entrant(1, 1), entrant(2, 2)], vec![entrant(3, 3)]);
    made.sides[0][1].waits = true;
    let mut direction = acts(vec![act(3, TACKLE, Some(2), Told::Takes(1))]);
    direction.acts.push_back(Stated::Change { out: Who::Monster(1), comes: Who::Monster(2) });
    direction.acts.push_back(Stated::Act(act(3, TACKLE, Some(2), Told::Takes(1))));
    made.direction = Some(direction);
    let mut battle = fight(made);
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    let blown = blows(&seen);
    assert_eq!(blown[0].landings[0].target, Who::Monster(1), "not there yet");
    assert_eq!(blown[1].landings[0].target, Who::Monster(2));
    assert!(seen.iter().any(|sight| matches!(sight, Sight::Swaps { out, comes, .. } if (*out, *comes) == (Who::Monster(1), Who::Monster(2)))));
}

#[test]
fn acts_are_played_in_their_order_each_as_it_is_said() {
    let script = vec![
        act(1, TACKLE, Some(3), Told::Takes(3)),
        act(3, TACKLE, Some(2), Told::Misses),
        act(2, TACKLE, Some(4), Told::Real),
        act(1, TACKLE, Some(3), Told::Critical),
        act(2, TACKLE, Some(3), Told::Shielded),
        act(1, TACKLE, Some(4), Told::Falls),
    ];
    let mut battle = directed(acts(script));
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    let blown = blows(&seen);
    assert_eq!(blown.iter().map(|blow| blow.who).collect::<Vec<Who>>(), [1, 3, 2, 1, 2, 1].map(Who::Monster));
    assert_eq!(blown[0].landings[0].strikes[0].taken, 3);
    assert!(blown[1].landings[0].missed);
    assert!(blown[2].landings[0].strikes[0].taken > 0, "from the formula");
    assert!(blown[3].landings[0].critical);
    assert!(blown[4].landings[0].blocked);
    assert!(seen.iter().any(|sight| matches!(sight, Sight::Shields { who } if *who == Who::Monster(3))));
    assert_eq!(blown[5].landings[0].strikes[0].left, 0);
    assert!(seen.iter().any(|sight| matches!(sight, Sight::Falls { who, .. } if *who == Who::Monster(4))));
    assert!(matches!(seen.last(), Some(Sight::Won { .. })), "it stops with its acts");
}

#[test]
fn a_blow_on_all_lands_on_each_foe_standing() {
    let mut all = act(1, TACKLE, None, Told::Takes(4));
    all.all = true;
    let mut battle = directed(acts(vec![act(1, TACKLE, Some(4), Told::Falls), all.clone(), {
        all.all = true;
        all.clone()
    }]));
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    assert_eq!(blows(&seen)[1].landings.len(), 1, "one was down");
    let mut both = directed(acts(vec![all]));
    let seen = played_out(&mut both, |_| Choice::Move(0));
    assert_eq!(blows(&seen)[0].landings.len(), 2);
}

#[test]
fn who_has_fallen_plays_nothing_and_a_written_blow_lands() {
    let mut battle = directed(acts(vec![act(1, TACKLE, Some(3), Told::Falls), act(3, TACKLE, Some(1), Told::Real), act(1, TACKLE, Some(4), Told::Real)]));
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    let blown = blows(&seen);
    assert_eq!(blown.iter().map(|blow| blow.who).collect::<Vec<Who>>(), [1, 1].map(Who::Monster));
    assert!(!blown[1].landings[0].missed);
}

#[test]
fn the_rest_by_chance_keeps_who_is_to_win_standing() {
    let mut made = setup(Mode::Together, vec![entrant(1, 1), entrant(2, 2)], vec![entrant(3, 3), entrant(4, 3)]);
    made.winner = Some(0);
    made.direction = Some(Direction { goes_on: true, chosen: Some(Who::Monster(2)), ..acts(vec![act(1, TACKLE, Some(3), Told::Takes(5))]) });
    let mut battle = fight(made);
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    assert_eq!(battle.over(), Some(0));
    assert!(!seen.iter().any(|sight| matches!(sight, Sight::Falls { who, .. } if *who == Who::Monster(2))));
    assert_eq!(battle.body(Who::Monster(4)).map(|body| body.down), Some(true));
}

#[test]
fn a_number_of_turns_ends_it_with_last_blows() {
    let mut made = setup(Mode::Together, vec![entrant(1, 1), entrant(2, 2)], vec![entrant(3, 3), entrant(4, 3)]);
    made.winner = Some(1);
    made.direction = Some(Direction { goes_on: true, turns: Some(3), ..Direction::default() });
    let mut battle = fight(made);
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    assert_eq!(battle.over(), Some(1));
    assert!(blows(&seen).iter().filter(|blow| blow.landings[0].critical && blow.landings[0].strikes[0].left == 0).count() >= 1);
    assert!(battle.teams[0].bodies.iter().all(|body| body.down));
}

#[test]
fn stats_grow_with_the_level() {
    let gain = LevelGain { total_exp: 0, hp: 3, attack: 2, sp_attack: 1, defense: 1, sp_defense: 1 };
    let kind = Kind { base: [10, 5, 5, 5, 5], growth: vec![gain; 100], ..Kind::default() };
    let stats = kind.stats(11);
    assert_eq!((stats.level, stats.max_hp, stats.hp, stats.attack, stats.sp_attack), (11, 40, 40, 25, 15));
}

#[test]
fn a_fight_opens_then_ends_with_a_winner() {
    let mut battle = alone(1, 2);
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    assert!(matches!(seen[0], Sight::Opens { .. }));
    assert!(seen.iter().any(|sight| matches!(sight, Sight::Falls { more: false, .. })));
    assert!(matches!(seen.last(), Some(Sight::Won { .. })));
}

#[test]
fn the_same_seed_the_same_fight() {
    let told = |seed| {
        let mut made = setup(Mode::Alone, vec![entrant(1, 1)], vec![entrant(3, 3)]);
        made.seed = seed;
        format!("{:?}", played_out(&mut fight(made), |_| Choice::Move(0)))
    };
    assert_eq!(told(5), told(5));
    assert_ne!(told(5), told(6));
}

#[test]
fn fire_burns_grass_more_and_rock_not_at_all() {
    let (battle, ember) = (alone(1, 2), rules().moves[EMBER as usize].clone());
    let (me, grass, rock) = (battle.fighter(0), battle.fighter(1), alone(1, 3));
    assert!(battle.worth(me, &ember, grass) > alone(1, 1).worth(me, &ember, alone(1, 1).fighter(1)));
    assert_eq!(rock.fares(&ember, rock.fighter(1)), matchup::IMMUNE);
    assert_eq!(battle.fares(&ember, grass), matchup::SUPER_EFFECTIVE);
}

#[test]
fn strikes_are_as_many_as_the_move_says() {
    let mut made = setup(Mode::Played, vec![entrant(3, 3)], vec![entrant(2, 2)]);
    made.sides[0][0].moves = vec![FURY];
    made.sides[1][0].life = Some((900, 900));
    let mut battle = fight(made);
    battle.step(None);
    let seen = battle.step(Some(Choice::Move(0)));
    let Some(Sight::Blow(blow)) = seen.iter().find(|sight| matches!(sight, Sight::Blow(_))) else {
        panic!("{seen:?}");
    };
    assert!((2..=5).contains(&blow.landings[0].strikes.len()), "{blow:?}");
}

#[test]
fn a_shield_stops_one_blow_and_goes() {
    let mut made = setup(Mode::Played, vec![entrant(1, 1)], vec![entrant(2, 2)]);
    made.sides[0][0].life = Some((500, 500));
    let mut battle = fight(made);
    battle.step(None);
    battle.step(Some(Choice::Shield));
    let seen = battle.step(None);
    let blocked = seen.iter().any(|sight| matches!(sight, Sight::Blow(blow) if blow.landings[0].blocked));
    assert!(blocked, "{seen:?}");
    assert_eq!(battle.fighter(0).stats.hp, 500);
}

#[test]
fn a_berry_gives_back_a_third_once() {
    let mut made = setup(Mode::Played, vec![entrant(1, 1)], vec![entrant(2, 2)]);
    made.sides[0][0].life = Some((10, 90));
    let mut battle = fight(made);
    battle.step(None);
    assert!(battle.offers().iter().any(|offer| offer.choice == Choice::Berry));
    let seen = battle.step(Some(Choice::Berry));
    assert!(matches!(seen[0], Sight::Berry { healed: Strike { left: 40, .. }, .. }), "{seen:?}");
    assert!(!battle.offers().iter().any(|offer| offer.choice == Choice::Berry));
}

#[test]
fn a_seed_brings_one_back_once() {
    let mut made = setup(Mode::Alone, vec![entrant(1, 1)], vec![entrant(2, 2)]);
    made.sides[0][0].seed = true;
    made.sides[0][0].life = Some((3, 30));
    made.sides[1][0].life = Some((900, 900));
    let seen = played_out(&mut fight(made), |_| Choice::Move(0));
    assert_eq!(seen.iter().filter(|sight| matches!(sight, Sight::Reborn { .. })).count(), 1, "{seen:?}");
}

#[test]
fn who_is_to_win_holds_on_and_wins() {
    for winner in 0..2 {
        let mut made = setup(Mode::Alone, vec![entrant(1, 2)], vec![entrant(2, 2)]);
        made.winner = Some(winner);
        made.sides[winner][0].life = Some((5, 40));
        let mut battle = fight(made);
        let seen = played_out(&mut battle, |_| Choice::Move(0));
        assert_eq!(battle.over(), Some(winner));
        let held = seen.iter().any(|sight| format!("{sight:?}").contains("held: true"));
        assert!(held, "{seen:?}");
    }
}

#[test]
fn the_next_of_a_team_comes_in_as_one_falls() {
    let mut made = setup(Mode::Alone, vec![entrant(1, 1), entrant(4, 2)], vec![entrant(2, 2), entrant(5, 3), entrant(6, 1)]);
    made.winner = Some(1);
    let mut battle = fight(made);
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    assert_eq!(battle.over(), Some(1));
    let entering = seen.iter().filter(|sight| matches!(sight, Sight::Enters { .. })).count();
    assert!(entering >= 1, "{seen:?}");
}

#[test]
fn who_plays_may_change_and_who_changes_comes_back_as_it_left() {
    let mut battle = fight(setup(Mode::Played, vec![entrant(1, 1), entrant(4, 2)], vec![entrant(2, 2)]));
    battle.step(None);
    assert!(battle.offers().iter().any(|offer| offer.said == "Switch"));
    assert!(battle.step(Some(Choice::Changing)).is_empty());
    let offers = battle.offers();
    assert_eq!(offers.len(), 2);
    assert_eq!(offers[1].choice, Choice::Back);
    let seen = battle.step(Some(offers[0].choice));
    assert!(matches!(seen[0], Sight::GoesBack { .. }) && matches!(seen[1], Sight::Enters { .. }), "{seen:?}");
    assert_eq!(battle.fighter(0).who, Who::Monster(4));
    assert!(battle.waiting(), "a change costs no turn");
}

#[test]
fn a_menu_tells_how_each_move_fares() {
    let mut made = setup(Mode::Played, vec![entrant(1, 1)], vec![entrant(2, 2)]);
    made.sides[0][0].moves = vec![EMBER, TACKLE];
    let mut battle = fight(made);
    battle.step(None);
    let said: Vec<String> = battle.offers().into_iter().map(|offer| offer.said).collect();
    assert_eq!(said, ["Ember +", "Tackle", "Oran Berry", "Protect"]);
    let mut made = setup(Mode::Played, vec![entrant(1, 1)], vec![entrant(3, 3)]);
    made.sides[0][0].moves = vec![EMBER];
    let mut battle = fight(made);
    battle.step(None);
    assert_eq!(battle.offers()[0].said, "Ember x");
}

#[test]
fn a_blast_costs_both() {
    let mut made = setup(Mode::Played, vec![entrant(1, 1), entrant(4, 2)], vec![entrant(2, 2)]);
    made.sides[0][0].moves = vec![BLAST];
    made.sides[1][0].life = Some((80, 80));
    let mut battle = fight(made);
    battle.step(None);
    battle.step(Some(Choice::Move(0)));
    assert_eq!(battle.fighter(1).stats.hp, 40);
    assert_eq!(battle.fighter(0).who, Who::Monster(4));
}

#[test]
fn a_great_beam_leaves_one_to_rest() {
    let mut made = setup(Mode::Played, vec![entrant(1, 1)], vec![entrant(2, 2)]);
    made.sides[0][0].moves = vec![BEAM];
    made.sides[1][0].life = Some((900, 900));
    let mut battle = fight(made);
    battle.step(None);
    battle.step(Some(Choice::Move(0)));
    battle.step(None);
    assert!(!battle.waiting());
    let seen = battle.step(None);
    assert!(matches!(seen[0], Sight::Rests { .. }), "{seen:?}");
}

#[test]
fn hail_wears_down_all_but_those_of_ice() {
    let mut made = setup(Mode::Alone, vec![entrant(1, 1)], vec![entrant(2, 2)]);
    made.sky = weather::HAIL;
    let seen = played_out(&mut fight(made), |_| Choice::Move(0));
    assert!(seen.iter().any(|sight| matches!(sight, Sight::Worn { harsh: true, .. })));
}

#[test]
fn rage_makes_one_stronger() {
    let mut battle = alone(1, 2);
    let before = battle.fighter(0).statuses.attack_stage;
    battle.enraged(Who::Monster(1), 2);
    assert_eq!(battle.fighter(0).statuses.attack_stage, before + 2);
}

#[test]
fn a_fight_that_goes_nowhere_still_ends() {
    let mut made = setup(Mode::Played, vec![entrant(1, 1)], vec![entrant(3, 3)]);
    made.sides[0][0].moves = vec![EMBER];
    made.sides[0][0].life = Some((9000, 9000));
    made.sides[1][0].life = Some((9000, 9000));
    let mut battle = fight(made);
    played_out(&mut battle, |_| Choice::Move(0));
    assert!(battle.over().is_some());
}

#[test]
fn two_against_two_all_on_the_field() {
    let mut made = setup(Mode::Together, vec![entrant(1, 1), entrant(4, 2)], vec![entrant(2, 2), entrant(5, 3)]);
    made.winner = Some(0);
    let mut battle = fight(made);
    let seen = played_out(&mut battle, |_| Choice::Move(0));
    assert_eq!(battle.over(), Some(0));
    assert_eq!(
        seen.iter().filter(|sight| matches!(sight, Sight::Leaves { .. })).count(),
        seen.iter().filter(|sight| matches!(sight, Sight::Falls { .. })).count()
    );
}

#[test]
fn a_foe_uses_what_does_not_strike_once() {
    let mut made = setup(Mode::Alone, vec![entrant(1, 1)], vec![entrant(2, 2)]);
    made.sides[1][0].moves = vec![TACKLE, SWORDS];
    made.sides[0][0].life = Some((900, 900));
    made.sides[1][0].life = Some((900, 900));
    let seen = played_out(&mut fight(made), |_| Choice::Move(0));
    let deeds = seen.iter().filter(|sight| matches!(sight, Sight::Deed(_))).count();
    assert!(deeds <= 1, "{deeds}");
}

#[test]
fn a_crowd_falls_or_runs() {
    let spot = |x: f32, y: f32| Vec2::new(x, y);
    let side = |first: u16, species: u16, x: f32| {
        (0..12).map(|at| (Who::Another { species: first, copy: at + 1 }, species, spot(x, 20.0 * at as f32))).collect::<Vec<_>>()
    };
    let mut crowd = Crowd::new([side(1, 1, 100.0), side(2, 2, 200.0)], Some(1), 3, Ground::default(), Arc::new(rules()));
    let mut seen = Vec::new();
    for _ in 0..40 {
        seen.extend(crowd.round());
        if crowd.over() {
            break;
        }
    }
    assert!(crowd.over());
    assert!(seen.iter().any(|rush| matches!(rush, crowd::Rush::Takes { left, .. } if *left <= 0)));
    let glad = |species: u16| seen.iter().filter(|rush| matches!(rush, crowd::Rush::Glad { who: Who::Another { species: of, .. } } if *of == species)).count();
    assert_eq!(glad(1), 0);
    assert!(glad(2) > 0);
}

#[test]
fn the_ground_keeps_one_from_walls() {
    let ground = Ground::from_rows(vec![[vec![false; 10], vec![true]].concat(); 10]);
    let (at, by) = (Vec2::new(60.0, 40.0), Vec2::new(20.0, 40.0));
    assert_eq!(ground.room_behind(at, by, 10.0), 10.0);
    assert!(ground.room_behind(at, by, 30.0) < 30.0);
    assert_eq!(ground.room_behind(at, by, 0.0), 0.0);
    assert!(ground.clear(by, at));
    assert!(!ground.clear(at, Vec2::new(100.0, 40.0)));
    let ground = Ground::from_rows(vec![vec![false, true], vec![true, false]]);
    assert!(ground.blocked(Vec2::new(10.0, 2.0), 0));
    assert!(!ground.blocked(Vec2::new(2.0, 2.0), 0));
    assert!(!Ground::default().known());
}

/// Fights simulated by the hundred, on the game's own tables (none when
/// `data/` is not there): each of them ends, however it is played.
mod simulated {
    use super::*;

    const FIGHTS: u32 = 300;

    fn tables(species: &[u16]) -> Option<Shared> {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/rom/dungeons.bin")).ok()?;
        let tables: Tables = bincode::deserialize(&bytes).ok()?;
        Some(Arc::new(Rules::of(&tables, species)))
    }

    #[test]
    #[ignore = "needs the extracted data/ (run with --ignored)"]
    fn a_variant_fights_as_its_kin() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/rom/dungeons.bin")).expect("the extracted data/");
        let tables: Tables = bincode::deserialize(&bytes).unwrap();
        let (variant, own) = (Rules::of_kin(&tables, &[4001], &|_| 25), Rules::of(&tables, &[25]));
        let (hunter, pikachu) = (&variant.kinds[&4001], &own.kinds[&25]);
        assert_eq!((&hunter.name, hunter.types, &hunter.learnset), (&pikachu.name, pikachu.types, &pikachu.learnset));
    }

    fn one(species: u16, level: i32) -> Entrant {
        Entrant { who: Who::Monster(species), species, level, life: None, moves: Vec::new(), seed: false, waits: false }
    }

    #[test]
    #[ignore = "needs the extracted data/ (run with --ignored)"]
    fn fights_end_however_they_are_played() {
        const PIKACHU: u16 = 25;
        const MEWTWO: u16 = 150;
        const ABO: u16 = 23;
        let rules = tables(&[PIKACHU, MEWTWO, ABO]).expect("the extracted data/");
        for seed in 0..FIGHTS {
            let sides = [vec![one(PIKACHU, 40), one(ABO, 40)], vec![one(MEWTWO, 40), one(ABO, 39)]];
            for mode in [Mode::Played, Mode::Alone] {
                let setup =
                    Setup { mode, sides: sides.clone(), winner: None, seed, sky: weather::CLEAR, edges: None, foes_life: 1.0, hardest: 20, direction: None };
                let mut battle = Battle::new(&setup, rules.clone());
                let mut roll = seed;
                for _ in 0..5000 {
                    if battle.over().is_some() {
                        break;
                    }
                    roll = roll.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    let offers = battle.offers();
                    let choice = battle.waiting().then(|| offers[(roll >> 8) as usize % offers.len()].choice);
                    battle.step(choice);
                }
                assert!(battle.over().is_some(), "{seed}");
            }
        }
    }
}
