//! A body's beats: where someone is put and which way they turn, a thing of
//! the place, an evolution, being seen or not. What someone does (an
//! animation, an emote, a move) is `acting`'s.

use engine::{BodyBeat, Direction, Play, Point, Who};

use super::{BESIDE, Writer, names, way_to};
use crate::notes::{Fate, Kind};
use crate::sky::{HIDDEN, direction};

/// A fade at least so long blinks (shorter: a pop, as in the game's smoke).
const BLINKING: f32 = 12.0;

impl Writer<'_> {
    /// A body beat: put, turned, shown or hidden (the rest is `acting`'s).
    pub(super) fn body(&mut self, number: i32, word: Option<&str>, beat: &BodyBeat) {
        match beat {
            // Floating words (`Missed!`): Sky's scenes have none.
            BodyBeat::Popup { text, .. } => {
                self.note(number, Kind::Effect, Fate::LeftOut, format!("the words “{}” floating up", text.trim()), "nothing: Sky's scenes have none")
            }
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
                self.hide(*who, &actor);
            }
            BodyBeat::Opacity { who, to, frames } => self.opacity(number, *who, *to, frames.0),
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
            self.problem(number, Kind::Item, Fate::LeftOut, format!("the thing {object} is not put in the scene"), "nothing");
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
        // (To an unseen point, or to a thing: turned its way.)
        if matches!(other, Who::Spot(_)) || other.is_thing() {
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
    /// hidden from the start); fading (a while): blinking in or out as Sky
    /// fades its actors (`SetBlink`, `m01a10a`).
    fn opacity(&mut self, number: i32, who: Who, to: f32, frames: f32) {
        let Some(actor) = self.who(number, who) else { return };
        let blinks = self.started && frames >= BLINKING;
        if to > 0.5 {
            let at = self.at.get(&who).copied().unwrap_or(self.camera);
            self.show_at(who, &actor, at);
            if blinks {
                self.blink(&actor, frames, true);
            }
        } else if blinks && self.opening.is_none() {
            self.blink(&actor, frames, false);
            self.line(&format!("SetOutputAttribute<actor {actor}>({HIDDEN});"));
        } else if self.actors.get(&who).is_some_and(|one| one.hidden && one.first.is_some() && !one.seen) {
            // Put, hidden, before it appears: hidden from the start.
        } else {
            self.hide(who, &actor);
        }
    }

    /// A blink in so many frames, in three steps: shown 2 frames in 3, then
    /// 1 in 2, then 1 in 3 (out; the other way in), then steady. The film's
    /// wait for the fade counts its time.
    fn blink(&mut self, actor: &str, frames: f32, coming: bool) {
        let mut steps = [(2, 1), (1, 1), (1, 2)];
        if coming {
            steps.reverse();
        }
        let each = (frames / 3.0).round().max(2.0) as i32;
        for (shown, hidden) in steps {
            self.line(&format!("SetBlink<actor {actor}>({shown}, {hidden});"));
            self.line(&format!("Wait({each});"));
        }
        self.line(&format!("SetBlink<actor {actor}>(0, 0);"));
        self.ahead += (each * 3) as f32;
    }

    /// Hidden; in the scene's opening (nothing has happened yet), hidden
    /// from the start instead, what it did before left out: no one shows on
    /// the first frames to go at once (`gyarados put`, `gyarados dive`).
    fn hide(&mut self, who: Who, actor: &str) {
        let Some(start) = self.opening.filter(|start| *start <= self.body.len()) else {
            self.line(&format!("SetOutputAttribute<actor {actor}>({HIDDEN});"));
            return;
        };
        let lines: Vec<String> = self.body[start..].lines().map(str::to_string).collect();
        self.body.truncate(start);
        for line in lines {
            // (A block of others beside the script keeps its start.)
            let others = line.trim_start().starts_with("// dw-beside ") && line.split_whitespace().skip(3).any(|one| one != actor);
            if !names(&line, actor) || others {
                self.body.push_str(&line);
                self.body.push('\n');
            }
        }
        self.floating.retain(|one| one != actor);
        self.showing.retain(|one| one != actor);
        self.stand_after.retain(|one| one != actor);
        if let Some(one) = self.actors.get_mut(&self.became.get(&who).copied().unwrap_or(who)) {
            one.hidden = true;
        }
    }

    /// Someone put somewhere, facing a way: where they first stand, if the scene has not begun.
    fn put(&mut self, number: i32, who: &Who, at: &Point, way: &Direction) {
        let Some(actor) = self.who(number, *who) else { return };
        let Some(point) = self.point(at) else {
            self.problem(number, Kind::Logic, Fate::LeftOut, "where they are put is not known before the film plays", "left out");
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
                self.set_along(*who, point);
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
