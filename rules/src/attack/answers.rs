//! What comes of a blow: by the move it was, then what it brings on its
//! striker and its target.

use crate::dungeon::combat::Damage;
use crate::dungeon::floor::chebyshev;
use crate::dungeon::status::{self, Aftermath};
use crate::dungeon::{Belly, ItemUse, MoveEffect, REGULAR_ATTACK_ID, ability, aftermath, eat_item, exclusive, iq, secondary_effects, stack, user_effect};

use super::{Attack, Happened, Party, Play, Role};

const FLYING: u8 = 10;
const STAGE_TOP: i32 = status::STAGE_COUNT as i32 - 1;

impl Play<'_, '_> {
    /// What comes of a blow by the move it was, before it is told.
    pub(super) fn struck(&mut self, user: &mut Party, victim: &mut Party, attack: &Attack, damage: Damage, type_id: u8) {
        let constants = self.field.constants;
        let slot = attack.slot;
        // Anger Point: a critical hit sends its attack to the top.
        if damage.critical && victim.alive() && ability::has(victim.stats, ability::ANGER_POINT) {
            victim.status.attack_stage = STAGE_TOP;
            self.says(Happened::AttackTopped);
        }
        if victim.status.raging && damage.amount > 0 && victim.alive() {
            victim.status.attack_stage = (victim.status.attack_stage + 1).min(STAGE_TOP);
            self.says(Happened::RageGrows);
        }
        victim.status.hit_by_move |= slot.id != REGULAR_ATTACK_ID;
        victim.status.hit_by_link |= attack.linked;
        // Vital Throw: struck up close by a physical move, it hurls the
        // striker at the striker's own side.
        if victim.status.vital_throw_turns > 0
            && slot.category == 0
            && damage.amount > 0
            && victim.alive()
            && user.alive()
            && chebyshev(user.at, victim.at) == 1
        {
            let facing = ((user.at.x - victim.at.x).signum(), (user.at.y - victim.at.y).signum());
            self.says(Happened::Hurled { facing });
        }
        // Pay Day: coins scatter where the blow lands.
        const PAY_DAY: u16 = 228;
        if slot.id == PAY_DAY && damage.amount > 0 {
            self.says(Happened::Drops(status::item::POKE));
        }
        // Pluck, Bug Bite: the user snaps up and eats what its target holds,
        // when that is food (a sticky item stays put).
        if matches!(slot.id, 490 | 529) && victim.stats.held != 0 && !stack::sticky(victim.stats.held) && user.alive() {
            let item = stack::id(victim.stats.held);
            let eaten = eat_item(item, self.field.rng, constants, user.stats, user.status, &mut Belly::full(), &mut []);
            if eaten != ItemUse::Nothing {
                victim.stats.held = 0;
                self.says(Happened::Ate(item));
            }
        }
        // Hyper Beam and its like: the user rests after.
        if status::HYPER_BEAMS.contains(&slot.id) && user.alive() && user.status.paused_turns == 0 {
            user.status.recharge(self.field.rng, constants);
            self.says(Happened::Rests);
        }
        // Spit Up lets out all that was stockpiled.
        if slot.id == 245 {
            user.status.stockpile = 0;
        }
        // Wake-Up Slap wakes its target.
        const WAKE_UP_SLAP: u16 = 531;
        if slot.id == WAKE_UP_SLAP && victim.status.asleep() && victim.alive() {
            (victim.status.sleep_turns, victim.status.napping, victim.status.nightmare) = (0, false, false);
            self.says(Happened::Woken);
        }
        // SmellingSalt cures the paralysis it struck twice as hard.
        const SMELLINGSALT: u16 = 78;
        if slot.id == SMELLINGSALT && victim.status.paralysis_turns > 0 && victim.alive() {
            victim.status.paralysis_turns = 0;
            self.says(Happened::Unparalyzed);
        }
        // Rapid Spin frees its user from what binds it or saps it:
        // constriction, Shadow Hold, Leech Seed.
        const RAPID_SPIN: u16 = 96;
        let bound = user.status.constricted_turns > 0 || user.status.held_turns > 0 || user.status.leech_seed_turns > 0;
        if slot.id == RAPID_SPIN && bound && user.alive() {
            (user.status.constricted_turns, user.status.held_turns, user.status.leech_seed_turns) = (0, 0, 0);
            user.status.leech_sower = None;
            self.says(Happened::SpunFree);
        }
        // Conversion 2: hit by a move, the monster turns to a type that
        // stands up to it (the type data says which; none, it stays).
        let resisting = constants.conversion2_types.get(type_id as usize).copied().unwrap_or(0);
        if victim.status.conversion2_turns > 0 && resisting != 0 && victim.alive() && victim.stats.types != (resisting, 0) {
            victim.status.remember_identity(victim.stats);
            victim.stats.types = (resisting, 0);
            self.says(Happened::TypeChanged);
        }
    }

    /// What a blow told brings on its striker and its target.
    pub(super) fn answers(&mut self, user: &mut Party, victim: &mut Party, attack: &Attack, damage: Damage) {
        let constants = self.field.constants;
        let slot = attack.slot;
        let felt = damage.amount > 0;
        // Thief and Covet take what the target holds, if the user holds
        // nothing.
        const THIEF: u16 = 214;
        const COVET: u16 = 287;
        if matches!(slot.id, THIEF | COVET) && felt && victim.stats.held != 0 && user.stats.held == 0 && !ability::has(victim.stats, ability::STICKY_HOLD) {
            user.stats.held = std::mem::take(&mut victim.stats.held);
            self.says(Happened::Stole(user.stats.held));
        }
        // Intimidator: the striker may cringe.
        if felt && user.alive() && victim.alive() && iq::has(victim.stats, iq::INTIMIDATOR) && self.rolled(constants.intimidator_chance) {
            self.affects(MoveEffect::Cringe, user, Role::User);
        }
        // Arena Trap (who strikes from the ground) and Shadow Tag hold the
        // striker where it stands.
        if felt && user.alive() && victim.alive() {
            let grounded = user.stats.types.0 != FLYING
                && user.stats.types.1 != FLYING
                && user.status.levitating_turns == 0
                && !ability::has(user.stats, ability::LEVITATE);
            let holds = (ability::shields(user.stats, victim.stats, ability::ARENA_TRAP) && grounded && self.rolled(constants.arena_trap_chance))
                || (ability::shields(user.stats, victim.stats, ability::SHADOW_TAG) && self.rolled(constants.shadow_tag_chance));
            if holds {
                self.affects(MoveEffect::Immobilize, user, Role::User);
            }
        }
        // Exclusive items answer blows: on the striker, or on the one
        // struck.
        if felt && user.alive() && victim.alive() {
            self.items_answer(user, victim, slot.category, damage);
        }
        // Touching some monsters costs: Static, Poison Point, Effect Spore
        // and Flame Body strike back, Rough Skin hurts.
        if slot.category == 0 && felt && user.alive() {
            if let Some(effect) = ability::on_contact(self.field.rng, user.stats, victim.stats) {
                self.affects(effect, user, Role::User);
            }
            let hurt = ability::rough_skin(user.stats, victim.stats, damage.amount);
            if hurt > 0 {
                user.stats.hp -= hurt;
                self.says(Happened::Contact(hurt));
            }
        }
        let id = felt.then_some(slot.id);
        // (Secret Power's, by the floor.)
        const SECRET_POWER: u16 = 263;
        let secret = (slot.id == SECRET_POWER).then(|| (constants.secret_power_chance, status::secret_power_effect(self.field.secret_power)));
        for (chance, effect) in id.map(|id| secondary_effects(id, constants)).unwrap_or_default().into_iter().chain(secret.filter(|_| id.is_some())) {
            if !victim.alive() || !self.rolled(ability::secondary_chance(user.stats, victim.stats, chance)) {
                continue;
            }
            self.affects(effect, victim, Role::Victim);
        }
        if let Some(aftermath) = id.and_then(aftermath) {
            // (Liquid Ooze: what is drained hurts; Magic Guard: no recoil.)
            if aftermath == Aftermath::Drain && ability::shields(user.stats, victim.stats, ability::LIQUID_OOZE) {
                let lost = (damage.amount / 2).max(1);
                user.stats.hp -= lost;
                self.says(Happened::Ooze(lost));
            } else if !(aftermath == Aftermath::Recoil && ability::has(user.stats, ability::MAGIC_GUARD)) {
                let lost = aftermath.apply(user.stats, damage.amount);
                self.says(Happened::Aftermath(lost));
            }
        }
        if let Some((chance, effect)) = id.and_then(|id| user_effect(id, constants))
            && self.rolled(chance)
        {
            self.affects(effect, user, Role::User);
        }
        // A Counter: some of the damage goes back to who dealt it.
        if let Some(kind) = victim.status.counter.filter(|_| victim.status.counter_turns > 0 && user.alive()) {
            let back = kind.returned(slot.category, damage.amount);
            if back > 0 {
                user.stats.hp -= back;
                self.says(Happened::Countered(back));
            }
        }
        // A Grudge: who knocked it out loses that move's PP.
        if !victim.alive() && std::mem::take(&mut victim.status.grudge) {
            user.status.spited = true;
            self.says(Happened::Grudge);
        }
    }

    fn items_answer(&mut self, user: &mut Party, victim: &mut Party, category: u8, damage: Damage) {
        let chance = self.field.constants.exclusive_reaction_chance;
        let held = victim.status.exclusive;
        // Struck by a physical move: all of it back, or a quarter.
        if category == 0 {
            let back = match (held.has(exclusive::effect::COUNTER_PHYSICAL_DAMAGE), held.has(exclusive::effect::COUNTER_25_PCT_PHYSICAL_DAMAGE)) {
                (true, _) => damage.amount,
                (false, true) => (damage.amount / 4).max(1),
                _ => 0,
            };
            if back > 0 {
                user.stats.hp -= back;
                self.says(Happened::Returned(back));
            }
        }
        if held.has(exclusive::effect::MAY_POISON_PARALYZE_SLEEP_ATTACKERS) && self.rolled(chance) {
            let effect = [MoveEffect::Poison, MoveEffect::Paralyze, MoveEffect::Sleep][self.field.rng.rand_int(3) as usize];
            self.affects(effect, user, Role::User);
        }
        for (which, effect, on_itself) in exclusive::reactions() {
            if !held.has(which) || !self.rolled(chance) {
                continue;
            }
            match on_itself {
                true => self.affects(effect, victim, Role::Victim),
                false => self.affects(effect, user, Role::User),
            };
        }
    }
}
