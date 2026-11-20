use super::super::tests::{entrant, fight, setup};
use super::*;

fn staged(mode: Mode, played: bool) -> Staging {
    let mut made = setup(mode, vec![entrant(1, 1), entrant(4, 2)], vec![entrant(2, 2), entrant(5, 3)]);
    made.winner = Some(0);
    let mut battle = fight(made);
    let at: Spots = [
        (Who::Monster(1), Vec2::new(100.0, 100.0)),
        (Who::Monster(2), Vec2::new(220.0, 100.0)),
        (Who::Monster(4), Vec2::new(60.0, 100.0)),
        (Who::Monster(5), Vec2::new(260.0, 100.0)),
    ]
    .into();
    let mut staging = Staging::new(Ground::default(), 3, at);
    staging.supporters = vec![Who::Monster(9)];
    staging.stand_ready(&battle, played);
    for _ in 0..2000 {
        if battle.over().is_some() {
            break;
        }
        let choice = battle.waiting().then_some(Choice::Move(0));
        for sight in battle.step(choice) {
            staging.sight(&sight, &battle);
        }
    }
    assert!(battle.over().is_some());
    staging
}

fn told(staging: &Staging) -> String {
    staging.steps.iter().map(|step| format!("{step:?}")).collect::<Vec<_>>().join("\n")
}

#[test]
fn every_line_a_fight_writes_is_read() {
    for (mode, played) in [(Mode::Alone, false), (Mode::Played, true), (Mode::Together, false)] {
        let staging = staged(mode, played);
        assert!(staging.unread.is_empty(), "{:?}", staging.unread);
    }
}

#[test]
fn a_fight_is_told_from_start_to_end() {
    let said = told(&staged(Mode::Alone, false));
    assert!(said.contains("The fight begins!"));
    assert!(said.contains("fainted!"));
    assert!(said.contains("wins the fight!"));
    assert!(said.contains("enters the fight!"));
}

#[test]
fn two_on_the_field_have_little_bars() {
    let said = told(&staged(Mode::Together, false));
    assert!(said.contains("win the fight!"));
    assert!(said.contains("Bar"));
}

#[test]
fn who_stands_goes_back_where_it_stood() {
    let there = Vec2::new(100.0, 100.0);
    let (first, second) = (Who::Monster(1), Who::Monster(2));
    let mut staging = Staging::new(Ground::default(), 3, [(first, there), (second, there)].into());
    staging.at.insert(first, there + Vec2::new(52.0, 0.0));
    staging.at.insert(second, there + Vec2::new(52.0, 0.0));
    staging.flown.push(second);
    staging.go_home();
    assert_eq!(staging.at[&first], there);
    let said = told(&staging);
    assert!(said.contains("Monster(1)") && !said.contains("Monster(2)"), "{said}");
    assert!(matches!(staging.steps.back(), Some(Step::Wait(frames)) if *frames == 28.0));
}

#[test]
fn a_crowd_writes_lines_that_are_read() {
    let spot = |x: f32, y: f32| Vec2::new(x, y);
    let side = |first: u16, x: f32| (0..6).map(|at| (Who::Another { species: first, copy: at + 1 }, first, spot(x, 20.0 * at as f32))).collect::<Vec<_>>();
    let mut crowd = Crowd::new([side(1, 100.0), side(2, 200.0)], None, 3, Ground::default(), Arc::new(super::super::tests::rules()));
    let mut staging = Staging::new(Ground::default(), 3, Spots::new());
    while !crowd.over() {
        for rush in crowd.round() {
            staging.rush(&rush);
        }
    }
    assert!(staging.unread.is_empty(), "{:?}", staging.unread);
    assert!(!staging.steps.is_empty());
}
