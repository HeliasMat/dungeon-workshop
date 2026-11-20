//! Monster stats, damage calculation and enemy decision making.
//!
//! The rules are this project's own, written to play like the mystery
//! dungeon games without copying them: see [`calc_damage_with`] for the
//! damage, [`move_hits`] for accuracy, [`targets_in_range`] for reach.

mod moves;
pub use moves::*;
mod chart;
pub use chart::*;
mod damage;
pub use damage::*;

use super::floor::{Floor, Mobility, Pos, Tile};
use super::rng::DungeonRng;
use super::status::{Constants, MoveEffect, STAGE_COUNT, is_fixed_damage, move_effect};
use super::weather;

/// Enemies notice a target in the same room, or this close in hallways.
const HALLWAY_SIGHT: i32 = 2;

/// One unit in 8-bit fixed point, the unit of the damage multipliers
/// moves and statuses pass around.
pub const FIXED_ONE: i32 = 0x100;

/// The eight directions in circular order, starting facing down.
pub(super) const AROUND: [(i32, i32); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub level: i32,
    pub max_hp: i32,
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub sp_attack: i32,
    pub sp_defense: i32,
    /// Type ids; the second is 0 for single-type monsters.
    pub types: (u8, u8),
    /// The species' two abilities, both at work (see
    /// [`super::ability`]); 0 for none.
    pub abilities: [u8; 2],
    /// The item it holds (see [`super::held`]); 0 for none.
    pub held: u16,
    /// Its IQ points, its species' IQ group and the IQ skills it has on,
    /// as bits by skill (see [`super::iq`]).
    pub iq: i32,
    pub iq_group: u8,
    pub iq_skills: u128,
    /// A female. Saved with the monster; the rules of blows and aim treat
    /// both genders alike.
    #[serde(default)]
    pub female: bool,
}

impl Stats {
    pub(crate) fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Wait,
    Move(i32, i32),
    /// Regular attack towards the given direction.
    Attack(i32, i32),
}

/// Decision for one enemy at `from` hunting `target`.
pub fn enemy_action(floor: &Floor, rng: &mut DungeonRng, from: Pos, target: Pos, occupied: impl Fn(Pos) -> bool, mobility: Mobility) -> Action {
    let (dx, dy) = (target.x - from.x, target.y - from.y);
    let distance = dx.abs().max(dy.abs());
    if distance == 1 && floor.can_step(from, dx, dy) {
        return Action::Attack(dx, dy);
    }

    let free = |sx: i32, sy: i32| (sx, sy) != (0, 0) && floor.can_step_as(from, sx, sy, mobility) && !occupied(Pos::new(from.x + sx, from.y + sy));
    let same_room = floor.tile(from).room().is_some() && floor.tile(from).room() == floor.tile(target).room();
    if same_room || distance <= HALLWAY_SIGHT {
        // Straight at the target first, then slide along either axis.
        let (sx, sy) = (dx.signum(), dy.signum());
        for (cx, cy) in [(sx, sy), (sx, 0), (0, sy)] {
            if free(cx, cy) {
                return Action::Move(cx, cy);
            }
        }
        return Action::Wait;
    }

    let (wx, wy) = (rng.rand_range(-1, 2), rng.rand_range(-1, 2));
    if free(wx, wy) { Action::Move(wx, wy) } else { Action::Wait }
}

#[cfg(test)]
mod tests;
