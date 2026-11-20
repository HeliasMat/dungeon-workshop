//! Fishing, the part that keeps what the film knows from one line to the
//! next: the line cast (`throw a line`) and what holds it, what bites by
//! chance and comes out (`haul`), the fish's run and the tension gauge
//! (`struggle`), the fish's shadow (`shadow`), the bait (`bait`); what was
//! caught, measured, let go, kept, told at the end (`measure`, `release`,
//! `bucket`, `net`, `empty_net`, `fishing_report`, `records`, `gift`,
//! `snatch`); those watching (`spectators`); out at sea, who rides and swims
//! where (`ride`, `dismount`, `swim`, `plunge`, `burst_out`, `take_off`,
//! `pursue`, `storm`), the friends who pull and the fish that tows
//! (`chain`, `tow`), the lantern at night (`lantern`), the gulls
//! (`gulls`), a whirlpool (`whirlpool`).
//!
//! Every other fishing word is a recipe, in `film/recipes/fishing/`, made of
//! the engine's commands and of the small gestures here: `bobber a` (his
//! bobber is `bobber` in the lines after, `{bobber.x}`), `reel_in a`,
//! `shadow a dives|flees`, `spectators joy|sorrow`. The rod is drawn in a
//! variant's own pictures (`pikachu_pecheur`, painted by hand): his cast is
//! his `Swing`, his pull his `Pull`, and where his line starts on each picture
//! is told in his table (`pointe`). Here is only who shows what.

use engine::{Beat, Cam, Num, Point, Pose, Who};
use glam::Vec2;

use super::items::set_owner;
use super::things::{arrives, thing_apart, thing_out};
use super::{Line, Word};
use crate::emit;
use crate::film::{Caught, Film};
use crate::names::species;
use crate::names::things::{known_thing, thing_named};
use crate::spots::routes::found;
use crate::words::{digits, plain};

pub const WORDS: &[Word] = &[
    Word::new("bite", &["[strong|nibble]"], "a fish takes the bait of the line in the water (the last cast): the recipe bite <who> (strong: yanked under; nibble: it only touches)").maybe(bitten),
    Word::new("cast", &["who:pokemon at target:place", "who:pokemon"], "his cast (Swing, on a fisher's variant), the line flies out, the bobber lands on the water there (or before him)")
        .does(cast_line),
    Word::new("_bobber", &["who:pokemon [x:number y:number]"], "his bobber is called \"bobber\" in the lines after (jump bobber, {bobber.x}), holdgaze; x y: it is there now").does(names_bobber).instant(),
    Word::new("_reel_in", &["who:pokemon [broken]"], "fishline off, the shadow dives, the gauge goes, the bobber taken away (broken: the gauge full first, the bobber stays on the water)").does(line_in),
    Word::new("_haul", &["who:pokemon [heavy] [unhooks]", "who:pokemon catch:word [x:number y:number] [heavy] [unhooks]"], "anim Pull, tremble (him, the bobber, the catch), particles drops, rings and bubbles, put on the bobber, fishline holds, hop of what comes out of the water (up to x y if told), emit drops, pose idle").does(pulled).brings(&["catch"]),
    Word::new("_shadow", &["giant x:number y:number to x2:number y2:number [in d:duration]", "who:pokemon flees|dives", "who:pokemon [big] [x:number y:number]"], "shadow giant x y to x y: the shadow of a very big fish under the sea; the shadow of a fish (a picture): put, layer under, anim, go up to the bobber, circles round; big; flees: go, gone; dives: opacity 0").does(shadow_said),
    Word::new("release", &["who:pokemon [catch:word] [x:number y:number]"], "popup, hop of the catch to the water, particles drops and rings, gone, emote").does(released),
    Word::new("_spectators", &["off", "joy|sorrow", "*who"], "then, at each catch: emote joy, jump; at each loss: emote sweat; joy, sorrow: they react at once; off").does(watched),
];

mod state;
pub use state::FishingState;
mod catches;
mod line;
mod spectacle;
mod water;

use catches::*;
use line::*;
use spectacle::*;
use water::*;

/// The thing at the end of a line: the name the recipes call it by, and its
/// picture (library/images/).
const BOBBER: &str = "bobber";
pub(crate) const BOBBER_PICTURE: &str = "bobber";
/// What comes out with `haul a random`, by chance (the same each time the film
/// plays): the sea's own, a berry, an apple, or nothing.
const CATCHES: [&str; 12] = ["magikarp", "poliwag", "krabby", "staryu", "shellder", "qwilfish", "remoraid", "chinchou", "Apple", "oranberry", "Apple", "-"];
const HEAVY: i32 = 150;
/// A leap out of the water (`jump a water`): how far, how high, how long.
const LEAP: i32 = 44;
const LEAP_HEIGHT: i32 = 38;
const LEAP_FRAMES: i32 = 40;
/// Rings on the water about a bobber at rest: so many a second.
const RESTING_RINGS: f32 = 0.7;
/// The frames a line flies for (see `Stage::fish_line`); a rod swung back before.
const CAST: i32 = 30;
const SWUNG: i32 = 14;
/// How a catch flies: how high, for how many frames; how long it is fought.
const CATCH_HEIGHT: i32 = 80;
const CATCH_FRAMES: i32 = 54;
const FOUGHT: i32 = 60;

/// Whether that one is a fisher, his rod drawn in his pictures.
fn fisher(film: &mut Film, who: &str) -> bool {
    let code = film.code(who);
    species(&code).is_some_and(|number| film.tables.fishing.contains(number))
}

/// A fisher turned to us and kept so while he sleeps: his sleep is painted
/// from the front only (his walk, his cast and his pull all round).
pub fn faces_us(film: &mut Film, who: &str) {
    // (The engine turns him to the first way he is painted in by itself.)
    if fisher(film, who) {
        film.on(who, |who| emit::views(who, vec![engine::Direction::Down]));
    }
}

/// A fisher keeps the way he faces while he fishes, his cast and his pull
/// painted on every side: the lines said around do not turn him from the
/// water.
pub fn keeps_his_way(film: &mut Film, who: &str) {
    if fisher(film, who) {
        film.on(who, |who| emit::hold_gaze(who, true));
    }
}

/// Free to turn again, his line in.
pub fn free_to_turn(film: &mut Film, who: &str) {
    if fisher(film, who) {
        film.on(who, |who| emit::hold_gaze(who, false));
    }
}

/// The fish's shadow under his bobber, if one waits there, gone under.
pub fn shadow_dives(film: &mut Film, who: &str) {
    if let Some(code) = film.fishing.shadows.remove(who) {
        film.on(&code, |code| emit::opacity(code, 0.0, 8.0));
    }
}

/// The tension of his line, if it was shown, gone with his line; full first
/// if it snapped.
pub fn tension_off(film: &mut Film, who: &str, snapped: bool) {
    if film.fishing.tension.as_deref() == Some(who) {
        if snapped {
            film.beat(emit::set(tension_of(who), emit::number(100)));
            film.wait(10);
        }
        film.beat(emit::hud(MIDDLE, None));
        film.beat(emit::gauge(MIDDLE, None));
        film.fishing.tension = None;
    }
}

/// Those watching react: glad at a catch, sorry at one lost.
pub fn cheer(film: &mut Film, glad: bool) {
    for (at, one) in film.fishing.fans.clone().iter().enumerate() {
        // (Not all at once: one after the other, a little.)
        let after = at as i32 * 6 + 1;
        if glad {
            film.on(one, |one| emit::after(after as f32, 0.0, emit::emote(one, scenes::vocabulary::emote::JOY, Some(40.0))));
            film.on(one, |one| emit::after(after as f32, 0.0, emit::jump(one, 4.0, 8.0)));
        } else {
            film.on(one, |one| emit::after(after as f32, 0.0, emit::emote(one, scenes::vocabulary::emote::SWEAT, Some(40.0))));
        }
    }
}

/// Where the tension of his line is kept on screen (`hud middle`).
const MIDDLE: usize = 1;

/// So many particles of a kind of the game about someone, of a colour of
/// the game (none: their kind's own).
fn drops(film: &mut Film, on: &str, kind: engine::ParticleKind, count: u16, colour: Option<engine::Colour>) {
    let colour = colour.map(emit::colour);
    film.on(on, |on| emit::particles(kind, Cam::Who(on), count, colour));
}

/// So many particles of a kind of the game at a point, of a colour of the
/// game (none: their kind's own).
fn drops_at(x: i32, y: i32, kind: engine::ParticleKind, count: u16, colour: Option<engine::Colour>) -> Beat {
    emit::particles(kind, Cam::at(Vec2::new(x as f32, y as f32)), count, colour.map(emit::colour))
}

/// Words floating up over someone, in a colour of the game.
fn floating(film: &mut Film, who: &str, colour: engine::Colour, text: &str) {
    let colour = emit::colour(colour);
    film.on(who, |who| emit::popup(who, colour, text));
}

/// A beat of someone with someone else (`approach`, `putnear`, `throw`,
/// `ride`); the other a crowd or no one: the line as it was.
fn with_other(film: &mut Film, who: &str, other: &str, beat: impl Fn(Who, Who) -> Beat) {
    if let Some(other) = film.one(other) {
        film.on(who, |who| beat(who, other));
    }
}

/// A point of the water as the film keeps it (numbers, or a place found as
/// it plays): its numbers, if they are.
fn numbers(x: &str, y: &str) -> Option<(f32, f32)> {
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// The numbers of a point, when both are written (not found as it plays).
fn written(at: &Point) -> Option<(f32, f32)> {
    match (&at.x, &at.y) {
        (Num::Lit(x), Num::Lit(y)) => Some((*x, *y)),
        _ => None,
    }
}

/// The number the tension gauge of his line shows.
fn tension_of(who: &str) -> String {
    format!("tension_{who}")
}

/// Whether that one has a rod drawn in its pictures; told when not.
fn has_a_rod(film: &mut Film, who: &str, line: &Line) -> bool {
    let code = film.code(who);
    let has = species(&code).is_some_and(|number| film.tables.fishing.contains(number));
    if !has {
        film.remark(line.number, format!("{who} has no rod: it is only drawn on pikachu_pecheur"));
    }
    has
}

/// What holds a line, told when there is none.
fn line_of(film: &mut Film, who: &str, line: &Line) -> Option<String> {
    let code = film.fishing.fishing.get(who).map(|(code, _)| code.clone());
    if code.is_none() {
        film.remark(line.number, format!("{who} has no line in the water: throw {who} line"));
    }
    code
}

/// Where his bobber is on the water, when it was cast to a point.
fn bobber_at(film: &Film, who: &str) -> Option<(i32, i32)> {
    let (x, y) = film.fishing.fishing.get(who)?.1.as_ref()?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// Whom a line is about (its `who`).
fn who_of(line: &Line) -> String {
    plain(line.args.word("who").unwrap_or_default())
}

/// The point a line tells under those names, if it tells it (a point past an
/// edge, to come in by it, may be below naught).
fn point_named(line: &Line, x: &str, y: &str) -> Option<(i32, i32)> {
    let near = |number: f32| (number.abs() <= crate::grammar::FARTHEST).then_some(number as i32);
    Some((near(line.args.number(x)?)?, near(line.args.number(y)?)?))
}

/// The point a line tells (`x y`), if it tells one.
fn point_of(line: &Line) -> Option<(i32, i32)> {
    point_named(line, "x", "y")
}

/// `throw a line [to] x y`, `throw a line on b`: his line cast (the
/// fisher's own words: see `items::verb`).
pub(crate) fn cast_line(film: &mut Film, line: &Line) {
    cast(film, line);
}

/// `throw a water to b`: a spout from a swimmer's back, on b.
pub(crate) fn spout(film: &mut Film, line: &Line) {
    spouts(film, line);
}

/// `jump a water x y`, `jump school x y`, `jump a wave`: leaps in the
/// water (see `moving::body`); none: not one of them.
pub(crate) fn leap(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    match () {
        _ if args.has("school") => school_leaps(film, line),
        _ if args.has("water") => leaps(film, line),
        _ if args.has("wave") => leaps_a_wave(film, line),
        _ => return false,
    }
    true
}

/// `shadow giant x y to x y`: a shadow under the open sea, no line needed;
/// `shadow a [big] [x y]`: the fish's, under his bobber; `shadow a flees`: it
/// darts off; `shadow a dives`: it goes down under (something bites). Both
/// nothing if no shadow waits there.
fn shadow_said(film: &mut Film, line: &Line) {
    let args = &line.args;
    match () {
        _ if args.has("giant") => giant_passes(film, line),
        _ if args.has("dives") => shadow_dives(film, &who_of(line)),
        _ if args.has("flees") => {
            if flees(film, &who_of(line)) {
                film.wait_frames(10, line.wait);
            }
        }
        _ => shadow(film, line),
    }
}

/// `bite [strong|nibble]`: a fish at the line in the water, the last cast
/// (`film/recipes/fishing/line.film`: `bite who`, `bite who strong`, `bite
/// who nibble`).
fn bitten(film: &mut Film, line: &Line) -> bool {
    if line.args.has("who") {
        return false;
    }
    let fisher = film.fishing.last_cast.clone().filter(|who| film.fishing.fishing.contains_key(who));
    let Some(who) = fisher.or_else(|| film.fishing.fishing.keys().next_back().cloned()) else {
        film.remark(line.number, "no line in the water for a fish to bite: who cast first".to_string());
        return true;
    };
    match ["strong", "nibble"].into_iter().find(|how| line.args.has(how)) {
        Some(how) => film.call(line.number, "_bite", &[&who, how]),
        None => film.call(line.number, "_bite", &[&who]),
    }
    true
}

/// `bobber a [x y]`: his bobber is `bobber` in the lines after (`jump
/// bobber 2 6f`, `{bobber.x}`), and he keeps facing the water; `x y`:
/// it lies there now (where what bites comes out). The gesture the fishing
/// recipes start with (`film/recipes/fishing/`).
fn names_bobber(film: &mut Film, line: &Line) {
    let who = who_of(line);
    let Some(code) = line_of(film, &who, line) else {
        return;
    };
    film.give_name(BOBBER.to_string(), code.clone());
    keeps_his_way(film, &who);
    if let Some((x, y)) = point_of(line) {
        film.fishing.fishing.insert(who, (code, Some((x.to_string(), y.to_string()))));
    }
}

/// `reel_in a [broken]`: his line in: the shadow under it gone, the
/// tension gauge too, his bobber off the water; `broken`: snapped, the gauge
/// full first, the bobber left floating (`gone bobber`, after). Nothing
/// if his line is not out.
fn line_in(film: &mut Film, line: &Line) {
    let who = who_of(line);
    // (Without a rod the line was cast all the same, told then: it comes in.)
    let snapped = line.args.has("broken");
    shadow_dives(film, &who);
    tension_off(film, &who, snapped);
    if let Some((code, _)) = film.fishing.fishing.remove(&who) {
        film.on(&who, |who| emit::fishline(who, None, None, true));
        if !snapped {
            film.on(&code, emit::gone);
        }
    }
}
