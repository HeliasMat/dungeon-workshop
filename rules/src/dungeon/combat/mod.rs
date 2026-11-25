//! Monster stats, damage calculation and enemy decision making.
//!
//! The damage and accuracy are the game's formulas and tables (see
//! [`calc_damage_with`], [`move_hits`]); the reach of moves and the way
//! foes walk are this project's ([`targets_in_range`], [`enemy_action`]).

mod moves;
pub use moves::*;
mod chart;
pub use chart::*;
mod damage;
pub use damage::*;

use super::floor::{Floor, Mobility, Pos, Tile, chebyshev};
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
    /// A female. Saved with the monster; females aim and dodge by the
    /// game's female tables, and land critical hits a little less often.
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

/// Where a monster walks when it hunts no one: the way it faces, and
/// the tile it heads for, kept from turn to turn.
pub struct Roaming<'a> {
    pub facing: (i32, i32),
    pub goal: &'a mut Option<Pos>,
}

/// Decision for one enemy at `from` hunting `target`; out of sight of it,
/// it roams (`roam`).
pub fn enemy_action(
    floor: &Floor,
    rng: &mut DungeonRng,
    from: Pos,
    target: Pos,
    occupied: impl Fn(Pos) -> bool,
    mobility: Mobility,
    roaming: Roaming,
) -> Action {
    let (dx, dy) = (target.x - from.x, target.y - from.y);
    let distance = dx.abs().max(dy.abs());
    if distance == 1 && floor.can_step(from, dx, dy) {
        return Action::Attack(dx, dy);
    }

    let free = |sx: i32, sy: i32| (sx, sy) != (0, 0) && floor.can_step_as(from, sx, sy, mobility) && !occupied(Pos::new(from.x + sx, from.y + sy));
    let same_room = floor.tile(from).room().is_some() && floor.tile(from).room() == floor.tile(target).room();
    if same_room || distance <= HALLWAY_SIGHT {
        *roaming.goal = None;
        // Straight at the target first, then slide along either axis.
        let (sx, sy) = (dx.signum(), dy.signum());
        for (cx, cy) in [(sx, sy), (sx, 0), (0, sy)] {
            if free(cx, cy) {
                return Action::Move(cx, cy);
            }
        }
        return Action::Wait;
    }
    roam(floor, rng, from, occupied, mobility, roaming)
}

/// The turns tried from the way a monster faces, as the game's
/// `FACING_DIRECTION_INCREMENTS`: ahead, then a little aside, back last.
const TURNS: [i32; 8] = [0, 1, -1, 2, -2, 3, -3, 4];

/// A hallway tile where ways part: more than two ways on from it.
fn at_crossing(floor: &Floor, at: Pos) -> bool {
    [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().filter(|&(dx, dy)| floor.can_step(at, dx, dy)).count() > 2
}

/// Hunting no one, as the game's `CalculateAiTargetPos`: in a hallway it
/// goes on the way it faces (at a crossing, any way but back); in a room
/// it heads for one of the room's exits, picked at random, then out by it
/// into the hallway; in a room without exits, a step any way.
pub fn roam(floor: &Floor, rng: &mut DungeonRng, from: Pos, occupied: impl Fn(Pos) -> bool, mobility: Mobility, roaming: Roaming) -> Action {
    let Roaming { facing, goal } = roaming;
    let free = |(dx, dy): (i32, i32)| (dx, dy) != (0, 0) && floor.can_step_as(from, dx, dy, mobility) && !occupied(Pos::new(from.x + dx, from.y + dy));
    let step = |way: (i32, i32)| if free(way) { Action::Move(way.0, way.1) } else { Action::Wait };
    let ahead = AROUND.iter().position(|&way| way == facing).unwrap_or(0) as i32;
    let Some(room) = floor.tile(from).room() else {
        *goal = None;
        let crossing = at_crossing(floor, from);
        let first = if crossing { rng.rand_int(8) } else { ahead };
        let back = AROUND[(ahead + 4).rem_euclid(8) as usize];
        let ways = TURNS.iter().map(|turn| AROUND[(first + turn).rem_euclid(8) as usize]);
        return ways.filter(|&way| !(crossing && way == back)).find(|&way| free(way)).map_or(Action::Wait, step);
    };
    // Its exit reached: out into the hallway.
    if floor.is_junction(from) && goal.is_none_or(|goal| goal == from) {
        *goal = None;
        let first = rng.rand_int(8);
        let out = (0..8).map(|turn| AROUND[((first + turn) % 8) as usize]);
        let hallway = |(dx, dy): (i32, i32)| floor.tile(Pos::new(from.x + dx, from.y + dy)) == Tile::Floor { room: None };
        if let Some(way) = out.filter(|&way| hallway(way)).find(|&way| free(way)) {
            return step(way);
        }
    }
    if goal.is_none_or(|goal| goal == from || floor.tile(goal).room() != Some(room)) {
        let exits: Vec<Pos> = Floor::positions().filter(|&tile| floor.tile(tile).room() == Some(room) && floor.is_junction(tile)).collect();
        *goal = None;
        // (Ten tries at an exit it is not on, as the game's.)
        for _ in 0..10 {
            let Some(&exit) = exits.get(rng.rand_int(exits.len().max(1) as i32) as usize) else {
                break;
            };
            if exit != from {
                *goal = Some(exit);
                break;
            }
        }
    }
    let Some(to) = *goal else {
        return step(AROUND[rng.rand_int(8) as usize]);
    };
    if chebyshev(from, to) == 1 {
        return step((to.x - from.x, to.y - from.y));
    }
    match super::ai::step_towards(floor, from, to, mobility, &occupied) {
        Some(way) => step(way),
        // (No way there for now: another exit next time.)
        None => {
            *goal = None;
            Action::Wait
        }
    }
}

#[cfg(test)]
mod tests;
