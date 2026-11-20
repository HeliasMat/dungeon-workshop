//! A number of a beat as written (`Num::Lit`) or one of the engine's found
//! as the scene plays, by its name (`Num::Var`, written `{name}`), and a
//! point of two of them.

use std::fmt;

use glam::Vec2;

use super::*;
use crate::stage::{Halo, Lit};

/// A number of a beat: as written, or one of the engine's numbers by its
/// name, known as the scene plays (`{_lieu3_x}`: a place found by the stage).
#[derive(Clone, Debug, PartialEq)]
pub enum Num {
    Lit(f32),
    Var(String),
}

impl Num {
    /// As a field: `{name}`, or a number.
    pub fn read(field: &str) -> Option<Num> {
        match field.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
            Some(name) if !name.is_empty() && !name.contains(['{', '}', ':']) => Some(Num::Var(name.to_string())),
            Some(_) => None,
            None => field.parse().ok().map(Num::Lit),
        }
    }

    /// Its value as written (a number not yet known: 0).
    pub fn value(&self) -> f32 {
        match self {
            Num::Lit(value) => *value,
            Num::Var(_) => 0.0,
        }
    }
}

impl From<f32> for Num {
    fn from(value: f32) -> Self {
        Num::Lit(value)
    }
}

impl fmt::Display for Num {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Num::Lit(value) => write!(out, "{value}"),
            Num::Var(name) => write!(out, "{{{name}}}"),
        }
    }
}

/// A point of the scene, each of its numbers as written or found as it plays.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub x: Num,
    pub y: Num,
}

impl Point {
    /// That point, as written.
    pub fn at(at: Vec2) -> Point {
        Point { x: Num::Lit(at.x), y: Num::Lit(at.y) }
    }

    /// Its values as written (a number not yet known: 0).
    pub fn vec2(&self) -> Vec2 {
        Vec2::new(self.x.value(), self.y.value())
    }

    /// As two fields.
    pub fn fields(&self) -> [String; 2] {
        [self.x.to_string(), self.y.to_string()]
    }

    fn nums_mut(&mut self) -> [&mut Num; 2] {
        [&mut self.x, &mut self.y]
    }
}

impl From<Vec2> for Point {
    fn from(at: Vec2) -> Self {
        Point::at(at)
    }
}

impl Cam {
    /// The view on that point, as written.
    pub fn at(at: Vec2) -> Cam {
        Cam::At(Point::at(at))
    }

    fn nums_mut(&mut self) -> Vec<&mut Num> {
        match self {
            Cam::At(at) => at.nums_mut().into(),
            Cam::Who(_) => Vec::new(),
        }
    }
}

impl Spot {
    fn nums_mut(&mut self) -> Vec<&mut Num> {
        match self {
            Spot::Written(at) | Spot::Word(at) | Spot::Exact(at) | Spot::Near(at) => at.nums_mut().into(),
            Spot::Shape { round, .. } => round.nums_mut(),
            _ => Vec::new(),
        }
    }
}

impl Beat {
    /// Every number of the beat that may be found as the scene plays (in the
    /// beat of an `After` or an `Every` too).
    pub fn nums_mut(&mut self) -> Vec<&mut Num> {
        match self {
            Beat::Body(BodyBeat::Put { at, .. } | BodyBeat::Hop { at, .. } | BodyBeat::LookAt { at, .. })
            | Beat::Motion(MotionBeat::Go { at, .. } | MotionBeat::Walk { at, .. } | MotionBeat::WalkTo { at, .. } | MotionBeat::Orbit { at, .. })
            | Beat::Screen(ScreenBeat::Picture { at, .. })
            | Beat::Flow(FlowBeat::WaitNear { at, .. } | FlowBeat::Door { at, .. })
            | Beat::Place(PlaceBeat::Arrives { to: at, .. })
            | Beat::Node(NodeBeat::Make { at, .. }) => at.nums_mut().into(),
            Beat::Place(PlaceBeat::Goes { points, .. }) => points.iter_mut().flat_map(Point::nums_mut).collect(),
            Beat::Place(PlaceBeat::Find { spot, .. } | PlaceBeat::Name { spot, .. } | PlaceBeat::Round { spot, .. } | PlaceBeat::Gathers { spot, .. }) => {
                spot.nums_mut()
            }
            Beat::Screen(ScreenBeat::Particles { on, .. } | ScreenBeat::Lightning(on) | ScreenBeat::Halo(Some(Halo { on, .. })))
            | Beat::View(
                ViewBeat::Camera(Some(on))
                | ViewBeat::Pan { to: on, .. }
                | ViewBeat::PanStays { to: on, .. }
                | ViewBeat::Glance { to: on, .. }
                | ViewBeat::SpotlightAdd(on, _)
                | ViewBeat::Spotlight(Some(Lit { on, .. })),
            ) => on.nums_mut(),
            Beat::Flow(FlowBeat::After { beat, .. } | FlowBeat::Every { beat, .. }) => beat.nums_mut(),
            Beat::Flow(FlowBeat::WaitFound(frames)) => vec![frames],
            _ => Vec::new(),
        }
    }

    /// The names of the numbers it holds that are found as the scene plays.
    pub fn vars(&self) -> Vec<String> {
        let mut beat = self.clone();
        beat.nums_mut()
            .into_iter()
            .filter_map(|num| match num {
                Num::Var(name) => Some(name.clone()),
                Num::Lit(_) => None,
            })
            .collect()
    }
}
