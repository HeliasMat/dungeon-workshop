//! Going about: being put, walking, running, coming in, going out, in a
//! formation or by a way; a spot of the film's named. Where they go is read
//! in `spots`, and found on the stage.
//!
//! - `body`: what one does where it stands (looks, jumps, flies, rounds).

use super::{Line, Word};
use crate::emit;
use crate::film::Film;
use crate::names::places::SIDES;
use crate::spots::formations::{formation, shape_of};
use crate::spots::routes::{USUAL, arrived, found, gait, go, pace_of};
use crate::spots::{Aim, Goal, goal_of, point_said, somewhere, xy};
use crate::words::{SPOTS, plain};

pub mod body;

pub const WORDS: &[Word] = &[
    Word::new(
        "put",
        &[
            "who:pokemon [where:place] facing way:word",
            "who:pokemon [where:place]",
            "rest...",
        ],
        "find, put (before the scene) or arrives (in the middle of the scene); facing: turned that way, or to someone",
    )
    .does(placed)
    .puts(&["who"]),
    Word::new(
        "walk",
        &[
            "who:pokemon [to] where:place [slow|brisk|slowly|fast] backward [in time:duration]",
            "who:pokemon [to] where:place [slow|brisk|slowly|fast] [in time:duration] [arrives arrival:duration]",
            "who:pokemon [to] where:place [slow|brisk|slowly|fast] via path...",
            "who:pokemon via path... [slow|brisk|slowly|fast] [in time:duration] [arrives arrival:duration]",
            "rest...",
        ],
        "find, goes; approach (to someone); leaves (to offleft); slow, brisk: its pace; backward: its back to where it goes, stepping; via a b: by those places first",
    )
    .does(walked),
    Word::new(
        "run",
        &[
            "who:pokemon [to] where:place [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "who:pokemon via path... [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "rest...",
        ],
        "find, goes; chase (to someone)",
    )
    .does(walked),
    Word::new(
        "enter",
        &[
            "row|column|triangle *who [where:place] [look to:word] [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "face to face *who [where:place] [look to:word] [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "*who from side:word [to where:place] [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "*who [via side:word] [to where:place] [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "rest...",
        ],
        "find, arrives enter [via:<entrance>]",
    )
    .does(came_in)
    .arrives(&["who"]),
    Word::new("exit", &["*who [via side:word]"], "leaves, waitfor, gone").does(goes_out),
    Word::new(
        "appear",
        &[
            "row|column|triangle *who [where:place] [look to:word] [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "face to face *who [where:place] [look to:word] [slowly|fast] [in time:duration] [arrives arrival:duration]",
            "who:pokemon [where:place] smoke|fade|flash|vision|light",
            "who:pokemon [where:place]",
            "rest...",
        ],
        "find, put; in the game's smoke (by default), a fade, a flash, a vision, or the light that gathers into a ring (light)",
    )
    .does(appeared)
    .arrives(&["who"]),
    Word::new(
        "face",
        &["to face *who [where:place] [look to:word] [slowly|fast] [in time:duration] [arrives arrival:duration]", "who:pokemon other:pokemon"],
        "face to face: find, put or arrives, turn",
    )
    .maybe(shaped)
    .puts(&["who"])
    .when("face"),
    Word::new(
        "form",
        &["row|column|ring|lineup *who [to] [where:place] [walk|run] [look looks:word] [slowly|fast] [in time:duration] [arrives arrival:duration]"],
        "find, put or arrives, turn: side by side (row), one behind the other (column), round the place facing it (ring), side by side facing us (lineup); put there before the scene shows, walked to after; walk, run: walked or run to it, even before",
    )
    .maybe(shaped)
    .puts(&["who"]),
    Word::new("spot", &["name:word where:place"], "spot throne 400 200: setspot, a spot of the film's, by its name").maybe(spot_said),
];

/// The frames someone walks up to another in, when no pace is said.
const APPROACH: i32 = 40;

/// One a vision or a memory brings on: gone when it ends (see `pictures`).
fn dreamt(film: &mut Film, who: &str) {
    if film.staging.vision && film.staging.brought.iter().any(|one| one == who) && !film.staging.dreamt.iter().any(|one| one == who) {
        film.staging.dreamt.push(who.to_string());
    }
}

fn place(film: &mut Film, line: &Line, who: &str) {
    dreamt(film, who);
    let spot = somewhere(film, line.args.words("where"), line.number);
    let at = found(film, who, spot, &[]);
    // (No one comes out of nowhere in the middle of a scene: they walk in by
    // the nearest edge, but in a vision, which a flash opens; who is there
    // already walks to it, not to jump before our eyes.)
    // (Where, found as it plays.)
    if film.staging.started && !film.staging.vision {
        film.arrives(who, "walk", USUAL, at, true, None);
    } else {
        film.put_point_at(who, at);
    }
}

/// Out of nowhere, but as the game shows it: in its puff of smoke (by
/// default), fading in, in a white flash, in a vision, or in the light
/// that gathers into a ring (hidden there until it does).
fn appears(film: &mut Film, line: &Line, who: &str) {
    dreamt(film, who);
    let spot = somewhere(film, line.args.words("where"), line.number);
    let at = found(film, who, spot, &["apart"]);
    film.put_point_at(who, at);
    super::acting::shows(film, who, super::acting::manner(&line.args), true);
}

/// Out of the place by that way, in file, and gone; the film waits for the
/// last unless told.
fn leave(film: &mut Film, those: &[String], way: &str, wait: bool) {
    for one in those {
        film.on(one, |who| emit::leaves(who, emit::exit(way), false));
    }
    if wait {
        for one in those {
            film.on(one, emit::wait_for);
            film.on(one, emit::gone);
        }
    }
}

fn walks(film: &mut Film, line: &Line, who: &str) {
    let pace = pace_of(&line.args);
    let place = line.args.words("where").to_vec();
    let goal = place.last().cloned().unwrap_or_default();
    let verb = line.verb.as_str();
    // (Off the place: out by the nearest way on that side, not into a wall.)
    if let Some(side) = plain(&goal).strip_prefix("off").filter(|side| SIDES.contains(side)) {
        leave(film, &[who.to_string()], side, line.wait);
        return;
    }
    if line.args.has("backward") {
        let spot = match goal_of(film, &place, line.number) {
            Goal::Spot(spot) => Some(spot),
            Goal::Nothing => film.spot_beside(&goal),
            Goal::Told => None,
        };
        if let Some(spot) = spot {
            super::acting::backward(film, line, who, spot);
        }
        return;
    }
    match goal_of(film, &place, line.number) {
        Goal::Spot(spot) => go(film, who, spot, verb, line.wait, &pace, true),
        Goal::Told => {}
        Goal::Nothing if film.actor(&goal).is_none() => {
            let names: Vec<&str> = SPOTS.iter().map(|(name, _)| *name).collect();
            film.wrong(line.number, format!("to where? “{goal}” (a spot: {}; or someone)", names.join(", ")));
        }
        Goal::Nothing if pace != USUAL => {
            if let Some(spot) = film.spot_beside(&goal) {
                go(film, who, spot, verb, line.wait, &pace, false);
            }
        }
        Goal::Nothing => {
            let other = plain(&goal);
            match (verb, film.other(&other)) {
                ("run", Some(other)) => film.on(who, |who| emit::chase(who, other)),
                ("run", None) => film.not_a(&other, "a Pokémon of the scene"),
                (_, Some(other)) => film.on(who, |who| emit::approach(who, other, APPROACH as f32)),
                (_, None) => film.not_a(&other, "a Pokémon of the scene"),
            }
            film.wait_for(who, line.wait);
        }
    }
}

/// `walk a via 100 200, throne, to b`: by these places in turn.
fn go_by(film: &mut Film, line: &Line, who: &str) {
    let pace = pace_of(&line.args);
    let mut points = Vec::new();
    let mut ways: Vec<Vec<String>> = Vec::new();
    for said in line.args.list("path") {
        // (`via rock tree`: each a place of its own, said by its name.)
        let named = |word: &String| film.actor(word).is_some() || super::spot_named(film, word).is_some() || SPOTS.iter().any(|(name, _)| *name == plain(word));
        if said.len() > 1 && said.iter().all(named) {
            ways.extend(said.into_iter().map(|one| vec![one]));
        } else {
            ways.push(said);
        }
    }
    if line.args.has("where") {
        ways.push(line.args.words("where").to_vec());
    }
    for mut said in ways {
        if said.first().is_some_and(|word| plain(word) == "to") {
            said.remove(0);
        }
        let Some(last) = said.last().cloned() else {
            continue;
        };
        let spot = match goal_of(film, &said, line.number) {
            Goal::Spot(spot) => Some(spot),
            Goal::Nothing if film.actor(&last).is_some() => film.spot_beside(&last),
            _ => None,
        };
        match spot {
            Some(spot) => points.push(found(film, who, spot, &[])),
            None => film.wrong(line.number, format!("via where? “{}”: via 100 200, tile 12 7, a spot, to b", said.join(" "))),
        }
    }
    if points.is_empty() {
        return;
    }
    // (Points found as it plays.)
    film.goes_through(who, gait(&line.verb), &pace, points);
    arrived(film, who, line.wait, &pace);
}

/// `enter a [b c] [via south] [to center]`: in by that way, from off the
/// place, one after the other; the film waits for the last.
fn comes_in(film: &mut Film, line: &Line) {
    let pace = pace_of(&line.args);
    let by = line.args.word("side").and_then(|way| film.way_said(way, line.number));
    let spot = somewhere(film, line.args.words("where"), line.number);
    let those = line.args.words("who").to_vec();
    for (at, who) in those.iter().enumerate() {
        dreamt(film, who);
        let to = found(film, who, spot.clone(), &["apart"]);
        // (Where, found as it plays.)
        film.arrives(who, "walk", &pace, to, false, by.as_ref().map(|way| way.to_string()));
        if at + 1 < those.len() {
            film.wait(20);
        }
    }
    if let Some(last) = those.last() {
        film.wait_for(last, line.wait);
    }
}

/// `exit a [b c] via left`, `exit a via east`: out by the nearest way on
/// that side, or by that one; all of them in file; gone once out.
fn goes_out(film: &mut Film, line: &Line) {
    let said = plain(line.args.word("side").unwrap_or("left"));
    let way = match said.as_str() {
        side if side.contains("right") => Some("right".to_string()),
        side if side.contains("left") => Some("left".to_string()),
        _ => film.way_said(&said, line.number),
    };
    if let Some(way) = way {
        leave(film, line.args.words("who"), &way, line.wait);
    }
}

/// `spot throne 400 200`, `spot throne tile 12 7`, `spot throne front_of a`.
fn names_a_spot(film: &mut Film, line: &Line) {
    let name = plain(line.args.word("name").unwrap_or_default());
    if SPOTS.iter().any(|(word, _)| *word == name) {
        film.wrong(line.number, format!("“{name}” is already a spot: choose another name"));
        return;
    }
    match point_said(film, line.args.words("where"), line.number) {
        Some(Aim::At(x, y)) => film.beat(emit::name_spot(name.clone(), engine::Spot::Exact(xy(x, y)))),
        Some(Aim::Found(spot)) => film.beat(emit::name_spot(name.clone(), spot)),
        None => {
            film.wrong(line.number, format!("where is that place? “{}”: spot throne 400 200, spot throne tile 12 7", line.text));
            return;
        }
    }
    film.spots.insert(name);
}

/// A line no form of the family fits: what is wrong with it, told.
fn told_wrong(film: &mut Film, line: &Line) {
    let said = line.args.words("rest");
    if film.who(&line.text, said, line.number).is_some() {
        let place: Vec<String> = said[1..].iter().filter(|word| plain(word) != "to").cloned().collect();
        let before = film.problems.len();
        somewhere(film, &place, line.number);
        if film.problems.len() == before {
            film.wrong(line.number, format!("“{}”: {} a to b, {} a 300 200", line.text, line.verb, line.verb));
        }
    }
}

/// `row a b c [where]`, `column`, `triangle`, `face to face a b`: they walk
/// there in that shape, or stand there before the scene shows.
fn shaped(film: &mut Film, line: &Line) -> bool {
    if shape_of(line).is_none() {
        return false;
    }
    let verb = match (line.args.has("run"), line.args.has("walk") || film.staging.started) {
        (true, _) => "run",
        (false, true) => "walk",
        (false, false) => "put",
    };
    formation(film, line, verb);
    true
}

/// A way of going said as a shape (`put row a b c`), or wrong: played so,
/// and true; else it goes its own way.
fn shaped_or_wrong(film: &mut Film, line: &Line) -> bool {
    if line.verb != "run_in" && shape_of(line).is_some() {
        formation(film, line, &line.verb);
    } else if line.args.has("rest") {
        told_wrong(film, line);
    } else {
        return false;
    }
    true
}

fn placed(film: &mut Film, line: &Line) {
    if !shaped_or_wrong(film, line) {
        let who = super::who_said(line);
        place(film, line, &who);
        if let Some(way) = line.args.word("way").map(plain) {
            // (Walked in mid-scene: turned once there.)
            if film.staging.started && !film.staging.vision {
                film.on(&who, emit::wait_for);
            }
            // (In the scene's head, facing someone: once all of it is put.)
            if !film.staging.started && film.actor(&way).is_some() {
                film.staging.facing_later.push((who, way));
                return;
            }
            super::acting::faced(film, &who, &way);
        }
    }
}

fn appeared(film: &mut Film, line: &Line) {
    if !shaped_or_wrong(film, line) {
        appears(film, line, &super::who_said(line));
    }
}

/// `walk a to b`, `run a via x y, z`.
fn walked(film: &mut Film, line: &Line) {
    // (A crowd's name: all of it, keeping its form.)
    if line.args.has("rest") && super::crowds::crowd_goes(film, line, line.args.words("rest")) {
        return;
    }
    if shaped_or_wrong(film, line) {
        return;
    }
    let who = super::who_said(line);
    if line.args.has("via") { go_by(film, line, &who) } else { walks(film, line, &who) }
}

fn came_in(film: &mut Film, line: &Line) {
    if !shaped_or_wrong(film, line) {
        comes_in(film, line);
    }
}

/// `spot throne 400 200`: a spot of the film's; `spot 3 ...`, the engine's.
fn spot_said(film: &mut Film, line: &Line) -> bool {
    if line.args.word("name").unwrap_or("0").starts_with(|letter: char| letter.is_ascii_digit()) {
        return false;
    }
    names_a_spot(film, line);
    true
}
