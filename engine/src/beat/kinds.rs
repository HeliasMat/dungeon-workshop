//! The small records and words the families of beats share.

use super::{Point, Value};
use crate::who::Who;

/// A gauge kept on screen: its number, against its most, in a colour.
#[derive(Clone, Debug, PartialEq)]
pub struct GaugeOf {
    pub var: String,
    pub most: Value,
    pub colour: [u8; 3],
}

/// What a scene's particles are (see `ScreenBeat::Particles`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Fireworks,
    Sparks,
    Fireflies,
    Embers,
    Smoke,
    Bubbles,
    Dust,
    Drops,
    Healing,
    Rings,
}

/// What is in the air over a scene (see `ScreenBeat::Weather`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Weather {
    #[default]
    Clear,
    Rain,
    Snow,
    Petals,
    Sparkles,
    Leaves,
    Hail,
    Sand,
    Mist,
    Sun,
}

/// A pose someone of the scene takes: the game shows it with the
/// matching animation of its sprite.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Pose {
    #[default]
    Idle,
    Walk,
    Hurt,
    Sleep,
    Attack,
}

/// What a bubble says it is: words said, thought, or shouted.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Voice {
    #[default]
    Said,
    Thought,
    Shouted,
}

/// Where the view of a scene looks: a point, or who it follows.
#[derive(Clone, Debug, PartialEq)]
pub enum Cam {
    At(Point),
    Who(Who),
}
