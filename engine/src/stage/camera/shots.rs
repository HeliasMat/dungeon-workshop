//! What the beats ask of the camera: where to look, how close, whom to keep
//! in sight; and the shot that frames a group by the size of its bodies.

use crate::Frames;
use glam::Vec2;

use super::*;
use crate::beat::ViewBeat;

/// The most of the view a body takes in a shot (see `PlaceBeat::Shot`).
const SHOT_ROOM: Vec2 = Vec2::new(200.0, 110.0);

/// How big someone may show in a framing with the room below holding it.
const FRAMED_BODY: Vec2 = Vec2::new(48.0, 56.0);

/// The furthest a framing goes back to keep them all.
const FURTHEST: f32 = 0.3;

impl Camera {
    /// Plays a beat of the view (`focus`: where it looks now); gives back
    /// those that are not the camera's (the light's, the tempo...).
    pub(crate) fn direct(&mut self, beat: ViewBeat, focus: Vec2, cast: &Bodies) -> Option<ViewBeat> {
        match beat {
            ViewBeat::Ease(ease) => self.ease = ease,
            ViewBeat::Glance { to, frames, hold } => {
                let back = self.camera.clone().unwrap_or(Cam::at(focus));
                self.pan = Some(Pan::toward(focus, to, frames.0).eased(self.ease));
                self.glance = Some(Glance { back, frames: frames.0.max(1.0), left: frames.0.max(1.0) + hold.max(0.0) });
            }
            ViewBeat::BoxRoom(on) => self.roomy = on,
            ViewBeat::PanStays { to, frames } => {
                self.pan = Some(Pan::toward(focus, to, frames.0).eased(self.ease));
                self.pan_stays = true;
            }
            ViewBeat::Frame { who, at, frames, most } => {
                self.roomy = true;
                if let Some((middle, close)) = self.fit_with(&who, &at, cast) {
                    let close = most.map_or(close, |most| close.min(most));
                    // (Framed above the box from the start: the lines that
                    // follow do not move the view again.)
                    if self.room == 0.0 {
                        self.room = BOX_HIGH / 2.0 / close.max(0.1);
                    }
                    self.pan = Some(Pan::toward(focus, Cam::at(middle), frames.0).eased(self.ease));
                    self.zoom = Zoom { now: self.zoom.now, to: close, left: frames.0.max(1.0) };
                }
            }
            ViewBeat::KeepFrame(who) => {
                self.roomy = who.is_some();
                self.framing = who.unwrap_or_default();
            }
            ViewBeat::Body { who, size } => _ = self.bodies.insert(who, size),
            ViewBeat::Quake { strength, frames } => {
                self.quake = (strength > 0.0).then_some(Quake { strength, left: frames.0.max(1.0), whole: frames.0.max(1.0) })
            }
            ViewBeat::Camera(camera) => {
                self.roomy &= camera.is_none();
                if camera.is_some() {
                    self.framing.clear();
                }
                self.camera = camera.or(self.camera.clone());
            }
            ViewBeat::Pan { to, frames } => {
                self.pan = Some(Pan::toward(focus, to, frames.0).eased(self.ease));
                self.pan_stays = false;
            }
            ViewBeat::Shake { axis, strength, frames } => self.shake = Shake { axis, strength, left: frames.map(Frames::get) },
            // (Told how close, the view stops keeping anyone in sight: whatever
            // framed it, a `view` or a fight, would take it back at once.)
            ViewBeat::Zoom { to, frames } => {
                self.framing.clear();
                self.zoom = Zoom { now: self.zoom.now, to: to.clamp(0.15, 4.0), left: frames.0.max(1.0) };
            }
            other => return Some(other),
        }
        None
    }

    /// Where the view looks and how close it comes to show them all (who of
    /// them is on the stage), with room around for sprites and bubbles.
    pub(crate) fn fit(&self, who: &[Who], cast: &Bodies) -> Option<(Vec2, f32)> {
        self.fit_with(who, &[], cast)
    }

    /// The same, points of the place shown with them (a lamp, a door).
    pub(crate) fn fit_with(&self, who: &[Who], points: &[Vec2], cast: &Bodies) -> Option<(Vec2, f32)> {
        // (Feet to head: what passes the usual is added on its sides and above.)
        let reach = |who: &Who, at: Vec2| {
            let over = (self.bodies.get(who).copied().unwrap_or(FRAMED_BODY) - FRAMED_BODY).max(Vec2::ZERO);
            (at - Vec2::new(over.x / 2.0, over.y), at + Vec2::new(over.x / 2.0, 0.0))
        };
        let mut places: Vec<(Vec2, Vec2)> = who.iter().filter_map(|who| standing(cast, *who).map(|at| reach(who, at))).collect();
        places.extend(points.iter().map(|at| (*at, *at)));
        let first = *places.first()?;
        let (low, high) = places.iter().fold(first, |(low, high), (from, to)| (low.min(*from), high.max(*to)));
        // (With room round them; never closer than the original's own
        // view, which shows its scenes as they are.)
        let span = high - low + Vec2::new(96.0, 96.0);
        let close = (SCREEN.x / span.x).min((SCREEN.y - BOX_HIGH) / span.y).clamp(FURTHEST, 1.0);
        Some(((low + high) / 2.0 - Vec2::new(0.0, LIFT), close))
    }

    /// How close a shot comes for someone: as asked, unless its body would
    /// not hold in the view.
    pub(crate) fn shot_close(&self, close: f32, who: Option<Who>) -> f32 {
        let body = who.map_or(Vec2::ZERO, |who| self.body(who)).max(Vec2::ONE);
        close.min(SHOT_ROOM.x / body.x).min(SHOT_ROOM.y / body.y)
    }
}
