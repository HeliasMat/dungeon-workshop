//! The strikes of a damaging move.

use crate::dungeon::combat::Damage;
use crate::dungeon::status;
use crate::dungeon::{
    FIXED_ONE, MoveEffect, MoveSlot, REGULAR_ATTACK_ID, ability, calc_damage_with, exclusive, fixed_damage, held, iq, matchup, move_hits, strike_count,
};

use super::{Attack, Happened, Party, Play, grounded};

const GROUND: u8 = 9;
const FIRE: u8 = 2;
const ELECTRIC: u8 = 5;

impl Play<'_, '_> {
    /// The move and the stages the hit is checked with, as abilities, IQ
    /// skills and held items make them (`MoveHitCheck`).
    pub(super) fn aim(&self, slot: MoveSlot, user: &Party, victim: &Party) -> (MoveSlot, i32, i32) {
        let constants = self.field.constants;
        let stages = (user.status.accuracy_stage, victim.status.evasion_stage);
        let (mut accuracy, mut evasion) =
            ability::hit_stages(user.stats, victim.stats, stages, slot.category == 0, self.field.weather, victim.status.confused());
        accuracy += iq::has(user.stats, iq::CONCENTRATOR) as i32;
        // (Y-Ray Specs on any but the leader: Blinker, an accuracy drop
        // here as the Blinker Seed's.)
        if held::holds(user.stats, held::Y_RAY_SPECS) && !user.leader {
            accuracy -= 2;
        }
        evasion -= iq::has(victim.stats, iq::CONCENTRATOR) as i32;
        if iq::has(victim.stats, iq::CLUTCH_PERFORMER) && ability::in_a_pinch(victim.stats) {
            evasion += 2;
        }
        // (An exclusive item: a stage of evasion in its weather.)
        evasion += victim.status.exclusive.in_weather(exclusive::InWeather::Evasion, self.field.weather) as i32;
        let mut aimed = slot;
        let drop = if held::holds(victim.stats, held::DETECT_BAND) { constants.detect_band_drop } else { 0 }
            + if iq::has(victim.stats, iq::QUICK_DODGER) { constants.quick_dodger_drop } else { 0 };
        for value in [&mut aimed.accuracy, &mut aimed.accuracy2] {
            if *value <= 100 {
                *value -= drop;
            }
        }
        // (Never missing: the Lock-On Specs, Sure-Hit Attacker for the
        // regular attack.)
        let sure = held::holds(user.stats, held::LOCKON_SPECS)
            || user.status.sure_shot_turns > 0
            || (iq::has(user.stats, iq::SURE_HIT_ATTACKER) && slot.id == REGULAR_ATTACK_ID);
        if sure {
            (aimed.accuracy, aimed.accuracy2) = (125, 125);
        }
        (aimed, accuracy, evasion)
    }

    /// Whether the last strike did not miss.
    pub(super) fn blows(&mut self, user: &mut Party, victim: &mut Party, attack: &Attack) -> bool {
        let (constants, chart, weather) = (self.field.constants, self.field.chart, self.field.weather);
        // (A Weather Band: as in clear weather for its holder.)
        let weather = if held::holds(user.stats, held::WEATHER_BAND) { crate::dungeon::weather::CLEAR } else { weather };
        let slot = attack.slot;
        // Dream Eater only works on a sleeper.
        const DREAM_EATER: u16 = 331;
        if slot.id == DREAM_EATER && !victim.status.asleep() {
            self.says(Happened::NotAsleep);
            return true;
        }
        // Skill Link: a move that strikes 2 to 5 times strikes 5 times.
        let strikes = match slot.strikes == 0 && ability::has(user.stats, ability::SKILL_LINK) {
            true => 5,
            false => strike_count(self.field.rng, slot.strikes),
        };
        // Brick Break shatters the target's Reflect or Light Screen first.
        const BRICK_BREAK: u16 = 72;
        if slot.id == BRICK_BREAK && matches!(victim.status.shield, Some(status::Shield::Reflect | status::Shield::LightScreen)) {
            victim.status.shield = None;
            self.says(Happened::Shattered);
        }
        // Under Gravity, Ground moves reach what flies or floats.
        let gravity = self.field.gravity && slot.type_id == GROUND;
        // Practice Swinger: having missed, this blow is a stage stronger.
        let swinging = std::mem::take(&mut user.status.practice_swing) && iq::has(user.stats, iq::PRACTICE_SWINGER);
        let bypass = user.status.exclusive.has(exclusive::effect::BYPASS_REFLECT_LIGHT_SCREEN);
        for strike in 0..strikes as usize {
            let floating = victim.status.levitating_turns > 0 && slot.type_id == GROUND && !gravity;
            let hidden = victim.status.out_of_reach().then(|| victim.status.reached_hidden(slot.id));
            const FEINT: u16 = 516;
            let shielded = hidden == Some(None) || (victim.status.protected() && slot.id != FEINT) || floating;
            if slot.id == FEINT && victim.status.protected() {
                victim.status.shield = None;
                self.says(Happened::Unshielded);
            }
            let (aimed, accuracy, evasion) = self.aim(slot, user, victim);
            let hit = !shielded && move_hits(self.field.rng, constants, &aimed, (accuracy, evasion), weather, (user.stats.female, victim.stats.female));
            let hit = hit && !self.forewarned(user, victim);
            if !hit {
                self.says(Happened::Missed);
                user.status.practice_swing = true;
                // Jump Kick, Hi Jump Kick: missing, the kicker comes down
                // hard, losing an eighth of its max HP (a quarter for the
                // higher kick).
                const JUMP_KICK: u16 = 204;
                const HI_JUMP_KICK: u16 = 206;
                if matches!(slot.id, JUMP_KICK | HI_JUMP_KICK) {
                    let share = if slot.id == HI_JUMP_KICK { 4 } else { 8 };
                    let crash = (user.stats.max_hp.min(999) / share).max(1);
                    user.stats.hp -= crash;
                    self.says(Happened::Crashed(crash));
                }
                return false;
            }
            let (type_id, damage_mult) = if slot.id == status::WEATHER_BALL { constants.weather_ball(weather) } else { (slot.type_id, attack.damage_mult) };
            // (Some moves hit harder or softer by their user's or target's
            // state.)
            let slowed = user.status.speed_stage_in(user.stats, weather) == 0;
            let by_state = status::power_multiplier(slot.id, slot.pp, slot.max_pp, constants, (user.stats, user.status, slowed), (victim.stats, victim.status));
            let damage_mult = damage_mult * by_state / FIXED_ONE * status::strike_multiplier(slot.id, strike, constants) / FIXED_ONE;
            let damage_mult = (damage_mult as f32 * self.tuning.worth) as i32;
            let fixed = fixed_damage(&slot, self.field.rng, constants, chart, user.stats, victim.stats);
            // Sturdy stands one-hit knock-out moves.
            const ONE_HIT_KO: i32 = 9999;
            if fixed == Some(ONE_HIT_KO) && ability::shields(user.stats, victim.stats, ability::STURDY) {
                self.says(Happened::Stood);
                return true;
            }
            let mut damage = match fixed {
                Some(0) => {
                    self.says(Happened::Failed);
                    return true;
                }
                Some(amount) => Damage { amount, matchup: matchup::NEUTRAL, critical: false },
                None => {
                    let mut target = victim.status.fighter(&if gravity { grounded(victim.stats) } else { *victim.stats }, victim.team);
                    let mut fighter = user.fighter();
                    // (An exclusive item's blows go through Reflect and
                    // Light Screen.)
                    if bypass {
                        (target.halves_physical, target.halves_special) = (false, false);
                    }
                    super::readied(&mut fighter, &mut target, slot.category != 0, weather, self.field.sides, swinging);
                    calc_damage_with(
                        self.field.rng,
                        constants,
                        chart,
                        &fighter,
                        &target,
                        type_id,
                        slot.power,
                        slot.crit_chance,
                        damage_mult,
                        slot.category != 0,
                        weather,
                    )
                }
            };
            // An ability may take the move in instead of its damage.
            let absorbed =
                (fixed.is_none() && slot.power > 0).then(|| ability::absorbs(user.stats, victim.stats, ability::attack_type(user.stats, type_id))).flatten();
            if let Some(absorbed) = absorbed {
                match absorbed {
                    ability::Absorbed::Heals => {
                        let healed = damage.amount.min(victim.stats.max_hp - victim.stats.hp).max(0);
                        victim.stats.hp += healed;
                        self.says(Happened::AbsorbedLife(healed));
                    }
                    ability::Absorbed::SpeedsUp => {
                        self.apply(MoveEffect::UserSpeedUp, victim);
                        self.says(Happened::AbsorbedSpeed);
                    }
                    ability::Absorbed::Nothing => self.says(Happened::Absorbed),
                }
                return true;
            }
            self.temper(&mut damage, user, victim, type_id, slot.category == 0, hidden == Some(Some(true)));
            if let Some(cap) = self.tuning.cap {
                damage.amount = damage.amount.min(cap);
            }
            victim.stats.hp = (victim.stats.hp - damage.amount).min(victim.stats.max_hp);
            if let Some(taken) = victim.status.bide.as_mut() {
                *taken += damage.amount.max(0);
            }
            victim.status.struck_by_team = user.team;
            victim.status.struck();
            self.struck(user, victim, attack, damage, type_id);
            victim.status.endure(victim.stats);
            // (An exclusive item may hold it on with 1 HP: the game's
            // chance, read from its overlay 10.)
            const ITEM_ENDURE_CHANCE: i32 = 50;
            let endured = !victim.alive() && victim.status.exclusive.has(exclusive::effect::MAY_ENDURE) && self.rolled(ITEM_ENDURE_CHANCE);
            if endured {
                victim.stats.hp = 1;
            }
            self.says(Happened::Hurt(damage));
            if endured {
                self.says(Happened::Endured);
            }
            self.answers(user, victim, attack, damage);
            if !victim.alive() || !user.alive() {
                break;
            }
        }
        true
    }

    /// What weakens or strengthens a blow past the damage counted.
    fn temper(&mut self, damage: &mut Damage, user: &mut Party, victim: &Party, type_id: u8, physical: bool, reached: bool) {
        // Mud Sport and Water Sport halve Electric and Fire moves.
        // (Halved is this project's own figure.)
        if (type_id == ELECTRIC && self.field.mud_sport) || (type_id == FIRE && self.field.water_sport) {
            damage.amount = (damage.amount / 2).max(1);
        }
        if type_id == ELECTRIC && std::mem::take(&mut user.status.charged) {
            damage.amount *= 2;
        }
        // Reached where its two-turn move hides it: twice as hard, mostly.
        if reached {
            damage.amount *= 2;
        }
        // Exclusive items: harder hits; halved, warded or drunk in.
        use exclusive::effect;
        let (given, taken) = (user.status.exclusive, victim.status.exclusive);
        if given.has(effect::DAMAGE_BOOST_50_PCT) {
            damage.amount += damage.amount / 2;
        }
        // (What halves physical or special blows, as Reflect and Light
        // Screen, does not stop who goes through them.)
        let through = given.has(effect::BYPASS_REFLECT_LIGHT_SCREEN);
        if taken.has(effect::HALVED_DAMAGE)
            || (physical && !through && taken.has(effect::HALVED_PHYSICAL_DAMAGE))
            || (!physical && !through && taken.has(effect::HALVED_SPECIAL_DAMAGE))
        {
            damage.amount = (damage.amount / 2).max(1);
        }
        match taken.shields_type(type_id) {
            Some(true) => {
                self.says(Happened::Absorbed);
                damage.amount = -damage.amount.max(1);
            }
            Some(false) => {
                self.says(Happened::Unafraid);
                damage.amount = 0;
            }
            None => {}
        }
        // Bide unleashed: twice what it took.
        if let Some(back) = user.status.bide_release {
            damage.amount = back;
        }
        // Doom Desire, Future Sight: whatever it deals is a set amount.
        if user.status.set_damage_turns > 0 && damage.amount > 0 {
            damage.amount = self.field.constants.set_damage;
        }
    }
}
