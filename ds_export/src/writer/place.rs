//! The place and who is in it: the ground and its named spots, paths walked,
//! crowds, those who arrive and leave.

use engine::{Beat, BodyBeat, CrowdForm, CrowdShape, Direction, Exit, Gait, Num, Pace, PlaceBeat, Point, Spot, Who};

use super::{BESIDE, OFF_SCREEN, Writer, way_to};
use crate::notes::{Fate, Kind};
use crate::sky::{HIDDEN, speed};

impl Writer<'_> {
    /// A place beat: the ground, a spot named, someone arriving or leaving, a crowd.
    pub(super) fn place(&mut self, number: i32, word: Option<&str>, beat: &PlaceBeat) {
        match beat {
            // Over walls, through them, in the water, by a word or by nature
            // (`fly`, `swim`, a ghost): Sky's actors go anywhere a script sends them.
            PlaceBeat::Flies(_) | PlaceBeat::Lands(_) | PlaceBeat::Swims { .. } | PlaceBeat::Ghosts(_) | PlaceBeat::Hovers(_) => {}
            PlaceBeat::Ground { middle, .. } => {
                self.middle = Some((middle.x, middle.y));
                self.camera = (middle.x, middle.y);
                self.line("camera_SetMyself<performer 0>();");
                if self.tall {
                    let above = self.above((middle.x, middle.y));
                    let position = self.position(above);
                    self.line(&format!("camera2_SetPositionMark({position});"));
                }
            }
            // A decor's ways in and out (`south`, `west`): places named, nothing to write.
            PlaceBeat::Entrances(ways) => {
                for (name, at) in ways {
                    self.places.insert(name.clone(), (at.x, at.y));
                }
            }
            // A place named (`name rock 300 200`): read into positions, as a place found.
            PlaceBeat::Name { name, spot } => {
                if let Some(point) = self.spot(spot) {
                    self.places.insert(name.clone(), point);
                }
            }
            PlaceBeat::Find { var, spot, .. } => match self.spot(spot) {
                Some(point) => {
                    self.places.insert(var.clone(), point);
                }
                None => self.problem(number, Kind::Logic, Fate::LeftOut, "this spot isn't known on DS yet", "left out"),
            },
            PlaceBeat::Goes { who, points, gait } => self.walk(number, who, points, gait),
            // A crowd: each one a free actor, round the place, facing it
            // (a line, or a heap, as a row).
            PlaceBeat::Crowd { who, spot, form, .. } => self.crowd(number, word, who, spot, form),
            // Wandering: Sky's own stroll (CORO_MOVE_RANDOM), until another action.
            PlaceBeat::Roams { who, wide } => {
                let Some(actor) = self.who(number, *who) else { return };
                if *wide > 0.0 {
                    self.line(&format!("ExecuteCommon<actor {actor}>(CORO_MOVE_RANDOM, 0);"));
                } else if let Some(&at) = self.at.get(who) {
                    // Back to where it stood.
                    self.go(*who, &actor, at, "1", false);
                    self.wait_for(&actor);
                }
            }
            PlaceBeat::Arrives { who, to, gait, from, .. } => self.arrive(number, who, to, gait, from),
            PlaceBeat::Leaves { who, by, run } => {
                let Some(actor) = self.who(number, *who) else { return };
                let y = self.at.get(who).map_or(self.camera.1, |at| at.1);
                let right = match by {
                    Exit::Side { right } => *right,
                    Exit::Named(_) => self.at.get(who).is_none_or(|at| at.0 >= self.camera.0),
                };
                let to = (if right { self.camera.0 + OFF_SCREEN } else { self.camera.0 - OFF_SCREEN }, y);
                self.calm();
                self.go(*who, &actor, to, if *run { "2" } else { "1" }, false);
            }
            other => self.unexported(number, word, &Beat::Place(other.clone())),
        }
    }

    /// Someone arriving: from off the screen, on its side (or the way they came), to where they stop.
    fn arrive(&mut self, number: i32, who: &Who, to: &Point, gait: &Gait, from: &Option<Exit>) {
        let (Some(actor), Some(to)) = (self.who(number, *who), self.point(to)) else {
            self.problem(number, Kind::Logic, Fate::LeftOut, "where they arrive is not known before the film plays", "left out");
            return;
        };
        let shown = self.at.contains_key(who) && !self.actors.get(who).is_some_and(|one| one.hidden && one.first.is_none());
        if from.is_none() && shown {
            self.go(*who, &actor, to, if gait.run { "2" } else { "1" }, false);
            return;
        }
        let right = match from {
            Some(Exit::Side { right }) => *right,
            _ => to.0 >= self.camera.0,
        };
        let start = (if right { self.camera.0 + OFF_SCREEN } else { self.camera.0 - OFF_SCREEN }, to.1);
        self.show_at(*who, &actor, start);
        let pace = if gait.run { "2" } else { "1" };
        self.go(*who, &actor, to, pace, false);
    }

    /// A crowd: each one a free actor, round the place, facing it (a line, or a heap, as a row).
    fn crowd(&mut self, number: i32, word: Option<&str>, who: &[Who], spot: &Option<Spot>, form: &CrowdForm) {
        let Some((x, y)) = spot.as_ref().and_then(|spot| self.spot(spot)).or(self.middle) else { return };
        let many = who.len() as f32;
        for (at, one) in who.iter().enumerate() {
            let point = match form.shape {
                CrowdShape::Ring | CrowdShape::Flock => {
                    let radius = form.wide.unwrap_or((many * 28.0 / std::f32::consts::TAU).max(40.0));
                    let angle = std::f32::consts::TAU * at as f32 / many;
                    (x + angle.sin() * radius, y - angle.cos() * radius * 0.75)
                }
                _ => (x + (at as f32 - (many - 1.0) / 2.0) * BESIDE, y),
            };
            let facing = if matches!(form.shape, CrowdShape::Ring | CrowdShape::Flock) { way_to(point, (x, y)) } else { Direction::Down };
            let at = Point { x: Num::Lit(point.0), y: Num::Lit(point.1) };
            self.plain(number, word, &Beat::Body(BodyBeat::Put { who: *one, at, direction: facing }));
            if let Some(actor) = self.actors.get(one).filter(|actor| actor.hidden).map(|actor| format!("ACTOR_{}", actor.name)) {
                self.line(&format!("ResetOutputAttribute<actor {actor}>({HIDDEN});"));
            }
        }
    }

    /// A path walked, step by step, the followers after the leader.
    fn walk(&mut self, number: i32, who: &Who, points: &[Point], gait: &Gait) {
        let Some(actor) = self.who(number, *who) else { return };
        let mut pace = if gait.run { 2.0 } else { 1.0 };
        if let Pace::Times(times) = gait.pace {
            pace *= times;
        }
        self.calm();
        for point in points {
            let Some(point) = self.point(point) else {
                self.problem(number, Kind::Logic, Fate::LeftOut, "where they go is not known before the film plays", "left out");
                return;
            };
            self.go(*who, &actor, point, &speed(pace * 256.0, 256.0), false);
            for follower in self.followers.iter().filter(|(_, leader)| leader == who).map(|(one, _)| *one).collect::<Vec<_>>() {
                let Some(follower_actor) = self.who(number, follower) else { continue };
                self.line("Wait(16);");
                self.line(&format!("MovePositionLives<actor {follower_actor}>({}, {actor});", speed(pace * 256.0, 256.0)));
                self.moving(&follower_actor);
                let side = if self.at.get(&follower).is_some_and(|at| at.0 < point.0) { -BESIDE } else { BESIDE };
                self.at.insert(follower, (point.0 + side, point.1));
            }
            // Sky waits for each step of a path before the next.
            if points.len() > 1 {
                self.wait_for(&actor);
            }
        }
    }
}
