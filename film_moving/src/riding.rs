//! Riding and carrying: on someone's back or head and off again (`ride`,
//! `get_off`), someone lifted over one's head and put down (`carry`,
//! `put_down`). The engine keeps the rider on its mount (`BodyBeat::Ride`):
//! he goes where it goes, turns with it as it goes, rises and sinks with it.
//! How high he sits is the mount's own: the game's points of it (its middle
//! for a back, its head for a head), so a rider sits on a Lapras as on a
//! Pikachu.

use film_core::emit;
use film_core::film::Film;
use film_core::spots::routes::found;
use film_core::spots::{Goal, goal_of};
use film_core::verbs::{Line, Word};
use film_core::words::plain;

pub const WORDS: &[Word] = &[
    Word::new(
        "ride",
        &[
            "who:pokemon on mount:pokemon [back|head] [high h:number] [behind b:number]",
            "who:pokemon mount:pokemon [h:number] [b:number]",
        ],
        "face, throw onto it, ride: on its back (as high as its middle, by the game's points) or its head; he follows his mount, turns with it as it goes, rises and sinks with it (its back or head on each frame, by the game's points); high, behind: so many pixels up, behind its middle; a heavy rider sinks his mount 1 to 3 pixels (the game's weights), one much heavier is remarked",
    ).family("Movement").ds_close("a run to the mount (no arc), MoveHeight up to its back, drawn in front of it; given each of its moves (the same path and pace, SlidePositionMark), turned as it goes", "no arc onto it; he sits over its middle, not its back's point, and doesn't rise and sink with it; the mount doesn't sink under a heavy one")
    .maybe(rode),
    Word::new(
        "get_off",
        &["who:pokemon [to] x:number y:number", "who:pokemon [to] where:place", "who:pokemon"],
        "ride off, hop down: there, or beside his mount",
    ).family("Movement").ds_close("MoveHeight down, a run to where he hops (no arc)", "no arc: a run down to the place")
    .does(got_off),
    Word::new(
        "dismount",
        &["who:pokemon [to] x:number y:number", "who:pokemon [to] where:place", "who:pokemon"],
        "get_off, the same",
    )
    .does(got_off),
    Word::new(
        "carry",
        &["who:pokemon what:pokemon"],
        "face, throw onto him, ride over his head (as high as his head, by the game's points): carried wherever he goes, turned with him",
    ).family("Body").ds_close("a run to the carrier (no arc), MoveHeight up over its head, drawn in front; given each of the carrier's moves", "no arc onto him; he sits over the carrier's middle, not its head's point")
    .does(carried),
    Word::new(
        "put_down",
        &["who:pokemon what:pokemon [to] x:number y:number", "who:pokemon what:pokemon [to] where:place", "who:pokemon what:pokemon"],
        "ride off, hop down: there, or before him",
    ).family("Body").ds_close("MoveHeight down, a run to where he hops (no arc)", "no arc: a run down to the place")
    .does(put_down),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("riding", WORDS);

/// A hop on or off: how high, how many frames.
const HOP_HEIGHT: f32 = 28.0;
const HOP_FRAMES: f32 = 30.0;
/// How high one sits on a back, how far behind its middle, when the game
/// has no points for the mount.
const BACK: f32 = 10.0;
/// How far above the top of a head one sits (the head's point is its
/// middle).
const ON_HEAD: f32 = 4.0;
const OVER_HEAD: f32 = 4.0;
/// The game's points: the head, the left hand, the right hand, the middle.
const HEAD: usize = 0;
const MIDDLE: usize = 3;

/// How high and how far behind its middle one sits on that mount: on its
/// back, as high as its middle, a quarter of its span behind; on its head,
/// a little over it.
fn seat(film: &mut Film, mount: &str, head: bool) -> (f32, f32) {
    let Some(points) = film.actor(mount).and_then(|code| film.points_of(&code)) else {
        return (BACK, 0.0);
    };
    if head {
        return ((-points[HEAD].1) as f32 + ON_HEAD, 0.0);
    }
    let span = (points[2].0 - points[1].0).abs() as f32;
    (((-points[MIDDLE].1) as f32).max(4.0), (span / 4.0).round())
}

/// `a ride on b [back|head] [high 12] [behind 6]`, `ride a b`: he hops onto
/// it and stays there. Else (`ride a 20 0.5s`) the engine's.
fn rode(film: &mut Film, line: &Line) -> bool {
    let Some(mount) = line.args.word("mount").map(plain) else {
        return false;
    };
    let who = film_core::verbs::who_said(line);
    let (high, behind) = seat(film, &mount, line.args.has("head"));
    let high = line.args.number("h").unwrap_or(high);
    let behind = line.args.number("b").unwrap_or(behind);
    gets_on(film, &who, &mount, high, behind, line.number);
    film.wait_frames(10, line.wait);
    true
}

/// How many pixels a mount sinks under a rider so heavy, against its own
/// weight: none under one a quarter of it, then 1, 2, 3 (as heavy as it).
const SINKS: [(f32, f32); 3] = [(0.25, 1.0), (0.6, 2.0), (1.0, 3.0)];
/// A rider so many times heavier than his mount: remarked.
const CRUSHING: f32 = 2.0;

/// A mount under a rider: so many pixels lower, with him, by their weights
/// (the game's tables); one much heavier than it, remarked.
fn weighs_on(film: &mut Film, who: &str, mount: &str, number: usize) {
    let weight = |film: &mut Film, one: &str| film.actor(one).and_then(|code| film.weight_of(&code)).map(f32::from);
    let (Some(rider), Some(under)) = (weight(film, who), weight(film, mount).filter(|weight| *weight > 0.0)) else {
        return;
    };
    let ratio = rider / under;
    if ratio >= CRUSHING {
        film.remark(number, format!("{who} is much heavier than {mount} ({ratio:.1} times): {mount} could not bear him"));
    }
    let Some(&(_, sinks)) = SINKS.iter().rev().find(|(least, _)| ratio >= *least) else {
        return;
    };
    let Some(node) = film.one(mount) else {
        return;
    };
    film.beat(emit::set_prop(engine::NodeRef::Who(node), engine::props::PropId::Height, engine::props::Given::Plus(engine::props::Value::Number(-sinks))));
    *film.staging.sunk.entry(mount.to_string()).or_default() += sinks;
    film.staging.sunk.insert(format!("{who}>"), sinks);
}

/// A rider off: his mount as high as it was before him.
fn eased(film: &mut Film, who: &str, mount: &str) {
    let Some(sinks) = film.staging.sunk.remove(&format!("{who}>")) else {
        return;
    };
    if let Some(sunk) = film.staging.sunk.get_mut(mount) {
        *sunk -= sinks;
    }
    if let Some(node) = film.one(mount) {
        film.beat(emit::set_prop(engine::NodeRef::Who(node), engine::props::PropId::Height, engine::props::Given::Plus(engine::props::Value::Number(sinks))));
    }
}

/// He hops onto the other, and rides there.
fn gets_on(film: &mut Film, who: &str, mount: &str, high: f32, behind: f32, number: usize) {
    let Some(under) = film.one(mount) else {
        return;
    };
    film.face(who, mount);
    film.on(who, |who| emit::throw(who, under, HOP_HEIGHT, HOP_FRAMES));
    film.sound("hop");
    film.wait(HOP_FRAMES as i64);
    film.on(who, |who| emit::ride(who, Some(under), high, behind));
    weighs_on(film, who, mount, number);
    film.sound("click");
    film.staging.mounts.insert(who.to_string(), mount.to_string());
}

/// Off whom he rides, he hops to that place: the one the line tells, or
/// beside (`beside`: before him who carried him).
fn hops_down(film: &mut Film, line: &Line, who: &str, beside: Option<String>) {
    film.on(who, |who| emit::ride(who, None, 0.0, 0.0));
    let args = &line.args;
    if let (Some(x), Some(y)) = (args.number("x"), args.number("y")) {
        film.on(who, |who| emit::hop(who, x, y, HOP_HEIGHT, HOP_FRAMES));
    } else {
        let spot = match args.has("where").then(|| goal_of(film, args.words("where"), line.number)) {
            Some(Goal::Spot(spot)) => Some(spot),
            Some(Goal::Nothing) => film.spot_beside(args.words("where").last().map(String::as_str).unwrap_or_default()),
            Some(Goal::Told) => return,
            None => beside.and_then(|other| film.spot_beside(&other)),
        };
        let Some(spot) = spot else {
            return;
        };
        let at = found(film, who, spot, &[]);
        film.on(who, |who| emit::hop_point(who, at.clone(), HOP_HEIGHT, HOP_FRAMES));
    }
    film.sound("hop");
    film.wait(HOP_FRAMES as i64);
    film.sound("click");
}

/// `a get_off [to x y]`: off his mount's back, he hops there (beside it).
fn got_off(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let mount = film.staging.mounts.remove(&who);
    if let Some(mount) = &mount {
        eased(film, &who, mount);
    }
    if mount.is_none() {
        film.remark(line.number, format!("{who} get_off: he is on nobody's back ({who} ride on <who>)"));
    }
    hops_down(film, line, &who, mount);
    film.wait_frames(10, line.wait);
}

/// `a carry b`: b hops onto a's head and is carried there.
fn carried(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let what = plain(line.args.word("what").unwrap_or_default());
    let high = match film.actor(&who).and_then(|code| film.points_of(&code)) {
        Some(points) => (-points[HEAD].1) as f32 + OVER_HEAD,
        None => 22.0,
    };
    gets_on(film, &what, &who, high, 0.0, line.number);
    film.wait_frames(10, line.wait);
}

/// `a put_down b [x y]`: b hops down off a's head, before him or there.
fn put_down(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let what = plain(line.args.word("what").unwrap_or_default());
    if film.staging.mounts.get(&what) != Some(&who) {
        film.remark(line.number, format!("{who} put_down {what}: he is not carrying him ({who} carry {what})"));
    }
    if film.staging.mounts.remove(&what).is_some() {
        eased(film, &what, &who);
    }
    hops_down(film, line, &what, Some(who));
    film.wait_frames(10, line.wait);
}
