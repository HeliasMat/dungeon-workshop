//! The light and what is in the air ([`Light`]): the fade, the light's
//! colour, a flash, the weather, the spotlight and an iris, particles, black
//! wiping over, the picture shown instead of the scene, ripples, bolts.

use super::*;

/// A circle wide enough to show the whole screen around its middle (an
/// iris wholly open).
pub(super) const WHOLE_SCREEN: f32 = 180.0;

/// The light and what is in the air.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Light {
    pub fade: Fade,
    pub tint: Tint,
    pub flash: Flash,
    /// What is in the air over the scene.
    pub weather: Weather,
    pub spotlight: Spotlight,
    /// The particles about the scene (see `ScreenBeat::Particles`).
    pub particles: Vec<Particle>,
    /// What gives off more of them (see `ScreenBeat::Emitter`).
    pub emitters: Vec<Emitter>,
    /// Black coming over the screen from a side (see `ScreenBeat::Wipe`).
    pub wipe: Wipe,
    /// The picture shown instead of the scene.
    pub cutaway: Option<Cutaway>,
    /// The screen has lost its colours (see `ScreenBeat::Grey`).
    pub grey: bool,
    /// Everyone's pictures stay as they are (see `ScreenBeat::Still`).
    pub still: bool,
    /// How much the screen ripples now, where that goes and the frames it
    /// still takes (see `ScreenBeat::Wave`).
    pub wave: f32,
    pub waving: Option<Tween<f32>>,
    /// Bolts of lightning striking: where, the frames left, and what draws
    /// their zigzag (see `ScreenBeat::Lightning`).
    pub bolts: Vec<Bolt>,
    /// More holes in the dark of the spotlight: on whom or where, how wide
    /// (see `ViewBeat::SpotlightAdd`).
    pub lights: Vec<Lit>,
    /// Soft lights about the scene (see `ScreenBeat::Halo`): on whom or where,
    /// how wide, of what colour.
    pub halos: Vec<Halo>,
}

/// A circle of light, the rest dark: on what, how wide, where it is now,
/// and how much it shows (0 to 1: it comes and goes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spotlight {
    pub on: Option<Cam>,
    pub radius: f32,
    pub at: Vec2,
    pub shown: f32,
    /// Wholly black around (an iris), not only dark; the width it goes
    /// to and the frames it still takes.
    pub black: bool,
    pub closing: Option<Tween<f32>>,
}

/// Black coming over the screen from a side (see `ScreenBeat::Wipe`): how much
/// of the screen it covers (0 to 1), where that goes, the frames it still
/// takes, and the side it comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wipe {
    pub shown: f32,
    pub to: f32,
    pub left: f32,
    pub side: u8,
}

/// A bolt of lightning: where it strikes, the frames it still shows, what
/// its zigzag is drawn from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bolt {
    pub at: Vec2,
    pub left: f32,
    pub drawn: u32,
}

/// A picture shown instead of the scene, which goes on behind it (see
/// `ScreenBeat::Cutaway`).
#[derive(Clone, Debug, PartialEq)]
pub struct Cutaway {
    pub level: u16,
    pub at: Vec2,
    /// Its light on the scene, what is dark of it leaving the scene seen.
    pub over: bool,
    /// Where the view glides, the frames that takes still.
    pub pan: Option<(Vec2, f32)>,
    /// The one it comes over, the frames left of that and all it takes.
    pub before: Option<Box<Cutaway>>,
    pub coming: (f32, f32),
}

impl Cutaway {
    pub(super) fn advance(&mut self, frames: f32) {
        if let Some((to, left)) = self.pan {
            let step = frames.min(left);
            self.at += (to - self.at) * step / left;
            self.pan = (left > step).then_some((to, left - step));
        }
        self.coming.0 = (self.coming.0 - frames).max(0.0);
        if self.coming.0 == 0.0 {
            self.before = None;
        }
        if let Some(before) = &mut self.before {
            before.advance(frames);
        }
    }

    /// How much of it shows (0 to 1).
    pub fn shown(&self) -> f32 {
        1.0 - self.coming.0 / self.coming.1
    }
}

/// How dark (or white) the screen is (0: not at all, 1: wholly), where it
/// goes, by how much a frame, and whether it is white.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fade {
    pub level: f32,
    pub to: f32,
    pub speed: f32,
    pub white: bool,
}

impl Fade {
    /// The screen plainly seen.
    pub const CLEAR: Fade = Fade { level: 0.0, to: 0.0, speed: 1.0, white: false };
    /// The screen dark.
    pub const DARK: Fade = Fade { level: 1.0, to: 1.0, speed: 1.0, white: false };

    /// How much of the screen it hides, going and ending softly.
    pub fn shown(&self) -> f32 {
        super::motion::eased(self.level)
    }
}

impl Default for Fade {
    fn default() -> Self {
        Fade::CLEAR
    }
}

/// The light's colour (red, green, blue, and how much of it, of 255),
/// where it goes, and the frames left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tint {
    pub now: [f32; 4],
    pub to: [f32; 4],
    pub left: f32,
}

/// A flash's colour (its last part: how much is left, of 255), and how
/// much it fades a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flash {
    pub colour: [f32; 4],
    pub speed: f32,
}

impl Light {
    /// A frame of the light: its colour going where it was sent, a flash
    /// fading, the picture instead of the scene gliding, the fade; the
    /// particles going their way, the ripples, the bolts, black wiping over.
    pub(super) fn pass(&mut self, frames: f32) {
        let tint = &mut self.tint;
        if tint.left > 0.0 {
            let step = (frames / tint.left).min(1.0);
            for (now, to) in tint.now.iter_mut().zip(tint.to.iter()) {
                *now += (*to - *now) * step;
            }
            tint.left -= frames;
        }
        self.flash.colour[3] = (self.flash.colour[3] - self.flash.speed * frames).max(0.0);
        if let Some(cutaway) = &mut self.cutaway {
            cutaway.advance(frames);
        }
        let fade = &mut self.fade;
        fade.level = match (fade.to - fade.level).abs() <= fade.speed * frames {
            true => fade.to,
            false => fade.level + fade.speed * frames * (fade.to - fade.level).signum(),
        };
        for particle in &mut self.particles {
            particle.at += particle.speed * frames;
            particle.speed.y += particle.fall * frames;
            particle.left -= frames;
        }
        self.particles.retain(|particle| particle.left > 0.0);
        Tween::run(&mut self.waving, &mut self.wave, frames);
        for bolt in &mut self.bolts {
            bolt.left -= frames;
        }
        self.bolts.retain(|bolt| bolt.left > 0.0);
        if self.wipe.left > 0.0 {
            let step = frames.min(self.wipe.left);
            self.wipe.shown += (self.wipe.to - self.wipe.shown) * (step / self.wipe.left);
            self.wipe.left -= step;
        }
    }

    /// Whether it still changes: a fade, a dimming, a tint, an iris, a wipe.
    pub(super) fn busy(&self) -> bool {
        self.fade.level != self.fade.to || self.tint.left > 0.0 || self.spotlight.closing.is_some() || self.wipe.left > 0.0
    }
}

impl Stage {
    /// More particles come off those who give them off.
    pub(super) fn emit(&mut self, frames: f32) {
        for at in 0..self.light.emitters.len() {
            let Emitter { kind, who, between, colour, .. } = self.light.emitters[at];
            let emitter = &mut self.light.emitters[at];
            emitter.wait -= frames;
            emitter.left = emitter.left.map(|left| left - frames);
            let Some(there) = self.cast.get(&who).map(|actor| Stage::source(kind, actor)) else {
                continue;
            };
            while self.light.emitters[at].wait <= 0.0 {
                self.light.emitters[at].wait += between;
                let particle = self.particle(kind, there, colour);
                self.light.particles.push(Particle { sharp: true, ..particle });
            }
        }
        let cast = &self.cast;
        self.light.emitters.retain(|emitter| cast.contains_key(&emitter.who) && emitter.left.is_none_or(|left| left > 0.0));
    }

    /// The spotlight on what it shines on, coming and going; an iris
    /// closing or opening, once wholly open gone.
    pub(super) fn pass_spotlight(&mut self, frames: f32) {
        let spotlight = &mut self.light.spotlight;
        let lit = match spotlight.on.clone() {
            Some(Cam::At(at)) => Some(at.vec2()),
            Some(Cam::Who(who)) => self.cast.get(&who).map(|actor| actor.position - Vec2::new(0.0, 12.0)),
            None => None,
        };
        if let Some(lit) = lit {
            spotlight.at = if spotlight.shown <= 0.0 { lit } else { spotlight.at.lerp(lit, (frames * 0.2).min(1.0)) };
        }
        if let Some(Tween { to, .. }) = spotlight.closing {
            Tween::run(&mut spotlight.closing, &mut spotlight.radius, frames);
            if spotlight.closing.is_none() && to >= WHOLE_SCREEN {
                *spotlight = Spotlight { at: spotlight.at, ..Spotlight::default() };
            }
        }
        let wanted = if spotlight.on.is_some() { 1.0 } else { 0.0 };
        spotlight.shown += (wanted - spotlight.shown).clamp(-frames / 20.0, frames / 20.0);
    }
}
