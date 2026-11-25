//! A body's beats: where someone is put and which way they turn, a thing of
//! the place, an evolution, being seen or not. What someone does (an
//! animation, an emote, a move) is `acting`'s.

use engine::{BodyBeat, Direction, Play, Point, Who};

use super::{BESIDE, Writer, way_to};
use crate::sky::{HIDDEN, direction};

impl Writer<'_> {
    /// A body beat: put, turned, shown or hidden (the rest is `acting`'s).
    pub(super) fn body(&mut self, number: i32, word: Option<&str>, beat: &BodyBeat) {
        match beat {
            // Floating words (`Missed!`): Sky's scenes have none.
            BodyBeat::Popup { text, .. } => self.notes.push(format!("line {number}: the words {text:?} floating up")),
            BodyBeat::Put { who: Who::Thing { object, .. }, at, .. } => self.put_thing(object, at),
            BodyBeat::Anim { who: Who::Thing { object, .. }, play, .. } => self.thing_anim(number, *object, play),
            BodyBeat::ThingName { who: Who::Thing { .. }, .. } => {}
            // Sunk in the water: Sky draws its actors whole (R-DS-LOSS).
            BodyBeat::Sink { .. } => {}
            // Into the ground and out of it: no shadow taken away (R-DS-LOSS).
            BodyBeat::Shadow { .. } if matches!(word, Some("burrow" | "pop_out")) => {}
            // The engine's evolution (slowed, shining, growing): Sky's white flash instead, below.
            BodyBeat::Colour { .. } | BodyBeat::Scale { .. } if matches!(word, Some("evolve" | "transform")) => {}
            // Evolving: a shine, the screen white, the evolved one (an actor of its
            // own) put where the other stood, the other hidden, the screen back.
            BodyBeat::Disguise { who, like: Some(like) } if word == Some("evolve") => self.evolve(number, who, like),
            BodyBeat::Mimic { who, .. } => self.mimic(number, *who),
            // An unseen point (where a dodger stood): only its place, kept.
            BodyBeat::Put { who: who @ Who::Spot(_), at, .. } => {
                if let Some(point) = self.point(at) {
                    self.at.insert(*who, point);
                }
            }
            BodyBeat::Put { who, at, direction: way } => self.put(number, who, at, way),
            BodyBeat::PutNear { who, other, side } => self.put_near(number, *who, other, side),
            BodyBeat::Turn { who, direction: way } => self.turn(number, *who, *way),
            BodyBeat::TurnBy { who, eighths } => {
                let Some(actor) = self.who(number, *who) else { return };
                let now = self.facing.get(who).copied().unwrap_or(Direction::Down);
                let way = Direction::from_number(((now.number() as i32 - 1 + eighths.round() as i32).rem_euclid(8) + 1) as u16);
                self.facing.insert(*who, way);
                self.line(&format!("Turn2Direction<actor {actor}>(4, 10, {});", direction(way)));
            }
            BodyBeat::TurnAround(who) => {
                let Some(actor) = self.who(number, *who) else { return };
                let now = self.facing.get(who).copied().unwrap_or(Direction::Down);
                let way = Direction::from_number(((now.number() + 3) % 8) + 1);
                self.facing.insert(*who, way);
                self.line(&format!("SetDirection<actor {actor}>({});", direction(way)));
            }
            BodyBeat::LookAt { who, at } => {
                let (Some(actor), Some(to), Some(&from)) = (self.who(number, *who), self.point(at), self.at.get(who)) else { return };
                let way = way_to(from, to);
                self.facing.insert(*who, way);
                self.line(&format!("SetDirection<actor {actor}>({});", direction(way)));
            }
            BodyBeat::Face { who, other } => self.face(number, *who, *other),
            BodyBeat::Stop(_) | BodyBeat::Fidget { .. } => {}
            BodyBeat::Gone(who) => {
                let Some(actor) = self.who(number, *who) else { return };
                self.line(&format!("SetOutputAttribute<actor {actor}>({HIDDEN});"));
            }
            BodyBeat::Opacity { who, to, .. } => self.opacity(number, *who, *to),
            other => self.acting(number, word, other),
        }
    }

    /// The place's things: objects of the game in the scene's layout.
    fn put_thing(&mut self, object: &u16, at: &Point) {
        let Some(point) = self.point(at) else { return };
        if self.objects.contains_key(object) {
            let position = self.position(point);
            self.line(&format!("SetPositionMark<object OBJECT_N{object}>({position});"));
        } else {
            self.objects.insert(*object, point);
        }
    }

    /// A thing opened: its animation once (Sky's objects' table: 17); closed:
    /// its first picture held (9).
    fn thing_anim(&mut self, number: i32, object: u16, play: &Play) {
        if !self.objects.contains_key(&object) {
            self.problems.push(format!("line {number}: the thing {object} is not put in the scene"));
            return;
        }
        let set = match play {
            Play::Hold(_) => 9,
            Play::Loop => 1,
            _ => 17,
        };
        self.line(&format!("SetAnimation<object OBJECT_N{object}>({set});"));
    }

    /// Transformed: a shine, the screen white and back; Sky's actors keep
    /// their look (R-DS-LOSS).
    fn mimic(&mut self, number: i32, who: Who) {
        let Some(old) = self.who(number, who) else { return };
        self.line(&format!("SetEffect<actor {old}>(691, 3);"));
        self.line("Wait(40);");
        self.line("screen_WhiteOut(1, 30);");
        self.line("WaitScreenFade();");
        self.line(&format!("SetEffect<actor {old}>(EFFECT_NONE, 3);"));
        self.line("screen_FadeIn(1, 30);");
        self.line("WaitScreenFade();");
    }

    /// Put beside another, on that side, facing them.
    fn put_near(&mut self, number: i32, who: Who, other: &Who, side: &Direction) {
        let (Some(actor), Some(&(x, y))) = (self.who(number, who), self.at.get(other)) else { return };
        let right = matches!(side, Direction::Right | Direction::UpRight | Direction::DownRight);
        let point = (if right { x + BESIDE } else { x - BESIDE }, y);
        self.at.insert(who, point);
        let position = self.position(point);
        self.line(&format!("SetPositionMark<actor {actor}>({position});"));
        let facing = way_to(point, (x, y));
        self.facing.insert(who, facing);
        self.line(&format!("SetDirection<actor {actor}>({});", direction(facing)));
    }

    /// Turned a way: the way they first face, if the scene has not begun.
    fn turn(&mut self, number: i32, who: Who, way: Direction) {
        let Some(actor) = self.who(number, who) else { return };
        self.facing.insert(who, way);
        let facing = direction(way);
        let started = self.started;
        match self.actors.get_mut(&who).filter(|_| !started).and_then(|one| one.first.as_mut()) {
            Some(first) => first.2 = facing,
            None => {
                self.line(&format!("Turn2Direction<actor {actor}>(4, 10, {facing});"));
                self.wait_for(&actor);
            }
        }
    }

    /// Turned to another; to an unseen point (`use Move at x y`), its way
    /// from where he stands.
    fn face(&mut self, number: i32, who: Who, other: Who) {
        if let Who::Spot(_) = other {
            let Some(actor) = self.who(number, who) else { return };
            let (Some(&from), Some(&to)) = (self.at.get(&who), self.at.get(&other)) else { return };
            let way = way_to(from, to);
            self.facing.insert(who, way);
            self.line(&format!("Turn2Direction<actor {actor}>(4, 10, {});", direction(way)));
            self.wait_for(&actor);
            return;
        }
        let (Some(actor), Some(other_actor)) = (self.who(number, who), self.who(number, other)) else { return };
        if let (Some(&from), Some(&to)) = (self.at.get(&who), self.at.get(&other)) {
            self.facing.insert(who, way_to(from, to));
        }
        self.line(&format!("Turn2DirectionLives<actor {actor}>(4, 10, {other_actor});"));
        self.wait_for(&actor);
    }

    /// Shown where they stand, or hidden (put hidden before it appears:
    /// hidden from the start).
    fn opacity(&mut self, number: i32, who: Who, to: f32) {
        let Some(actor) = self.who(number, who) else { return };
        if to > 0.5 {
            let at = self.at.get(&who).copied().unwrap_or(self.camera);
            self.show_at(who, &actor, at);
        } else if self.actors.get(&who).is_some_and(|one| one.hidden && one.first.is_some()) {
            // Put, hidden, before it appears: hidden from the start.
        } else {
            self.line(&format!("SetOutputAttribute<actor {actor}>({HIDDEN});"));
        }
    }

    /// Someone put somewhere, facing a way: where they first stand, if the scene has not begun.
    fn put(&mut self, number: i32, who: &Who, at: &Point, way: &Direction) {
        let Some(actor) = self.who(number, *who) else { return };
        let Some(point) = self.point(at) else {
            self.problems.push(format!("line {number}: where {who:?} is put is not known before the film plays"));
            return;
        };
        self.at.insert(*who, point);
        self.facing.insert(*who, *way);
        let facing = direction(*way);
        match self.actors.get_mut(&self.became.get(who).copied().unwrap_or(*who)) {
            Some(one) if !self.started || (one.hidden && one.first.is_none()) => one.first = Some((point.0, point.1, facing)),
            _ => {
                let position = self.position(point);
                self.line(&format!("SetPositionMark<actor {actor}>({position});"));
                self.line(&format!("SetDirection<actor {actor}>({facing});"));
            }
        }
    }

    /// Evolving: a shine, the screen white, the evolved one (an actor of its own) put where the other stood, the other hidden, the screen back.
    fn evolve(&mut self, number: i32, who: &Who, like: &Who) {
        let (Some(old), Some(&at)) = (self.who(number, *who), self.at.get(who)) else { return };
        let Some(new) = self.who(number, *like) else { return };
        let facing = direction(self.facing.get(who).copied().unwrap_or(Direction::Down));
        let position = self.position(at);
        self.line(&format!("SetEffect<actor {old}>(691, 3);"));
        self.line("Wait(40);");
        self.line("screen_WhiteOut(1, 30);");
        self.line("WaitScreenFade();");
        self.line(&format!("SetEffect<actor {old}>(EFFECT_NONE, 3);"));
        self.line(&format!("SetOutputAttribute<actor {old}>({HIDDEN});"));
        self.line(&format!("SetPositionMark<actor {new}>({position});"));
        self.line(&format!("SetDirection<actor {new}>({facing});"));
        self.line(&format!("ResetOutputAttribute<actor {new}>({HIDDEN});"));
        self.line("screen_FadeIn(1, 30);");
        self.line("WaitScreenFade();");
        self.became.insert(*who, *like);
    }
}
