//! What a monster suffers from: the conditions and their changes.

use super::*;

/// The protections a monster can put up; a new one replaces the current.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shield {
    /// Moves used on the monster fail.
    Protect,
    /// No status problem can be inflicted.
    Safeguard,
    /// Stats cannot be lowered.
    Mist,
    /// Physical damage taken is halved.
    Reflect,
    /// Special damage taken is halved.
    LightScreen,
    /// Any hit leaves at least 1 HP.
    Endure,
    /// Changes the natural regeneration.
    Wish,
    /// No critical hit lands on it (Lucky Chant).
    LuckyChant,
    /// Heals regularly.
    AquaRing,
}

/// The turn count of a sleep that never wears off by itself: monsters
/// found asleep on a floor, or turned to stone, stay so until hit.
pub(crate) const DEEP_SLEEP: i32 = 999;

#[derive(Clone, Copy, Debug)]
pub struct Statuses {
    /// Turns of sleep left.
    pub sleep_turns: i32,
    pub poisoned: bool,
    pub(super) poison_timer: i32,
    /// Poisoned badly (Toxic): it hurts more and more, and no HP come back.
    pub badly_poisoned: bool,
    pub(super) bad_poison_ticks: i32,
    /// Turns left of each status. Burn, poison and paralysis are the
    /// "lasting" problems: a monster has one of them at most.
    pub burn_turns: i32,
    pub(super) burn_timer: i32,
    pub paralysis_turns: i32,
    pub frozen_turns: i32,
    pub cringe_turns: i32,
    pub confused_turns: i32,
    /// Turns left of terror: it flees and does not attack.
    pub terrified_turns: i32,
    /// Turns left of invisibility: the foes do not see it.
    pub invisible_turns: i32,
    /// Turns left of Magic Coat (Rebound Orb): the moves that would give it
    /// a status go back to their user.
    pub bouncing_turns: i32,
    /// Turns left of Snatch (Snatch Orb): what a foe's move would do for
    /// that foe goes to it instead.
    pub snatching_turns: i32,
    /// A decoy (Decoy Orb): the other foes go for it until the floor is
    /// left.
    pub decoy: bool,
    pub decoy_turns: i32,
    /// Follow Me: the foes go for it first.
    pub drawing_turns: i32,
    /// Vital Throw: who strikes it up close is hurled.
    pub vital_throw_turns: i32,
    /// What the exclusive items in the team's bag do for it.
    pub exclusive: crate::dungeon::exclusive::Exclusive,
    /// Conversion 2: hit by a move, it takes a type that resists it.
    pub conversion2_turns: i32,
    /// Turns left of Leech Seed, and the turns since its last tick.
    pub leech_seed_turns: i32,
    pub(super) leech_seed_timer: i32,
    /// Leech Seed: who sowed it (`MonsterId`), and the HP drained since
    /// they last went to it (see the caller).
    pub leech_sower: Option<u32>,
    pub leech_drained: i32,
    /// Turns left before it falls asleep (Yawn).
    pub yawning_turns: i32,
    /// Turns left of Taunt: no status moves.
    pub taunted_turns: i32,
    /// Turns left of Sure Shot (no missing) and of infatuation (no
    /// attacking).
    pub sure_shot_turns: i32,
    pub infatuated_turns: i32,
    /// Raging (Rage): hurt, its attack rises.
    pub raging: bool,
    /// What Stockpile stored (up to 3).
    pub stockpile: i32,
    /// A trap it lays under itself (Spikes), for the caller to lay.
    pub laying: Option<usize>,
    /// Turns left of sleeplessness (Worry Seed).
    pub sleepless_turns: i32,
    /// Charged (Charge): its next Electric move hits twice as hard.
    pub charged: bool,
    /// Turns left of floating (Magnet Rise), of Embargo (no items) and of
    /// Shadow Hold (no moving: Mean Look, Block).
    pub levitating_turns: i32,
    pub embargo_turns: i32,
    pub held_turns: i32,
    /// Its species' weight, as Low Kick and Grass Knot's multiplier (8
    /// fraction bits; see the caller).
    pub weight: i32,
    /// Constricted: stuck in place, hurt every so often.
    pub constricted_turns: i32,
    pub(super) constrict_timer: i32,
    pub(super) speed_boost_timer: i32,
    /// The last move it used, and the turns left of Encore (that move
    /// again and again).
    pub last_move: Option<u16>,
    pub encore_turns: i32,
    /// Turns left of seeing things (Cross-Eyed: an X-Eye Seed): it acts
    /// as confused, and the others look strange to it.
    pub cross_eyed_turns: i32,
    /// Asleep in a nightmare (hurt on waking), its last move to lose its PP
    /// (Spite, a Grudge), its Grudge, the turns left of Heal Block and the
    /// move Torment forbids.
    pub nightmare: bool,
    pub spited: bool,
    pub grudge: bool,
    pub heal_block_turns: i32,
    pub tormented: Option<u16>,
    /// Turns left of a Counter status, and which; the count of Perish Song.
    pub counter: Option<CounterKind>,
    pub counter_turns: i32,
    pub perish_turns: i32,
    /// Turns left of set damage (Doom Desire) and of a pause (Imprison).
    pub set_damage_turns: i32,
    pub paused_turns: i32,
    /// Its own abilities and types, while a move has changed them for the
    /// floor (see `remember_identity`).
    pub identity: Option<([u8; 2], (u8, u8))>,
    /// A move Sketch is to be replaced with.
    pub sketched: Option<u16>,
    /// Asleep from Rest: on waking, healed (`napping_heal` HP) and cured.
    pub napping: bool,
    /// Bide, Revenge: the damage taken while biding, and what it sends
    /// back as it is unleashed (see the caller).
    pub bide: Option<i32>,
    pub bide_release: Option<i32>,
    /// A Destiny Bond: its token (the same on the two bound), whether this
    /// is the one whose losses are shared, and the turns left.
    pub bond: Option<(u32, bool)>,
    pub bond_turns: i32,
    /// Transformed: which of the floor's monsters it looks like, and the
    /// turns left.
    pub disguise: Option<u16>,
    pub disguise_turns: i32,
    /// It walks through walls and over any terrain until the floor is
    /// left (Mobile Orb).
    pub mobile: bool,
    /// What it throws flies until a wall stops it (Longtoss Orb), and
    /// goes through everyone in the way (Pierce Orb), until the floor is
    /// left.
    pub long_toss: bool,
    pub piercing: bool,
    /// Its blows are all critical hits until the floor is left (Focus
    /// Energy).
    pub focused: bool,
    /// Until the floor is left: it walks on water (Slip Seed), and the dark
    /// closes in around it, even in rooms (Dropeye Seed).
    pub slipping: bool,
    pub dropeye: bool,
    /// Turns counted by a Warp Scarf and a No-Slip Cap it holds.
    pub(crate) warp_scarf_count: i32,
    pub(crate) no_slip_count: i32,
    /// Multipliers of attack and defense, in 8-bit fixed point.
    pub attack_multiplier: i32,
    pub defense_multiplier: i32,
    pub sp_attack_stage: i32,
    pub sp_defense_stage: i32,
    pub sp_attack_multiplier: i32,
    pub sp_defense_multiplier: i32,
    pub shield: Option<Shield>,
    pub(super) shield_turns: i32,
    /// Turns left of a curse, and until its next damage.
    pub cursed_turns: i32,
    pub(super) curse_timer: i32,
    /// Turns left of Ingrain, and until its next heal.
    pub ingrain_turns: i32,
    pub(super) ingrain_timer: i32,
    /// Turns until Aqua Ring's next heal.
    pub(super) heal_timer: i32,
    /// Natural regeneration: grows by the max HP every turn.
    pub(super) regen_counter: i32,
    /// Movement type of the monster's species, as stored in the ROM.
    pub movement: u8,
    /// The two-turn move being charged, to be unleashed next turn.
    pub charging: Option<u16>,
    /// Turns left of each speed boost and each speed drop.
    pub(super) speed_up: [i32; SPEED_COUNTERS],
    pub(super) speed_down: [i32; SPEED_COUNTERS],
    /// Stat stages, from 0 to 20; they last until the floor is left.
    pub attack_stage: i32,
    pub defense_stage: i32,
    pub accuracy_stage: i32,
    pub evasion_stage: i32,
    /// For the experience a defeated monster gives: whether a move (not
    /// just regular attacks) ever hit it, and whether its last blow came
    /// from the player's team.
    pub hit_by_move: bool,
    /// Struck by a chain of linked moves (more experience for it).
    pub hit_by_link: bool,
    pub struck_by_team: bool,
    /// A move just missed it (shown over it as "MISS", then cleared).
    pub missed: bool,
}

impl Default for Statuses {
    fn default() -> Self {
        Self {
            sleep_turns: 0,
            poisoned: false,
            poison_timer: 0,
            badly_poisoned: false,
            bad_poison_ticks: 0,
            burn_turns: 0,
            burn_timer: 0,
            paralysis_turns: 0,
            frozen_turns: 0,
            cringe_turns: 0,
            confused_turns: 0,
            terrified_turns: 0,
            invisible_turns: 0,
            bouncing_turns: 0,
            snatching_turns: 0,
            decoy: false,
            decoy_turns: 0,
            drawing_turns: 0,
            vital_throw_turns: 0,
            exclusive: Default::default(),
            conversion2_turns: 0,
            leech_seed_turns: 0,
            leech_seed_timer: 0,
            leech_sower: None,
            leech_drained: 0,
            yawning_turns: 0,
            taunted_turns: 0,
            sure_shot_turns: 0,
            infatuated_turns: 0,
            raging: false,
            stockpile: 0,
            laying: None,
            sleepless_turns: 0,
            charged: false,
            levitating_turns: 0,
            embargo_turns: 0,
            held_turns: 0,
            constricted_turns: 0,
            constrict_timer: 0,
            speed_boost_timer: 0,
            weight: 0x100,
            last_move: None,
            encore_turns: 0,
            cross_eyed_turns: 0,
            nightmare: false,
            spited: false,
            grudge: false,
            heal_block_turns: 0,
            tormented: None,
            counter: None,
            counter_turns: 0,
            perish_turns: 0,
            set_damage_turns: 0,
            paused_turns: 0,
            identity: None,
            sketched: None,
            napping: false,
            hit_by_link: false,
            bide: None,
            bide_release: None,
            bond: None,
            bond_turns: 0,
            disguise: None,
            disguise_turns: 0,
            mobile: false,
            long_toss: false,
            piercing: false,
            focused: false,
            slipping: false,
            dropeye: false,
            warp_scarf_count: 0,
            no_slip_count: 0,
            attack_multiplier: 0x100,
            defense_multiplier: 0x100,
            sp_attack_stage: NEUTRAL_STAGE,
            sp_defense_stage: NEUTRAL_STAGE,
            sp_attack_multiplier: 0x100,
            sp_defense_multiplier: 0x100,
            shield: None,
            shield_turns: 0,
            cursed_turns: 0,
            curse_timer: 0,
            ingrain_turns: 0,
            ingrain_timer: 0,
            heal_timer: 0,
            regen_counter: 0,
            movement: 0,
            charging: None,
            speed_up: [0; SPEED_COUNTERS],
            speed_down: [0; SPEED_COUNTERS],
            attack_stage: NEUTRAL_STAGE,
            defense_stage: NEUTRAL_STAGE,
            accuracy_stage: NEUTRAL_STAGE,
            evasion_stage: NEUTRAL_STAGE,
            hit_by_move: false,
            struck_by_team: false,
            missed: false,
        }
    }
}

impl Statuses {
    /// Fresh statuses for a monster of a species with this movement type.
    pub fn for_species(movement: u8) -> Self {
        Self { movement, ..Self::default() }
    }

    /// The monster as the damage calculation sees it: its stats with what
    /// its exclusive items add, its stages, multipliers and screens.
    pub fn fighter(&self, stats: &Stats, team_member: bool) -> Fighter {
        use crate::dungeon::exclusive::effect;
        let [attack, defense, sp_attack, sp_defense] = self.exclusive.boosts;
        let stats = Stats {
            attack: stats.attack + attack,
            defense: stats.defense + defense,
            sp_attack: stats.sp_attack + sp_attack,
            sp_defense: stats.sp_defense + sp_defense,
            ..*stats
        };
        let lucky = self.shield == Some(Shield::LuckyChant) || self.exclusive.has(effect::NO_CRITICAL_HITS);
        Fighter {
            stats,
            attack_stage: self.attack_stage,
            defense_stage: self.defense_stage,
            attack_multiplier: self.attack_multiplier,
            defense_multiplier: self.defense_multiplier,
            sp_attack_stage: self.sp_attack_stage,
            sp_defense_stage: self.sp_defense_stage,
            sp_attack_multiplier: self.sp_attack_multiplier,
            sp_defense_multiplier: self.sp_defense_multiplier,
            halves_physical: self.shield == Some(Shield::Reflect),
            halves_special: self.shield == Some(Shield::LightScreen),
            team_member,
            burned: self.burn_turns > 0,
            ailing: self.ailing(),
            focused: self.focused,
            lucky,
        }
    }

    /// It suffers from a status problem: what Facade, Quick Feet and the
    /// like look at. Sleep, the lasting problems, freezing, confusion and
    /// a curse count; a stat drop does not.
    pub(crate) fn ailing(&self) -> bool {
        let lasting = self.poisoned || self.burn_turns > 0 || self.paralysis_turns > 0;
        lasting || self.asleep() || self.frozen_turns > 0 || self.confused_turns > 0 || self.cursed_turns > 0
    }

    /// Every stat stage and multiplier back to neutral.
    pub(crate) fn reset_stages(&mut self) {
        for stage in [
            &mut self.attack_stage,
            &mut self.defense_stage,
            &mut self.accuracy_stage,
            &mut self.evasion_stage,
            &mut self.sp_attack_stage,
            &mut self.sp_defense_stage,
        ] {
            *stage = NEUTRAL_STAGE;
        }
        for multiplier in [&mut self.attack_multiplier, &mut self.defense_multiplier, &mut self.sp_attack_multiplier, &mut self.sp_defense_multiplier] {
            *multiplier = 0x100;
        }
    }

    /// The floor is left: the stat stages back to neutral, and what lasts
    /// a floor gone.
    pub(crate) fn leave_floor(&mut self) {
        self.reset_stages();
        (self.slipping, self.dropeye) = (false, false);
    }

    pub fn asleep(&self) -> bool {
        self.sleep_turns > 0
    }

    /// A count of turns drawn in a range, both ends included, at least 1.
    pub(super) fn roll(rng: &mut DungeonRng, (low, high): (i32, i32)) -> i32 {
        rng.rand_range(low, high.max(low) + 1).max(1)
    }

    /// How long a status lasts: a count drawn in its range, and one more
    /// for the turn it starts on (that turn's end counts it down at once).
    pub(super) fn duration(rng: &mut DungeonRng, range: (i32, i32)) -> i32 {
        Self::roll(rng, range) + 1
    }

    /// Asleep for a count drawn in the sleep range.
    pub(crate) fn fall_asleep(&mut self, rng: &mut DungeonRng, constants: &Constants) {
        self.sleep_turns = Self::roll(rng, constants.sleep_turns);
    }

    /// Cures the lasting problem it has, to make room for another.
    pub(super) fn end_lasting(&mut self) {
        (self.poisoned, self.badly_poisoned) = (false, false);
        (self.burn_turns, self.paralysis_turns) = (0, 0);
    }

    /// Poisoned until cured; a monster already poisoned stays as it is.
    pub fn poison(&mut self) {
        if self.poisoned {
            return;
        }
        self.end_lasting();
        self.poisoned = true;
        (self.poison_timer, self.bad_poison_ticks) = (0, 0);
    }

    /// After Hyper Beam and the moves like it the user must catch its
    /// breath: its next turn is lost.
    pub fn recharge(&mut self, rng: &mut DungeonRng, constants: &Constants) {
        let rest = Self::duration(rng, constants.recharge_turns);
        self.paused_turns = self.paused_turns.max(rest);
    }

    /// Burned: hurt now and then, its physical blows weaker.
    pub fn burn(&mut self, rng: &mut DungeonRng, constants: &Constants) {
        let turns = Self::duration(rng, constants.burn_turns);
        if self.burn_turns == 0 {
            self.end_lasting();
            self.burn_timer = 0;
        }
        self.burn_turns = self.burn_turns.max(turns);
    }

    /// Paralysed: slower, and unable to attack while it lasts.
    pub(crate) fn paralyze(&mut self, rng: &mut DungeonRng, constants: &Constants) {
        let turns = Self::duration(rng, constants.paralysis_turns);
        if self.paralysis_turns == 0 {
            self.end_lasting();
        }
        self.paralysis_turns = self.paralysis_turns.max(turns);
    }

    /// Frozen solid: it does nothing until it thaws.
    pub(crate) fn freeze(&mut self, rng: &mut DungeonRng, constants: &Constants) {
        let turns = Self::duration(rng, constants.frozen_turns);
        self.frozen_turns = self.frozen_turns.max(turns);
    }

    /// Cringing: it walks but does not attack. A cringe chases confusion
    /// away, and the other way round.
    pub fn cringe(&mut self, rng: &mut DungeonRng, constants: &Constants) {
        let turns = Self::duration(rng, constants.cringe_turns);
        self.confused_turns = 0;
        self.cringe_turns = self.cringe_turns.max(turns);
    }

    /// Confused: it stumbles about in random directions.
    pub(crate) fn confuse(&mut self, rng: &mut DungeonRng, constants: &Constants) {
        let turns = Self::duration(rng, constants.confused_turns);
        self.cringe_turns = 0;
        self.confused_turns = self.confused_turns.max(turns);
    }
}
