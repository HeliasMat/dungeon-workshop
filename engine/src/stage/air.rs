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
    /// Shown so far round its point, at that angle (radians), turning so
    /// much a frame (0: on its point): a strand winding round a body.
    pub swirl: f32,
    pub angle: f32,
    pub spin: f32,
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
    /// One of the particles of a kind (see `ScreenBeat::Particles`), from a
    /// point, off a body of that size (see `Stage::girth`: strands wind
    /// round it).
    pub fn particle(&mut self, kind: ParticleKind, at: Vec2, body: Vec2, colour: Option<[u8; 3]>) -> Particle {
        pub(super) const FIREWORKS: [[u8; 3]; 6] = [[255, 90, 90], [255, 220, 80], [120, 200, 255], [140, 255, 140], [255, 140, 230], [255, 255, 255]];
        let mut drawn = || self.chance(1000) as f32 / 1000.0;
        let (way, far) = (drawn(), drawn());
        let (spread, life) = (drawn(), drawn());
        let mut particle = Particle {
            at,
            speed: Vec2::ZERO,
            fall: 0.0,
            left: 0.0,
            life: 0.0,
            colour: [255, 255, 255],
            blinks: false,
            sharp: false,
            ring: 0.0,
            swirl: 0.0,
            angle: 0.0,
            spin: 0.0,
        };
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
            // (Bubbles: going up from all round, wobbling a little; sharp
            // pixels of water, no glow.)
            ParticleKind::Bubbles => {
                particle.at += Vec2::new((way - 0.5) * 24.0, (far - 0.5) * 12.0);
                particle.speed = Vec2::new((spread - 0.5) * 0.3, -0.5 - far * 0.5);
                (particle.life, particle.blinks, particle.sharp) = (45.0 + life * 30.0, true, true);
                particle.colour = colour.unwrap_or([150, 215, 255]);
            }
            // (Dust: coming down softly from above.)
            ParticleKind::Dust => {
                particle.at += Vec2::new((way - 0.5) * 36.0, -24.0 * far);
                particle.speed = Vec2::new((spread - 0.5) * 0.3, 0.25 + far * 0.2);
                particle.life = 60.0 + life * 40.0;
                particle.colour = colour.unwrap_or([235, 220, 150]);
            }
            // (Drops: flung every way, falling; sharp pixels of water, no glow.)
            ParticleKind::Drops => {
                particle.speed = Vec2::from_angle(way * std::f32::consts::TAU) * (0.7 + far);
                (particle.fall, particle.life, particle.sharp) = (0.07, 22.0 + life * 16.0, true);
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
            // (Cinders: about the body, rising a little and dying; sharp
            // pixels, no glow.)
            ParticleKind::Cinders => {
                particle.at += Vec2::new(((way - 0.5) * 28.0).round(), ((far - 0.5) * 18.0).round());
                particle.speed = Vec2::new((spread - 0.5) * 0.2, -0.25 - far * 0.2);
                (particle.life, particle.sharp) = (30.0 + life * 30.0, true);
                particle.colour = colour.unwrap_or(if spread < 0.5 { [255, 120, 32] } else { [255, 208, 64] });
            }
            // (A zap: a spark flung off fast, gone at once; sharp pixels.)
            ParticleKind::Zap => {
                particle.at += Vec2::new(((way - 0.5) * 12.0).round(), ((far - 0.5) * 10.0).round());
                particle.speed = Vec2::from_angle(spread * std::f32::consts::TAU) * (1.2 + far);
                (particle.life, particle.sharp) = (8.0 + life * 8.0, true);
                particle.colour = colour.unwrap_or(if life < 0.5 { [255, 240, 96] } else { [255, 255, 255] });
            }
            // (A drip: off the side of the head, sliding down; sharp pixels.)
            ParticleKind::Drip => {
                particle.at += Vec2::new(if way < 0.5 { -8.0 } else { 8.0 }, 0.0);
                particle.speed = Vec2::new(0.0, 0.15);
                (particle.fall, particle.life, particle.sharp) = (0.02, 34.0 + life * 10.0, true);
                particle.colour = colour.unwrap_or([104, 184, 255]);
            }
            // (Leaves: from about the head, falling slowly aside; sharp pixels.)
            ParticleKind::Leaves => {
                particle.at += Vec2::new(((way - 0.5) * 30.0).round(), (-far * 6.0).round());
                particle.speed = Vec2::new((spread - 0.5) * 0.6, 0.3 + far * 0.2);
                (particle.life, particle.sharp) = (50.0 + life * 30.0, true);
                particle.colour = colour.unwrap_or(if spread < 0.5 { [96, 200, 72] } else { [56, 152, 56] });
            }
            // (Strands: two, winding up round the body from its feet to
            // its head, in bands of the rainbow; sharp pixels.)
            ParticleKind::Strands => {
                const RAINBOW: [[u8; 3]; 6] = [[248, 56, 56], [248, 152, 40], [248, 232, 64], [72, 216, 72], [64, 144, 248], [184, 88, 248]];
                // (From its feet to a little over its head, a turn and a
                // half, a band of colour every few pixels.)
                let tall = body.y;
                let high = (way * tall).round();
                particle.at += Vec2::new(0.0, -high);
                particle.speed = Vec2::new(0.0, -0.15);
                particle.swirl = body.x;
                particle.angle = high / tall * 1.25 * std::f32::consts::TAU + if far < 0.5 { 0.0 } else { std::f32::consts::PI };
                particle.spin = 0.16;
                (particle.life, particle.sharp) = (26.0 + life * 10.0, true);
                particle.colour = colour.unwrap_or(RAINBOW[(high / 4.0) as usize % RAINBOW.len()]);
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
    /// at its feet, on the ground; cinders and zaps about its middle, leaves
    /// just over its head.
    pub(super) fn source(kind: ParticleKind, actor: &Actor) -> Vec2 {
        match kind {
            ParticleKind::Rings | ParticleKind::Strands => actor.position,
            ParticleKind::Cinders | ParticleKind::Zap => actor.over_head(12.0),
            ParticleKind::Leaves | ParticleKind::Drip => actor.over_head(30.0),
            _ => actor.over_head(16.0),
        }
    }

    /// How far round and how high strands wind about someone: from its
    /// feet to near the top of its body (as the view knows it), a little
    /// wider than it.
    pub(super) fn girth(&self, who: Who) -> Vec2 {
        let body = self.view.body(who);
        Vec2::new((body.x * 0.5 + 2.0).clamp(8.0, 26.0), (body.y * 0.85).clamp(14.0, 64.0))
    }

    /// Where a swirling particle is shown (see `Particle::swirl`): round its
    /// point, seen from above at a slant, its round closing as it dies;
    /// whether it is behind the body.
    pub fn swirled(particle: &Particle) -> (Vec2, bool) {
        if particle.swirl <= 0.0 {
            return (particle.at, false);
        }
        let wide = particle.swirl * (0.55 + 0.45 * (particle.left / particle.life).clamp(0.0, 1.0));
        let (sin, cos) = particle.angle.sin_cos();
        (particle.at + Vec2::new((cos * wide).round(), (sin * wide * 0.45).round()), sin < 0.0)
    }

    /// A number below `below`, drawn by chance.
    pub(super) fn chance(&mut self, below: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.seed >> 16) % below.max(1)
    }
}
