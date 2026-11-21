//! The parts of a body on the stage: the ways it faces, what it is to show
//! next ([`Cue`]), where it goes ([`Way`]), how it is off the ground
//! ([`Air`]), what it does by itself ([`Habits`]), how it shows ([`Looks`]),
//! going round a point ([`Orbit`]), the bits of how it is drawn.

use super::*;

/// The eight directions as steps, starting facing down and turning
/// counter-clockwise (direction 1 is the first).
pub(in crate::stage) const DIRECTIONS: [(i32, i32); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];

/// A walker's pace and a runner's, in pixels a frame.
pub const WALK: f32 = 1.2;

pub(crate) const RUN: f32 = 2.6;

/// The way a direction (1 to 8) goes, a step long (aslant a little more).
pub(crate) fn step_of(direction: u16) -> Vec2 {
    let (x, y) = DIRECTIONS[(direction.max(1) as usize - 1) % 8];
    Vec2::new(x as f32, y as f32)
}

/// A direction (1 to 8) turned so many eighths, counter-clockwise as the
/// directions go (4: about face).
pub(crate) fn turned(direction: u16, eighths: u16) -> u16 {
    (direction.max(1) - 1 + eighths % 8) % 8 + 1
}

/// Where a direction (1 to 8) looks, a step one pixel long.
pub(crate) fn facing(direction: u16) -> Vec2 {
    let turned = (direction as f32 - 1.0) * std::f32::consts::FRAC_PI_4;
    Vec2::new(turned.sin(), turned.cos())
}

/// The direction (1 to 8) from one point towards another, if they differ.
pub fn towards(from: Vec2, to: Vec2) -> Option<u16> {
    let way = (to - from).try_normalize()?;
    let step = ((way.x * 1.3).round() as i32, (way.y * 1.3).round() as i32);
    DIRECTIONS.iter().position(|&around| around == step).map(|at| at as u16 + 1)
}

/// What someone of the stage is to show once, as the game draws it.
/// When several come at once, the strongest wins (see [`Cue::rank`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cue {
    /// One of its sprite's animations: its group, and how it plays.
    Anim { group: usize, play: crate::beat::Play },
    /// Taking a blow.
    Hurt,
    /// Using a move: the group of its animations.
    Motion(u8),
    /// A pose of its own.
    Pose(Pose),
}

impl Cue {
    /// A pose over a move, over a blow, over an animation.
    pub fn rank(&self) -> u8 {
        match self {
            Cue::Anim { .. } => 0,
            Cue::Hurt => 1,
            Cue::Motion(_) => 2,
            Cue::Pose(_) => 3,
        }
    }
}

/// Where it goes, and how.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Way {
    /// Where it goes, the frames it still takes, and whether it slides
    /// there (not turned to where it goes, not walking).
    pub(crate) to: Vec2,
    pub(crate) left: f32,
    pub(crate) slide: bool,
    /// Its pace when it walks (pixels a frame); where it walks next once
    /// where it goes now (a path), and whether it runs there.
    pub(crate) pace: f32,
    pub(crate) route: VecDeque<Vec2>,
    pub(crate) running: bool,
    /// A point it turns to once it gets where it goes.
    pub(crate) then_face: Option<Vec2>,
    /// The way it was on when it was stopped.
    pub(crate) halted: Option<Halted>,
}

/// A way someone was stopped on: where it went, what was left of it, how.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Halted {
    pub(crate) to: Vec2,
    pub(crate) route: VecDeque<Vec2>,
    pub(crate) running: bool,
    pub(crate) then_face: Option<Vec2>,
}

/// How it is off the ground, and how it moves on the spot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Air {
    /// How high over the ground; the height it goes to and the frames it
    /// still takes.
    pub(crate) height: f32,
    pub(crate) rise: Option<Tween<f32>>,
    /// A jump: its height at the top and the frames of the whole jump.
    pub(crate) arc: Option<(f32, f32)>,
    /// The height a thrown thing keeps all the way, over its arc: what it
    /// was at when thrown (a shot from a rifle flies at the rifle's height).
    pub(crate) lift: f32,
    /// A spin: the turns still to make, the frames each takes, and those
    /// gone since the last.
    pub(crate) spin: Option<Spin>,
    /// How far its picture is turned by a spin that turns it (degrees,
    /// clockwise), over the angle of its node.
    pub(crate) turn: f32,
    pub(crate) tremble: f32,
    /// How many pixels it trembles to and fro.
    pub(crate) tremble_by: f32,
    /// Floating up and down: how high, the frames of a swing, and those
    /// gone so far.
    pub(crate) float: Option<Swing>,
    pub(crate) bounce: Option<Swing>,
    /// Its next turn goes the slow way, a way every so many frames (see
    /// `BodyBeat::TurnSlow`); a slow turn going: the way it ends looking,
    /// the frames of a step and those gone.
    pub(crate) slow_turn: Option<f32>,
    pub(crate) turning: Option<(u16, f32, f32)>,
}

/// What it does by itself, and what it is to the others.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Habits {
    /// Who it follows about, how far behind it stops; the frames it still trembles.
    pub(crate) follows: Option<Who>,
    pub(crate) follow_gap: f32,
    /// Whom it copies as a reflection would, and where that one stood
    /// last (see `MotionBeat::Mirror`).
    pub(crate) mirrors: Option<Mirroring>,
    /// Who carries it over its head (see `BodyBeat::Carry`).
    pub(crate) carried_by: Option<Who>,
    /// What is carried is in the hands, before who has it.
    pub(crate) held: bool,
    /// It rides on the back of who carries it: how high over its feet, how
    /// far to its side (see `BodyBeat::Ride`).
    pub(crate) rides: Option<(f32, f32)>,
    /// It leaves the stage once it gets where it goes (a thing given).
    pub(crate) vanishes: bool,
    /// It wanders about a point, never further from it than that.
    pub(crate) wander: Option<Wandering>,
    /// Going round a point (see [`Orbit`]).
    pub(crate) orbit: Option<Orbit>,
    pub(crate) fidgets: bool,
}

/// Turning round on the spot: the turns left, the frames of a turn, those
/// gone in it. `rotate`: its picture itself turns, not the way it faces;
/// then `left` is the frames left and `each` the degrees a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Spin {
    pub(crate) left: f32,
    pub(crate) each: f32,
    pub(crate) elapsed: f32,
    pub(crate) rotate: bool,
}

impl Spin {
    /// A spin of so many turns in so many frames: by the eight ways it
    /// faces, or its picture turning (clockwise for more than none).
    pub(crate) fn new(turns: f32, frames: f32, rotate: bool) -> Spin {
        let frames = frames.max(1.0);
        match rotate {
            true => Spin { left: frames, each: turns * 360.0 / frames, elapsed: 0.0, rotate },
            // (Eight directions a turn.)
            false => {
                let steps = (turns * 8.0).max(1.0);
                Spin { left: steps, each: frames / steps, elapsed: 0.0, rotate }
            }
        }
    }
}

/// Copying someone as a reflection would, and where that one stood last
/// (see `MotionBeat::Mirror`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Mirroring {
    pub(crate) model: Who,
    pub(crate) last: Vec2,
}

/// Wandering about a point, never further from it than that.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Wandering {
    pub(crate) home: Vec2,
    pub(crate) radius: f32,
}

/// How it shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Looks {
    /// Who it is shown as (see `BodyBeat::Disguise`).
    pub(crate) looks_like: Option<Who>,
    /// How much of it shows (1 wholly, 0 not at all), and where that goes
    /// and the frames it still takes.
    pub(crate) opacity: f32,
    pub(crate) fading: Option<Tween<f32>>,
    pub(crate) colouring: Option<Tween<[f32; 3]>>,
    /// Its size (1: its own); where it goes and the frames it still takes.
    pub(crate) scale: f32,
    pub(crate) scaling: Option<Tween<f32>>,
    /// How fast its pictures go by; whether it stays as it is.
    pub(crate) speed: f32,
    pub(crate) frozen: bool,
    /// How fast its own time goes (see `BodyBeat::Slow`).
    pub(crate) time: f32,
    /// Who it is drawn in front of; how much lower it is drawn as if it stood.
    pub(crate) in_front_of: Option<Who>,
    pub(crate) depth: f32,
}

/// Someone going round a point: how far from it, at what angle now (in
/// radians), by how much a frame, and the frames left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub center: Vec2,
    pub radius: f32,
    pub angle: f32,
    pub speed: f32,
    pub left: f32,
    /// Who it goes round, its middle going with them.
    pub around: Option<Who>,
}

/// The bits of how someone is drawn (see `BodyBeat::Attr`).
pub mod attribute {
    /// A little before the others standing at the same height.
    pub const FRONT: u16 = 1;
    /// Under everyone.
    pub(crate) const UNDER: u16 = 2;
    /// Over everyone.
    pub(crate) const OVER: u16 = 4;
    /// Over the layer of the map characters walk under.
    pub const OVER_MAP: u16 = 8;
    /// Showing and hiding in turn.
    pub const BLINKING: u16 = 32;
    pub const SEEN_THROUGH: u16 = 64;
    pub const HIDDEN: u16 = 128;
    /// Placed on the screen, wherever the view is.
    pub(crate) const ON_SCREEN: u16 = 2048;
}
