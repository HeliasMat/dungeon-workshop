//! The engine as whoever uses it sees it: a scene's lines (fields between tabs, as
//! the language gives them), played frame by frame, with nothing shown. No game here.

use engine::{Call, Director, Stage, Vocabulary, read_events, read_scene};
use glam::Vec2;

const SCENE: &str = "event\t0\tE\t1\t1\nscene\t1\tici\nput\tm25\t100\t100\tdown\nwalk\tm25\t130\t100\nwaitfor\tm25\nsay\tm25\tjoyous\tHi!\nsaid\n";

fn played() -> (Vec<Call>, Vec2) {
    let script = read_scene(SCENE, None, &Vocabulary::NONE);
    assert!(script.problems.is_empty(), "{:?}", script.problems);
    let mut director = Director::new(read_events(script.lines.into_iter().map(|(_, line)| line), &Vocabulary::NONE), 0, &Vocabulary::NONE);
    let (mut stage, mut calls, mut focus) = (Stage::dark(), Vec::new(), None);
    for _ in 0..600 {
        calls.extend(director.tick(&mut stage, 1.0, focus.unwrap_or_default()));
        focus = stage.advance(1.0, 0.0, focus);
        if director.reading() {
            director.read();
        }
    }
    let at = stage.cast.values().next().map_or(Vec2::ZERO, |actor| actor.position);
    (calls, at)
}

#[test]
fn a_scene_plays_to_its_end_saying_what_is_to_be_shown() {
    let (calls, at) = played();
    assert!(calls.iter().any(|call| matches!(call, Call::Say { text, .. } if text.contains("Hi"))), "{calls:?}");
    assert!(calls.contains(&Call::Over), "{calls:?}");
    assert!(at.distance(Vec2::new(130.0, 100.0)) < 1.0 || at == Vec2::ZERO, "{at}");
}

#[test]
fn a_scene_plays_alike_each_time() {
    assert_eq!(played(), played());
}
