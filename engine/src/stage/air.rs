//! What goes about the stage's air: particles, and words rising.

use super::*;

/// A spark of the scene's particles (see `ScreenBeat::Particles`): where it is,
/// where it goes each frame and how that bends (falling, or rising when
/// below 0), the frames it has left of those it lives, its colour, and
/// whether it blinks (a firefly).
#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    pub at: Vec2,
    pub speed: Vec2,
    pub fall: f32,
    pub left: f32,
    pub life: f32,
    pub colour: [u8; 3],
    pub blinks: bool,
    /// A square of its colour, without the glow of the others.
    pub sharp: bool,
    /// A ring on water instead, widening to that radius (0: a spark).
    pub ring: f32,
}

/// What gives off particles (see `ScreenBeat::Emitter`): what, off whom, the
/// frames between two, of what colour, the frames it still lasts, and those
/// before the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emitter {
    pub kind: ParticleKind,
    pub who: Who,
    pub between: f32,
    pub colour: Option<[u8; 3]>,
    pub left: Option<f32>,
    pub wait: f32,
}

impl Stage {
    /// One of the particles of a kind (see `ScreenBeat::Particles`), from a point.
    pub fn particle(&mut self, kind: ParticleKind, at: Vec2, colour: Option<[u8; 3]>) -> Particle {
        pub(super) const FIREWORKS: [[u8; 3]; 6] = [[255, 90, 90], [255, 220, 80], [120, 200, 255], [140, 255, 140], [255, 140, 230], [255, 255, 255]];
        let mut drawn = || self.chance(1000) as f32 / 1000.0;
        let (way, far) = (drawn(), drawn());
        let (spread, life) = (drawn(), drawn());
        let mut particle = Particle { at, speed: Vec2::ZERO, fall: 0.0, left: 0.0, life: 0.0, colour: [255, 255, 255], blinks: false, sharp: false, ring: 0.0 };
        match kind {
            // (Fireworks: every way at once, then falling.)
            ParticleKind::Fireworks => {
                particle.speed = Vec2::from_angle(way * std::f32::consts::TAU) * (0.6 + far * 1.2);
                (particle.fall, particle.life) = (0.025, 70.0 + life * 30.0);
                particle.colour = colour.unwrap_or(FIREWORKS[(spread * FIREWORKS.len() as f32) as usize % FIREWORKS.len()]);
            }
            // (Sparks: thrown up, falling fast.)
            ParticleKind::Sparks => {
                particle.speed = Vec2::from_angle(-std::f32::consts::FRAC_PI_2 + (way - 0.5) * 1.6) * (0.8 + far);
                (particle.fall, particle.life) = (0.06, 25.0 + life * 15.0);
                particle.colour = colour.unwrap_or([255, 230, 120]);
            }
            // (Fireflies: here and there about the point, drifting, blinking.)
            ParticleKind::Fireflies => {
                particle.at += Vec2::new((way - 0.5) * 80.0, (far - 0.5) * 50.0);
                particle.speed = Vec2::new((spread - 0.5) * 0.3, (life - 0.5) * 0.2);
                (particle.life, particle.blinks) = (240.0 + life * 180.0, true);
                particle.colour = colour.unwrap_or([200, 255, 120]);
            }
            // (Smoke: pale, rising and spreading, slowly.)
            ParticleKind::Smoke => {
                particle.at += Vec2::new((way - 0.5) * 12.0, 0.0);
                particle.speed = Vec2::new((spread - 0.5) * 0.5, -0.25 - far * 0.25);
                particle.life = 70.0 + life * 50.0;
                particle.colour = colour.unwrap_or([200, 200, 205]);
            }
            // (Bubbles: going up from all round, wobbling a little.)
            ParticleKind::Bubbles => {
                particle.at += Vec2::new((way - 0.5) * 24.0, (far - 0.5) * 12.0);
                particle.speed = Vec2::new((spread - 0.5) * 0.3, -0.5 - far * 0.5);
                (particle.life, particle.blinks) = (45.0 + life * 30.0, true);
                particle.colour = colour.unwrap_or([150, 215, 255]);
            }
            // (Dust: coming down softly from above.)
            ParticleKind::Dust => {
                particle.at += Vec2::new((way - 0.5) * 36.0, -24.0 * far);
                particle.speed = Vec2::new((spread - 0.5) * 0.3, 0.25 + far * 0.2);
                particle.life = 60.0 + life * 40.0;
                particle.colour = colour.unwrap_or([235, 220, 150]);
            }
            // (Drops: flung every way, falling.)
            ParticleKind::Drops => {
                particle.speed = Vec2::from_angle(way * std::f32::consts::TAU) * (0.7 + far);
                (particle.fall, particle.life) = (0.07, 22.0 + life * 16.0);
                particle.colour = colour.unwrap_or([120, 190, 255]);
            }
            // (A healing glow: going straight up, blinking.)
            ParticleKind::Healing => {
                particle.at += Vec2::new((way - 0.5) * 26.0, (far - 0.5) * 14.0 + 12.0);
                particle.speed = Vec2::new(0.0, -0.5 - far * 0.4);
                (particle.life, particle.blinks) = (40.0 + life * 25.0, true);
                particle.colour = colour.unwrap_or([150, 255, 170]);
            }
            // (Rings: on the water where it is (at its feet, see `Stage::source`),
            // still, widening as they fade; sharp, as the game's.)
            ParticleKind::Rings => {
                particle.at += Vec2::new((way - 0.5) * 6.0, (far - 0.5) * 4.0);
                (particle.life, particle.sharp, particle.ring) = (40.0 + life * 24.0, true, 9.0 + spread * 8.0);
                particle.colour = colour.unwrap_or([230, 245, 255]);
            }
            // (Embers: rising slowly, swaying.)
            ParticleKind::Embers => {
                particle.at += Vec2::new((way - 0.5) * 20.0, 0.0);
                particle.speed = Vec2::new((spread - 0.5) * 0.2, -0.3 - far * 0.3);
                particle.life = 100.0 + life * 60.0;
                particle.colour = colour.unwrap_or([255, 140, 40]);
            }
        }
        particle.left = particle.life;
        particle
    }

    /// Where particles of a kind come off someone: over its head, or rings
    /// at its feet, on the ground.
    pub(super) fn source(kind: ParticleKind, actor: &Actor) -> Vec2 {
        match kind {
            ParticleKind::Rings => actor.position,
            _ => actor.over_head(16.0),
        }
    }

    /// A number below `below`, drawn by chance.
    pub(super) fn chance(&mut self, below: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.seed >> 16) % below.max(1)
    }
}
