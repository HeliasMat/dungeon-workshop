//! Crowds: gatherings, generations, joy running through them.

use engine::{CrowdForm, CrowdShape, Direction};

use crate::emit;
use crate::film::Film;
use crate::spots::{Goal, goal_of};
use crate::verbs::{Line, Names, Word};
use crate::words::{engine_word, plain};

pub const WORDS: &[Word] = &[
    Word::new(
        "crowd",
        &["species:word n:number [rest...]"],
        "crowd, fidget: crowd <species> <n> [where: a spot, x y] then, in any order, [radius 40] [tight|dense] [shape ring|flock|line|heap] [from east|west|north|south] [fast]",
    ).family("Presence").ds_close("a free actor each (45 a film, 24 a scene), round the place facing it; no fidgeting", "no fidgeting, and 24 at most a scene")
    .names(Names::NoOne)
    .does(crowd_said),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: crate::verbs::Group = crate::verbs::Group::new("crowds", WORDS);

/// The most a crowd or a gathering has: more would not fit on any screen.
const MOST_IN_A_CROWD: usize = 200;

fn crowd(film: &mut Film, line: &Line) {
    let (kind, count) = (plain(line.args.word("species").unwrap_or_default()), line.args.number("n").map_or(0, |count| count as usize));
    if count > MOST_IN_A_CROWD {
        film.wrong(line.number, format!("a crowd of {count}: at most {MOST_IN_A_CROWD}"));
        return;
    }
    if film.actor(&kind).is_none() {
        film.unknown_someone(line.number, &kind);
        return;
    }
    let names: Vec<String> = (0..count).map(|at| if at > 0 { format!("{kind}{}", at + 1) } else { kind.clone() }).collect();
    for name in &names {
        film.actor(name);
    }
    film.groups.insert(format!("{kind}s"), names.clone());
    let rest = line.args.words("rest").to_vec();
    let place_ends = rest.iter().position(|word| FORM_WORDS.contains(&plain(word).as_str())).unwrap_or(rest.len());
    let said = rest[..place_ends].to_vec();
    let at = match goal_of(film, &said, line.number) {
        Goal::Spot(at) => Some(at),
        Goal::Told => return,
        Goal::Nothing if said.is_empty() => None,
        Goal::Nothing => {
            film.wrong(line.number, format!("where? “{}”: a spot, or x y", said.join(" ")));
            return;
        }
    };
    let Some(form) = crowd_form(film, line.number, &rest[place_ends..]) else {
        return;
    };
    // (No more than anyone else does it come out of nowhere; from a side
    // said, it walks in from there.)
    let walking = form.from.is_some() || (film.staging.started && !film.staging.vision);
    match film.bodies(&names).filter(|who| !who.is_empty()) {
        Some(who) => film.beat(emit::crowd(who, at, walking, form)),
        None => film.not_a(names.join(" "), "Pokémon of the scene"),
    }
    for name in &names {
        film.on(name, |who| emit::fidget(who, true));
    }
}

/// The sides a crowd walks in from, by the film's words: the view's way.
const SIDES_IN: [(&str, Direction); 8] = [
    ("east", Direction::Right),
    ("right", Direction::Right),
    ("west", Direction::Left),
    ("left", Direction::Left),
    ("north", Direction::Up),
    ("up", Direction::Up),
    ("south", Direction::Down),
    ("down", Direction::Down),
];

/// The words that say a crowd's form, after where it stands.
const FORM_WORDS: [&str; 6] = ["radius", "tight", "dense", "shape", "from", "fast"];

/// How a crowd stands and comes, as its line says, in any order (`radius 40`,
/// `tight`, `shape flock`, `from east`, `fast`); none if it says it wrong
/// (told).
fn crowd_form(film: &mut Film, number: usize, words: &[String]) -> Option<CrowdForm> {
    let mut form = CrowdForm::default();
    let words: Vec<String> = words.iter().map(|word| plain(word)).collect();
    let mut at = 0;
    while let Some(word) = words.get(at) {
        let next = words.get(at + 1).map(String::as_str);
        // (A word alone, or with the one after it.)
        at += if matches!(word.as_str(), "tight" | "dense" | "fast") { 1 } else { 2 };
        let wrong = match (word.as_str(), next) {
            ("tight" | "dense", _) => {
                form.tight = true;
                continue;
            }
            ("fast", _) => {
                form.run = true;
                continue;
            }
            ("radius", Some(wide)) => match wide.parse::<f32>() {
                Ok(wide) if wide > 0.0 => {
                    form.wide = Some(wide);
                    continue;
                }

                _ => format!("radius {wide}: a number of pixels"),
            },
            ("shape", Some(shape)) => match CrowdShape::of(shape) {
                Some(shape) => {
                    form.shape = shape;
                    continue;
                }
                None => format!("what shape? “{shape}”: ring, flock, line or heap"),
            },
            ("from", Some(side)) => match SIDES_IN.iter().find(|(known, _)| *known == side) {
                Some((_, way)) => {
                    form.from = Some(*way);
                    continue;
                }
                None => format!("from where? “{side}”: east, west, north or south"),
            },
            (word, _) => format!("“{word}”? after where: radius 40, tight, shape ring|flock|line|heap, from east|west|north|south, fast"),
        };
        film.wrong(number, wrong);
        return None;
    }
    Some(form)
}

/// `run spearows to 300 200`, `walk spearows to clearing`: all of
/// a crowd goes there together, each keeping its place in it; the film
/// waits for them unless told. False: not a crowd.
pub fn crowd_goes(film: &mut Film, line: &Line, words: &[String]) -> bool {
    let Some(names) = words.first().and_then(|first| film.groups.get(&plain(first)).cloned()) else {
        return false;
    };
    let place: Vec<String> = words[1..].iter().filter(|word| plain(word) != "to").cloned().collect();
    let to = match goal_of(film, &place, line.number) {
        Goal::Spot(to) => to,
        Goal::Told => return true,
        Goal::Nothing => {
            film.wrong(line.number, format!("to where? “{}”: a spot, or x y", place.join(" ")));
            return true;
        }
    };
    match film.bodies(&names).filter(|who| !who.is_empty()) {
        Some(who) => film.beat(emit::flock(who, to, line.verb == "run")),
        None => film.not_a(names.join(" "), "Pokémon of the scene"),
    }
    if let Some(last) = names.last() {
        film.wait_for(last, line.wait);
    }
    true
}

/// The name a line gives a crowd, if it gives one.
fn crowd_named(line: &Line) -> Option<&str> {
    match line.verb.as_str() {
        "gathering" | "all_pokemon" | "gang" | "generation" => line.args.word("group"),
        _ => None,
    }
}

/// The ways the engine's lines are turned to: a crowd so named would turn
/// them into its members (`put krabby 300 90 left`).
const WAYS: [&str; 8] = ["up", "down", "left", "right", "upleft", "upright", "downleft", "downright"];

/// Whether a word is one the engine reads as its own: no crowd can be named
/// so (`gathering camera ...`, `gang say ...`, `gathering left ...`).
fn reserved(word: &str) -> bool {
    engine_word(word) || WAYS.contains(&word)
}

/// A crowd named as one of the engine's words: told, and nothing played.
fn name_taken(film: &mut Film, line: &Line) -> bool {
    let Some(name) = crowd_named(line).map(plain).filter(|name| reserved(name)) else {
        return false;
    };
    film.wrong(line.number, format!("“{name}” is already a word of the engine: give this crowd another name"));
    true
}

fn crowd_said(film: &mut Film, line: &Line) {
    if !name_taken(film, line) {
        crowd(film, line);
    }
}
