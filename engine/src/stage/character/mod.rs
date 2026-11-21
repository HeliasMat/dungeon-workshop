//! A body on the stage, the one model of it: where it stands and goes
//! ([`Actor`]: its way, its height, what it does by itself, how it shows),
//! and how it is shown, kept through the scenes ([`Figure`]: a variant's
//! painted ways). The beats speak to it
//! through the stage; whoever draws reads it.
//!
//! - `parts`: what an actor is made of;
//! - `figure`: how it is shown;
//! - `habits`: who moves by itself, round the walls and the others.

use super::*;

mod figure;
mod habits;
mod parts;

pub use figure::*;
pub(super) use habits::*;
pub use parts::*;

/// Someone or something a scene has put on its stage, as it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub position: Vec2,
    /// The way it faces (1 to 8, see [`towards`]).
    pub direction: u16,
    /// How scripts have it drawn (hidden, blinking, over the others...):
    /// the game reads them.
    pub attributes: u16,
    /// What it is to show next (see [`Cue`]).
    pub cue: Option<Cue>,
    /// What its picture is multiplied by (red, green, blue, of 255), and
    /// where that goes and the frames it still takes.
    pub colour: [f32; 3],
    /// It casts no shadow.
    pub shadowless: bool,
    /// Its picture is drawn the other way round, left to right.
    pub flipped: bool,
    /// How deep it is sunk in the water, in pixels (see `BodyBeat::Sink`).
    pub sunk: f32,
    /// What it is sunk in is sand, not water.
    pub in_sand: bool,
    /// How much of its life is left, for a bar over it.
    pub bar: Option<f32>,
    /// How many blows it has taken.
    pub hits: u32,
    pub(crate) way: Way,
    pub(crate) air: Air,
    pub(crate) habits: Habits,
    pub(crate) looks: Looks,
}

impl Actor {
    /// Someone just put on the stage, standing still.
    pub fn at(position: Vec2, direction: u16) -> Self {
        Self {
            position,
            direction,
            attributes: 0,
            cue: None,
            colour: [255.0; 3],
            shadowless: false,
            flipped: false,
            sunk: 0.0,
            in_sand: false,
            bar: None,
            hits: 0,
            way: Way { to: position, left: 0.0, slide: false, pace: WALK, route: VecDeque::new(), running: false, then_face: None, halted: None },
            air: Air {
                height: 0.0,
                rise: None,
                arc: None,
                lift: 0.0,
                spin: None,
                turn: 0.0,
                tremble: 0.0,
                tremble_by: 1.0,
                float: None,
                bounce: None,
                slow_turn: None,
                turning: None,
            },
            habits: Habits {
                follows: None,
                follow_gap: crate::stage::FOLLOW_GAP,
                mirrors: None,
                carried_by: None,
                held: false,
                rides: None,
                vanishes: false,
                wander: None,
                orbit: None,
                fidgets: false,
            },
            looks: Looks {
                looks_like: None,
                opacity: 1.0,
                fading: None,
                colouring: None,
                scale: 1.0,
                scaling: None,
                speed: 1.0,
                frozen: false,
                time: 1.0,
                in_front_of: None,
                depth: 0.0,
            },
        }
    }

    /// Whether it is still moving: going somewhere (a path's last point
    /// included), rising, spinning, fading. (Wandering about is not.)
    pub fn busy(&self) -> bool {
        !self.looks.frozen
            && (self.way.left > 0.0
                || self.air.rise.is_some()
                || self.air.spin.is_some()
                || self.air.turning.is_some()
                || !self.way.route.is_empty()
                || self.looks.fading.is_some()
                || self.habits.orbit.is_some()
                || self.looks.colouring.is_some()
                || self.looks.scaling.is_some())
    }

    /// How fast its pictures go by now (0: it is frozen).
    pub fn picture_speed(&self) -> f32 {
        if self.looks.frozen { 0.0 } else { self.looks.speed * self.looks.time }
    }

    /// How fast its own time goes (1: as the scene's; see `BodyBeat::Slow`).
    pub fn own_time(&self) -> f32 {
        self.looks.time
    }

    /// How far down it counts among those it is drawn with.
    pub fn drawn_at(&self) -> f32 {
        self.position.y + self.looks.depth
    }

    /// Who it is drawn in front of.
    pub fn in_front_of(&self) -> Option<Who> {
        self.looks.in_front_of
    }

    /// Its pace now: its own, or a runner's as much faster.
    pub(in crate::stage) fn pace_now(&self, run: bool) -> f32 {
        if run { self.way.pace * RUN / WALK } else { self.way.pace }
    }

    /// Walks somewhere at its pace (or runs).
    pub(crate) fn walk_to(&mut self, to: Vec2, run: bool) {
        let frames = self.position.distance(to) / self.pace_now(run).max(0.1);
        self.go(to, frames, false);
    }

    /// Where it is going in the end.
    pub fn goal(&self) -> Vec2 {
        self.way.route.back().copied().unwrap_or(self.way.to)
    }

    /// Whether it runs where it goes.
    pub fn runs(&self) -> bool {
        self.way.running
    }

    /// Whether it walks (goes somewhere without sliding).
    pub fn walks(&self) -> bool {
        self.way.left > 0.0 && !self.way.slide
    }

    /// Its size (1: its own), as it was told.
    pub fn scale(&self) -> f32 {
        self.looks.scale
    }

    /// How much of it shows now.
    pub fn shown_opacity(&self) -> f32 {
        self.looks.opacity.clamp(0.0, 1.0)
    }

    /// How far a spin turns its picture now (degrees, clockwise), over its
    /// node's angle.
    pub fn shown_turn(&self) -> f32 {
        self.air.turn
    }

    /// A spin that turns its picture, done at once: where it ends, whole
    /// turns back upright.
    pub(crate) fn end_spin(&mut self) {
        if let Some(Spin { left, each, rotate: true, .. }) = self.air.spin {
            self.air.turn += left * each;
        }
        self.air.spin = None;
        self.end_turn();
    }

    /// Its turn kept under a turn; none if it came round.
    fn end_turn(&mut self) {
        let turn = self.air.turn.rem_euclid(360.0);
        self.air.turn = if turn < 0.01 || turn > 359.99 { 0.0 } else { turn };
    }

    /// Its size now.
    pub fn shown_scale(&self) -> f32 {
        self.looks.scale
    }

    /// How high over the ground it shows: its height, and as high again
    /// as it floats now.
    pub fn shown_height(&self) -> f32 {
        let floating = self.air.float.map_or(0.0, |Swing { height, frames, gone }| height * (0.5 - 0.5 * (std::f32::consts::TAU * gone / frames).cos()));
        // (Hops: up and down again, touching the ground between two.)
        let hopping = self.air.bounce.map_or(0.0, |Swing { height, frames, gone }| height * (std::f32::consts::PI * gone / frames).sin().abs());
        self.air.height + floating + hopping
    }

    /// Whether it can be seen: not hidden, not faded away.
    pub fn seen(&self) -> bool {
        self.attributes & attribute::HIDDEN == 0 && self.looks.opacity > 0.05
    }

    /// Where it shows: where it stands, a pixel to or fro if it trembles.
    pub fn shown_at(&self) -> Vec2 {
        match self.air.tremble > 0.0 {
            true => self.position + Vec2::X * self.air.tremble_by * if (self.air.tremble as i32 / 2) % 2 == 0 { 1.0 } else { -1.0 },
            false => self.position,
        }
    }

    /// Has it show something, unless something stronger is waiting.
    pub fn cue(&mut self, cue: Cue) {
        if self.cue.is_none_or(|waiting| waiting.rank() <= cue.rank()) {
            self.cue = Some(cue);
        }
    }

    /// A turn just made (from `before`), the slow way if it was told so:
    /// back where it looked, then a way at a time (`advance`).
    pub(in crate::stage) fn turned_from(&mut self, before: u16) {
        let Some(every) = self.air.slow_turn.take() else {
            return;
        };
        if self.direction != before {
            self.air.turning = Some((self.direction, every.max(1.0), 0.0));
            self.direction = before;
        }
    }

    /// A step of a slow turn when its time comes: the nearer way round,
    /// counter-clockwise when both are as near.
    fn turn_slowly(&mut self, frames: f32) {
        let Some((to, every, gone)) = self.air.turning else {
            return;
        };
        let mut gone = gone + frames;
        while gone >= every && self.direction != to {
            gone -= every;
            let ahead = (to + 8 - self.direction) % 8;
            self.direction = parts::turned(self.direction, if ahead <= 4 { 1 } else { 7 });
        }
        self.air.turning = (self.direction != to).then_some((to, every, gone));
    }

    /// Sends it somewhere in that many frames.
    pub fn go(&mut self, to: Vec2, frames: f32, slide: bool) {
        (self.way.to, self.way.left, self.way.slide) = (to, frames.max(1.0), slide);
        self.air.turning = None;
    }

    /// One frame's worth (or more) of its walk, rise, jump and spin.
    pub(in crate::stage) fn advance(&mut self, frames: f32) {
        if self.looks.frozen {
            return;
        }
        if self.way.left > 0.0 {
            let way = self.way.to - self.position;
            let step = frames.min(self.way.left);
            self.position += way * (step / self.way.left);
            self.way.left -= step;
            // (Turned to where it goes, unless it slides there.)
            if !self.way.slide {
                self.direction = towards(Vec2::ZERO, way).unwrap_or(self.direction);
            }
        }
        Tween::run(&mut self.air.rise, &mut self.air.height, frames);
        // (A jump: up and down again along the way.)
        if let Some((peak, whole)) = self.air.arc {
            let done = 1.0 - (self.way.left / whole).clamp(0.0, 1.0);
            self.air.height = self.air.lift + 4.0 * peak * done * (1.0 - done);
            if self.way.left <= 0.0 {
                (self.air.height, self.air.arc, self.air.lift) = (0.0, None, 0.0);
            }
        }
        match self.air.spin {
            Some(Spin { left, each, rotate: true, .. }) => {
                let step = frames.min(left);
                self.air.turn += each * step;
                self.air.spin = (left - step > 0.0).then_some(Spin { left: left - step, each, elapsed: 0.0, rotate: true });
                if self.air.spin.is_none() {
                    self.end_turn();
                }
            }
            Some(Spin { left, each, elapsed, rotate: false }) => {
                let elapsed = elapsed + frames;
                let turned = (elapsed / each).floor();
                self.direction = parts::turned(self.direction, turned as u16);
                let left = left - turned;
                self.air.spin = (left > 0.0).then_some(Spin { left, each, elapsed: elapsed - turned * each, rotate: false });
            }
            None => {}
        }
        if self.air.tremble > 0.0 {
            self.air.tremble -= frames;
        }
        self.turn_slowly(frames);
        // (On along its path, once where it went.)
        if self.way.left <= 0.0
            && let Some(next) = self.way.route.pop_front()
        {
            self.walk_to(next, self.way.running);
        }
        if let Some(orbit) = &mut self.habits.orbit {
            let step = frames.min(orbit.left);
            orbit.angle += orbit.speed * step;
            orbit.left -= step;
            let there = orbit.center + Vec2::from_angle(orbit.angle) * orbit.radius;
            self.direction = towards(self.position, there).unwrap_or(self.direction);
            self.position = there;
        }
        self.habits.orbit = self.habits.orbit.filter(|orbit| orbit.left > 0.0);
        if self.way.left <= 0.0
            && self.way.route.is_empty()
            && let Some(point) = self.way.then_face.take()
        {
            self.direction = towards(self.position, point).unwrap_or(self.direction);
        }
        Tween::run(&mut self.looks.scaling, &mut self.looks.scale, frames);
        for swing in [&mut self.air.float, &mut self.air.bounce].into_iter().flatten() {
            swing.gone += frames;
        }
        Tween::run(&mut self.looks.colouring, &mut self.colour, frames);
        Tween::run(&mut self.looks.fading, &mut self.looks.opacity, frames);
    }
}
