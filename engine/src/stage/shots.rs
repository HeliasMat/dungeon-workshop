//! What a move throws, on its way from its user to its target ([`Shot`]).

use super::*;

/// What a move throws, on its way from its user to its target: who is hit,
/// and what shows on it, when it gets there. (Of no effect: what comes a
/// little later, seen nowhere meanwhile.)
#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub effect: u16,
    pub from: Vec2,
    pub target: Who,
    pub hit: u16,
    pub gone: f32,
    /// The frames it flies for.
    pub whole: f32,
    /// It flies in an arc.
    pub lob: bool,
    /// Who it gets to takes the blow.
    pub hurts: bool,
}

/// The frames a shot flies for; how high one flies in an arc (as in dungeons).
pub const SHOT_FRAMES: f32 = 16.0;
pub(super) const LOB_HEIGHT: f32 = 24.0;

impl Shot {
    /// Where it is, on its way to where its target stands.
    pub fn at(&self, to: Vec2) -> Vec2 {
        self.from.lerp(to, (self.gone / self.whole).min(1.0))
    }

    /// How high it is above its way.
    pub fn rise(&self) -> f32 {
        let done = (self.gone / self.whole).clamp(0.0, 1.0);
        if self.lob { LOB_HEIGHT * 4.0 * done * (1.0 - done) } else { 0.0 }
    }
}

impl Stage {
    /// Something thrown at someone, who is hit when it gets there.
    #[cfg(test)]
    pub(crate) fn shoot(&mut self, who: Who, target: Who, effect: u16, hit: u16) {
        self.shoot_as(who, target, effect, hit, SHOT_FRAMES, false, true);
    }

    /// The same, in so many frames, in an arc or not; on someone who only
    /// shows it, or who takes it.
    pub fn shoot_as(&mut self, who: Who, target: Who, effect: u16, hit: u16, whole: f32, lob: bool, hurts: bool) {
        if let Some(from) = self.cast.get(&who).map(|actor| actor.position) {
            self.shots.push(Shot { effect, from, target, hit, gone: 0.0, whole: whole.max(1.0), lob, hurts });
        }
    }

    /// Those thrown go on; those there hit.
    pub(super) fn pass_shots(&mut self, frames: f32) {
        for shot in std::mem::take(&mut self.shots) {
            let gone = shot.gone + frames;
            if gone < shot.whole {
                self.shots.push(Shot { gone, ..shot });
                continue;
            }
            if let Some(actor) = self.cast.get_mut(&shot.target).filter(|_| shot.hurts) {
                actor.hits += 1;
                actor.cue(Cue::Hurt);
            }
            if shot.hit != 0 {
                self.effect(shot.target, shot.hit, None);
            }
        }
    }
}
