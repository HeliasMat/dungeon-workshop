//! Out of sight: behind something, crouched (`hide`), a look out and down
//! again (`peek`), into the ground and out of it (`burrow`, `pop_out`).
//! Behind is higher up the screen: who stands lower is drawn over him, and
//! a place's upper layer covers whoever stands behind it. Into the ground,
//! he is cut by its line (`BodyBeat::Sink`, of sand), step by step, earth
//! flung about in sharp clods.

use engine::{Colour, Direction, ParticleKind, Spot};

use film_core::emit;
use film_core::film::Film;
use film_core::spots::routes::{USUAL, found};
use film_core::spots::somewhere;
use film_core::verbs::{Line, Word};
use film_core::words::plain;

pub const WORDS: &[Word] = &[
    Word::new(
        "hide",
        &["who:pokemon off", "who:pokemon behind x:number y:number", "who:pokemon behind what:word"],
        "walks behind it (just over it on the screen, drawn under it; a place's upper layer covers him there), turns to us, crouches (Cringe, held); off: up again",
    )
    .does(hid),
    Word::new("peek", &["who:pokemon"], "up from where he hides, a look one way and the other, crouched again").does(peeked),
    Word::new(
        "burrow",
        &["who:pokemon [at] where:place", "who:pokemon"],
        "digs in where he stands (there, walking first): trembles, clods of earth flung, sink of sand step by step, gone under the ground",
    )
    .does(burrowed),
    Word::new(
        "pop_out",
        &["who:pokemon [at] where:place", "who:pokemon"],
        "out of the ground (there, if told): clods of earth, up through it step by step, a hop",
    )
    .does(popped_out),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("hiding", WORDS);

/// How far over the obstacle's feet one hides, on the screen.
const BEHIND: f32 = 6.0;
/// Digging: the frames of each step down (or up), the steps; clods each
/// step.
const STEP: i64 = 8;
const STEPS: usize = 4;
const CLODS: u16 = 6;
/// A hop out of the ground.
const HOP: (f32, f32) = (8.0, 12.0);

/// How tall someone is: all of him under the ground.
fn height_of(film: &mut Film, who: &str) -> f32 {
    film.actor(who).and_then(|code| film.body_of(&code)).map_or(28.0, |(_, high)| high as f32 + 4.0)
}

/// He crouches, as low as his sprite goes (its Cringe, else the nearest it has).
fn crouches(film: &mut Film, who: &str, number: usize) {
    let Some((group, _)) = film_acting::pose_named("cringe") else { return };
    if let Some(group) = film_acting::animation_had(film, who, group, number) {
        film.on(who, |one| emit::anim(one, group, false, false));
    }
}

/// `a hide behind b`, `a hide behind chest`, `a hide behind 300 140`:
/// there, turned to us, crouched; `a hide off`: up again.
fn hid(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    if line.args.has("off") {
        if !film.staging.hidden.remove(&who) {
            film.remark(line.number, format!("{who} hide off: he is not hiding ({who} hide behind <what>)"));
        }
        film.on(&who, |one| emit::pose(one, engine::Pose::Idle));
        film.wait_frames(10, line.wait);
        return;
    }
    let spot = match (line.args.number("x"), line.args.number("y")) {
        (Some(x), Some(y)) => film.point_at(x, y).map(Spot::Exact),
        _ => {
            let what = plain(line.args.word("what").unwrap_or_default());
            film.spot_by(&what, 0.0, -BEHIND)
        }
    };
    let Some(spot) = spot else { return };
    let at = found(film, &who, spot, &["exact"]);
    film.goes_through(&who, "walk", USUAL, vec![at]);
    film.on(&who, emit::wait_for);
    film.on(&who, |one| emit::turn(one, Direction::Down));
    crouches(film, &who, line.number);
    film.staging.hidden.insert(who);
    film.wait_frames(16, line.wait);
}

/// `a peek`: up from his hiding place, a look one way and the other, down again.
fn peeked(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    if !film.staging.hidden.contains(&who) {
        film.remark(line.number, format!("{who} peek: he is not hiding ({who} hide behind <what>)"));
    }
    film.on(&who, |one| emit::pose(one, engine::Pose::Idle));
    film.wait(12);
    for (way, frames) in [(Direction::DownLeft, 18), (Direction::DownRight, 22), (Direction::Down, 8)] {
        film.on(&who, |one| emit::turn(one, way));
        film.wait(frames);
    }
    crouches(film, &who, line.number);
    film.wait_frames(10, line.wait);
}

/// Clods of earth flung up about him: sharp drops, the earth's colour.
fn clods(film: &mut Film, who: &str, count: u16) {
    let earth = Some(emit::colour(Colour::Brown));
    film.on(who, |one| emit::particles(ParticleKind::Drops, engine::Cam::Who(one), count, earth));
}

/// `a burrow [at x y]`: (walking there first) he digs in, down through the
/// ground step by step, earth flung, gone under it.
fn burrowed(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    if line.args.has("where") {
        let spot = somewhere(film, line.args.words("where"), line.number);
        let at = found(film, &who, spot, &[]);
        film.goes_through(&who, "walk", USUAL, vec![at]);
        film.on(&who, emit::wait_for);
    }
    film.on(&who, |one| emit::turn(one, Direction::Down));
    film.on(&who, |one| emit::shadow(one, false));
    film.on(&who, |one| emit::tremble(one, (STEP * STEPS as i64) as f32, 1.0));
    film.sound("rocks");
    let to = height_of(film, &who);
    for step in 1..=STEPS {
        clods(film, &who, CLODS);
        film.on(&who, |one| emit::sink(one, to * step as f32 / STEPS as f32, true));
        film.wait(STEP);
    }
    film.on(&who, |one| emit::opacity(one, 0.0, 0.0));
    clods(film, &who, CLODS);
    film.staging.buried.insert(who);
    film.wait_frames(20, line.wait);
}

/// `a pop_out [at x y]`: from under the ground (there, if told), up
/// through it step by step, earth flung, a hop.
fn popped_out(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    film.staging.buried.remove(&who);
    let from = height_of(film, &who);
    film.on(&who, |one| emit::shadow(one, false));
    film.on(&who, |one| emit::sink(one, from, true));
    if line.args.has("where") {
        let spot = somewhere(film, line.args.words("where"), line.number);
        let at = found(film, &who, spot, &[]);
        film.on(&who, |one| emit::put_point(one, at.clone(), Direction::Down));
    }
    film.on(&who, |one| emit::opacity(one, 1.0, 0.0));
    film.sound("rocks");
    for step in (0..STEPS).rev() {
        clods(film, &who, CLODS);
        film.on(&who, |one| emit::sink(one, from * step as f32 / STEPS as f32, true));
        film.wait(STEP);
    }
    film.on(&who, |one| emit::shadow(one, true));
    film.jump(&who, HOP.0, HOP.1);
    film.sound("hop");
    film.wait_frames(HOP.1 as i64 + 10, line.wait);
}
