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
    /// Given Brick-Tough's HP (see `iq::keep_boosts`).
    pub tough: bool,
    /// The max HP its exclusive items gave, and the moves they gave PP
    /// (see `keep_item_boosts`).
    pub item_hp: i32,
    pub item_pp: Vec<u16>,
}

/// What an exclusive item adds to each move's PP (the game's, read from
/// its arm9).
const ITEM_PP_BOOST: i32 = 2;

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
            tough: false,
            item_hp: 0,
            item_pp: Vec::new(),
        }
    }

    pub fn alive(&self) -> bool {
        self.stats.is_alive()
    }

    /// What Brick-Tough and Multitalent gave taken back, as the team leaves
    /// the dungeon; and what its exclusive items gave.
    pub(crate) fn unboost(&mut self) {
        let skills = std::mem::take(&mut self.stats.iq_skills);
        crate::dungeon::iq::keep_boosts(&mut self.stats, &mut self.moves, &mut self.tough);
        self.stats.iq_skills = skills;
        let exclusive = std::mem::take(&mut self.status.exclusive);
        self.keep_item_boosts();
        self.status.exclusive = exclusive;
    }

    /// The max HP and each move's PP its exclusive items add, given or
    /// taken back as they come and go (and to a move learnt meanwhile).
    pub(crate) fn keep_item_boosts(&mut self) {
        let hp = self.status.exclusive.max_hp_boost();
        if hp != self.item_hp {
            let by = hp - std::mem::replace(&mut self.item_hp, hp);
            self.stats.max_hp = (self.stats.max_hp + by).max(1);
            self.stats.hp = match by > 0 && self.stats.hp > 0 {
                true => self.stats.hp + by,
                false => self.stats.hp.min(self.stats.max_hp),
            };
        }
        let boosted = self.status.exclusive.has(crate::dungeon::exclusive::effect::MAX_PP_BOOST);
        for slot in &mut self.moves {
            if self.item_pp.contains(&slot.id) == boosted {
                continue;
            }
            let by = if boosted { ITEM_PP_BOOST } else { -ITEM_PP_BOOST };
            slot.max_pp = (slot.max_pp + by).max(1);
            slot.pp = if boosted { slot.pp + by } else { slot.pp.min(slot.max_pp) };
        }
        self.item_pp = if boosted { self.moves.iter().map(|slot| slot.id).collect() } else { Vec::new() };
    }

    /// One of the player's team.
    pub fn team(&self) -> bool {
        matches!(self.side, Side::Leader | Side::Ally)
    }
}
