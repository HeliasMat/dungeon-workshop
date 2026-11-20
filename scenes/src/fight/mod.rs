//! A fight: the only place its rules are. Fighters from their species and
//! level, sides, relays, what foes choose, moves as the dungeons count them
//! (`calc_damage_with`, `move_hits`, statuses), berries, seeds, shields,
//! the winner chosen. Pure: no window, no real time; `Battle::step` gives
//! what happens as `Sight`s, and `stage` turns those into the lines of a
//! scene. `host` plays them while a scene holds; `hand` makes `Sight`s of
//! blows a film writes itself.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use glam::Vec2;

use engine::Who;
use rules::dungeon::combat::range;
use rules::dungeon::status::{Constants, Statuses};
use rules::dungeon::{LevelGain, MoveSlot, Stats, TypeChart};
use rules::tables::Tables;

mod battle;
pub mod crowd;
pub mod ground;
mod hand;
pub mod host;
pub mod said;
pub mod stage;
#[cfg(test)]
mod tests;

pub use battle::{Battle, Body};
pub use crowd::Crowd;
pub use ground::Ground;
pub use host::Fighting;

/// A move as a fight knows it: what the dungeons count with, what it is
/// called, how fast what it throws flies.
#[derive(Clone, Debug)]
pub struct Move {
    pub slot: MoveSlot,
    pub name: String,
    pub flight: f32,
    pub flies: bool,
}

impl Move {
    /// Whether it strikes, or does something else.
    pub fn strikes(&self) -> bool {
        self.slot.power > 0 || rules::dungeon::status::is_fixed_damage(self.slot.id) || self.slot.category != STATUS_MOVE
    }

    /// Whether it goes up to its target: a physical one, of those that
    /// reach the one in front.
    pub fn touches(&self) -> bool {
        self.slot.category == 0 && self.slot.range == range::FRONT && self.slot.power > 0
    }
}

/// What the game calls a status move.
pub(crate) const STATUS_MOVE: u8 = 2;

/// What every monster of a kind shares, as far as fights go.
#[derive(Clone, Debug, Default)]
pub struct Kind {
    pub name: String,
    pub types: (u8, u8),
    /// Life, attack, defense, special attack, special defense.
    pub base: [i32; 5],
    pub growth: Vec<LevelGain>,
    pub movement: u8,
    pub learnset: Vec<(u8, u16)>,
}

impl Kind {
    /// Its stats at a level: its base, and all each level gave it.
    pub fn stats(&self, level: i32) -> Stats {
        let [mut hp, mut attack, mut defense, mut sp_attack, mut sp_defense] = self.base;
        for gain in self.growth.iter().skip(1).take((level - 1).max(0) as usize) {
            hp += gain.hp;
            attack += gain.attack;
            defense += gain.defense;
            sp_attack += gain.sp_attack;
            sp_defense += gain.sp_defense;
        }
        let max_hp = hp.max(1);
        Stats {
            level,
            max_hp,
            hp: max_hp,
            attack,
            defense,
            sp_attack,
            sp_defense,
            types: self.types,
            abilities: [0; 2],
            held: 0,
            iq: 0,
            iq_group: 0,
            iq_skills: 0,
            female: false,
        }
    }
}

/// All a fight goes by, taken once from the game's tables.
#[derive(Clone, Debug, Default)]
pub struct Rules {
    pub constants: Constants,
    pub chart: Option<TypeChart>,
    /// By move.
    pub moves: Vec<Move>,
    /// The same, as an attack reads them.
    slots: Vec<MoveSlot>,
    /// Those in the fight, by species.
    pub kinds: HashMap<u16, Kind>,
}

impl Rules {
    pub fn new(constants: Constants, chart: Option<TypeChart>, moves: Vec<Move>, kinds: HashMap<u16, Kind>) -> Self {
        let slots = moves.iter().map(|known| known.slot).collect();
        Rules { constants, chart, moves, slots, kinds }
    }

    /// What a fight of those species needs of the game's tables.
    pub fn of(tables: &Tables, species: &[u16]) -> Self {
        Self::of_kin(tables, species, &|id| id)
    }

    /// The same, a variant being of its kin: `kin` says whose tables it uses.
    pub fn of_kin(tables: &Tables, species: &[u16], kin: &dyn Fn(u16) -> u16) -> Self {
        let moves = tables.move_slots.iter().enumerate().map(|(id, slot)| Move {
            slot: *slot,
            name: tables.moves.get(id).cloned().unwrap_or_default(),
            flight: tables.move_motions.get(id).map_or(6.0, |motion| (motion.flight as f32).max(1.0)),
            flies: tables.move_effects.get(id).is_some_and(|effects| effects[3] != 0),
        });
        let kinds = species.iter().filter_map(|&id| {
            let species = tables.species.get(kin(id) as usize)?;
            let learnset = species.learnset.iter().map(|(level, slot)| (*level, slot.id)).collect();
            Some((
                id,
                Kind {
                    name: species.name.clone(),
                    types: species.types,
                    base: species.base,
                    growth: species.growth.clone(),
                    movement: species.movement,
                    learnset,
                },
            ))
        });
        Rules::new(tables.constants, tables.types.clone(), moves.collect(), kinds.collect())
    }

    /// As many moves as a Pokémon knows.
    const MOVES_KNOWN: usize = 4;

    /// The last moves someone has learnt at its level, of those a fight
    /// knows what to do with.
    pub fn known(&self, species: u16, level: i32) -> Vec<Move> {
        let learnt = self.kinds.get(&species).map_or(&[][..], |kind| &kind.learnset);
        let mut known: Vec<u16> = Vec::new();
        for &(_, id) in learnt.iter().filter(|(at, _)| *at as i32 <= level) {
            known.retain(|&other| other != id);
            known.push(id);
        }
        let usable: Vec<Move> = known.into_iter().filter_map(|id| self.moves.get(id as usize)).filter(|known| battle::of_use(known)).cloned().collect();
        let found = usable[usable.len().saturating_sub(Self::MOVES_KNOWN)..].to_vec();
        if found.is_empty() { vec![self.plain_move()] } else { found }
    }

    /// What anyone can do.
    fn plain_move(&self) -> Move {
        const TACKLE: usize = 154;
        self.moves.get(TACKLE).cloned().unwrap_or(Move { slot: MoveSlot::fallback_regular_attack(), name: "Tackle".to_string(), flight: 6.0, flies: false })
    }
}

/// How a fight goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Whoever watches chooses for the first side.
    Played,
    /// By itself, one at a time on each side.
    Alone,
    /// By itself, all on the field.
    Together,
}

/// Someone in a fight, before it starts.
#[derive(Clone, Debug)]
pub struct Entrant {
    pub who: Who,
    pub species: u16,
    pub level: i32,
    /// Its life, if it is not the game's: left, most.
    pub life: Option<(i32, i32)>,
    /// Its moves, if they are not those of its level.
    pub moves: Vec<u16>,
    /// A seed that brings it back when it falls.
    pub seed: bool,
    /// It waits off the fight until someone gives it its place.
    pub waits: bool,
}

/// How a film says a blow went; what it does not say comes from the rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Told {
    Real,
    Misses,
    Critical,
    /// It brings the target down, whatever is left of it.
    Falls,
    Takes(i32),
    /// It stops on a shield the target raises.
    Shielded,
}

/// A turn a film wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    pub who: Who,
    pub power: u16,
    /// Those it is on; none, the first foe standing.
    pub targets: Vec<Who>,
    /// On all the foes standing.
    pub all: bool,
    pub told: Told,
    /// Shown as a critical blow, whatever it takes.
    pub critical: bool,
    /// All of it slowed down.
    pub slow: bool,
    /// Slowed down as it lands, for so many frames.
    pub slowed_hit: Option<i32>,
    /// Who steps in and takes it instead.
    pub taken_by: Option<Who>,
}

impl Act {
    pub fn of(who: Who, power: u16) -> Self {
        Act { who, power, targets: Vec::new(), all: false, told: Told::Real, critical: false, slow: false, slowed_hit: None, taken_by: None }
    }
}

/// What a film states of a fight, in its order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stated {
    Act(Act),
    /// A passage of the film, by its label, between two blows.
    Passage(String),
    /// So many frames go by.
    Pause(i32),
    /// A line of the log, in its own words.
    Log(String),
    /// Someone gives its place to one who waits.
    Change {
        out: Who,
        comes: Who,
    },
    /// Someone eats or uses a thing (none: as much as a berry gives).
    Uses {
        who: Who,
        thing: Who,
        name: String,
        worth: Option<i32>,
    },
    /// Someone throws a thing at someone.
    Flings {
        who: Who,
        thing: Who,
        name: String,
        worth: Option<i32>,
        target: Who,
    },
}

/// A fight stated in advance: all on the field, its acts in turn.
#[derive(Clone, Debug, Default)]
pub struct Direction {
    pub acts: VecDeque<Stated>,
    /// Past the acts it goes on by chance; else it stops there.
    pub goes_on: bool,
    /// Who is to win: it does not fall, whoever else does.
    pub chosen: Option<Who>,
    /// The most turns it has, all told.
    pub turns: Option<u32>,
}

/// All a fight starts from.
#[derive(Clone, Debug)]
pub struct Setup {
    pub mode: Mode,
    pub sides: [Vec<Entrant>; 2],
    /// The side that is to win, whatever comes.
    pub winner: Option<usize>,
    pub seed: u32,
    /// The weather, as the dungeons number it (`dungeon::weather`).
    pub sky: u8,
    /// What each side's blows are worth, if it is not worked out.
    pub edges: Option<(f32, f32)>,
    /// How many times its life the other side has.
    pub foes_life: f32,
    /// The most of its life a foe's blow takes, of a hundred, in a fight
    /// one plays.
    pub hardest: i32,
    pub direction: Option<Direction>,
}

/// What a move can leave someone with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ailment {
    Burnt,
    Frozen,
    Paralysed,
    Asleep,
    Poisoned,
    Confused,
}

/// A stat a move changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Attack,
    Defense,
    SpAttack,
    SpDefense,
    Accuracy,
    Evasion,
    Speed,
}

/// What became of someone's life: what was taken (less than 0: given
/// back), what is left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strike {
    pub taken: i32,
    pub left: i32,
    /// It would have fallen; it holds on.
    pub held: bool,
}

/// What a move did to someone.
#[derive(Clone, Debug, PartialEq)]
pub struct Landing {
    pub target: Who,
    pub strikes: Vec<Strike>,
    pub missed: bool,
    /// It stopped on a shield.
    pub blocked: bool,
    pub critical: bool,
    /// Overall type matchup (`dungeon::combat::matchup`).
    pub matchup: u8,
    pub ailment: Option<Ailment>,
    pub stats: Vec<(Stat, bool)>,
}

/// A move used on someone.
#[derive(Clone, Debug)]
pub struct Blow {
    pub who: Who,
    pub power: Move,
    pub landings: Vec<Landing>,
    /// What it gives back to or costs who strikes.
    pub drained: Option<Strike>,
    pub recoil: Option<Strike>,
    /// It must rest after this.
    pub spent: bool,
    /// It blows up.
    pub blast: Option<Strike>,
    /// What it changes of who uses it.
    pub stats: Vec<(Stat, bool)>,
    /// Beside another's: all goes on at the same moment.
    pub beside: bool,
}

/// What a move did that strikes no one.
#[derive(Clone, Debug)]
pub struct Deed {
    pub who: Who,
    pub power: Move,
    pub healed: Option<Strike>,
    pub stats: Vec<(Stat, bool)>,
    /// On the other, if it aims at it.
    pub landing: Option<Landing>,
}

/// What happens in a fight, as it is told and shown; none of it says how.
#[derive(Clone, Debug)]
pub enum Sight {
    /// Both see each other: it starts.
    Opens {
        first: Who,
        second: Who,
    },
    StepsAside {
        who: Who,
        other: Who,
    },
    Blow(Blow),
    Deed(Deed),
    Berry {
        who: Who,
        healed: Strike,
    },
    /// Someone eats a thing of the film's.
    Uses {
        who: Who,
        thing: Who,
        name: String,
        healed: Strike,
    },
    /// Someone throws a thing at someone.
    Flings {
        who: Who,
        thing: Who,
        name: String,
        target: Who,
        strike: Strike,
    },
    Shields {
        who: Who,
    },
    /// Its turn goes by, of that.
    Stopped {
        who: Who,
        ailment: Ailment,
    },
    Rests {
        who: Who,
    },
    /// A burn, poison, or (`harsh`) the sky takes a little of its life.
    Worn {
        who: Who,
        strike: Strike,
        harsh: bool,
    },
    Cured {
        who: Who,
        ailment: Ailment,
    },
    Reborn {
        who: Who,
        healed: Strike,
    },
    /// It falls; `more`: its side has others to come.
    Falls {
        who: Who,
        more: bool,
    },
    /// The next of a side comes in, its life as it was left.
    Enters {
        side: usize,
        who: Who,
        left: i32,
        most: i32,
    },
    /// Who fought rests; it goes back.
    GoesBack {
        side: usize,
        who: Who,
    },
    /// Two of a side strike together.
    Partners {
        first: Who,
        second: Who,
    },
    /// Who fell is out of the fight, all being on it: its bar gone, it lies
    /// where it fell (no one takes its place), to get up when the story says.
    Leaves {
        who: Who,
    },
    /// One side has won, at that level.
    Won {
        side: usize,
        level: i32,
    },
    /// A passage of the film, by its label.
    Passage(String),
    /// So many frames go by.
    Waits(i32),
    /// A line of the log, in a film's words.
    Says(String),
    /// Time goes that fast from now on (1: as usual).
    Tempo(f32),
    /// The next blow to land is slowed down for so many frames.
    SlowHit(i32),
    /// Someone steps between a blow and who it was for.
    StepsBetween {
        who: Who,
        target: Who,
        from: Who,
    },
    /// Who stepped in goes back.
    StepsBack {
        who: Who,
    },
    /// Someone gives its place to another, and its bar.
    Swaps {
        out: Who,
        comes: Who,
        left: i32,
        most: i32,
    },
}

/// One of the things whoever plays can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Move(usize),
    Berry,
    Shield,
    /// The list of who can come in.
    Changing,
    Change(usize),
    Back,
}

/// A line of the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub said: String,
    pub choice: Choice,
}

/// Where people stand, as a fight of crowds counts it.
pub(crate) type Spots = HashMap<Who, Vec2>;

/// The rules, shared by what a fight is made of.
pub type Shared = Arc<Rules>;
