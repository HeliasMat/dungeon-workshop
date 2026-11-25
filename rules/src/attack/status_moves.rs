//! A status move on its target.

use crate::dungeon::status::{StatSwap, Statuses};
use crate::dungeon::{FIXED_ONE, MoveEffect, ability, held, move_hits};

use super::{Attack, Happened, Party, Play, Role};

impl Play<'_, '_> {
    /// Whether it did not miss.
    pub(super) fn status_move(&mut self, user: &mut Party, victim: &mut Party, attack: &Attack, effect: MoveEffect) -> bool {
        let (constants, weather) = (self.field.constants, self.field.weather);
        let (aimed, accuracy, evasion) = self.aim(attack.slot, user, victim);
        let females = (user.stats.female, victim.stats.female);
        let hit = !victim.status.protected() && move_hits(self.field.rng, constants, &aimed, (accuracy, evasion), weather, females);
        if !hit {
            self.says(Happened::Missed);
            return false;
        }
        match effect {
            // Destiny Bond: the two bound by one token.
            MoveEffect::DestinyBond => {
                *self.field.bonds += 1;
                let turns = constants.destiny_bond_turns;
                let turns = self.field.rng.rand_range(turns.0, turns.1.max(turns.0 + 1));
                user.status.bond = Some((*self.field.bonds, true));
                victim.status.bond = Some((*self.field.bonds, false));
                (user.status.bond_turns, victim.status.bond_turns) = (turns, turns);
                self.says(Happened::Bound);
            }
            // Swaps of stat changes, of held items.
            MoveEffect::Swap(which) => {
                swap_stats(user.status, victim.status, which);
                self.says(Happened::StatsSwapped);
            }
            MoveEffect::Trick => {
                if user.stats.held == 0 || victim.stats.held == 0 || ability::has(victim.stats, ability::STICKY_HOLD) {
                    self.says(Happened::Failed);
                } else {
                    std::mem::swap(&mut user.stats.held, &mut victim.stats.held);
                    self.says(Happened::ItemsSwapped);
                }
            }
            // Mimic: the target's last move, on it in turn; Sketch: learnt.
            MoveEffect::Mimic | MoveEffect::Sketch => return self.mimic(user, victim, attack, effect),
            MoveEffect::RolePlay | MoveEffect::SkillSwap | MoveEffect::GastroAcid => self.abilities_moved(user, victim, effect),
            // Psycho Shift: the user's status problems go to the target.
            MoveEffect::PsychoShift => {
                if !user.status.ailing() {
                    self.says(Happened::Failed);
                    return true;
                }
                let (from, to) = (&mut *user.status, &mut *victim.status);
                (to.sleep_turns, to.poisoned, to.badly_poisoned) = (from.sleep_turns, from.poisoned, from.badly_poisoned);
                (to.burn_turns, to.paralysis_turns, to.frozen_turns) = (from.burn_turns, from.paralysis_turns, from.frozen_turns);
                from.cure_all();
                self.says(Happened::AilmentsPassed);
            }
            // Pain Split: the HP of the two, shared evenly.
            MoveEffect::PainSplit => {
                let even = (user.stats.hp.max(0) + victim.stats.hp.max(0)) / 2;
                user.stats.hp = even.min(user.stats.max_hp);
                victim.stats.hp = even.min(victim.stats.max_hp);
                self.says(Happened::LifeShared);
            }
            // Psych Up: the user takes on the target's stat changes.
            MoveEffect::PsychUp => {
                let from = &*victim.status;
                (user.status.attack_stage, user.status.defense_stage) = (from.attack_stage, from.defense_stage);
                (user.status.sp_attack_stage, user.status.sp_defense_stage) = (from.sp_attack_stage, from.sp_defense_stage);
                (user.status.accuracy_stage, user.status.evasion_stage) = (from.accuracy_stage, from.evasion_stage);
                self.says(Happened::StatsCopied);
            }
            // A Pass Scarf shrugs the effect off onto someone next to it.
            _ if held::holds(victim.stats, held::PASS_SCARF) => self.says(Happened::Passed(effect)),
            _ if victim.status.bouncing_turns > 0 => {
                self.says(Happened::Bounced);
                self.affects(effect, user, Role::User);
            }
            _ => {
                let applied = self.apply(effect, victim);
                // Wrap holds its user as long as its target.
                if applied && effect == MoveEffect::Wrap {
                    user.status.paused_turns = victim.status.paused_turns;
                }
                if applied && effect == MoveEffect::LeechSeed {
                    victim.status.leech_sower = Some(user.id);
                }
                self.says(Happened::Effect { on: Role::Victim, effect, applied });
            }
        }
        true
    }

    /// Mimic: the target's last move, on it in turn; Sketch: learnt.
    fn mimic(&mut self, user: &mut Party, victim: &mut Party, attack: &Attack, effect: MoveEffect) -> bool {
        let (constants, weather) = (self.field.constants, self.field.weather);
        let copied = victim.status.last_move.and_then(|id| self.field.moves.get(id as usize).copied());
        let Some(copied) = copied.filter(|copied| !matches!(copied.id, 138 | 327)) else {
            self.says(Happened::Failed);
            return true;
        };
        if effect == MoveEffect::Sketch {
            user.status.sketched = Some(copied.id);
            return true;
        }
        self.says(Happened::Mimics(copied.id));
        let mimicked = Attack { slot: copied, effect: copied.effect_in(constants, weather, user.stats), damage_mult: FIXED_ONE, linked: attack.linked };
        match mimicked.effect {
            Some(effect) if effect.on_user() => {
                self.affects(effect, user, Role::User);
                self.says(Happened::Cast(effect));
                true
            }
            _ => self.land(user, victim, &mimicked),
        }
    }

    /// Abilities taken, swapped or lost, for the floor: Role Play, Skill
    /// Swap, Gastro Acid.
    fn abilities_moved(&mut self, user: &mut Party, victim: &mut Party, effect: MoveEffect) {
        user.status.remember_identity(user.stats);
        victim.status.remember_identity(victim.stats);
        match effect {
            MoveEffect::RolePlay => {
                user.stats.abilities = victim.stats.abilities;
                self.says(Happened::AbilityCopied);
            }
            MoveEffect::SkillSwap => {
                std::mem::swap(&mut user.stats.abilities, &mut victim.stats.abilities);
                self.says(Happened::AbilitiesSwapped);
            }
            _ => {
                victim.stats.abilities = [0, 0];
                self.says(Happened::AbilityLost);
            }
        }
    }
}

/// Swaps of stat changes: the defenses', the attacks', or all of them.
fn swap_stats(a: &mut Statuses, b: &mut Statuses, which: StatSwap) {
    if matches!(which, StatSwap::Defenses | StatSwap::All) {
        std::mem::swap(&mut a.defense_stage, &mut b.defense_stage);
        std::mem::swap(&mut a.sp_defense_stage, &mut b.sp_defense_stage);
        std::mem::swap(&mut a.defense_multiplier, &mut b.defense_multiplier);
        std::mem::swap(&mut a.sp_defense_multiplier, &mut b.sp_defense_multiplier);
    }
    if matches!(which, StatSwap::Attacks | StatSwap::All) {
        std::mem::swap(&mut a.attack_stage, &mut b.attack_stage);
        std::mem::swap(&mut a.sp_attack_stage, &mut b.sp_attack_stage);
        std::mem::swap(&mut a.attack_multiplier, &mut b.attack_multiplier);
        std::mem::swap(&mut a.sp_attack_multiplier, &mut b.sp_attack_multiplier);
    }
    if which == StatSwap::All {
        std::mem::swap(&mut a.accuracy_stage, &mut b.accuracy_stage);
        std::mem::swap(&mut a.evasion_stage, &mut b.evasion_stage);
    }
}
