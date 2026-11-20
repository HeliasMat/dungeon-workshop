//! Going somewhere: the place found when it plays, the way walked or run
//! there, how fast (`slowly`, `fast`, `in 2s`, `arrives 3s`).

use crate::emit;
use crate::film::Film;
use crate::words::WALK;
use engine::{Point, Spot};

/// A pace as the engine reads it (see `Pace`); as it usually is.
pub type Pace = String;
pub const USUAL: &str = "-";

/// How fast a line says to go (`slowly`, `fast`, `in 2s`, `arrives 3s`),
/// read in its values.
pub fn pace_of(args: &crate::grammar::Bound) -> Pace {
    if args.has("slowly") || args.has("slow") {
        return "x0.5".to_string();
    }
    if args.has("fast") || args.has("brisk") {
        return "x1.6".to_string();
    }
    match args.frames("time").or(args.frames("arrival")) {
        Some(frames) => format!("s{}", frames as f32 / 60.0),
        None => USUAL.to_string(),
    }
}

/// How a verb goes: `run` runs, the others walk.
pub fn gait(verb: &str) -> &'static str {
    if verb == "run" { "run" } else { "walk" }
}

/// A place found when played, under a name of its own (its two numbers:
/// `<name>_x`, `<name>_y`).
pub fn find(film: &mut Film, who: &str, spot: Spot, how: &[&str]) -> String {
    let var = film.found_var();
    let someone = if who == "-" { None } else { film.someone(who) };
    film.beat(emit::find(var.clone(), someone, spot, how.contains(&"apart"), how.contains(&"exact")));
    var
}

/// A place found when played: the point it gives.
pub fn found(film: &mut Film, who: &str, spot: Spot, how: &[&str]) -> Point {
    emit::found_at(&find(film, who, spot, how))
}

/// Walks or runs there, round the walls and those standing there.
pub fn go(film: &mut Film, who: &str, spot: Spot, verb: &str, wait: bool, pace: &str, apart: bool) {
    let at = found(film, who, spot, if apart { &["apart"] } else { &[] });
    film.goes_through(who, gait(verb), pace, vec![at]);
    arrived(film, who, wait, pace);
}

/// What comes once someone is on its way: the film waits, and it walks as
/// usual from then on.
pub fn arrived(film: &mut Film, who: &str, wait: bool, pace: &str) {
    if wait {
        film.on(who, emit::wait_for);
        if pace != USUAL {
            film.on(who, |who| emit::speed(who, WALK));
        }
    }
}

/// Out of nowhere, but in a puff of smoke, and with a sound.
pub fn puff(film: &mut Film, at: Point) {
    film.beat(emit::particles(engine::ParticleKind::Smoke, engine::Cam::At(at), 30, None));
    film.sound("shock");
    film.wait(8);
}
