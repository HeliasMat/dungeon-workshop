//! What a monster's conditions do turn by turn.
//!
//! At the end of each of its turns a monster's conditions count down; the
//! ones that hurt or heal do so every few turns, each on a timer of its
//! own that counts up to its period.

use super::*;

/// Counts a periodic timer up by one turn; true on the turn it fires (and
/// it starts again from nothing).
fn fires(timer: &mut i32, period: i32) -> bool {
    *timer += 1;
    if *timer >= period.max(1) {
        *timer = 0;
        return true;
    }
    false
}

/// One turn off a countdown, never below nothing.
fn count_down(turns: &mut i32) {
    *turns = (*turns - 1).max(0);
}

/// HP back, up to the most it can have.
fn heal(stats: &mut Stats, amount: i32) {
    stats.hp = (stats.hp + amount).min(stats.max_hp);
}

impl Statuses {
    pub(super) fn shield(&mut self, shield: Shield, rng: &mut DungeonRng, range: (i32, i32)) {
        (self.shield, self.shield_turns) = (Some(shield), Self::duration(rng, range));
    }

    /// Natural HP regeneration, for team members only. The species'
    /// regeneration speed says in how many turns a monster gets back its
    /// whole max HP (Wish makes that shorter); the HP come back a little
    /// every turn, the fractions carried over. Bad poison and Heal Block
    /// stop it; the Heal Ribbon and Quick Healer make it faster.
    pub(crate) fn regenerate(&mut self, stats: &mut Stats, constants: &Constants) {
        if constants.regen_speed <= 0 || !stats.is_alive() || (self.poisoned && self.badly_poisoned) || self.heal_block_turns > 0 {
            return;
        }
        let wish = if self.shield == Some(Shield::Wish) { constants.wish_bonus_regen } else { 0 };
        let period = (constants.regen_speed + wish).clamp(20, 600);
        let quick = if crate::dungeon::iq::has(stats, crate::dungeon::iq::QUICK_HEALER) { 2 } else { 1 };
        let pace = crate::dungeon::held::regen_times(stats) * quick;
        self.regen_counter += stats.max_hp.min(999) * pace;
        heal(stats, self.regen_counter / period);
        self.regen_counter %= period;
    }

    /// Keeps its own abilities and types before a move changes them (they
    /// come back when the floor is left).
    pub(crate) fn remember_identity(&mut self, stats: &Stats) {
        if self.identity.is_none() {
            self.identity = Some((stats.abilities, stats.types));
        }
    }

    /// A rooted monster cannot leave its tile.
    pub(crate) fn rooted(&self) -> bool {
        self.ingrain_turns > 0 || self.held_turns > 0 || self.constricted_turns > 0
    }

    /// After a hit: an enduring monster keeps 1 HP.
    pub fn endure(&self, stats: &mut Stats) {
        if self.shield == Some(Shield::Endure) {
            stats.hp = stats.hp.max(1);
        }
    }

    /// Moves used on a protected monster fail.
    pub(crate) fn protected(&self) -> bool {
        self.shield == Some(Shield::Protect)
    }

    /// While charging one of these moves the monster is out of reach: Fly,
    /// Dig, Dive, Bounce, Shadow Force.
    pub fn out_of_reach(&self) -> bool {
        matches!(self.charging, Some(153 | 8 | 156 | 205 | 477))
    }

    /// The few moves that still find a monster where its two-turn move
    /// hides it. In the air (Fly, Bounce): Thunder and Gust reach it,
    /// Twister and Sky Uppercut twice as hard; under water (Dive):
    /// Whirlpool and Surf, twice as hard; underground (Dig): Earthquake and
    /// Magnitude, twice as hard. `None`: the move misses; `Some(true)`: its
    /// damage is doubled.
    pub(crate) fn reached_hidden(&self, move_id: u16) -> Option<bool> {
        const FLY: u16 = 153;
        const BOUNCE: u16 = 205;
        const DIVE: u16 = 156;
        const DIG: u16 = 8;
        const IN_THE_AIR: [(u16, bool); 4] = [(136, false), (64, false), (57, true), (162, true)];
        const UNDER_WATER: [(u16, bool); 2] = [(32, true), (219, true)];
        const UNDERGROUND: [(u16, bool); 2] = [(118, true), (296, true)];
        let reaching: &[(u16, bool)] = match self.charging? {
            FLY | BOUNCE => &IN_THE_AIR,
            DIVE => &UNDER_WATER,
            DIG => &UNDERGROUND,
            _ => &[],
        };
        reaching.iter().find(|(id, _)| *id == move_id).map(|&(_, doubled)| doubled)
    }

    /// Asleep, frozen or catching its breath, it loses its turn.
    pub(crate) fn cannot_act(&self) -> bool {
        self.asleep() || self.frozen_turns > 0 || self.paused_turns > 0
    }

    /// Asleep (and nothing else keeping it from acting), the move it uses
    /// all the same: Snore or Sleep Talk, while they have PP.
    pub(crate) fn sleep_move(&self, moves: &[MoveSlot]) -> Option<usize> {
        const SNORE: u16 = 25;
        const SLEEP_TALK: u16 = 227;
        let only_asleep = self.asleep() && self.frozen_turns == 0 && self.paused_turns == 0;
        if !only_asleep {
            return None;
        }
        moves.iter().position(|slot| matches!(slot.id, SNORE | SLEEP_TALK) && slot.pp > 0 && !slot.sealed)
    }

    /// Cringing, paralysed or smitten monsters can still walk, not attack.
    pub(crate) fn cannot_attack(&self) -> bool {
        self.cannot_act() || self.cringe_turns > 0 || self.paralysis_turns > 0 || self.infatuated_turns > 0
    }

    /// Confused monsters move and attack in random directions.
    pub fn confused(&self) -> bool {
        self.confused_turns > 0 || self.cross_eyed_turns > 0
    }

    /// Removes every status problem; stat stages stay.
    pub(crate) fn cure_all(&mut self) {
        self.end_lasting();
        (self.sleep_turns, self.nightmare) = (0, false);
        (self.frozen_turns, self.cringe_turns, self.confused_turns) = (0, 0, 0);
        self.speed_down = [0; SPEED_COUNTERS];
        self.cursed_turns = 0;
    }

    /// A hit ends a sleep that would never wear off by itself; any other
    /// sleep goes on.
    pub fn struck(&mut self) {
        if self.sleep_turns == DEEP_SLEEP {
            self.sleep_turns = 0;
        }
    }

    /// Passive effects at the end of the monster's turn. Magic Guard
    /// keeps every loss of HP of them away, but for Perish Song's end.
    pub(crate) fn end_turn(&mut self, stats: &mut Stats, constants: &Constants) {
        let guarded = crate::dungeon::ability::has(stats, crate::dungeon::ability::MAGIC_GUARD);
        let (before, perishing) = (stats.hp, self.perish_turns == 1);
        self.end_turn_unguarded(stats, constants);
        if guarded && !perishing {
            stats.hp = stats.hp.max(before);
        }
    }

    pub(super) fn end_turn_unguarded(&mut self, stats: &mut Stats, constants: &Constants) {
        if crate::dungeon::ability::has(stats, crate::dungeon::ability::SPEED_BOOST) && fires(&mut self.speed_boost_timer, constants.speed_boost_every) {
            self.natural_speed_boost(constants);
        }
        self.tick_perish(stats);
        self.tick_sleep(stats, constants);
        self.tick_countdowns();
        self.tick_heals(stats, constants);
        self.tick_harm(stats, constants);
    }

    /// Perish Song's count: when it runs out, the monster faints.
    fn tick_perish(&mut self, stats: &mut Stats) {
        if self.perish_turns > 0 {
            self.perish_turns -= 1;
            if self.perish_turns == 0 {
                stats.hp = 0;
            }
        }
    }

    /// Sleep wears off (but a deep one); waking from a nightmare hurts,
    /// waking from Rest's nap heals and cures. A yawning monster drops off
    /// when its yawn is over, for the shortest sleep.
    fn tick_sleep(&mut self, stats: &mut Stats, constants: &Constants) {
        if self.sleep_turns > 0 && self.sleep_turns != DEEP_SLEEP {
            self.sleep_turns -= 1;
            if self.sleep_turns == 0 {
                self.wake(stats, constants);
            }
        }
        if self.yawning_turns > 0 {
            self.yawning_turns -= 1;
            if self.yawning_turns == 0 && !self.asleep() && self.sleepless_turns == 0 {
                self.sleep_turns = constants.sleep_turns.0.max(1);
            }
        }
    }

    /// What waking up brings.
    fn wake(&mut self, stats: &mut Stats, constants: &Constants) {
        if std::mem::take(&mut self.nightmare) {
            stats.hp -= constants.nightmare_damage;
        }
        if std::mem::take(&mut self.napping) {
            heal(stats, constants.napping_heal);
            self.cure_all();
        }
    }

    /// The plain countdowns: they only run out.
    fn tick_countdowns(&mut self) {
        for turns in [
            &mut self.paralysis_turns,
            &mut self.frozen_turns,
            &mut self.cringe_turns,
            &mut self.confused_turns,
            &mut self.terrified_turns,
            &mut self.invisible_turns,
            &mut self.taunted_turns,
            &mut self.sure_shot_turns,
            &mut self.infatuated_turns,
            &mut self.sleepless_turns,
            &mut self.levitating_turns,
            &mut self.embargo_turns,
            &mut self.held_turns,
            &mut self.encore_turns,
            &mut self.cross_eyed_turns,
            &mut self.heal_block_turns,
            &mut self.counter_turns,
            &mut self.bond_turns,
            &mut self.disguise_turns,
            &mut self.set_damage_turns,
            &mut self.paused_turns,
            &mut self.bouncing_turns,
            &mut self.snatching_turns,
            &mut self.drawing_turns,
            &mut self.vital_throw_turns,
            &mut self.conversion2_turns,
        ] {
            count_down(turns);
        }
        self.speed_up.iter_mut().chain(&mut self.speed_down).for_each(count_down);
        if self.decoy_turns > 0 {
            self.decoy_turns -= 1;
            self.decoy &= self.decoy_turns > 0;
        }
        if self.shield.is_some() {
            self.shield_turns -= 1;
            if self.shield_turns <= 0 {
                self.shield = None;
            }
        }
    }

    /// Ingrain and Aqua Ring heal every few turns while they last.
    fn tick_heals(&mut self, stats: &mut Stats, constants: &Constants) {
        if self.ingrain_turns > 0 {
            self.ingrain_turns -= 1;
            if fires(&mut self.ingrain_timer, constants.ingrain_cooldown) {
                heal(stats, constants.ingrain_regen);
            }
        }
        if self.shield == Some(Shield::AquaRing) && fires(&mut self.heal_timer, constants.aqua_ring_cooldown) {
            heal(stats, constants.aqua_ring_regen);
        }
    }

    /// What hurts every few turns: a curse, Leech Seed, a burn, poison
    /// and a constriction.
    fn tick_harm(&mut self, stats: &mut Stats, constants: &Constants) {
        if self.cursed_turns > 0 {
            self.cursed_turns -= 1;
            if fires(&mut self.curse_timer, constants.curse_cooldown) {
                stats.hp -= (stats.max_hp.min(999) / 4).max(1);
            }
        }
        if self.leech_seed_turns > 0 {
            self.leech_seed_turns -= 1;
            if fires(&mut self.leech_seed_timer, constants.leech_seed_cooldown) {
                // (What it had left goes to the sower's side.)
                self.leech_drained += constants.leech_seed_drain.min(stats.hp.max(0));
                stats.hp -= constants.leech_seed_drain;
            }
        }
        if self.burn_turns > 0 {
            self.burn_turns -= 1;
            if fires(&mut self.burn_timer, constants.burn_cooldown) {
                stats.hp -= constants.burn_damage;
            }
        }
        self.tick_poison(stats, constants);
        if self.constricted_turns > 0 {
            self.constricted_turns -= 1;
            if fires(&mut self.constrict_timer, constants.constriction_cooldown) {
                stats.hp -= constants.constriction_damage;
            }
        }
    }

    /// Poison hurts every few turns; bad poison more often, and more each
    /// time. Poison Heal turns the loss into a gain.
    fn tick_poison(&mut self, stats: &mut Stats, constants: &Constants) {
        if !self.poisoned {
            return;
        }
        let amount = match self.badly_poisoned {
            true if fires(&mut self.poison_timer, constants.bad_poison_cooldown) => {
                let table = &constants.bad_poison_damage;
                let hurt = table[(self.bad_poison_ticks.max(0) as usize).min(table.len() - 1)];
                self.bad_poison_ticks += 1;
                hurt
            }
            false if fires(&mut self.poison_timer, constants.poison_cooldown) => constants.poison_damage,
            _ => return,
        };
        match crate::dungeon::ability::has(stats, crate::dungeon::ability::POISON_HEAL) {
            true => heal(stats, amount),
            false => stats.hp -= amount,
        }
    }
}
