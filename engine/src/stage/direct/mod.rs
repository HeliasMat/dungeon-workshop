//! The beats the stage plays, by what they change: here those of a body;
//! `motion`, of where they go; `screen`, of the light, the screen and the
//! player.

use super::*;

mod motion;
mod screen;

/// Where someone stands, if it is there.
fn position(stage: &Stage, who: Who) -> Option<Vec2> {
    stage.cast.get(&who).map(|actor| actor.position)
}

impl Stage {
    /// Plays a beat that only changes the stage (`focus`: where the view
    /// looks now); gives back those it does not play.
    pub fn direct(&mut self, beat: Beat, focus: Vec2) -> Option<Beat> {
        match beat {
            Beat::Body(beat) => self.direct_body(beat, focus).map(Beat::Body),
            Beat::Motion(beat) => self.direct_motion(beat).map(Beat::Motion),
            Beat::View(beat) => self.direct_view(beat, focus).map(Beat::View),
            Beat::Screen(beat) => self.direct_screen(beat, focus).map(Beat::Screen),
            Beat::Play(beat) => self.direct_play(beat, focus).map(Beat::Play),
            Beat::Node(beat) => self.direct_node(beat).map(Beat::Node),
            other => Some(other),
        }
    }

    fn direct_body(&mut self, beat: BodyBeat, focus: Vec2) -> Option<BodyBeat> {
        // (A turn told to go slowly: made, then undone to go a way at a time.)
        let turner = match &beat {
            BodyBeat::Turn { who, .. }
            | BodyBeat::Face { who, .. }
            | BodyBeat::LookAt { who, .. }
            | BodyBeat::TurnAround(who)
            | BodyBeat::TurnBy { who, .. } => Some(*who),
            _ => None,
        };
        let Some(who) = turner else {
            return self.direct_body_now(beat, focus);
        };
        // (Turned at once: a slow turn going stops there.)
        if self.cast.get(&who).is_none_or(|actor| actor.air.slow_turn.is_none()) {
            self.with(who, |actor| actor.air.turning = None);
            return self.direct_body_now(beat, focus);
        }
        let before = self.cast.get(&who).map_or(1, |actor| actor.direction);
        let left = self.direct_body_now(beat, focus);
        self.with(who, |actor| actor.turned_from(before));
        left
    }

    fn direct_body_now(&mut self, beat: BodyBeat, _focus: Vec2) -> Option<BodyBeat> {
        match beat {
            BodyBeat::TurnSlow { who, every } => self.with(who, |actor| actor.air.slow_turn = Some(every.0)),
            BodyBeat::Turn { who, direction } => self.with(who, |actor| actor.direction = direction.number()),
            BodyBeat::Views { who, ways } => self.paint(who, ways.into_iter().map(Direction::number).collect()),
            BodyBeat::Anim { who, anim, play } => {
                self.with(who, |actor| actor.cue(Cue::Anim { group: anim.index(), play }));
                match play {
                    Play::Hold(step) => self.framed(who, Some(step)),
                    Play::Times(_) => {}
                    _ => self.framed(who, None),
                }
            }
            BodyBeat::ThingName { who, name } => self.marked(who, |marks| marks.name = name),
            BodyBeat::Takeable { who, on } => self.marked(who, |marks| marks.takeable = on),
            BodyBeat::Shine { who, on } => self.marked(who, |marks| marks.shines = on),
            BodyBeat::Large(who) => self.words.large = who,
            BodyBeat::FishLine { who, to, at, cast } => self.fish_line(who, to, at, cast),
            BodyBeat::Pose { who, pose } => self.with(who, |actor| actor.cue(Cue::Pose(pose))),
            BodyBeat::Rise { who, height, frames } => self.with(who, |actor| actor.air.rise = Some(Tween::new(height, frames.0.max(1.0)))),
            BodyBeat::Attr { who, bits } => self.with(who, |actor| actor.attributes = bits),
            BodyBeat::Tremble { who, frames, strength } => self.with(who, |actor| (actor.air.tremble, actor.air.tremble_by) = (frames.0, strength)),
            BodyBeat::Slow { who, by } => self.with(who, |actor| actor.looks.time = by),
            BodyBeat::Freeze { who, on } => self.with(who, |actor| actor.looks.frozen = on),
            BodyBeat::InFront { who, of } => self.with(who, |actor| actor.looks.in_front_of = of),
            BodyBeat::Hop { who, at, height, frames } => self.with(who, |actor| {
                actor.go(at.vec2(), frames.0, true);
                // (From up there, it keeps that height, as a throw.)
                actor.air.lift = actor.air.height;
                actor.air.arc = Some((height, frames.0.max(1.0)));
            }),
            // (A jump where it stands: its arc, with no way to go.)
            BodyBeat::Jump { who, height, frames } => self.with(who, |actor| {
                actor.go(actor.position, frames.0, true);
                actor.air.arc = Some((height, frames.0.max(1.0)));
            }),
            BodyBeat::Spin { who, turns, frames, rotate } => self.with(who, |actor| actor.air.spin = Some(Spin::new(turns, frames.0, rotate))),
            BodyBeat::Opacity { who, to, frames } => self.with(who, |actor| match frames > Frames(0.0) {
                true => actor.looks.fading = Some(Tween::new(to.clamp(0.0, 1.0), frames.0)),
                false => (actor.looks.opacity, actor.looks.fading) = (to.clamp(0.0, 1.0), None),
            }),
            BodyBeat::Bar { who, share } => self.with(who, |actor| actor.bar = share),
            BodyBeat::Fidget { who, on } => self.with(who, |actor| actor.habits.fidgets = on),
            BodyBeat::Bounce { who, height, frames } => self.with(who, |actor| actor.air.bounce = Swing::new(height, frames.0)),
            BodyBeat::Layer { who, place } => {
                use crate::stage::attribute::{OVER, UNDER};
                let set = match place {
                    Stacked::Over => OVER,
                    Stacked::Under => UNDER,
                    Stacked::Among => 0,
                };
                self.with(who, |actor| actor.attributes = (actor.attributes & !(OVER | UNDER)) | set);
            }
            BodyBeat::Blink { who, on } => {
                use crate::stage::attribute::BLINKING;
                self.with(who, |actor| actor.attributes = if on { actor.attributes | BLINKING } else { actor.attributes & !BLINKING });
            }
            BodyBeat::Scale { who, to, frames } => self.with(who, |actor| match frames > Frames(0.0) {
                true => actor.looks.scaling = Some(Tween::new(to, frames.0)),
                false => (actor.looks.scale, actor.looks.scaling) = (to, None),
            }),
            BodyBeat::Shadow { who, on } => self.with(who, |actor| actor.shadowless = !on),
            BodyBeat::Flip { who, on } => self.with(who, |actor| actor.flipped = on),
            BodyBeat::Sink { who, depth, sand } => self.with(who, |actor| {
                actor.sunk = depth;
                actor.in_sand = sand;
            }),
            // (As put, where it stands, the way it faces.)
            BodyBeat::Reset(who) => self.with(who, |actor| *actor = Actor::at(actor.position, actor.direction)),
            BodyBeat::Status { who, icon, add } => {
                let here = self.cast.contains_key(&who);
                self.emotes.status(who, icon, here, add);
            }
            BodyBeat::Carry { what, by, hands } => {
                self.with(what, |actor| {
                    (actor.habits.carried_by, actor.habits.held) = (by, hands && by.is_some());
                    if by.is_none() {
                        actor.air.height = 0.0;
                    }
                });
                self.handing.retain(|handing| handing.what != what);
            }
            BodyBeat::Ride { who, on, high, beside } => self.with(who, |actor| {
                (actor.habits.carried_by, actor.habits.held) = (on, false);
                actor.habits.rides = on.map(|_| (high, beside));
                if on.is_none() {
                    actor.air.height = 0.0;
                }
            }),
            BodyBeat::Throw { who, at, height, frames } => {
                if let Some(there) = self.cast.get(&at).map(|actor| actor.position) {
                    self.with(who, |actor| {
                        actor.go(there, frames.0, true);
                        // (Thrown from up there, it flies at that height.)
                        actor.air.lift = actor.air.height;
                        actor.air.arc = Some((height, frames.0.max(1.0)));
                    });
                }
            }
            BodyBeat::Fall { who, height, frames } => self.with(who, |actor| {
                actor.air.height = height;
                actor.air.rise = Some(Tween::new(0.0, frames.0.max(1.0)));
            }),
            BodyBeat::Float { who, height, frames } => self.with(who, |actor| actor.air.float = Swing::new(height, frames.0)),
            BodyBeat::Colour { who, colour, frames } => self.with(who, |actor| match frames > Frames(0.0) {
                true => actor.looks.colouring = Some(Tween::new(colour, frames.0)),
                false => (actor.colour, actor.looks.colouring) = (colour, None),
            }),
            // (From oneself: back the way one looks, still looking there.)
            BodyBeat::Recoil { who, from, distance } if who == from => self.with(who, |actor| {
                let away = -step_of(actor.direction).normalize_or(Vec2::X);
                actor.go(actor.position + away * distance, 10.0, true);
                (actor.air.arc, actor.air.tremble) = (Some((4.0, 10.0)), 14.0);
                actor.hits += 1;
                actor.cue(Cue::Hurt);
            }),
            BodyBeat::Recoil { who, from, distance } => {
                if let Some(there) = position(self, from) {
                    self.with(who, |actor| {
                        let away = (actor.position - there).try_normalize().unwrap_or(Vec2::X);
                        actor.direction = towards(actor.position, there).unwrap_or(actor.direction);
                        actor.go(actor.position + away * distance, 10.0, true);
                        (actor.air.arc, actor.air.tremble) = (Some((4.0, 10.0)), 14.0);
                        actor.hits += 1;
                        actor.cue(Cue::Hurt);
                    });
                }
            }
            BodyBeat::LookAt { who, at } => self.with(who, |actor| actor.direction = towards(actor.position, at.vec2()).unwrap_or(actor.direction)),
            BodyBeat::Face { who, other } => {
                if let Some(there) = position(self, other) {
                    self.with(who, |actor| actor.direction = towards(actor.position, there).unwrap_or(actor.direction));
                }
            }
            BodyBeat::Effect { who, id, frames, add } => self.effect_with(who, id.0, frames.map(Frames::get), add),
            BodyBeat::TurnAround(who) => self.with(who, |actor| actor.direction = turned(actor.direction, 4)),
            BodyBeat::TurnBy { who, eighths } => {
                self.with(who, |actor| actor.direction = turned(actor.direction, (eighths.round() as i32).rem_euclid(8) as u16))
            }
            BodyBeat::Stop(who) => self.with(who, |actor| {
                // (Where it was going is kept, for `MotionBeat::Resume`.)
                if actor.way.left > 0.0 && !actor.way.slide || !actor.way.route.is_empty() {
                    actor.way.halted =
                        Some(Halted { to: actor.way.to, route: actor.way.route.clone(), running: actor.way.running, then_face: actor.way.then_face });
                }
                (actor.way.to, actor.way.left, actor.way.route) = (actor.position, 0.0, VecDeque::new());
                (actor.habits.orbit, actor.habits.wander, actor.habits.follows) = (None, None, None);
                actor.end_spin();
                (actor.air.float, actor.way.then_face) = (None, None);
                (actor.air.bounce, actor.habits.fidgets) = (None, false);
            }),
            BodyBeat::Popup { who, text, colour } => {
                if let Some(actor) = self.cast.get(&who) {
                    self.emotes.popup(actor, text, colour);
                }
            }
            other => return Some(other),
        }
        None
    }
}
