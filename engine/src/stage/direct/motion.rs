//! The motion beats the stage plays: walks, glides, ways, who follows whom.

use super::*;

impl Stage {
    pub(in crate::stage) fn direct_motion(&mut self, beat: MotionBeat) -> Option<MotionBeat> {
        match beat {
            MotionBeat::Go { who, at, frames, slide } => self.with(who, |actor| actor.go(at.vec2(), frames.0, slide)),
            MotionBeat::Speed { who, pace } => self.with(who, |actor| actor.way.pace = pace.max(0.1)),
            MotionBeat::Orbit { who, at, turns, frames } => self.with(who, |actor| {
                let center = at.vec2();
                let from = actor.position - center;
                let radius = if from.length() < 1.0 { 24.0 } else { from.length() };
                let frames = frames.0.max(1.0);
                let speed = turns * std::f32::consts::TAU / frames;
                actor.habits.orbit = Some(Orbit { center, radius, angle: from.y.atan2(from.x), speed, left: frames, around: None });
            }),
            MotionBeat::OrbitAround { who, other, turns, frames } => {
                if let Some(there) = position(self, other) {
                    self.direct_motion(MotionBeat::Orbit { who, at: there.into(), turns, frames });
                    self.with(who, |actor| {
                        if let Some(orbit) = &mut actor.habits.orbit {
                            orbit.around = Some(other);
                        }
                    });
                }
            }
            MotionBeat::Resume(who) => self.with(who, |actor| {
                if let Some(Halted { to, route, running, then_face }) = actor.way.halted.take() {
                    (actor.way.route, actor.way.running, actor.way.then_face) = (route, running, then_face);
                    actor.walk_to(to, running);
                }
            }),
            MotionBeat::Step { who, dx, dy, run } => self.with(who, |actor| actor.walk_to(actor.position + Vec2::new(dx, dy), run)),
            MotionBeat::Follow { who, leader, gap } => self.with(who, |actor| {
                actor.habits.follows = leader;
                actor.habits.follow_gap = gap.unwrap_or(crate::stage::FOLLOW_GAP);
            }),
            MotionBeat::Walk { who, at, run } => self.with(who, |actor| actor.walk_to(at.vec2(), run)),
            MotionBeat::Path { who, points, run } => self.with(who, |actor| {
                let mut points: VecDeque<Vec2> = points.into();
                if let Some(first) = points.pop_front() {
                    (actor.way.route, actor.way.running) = (points, run);
                    actor.walk_to(first, run);
                }
            }),
            MotionBeat::Wander { who, radius } => {
                self.with(who, |actor| actor.habits.wander = (radius > 0.0).then_some(Wandering { home: actor.position, radius }))
            }
            MotionBeat::Flee { who, from, distance } => {
                if let Some(there) = position(self, from) {
                    self.with(who, |actor| {
                        let away = (actor.position - there).try_normalize().unwrap_or(Vec2::X);
                        actor.walk_to(actor.position + away * distance, true);
                    });
                }
            }
            MotionBeat::Chase { who, other } => {
                if let Some(there) = position(self, other) {
                    self.with(who, |actor| {
                        let from = (actor.position - there).try_normalize().unwrap_or(Vec2::X);
                        actor.walk_to(there + from * 26.0, true);
                        actor.way.then_face = Some(there);
                    });
                }
            }
            MotionBeat::LineUp { who, at, gap } => {
                for (place, who) in who.into_iter().enumerate() {
                    let slot = at + gap * place as f32;
                    self.with(who, |actor| {
                        actor.walk_to(slot, false);
                        actor.way.then_face = Some(slot + Vec2::Y * 10.0);
                    });
                }
            }
            MotionBeat::Spacing(on) => self.spacing = on,
            // (Its way to begin with: from the leader back through the others
            // where they stand.)
            MotionBeat::Train { gap, who } => {
                let way: Vec<Vec2> = who.iter().filter_map(|who| self.cast.get(who).map(|actor| actor.position)).collect();
                self.train = (who.len() >= 2).then_some(Train { gap, who, way });
            }
            MotionBeat::Mirror { who, model } => {
                let from = model.and_then(|model| self.cast.get(&model)).map(|actor| actor.position);
                self.with(who, |actor| actor.habits.mirrors = model.zip(from).map(|(model, last)| Mirroring { model, last }));
            }
            // (No collisions: straight there.)
            MotionBeat::WalkTo { who, at, run } if self.no_collisions => {
                self.direct_motion(MotionBeat::Path { who, points: vec![at.vec2()], run });
            }
            MotionBeat::WalkTo { who, at, run } => {
                let solids: Vec<Vec2> =
                    self.cast.iter().filter(|(other, actor)| **other != who && actor.habits.solid).map(|(_, actor)| actor.position).collect();
                let from = self.cast.get(&who).map(|actor| actor.position);
                // (Round the place's walls too, if there is a way so.)
                let walls: Vec<Rect> = self.play.walls.iter().chain(&self.play.place_walls).copied().collect();
                let way = from.and_then(|from| way_round(&walls, &solids, from, at.vec2()).or_else(|| way_round(&self.play.walls, &solids, from, at.vec2())));
                if let Some(points) = way {
                    self.direct_motion(MotionBeat::Path { who, points, run });
                }
            }
            MotionBeat::Approach { who, other, frames } => {
                if let Some(there) = position(self, other) {
                    self.with(who, |actor| {
                        // (Beside it, on the side it comes from.)
                        let from = (actor.position - there).try_normalize().unwrap_or(Vec2::X);
                        actor.go(there + from * 26.0, frames.0, false);
                    });
                }
            }
            // (The game takes away what showed who left, see `stage_actors`.)
            MotionBeat::KeepOnly(who) => self.cast.retain(|one, _| who.contains(one)),
            other => return Some(other),
        }
        None
    }
}
