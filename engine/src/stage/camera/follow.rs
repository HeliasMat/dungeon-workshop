//! A frame of the camera going by: it keeps clear of the dialogue box,
//! glides (slowly at its ends) or follows, eases its closeness, shakes,
//! comes back from a glance, keeps a group in sight.

use glam::Vec2;

use super::*;

impl Camera {
    /// Lets that many frames pass (`clock`: the seconds gone, for what
    /// shakes; `focus`: where it looked; `boxed`: a line shows in the box,
    /// at the top or not). Gives where it looks now.
    pub(crate) fn advance(&mut self, frames: f32, clock: f32, focus: Option<Vec2>, cast: &Bodies, boxed: bool, box_top: bool) -> Option<Vec2> {
        self.make_room(frames, boxed, box_top);
        // (Where it looked: who it keeps in sight, it goes to from there,
        // whatever else it was told.)
        let looked = focus;
        let focus = self.glide(frames, focus, cast);
        self.ease(frames);
        let shaken = self.shaken(frames, clock);
        self.pass_glance(frames, focus);
        self.keep_in_sight(frames, looked, focus, cast).map(|focus| focus + shaken)
    }

    /// The view moves off the dialogue box as a line shows, and stays so
    /// between lines: it does not bob with each of them.
    fn make_room(&mut self, frames: f32, boxed: bool, box_top: bool) {
        let side = if box_top { -1.0 } else { 1.0 };
        let wanted = match (self.roomy, boxed) {
            (true, true) => side * BOX_HIGH / 2.0 / self.zoom.now.max(0.1),
            (true, false) => self.room,
            (false, _) => 0.0,
        };
        self.room += (wanted - self.room) * (frames * 0.12).min(1.0);
    }

    /// Where the view looks now.
    fn glide(&mut self, frames: f32, mut focus: Option<Vec2>, cast: &Bodies) -> Option<Vec2> {
        // (The view gliding, slowly at its ends; else where it is sent.)
        if let Some(pan) = &mut self.pan {
            pan.left -= frames;
            let done = 1.0 - (pan.left / pan.frames).clamp(0.0, 1.0);
            let eased = pan.ease.along(done);
            let spot = match &pan.to {
                Cam::At(at) => Some(at.vec2()),
                Cam::Who(who) => standing(cast, *who),
            };
            let target = spot.map(|target| target + Vec2::Y * self.room);
            focus = target.map(|target| pan.from.lerp(target, eased)).or(focus);
            if pan.left <= 0.0 {
                self.camera = Some(match (std::mem::take(&mut self.pan_stays), spot) {
                    (true, Some(spot)) => Cam::at(spot),
                    _ => pan.to.clone(),
                });
                self.pan = None;
            }
        } else {
            let offset = Vec2::Y * self.room;
            focus = match self.camera {
                Some(Cam::At(ref at)) => Some(at.vec2() + offset),
                Some(Cam::Who(who)) => match (standing(cast, who).map(|at| at + offset), focus) {
                    (Some(there), Some(now)) if self.cam_lag > 0.0 => Some(now.lerp(there, (frames / self.cam_lag).min(1.0))),
                    (there, now) => there.or(now),
                },
                None => focus,
            };
        }
        focus
    }

    /// Its closeness going where it was sent.
    fn ease(&mut self, frames: f32) {
        let zoom = &mut self.zoom;
        if zoom.left > 0.0 {
            zoom.now += (zoom.to - zoom.now) * (frames / zoom.left).min(1.0);
            zoom.left -= frames;
        }
    }

    /// How far the view is thrown aside now.
    fn shaken(&mut self, frames: f32, clock: f32) -> Vec2 {
        // (Shaking: to and fro around where it looks, several times a
        // second.)
        if let Some(left) = &mut self.shake.left {
            *left -= frames;
            if *left <= 0.0 {
                self.shake = Shake::default();
            }
        }
        let Shake { axis, strength, .. } = self.shake;
        let swing = ((clock * 60.0 / 2.0).floor() as i32 % 2 * 2 - 1) as f32;
        let mut shaken = match strength > 0.0 {
            true => (if axis == 1 { Vec2::X } else { Vec2::Y }) * strength * swing,
            false => Vec2::ZERO,
        };
        // (A quake shakes both ways, less and less.)
        if let Some(quake) = &mut self.quake {
            quake.left -= frames;
            let now = quake.strength * (quake.left / quake.whole).max(0.0);
            shaken += Vec2::new(swing, if (clock * 60.0 / 3.0).floor() as i32 % 2 == 0 { 1.0 } else { -1.0 }) * now;
        }
        self.quake = self.quake.filter(|quake| quake.left > 0.0);
        shaken
    }

    /// A glance over, the view glides back.
    fn pass_glance(&mut self, frames: f32, focus: Option<Vec2>) {
        if let Some(glance) = &mut self.glance {
            glance.left -= frames;
        }
        if let (Some(Glance { back, frames, left }), Some(now)) = (self.glance.clone(), focus)
            && left <= 0.0
        {
            self.pan = Some(Pan { from: now, to: back, frames, left: frames, ease: self.ease });
            self.glance = None;
        }
    }

    /// Those it frames kept in sight: it comes to them from where it looked.
    fn keep_in_sight(&mut self, frames: f32, looked: Option<Vec2>, focus: Option<Vec2>, cast: &Bodies) -> Option<Vec2> {
        match self.fit(&self.framing, cast) {
            Some((middle, close)) => {
                self.zoom.now += (close - self.zoom.now) * (frames * 0.05).min(1.0);
                (self.zoom.to, self.zoom.left) = (self.zoom.now, 0.0);
                let middle = middle + Vec2::Y * self.room;
                Some(looked.map_or(middle, |now| now.lerp(middle, (frames * 0.1).min(1.0))))
            }
            None => focus,
        }
    }
}
