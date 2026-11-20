//! A monster on a floor: the leader, those of its team, foes, Kecleon. All
//! of them live in one list, `Dungeon::monsters`, in the order they act.

use crate::dungeon::{Belly, MoveSlot, Pos, Stats, Statuses};

use super::team::Experience;

/// A monster's name in the list: it stays when others die in a turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MonsterId(pub u32);

/// Which side a monster is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The one the player moves.
    Leader,
    /// A member of the team: it decides by itself.
    Ally,
    Foe,
    /// Kecleon keeping its shop: no one's foe until there is a thief.
    Keeper,
}

/// What an enemy is, for it to ask to join the team: its species and its
/// chances out of 1000.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recruitable {
    pub species: u16,
    pub rate: i16,
}

/// Abilities and types a monster had before it took others'.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Identity {
    pub abilities: [u8; 2],
    pub types: (u8, u8),
}

#[derive(Clone)]
pub struct Monster {
    pub id: MonsterId,
    pub side: Side,
    pub name: String,
    /// Which monster it is (none: a stand-in).
    pub species: Option<u16>,
    pub stats: Stats,
    pub moves: Vec<MoveSlot>,
    pub status: Statuses,
    /// Only the leader gets hungry.
    pub belly: Option<Belly>,
    pub at: Pos,
    pub facing: (i32, i32),
    /// What defeating it is worth, before `defeat_exp`.
    pub exp_value: i32,
    /// Of a team member.
    pub experience: Option<Experience>,
    /// Of a foe that may ask to join.
    pub recruitable: Option<Recruitable>,
    /// A Monster House's, asleep until the leader walks in.
    pub dormant: bool,
    /// A member past the leader and its partner, and the recruit it is, by
    /// its place in the list, if it is one.
    pub extra: Option<Option<usize>>,
    /// The number of the job it is about, among those the team holds.
    pub job_target: Option<usize>,
    /// The job it is escorted for.
    pub escorted: Option<usize>,
    /// An outlaw running while it has more than half its HP.
    pub fleeing: bool,
    pub identity: Option<Identity>,
    /// What it shows as instead of its own sprite (Transform; a disguise).
    pub looks_like: Option<u16>,
    /// Defeated: the frames before it leaves the list, seen blinking out.
    pub fading: Option<u32>,
}

impl Monster {
    /// One with nothing yet of its own but what is given.
    pub fn new(id: MonsterId, side: Side, name: String, species: Option<u16>, stats: Stats, moves: Vec<MoveSlot>, status: Statuses, at: Pos) -> Self {
        Self {
            id,
            side,
            name,
            species,
            stats,
            moves,
            status,
            belly: None,
            at,
            facing: (0, 1),
            exp_value: 0,
            experience: None,
            recruitable: None,
            dormant: false,
            extra: None,
            job_target: None,
            escorted: None,
            fleeing: false,
            identity: None,
            looks_like: None,
            fading: None,
        }
    }

    pub fn alive(&self) -> bool {
        self.stats.is_alive()
    }

    /// One of the player's team.
    pub fn team(&self) -> bool {
        matches!(self.side, Side::Leader | Side::Ally)
    }
}
