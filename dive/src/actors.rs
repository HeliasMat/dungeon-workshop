//! Each monster as it is seen: where it has slid to, which of its
//! animations plays. What it is and where it stands are `Dungeon`'s.

use std::rc::Rc;

use glam::Vec2;
use rules::dungeon::Pos;
use rules::run::dungeon::STEP_FRAMES;
use rules::run::{Dungeon, Monster};
use shell::art::Art;
use shell::images::{Anim, Frame, Sprite};

use super::{TILE, tile_middle};

/// Pixels a frame of a monster sliding between tiles.
const SLIDE: f32 = TILE as f32 / STEP_FRAMES as f32;
/// A longer way than this (a warp) is jumped.
const JUMPED: f32 = TILE as f32 * 2.5;
/// Where a monster's own animations start among its animations (after
/// those of dungeons).
const OWN_ANIMATIONS: usize = 5;

/// Which way of a sprite faces that way: ways start facing down and turn
/// counter-clockwise.
pub fn way_of(facing: (i32, i32)) -> usize {
    match facing {
        (1, 1) => 1,
        (1, 0) => 2,
        (1, -1) => 3,
        (0, -1) => 4,
        (-1, -1) => 5,
        (-1, 0) => 6,
        (-1, 1) => 7,
        _ => 0,
    }
}

/// What a monster shows as: its own picture, or the one it took.
pub fn looks_of(monster: &Monster) -> Option<u16> {
    monster.looks_like.or(monster.species)
}

pub struct Actor {
    pub sprite: Option<Rc<Sprite>>,
    looks: Option<u16>,
    /// Where it shows, in pixels of the floor, and the tile it last went to.
    pub at: Vec2,
    tile: Pos,
    anim: Anim,
    /// The animation a move has it play, in place of `anim`.
    motion: Option<usize>,
    /// It goes back to resting once this animation has played.
    once: bool,
    way: usize,
    step: usize,
    ticks: u32,
    /// The frames before it takes its blow.
    pub hurt_in: Option<f32>,
    /// The frames its status icons have shown.
    pub icons: u32,
}

impl Actor {
    pub fn new(monster: &Monster, art: &Art) -> Self {
        let looks = looks_of(monster);
        Actor {
            sprite: looks.and_then(|looks| art.monster(looks)),
            looks,
            at: tile_middle(monster.at),
            tile: monster.at,
            anim: Anim::Idle,
            motion: None,
            once: false,
            way: way_of(monster.facing),
            step: 0,
            ticks: 0,
            hurt_in: None,
            icons: 0,
        }
    }

    pub fn start(&mut self, anim: Anim, once: bool) {
        (self.anim, self.once, self.motion, self.step, self.ticks) = (anim, once, None, 0, 0);
    }

    /// Plays once the animation a move gives its user: one of its own, or
    /// its attack without it. (Group 1 is the attack it already plays; past
    /// 12, none of a sprite's.)
    pub fn start_motion(&mut self, group: u8) {
        if !(2..=12).contains(&group) {
            return;
        }
        self.start(Anim::Attack, true);
        let index = OWN_ANIMATIONS + group as usize;
        if self.sprite.as_ref().is_some_and(|sprite| sprite.has(index)) {
            self.motion = Some(index);
        }
    }

    /// How long the animation it plays now lasts, in frames.
    pub fn playing_for(&self) -> u32 {
        self.frames().iter().map(|frame| frame.ticks).sum()
    }

    fn frames(&self) -> &[Frame] {
        let Some(sprite) = &self.sprite else {
            return &[];
        };
        let moving = self.motion.and_then(|motion| sprite.ways(motion).get(self.way)).filter(|steps| !steps.is_empty());
        moving.map_or_else(|| sprite.steps(self.anim, self.way).1, Vec::as_slice)
    }

    /// The picture it shows now.
    pub fn picture(&self) -> Option<&Frame> {
        let frames = self.frames();
        frames.get(self.step.min(frames.len().saturating_sub(1)))
    }

    /// A frame goes by for it.
    pub fn advance(&mut self, monster: &Monster, art: &Art) {
        if looks_of(monster) != self.looks {
            let at = self.at;
            *self = Self { at, ..Self::new(monster, art) };
        }
        let target = tile_middle(monster.at);
        if monster.at != self.tile {
            self.tile = monster.at;
            if self.anim != Anim::Walk && !self.once && self.at.distance(target) <= JUMPED {
                self.start(Anim::Walk, true);
            }
        }
        let (far, sliding) = (self.at.distance(target), self.at != target);
        self.at = match far <= SLIDE || far > JUMPED {
            true => target,
            false => self.at + (target - self.at) / far * SLIDE,
        };
        if let Some(left) = &mut self.hurt_in {
            *left -= 1.0;
            if *left <= 0.0 {
                self.hurt_in = None;
                self.start(Anim::Hurt, true);
            }
        }
        let way = way_of(monster.facing);
        if way != self.way {
            (self.way, self.step, self.ticks) = (way, 0, 0);
        }
        let resting = if monster.status.asleep() { Anim::Sleep } else { Anim::Idle };
        if !self.once && self.anim != resting {
            self.start(resting, false);
        }
        let Some(ticks) = self.picture().map(|frame| frame.ticks.max(1)) else {
            return;
        };
        self.ticks += 1;
        if self.ticks < ticks {
            return;
        }
        self.ticks = 0;
        self.step += 1;
        if self.step >= self.frames().len() {
            self.step = 0;
            // (Who still walks goes on walking.)
            if self.once && !(sliding && self.anim == Anim::Walk) {
                self.start(resting, false);
            }
        }
    }
}

/// Where a move or an effect shows on whoever stands on a tile, from its
/// middle.
pub fn point_on(dungeon: &Dungeon, art: &Art, tile: Pos, point: u8) -> Vec2 {
    let found = dungeon.at(tile).and_then(looks_of).map_or((0, 0), |looks| art.point(looks, point));
    Vec2::new(found.0 as f32, found.1 as f32)
}
