use super::*;
use crate::dungeon::status::tests::{RULES, monster, slot};
use crate::dungeon::status::{Shield, move_effect};

/// A blow of that move from one fresh monster on another, as they are.
fn blow(id: u16, user: (&mut Stats, &mut Statuses), victim: (&mut Stats, &mut Statuses)) -> Outcome {
    let attack = Attack { slot: MoveSlot { id, accuracy: 125, ..slot() }, effect: None, damage_mult: FIXED_ONE, linked: false };
    attack_with(3, &attack, user, victim)
}

/// An attack, chance drawn from that seed.
fn attack_with(seed: u32, attack: &Attack, user: (&mut Stats, &mut Statuses), victim: (&mut Stats, &mut Statuses)) -> Outcome {
    let mut rng = DungeonRng::new(seed);
    let mut bonds = 0;
    let mut field = Field {
        rng: &mut rng,
        constants: &RULES,
        chart: None,
        moves: &[],
        weather: 0,
        gravity: false,
        mud_sport: false,
        water_sport: false,
        secret_power: 9,
        bonds: &mut bonds,
        sides: Default::default(),
    };
    let mut user = Party { stats: user.0, status: user.1, team: true, leader: true, at: Pos::new(5, 5), id: 1 };
    let mut victim = Party { stats: victim.0, status: victim.1, team: false, leader: false, at: Pos::new(6, 5), id: 2 };
    resolve(&mut user, &mut victim, attack, Tuning::default(), &mut field)
}

#[test]
fn abilities_that_answer_a_move() {
    use crate::dungeon::ability::*;
    let sturdy = || Stats { hp: 200, max_hp: 200, ..monster() };
    // Color Change takes the type of the move that hit.
    let (mut one, mut other) = (sturdy(), Stats { types: (2, 0), abilities: [COLOR_CHANGE, 0], ..sturdy() });
    let outcome = blow(1, (&mut one, &mut Statuses::default()), (&mut other, &mut Statuses::default()));
    assert!(outcome.happened.contains(&Happened::ColorChanged(1)) && other.types == (1, 0));
    // Trace copies the user's ability in its place.
    let (mut one, mut tracer) = (Stats { abilities: [KEEN_EYE, 0], ..sturdy() }, Stats { abilities: [TRACE, 0], ..sturdy() });
    let outcome = blow(1, (&mut one, &mut Statuses::default()), (&mut tracer, &mut Statuses::default()));
    assert!(outcome.happened.contains(&Happened::Traced) && tracer.abilities[0] == KEEN_EYE);
    // Rock Head: no recoil (Submission).
    let recoil = |outcome: &Outcome| outcome.happened.iter().any(|happened| matches!(happened, Happened::Aftermath(_)));
    let (mut head, mut target) = (Stats { abilities: [ROCK_HEAD, 0], ..sturdy() }, sturdy());
    assert!(!recoil(&blow(116, (&mut head, &mut Statuses::default()), (&mut target, &mut Statuses::default()))));
    head.abilities = [0, 0];
    assert!(recoil(&blow(116, (&mut head, &mut Statuses::default()), (&mut target, &mut Statuses::default()))));
    // Synchronize shares a poisoning.
    let poison =
        Attack { slot: MoveSlot { id: 77, power: 0, accuracy: 125, ..slot() }, effect: Some(MoveEffect::Poison), damage_mult: FIXED_ONE, linked: false };
    let (mut one, mut synced) = (sturdy(), Stats { abilities: [SYNCHRONIZE, 0], ..sturdy() });
    let outcome = attack_with(3, &poison, (&mut one, &mut Statuses::default()), (&mut synced, &mut Statuses::default()));
    assert!(outcome.happened.contains(&Happened::Synchronized { on: Role::Victim, effect: MoveEffect::Poison }));
    // Aftermath: knocked out, it blows up now and then.
    let strike = Attack { slot: MoveSlot { accuracy: 125, ..slot() }, effect: None, damage_mult: FIXED_ONE, linked: false };
    let burst = (1..20).any(|seed| {
        let (mut one, mut weak) = (sturdy(), Stats { hp: 1, abilities: [AFTERMATH, 0], ..monster() });
        attack_with(seed, &strike, (&mut one, &mut Statuses::default()), (&mut weak, &mut Statuses::default())).happened.contains(&Happened::Burst)
    });
    assert!(burst);
}

#[test]
fn the_moves_of_their_own_handlers() {
    let (mut one, mut other) = (Stats { hp: 200, max_hp: 200, ..monster() }, Stats { hp: 200, max_hp: 200, ..monster() });
    // Pursuit strikes back as Counter; Outrage leaves its user confused.
    assert_eq!(move_effect(38), Some(MoveEffect::Counter(crate::dungeon::status::CounterKind::Physical)));
    assert_eq!(crate::dungeon::user_effect(91, &RULES), Some((100, MoveEffect::Confuse)));
    // Brick Break breaks a Reflect.
    let (mut mine, mut theirs) = (Statuses::default(), Statuses::default());
    theirs.shield = Some(Shield::Reflect);
    let outcome = blow(72, (&mut one, &mut mine), (&mut other, &mut theirs));
    assert!(outcome.happened.contains(&Happened::Shattered) && theirs.shield.is_none());
    // SmellingSalt cures the paralysis.
    theirs.paralysis_turns = 3;
    let outcome = blow(78, (&mut one, &mut mine), (&mut other, &mut theirs));
    assert!(outcome.happened.contains(&Happened::Unparalyzed) && theirs.paralysis_turns == 0);
    // Rapid Spin frees its user.
    (mine.constricted_turns, mine.leech_seed_turns) = (4, 4);
    let outcome = blow(96, (&mut one, &mut mine), (&mut other, &mut theirs));
    assert!(outcome.happened.contains(&Happened::SpunFree) && mine.constricted_turns == 0 && mine.leech_seed_turns == 0);
}

#[test]
fn a_pass_scarf_passes_the_effect_on() {
    let mut rng = DungeonRng::new(3);
    let mut bonds = 0;
    let mut field = Field {
        rng: &mut rng,
        constants: &RULES,
        chart: None,
        moves: &[],
        weather: 0,
        gravity: false,
        mud_sport: false,
        water_sport: false,
        secret_power: 9,
        bonds: &mut bonds,
        sides: Default::default(),
    };
    let (mut one, mut other) = (monster(), Stats { held: crate::dungeon::held::PASS_SCARF, ..monster() });
    let (mut mine, mut theirs) = (Statuses::default(), Statuses::default());
    let mut user = Party { stats: &mut one, status: &mut mine, team: true, leader: true, at: Pos::new(5, 5), id: 1 };
    let mut victim = Party { stats: &mut other, status: &mut theirs, team: false, leader: false, at: Pos::new(6, 5), id: 2 };
    let attack =
        Attack { slot: MoveSlot { id: 79, power: 0, accuracy: 125, ..slot() }, effect: Some(MoveEffect::Sleep), damage_mult: FIXED_ONE, linked: false };
    let outcome = resolve(&mut user, &mut victim, &attack, Tuning::default(), &mut field);
    assert!(outcome.happened.contains(&Happened::Passed(MoveEffect::Sleep)) && !theirs.asleep());
}
