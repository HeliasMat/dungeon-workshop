//! The ways someone goes about a place: round the walls and those standing
//! there, at a pace, out by a side or a named way, in from one.

use super::*;

/// The ground one stands on furthest on a side, row by row: the place's edge
/// there.
fn edge(ground: &Ground, right: bool) -> Vec<Vec2> {
    let mut found = Vec::new();
    for (y, row) in ground.rows().iter().enumerate() {
        let mut tiles = (0..row.len()).map(|x| Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * TILE).filter(|at| !ground.blocked(*at, 1));
        let outer = if right { tiles.next_back() } else { tiles.next() };
        found.extend(outer);
    }
    let furthest = found.iter().map(|at| if right { at.x } else { -at.x }).fold(f32::MIN, f32::max);
    found.retain(|at| furthest - if right { at.x } else { -at.x } <= 2.0 * TILE);
    found
}

impl Stage {
    /// The way by these points round the walls and those standing there.
    /// (With no collisions, nothing is crowded: straight by the points.)
    pub(crate) fn way_by(&mut self, who: Who, points: &[Vec2]) -> Vec<Vec2> {
        let (Some(crowded), Some(mut here), Some(ground)) =
            (self.crowded(Some(who), false), self.cast.get(&who).map(|actor| actor.position), self.place.ground.clone())
        else {
            return points.to_vec();
        };
        let mut way = Vec::new();
        let flying = self.place.flying.contains(&who);
        for asked in points {
            let goal = *asked;
            if ground.blocked(goal, 0) && !flying {
                self.note(told!("{} goes into a wall at {}: choose another spot", who, point(goal)), false);
            }
            // (Round those who stand there; through them when there is no other way.)
            let found = match crowded.blocked(here, 0) || crowded.blocked(goal, 0) || flying {
                true => Some(vec![goal]),
                false => crowded.route(here, goal).or_else(|| ground.route(here, goal)),
            };
            way.extend(found.unwrap_or_else(|| {
                self.note(told!("{} cannot go to {}: the way is blocked", who, point(goal)), false);
                vec![goal]
            }));
            here = goal;
        }
        way
    }

    /// How fast someone goes along a way: told of it, or back to a walker's.
    pub(crate) fn pace(&mut self, who: Who, way: &[Vec2], Gait { run, pace }: Gait) {
        let much = match pace {
            Pace::Usual => {
                if self.place.paced.remove(&who) {
                    self.direct_motion(MotionBeat::Speed { who, pace: WALK });
                }
                return;
            }
            Pace::Times(much) => much,
            Pace::Seconds(seconds) => {
                let Some(mut here) = self.cast.get(&who).map(|actor| actor.position) else {
                    return;
                };
                let mut length = 0.0;
                for point in way {
                    (length, here) = (length + here.distance(*point), *point);
                }
                let each = length / (seconds * 60.0).max(1.0);
                each * if run { WALK / RUN } else { 1.0 } / WALK
            }
        };
        self.place.paced.insert(who);
        self.direct_motion(MotionBeat::Speed { who, pace: (much * WALK).max(0.1) });
    }

    /// The nearest ground to a point that the view does not show, that side
    /// first; none where there is no such ground.
    pub(crate) fn way_out(&self, at: Vec2, right: bool, seen: Vec2) -> Option<Vec2> {
        /// How many of the nearest are tried, the way from each to there.
        const TRIED: usize = 48;
        let ground = self.place.ground.as_ref()?;
        // (Out of what the view shows now, not of the place's middle: one
        // put there is not seen to come out of nowhere.)
        let half = UNSEEN / self.view.closeness().max(0.1);
        let mut unseen = Vec::new();
        for (y, row) in ground.rows().iter().enumerate() {
            for (x, solid) in row.iter().enumerate() {
                let tile = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * TILE;
                if !solid && ((tile.x - seen.x).abs() > half.x || (tile.y - seen.y).abs() > half.y) {
                    unseen.push(tile);
                }
            }
        }
        // (The nearest it can walk from, on that side first.)
        let reached = |spots: Vec<Vec2>| {
            let mut spots = spots;
            spots.sort_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at)));
            spots.into_iter().take(TRIED).find(|start| self.no_collisions || ground.route(*start, at).is_some())
        };
        let sided: Vec<Vec2> = unseen.iter().copied().filter(|tile| (tile.x > seen.x) == right).collect();
        reached(sided).or_else(|| reached(unseen))
    }

    /// A way out of the place for someone near a point, with the point past
    /// it where one is out of the place and the way out from there: the
    /// nearest one it can get to, of those of the place on that side (or of
    /// that name), else the furthest ground on that side, one nobody goes
    /// out by first. None where the place has no ground.
    pub(crate) fn exit_of(&mut self, who: Who, near: Vec2, by: &Exit) -> Option<(Vec2, Vec2, Vec2)> {
        let ground = self.place.ground.clone()?;
        let middle = self.place.middle;
        let standing = |at: &Vec2| !ground.blocked(*at, 1);
        let doors: Vec<Vec2> = match by {
            Exit::Named(name) => match self.place.entrances.iter().find(|(known, _)| known == name) {
                Some((_, at)) => vec![ground.free(*at)],
                None => {
                    let known: Vec<&str> = self.place.entrances.iter().map(|(name, _)| name.as_str()).collect();
                    self.note(
                        format!("“{name}” is not an entrance of this place ({})", if known.is_empty() { "it has none".to_string() } else { known.join(", ") }),
                        true,
                    );
                    return None;
                }
            },
            Exit::Side { right } => {
                let sided: Vec<Vec2> = self
                    .place
                    .entrances
                    .iter()
                    .filter(|(_, at)| (at.x - middle.x) * if *right { 1.0 } else { -1.0 } > 2.0 * TILE)
                    .map(|(_, at)| ground.free(*at))
                    .collect();
                match sided.is_empty() {
                    false => sided,
                    true => edge(&ground, *right),
                }
            }
        };
        let crowded = self.crowded(Some(who), false);
        let straight = self.no_collisions;
        let reached = |door: &Vec2| {
            standing(door) && (straight || crowded.as_ref().is_some_and(|crowded| crowded.route(near, *door).is_some()) || ground.route(near, *door).is_some())
        };
        let taken: Vec<Vec2> = self.place.leaving.iter().filter(|(one, _)| **one != who && self.cast.contains_key(one)).map(|(_, door)| *door).collect();
        let mut doors: Vec<Vec2> = doors.into_iter().filter(reached).collect();
        doors.sort_by(|a, b| {
            let free = |door: &Vec2| taken.iter().any(|other| other.distance(*door) < 12.0);
            (free(a), a.distance_squared(near)).partial_cmp(&(free(b), b.distance_squared(near))).unwrap_or(std::cmp::Ordering::Equal)
        });
        let door = *doors.first()?;
        // (Out by the nearest edge of the place, unless it is far from any.)
        let size = ground.size();
        let edges = [(door.x, Vec2::NEG_X), (size.x - door.x, Vec2::X), (door.y, Vec2::NEG_Y), (size.y - door.y, Vec2::Y)];
        let (far, out) = edges.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap_or((0.0, Vec2::X));
        let beyond = if far <= NEAR_EDGE { door + out * (far + PAST) } else { door };
        Some((door, beyond, out))
    }

    /// The way someone still has to go, in pixels.
    pub(crate) fn left_to_walk(&self, who: Who) -> f32 {
        let Some(actor) = self.cast.get(&who) else {
            return 0.0;
        };
        let mut here = actor.way.to;
        let mut length = if actor.way.left > 0.0 { actor.position.distance(here) } else { 0.0 };
        for point in &actor.way.route {
            (length, here) = (length + here.distance(*point), *point);
        }
        length
    }

    /// Someone walks out of the place by that way, behind who goes out there
    /// already, and is gone once out. Whether there was a way.
    pub(crate) fn leave(&mut self, who: Who, by: &Exit, run: bool, seen: Vec2) -> bool {
        let Some(at) = self.cast.get(&who).map(|actor| actor.position) else {
            return true;
        };
        let cast = &self.cast;
        self.place.leaving.retain(|one, _| cast.contains_key(one));
        let Some((door, beyond, _)) = self.exit_of(who, at, by) else {
            return false;
        };
        let mut way = self.way_by(who, &[door]);
        if beyond != door {
            way.push(beyond);
        }
        // (A way out far from the place's edge, in sight: on out of sight
        // first, by a way it can walk, not gone before our eyes.)
        let half = UNSEEN / self.view.closeness().max(0.1);
        let door = match beyond == door && (door.x - seen.x).abs() <= half.x && (door.y - seen.y).abs() <= half.y {
            true => match self.way_out(door, door.x >= seen.x, seen) {
                Some(out) => {
                    way.extend(
                        self.place.ground.as_ref().filter(|_| !self.no_collisions).and_then(|ground| ground.route(door, out)).unwrap_or_else(|| vec![out]),
                    );
                    out
                }
                None => door,
            },
            false => door,
        };
        let mut here = at;
        let mut length = 0.0;
        for point in &way {
            (length, here) = (length + here.distance(*point), *point);
        }
        // (In file: no sooner out than the one before, a step behind.)
        let pace = WALK;
        let before = self.place.leaving.iter().filter(|(_, other)| other.distance(door) < 12.0).map(|(one, _)| self.left_to_walk(*one)).fold(0.0_f32, f32::max);
        let slowed = before > 0.0 && length < before + IN_FILE;
        match slowed {
            true => {
                self.place.paced.insert(who);
                self.direct_motion(MotionBeat::Speed { who, pace: (pace * length / (before + IN_FILE)).max(pace * 0.35) });
            }
            false => self.pace(who, &way, Gait { run, pace: Pace::Usual }),
        }
        self.direct_motion(MotionBeat::Path { who, points: way, run });
        self.with(who, |actor| actor.habits.vanishes = true);
        self.place.leaving.insert(who, door);
        true
    }

    /// Where someone coming in by a way starts, off the place, and the way in
    /// to the point it goes to; behind who comes in there already.
    pub(crate) fn way_in(&mut self, who: Who, to: Vec2, by: &Exit) -> Option<(Vec2, Vec2)> {
        let (door, beyond, out) = self.exit_of(who, to, by)?;
        let cast = &self.cast;
        self.place.entering.retain(|one, _| cast.get(one).is_some_and(Actor::busy));
        let ahead = self.place.entering.values().filter(|other| other.distance(door) < 12.0).count();
        self.place.entering.insert(who, door);
        Some((beyond + out * IN_FILE * ahead as f32, door))
    }

    /// A point out of sight, by that side.
    pub(crate) fn off_side(&self, right: bool) -> Vec2 {
        self.place.middle + Vec2::X * if right { OFF } else { -OFF }
    }

    /// Whether a point is out of what the view shows from there.
    pub(crate) fn out_of_view(&self, at: Vec2, seen: Vec2) -> bool {
        let half = UNSEEN / self.view.closeness().max(0.1);
        (at.x - seen.x).abs() > half.x || (at.y - seen.y).abs() > half.y
    }

    /// How far along a way all those points go together until each is out of
    /// what the view shows from there and off the place, a step more: where
    /// a crowd that keeps its form comes in from, unseen however the view
    /// then frames it.
    pub(crate) fn off_view_along(&self, seen: Vec2, points: &[Vec2], way: Vec2) -> f32 {
        let half = UNSEEN / self.view.closeness().max(0.1);
        let (low, high) = (seen - half, seen + half);
        // (The place's edges, if it has ground; the view's at least.)
        let (low, high) = match self.place.ground.as_ref() {
            Some(ground) => (low.min(Vec2::ZERO), high.max(ground.size())),
            None => (low, high),
        };
        let out = |at: f32, low: f32, high: f32, way: f32| match way {
            way if way > 0.01 => (high - at) / way,
            way if way < -0.01 => (low - at) / way,
            _ => f32::INFINITY,
        };
        let far = points.iter().map(|at| out(at.x, low.x, high.x, way.x).min(out(at.y, low.y, high.y, way.y))).fold(0.0_f32, f32::max);
        far.min(4096.0) + IN_FILE
    }

    /// Just out of what the view shows from there, on that side, at that
    /// height: where one who flies in comes from, not seen to appear.
    pub(crate) fn off_view(&self, seen: Vec2, right: bool, y: f32) -> Vec2 {
        let half = UNSEEN.x / self.view.closeness().max(0.1);
        Vec2::new(seen.x + if right { half } else { -half }, y)
    }
}
