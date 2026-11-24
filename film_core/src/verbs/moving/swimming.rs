//! In the water: swimming on it (`swim`), under it and up again (`dive`,
//! `surface`), out of it onto the shore (`get_out`). A swimmer is sunk in
//! the water (`BodyBeat::Sink`) as deep as a good part of his body, the
//! water's line waving on him, bobbing a little; he goes straight across it
//! (a place's water is ground no one walks on: he goes as those who fly,
//! `flies`), rings on the water where he sets off and stops and in his wake.
//! Who rides him sinks with him.

use engine::{Colour, ParticleKind};

use crate::emit;
use crate::film::Film;
use crate::spots::routes::{USUAL, found, pace_of};
use crate::spots::somewhere;
use crate::verbs::{Line, Word};

pub const WORDS: &[Word] = &[
    Word::new(
        "swim",
        &[
            "who:pokemon from x0:number y0:number [to] x:number y:number [slow|slowly|fast] [in time:duration]",
            "who:pokemon [to] where:place [slow|slowly|fast] [in time:duration]",
            "who:pokemon [here]",
        ],
        "in the water: sink (a good part of him under it, its line waving), float (a little bob), flies (straight across the water); to a place: turned to it, goes there, rings in his wake and where he stops; from x y: he comes in from there",
    )
    .does(swims)
    .puts(&["who"])
    .when("x0"),
    Word::new("dive", &["who:pokemon [at] where:place", "who:pokemon"], "jump, a splash (drops, rings), sinks under the water, opacity 0, bubbles; at: swims there first")
        .does(dives),
    Word::new("plunge", &["who:pokemon [at] where:place", "who:pokemon"], "dive, the same").does(dives),
    Word::new("surface", &["who:pokemon [at] where:place", "who:pokemon"], "up again from under the water (there, if told): rises through it, a splash (drops, rings)")
        .does(surfaces),
    Word::new(
        "get_out",
        &["who:pokemon [to] where:place", "who:pokemon"],
        "out of the water: a splash, hop onto the shore there (the nearest free ground; beside, if not told), walking again (lands)",
    )
    .does(gets_out),
];

/// Its words, a family of the language (`Lexicon`).
pub const FAMILY: crate::verbs::Family = crate::verbs::Family::new("swimming", WORDS);

/// His bob on the water: how high, how long.
const BOB: f32 = 2.0;
const BOB_FRAMES: f32 = 90.0;
/// How a swimmer goes, when the line does not say.
const PACE: &str = "x0.7";
/// His wake: rings a second.
const WAKE: f32 = 4.0;
/// A dive: the jump before, the frames of each step down (or up), the
/// steps.
const DIVE_JUMP: (f32, f32) = (6.0, 12.0);
const STEP: i64 = 5;
const STEPS: usize = 3;
/// A hop out of the water.
const HOP_HEIGHT: f32 = 24.0;
const HOP_FRAMES: f32 = 30.0;

/// How deep someone swims (`Film::depth_in_water`).
fn depth_of(film: &mut Film, who: &str) -> f32 {
    film.depth_in_water(who)
}

/// How tall someone is: all of him under the water.
fn height_of(film: &mut Film, who: &str) -> f32 {
    film.actor(who).and_then(|code| film.body_of(&code)).map_or(crate::names::DEPTH / crate::names::UNDER, |(_, high)| high as f32 + 4.0)
}

/// Water thrown up about someone: so many drops, so many rings.
fn splash(film: &mut Film, who: &str, drops: u16, rings: u16) {
    let white = Some(emit::colour(Colour::White));
    film.on(who, |one| emit::particles(ParticleKind::Drops, engine::Cam::Who(one), drops, white));
    if rings > 0 {
        film.on(who, |one| emit::particles(ParticleKind::Rings, engine::Cam::Who(one), rings, None));
    }
}

/// He is in the water, if he was not: sunk in it, bobbing, across it as
/// those who fly; a ring or two where he is.
pub(crate) fn in_the_water(film: &mut Film, who: &str) {
    if !film.staging.swimming.insert(who.to_string()) {
        return;
    }
    let depth = depth_of(film, who);
    film.on(who, |one| emit::flies(vec![one]));
    film.on(who, |one| emit::sink(one, depth, false));
    film.on(who, |one| emit::float(one, BOB, BOB_FRAMES));
    film.on(who, |one| emit::particles(ParticleKind::Rings, engine::Cam::Who(one), 2, None));
}

/// `a swim [here]`, `a swim to x y [in 2s]`, `a swim from x y to x y`.
fn swims(film: &mut Film, line: &Line) {
    let who = super::super::who_said(line);
    let args = &line.args;
    if let (Some(x), Some(y)) = (args.number("x0"), args.number("y0")) {
        film.put_at(&who, x, y);
    }
    in_the_water(film, &who);
    swim_to(film, line, &who, line.wait);
}

/// He swims where the line says, if it says somewhere; waited, if told.
fn swim_to(film: &mut Film, line: &Line, who: &str, wait: bool) {
    let args = &line.args;
    let who = who.to_string();
    let at = match (args.number("x"), args.number("y")) {
        (Some(x), Some(y)) => film.point_at(x, y),
        _ if args.has("where") => {
            let spot = somewhere(film, args.words("where"), line.number);
            Some(found(film, &who, spot, &[]))
        }
        _ => None,
    };
    let Some(at) = at else {
        film.wait_frames(10, wait);
        return;
    };
    let pace = match pace_of(args) {
        pace if pace == USUAL => PACE.to_string(),
        pace => pace,
    };
    film.on(&who, |one| emit::look_at_point(one, at.clone()));
    let white = Some(emit::colour(Colour::White));
    film.on(&who, |one| emit::emitter(ParticleKind::Rings, one, WAKE, white, None));
    film.goes_through(&who, "walk", &pace, vec![at]);
    if wait {
        film.on(&who, emit::wait_for);
        film.on(&who, emit::emitter_off);
        film.on(&who, |one| emit::particles(ParticleKind::Rings, engine::Cam::Who(one), 2, None));
    } else {
        film.wait_for(&who, false);
    }
}

/// `a dive [at x y]`: a little jump, a splash, down through the water step
/// by step, gone under it; bubbles where he was.
fn dives(film: &mut Film, line: &Line) {
    let who = super::super::who_said(line);
    in_the_water(film, &who);
    if line.args.has("where") {
        swim_to(film, line, &who, true);
    }
    film.on(&who, emit::emitter_off);
    film.jump(&who, DIVE_JUMP.0, DIVE_JUMP.1);
    film.wait(DIVE_JUMP.1 as i64);
    film.sound("splash");
    splash(film, &who, 16, 3);
    let (from, to) = (depth_of(film, &who), height_of(film, &who));
    for step in 1..=STEPS {
        let depth = from + (to - from) * step as f32 / STEPS as f32;
        film.on(&who, |one| emit::sink(one, depth, false));
        film.wait(STEP);
    }
    film.on(&who, |one| emit::opacity(one, 0.0, 0.0));
    film.on(&who, |one| emit::particles(ParticleKind::Bubbles, engine::Cam::Who(one), 6, None));
    film.staging.under.insert(who);
    film.wait_frames(20, line.wait);
}

/// `a surface [at x y]`: from under the water (there, if told), up through
/// it step by step, a splash.
fn surfaces(film: &mut Film, line: &Line) {
    let who = super::super::who_said(line);
    if !film.staging.under.remove(&who) {
        film.remark(line.number, format!("{who} surface: he is not under the water ({who} dive)"));
    }
    in_the_water(film, &who);
    let (from, to) = (height_of(film, &who), depth_of(film, &who));
    film.on(&who, |one| emit::sink(one, from, false));
    if line.args.has("where") {
        let spot = somewhere(film, line.args.words("where"), line.number);
        let at = found(film, &who, spot, &[]);
        film.on(&who, |one| emit::put_point(one, at.clone(), engine::Direction::Down));
    }
    film.on(&who, |one| emit::particles(ParticleKind::Bubbles, engine::Cam::Who(one), 6, None));
    film.on(&who, |one| emit::opacity(one, 1.0, 0.0));
    for step in 1..=STEPS {
        let depth = from + (to - from) * step as f32 / STEPS as f32;
        film.on(&who, |one| emit::sink(one, depth, false));
        film.wait(STEP);
    }
    film.sound("splash");
    splash(film, &who, 12, 2);
    film.jump(&who, DIVE_JUMP.0, DIVE_JUMP.1);
    film.wait_frames(DIVE_JUMP.1 as i64 + 10, line.wait);
}

/// `a get_out [to x y]`: out of the water with a splash, a hop onto the
/// shore there, walking again.
fn gets_out(film: &mut Film, line: &Line) {
    let who = super::super::who_said(line);
    if !film.staging.swimming.remove(&who) {
        film.remark(line.number, format!("{who} get_out: he is not in the water ({who} swim)"));
    }
    film.staging.under.remove(&who);
    film.on(&who, emit::emitter_off);
    film.on(&who, |one| emit::float(one, 0.0, BOB_FRAMES));
    // (On the ground again: the place's walls hold him, the shore found for
    // him is free ground.)
    film.on(&who, |one| emit::lands(vec![one]));
    let spot = match line.args.has("where") {
        true => somewhere(film, line.args.words("where"), line.number),
        false => film.spot_beside(&who).unwrap_or_else(|| film.middle_spot()),
    };
    let at = found(film, &who, spot, &[]);
    film.sound("splash");
    splash(film, &who, 14, 2);
    film.on(&who, |one| emit::hop_point(one, at.clone(), HOP_HEIGHT, HOP_FRAMES));
    // (Out of the water once he is up in the air.)
    film.on(&who, |one| emit::after(6.0, 0.0, emit::sink(one, 0.0, false)));
    film.wait(HOP_FRAMES as i64);
    film.sound("hop");
    film.wait_frames(10, line.wait);
}
