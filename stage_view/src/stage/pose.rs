//! How someone shows at the moment: which animation, which step.

use std::rc::Rc;

use engine::Pose;

use super::SCENE_ANIMATIONS;
use shell::images::{Frame, Sprite};

/// The animations every monster has.
pub(super) const WALK: usize = 0;
pub(super) const IDLE: usize = 1;
pub(super) const HURT: usize = 2;
pub(super) const SLEEP: usize = 3;
pub(super) const ATTACK: usize = 4;

/// How someone of the stage shows at the moment.
#[derive(Clone)]
pub(super) struct Shown {
    pub(super) sprite: Rc<Sprite>,
    /// Its animation, its step, the 60ths of a second gone in that step.
    pub(super) animation: usize,
    pub(super) frame: usize,
    pub(super) elapsed: f32,
    /// It goes back to how it stands at its end.
    pub(super) once: bool,
    /// One the scene chose (see `BodyBeat::Anim`), that stays until another:
    /// it starts again at its end or stays on its last step; it stays on
    /// its first.
    pub(super) staged: bool,
    pub(super) looped: bool,
    pub(super) frozen: bool,
    /// The times it still plays, past this one.
    pub(super) again: u16,
    /// Lying on the ground, under everyone.
    pub(super) flat: bool,
    pub(super) shadow: Option<u8>,
}

impl Shown {
    pub(super) fn start(&mut self, animation: usize, once: bool) {
        (self.animation, self.once, self.frame, self.elapsed) = (animation, once, 0, 0.0);
        (self.staged, self.looped, self.frozen, self.again) = (false, true, false, 0);
    }

    /// A step of an animation of its own, held.
    pub(super) fn held(&mut self, group: usize, step: usize) {
        self.stage(group, false, true);
        if self.has(SCENE_ANIMATIONS + group) {
            self.frame = step;
        }
    }

    /// An animation of its own so many times, then as it stands.
    pub(super) fn times(&mut self, group: usize, times: u16) {
        let index = SCENE_ANIMATIONS + group;
        if self.has(index) {
            self.start(index, true);
            (self.staged, self.again) = (true, times.saturating_sub(1));
        }
    }

    /// An animation of its own, if it has it.
    pub(super) fn stage(&mut self, group: usize, looped: bool, frozen: bool) {
        let index = SCENE_ANIMATIONS + group;
        match self.has(index) {
            true => {
                self.start(index, false);
                (self.staged, self.looped, self.frozen) = (true, looped, frozen);
            }
            // (Once and no more: as it stands. Else, its way of standing.)
            false if !looped => {}
            false => self.start(IDLE, false),
        }
    }

    pub(super) fn has(&self, index: usize) -> bool {
        self.sprite.has(index)
    }

    /// A move's motion: one of its own animations, or its attack.
    pub(super) fn motion(&mut self, group: u8) {
        self.start(self.motion_of(group), true);
    }

    fn motion_of(&self, group: u8) -> usize {
        let index = SCENE_ANIMATIONS + group as usize;
        if self.has(index) { index } else { ATTACK }
    }

    /// The 60ths of a second from a move's motion starting to the step its
    /// sprite marks for the blow, that way (none marked: none).
    pub(super) fn hit_of(&self, group: u8, way: usize) -> Option<f32> {
        let ways = self.sprite.ways(self.motion_of(group));
        let steps = ways.get(way % ways.len().max(1))?;
        let hit = steps.iter().position(|frame| frame.mark == assets::tables::HIT_MARK)?;
        Some(steps[..hit].iter().map(|frame| frame.ticks as f32).sum())
    }

    pub(super) fn steps(&self, way: usize) -> &[Frame] {
        let ways = self.sprite.ways(self.showing());
        ways.get(way % ways.len().max(1)).map_or(&[], Vec::as_slice)
    }

    pub(super) fn advance(&mut self, frames: f32, way: usize) {
        let count = self.steps(way).len();
        if count == 0 || self.frozen {
            return;
        }
        self.frame %= count;
        self.elapsed += frames;
        loop {
            let ticks = self.steps(way).get(self.frame).map_or(1, |frame| frame.ticks) as f32;
            if self.elapsed < ticks {
                return;
            }
            self.elapsed -= ticks;
            if self.frame + 1 < count {
                self.frame += 1;
            } else if self.again > 0 {
                (self.again, self.frame) = (self.again - 1, 0);
            } else if self.once {
                self.start(IDLE, false);
                return;
            } else if !self.looped {
                self.elapsed = 0.0;
                return;
            } else {
                self.frame = 0;
            }
        }
    }

    /// The animation it shows: the one asked if it has it.
    pub(super) fn showing(&self) -> usize {
        if self.has(self.animation) { self.animation } else { IDLE }
    }

    /// The step of its animation it shows.
    pub(super) fn step(&self) -> usize {
        self.frame
    }

    pub(super) fn picture(&self, way: usize) -> Option<&Frame> {
        let steps = self.steps(way);
        steps.get(self.frame.min(steps.len().saturating_sub(1)))
    }
}

pub(super) fn pose_of(pose: Pose) -> usize {
    match pose {
        Pose::Idle => IDLE,
        Pose::Walk => WALK,
        Pose::Hurt => HURT,
        Pose::Sleep => SLEEP,
        Pose::Attack => ATTACK,
    }
}

/// The row of a sheet for a way faced (1 to 8).
pub(super) fn way_of(direction: u16) -> usize {
    (direction.max(1) as usize - 1) % 8
}
