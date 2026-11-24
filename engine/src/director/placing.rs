//! What is found on the stage as it plays (see `PlaceBeat`).

use std::f32::consts::TAU;

use super::*;

/// The ways one faces (see `DIRECTION_NAMES`).
const DOWN: u16 = 1;
const RIGHT: u16 = 3;
const LEFT: u16 = 7;
/// The number a film puts its lines in.
pub const LINE: &str = "_line";

impl Director {
    fn set_point(&mut self, var: &str, at: Vec2) {
        self.vars.insert(format!("{var}_x"), at.x.round() as f64);
        self.vars.insert(format!("{var}_y"), at.y.round() as f64);
    }

    fn put(&mut self, who: Who, at: Vec2, direction: u16, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        self.happen(Beat::Body(BodyBeat::Put { who, at: at.into(), direction: Direction::from_number(direction) }), stage, focus, calls);
    }

    fn goes(&mut self, who: Who, points: &[Vec2], gait: Gait, stage: &mut Stage) {
        let way = stage.way_by(who, points);
        stage.pace(who, &way, gait);
        stage.direct(Beat::Motion(MotionBeat::Path { who, points: way, run: gait.run }), Vec2::ZERO);
    }

    /// Who arrives walks in from where the view does not go, on the ground;
    /// in a place with none, it appears in a puff of smoke.
    /// `from`: by that way in, from off the place, in file behind who comes
    /// in there too; the place's own ways in are taken first.
    fn comes_in(&mut self, who: Who, to: Vec2, gait: Gait, from: Option<Exit>, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        let right = to.x >= stage.place.middle.x;
        let look = if right { LEFT } else { RIGHT };
        let by = from.or_else(|| (!stage.place.entrances.is_empty()).then_some(Exit::Side { right }));
        if let Some(by) = by.filter(|_| !stage.place.flying.contains(&who))
            && let Some((start, door)) = stage.way_in(who, to, &by)
        {
            // (A way in far from the place's edge, in sight: from out of
            // sight to it first, by a way one can walk.)
            let start = match start == door && !stage.out_of_view(door, focus) {
                true => stage.way_out(door, right, focus).unwrap_or(start),
                false => start,
            };
            self.put(who, door, look, stage, focus, calls);
            let mut way = stage.way_by(who, &[to]);
            way.insert(0, door);
            if start != door
                && let Some(ground) = stage.place.ground.as_ref().filter(|_| !stage.no_collisions)
                && let Some(route) = ground.route(start, door)
            {
                way.splice(0..0, route);
            }
            if let Some(actor) = stage.cast.get_mut(&who) {
                (actor.position, actor.way.to) = (start, start);
            }
            stage.pace(who, &way, gait);
            stage.direct(Beat::Motion(MotionBeat::Path { who, points: way, run: gait.run }), Vec2::ZERO);
            return;
        }
        if stage.place.flying.contains(&who) {
            self.put(who, stage.off_view(focus, right, to.y), look, stage, focus, calls);
            stage.direct(Beat::Motion(MotionBeat::Walk { who, at: to.into(), run: false }), focus);
            return;
        }
        let start = if stage.place.ground.is_some() { stage.way_out(to, right, focus) } else { Some(stage.off_side(right)) };
        let crowded = stage.crowded(Some(who), false);
        let start = start.filter(|start| crowded.as_ref().is_none_or(|ground| ground.route(*start, to).is_some()));
        match start {
            Some(start) => {
                self.put(who, start, look, stage, focus, calls);
                self.goes(who, &[to], gait, stage);
            }
            None => {
                let smoke = ScreenBeat::Particles { kind: ParticleKind::Smoke, on: Cam::at(to.round()), count: 30, colour: None };
                self.happen(Beat::Screen(smoke), stage, focus, calls);
                self.put(who, to, DOWN, stage, focus, calls);
            }
        }
    }

    pub(super) fn happen_place(&mut self, beat: PlaceBeat, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        stage.place.line = self.vars.get(LINE).copied().unwrap_or(0.0) as i32;
        match beat {
            PlaceBeat::Ground { middle, ground } => {
                stage.place.middle = middle;
                stage.place.ground = ground;
                stage.place.flying.clear();
                stage.place.swimmers.clear();
                stage.place.swimming.clear();
                stage.place.ghosts.clear();
                stage.place.hovering.clear();
                stage.place.entrances.clear();
                stage.place.leaving.clear();
                stage.place.entering.clear();
            }
            PlaceBeat::Flies(who) => stage.place.flying.extend(who),
            PlaceBeat::Lands(who) => {
                stage.place.flying.retain(|one| !who.contains(one));
                stage.place.swimmers.retain(|one, _| !who.contains(one));
                stage.place.ghosts.retain(|one| !who.contains(one));
                stage.place.hovering.retain(|one| !who.contains(one));
            }
            PlaceBeat::Swims { depth, who } => stage.place.swimmers.extend(who.into_iter().map(|one| (one, depth))),
            PlaceBeat::Ghosts(who) => stage.place.ghosts.extend(who),
            PlaceBeat::Hovers(who) => stage.place.hovering.extend(who),
            PlaceBeat::Name { name, spot } => {
                let at = stage.find(None, &spot, false, true);
                stage.place.spots.insert(name, at);
            }
            PlaceBeat::Find { var, who, spot, apart, exact } => {
                let at = stage.find(who, &spot, apart, exact);
                self.set_point(&var, at);
            }
            PlaceBeat::Spread { who, picture } => {
                let middle = stage.place.middle;
                let count = who.len() as f32;
                let mut lowest = stage
                    .cast
                    .iter()
                    .filter(|(one, _)| matches!(one, Who::Character(_) | Who::Monster(_) | Who::Another { .. }))
                    .map(|(_, actor)| actor.position.y)
                    .fold(f32::MIN, f32::max);
                for (place, one) in who.iter().enumerate() {
                    let shift = (place as f32 - (count - 1.0) / 2.0) * 44.0;
                    let look = if who.len() == 1 {
                        DOWN
                    } else if shift < 0.0 {
                        RIGHT
                    } else {
                        LEFT
                    };
                    let wanted = Vec2::new((middle.x + shift).floor(), middle.y + if place % 2 == 1 { 6.0 } else { 0.0 });
                    let there = stage.crowded(Some(*one), false).map_or(wanted, |ground| ground.free(wanted));
                    self.put(*one, there, look, stage, focus, calls);
                    lowest = lowest.max(there.y);
                }
                // (The view on them all, none under the dialogue box or its
                // portrait; a picture is seen as the catalogue has it.)
                let lowest = if lowest == f32::MIN { middle.y } else { lowest };
                let y = if picture { middle.y } else { (middle.y - 8.0).max(lowest + 10.0) };
                stage.direct(Beat::View(ViewBeat::Camera(Some(Cam::at(Vec2::new(middle.x, y))))), focus);
            }
            PlaceBeat::Goes { who, points, gait } => self.goes(who, &points.iter().map(Point::vec2).collect::<Vec<Vec2>>(), gait, stage),
            PlaceBeat::Arrives { who, to, gait, placing, from } => match (placing, stage.cast.contains_key(&who), stage.place.ground.is_some()) {
                (true, true, true) => self.goes(who, &[to.vec2()], gait, stage),
                (true, true, false) => self.put(who, to.vec2(), DOWN, stage, focus, calls),
                _ => self.comes_in(who, to.vec2(), gait, from, stage, focus, calls),
            },
            PlaceBeat::Leaves { who, by, run } => {
                if !stage.leave(who, &by, run, focus) {
                    let right = match &by {
                        Exit::Side { right } => *right,
                        Exit::Named(_) => stage.cast.get(&who).is_some_and(|actor| actor.position.x >= stage.place.middle.x),
                    };
                    // (No way out it can walk to: out of sight by one it can,
                    // and gone there; with none, it fades where it is. Never
                    // through a wall.)
                    let at = stage.cast.get(&who).map(|actor| actor.position);
                    let hid = match (at, stage.place.ground.is_some()) {
                        (Some(at), true) => stage.way_out(at, right, focus),
                        _ => Some(stage.off_side(right)),
                    };
                    match hid {
                        Some(out) => {
                            let way = stage.way_by(who, &[out]);
                            stage.direct(Beat::Motion(MotionBeat::Path { who, points: way, run }), focus);
                            stage.place.leaving.insert(who, out);
                            if let Some(actor) = stage.cast.get_mut(&who) {
                                actor.habits.vanishes = true;
                            }
                        }
                        None => _ = stage.direct(Beat::Body(BodyBeat::Opacity { who, to: 0.0, frames: Frames(30.0) }), focus),
                    }
                }
            }
            PlaceBeat::Entrances(ways) => stage.place.entrances = ways,
            PlaceBeat::Round { who, spot } => {
                if let Some(at) = stage.point(Some(who), &spot) {
                    stage.round_check(who, at);
                }
            }
            PlaceBeat::AllLook { way, at } => {
                for who in stage.bodies_here().into_iter().filter(|who| Some(*who) != at) {
                    let beat = match (way, at) {
                        (Some(direction), _) => BodyBeat::Turn { who, direction },
                        (None, Some(other)) => BodyBeat::Face { who, other },
                        _ => continue,
                    };
                    self.happen(Beat::Body(beat), stage, focus, calls);
                }
            }
            PlaceBeat::Crowd { who, spot, walking, form } => {
                let asked = spot.as_ref().and_then(|spot| stage.point(None, spot)).unwrap_or(stage.place.middle - Vec2::Y * 10.0);
                let (at, spots) = match form.shape {
                    CrowdShape::Ring => {
                        let (at, wide) = stage.crowd_round(&who, asked);
                        let wide = form.wide.unwrap_or(wide) * if form.tight { 0.5 } else { 1.0 };
                        let spots = (0..who.len())
                            .map(|place| {
                                let angle = place as f32 * TAU / who.len() as f32;
                                at + Vec2::new(angle.cos(), angle.sin() * 0.6) * wide
                            })
                            .collect();
                        (at, spots)
                    }
                    _ => (asked, stage.crowd_shape(asked, who.len(), &form)),
                };
                let gait = Gait { run: form.run, pace: Pace::Usual };
                // (From a side of the view: all of them out of sight that way,
                // each keeping its place in the crowd, straight to it.)
                let way = form.from.map(|way| crate::stage::step_of(way.number()).normalize());
                let start = way.map(|way| stage.off_view_along(focus, &spots, way));
                for (place, one) in who.iter().enumerate() {
                    let wanted = spots.get(place).copied().unwrap_or(at);
                    let there = stage.crowded(Some(*one), false).map_or(wanted, |ground| ground.free(wanted));
                    let facing = match way {
                        Some(way) => crate::stage::towards(Vec2::ZERO, -way).unwrap_or(LEFT),
                        None if there.x < at.x => RIGHT,
                        None => LEFT,
                    };
                    match (walking, way.zip(start)) {
                        (true, Some((way, far))) => {
                            self.put(*one, there + way * far, facing, stage, focus, calls);
                            stage.pace(*one, &[there], gait);
                            stage.direct(Beat::Motion(MotionBeat::Walk { who: *one, at: there.into(), run: form.run }), focus);
                        }
                        (true, None) => self.comes_in(*one, there, gait, None, stage, focus, calls),
                        (false, _) => self.put(*one, there, facing, stage, focus, calls),
                    }
                }
            }
            PlaceBeat::Flock { who, to, run } => {
                let standing: Vec<(Who, Vec2)> = who.iter().filter_map(|one| stage.cast.get(one).map(|actor| (*one, actor.position))).collect();
                let middle = standing.iter().map(|(_, at)| *at).sum::<Vec2>() / standing.len().max(1) as f32;
                let goal = stage.point(None, &to).unwrap_or(middle);
                for (one, at) in standing {
                    let wanted = at + goal - middle;
                    let there = match stage.no_collisions {
                        true => wanted,
                        false => stage.place.ground.as_ref().map_or(wanted, |ground| ground.free(wanted)),
                    };
                    stage.direct(Beat::Motion(MotionBeat::Walk { who: one, at: there.into(), run }), focus);
                }
            }
            PlaceBeat::Gathers { var, spot, step, who } => {
                let at = stage.find(None, &spot, false, true);
                let spots = stage.spots_around(at, who.len(), step);
                if spots.len() < who.len() {
                    stage.note_room(spots.len(), who.len());
                }
                for (place, one) in who.iter().enumerate() {
                    // (One the place has no room for: its point is where they
                    // gather, so a line naming it still reads; no body put.)
                    let Some(&there) = spots.get(place) else {
                        self.set_point(&format!("{var}{place}"), at);
                        continue;
                    };
                    self.set_point(&format!("{var}{place}"), there);
                    if let Some(one) = one {
                        // (Most look where the view looks, at what goes on; a
                        // few at a neighbour.)
                        let aside = crate::stage::scatter(place as i32, 7, 4);
                        let side = crate::stage::scatter(place as i32, 7, 5).signum();
                        let looked = if aside > 0.6 { there + Vec2::new(side * 40.0, 10.0) } else { focus };
                        let direction = crate::stage::towards(there, looked).unwrap_or(if there.x < at.x { RIGHT } else { LEFT });
                        self.put(*one, there, direction, stage, focus, calls);
                    }
                }
            }
            PlaceBeat::Advance { who, by } => {
                for one in who {
                    let Some(start) = stage.cast.get(&one).map(|actor| actor.position) else {
                        continue;
                    };
                    let goal = start + by;
                    if stage.no_collisions || stage.place.ground.as_ref().is_none_or(|ground| !ground.blocked(goal, 1)) {
                        stage.direct(Beat::Motion(MotionBeat::Walk { who: one, at: goal.into(), run: true }), focus);
                    }
                }
            }
            PlaceBeat::Seats { var, who, seats } => {
                let ground = stage.place.ground.clone().filter(|_| !stage.no_collisions);
                let mut kept: Vec<(Who, Vec2)> = Vec::new();
                for one in who {
                    let start = stage.cast.get(&one).map(|actor| actor.position);
                    let reachable = |seat: Vec2| ground.as_ref().zip(start).is_none_or(|(ground, start)| ground.route(start, seat).is_some());
                    if let Some(seat) = seats.iter().copied().find(|seat| kept.iter().all(|(_, other)| seat.distance(*other) >= 24.0) && reachable(*seat)) {
                        kept.push((one, seat));
                    }
                }
                let table = kept.iter().map(|(_, seat)| *seat).sum::<Vec2>() / kept.len().max(1) as f32;
                self.set_point(&var, table);
                for (one, seat) in kept {
                    self.goes(one, &[seat], Gait::WALK, stage);
                }
            }
            PlaceBeat::Shot { close, who } => {
                let close = stage.view.shot_close(close, who);
                stage.direct(Beat::View(ViewBeat::Zoom { to: close, frames: Frames(40.0) }), focus);
            }
            PlaceBeat::Roams { who, wide } => {
                if let Some(radius) = stage.room_to_roam(who, wide) {
                    stage.direct(Beat::Motion(MotionBeat::Wander { who, radius }), focus);
                }
            }
        }
    }
}
