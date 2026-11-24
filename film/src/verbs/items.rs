//! Things in the hands: held, given, thrown, eaten, read, found.

use engine::Pose;

use super::things::{arrives, thing_out};
use super::{Line, Names, Word};
use crate::emit;
use crate::film::{Film, Flying};
use crate::names::things::{Thing, thing_said};
use crate::spots::ending_point;
use crate::spots::routes::found;
use crate::words::plain;

pub const WORDS: &[Word] = &[
    Word::new("give", &["from:pokemon thing... to other:pokemon", "thing... from:pokemon to:pokemon"], "give").names(Names::NoOne).does(given),
    Word::new("hold", &["who:pokemon thing..."], "putnear, thing, hold (or approach and hold if it lies on the ground): between his hands, as the game's points have them on each frame").does(held),
    Word::new("drop", &["who:pokemon at what:place", "who:pokemon [thing...]"], "hold off, fall (at a place: set there, a little hop)").does(let_go),
    Word::new("eat", &["who:pokemon [thing...]"], "anim Eat, scale, gone, emote").does(eaten),
    Word::new("throw", &["who:pokemon line [to|on] [target:place]", "who:pokemon water [to] target:place", "who:pokemon thing... at target:place [straight]", "who:pokemon thing... [to] target:pokemon [straight]"], "throw a Apple to b: face, hold off, throw, then catch or miss; `straight`: tight and fast, at the height of the hands (a shot); throw a line to x y (or on b), the fisher; throw a water to b, a swimmer: jump, particles drops").does(thrown_said),
];

/// The bites of a thing eaten: what is left of it, the frames between two.
const BITES: [f32; 3] = [0.75, 0.5, 0.25];
const BITE_FRAMES: i32 = 14;
/// The frames a thing thrown flies for, how high; those that come after.
const FLIGHT: i32 = 26;
const FLIGHT_HEIGHT: i32 = 24;
/// A shot (`straight`): its frames, the little rise over its straight line.
const SHOT: i32 = 10;
const SHOT_HEIGHT: i32 = 2;
const SHOT_LIFT: f32 = 12.0;
const AFTER_FLIGHT: i32 = 24;

/// Where in `film.things.stuff` what someone holds is: that thing, or the last it took.
pub fn held_by(film: &Film, who: &str, key: Option<&str>) -> Option<usize> {
    film.things.stuff.iter().rposition(|stuff| stuff.owner.as_deref() == Some(who) && key.is_none_or(|key| key == stuff.key))
}

/// What is in someone's hands: the thing, taken out if it is not yet; one
/// lying there, taken up.
pub fn in_hands(film: &mut Film, who: &str, thing: &Thing) -> String {
    if let Some(at) = held_by(film, who, Some(&thing.key)) {
        return film.things.stuff[at].code.clone();
    }
    if let Some(at) = film.things.stuff.iter().rposition(|stuff| stuff.key == thing.key && stuff.owner.is_none()) {
        return take_up(film, who, at);
    }
    let code = thing_out(film, thing);
    put_by(film, &code, who, engine::Direction::Down);
    arrives(film, &code, thing);
    hold(film, &code, Some(who), true);
    set_owner(film, &code, Some(who));
    code
}

/// `putnear <thing> <who> <side>`: the thing put beside someone.
fn put_by(film: &mut Film, code: &str, who: &str, side: engine::Direction) {
    match film.other(who) {
        Some(by) => film.on(code, |thing| emit::put_near(thing, by, side)),
        None => film.not_a(who, "a Pokémon of the scene"),
    }
}

/// `hold <thing> <who>`, in its hands (`carry`: over its head); `hold
/// <thing> off`: put down where it is.
fn hold(film: &mut Film, code: &str, by: Option<&str>, hands: bool) {
    let Some(by) = by else {
        film.on(code, |thing| emit::carry(thing, None, hands));
        return;
    };
    match film.other(by) {
        Some(who) => film.on(code, |thing| emit::carry(thing, Some(who), hands)),
        None => film.not_a(by, "a Pokémon of the scene"),
    }
}

pub fn set_owner(film: &mut Film, code: &str, owner: Option<&str>) {
    if let Some(stuff) = film.things.stuff.iter_mut().find(|stuff| stuff.code == code) {
        stuff.owner = owner.map(str::to_string);
    }
}

/// Someone goes to what lies there and takes it.
pub fn take_up(film: &mut Film, who: &str, at: usize) -> String {
    let code = film.things.stuff[at].code.clone();
    match film.someone(&code) {
        Some(thing) => film.on(who, |who| emit::approach(who, thing, 30.0)),
        None => film.not_a(&code, "a thing of the scene"),
    }
    film.on(who, emit::wait_for);
    film.jump(who, 3.0, 6.0);
    hold(film, &code, Some(who), true);
    film.sound("click");
    set_owner(film, &code, Some(who));
    code
}

/// What a line says someone handles: the thing it names, taken in hand, or
/// what it holds already; told when it names none and holds nothing.
fn handled(film: &mut Film, who: &str, named: &[String], line: &Line, asked: &str) -> Option<String> {
    if !named.is_empty() {
        let thing = thing_said(film, named, line.number)?;
        return Some(in_hands(film, who, &thing));
    }
    match held_by(film, who, None) {
        Some(at) => Some(film.things.stuff[at].code.clone()),
        None => {
            film.wrong(line.number, format!("{who} holds nothing: say {asked}"));
            None
        }
    }
}

/// `set_down who [object] [spot | x y]`: beside it by default.
fn put_down(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    let (named, spot_said) = ending_point(film, line.args.words("what"));
    let Some(code) = handled(film, &who, &named, line, "which object he sets down") else {
        return;
    };
    set_owner(film, &code, None);
    let Some(spot) = spot_said.or_else(|| film.spot_by(&who, 20.0, 8.0)) else {
        return;
    };
    let to = found(film, &who, spot, &[]);
    hold(film, &code, None, true);
    film.jump(&who, 3.0, 6.0);
    // (Where it lands, found as it plays.)
    film.on_at(&code, to, |who, at| emit::hop_point(who, at, 10.0, 16.0));
    film.sound("click");
    film.wait_frames(22, line.wait);
}

pub(crate) fn eaten(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    let Some(code) = handled(film, &who, line.args.words("thing"), line, "what he eats") else {
        return;
    };
    film.things.stuff.retain(|stuff| stuff.code != code);
    film.anim(&who, "Eat", false);
    for left in BITES {
        film.wait(i64::from(BITE_FRAMES));
        film.on(&code, |thing| emit::scale(thing, left, 0.0));
        film.sound("click");
    }
    film.wait(8);
    film.on(&code, emit::gone);
    film.on(&who, |who| emit::pose(who, Pose::Idle));
    film.emote(&who, "joy", Some(40.0));
    film.wait_frames(30, line.wait);
}

/// Handed from someone to someone else: seen over the giver, in the air,
/// then in the other's hands.
fn given(film: &mut Film, line: &Line) {
    let Some(thing) = thing_said(film, line.args.words("thing"), line.number) else {
        return;
    };
    let (giver, taker) = (plain(line.args.word("from").unwrap_or_default()), plain(line.args.word("other").or(line.args.word("to")).unwrap_or_default()));
    let code = match held_by(film, &giver, Some(&thing.key)) {
        Some(at) => film.things.stuff[at].code.clone(),
        None => {
            film.remark(line.number, format!("{giver} gives {} without holding it: it comes out of nowhere (hold {giver} {})", thing.name, thing.name));
            let code = thing_out(film, &thing);
            give(film, &code, &giver, &taker);
            arrives(film, &code, &thing);
            set_owner(film, &code, Some(&taker));
            film.wait_frames(46, line.wait);
            return;
        }
    };
    give(film, &code, &giver, &taker);
    set_owner(film, &code, Some(&taker));
    film.wait_frames(46, line.wait);
}

/// `give <thing> <giver> <taker> 30 keep`: handed over, kept in the other's hands.
fn give(film: &mut Film, code: &str, giver: &str, taker: &str) {
    match (film.other(giver), film.other(taker)) {
        (Some(from), Some(to)) => film.on(code, |thing| emit::give(thing, from, to, 30.0, true)),
        _ => film.unread(format!("{giver} {taker}")),
    }
}

fn thrown(film: &mut Film, line: &Line) {
    let (who, target) = (plain(line.args.word("who").unwrap_or_default()), plain(line.args.word("target").unwrap_or_default()));
    let Some(thing) = thing_said(film, line.args.words("thing"), line.number) else {
        return;
    };
    let code = in_hands(film, &who, &thing);
    set_owner(film, &code, None);
    film.face(&who, &target);
    hold(film, &code, None, true);
    let shot = line.args.has("straight");
    let (height, frames) = if shot { (SHOT_HEIGHT, SHOT) } else { (FLIGHT_HEIGHT, FLIGHT) };
    // (`throw Apple at left_of piplup`: at a place, where it lands and stays.)
    if film.other(&target).is_none() && line.args.has("at") {
        let (_, spot) = ending_point(film, line.args.words("target"));
        if let Some(spot) = spot {
            let to = found(film, &who, spot, &[]);
            film.on_at(&code, to, |thing, at| emit::hop_point(thing, at, height as f32 * 1.5, frames as f32 * 1.4));
            film.wait(i64::from(frames) * 3 / 2);
            film.sound("click");
        }
        return;
    }
    match film.other(&target) {
        // (A shot leaves at the height of the hands and keeps it: let go,
        // lifted back there, then off.)
        Some(at) if shot => {
            film.on(&code, |thing| emit::rise(thing, SHOT_LIFT, 1.0));
            film.on(&code, |thing| emit::after(2.0, 0.0, emit::throw(thing, at, height as f32, frames as f32)));
        }
        Some(at) => film.on(&code, |thing| emit::throw(thing, at, height as f32, frames as f32)),
        None => film.not_a(&target, "a Pokémon of the scene"),
    }
    film.wait(i64::from(frames));
    film.flying = Some(Flying { code, target, wait: line.wait, thrower: who });
}

impl Film {
    /// What was thrown gets there: it hits, unless the next line says that
    /// it is caught or missed. It stays where it falls.
    pub fn throw_lands(&mut self) {
        if let Some(Flying { code, target, wait, thrower }) = self.flying.take() {
            match self.other(&thrower) {
                Some(from) => self.on(&target, |target| emit::recoil(target, from, 10.0)),
                None => self.not_a(&thrower, "a Pokémon of the scene"),
            }
            self.emote(&target, "exclaim", Some(30.0));
            // (Where it lands, found as it plays.)
            if let Some(by) = self.spot_by(&target, 18.0, 6.0) {
                let to = found(self, &code, by, &[]);
                self.on_at(&code, to, |who, at| emit::hop_point(who, at, 8.0, 14.0));
            }
            self.sound("click");
            if wait {
                // (At the engine's pace, not the film's.)
                self.beat(emit::wait(AFTER_FLIGHT as u32));
            }
        }
    }
}

fn let_go(film: &mut Film, line: &Line) {
    // (`pikachu drop at left_of piplup`: set down there.)
    if line.args.has("at") {
        put_down(film, line);
        return;
    }
    let who = plain(line.args.word("who").unwrap_or_default());
    let told = line.args.has("thing");
    let thing = if told { thing_said(film, line.args.words("thing"), line.number) } else { None };
    if told && thing.is_none() {
        return;
    }
    match held_by(film, &who, thing.as_ref().map(|thing| thing.key.as_str())) {
        Some(at) => {
            let code = film.things.stuff[at].code.clone();
            film.things.stuff[at].owner = None;
            hold(film, &code, None, true);
            film.on(&code, |thing| emit::fall(thing, 9.0, 8.0));
            film.sound("click");
        }
        None => film.wrong(line.number, format!("{who} holds nothing")),
    }
}

/// `hold a letter`: in his hands.
fn held(film: &mut Film, line: &Line) {
    if let Some(thing) = thing_said(film, line.args.words("thing"), line.number) {
        in_hands(film, &super::who_said(line), &thing);
    }
}

/// What is thrown says how: a fisher's line, a swimmer's spout, a thing.
fn thrown_said(film: &mut Film, line: &Line) {
    match () {
        _ if line.args.has("line") => super::fishing::cast_line(film, line),
        _ if line.args.has("water") => super::fishing::spout(film, line),
        _ => thrown(film, line),
    }
}
