//! Who moves by itself on the stage, round the walls and the others.

use super::*;

/// A way from a point to another going round the walls and the solid
/// (see `MotionBeat::WalkTo`): the points where it turns, the last the end; the
/// end alone when nothing is in the way; none when there is no way.
pub(in crate::stage) fn way_round(walls: &[Rect], solids: &[Vec2], from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    /// The squares the way is looked for on; how far round the two points.
    pub(in crate::stage) const CELL: f32 = 8.0;
    pub(in crate::stage) const MARGIN: f32 = 160.0;
    let blocked = |at: Vec2| walled(walls, at) || solids.iter().any(|solid| solid.distance(at) < 14.0);
    let clear = |a: Vec2, b: Vec2| {
        let steps = (a.distance(b) / 4.0).ceil().max(1.0) as usize;
        (1..=steps).all(|step| !blocked(a + (b - a) * (step as f32 / steps as f32)))
    };
    if clear(from, to) {
        return Some(vec![to]);
    }
    let corner = from.min(to) - Vec2::splat(MARGIN);
    let size = (from.max(to) + Vec2::splat(MARGIN) - corner) / CELL;
    let (wide, high) = (size.x.ceil() as i32, size.y.ceil() as i32);
    let cell_of = |at: Vec2| (((at.x - corner.x) / CELL) as i32, ((at.y - corner.y) / CELL) as i32);
    let middle = |(x, y): (i32, i32)| corner + Vec2::new((x as f32 + 0.5) * CELL, (y as f32 + 0.5) * CELL);
    let index = |(x, y): (i32, i32)| (y * wide + x) as usize;
    let (start, goal) = (cell_of(from), cell_of(to));
    let guess = |(x, y): (i32, i32)| ((((x - goal.0).pow(2) + (y - goal.1).pow(2)) as f32).sqrt() * 10.0) as u32;
    let (mut best, mut came) = (vec![u32::MAX; (wide * high) as usize], vec![None; (wide * high) as usize]);
    let mut open = BinaryHeap::new();
    best[index(start)] = 0;
    open.push(Reverse((guess(start), start)));
    while let Some(Reverse((_, cell))) = open.pop() {
        if cell == goal {
            break;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let next = (cell.0 + dx, cell.1 + dy);
            if next.0 < 0 || next.1 < 0 || next.0 >= wide || next.1 >= high || (next != goal && blocked(middle(next))) {
                continue;
            }
            let cost = best[index(cell)] + if dx != 0 && dy != 0 { 14 } else { 10 };
            if cost < best[index(next)] {
                best[index(next)] = cost;
                came[index(next)] = Some(cell);
                open.push(Reverse((cost + guess(next), next)));
            }
        }
    }
    if best[index(goal)] == u32::MAX {
        return None;
    }
    // (Back from the end, then only where it must turn.)
    let mut cells = vec![goal];
    while let Some(back) = came[index(*cells.last()?)] {
        cells.push(back);
    }
    let points: Vec<Vec2> = cells.into_iter().rev().skip(1).map(middle).collect();
    let mut kept = Vec::new();
    let mut here = from;
    for (at, point) in points.iter().enumerate() {
        let next = points.get(at + 1).copied().unwrap_or(to);
        if !clear(here, next) {
            kept.push(*point);
            here = *point;
        }
    }
    kept.push(to);
    Some(kept)
}

/// Whether a point is in one of the walls (see `PlayBeat::Wall`), with a few
/// pixels of the body round it.
pub(in crate::stage) fn walled(walls: &[Rect], at: Vec2) -> bool {
    pub(in crate::stage) const BODY: f32 = 5.0;
    walls.iter().any(|wall| wall.holds(at, BODY))
}

impl Stage {
    /// The one the player walks about (see `PlayBeat::Control`) goes that way
    /// (none: stops where it is, if the player had it walking), running or
    /// not. Its walks the scene asks for are left as they are.
    pub fn steer(&mut self, toward: Vec2, run: bool) {
        let Some((me, at, pace)) = self.play.controlled.and_then(|who| self.cast.get(&who).map(|actor| (who, actor.position, actor.pace_now(run)))) else {
            return;
        };
        let stop = |stage: &mut Stage| {
            if let Some(actor) = stage.cast.get_mut(&me) {
                (actor.way.to, actor.way.left) = (actor.position, 0.0);
            }
        };
        if toward.length() <= 0.0 {
            if std::mem::take(&mut self.play.steered) {
                stop(self);
            }
            return;
        }
        let way = toward.normalize();
        self.play.steered = true;
        // (Walking into something that may be pushed: it goes a step that
        // way, if it can; the one pushing waits for it.)
        let pushed = self.cast.iter().find(|(who, actor)| {
            **who != me
                && actor.habits.pushable
                && actor.way.left <= 0.0
                && actor.position.distance(at) < 20.0
                && (actor.position - at).normalize_or_zero().dot(way) > 0.7
        });
        if let Some((pushed, there)) = pushed.map(|(who, actor)| (*who, actor.position)) {
            let step = (if way.x.abs() > way.y.abs() { Vec2::new(way.x.signum(), 0.0) } else { Vec2::new(0.0, way.y.signum()) }) * 16.0;
            let to = there + step;
            if self.kept_in(to) == to && (self.no_collisions || (!walled(&self.play.walls, to) && !self.solid_near(pushed, there, to))) {
                self.with(pushed, |actor| actor.go(to, 10.0, true));
            }
            stop(self);
            return;
        }
        // (A few frames ahead, again each frame: on without a stop; not into
        // a wall nor through someone solid, along it if it can.)
        let ahead = self.kept_in(at + way * pace * 4.0);
        // (Who stands in a wall already, put there, may get out of it. The
        // walls: the scene's, the place's, its ground.)
        let solid = |stage: &Stage, point: Vec2| {
            walled(&stage.play.walls, point)
                || walled(&stage.play.place_walls, point)
                || stage.place.ground.as_ref().is_some_and(|ground| ground.blocked(point, 0))
        };
        let free = |stage: &Stage, to: Vec2| stage.no_collisions || ((!solid(stage, to) || solid(stage, at)) && !stage.solid_near(me, at, to));
        let along = [ahead, Vec2::new(ahead.x, at.y), Vec2::new(at.x, ahead.y)];
        match along.into_iter().find(|to| to.distance(at) > 0.5 && free(self, *to)) {
            Some(to) => self.with(me, |actor| actor.walk_to(to, run)),
            None => stop(self),
        }
    }

    /// The free point that far before someone, as it faces now (behind it
    /// when less than 0); `goer` is no one in the way.
    pub(crate) fn before(&self, who: Who, far: f32, goer: Option<Who>) -> Option<Vec2> {
        /// The rings of tiles looked at round the point, and how far from
        /// someone a point is free.
        const RINGS: i32 = 8;
        const TILE: f32 = 8.0;
        const BODY: f32 = 14.0;
        let actor = self.cast.get(&who)?;
        let wanted = actor.position + step_of(actor.direction).normalize_or_zero() * far;
        if self.no_collisions {
            return Some(wanted);
        }
        let free = |at: Vec2| {
            !walled(&self.play.place_walls, at)
                && !walled(&self.play.walls, at)
                && !self.cast.iter().any(|(one, other)| Some(*one) != goer && other.position.distance(at) < BODY)
        };
        let ring = |far: i32| (-far..=far).flat_map(move |dx| (-far..=far).filter(move |dy| dx.abs().max(dy.abs()) == far).map(move |dy| (dx, dy)));
        let near = (0..=RINGS).flat_map(ring).map(|(dx, dy)| wanted + Vec2::new(dx as f32, dy as f32) * TILE);
        Some(near.into_iter().find(|at| free(*at)).unwrap_or(wanted))
    }

    /// A point kept in the box the one walked about stays in, if any.
    pub(in crate::stage) fn kept_in(&self, at: Vec2) -> Vec2 {
        self.play.bounds.map_or(at, |bounds| bounds.clamp(at))
    }

    /// Whether going from a point to another, someone would walk into one
    /// of the solid (getting nearer than they stand: never stuck in one).
    pub(in crate::stage) fn solid_near(&self, me: Who, from: Vec2, to: Vec2) -> bool {
        pub(super) const BODY: f32 = 14.0;
        self.cast.iter().any(|(who, actor)| {
            *who != me && actor.habits.solid && actor.position.distance(to) < BODY && actor.position.distance(to) < actor.position.distance(from)
        })
    }

    /// Whether one sees another: near enough, in front of it within that
    /// angle (degrees, the whole cone), no wall between them.
    pub fn sees(&self, who: Who, other: Who, distance: f32, angle: f32) -> bool {
        let (Some(watcher), Some(seen)) = (self.cast.get(&who), self.cast.get(&other)) else {
            return false;
        };
        let gap = seen.position - watcher.position;
        if gap.length() > distance {
            return false;
        }
        let ahead = step_of(watcher.direction).normalize_or_zero();
        let within = gap.length() < 4.0 || gap.normalize().dot(ahead) >= (angle.to_radians() / 2.0).cos();
        // (A wall between: looked for every few pixels along the way.)
        let steps = (gap.length() / 4.0).ceil() as usize;
        within && !(1..steps).any(|step| walled(&self.play.walls, watcher.position + gap * (step as f32 / steps as f32)))
    }

    /// Who walks steps aside from who it comes up against, to its right:
    /// two meeting head on pass each other. (Where it goes it still gets
    /// to in its time: see `Actor::advance`. Only `wall`s are known here,
    /// not the map's own walls.)
    pub(in crate::stage) fn step_aside(&mut self, frames: f32) {
        /// How far apart bodies keep, how fast one steps aside (pixels a
        /// frame), and the frames before it gets there when it stops
        /// minding: it has somewhere to stand.
        pub(super) const ROOM: f32 = 18.0;
        pub(super) const ASIDE: f32 = 1.1;
        pub(super) const ARRIVING: f32 = 10.0;
        let body = |who: &Who, actor: &Actor| {
            matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. })
                && actor.air.height == 0.0
                && actor.habits.carried_by.is_none()
                && actor.attributes & attribute::HIDDEN == 0
        };
        let bodies: Vec<(Who, Vec2)> = self.cast.iter().filter(|(who, actor)| body(who, actor)).map(|(who, actor)| (*who, actor.position)).collect();
        let walls = std::mem::take(&mut self.play.walls);
        for (who, actor) in self.cast.iter_mut().filter(|(who, actor)| actor.walks() && actor.way.left > ARRIVING && body(who, actor)) {
            let ahead = (actor.way.to - actor.position).normalize_or_zero();
            let mut push = Vec2::ZERO;
            for (_, at) in bodies.iter().filter(|(other, _)| other != who) {
                let from = actor.position - *at;
                let near = from.length();
                if near >= ROOM {
                    continue;
                }
                let closeness = 1.0 - near / ROOM;
                // (Right on top of it, or coming straight at it: aside,
                // the same way for both.)
                push += from.normalize_or(ahead.perp()) * closeness;
                if from.dot(ahead) < 0.0 {
                    push += ahead.perp() * closeness;
                }
            }
            let there = actor.position + push.clamp_length_max(1.0) * ASIDE * frames;
            if !walled(&walls, there) && !walled(&self.play.place_walls, there) {
                actor.position = there;
            }
        }
        self.play.walls = walls;
    }

    /// Who wanders about and stands still now and then goes elsewhere near
    /// where it wanders, after a while.
    pub(in crate::stage) fn wander(&mut self, frames: f32) {
        // (Who looks about turns now and then, about every two seconds.)
        let fidgeting: Vec<Who> = self.cast.iter().filter(|(_, actor)| actor.habits.fidgets && !actor.busy()).map(|(who, _)| *who).collect();
        for who in fidgeting {
            if self.chance(1000) as f32 >= frames * 1000.0 / 120.0 {
                continue;
            }
            let turn = [1, 7, 2, 6][self.chance(4) as usize];
            let actor = self.cast.get_mut(&who).expect("someone looking about");
            actor.direction = turned(actor.direction, turn);
        }
        let wanderers: Vec<Who> = self.cast.iter().filter(|(_, actor)| actor.habits.wander.is_some() && !actor.busy()).map(|(who, _)| *who).collect();
        for who in wanderers {
            // (About once in a second and a half.)
            if self.chance(1000) as f32 >= frames * 1000.0 / 90.0 {
                continue;
            }
            let (angle, far) = (self.chance(360) as f32, self.chance(100) as f32 / 100.0);
            let actor = self.cast.get_mut(&who).expect("a wanderer");
            let Wandering { home, radius } = actor.habits.wander.expect("wandering");
            actor.walk_to(home + Vec2::from_angle(angle.to_radians()) * radius * far, false);
        }
    }

    /// Who copies another as a reflection would (see `MotionBeat::Mirror`): as
    /// far as the other went since, the other way across, the same way up
    /// and down, walking there; not into a wall, nor out of the box the
    /// player is kept in. Turned as the other is, the other way across.
    /// (One gone further than a walk at once, put there: not followed.)
    pub(in crate::stage) fn mirror(&mut self) {
        let places: HashMap<Who, (Vec2, u16)> = self.cast.iter().map(|(who, actor)| (*who, (actor.position, actor.direction))).collect();
        let copying: Vec<(Who, Who, Vec2)> =
            self.cast.iter().filter_map(|(who, actor)| actor.habits.mirrors.map(|Mirroring { model, last }| (*who, model, last))).collect();
        for (who, model, last) in copying {
            let Some(&(now, facing)) = places.get(&model) else {
                continue;
            };
            let moved = now - last;
            let at = self.cast[&who].position;
            let to = at + Vec2::new(-moved.x, moved.y);
            let free = moved.length() > 0.01 && moved.length() < 24.0 && (self.no_collisions || !walled(&self.play.walls, to)) && self.kept_in(to) == to;
            let way = step_of(facing);
            let turned = towards(Vec2::ZERO, Vec2::new(-way.x, way.y));
            if let Some(actor) = self.cast.get_mut(&who) {
                actor.habits.mirrors = Some(Mirroring { model, last: now });
                if free {
                    actor.go(to, 1.0, false);
                }
                actor.direction = turned.unwrap_or(actor.direction);
            }
        }
    }

    /// A line (see `MotionBeat::Train`): the leader's way kept, and each of the
    /// others walks to where it is on it, so far behind the one before
    /// (running when far behind); the way past the last left behind.
    pub(in crate::stage) fn train(&mut self) {
        let Some(Train { gap, who, way }) = self.train.as_mut() else {
            return;
        };
        let Some(lead) = who.first().and_then(|leader| self.cast.get(leader)).map(|actor| actor.position) else {
            return;
        };
        if way.first().is_none_or(|first| first.distance(lead) >= 1.0) {
            way.insert(0, lead);
        }
        // (The point that far along the way, if it goes that far.)
        let along = |way: &[Vec2], far: f32| {
            let mut left = far;
            for pair in way.windows(2) {
                let length = pair[0].distance(pair[1]);
                if length >= left {
                    return Some(pair[0] + (pair[1] - pair[0]) * (left / length.max(0.001)));
                }
                left -= length;
            }
            None
        };
        for (place, follower) in who.iter().enumerate().skip(1) {
            let Some(spot) = along(way, *gap * place as f32) else {
                continue;
            };
            if let Some(actor) = self.cast.get_mut(follower)
                && actor.position.distance(spot) > 0.5
            {
                let far = actor.position.distance(spot) > *gap;
                actor.walk_to(spot, far);
            }
        }
        // (No more of the way than the last one needs.)
        let needed = *gap * who.len() as f32;
        let (mut length, mut keep) = (0.0, way.len());
        for (at, pair) in way.windows(2).enumerate() {
            length += pair[0].distance(pair[1]);
            if length > needed {
                keep = at + 2;
                break;
            }
        }
        way.truncate(keep);
    }

    /// Who follows someone and has fallen behind walks after them, to stop
    /// a step behind (else aslant, wherever nobody else stands).
    pub(in crate::stage) fn follow(&mut self) {
        let places: HashMap<Who, Vec2> = self.cast.iter().map(|(who, actor)| (*who, actor.position)).collect();
        // (Who rides one that follows goes with it: never in its way.)
        let riding: HashMap<Who, Who> = self.cast.iter().filter_map(|(who, actor)| Some((*who, actor.habits.carried_by?))).collect();
        // (The place's ground, walked round when it is in the way.)
        let ground = self.place.ground.clone().filter(|_| !self.no_collisions);
        for (who, actor) in self.cast.iter_mut() {
            let Some(leader) = actor.habits.follows.and_then(|leader| places.get(&leader)) else {
                continue;
            };
            let (gap, behind) = (*leader - actor.position, actor.habits.follow_gap);
            if gap.length() <= behind + 4.0 || actor.way.left > 0.0 {
                continue;
            }
            let back = -gap.normalize();
            let free = |at: &Vec2| places.iter().all(|(other, place)| other == who || riding.get(other) == Some(who) || place.distance(*at) > 18.0);
            let spots = [0.0_f32, 0.7, -0.7, 1.4, -1.4].map(|turn| *leader + Vec2::from_angle(turn).rotate(back) * behind);
            let to = spots.iter().copied().find(free).unwrap_or(spots[0]);
            // (Not through the scenery: by its way round, to its first turn.)
            let to = match &ground {
                Some(ground) if !ground.clear(actor.position, to) => {
                    ground.route(actor.position, to).and_then(|way| way.first().copied()).unwrap_or(actor.position)
                }
                _ => to,
            };
            if to.distance(actor.position) < 0.5 {
                continue;
            }
            actor.go(to, actor.position.distance(to) / 1.6, false);
        }
    }
}
