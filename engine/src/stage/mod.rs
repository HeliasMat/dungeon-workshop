//! The stage of a scene, with no engine: who stands on it and how they
//! move (walks, jumps, spins, trembles, following someone about), where
//! the view looks and how close, the light, the weather, bubbles, film
//! bands and captions. The beats that only change the stage are played
//! here ([`Stage::direct`]), and time passes on it frame by frame
//! ([`Stage::advance`]); the game shows it as it likes.
//!
//! - `camera/`: the view, its one owner ([`Camera`], [`Viewport`]);
//! - `layers`: in what order they are drawn ([`Layer`]);
//! - `character/`: a body on it, the one model ([`Actor`], [`Figure`]);
//! - `motion`: how a value goes somewhere in time; `air`: particles;
//! - `placing`, `ground`: where they may stand;
//! - `thing`: the things on it, carried and handed over;
//! - `node`: everything it shows as one model, its properties by the
//!   registry's names ([`node::Node`], [`Stage::get`], [`Stage::set`]);
//! - `light`, `words`, `emotes`: the light, the words over it, what shows
//!   over heads;
//! - `shots`: what moves throw; `controls`: what the player walks about;
//! - `direct`: the beats played on it.

use std::collections::{BTreeMap, HashMap, VecDeque};

use glam::Vec2;

use crate::beat::*;
use crate::who::Who;

mod air;
mod camera;
mod character;
mod controls;
mod direct;
mod ease;
mod emotes;
pub mod framed;
mod ground;
mod layers;
mod light;
mod motion;
pub mod node;
mod placing;
pub(crate) use placing::scatter;
mod shots;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_attach;
#[cfg(test)]
mod tests_nodes;
mod thing;
pub mod tracks;
mod words;

/// How far behind one who follows stops, when not told.
pub(crate) const FOLLOW_GAP: f32 = 22.0;
/// How who hovers by nature bobs: how high, how many frames.
const HOVER: (f32, f32) = (2.0, 60.0);

pub use air::*;
pub use camera::*;
pub use character::*;
pub use controls::*;
pub use ease::{Ease, EaseFamily};
pub use emotes::*;
pub use ground::{Ground, TILE};
pub use layers::*;
pub use light::*;
pub use motion::*;
pub use node::{Node, SetError};
pub use placing::{Note, Placing};
pub use shots::*;
pub use thing::*;
pub use tracks::Track;
pub use words::*;

/// A line walking where its leader walked: how far apart, the leader then
/// the others, and the leader's way, the latest point first.
#[derive(Clone, Debug, PartialEq)]
pub struct Train {
    pub gap: f32,
    pub who: Vec<Who>,
    pub way: Vec<Vec2>,
}

/// A scene's stage: who is on it (see [`Who`]), the camera, the light, the
/// words. The view's focus is the game's to keep: it is given to
/// [`Stage::direct`] and [`Stage::advance`], which hands it back.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stage {
    pub cast: BTreeMap<Who, Actor>,
    pub shots: Vec<Shot>,
    /// What shows over them: emotes, effects, statuses, words rising.
    pub emotes: Emotes,
    /// Who walks steps round the others (see `MotionBeat::Spacing`).
    pub spacing: bool,
    /// No collisions (see `PlayBeat::Collisions`): walks go straight where
    /// they are sent, through walls and bodies, and places are as written.
    pub no_collisions: bool,
    /// What chance has drawn last (for who wanders about).
    pub seed: u32,
    /// How much faster time goes on the scene (none: as usual; see
    /// `ViewBeat::Tempo`).
    pub tempo: Option<f32>,
    /// A line walking where its leader walked (see `MotionBeat::Train`): how far
    /// apart, the leader then the others, and the leader's way, the
    /// latest point first.
    pub train: Option<Train>,
    pub view: Camera,
    pub light: Light,
    pub words: Words,
    pub play: PlayState,
    /// What it knows of the ground (see `PlaceBeat`).
    pub place: Placing,
    /// What is told of the things on it (see [`Thing`]).
    pub(crate) marks: BTreeMap<Who, Marks>,
    /// Things on their way from a hand to another.
    pub(crate) handing: Vec<Handing>,
    /// The lines that go from someone's rod to something.
    pub lines: BTreeMap<Who, Who>,
    /// How each one is shown, kept through the scenes: the
    /// ways it is painted in (see `Figure`).
    pub figures: BTreeMap<Who, Figure>,
    /// What each node keeps past what its owner has (an angle, a blend, a
    /// `z`...), and the nodes of a film's own wholly (see `node`): read
    /// them by `Stage::node`, which puts the two together.
    pub nodes: BTreeMap<NodeRef, Node>,
    /// The properties of nodes going through keys in time, the nodes going
    /// along ways (see `tracks`), in the order they started.
    pub tracks: Vec<Track>,
}

impl Stage {
    /// An empty stage on a dark screen: an event of the story opens so,
    /// and its scenes fade in.
    pub fn dark() -> Self {
        Stage { light: Light { fade: Fade::DARK, ..Light::default() }, ..Stage::default() }
    }

    /// Puts someone on the stage, standing still, in the place of who
    /// stood there under that number.
    pub fn put(&mut self, who: Who, at: Vec2, direction: u16) {
        self.cast.insert(who, Actor::at(at, direction));
    }

    /// Takes someone off the stage, with what it showed and what it holds.
    pub fn remove(&mut self, who: Who) {
        self.cast.remove(&who);
        self.cast.retain(|_, actor| actor.habits.carried_by != Some(who) || !actor.habits.held);
        self.emotes.effects.remove(&who);
        self.nodes.remove(&crate::beat::NodeRef::Who(who));
        self.words.bubbles.retain(|bubble| bubble.who != who);
    }

    /// Has someone of the stage show an effect, in the place of those it
    /// showed (0: none), for that many frames when given.
    pub fn effect(&mut self, who: Who, id: u16, frames: Option<f32>) {
        self.effect_with(who, id, frames, false);
    }

    /// The same, with those it shows if `add`.
    pub fn effect_with(&mut self, who: Who, id: u16, frames: Option<f32>, add: bool) {
        let here = self.cast.contains_key(&who);
        self.emotes.effect(who, id, frames, here, add);
    }

    /// Hands a thing over from someone to someone else: it comes out of
    /// the giver's hands and goes to the other in an arc, then leaves the
    /// stage (`keeps`: see `Stage::hand_over`). Gives where it starts, if
    /// both are on the stage.
    pub fn give(&mut self, what: Who, from: Who, to: Who, frames: f32, keeps: bool) -> Option<Vec2> {
        let start = self.cast.get(&from)?.position + Vec2::new(0.0, 2.0);
        let end = self.cast.get(&to)?.position + Vec2::new(0.0, 2.0);
        if keeps {
            self.hand_over(what, from, to, frames);
            return Some(start);
        }
        let mut thing = Actor::at(start, 1);
        thing.go(end, frames, true);
        (thing.air.height, thing.air.arc, thing.habits.vanishes) = (14.0, Some((24.0, frames.max(1.0))), true);
        self.cast.insert(what, thing);
        Some(start)
    }

    /// Whether anyone is still moving, or a track that ends still plays.
    pub fn busy(&self) -> bool {
        self.cast.values().any(Actor::busy) || self.tracks.iter().any(Track::ends)
    }

    /// Everything that is going somewhere gets there at once: who walks,
    /// rises, jumps, spins, goes round, fades or changes colour, the view,
    /// the light, the film bands. (A scene skipped up to a point: see
    /// `Director::skip_to`.)
    pub fn settle(&mut self) {
        self.words.caption = None;
        self.words.log.clear();
        for actor in self.cast.values_mut() {
            actor.position = actor.way.route.back().copied().unwrap_or(if actor.way.left > 0.0 { actor.way.to } else { actor.position });
            (actor.way.route, actor.way.left) = (VecDeque::new(), 0.0);
            if let Some(Tween { to, .. }) = actor.air.rise.take() {
                actor.air.height = to;
            }
            if actor.air.arc.take().is_some() {
                actor.air.height = 0.0;
            }
            actor.end_spin();
            (actor.habits.orbit, actor.air.tremble) = (None, 0.0);
            if let Some(Tween { to, .. }) = actor.looks.fading.take() {
                actor.looks.opacity = to;
            }
            if let Some(Tween { to, .. }) = actor.looks.colouring.take() {
                actor.colour = to;
            }
            if let Some(Tween { to, .. }) = actor.looks.scaling.take() {
                actor.looks.scale = to;
            }
            if let Some(point) = actor.way.then_face.take() {
                actor.direction = towards(actor.position, point).unwrap_or(actor.direction);
            }
        }
        self.cast.retain(|_, actor| !actor.habits.vanishes);
        self.hand_over_at_once();
        self.view.settle();
        (self.light.tint.now, self.light.tint.left) = (self.light.tint.to, 0.0);
        self.light.fade.level = self.light.fade.to;
        self.words.letterbox.shown = self.words.letterbox.to;
        // (What was said in bubbles on the way is gone with it.)
        self.words.bubbles.clear();
        self.settle_tracks();
    }

    /// Another place comes on (a backdrop): the view as it usually is, not
    /// as close, aside or framing as the last place had it; the last
    /// place's walls, box and pictures gone with it. (Where it looks, the
    /// light and an iris stay: the scene says so.)
    pub(crate) fn new_place(&mut self) {
        self.view.new_place();
        self.clear_walls();
        (self.play.place_walls, self.play.bounds) = (Vec::new(), None);
        self.words.pictures.clear();
    }

    /// Whether the light still changes: a fade, a dimming, a tint.
    pub(crate) fn light_busy(&self) -> bool {
        self.light.busy() || self.words.credits.is_some()
    }

    /// The end of it all: the screen dark with a few words that stay.
    pub fn curtain(&mut self, words: &str) {
        let black = [0.0, 0.0, 0.0, 255.0];
        // (No one stays on a stage whose curtain fell.)
        self.cast.clear();
        (self.light.fade, self.light.tint) = (Fade::CLEAR, Tint { now: black, to: black, left: 0.0 });
        (self.words.bubbles, self.light.weather) = (Vec::new(), Weather::Clear);
        self.words.caption = Some(Caption { left: f32::INFINITY, text: words.to_string(), typed: None });
    }

    /// Does something to someone of the stage, if they are on it.
    fn with(&mut self, who: Who, change: impl FnOnce(&mut Actor)) {
        if let Some(actor) = self.cast.get_mut(&who) {
            change(actor);
        }
    }

    /// Lets that many frames pass (`clock`: the seconds gone since the
    /// game started, for what shakes): who walks goes on walking, who
    /// follows someone walks after them, the view glides, the light, the
    /// closeness and the film bands go where they were sent, bubbles and
    /// captions go when their time is up, a thing handed over leaves once
    /// it is there. Gives where the view looks (`focus`: where it looked).
    pub fn advance(&mut self, frames: f32, clock: f32, focus: Option<Vec2>) -> Option<Vec2> {
        self.move_by_themselves(frames);
        self.pass_things(frames);
        self.pass_cast(frames);
        let focus = self.view.advance(frames, clock, focus, &self.cast, self.words.boxed, self.words.box_top);
        self.light.pass(frames);
        self.words.pass(frames);
        self.pass_spotlight(frames);
        self.reveal_map();
        focus
    }

    /// What the team sees of a floor whose map is found as it goes: the
    /// ground one walks to from where each stands, so far, never through a
    /// wall (a room seen whole once one is in it).
    pub(crate) fn reveal_map(&mut self) {
        const SEEN: i32 = 10;
        let (Some(team), Some(ground)) = (self.words.minimap.as_ref(), self.place.ground.as_ref()) else {
            return;
        };
        let Some(seen) = self.words.minimap_seen.as_mut() else {
            return;
        };
        let rows = ground.rows();
        let free = |x: i32, y: i32| y >= 0 && x >= 0 && (y as usize) < rows.len() && (x as usize) < rows[0].len() && !rows[y as usize][x as usize];
        for who in team {
            let Some(at) = self.cast.get(who).map(|actor| actor.position) else {
                continue;
            };
            let start = ((at.x / TILE) as i32, (at.y / TILE) as i32);
            let mut todo = std::collections::VecDeque::from([start]);
            let mut been = std::collections::HashSet::from([start]);
            while let Some((x, y)) = todo.pop_front() {
                if let Some(tile) = seen.get_mut(y as usize).and_then(|row| row.get_mut(x as usize)) {
                    *tile = true;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let next = (x + dx, y + dy);
                    if free(next.0, next.1) && (next.0 - start.0).abs().max((next.1 - start.1).abs()) <= SEEN && been.insert(next) {
                        todo.push_back(next);
                    }
                }
            }
        }
    }

    /// Who swims by nature is in the water where no one walks (sunk so deep)
    /// and out of it on the ground; not while in the air or carried.
    fn in_the_water(&mut self) {
        // (Who hovers by nature bobs once there.)
        let here: Vec<Who> = self.place.hovering.iter().copied().filter(|who| self.cast.contains_key(who)).collect();
        for who in here {
            self.place.hovering.remove(&who);
            self.with(who, |actor| actor.air.float = crate::stage::motion::Swing::new(HOVER.0, HOVER.1));
        }
        let Some(ground) = self.place.ground.as_ref() else {
            return;
        };
        for (who, depth) in &self.place.swimmers {
            let Some(actor) = self.cast.get_mut(who) else {
                continue;
            };
            let wet = !actor.carried() && actor.air.height < 2.0 && ground.blocked(actor.position, 0);
            match (wet, self.place.swimming.contains(who)) {
                (true, false) => {
                    (actor.sunk, actor.in_sand) = (*depth, false);
                    self.place.swimming.insert(*who);
                }
                (false, true) => {
                    actor.sunk = 0.0;
                    self.place.swimming.remove(who);
                }
                _ => {}
            }
        }
    }

    /// Who follows, wanders; what is carried goes with who has it.
    fn move_by_themselves(&mut self, frames: f32) {
        // (Who goes round someone goes round where that one is now.)
        let places: HashMap<Who, Vec2> = self.cast.iter().map(|(who, actor)| (*who, actor.position)).collect();
        for orbit in self.cast.values_mut().filter_map(|actor| actor.habits.orbit.as_mut()) {
            if let Some(there) = orbit.around.and_then(|other| places.get(&other)) {
                orbit.center = *there;
            }
        }
        self.follow();
        self.train();
        self.mirror();
        self.wander(frames);
    }

    /// What goes on by itself about the scene.
    fn pass_things(&mut self, frames: f32) {
        self.pass_shots(frames);
        self.emit(frames);
        self.emotes.rise(frames);
    }

    fn pass_cast(&mut self, frames: f32) {
        // (Each in its own time: slowed down, its moves are.)
        for actor in self.cast.values_mut() {
            actor.advance(frames * actor.looks.time);
        }
        // (What tracks give, before who hangs on them goes along.)
        self.pass_tracks(frames);
        if self.spacing && !self.no_collisions {
            self.step_aside(frames);
        }
        self.pass_handing(frames);
        self.in_the_water();
        self.carried_along();
        self.attached_along();
        let places: HashMap<Who, Vec2> = self.cast.iter().map(|(who, actor)| (*who, actor.position)).collect();
        // (Who has a line out keeps an eye on it.)
        for (who, to) in self.lines.clone() {
            if let Some(there) = places.get(&to) {
                self.with(who, |actor| actor.direction = towards(actor.position, *there).unwrap_or(actor.direction));
            }
        }
        self.keep_painted_ways();
        self.cast.retain(|_, actor| !(actor.habits.vanishes && !actor.busy()));
        // (Who rides, or is carried, goes with what bears it.)
        let here: std::collections::HashSet<Who> = self.cast.keys().copied().collect();
        self.cast.retain(|_, actor| actor.habits.carried_by.is_none_or(|mount| here.contains(&mount)));
        let cast = &self.cast;
        self.marks.retain(|who, _| cast.contains_key(who));
        self.lines.retain(|who, to| cast.contains_key(who) && cast.contains_key(to));
        self.emotes.pass(frames, cast);
    }
}
