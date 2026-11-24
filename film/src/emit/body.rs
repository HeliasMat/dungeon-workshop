//! The beats of a body: where it stands, how it shows, jumps, emotes over
//! someone.

use engine::{AnimId, Beat, BodyBeat, Direction, EmoteId, Frames, MoveId, Play, Point, Pose, Stacked, Who};
use scenes::vocabulary::POKEMON;

use crate::film::Film;

/// Someone jumps so high, for so many frames.
pub fn jump(who: Who, height: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Jump { who, height, frames: Frames(frames) })
}

/// An emote of the game over someone, for so many frames if told.
pub fn emote(who: Who, id: EmoteId, frames: Option<f32>) -> Beat {
    emote_with(who, id, frames, false)
}

/// The same, with those shown over him if `add` (else in their place).
pub fn emote_with(who: Who, id: EmoteId, frames: Option<f32>, add: bool) -> Beat {
    Beat::Body(BodyBeat::Effect { who, id, frames: frames.map(Frames), add })
}

/// Someone put at a point, looking that way.
pub fn put(who: Who, x: f32, y: f32, direction: Direction) -> Beat {
    put_point(who, glam::Vec2::new(x, y).into(), direction)
}

/// Someone put at a point (found as it plays, maybe), looking that way.
pub fn put_point(who: Who, at: Point, direction: Direction) -> Beat {
    Beat::Body(BodyBeat::Put { who, at, direction })
}

/// Someone put beside someone else, on that side, turned to them.
pub fn put_near(who: Who, other: Who, side: Direction) -> Beat {
    Beat::Body(BodyBeat::PutNear { who, other, side })
}

/// Someone leaves the stage at once.
pub fn gone(who: Who) -> Beat {
    Beat::Body(BodyBeat::Gone(who))
}

/// Someone turns to someone else.
pub fn face(who: Who, other: Who) -> Beat {
    Beat::Body(BodyBeat::Face { who, other })
}

/// Someone shows that much (1 wholly, 0 not at all), in that many frames.
pub fn opacity(who: Who, to: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Opacity { who, to, frames: Frames(frames) })
}

/// Someone casts its shadow, or not.
pub fn shadow(who: Who, on: bool) -> Beat {
    Beat::Body(BodyBeat::Shadow { who, on })
}

/// Someone drawn over everyone else.
pub fn on_top(who: Who) -> Beat {
    Beat::Body(BodyBeat::Layer { who, place: Stacked::Over })
}

/// Someone stands so high, got there in that many frames.
pub fn rise(who: Who, height: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Rise { who, height, frames: Frames(frames) })
}

/// Someone plays an animation (`emit::anim_of`), looped or once, or
/// stays on its first picture.
pub fn anim(who: Who, group: AnimId, looped: bool, frozen: bool) -> Beat {
    Beat::Body(BodyBeat::Anim { who, anim: group, play: Play::of(looped, frozen) })
}

/// Someone takes a pose of its own: standing, asleep, hurt, attacking.
pub fn pose(who: Who, pose: Pose) -> Beat {
    Beat::Body(BodyBeat::Pose { who, pose })
}

/// Someone shown with another's picture, still itself in what it says (none:
/// its own again).
pub fn mimic(who: Who, like: Option<Who>) -> Beat {
    Beat::Body(BodyBeat::Mimic { who, like })
}

/// Someone trembles for that many frames, that many pixels to and fro.
pub fn tremble(who: Who, frames: f32, strength: f32) -> Beat {
    Beat::Body(BodyBeat::Tremble { who, frames: Frames(frames), strength })
}

/// Words floating up over someone, in that colour.
pub fn popup(who: Who, colour: [u8; 3], text: impl Into<String>) -> Beat {
    Beat::Body(BodyBeat::Popup { who, text: text.into(), colour })
}

/// Someone drawn under everyone else.
pub fn below(who: Who) -> Beat {
    Beat::Body(BodyBeat::Layer { who, place: Stacked::Under })
}

/// Someone drawn among the others again, neither over nor under them.
pub fn among(who: Who) -> Beat {
    Beat::Body(BodyBeat::Layer { who, place: Stacked::Among })
}

/// Someone shows a step of an animation and stays on it (0: its first).
pub fn held_step(who: Who, group: AnimId, step: usize) -> Beat {
    Beat::Body(BodyBeat::Anim { who, anim: group, play: Play::Hold(step) })
}

/// Someone plays an animation so many times, then stands again.
pub fn anim_times(who: Who, group: AnimId, times: u16) -> Beat {
    Beat::Body(BodyBeat::Anim { who, anim: group, play: Play::Times(times) })
}

/// Someone grows or shrinks to that size (1: its own), in that many frames.
pub fn scale(who: Who, to: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Scale { who, to, frames: Frames(frames) })
}

/// What someone's picture is multiplied by (of 255: 255 255 255 as it is,
/// 0 0 0 a silhouette), in that many frames.
pub fn recolour(who: Who, colour: [f32; 3], frames: f32) -> Beat {
    Beat::Body(BodyBeat::Colour { who, colour, frames: Frames(frames) })
}

/// Someone falls from that high to the ground, over so many frames.
pub fn fall(who: Who, height: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Fall { who, height, frames: Frames(frames) })
}

/// Someone floats up and down, that high, a swing taking that many frames
/// (0 high: stops).
pub fn float(who: Who, height: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Float { who, height, frames: Frames(frames) })
}

/// What a thing is called, as it is told.
pub fn thing_name(who: Who, name: impl Into<String>) -> Beat {
    Beat::Body(BodyBeat::ThingName { who, name: name.into() })
}

/// The file of a picture of a film's own, how many pictures side by side
/// in it, the 60ths of a second each shows (none: the usual pace).
pub fn drawing(who: Who, file: impl Into<String>, cells: u16, ticks: Vec<u32>) -> Beat {
    Beat::Body(BodyBeat::Drawing { who, file: file.into(), cells, ticks })
}

/// A thing can be picked up, or not.
pub fn takeable(who: Who, on: bool) -> Beat {
    Beat::Body(BodyBeat::Takeable { who, on })
}

/// A thing glints, or not.
pub fn shine(who: Who, on: bool) -> Beat {
    Beat::Body(BodyBeat::Shine { who, on })
}

/// A thing carried by someone, over its head or in its hands (none: put
/// down where it is).
pub fn carry(what: Who, by: Option<Who>, hands: bool) -> Beat {
    Beat::Body(BodyBeat::Carry { what, by, hands })
}

/// A thing handed from someone to someone else in an arc, in so many
/// frames; kept in the other's hands, or gone there.
pub fn give(what: Who, from: Who, to: Who, frames: f32, keeps: bool) -> Beat {
    Beat::Body(BodyBeat::Give { what, from, to, frames: Frames(frames), keeps })
}

/// Someone thrown in an arc that high to where another stands now, in so
/// many frames.
pub fn throw(who: Who, at: Who, height: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Throw { who, at, height, frames: Frames(frames) })
}

/// Someone takes a blow from someone else: thrown back that far, hurt.
pub fn recoil(who: Who, from: Who, distance: f32) -> Beat {
    Beat::Body(BodyBeat::Recoil { who, from, distance })
}

/// A thing shown large over the scene (none: no longer).
pub fn large(who: Option<Who>) -> Beat {
    Beat::Body(BodyBeat::Large(who))
}

/// Someone spins round that many times, in that many frames.
pub fn spin(who: Who, turns: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Spin { who, turns, frames: Frames(frames), rotate: false })
}

/// A dungeon's status icon over someone (`status_icon`; none: taken off).
pub fn status(who: Who, icon: Option<&'static str>) -> Beat {
    Beat::Body(BodyBeat::Status { who, icon, add: false })
}

/// A status icon of the game by its name (`sleep`).
pub fn status_icon(name: &str) -> Option<&'static str> {
    POKEMON.status_icons.iter().find(|own| **own == name).copied()
}

/// Someone turns that way.
pub fn turn(who: Who, direction: Direction) -> Beat {
    Beat::Body(BodyBeat::Turn { who, direction })
}

/// Someone turns to look at a point.
pub fn look_at(who: Who, x: f32, y: f32) -> Beat {
    look_at_point(who, glam::Vec2::new(x, y).into())
}

/// Someone turns to look at a point (found as it plays, maybe).
pub fn look_at_point(who: Who, at: Point) -> Beat {
    Beat::Body(BodyBeat::LookAt { who, at })
}

/// Someone stops whatever it does: walking, going round, floating...
pub fn stop(who: Who) -> Beat {
    Beat::Body(BodyBeat::Stop(who))
}

/// Someone as it was: its size, colour, height, animation.
pub fn reset(who: Who) -> Beat {
    Beat::Body(BodyBeat::Reset(who))
}

/// Someone looks about now and then as it stands, or no longer.
pub fn fidget(who: Who, on: bool) -> Beat {
    Beat::Body(BodyBeat::Fidget { who, on })
}

/// Someone hops on the spot again and again, that high, a hop taking that
/// many frames (0 high: stops).
pub fn bounce(who: Who, height: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Bounce { who, height, frames: Frames(frames) })
}

/// Someone blinks, there and not there, or no longer.
pub fn blink(who: Who, on: bool) -> Beat {
    Beat::Body(BodyBeat::Blink { who, on })
}

/// Someone hops to a point in an arc that high, in so many frames.
pub fn hop(who: Who, x: f32, y: f32, height: f32, frames: f32) -> Beat {
    hop_point(who, glam::Vec2::new(x, y).into(), height, frames)
}

/// Someone hops to a point (found as it plays, maybe) in an arc that high.
pub fn hop_point(who: Who, at: Point, height: f32, frames: f32) -> Beat {
    Beat::Body(BodyBeat::Hop { who, at, height, frames: Frames(frames) })
}

/// A fishing line from someone's rod to something (none: no line), cast to
/// a point if told; or, not cast, only tied to what is at its end already.
pub fn fishline(who: Who, to: Option<Who>, at: Option<glam::Vec2>, cast: bool) -> Beat {
    Beat::Body(BodyBeat::FishLine { who, to, at, cast })
}

/// Someone rides on someone else's back, that high, that far beside its
/// middle (none: gets off).
pub fn ride(who: Who, on: Option<Who>, high: f32, beside: f32) -> Beat {
    Beat::Body(BodyBeat::Ride { who, on, high, beside })
}

/// Someone uses a move of the game (`POKEMON` moves, by number), on someone
/// else or no one.
pub fn uses_move(who: Who, id: MoveId, target: Option<Who>) -> Beat {
    Beat::Body(BodyBeat::Move { who, id, target })
}

impl Film {
    /// `jump <who> <height> <frames>`.
    pub fn jump(&mut self, who: &str, height: f32, frames: f32) {
        self.on(who, |who| jump(who, height, frames));
    }

    /// `face <who> <other>`: someone turns to someone else (a crowd turned
    /// to, or no one: told).
    pub fn face(&mut self, who: &str, other: &str) {
        if let Some(other) = self.one(other) {
            self.on(who, |who| face(who, other));
        }
    }

    /// `anim <who> <animation> [1]`, the animation by a name the game gives
    /// it, looped or once. One the game has not: told.
    pub fn anim(&mut self, who: &str, name: &str, looped: bool) {
        match super::anim_of(name) {
            Some(group) => self.on(who, |who| anim(who, group, looped, false)),
            None => self.not_a(name, "an animation of the game"),
        }
    }

    /// `emote <who> <emote> [frames]`, the emote by a name the game gives it.
    pub fn emote(&mut self, who: &str, name: &str, frames: Option<f32>) {
        match POKEMON.emote(name) {
            Some(id) => self.on(who, |who| emote(who, EmoteId(id), frames)),
            None => self.not_a(name, "an emote of the game"),
        }
    }
}

/// The ways someone is painted in (none: all of them), the only ones it faces by itself.
pub fn views(who: Who, ways: Vec<Direction>) -> Beat {
    Beat::Body(BodyBeat::Views { who, ways })
}

/// What a move shows on someone, by the move's number.
pub fn move_effect(who: Who, id: MoveId) -> Beat {
    Beat::Body(BodyBeat::MoveEffect { who, id })
}

/// What a move throws from someone to someone else, in so many frames if told.
pub fn move_shot(from: Who, to: Who, id: MoveId, frames: Option<f32>) -> Beat {
    Beat::Body(BodyBeat::MoveShot { from, to, id, frames: frames.map(Frames) })
}

/// Someone sunk so many pixels in the water (in the sand, if told): drawn
/// that much lower, the rest of him under it.
pub fn sink(who: Who, depth: f32, sand: bool) -> Beat {
    Beat::Body(BodyBeat::Sink { who, depth, sand })
}

/// Someone's picture turned round left to right, or as it was.
pub fn flip(who: Who, on: bool) -> Beat {
    Beat::Body(BodyBeat::Flip { who, on })
}

/// Someone drawn just in front of someone else (none: as it stands again).
pub fn in_front(who: Who, of: Option<Who>) -> Beat {
    Beat::Body(BodyBeat::InFront { who, of })
}
