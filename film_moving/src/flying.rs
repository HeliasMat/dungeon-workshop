//! In the air: up off the ground (`fly`, `take_off`), across the place over
//! its walls (`fly to`), down again (`land`). A flier rises so high and
//! bobs there (`BodyBeat::Rise`, `BodyBeat::Float`), his shadow on the
//! ground under him, smaller the higher he is; he goes straight, over what
//! stands in his way (`flies`). Who rides him goes up with him. No dust:
//! its particles glow, and the films keep to sharp pixels.

use film_core::emit;
use film_core::film::Film;
use film_core::spots::routes::{found, pace_of};
use film_core::spots::somewhere;
use film_core::verbs::{Line, Word};

pub const WORDS: &[Word] = &[
    Word::new(
        "fly",
        &["who:pokemon [to] where:place [height h:number] [slow|slowly|fast] [in time:duration]", "who:pokemon [height h:number]"],
        "flies (straight, over the walls), rise so high (24 pixels; height h), float (a bob), wings; to a place: turned to it, goes there in the air",
    )
    .does(flown),
    Word::new("take_off", &["who:pokemon [height h:number]"], "fly, the same").does(flown),
    Word::new(
        "land",
        &["who:pokemon [at|to] where:place", "who:pokemon"],
        "flies there first, if told; float off, rise 0 (down), its landing (HitGround); on the ground again (lands)",
    )
    .does(landed),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("flying", WORDS);

/// How high one flies, when the line does not say; how long the way up
/// takes, and down.
const HEIGHT: f32 = 24.0;
const UP: f32 = 24.0;
const DOWN: f32 = 16.0;
/// His bob in the air: how high, how long.
const BOB: f32 = 3.0;
const BOB_FRAMES: f32 = 60.0;

/// He is up in the air, so high, if he was not there already: up from the
/// ground with a beat of wings (at once, before the scene shows).
pub(crate) fn up(film: &mut Film, who: &str, height: f32) {
    if film.staging.flying.get(who) == Some(&height) {
        return;
    }
    let frames = if film.staging.started { UP } else { 1.0 };
    film.staging.flying.insert(who.to_string(), height);
    film.on(who, |one| emit::flies(vec![one]));
    film.on(who, |one| emit::rise(one, height, frames));
    film.on(who, |one| emit::float(one, BOB, BOB_FRAMES));
    if film.staging.started {
        film.sound("wings");
        film.wait(frames as i64);
    }
}

/// `a fly [height h]`, `a fly to x y`: up in the air, and there.
fn flown(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let height = line.args.number("h").or_else(|| film.staging.flying.get(&who).copied()).unwrap_or(HEIGHT);
    up(film, &who, height);
    fly_to(film, line, &who, line.wait);
    if !line.args.has("where") {
        film.wait_frames(6, line.wait);
    }
}

/// He flies where the line says, if it says somewhere: turned to it,
/// straight there; waited, if told.
fn fly_to(film: &mut Film, line: &Line, who: &str, wait: bool) {
    if !line.args.has("where") {
        return;
    }
    let spot = somewhere(film, line.args.words("where"), line.number);
    let at = found(film, who, spot, &[]);
    film.on(who, |one| emit::look_at_point(one, at.clone()));
    film.goes_through(who, "walk", &pace_of(&line.args), vec![at]);
    film.wait_for(who, wait);
}

/// `a land [at x y]`: there first, if told; down to the ground, its landing
/// and a little dust; walking again.
fn landed(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    if line.args.has("where") {
        let height = film.staging.flying.get(&who).copied().unwrap_or(HEIGHT);
        up(film, &who, height);
        fly_to(film, line, &who, true);
    }
    if film.staging.flying.remove(&who).is_none() {
        film.remark(line.number, format!("{who} land: he is not flying ({who} fly)"));
    }
    film.on(&who, |one| emit::float(one, 0.0, BOB_FRAMES));
    film.on(&who, |one| emit::rise(one, 0.0, DOWN));
    film.wait(DOWN as i64);
    film.anim(&who, "HitGround", false);
    film.sound("hop");
    film.on(&who, |one| emit::lands(vec![one]));
    film.wait_frames(20, line.wait);
}
