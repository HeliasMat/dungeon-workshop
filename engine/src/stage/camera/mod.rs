//! The camera: the one owner of the view. Where it looks (a point, someone
//! it follows, a group it keeps in sight), how close (never past what it
//! was told; a framing never closer than 1), its glides and eases, its
//! shakes, the room it leaves the dialogue box. The beats speak to it by
//! [`Camera::direct`]; time passes on it by [`Camera::advance`]; whoever
//! draws asks it [`Viewport::of`], which keeps it within a place's edges.
//!
//! - `shots`: what the beats ask of it, and how a group is framed;
//! - `follow`: a frame of it going by;
//! - `viewport`: what part of a place that shows on a screen.

use std::collections::{BTreeMap, HashMap};

use glam::Vec2;

use super::{Actor, Ease};
use crate::beat::Cam;
use crate::who::Who;

mod follow;
mod shots;
mod viewport;

pub use viewport::Viewport;

/// What the view shows at its usual closeness, in pixels (the lower
/// screen).
pub(crate) const SCREEN: Vec2 = Vec2::new(256.0, 192.0);

/// How much of it the dialogue box takes, at its bottom.
pub(crate) const BOX_HIGH: f32 = 56.0;

/// The size of a body none was told of (for the places and a shot).
pub(crate) const USUAL_BODY: Vec2 = Vec2::new(24.0, 32.0);

/// How far above the middle of what it frames the view looks (room for heads
/// over feet); a film's glide to a word of a place looks there too.
pub const LIFT: f32 = 8.0;

/// Who stands on the stage, as the camera reads them.
pub(crate) type Bodies = BTreeMap<Who, Actor>;

/// Where someone stands, if it is there.
fn standing(cast: &Bodies, who: Who) -> Option<Vec2> {
    cast.get(&who).map(|actor| actor.position)
}

/// The view: where it looks, how close, how it shakes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Camera {
    /// Who or what the view follows (none: where it is).
    pub camera: Option<Cam>,
    pub pan: Option<Pan>,
    pub shake: Shake,
    pub zoom: Zoom,
    /// The view quaking, less and less: how hard, the frames left and
    /// those it takes in all.
    pub quake: Option<Quake>,
    /// A glance (see `ViewBeat::Glance`): where the view goes back to, in how
    /// many frames, and the frames before it does.
    pub glance: Option<Glance>,
    /// Who the view keeps in sight (none: it goes as it was told).
    pub framing: Vec<Who>,
    /// How big those it was told of show, wide and high.
    pub bodies: HashMap<Who, Vec2>,
    /// It keeps clear of the dialogue box; how far aside that has it now.
    pub roomy: bool,
    pub room: f32,
    /// The glide at hand stays where it gets to.
    pub pan_stays: bool,
    /// How the glides begun from now on go.
    pub ease: Ease,
}

/// The view gone to look aside: where it goes back to, the frames that
/// takes, those left before it does (see `ViewBeat::Glance`).
#[derive(Clone, Debug, PartialEq)]
pub struct Glance {
    pub back: Cam,
    pub frames: f32,
    pub left: f32,
}

/// The view quaking: how hard, less and less, the frames left of those it
/// lasts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quake {
    pub strength: f32,
    pub left: f32,
    pub whole: f32,
}

/// The view's closeness (2: twice as big): now, where it goes, the frames
/// left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    pub now: f32,
    pub to: f32,
    pub left: f32,
}

impl Default for Zoom {
    fn default() -> Self {
        Zoom { now: 1.0, to: 1.0, left: 0.0 }
    }
}

/// The view gliding: from where, to what, its frames and those left.
#[derive(Clone, Debug, PartialEq)]
pub struct Pan {
    pub from: Vec2,
    pub to: Cam,
    pub frames: f32,
    pub left: f32,
    /// How it goes, as the camera was told when it began.
    pub ease: Ease,
}

impl Pan {
    /// A glide from where the view looks, in at least a frame.
    fn toward(focus: Vec2, to: Cam, frames: f32) -> Self {
        Pan { from: focus, to, frames: frames.max(1.0), left: frames.max(1.0), ease: Ease::Smooth }
    }

    /// The same, going as told.
    fn eased(self, ease: Ease) -> Self {
        Pan { ease, ..self }
    }
}

/// The view shaking along an axis (1 sideways, 2 up and down), by that
/// many pixels (0: still).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shake {
    pub axis: u16,
    pub strength: f32,
    /// The frames it still shakes (none: until told to stop).
    pub left: Option<f32>,
}

impl Camera {
    /// Whether the view still moves: gliding, coming closer or going back.
    pub(crate) fn busy(&self) -> bool {
        self.pan.is_some() || self.zoom.left > 0.0 || self.glance.is_some()
    }

    /// How close it is now (1 if it was never told).
    pub fn closeness(&self) -> f32 {
        if self.zoom.now > 0.0 { self.zoom.now } else { 1.0 }
    }

    /// The size of someone's body, as it was told (else the usual one).
    pub fn body(&self, who: Who) -> Vec2 {
        self.bodies.get(&who).copied().unwrap_or(USUAL_BODY)
    }

    /// Another place: the view as it usually is, not as close, aside or
    /// framing as the last place had it. (Where it looks stays.)
    pub(crate) fn new_place(&mut self) {
        (self.zoom, self.pan) = (Zoom::default(), None);
        self.framing.clear();
        (self.roomy, self.room, self.pan_stays) = (false, 0.0, false);
    }

    /// Where it goes, it gets to at once.
    pub(crate) fn settle(&mut self) {
        if let Some(pan) = self.pan.take() {
            self.camera = Some(pan.to);
        }
        (self.zoom.now, self.zoom.left) = (self.zoom.to, 0.0);
    }
}
