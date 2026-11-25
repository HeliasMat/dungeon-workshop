//! Traps sprung, blasts.

use super::*;

/// The kinds of traps, by the number the floor data gives them.
pub mod trap {
    pub(crate) const MUD: usize = 1;
    pub(crate) const STICKY: usize = 2;
    pub(crate) const GRIMY: usize = 3;
    pub(crate) const SUMMON: usize = 4;
    pub(crate) const PITFALL: usize = 5;
    pub(crate) const WARP: usize = 6;
    pub(crate) const GUST: usize = 7;
    pub(crate) const SPIN: usize = 8;
    pub(crate) const SLUMBER: usize = 9;
    pub(crate) const SLOW: usize = 10;
    pub(crate) const SEAL: usize = 11;
    pub(crate) const POISON: usize = 12;
    pub(crate) const SELFDESTRUCT: usize = 13;
    pub(crate) const EXPLOSION: usize = 14;
    pub(crate) const PP_ZERO: usize = 15;
    pub(crate) const CHESTNUT: usize = 16;
    pub(crate) const WONDER_TILE: usize = 17;
    pub const POKEMON: usize = 18;
    pub(crate) const SPIKED_TILE: usize = 19;
    pub(crate) const STEALTH_ROCK: usize = 20;
    pub(crate) const TOXIC_SPIKES: usize = 21;
    pub(crate) const TRIP: usize = 22;
    pub(crate) const RANDOM: usize = 23;
    pub(crate) const GRUDGE: usize = 24;
    /// Number of trap kinds.
    pub const COUNT: usize = 25;
}

/// What a trap did beyond changing the victim, for the caller to carry out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrapOutcome {
    /// Nothing more to do (including traps without an effect yet).
    Done,
    /// Move the victim to a random tile of the floor.
    Warp,
    /// Drop the victim to the next floor.
    Fall,
    /// This many enemies appear around the trap, holding a grudge (they
    /// come at the team fiercer) or not.
    Summon(i32, bool),
    /// Turn the items of the room (or of the 3x3 area around the trap, in a
    /// hallway) into enemies.
    ItemsToMonsters,
    /// Blow the victim away in a random direction.
    Blow,
    /// An explosion reaching this many tiles around the trap.
    Explode(i32),
    /// Turn the food in the bag into Grimy Food.
    GrimyFood,
    /// Make an item of the bag sticky.
    Sticky,
    /// Make the victim drop what it holds.
    Trip,
}

/// Applies a trap to the monster stepping on it. The status traps go
/// through [`apply_effect`], so Safeguard, Mist and the like guard
/// against them as against moves.
pub fn spring_trap(
    kind: usize,
    rng: &mut DungeonRng,
    constants: &Constants,
    stats: &mut Stats,
    statuses: &mut Statuses,
    moves: &mut [MoveSlot],
) -> TrapOutcome {
    let affect = |effect, rng: &mut DungeonRng, stats: &mut Stats, statuses: &mut Statuses| {
        apply_effect(effect, rng, constants, stats, statuses);
    };
    match kind {
        trap::MUD => affect(mud(rng), rng, stats, statuses),
        trap::SPIN => affect(MoveEffect::Confuse, rng, stats, statuses),
        trap::SLOW => affect(MoveEffect::TargetSpeedDown, rng, stats, statuses),
        trap::SLUMBER => affect(MoveEffect::Sleep, rng, stats, statuses),
        trap::POISON => affect(MoveEffect::Poison, rng, stats, statuses),
        // A few spikes, and the poison on them.
        trap::TOXIC_SPIKES => {
            stats.hp -= (constants.spiked_tile_damage / 2).max(1);
            affect(MoveEffect::Poison, rng, stats, statuses);
        }
        trap::CHESTNUT => stats.hp -= constants.chestnut_damage,
        trap::SPIKED_TILE => stats.hp -= constants.spiked_tile_damage,
        // Pointed stones: Rock types shrug them off.
        trap::STEALTH_ROCK if !of_type(stats, ROCK) => stats.hp -= constants.stealth_rock_damage,
        // (An exclusive item keeps its moves from being sealed.)
        trap::SEAL if !statuses.exclusive.has(crate::dungeon::exclusive::effect::NO_MOVE_DISABLING) => seal_a_move(rng, moves),
        trap::PP_ZERO => drain_a_move(rng, moves),
        trap::WONDER_TILE => statuses.reset_stages(),
        trap::PITFALL => {
            stats.hp -= constants.pitfall_trap_damage;
            return TrapOutcome::Fall;
        }
        trap::SUMMON => return TrapOutcome::Summon(rng.rand_range(2, 4), false),
        trap::GRUDGE => return TrapOutcome::Summon(rng.rand_range(3, 5), true),
        trap::WARP => return TrapOutcome::Warp,
        trap::GUST => return TrapOutcome::Blow,
        trap::SELFDESTRUCT => return TrapOutcome::Explode(1),
        trap::EXPLOSION => return TrapOutcome::Explode(2),
        trap::GRIMY => return TrapOutcome::GrimyFood,
        trap::STICKY => return TrapOutcome::Sticky,
        trap::TRIP => return TrapOutcome::Trip,
        trap::POKEMON => return TrapOutcome::ItemsToMonsters,
        trap::RANDOM => return spring_trap(any_other_trap(rng), rng, constants, stats, statuses, moves),
        _ => {}
    }
    TrapOutcome::Done
}

const ROCK: u8 = 13;
const FIRE: u8 = 2;

fn of_type(stats: &Stats, kind: u8) -> bool {
    stats.types.0 == kind || stats.types.1 == kind
}

/// The Mud Trap: one of the four fighting stats sinks by two stages.
fn mud(rng: &mut DungeonRng) -> MoveEffect {
    const SINKS: [MoveEffect; 4] =
        [MoveEffect::TargetAttack(-2), MoveEffect::TargetDefense(-2), MoveEffect::TargetSpAttack(-2), MoveEffect::TargetSpDefense(-2)];
    SINKS[rng.rand_int(4) as usize]
}

/// The Seal Trap: one more of the moves, picked at random, is sealed.
fn seal_a_move(rng: &mut DungeonRng, moves: &mut [MoveSlot]) {
    let open: Vec<usize> = (0..moves.len()).filter(|&at| !moves[at].sealed).collect();
    if let Some(&at) = open.get(rng.rand_int(open.len().max(1) as i32) as usize) {
        moves[at].sealed = true;
    }
}

/// The PP-Zero Trap: one move that still had PP has none left.
fn drain_a_move(rng: &mut DungeonRng, moves: &mut [MoveSlot]) {
    let full: Vec<usize> = (0..moves.len()).filter(|&at| moves[at].pp > 0).collect();
    if let Some(&at) = full.get(rng.rand_int(full.len().max(1) as i32) as usize) {
        moves[at].pp = 0;
    }
}

/// What a Random Trap turns into: any trap that does something, but a
/// Wonder Tile (a gift) or itself.
fn any_other_trap(rng: &mut DungeonRng) -> usize {
    let kinds: Vec<usize> = (1..trap::COUNT).filter(|&kind| kind != trap::WONDER_TILE && kind != trap::RANDOM).collect();
    kinds[rng.rand_int(kinds.len() as i32) as usize]
}

/// What an explosion does to a monster caught in it. The team is spared
/// the worst: its members lose half of the HP they have (at least 1);
/// anyone else takes the blast's damage for its reach. Fire types take
/// half as much.
pub fn explosion_damage(constants: &Constants, radius: i32, stats: &Stats, team_member: bool) -> i32 {
    let full = match team_member {
        true => stats.hp / 2,
        false => constants.explosion_damages[radius.clamp(0, 2) as usize],
    };
    let taken = if of_type(stats, FIRE) { full / 2 } else { full };
    if team_member { taken.max(1) } else { taken }
}
