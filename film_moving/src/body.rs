//! What one does where it stands, or round it: looks, jumps, flies, waits,
//! walks a path of points, rounds a point, wanders; several in file.

use film_core::emit;
use film_core::film::Film;
use film_core::verbs::{Line, Word};
use film_core::words::{WAYS, plain};

pub const WORDS: &[Word] = &[
    Word::new("look", &["who:pokemon to:word"], "turn (a direction) or face (someone)").does(looked),
    Word::new("jump", &["who:word [rest...]"], "jump").does(jumped),
    Word::new(
        "wait",
        &["sound name:thing...", "music d:duration", "d:duration", "all", "*who", "name:word", "[text...]"],
        "wait 2s: t (at the film's pace); wait a b: waitfor each; wait all: waitall, and what plays beside; wait sound name: sound, waitsound (as long as it lasts); wait music 12s: waituntil music_clock",
    ).family("Time").ds_direct("Wait(frames) / WaitExecuteLives / all: each one moving, WaitExecutePerformer(0)").word_first()
    .does(waited)
    .until(),
    Word::new("wander", &["who:pokemon [n:number]"], "roams").does(roamed),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("body", WORDS);

/// `wait 2s`, `wait all`, `wait a b`; `wait wind`: what is
/// played beside the film under that name.
fn waited(film: &mut Film, line: &Line) {
    let args = &line.args;
    let those: Vec<String> = args.words("who").iter().map(|one| plain(one)).collect();
    if args.has("sound") {
        film_speech::heard::waited_sound(film, line);
    } else if args.has("music") {
        if film_core::groups::timing::music_known(film, line.number, "wait music") {
            film.beat(emit::wait_reached(engine::MUSIC_CLOCK, f64::from(args.frames("d").unwrap_or_default())));
        }
    } else if args.has("all") {
        film.beat(emit::wait_all());
        film_core::groups::timing::wait_meanwhile(film);
    } else if args.has("d") {
        let said = args.word("d").unwrap_or_default();
        if film_core::words::digits(said) {
            film.remark(line.number, format!("“wait {said}”: a duration without its unit, in frames; write {}", film_core::words::in_seconds_said(said)));
        }
        film.wait_paced(i64::from(args.frames("d").unwrap_or_default()));
    } else if !those.is_empty() {
        for one in &those {
            film.on(one, emit::wait_for);
        }
    } else if let Some(node) = args.word("name").map(str::to_lowercase).filter(|name| film.staging.nodes.contains_key(name)) {
        // (A node of the film's own: its tracks.)
        film.beat(emit::wait_track(engine::NodeRef::Named(node)));
    } else if let Some(thread) = args.word("name").and_then(|name| film.timing.named.get(&plain(name))).cloned() {
        film.beat(emit::wait_until(format!("{thread}_done"), emit::number(1)));
    } else {
        film.wrong(line.number, "wait 2s, wait a b, or wait all".to_string());
    }
}

/// `wander a [radius]`: its round kept on the ground, as wide as the ground leaves it.
fn roamed(film: &mut Film, line: &Line) {
    let (who, wide) = (film_core::verbs::who_said(line), line.args.word("n").unwrap_or("24"));
    match wide.parse::<f32>() {
        Ok(wide) => film.on(&who, |who| emit::roams(who, wide)),
        Err(_) => film.not_a(wide, "a width"),
    }
}

/// `look a b`, `look a right`: at someone, or that way.
fn looked(film: &mut Film, line: &Line) {
    let towards = plain(line.args.word("to").unwrap_or_default());
    let who = film_core::verbs::who_said(line);
    match (WAYS.contains(&towards.as_str()), engine::Direction::of(&towards)) {
        (true, Some(way)) => film.on(&who, |who| emit::turn(who, way)),
        (true, None) => film.not_a(&towards, "a direction"),
        (false, _) => film.face(&who, &towards),
    }
}

/// `jump a` (in the water, `jump a water x y`: fishing's).
fn jumped(film: &mut Film, line: &Line) {
    film.jump(&film_core::verbs::who_said(line), 8.0, 10.0);
}
