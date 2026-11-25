//! One attack: an attacker, a defender, a move, chance and the weather go
//! in; what happened comes out, in order, in plain data. The only place an
//! attack is played out: `run` tells what happened on the floor, `fight`
//! shows it on the stage. The numbers themselves are `dungeon::combat`'s
//! (damage, accuracy) and `dungeon::status`'s (effects).

use crate::dungeon::combat::{Damage, FIXED_ONE, Fighter, calc_damage_with};
use crate::dungeon::{Constants, DungeonRng, MoveEffect, MoveSlot, Pos, Stats, Statuses, TypeChart, apply_effect};

mod answers;
mod blows;
mod status_moves;
#[cfg(test)]
mod tests;

/// What of the victim's whole life a blast takes on a stage (a half).
const BLAST_SHARE: i32 = 2;
/// The rolls a foreseen blow is the mean of.
const FORESEEN_ROLLS: u32 = 4;

/// One of those an attack is between, borrowed from whoever keeps it.
pub struct Party<'a> {
    pub stats: &'a mut Stats,
    pub status: &'a mut Statuses,
    /// Of the player's team; its leader.
    pub team: bool,
    pub leader: bool,
    pub at: Pos,
    /// Who it is, for what comes back to it later (0 in a fight on scene).
    pub id: u32,
}

impl Party<'_> {
    fn alive(&self) -> bool {
        self.stats.is_alive()
    }

    fn fighter(&self) -> Fighter {
        self.status.fighter(self.stats, self.team)
    }
}

/// All an attack goes by besides those two.
pub struct Field<'a> {
    pub rng: &'a mut DungeonRng,
    pub constants: &'a Constants,
    pub chart: Option<&'a TypeChart>,
    /// By move number, for those that become another.
    pub moves: &'a [MoveSlot],
    pub weather: u8,
    pub gravity: bool,
    pub mud_sport: bool,
    pub water_sport: bool,
    /// What Secret Power does here.
    pub secret_power: u8,
    /// The tokens of Destiny Bond given so far.
    pub bonds: &'a mut u32,
}

/// A move about to land.
#[derive(Clone, Copy)]
pub struct Attack {
    /// One of a chain of linked moves.
    pub linked: bool,
    pub slot: MoveSlot,
    /// Set for status moves.
    pub effect: Option<MoveEffect>,
    pub damage_mult: i32,
}

/// What whoever plays the attack weighs its blows with (a fight's own).
#[derive(Clone, Copy)]
pub struct Tuning {
    /// What the blows are worth, all told.
    pub worth: f32,
    /// The most a strike takes.
    pub cap: Option<i32>,
    /// On a stage: there is no floor for a blast to blow up.
    pub staged: bool,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning { worth: 1.0, cap: None, staged: false }
    }
}

/// Which of the two something is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Victim,
}

/// A thing that happened, in the order it did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Happened {
    /// A sound move, not heard.
    Deaf,
    Missed,
    Failed,
    Bound,
    StatsSwapped,
    ItemsSwapped,
    Mimics(u16),
    AbilityCopied,
    AbilitiesSwapped,
    AbilityLost,
    AilmentsPassed,
    LifeShared,
    StatsCopied,
    Bounced,
    /// An effect on one of them.
    Effect {
        on: Role,
        effect: MoveEffect,
        applied: bool,
    },
    /// An effect that changes the floor, the user having cast it.
    Cast(MoveEffect),
    /// A blast on a stage: what it should take from the victim and the user.
    /// (Theirs to take: who is to win holds on.)
    Blast {
        taken: i32,
        spent: i32,
    },
    NotAsleep,
    Unshielded,
    /// The victim's Reflect or Light Screen broke (Brick Break).
    Shattered,
    /// The user crashes down and loses that.
    Crashed(i32),
    /// It stands a one-hit move.
    Stood,
    Absorbed,
    AbsorbedLife(i32),
    AbsorbedSpeed,
    Unafraid,
    Hurt(Damage),
    AttackTopped,
    RageGrows,
    /// The victim hurls the user away, in that direction.
    Hurled {
        facing: (i32, i32),
    },
    /// Something falls by the victim.
    Drops(u16),
    /// The user eats what the victim held.
    Ate(u16),
    /// The user must rest.
    Rests,
    Woken,
    /// The victim's paralysis is cured (SmellingSalt).
    Unparalyzed,
    /// The user broke free of what bound or sapped it (Rapid Spin).
    SpunFree,
    TypeChanged,
    Stole(u16),
    /// What the victim's item sent back.
    Returned(i32),
    /// What touching the victim cost.
    Contact(i32),
    /// What sucking on Liquid Ooze cost.
    Ooze(i32),
    /// Lost by the user past the blow (less than 0: given back).
    Aftermath(i32),
    Countered(i32),
    Grudge,
    /// The victim's Pass Scarf sent the effect on to someone next to it.
    Passed(MoveEffect),
}

/// All an attack did.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// It did not miss.
    pub landed: bool,
    pub happened: Vec<Happened>,
}

/// An attack lands: all it does to the two.
pub fn resolve(user: &mut Party, victim: &mut Party, attack: &Attack, tuning: Tuning, field: &mut Field) -> Outcome {
    let mut play = Play { field, tuning, happened: Vec::new() };
    let landed = play.land(user, victim, attack);
    Outcome { landed, happened: play.happened }
}

/// What a blow takes, about: rolled apart from any chance of a fight or a
/// floor, with nothing touched. For who weighs its moves.
pub fn foreseen(constants: &Constants, chart: Option<&TypeChart>, user: &Fighter, victim: &Fighter, slot: &MoveSlot, weather: u8) -> i32 {
    let mut rng = DungeonRng::new(1);
    let mut roll = || calc_damage_with(&mut rng, constants, chart, user, victim, slot.type_id, slot.power, 0, FIXED_ONE, slot.category == 1, weather).amount;
    (0..FORESEEN_ROLLS).map(|_| roll()).sum::<i32>() / FORESEEN_ROLLS as i32
}

/// An attack being played out.
struct Play<'a, 'b> {
    field: &'a mut Field<'b>,
    tuning: Tuning,
    happened: Vec<Happened>,
}

impl Play<'_, '_> {
    fn says(&mut self, happened: Happened) {
        self.happened.push(happened);
    }

    /// An effect on one of the two, told.
    fn affects(&mut self, effect: MoveEffect, party: &mut Party, on: Role) -> bool {
        let applied = self.apply(effect, party);
        self.says(Happened::Effect { on, effect, applied });
        applied
    }

    fn apply(&mut self, effect: MoveEffect, party: &mut Party) -> bool {
        apply_effect(effect, self.field.rng, self.field.constants, party.stats, party.status)
    }

    fn rolled(&mut self, chance: i32) -> bool {
        self.field.rng.rand_int(100) < chance
    }

    fn land(&mut self, user: &mut Party, victim: &mut Party, attack: &Attack) -> bool {
        use crate::dungeon::ability;
        if self.tuning.staged && matches!(attack.effect, Some(MoveEffect::Explode(_))) {
            self.says(Happened::Blast { taken: victim.stats.max_hp / BLAST_SHARE, spent: user.stats.hp });
            return true;
        }
        // Soundproof: sound moves are not heard.
        if ability::shields(user.stats, victim.stats, ability::SOUNDPROOF) && ability::SOUND_MOVES.contains(&attack.slot.id) {
            self.says(Happened::Deaf);
            return true;
        }
        match attack.effect {
            Some(effect) => self.status_move(user, victim, attack, effect),
            None => self.blows(user, victim, attack),
        }
    }
}

/// A monster as Gravity has it: no Flying type, no Levitate.
fn grounded(stats: &Stats) -> Stats {
    use crate::dungeon::ability::LEVITATE;
    const FLYING: u8 = 10;
    let mut stats = *stats;
    for kind in [&mut stats.types.0, &mut stats.types.1] {
        if *kind == FLYING {
            *kind = 0;
        }
    }
    for ability in &mut stats.abilities {
        if *ability == LEVITATE {
            *ability = 0;
        }
    }
    stats
}
