//! An effect landing on a monster.
//!
//! First what may keep it off (exclusive items, Heal Block, a shield, an
//! ability, an item held, an IQ skill), then what it does: stat stages,
//! speed, a status problem, HP back, a shield or a timed condition. The
//! effects that move monsters, change the floor or read two monsters are
//! the caller's: here they only succeed.

use super::*;

/// The six stats with stages.
#[derive(Clone, Copy)]
enum Stat {
    Attack,
    Defense,
    SpAttack,
    SpDefense,
    Accuracy,
    Evasion,
}

impl Statuses {
    fn stage_mut(&mut self, stat: Stat) -> &mut i32 {
        match stat {
            Stat::Attack => &mut self.attack_stage,
            Stat::Defense => &mut self.defense_stage,
            Stat::SpAttack => &mut self.sp_attack_stage,
            Stat::SpDefense => &mut self.sp_defense_stage,
            Stat::Accuracy => &mut self.accuracy_stage,
            Stat::Evasion => &mut self.evasion_stage,
        }
    }

    /// Moves a stage by that many steps (twice as many with Simple),
    /// within the stages there are.
    fn shift(&mut self, stat: Stat, by: i32, simple: bool) {
        let by = if simple { by * 2 } else { by };
        let stage = self.stage_mut(stat);
        *stage = (*stage + by).clamp(0, STAGE_COUNT as i32 - 1);
    }
}

/// Multiplies a stat multiplier (8 fraction bits), kept between about a
/// hundredth and a hundred.
fn scale(multiplier: &mut i32, by: i32) {
    *multiplier = ((*multiplier * by + 0x80) >> 8).clamp(2, 100 << 8);
}

/// The stage changes an effect makes, or none.
fn stage_changes(effect: MoveEffect) -> Vec<(Stat, i32)> {
    use MoveEffect::*;
    use Stat::*;
    match effect {
        UserAttack(by) | TargetAttack(by) => vec![(Attack, by)],
        UserDefense(by) | TargetDefense(by) => vec![(Defense, by)],
        UserSpAttack(by) | TargetSpAttack(by) => vec![(SpAttack, by)],
        UserSpDefense(by) | TargetSpDefense(by) => vec![(SpDefense, by)],
        TargetAccuracy(by) => vec![(Accuracy, by)],
        UserEvasion(by) | TargetEvasion(by) => vec![(Evasion, by)],
        CalmMind => vec![(SpAttack, 1), (SpDefense, 1)],
        CosmicPower => vec![(Defense, 1), (SpDefense, 1)],
        CloseCombat => vec![(Defense, -1), (SpDefense, -1)],
        BulkUp | CurseSelf => vec![(Attack, 1), (Defense, 1)],
        Tickle | Superpower => vec![(Attack, -1), (Defense, -1)],
        DragonDance => vec![(Attack, 1)],
        BoostAll => vec![(Attack, 1), (Defense, 1), (SpAttack, 1), (SpDefense, 1)],
        HelpingHand => vec![(Attack, 1), (SpAttack, 1)],
        Swagger => vec![(Attack, 2)],
        Flatter => vec![(SpAttack, 1)],
        _ => Vec::new(),
    }
}

/// The speed change an effect makes: up (true) or down (false).
fn speed_change(effect: MoveEffect) -> Option<bool> {
    use MoveEffect::*;
    match effect {
        UserSpeedUp | DragonDance | BoostAll => Some(true),
        UserSpeedDown | TargetSpeedDown | CurseSelf => Some(false),
        _ => None,
    }
}

/// What the monster's exclusive items keep away.
fn kept_by_exclusive(effect: MoveEffect, statuses: &Statuses) -> bool {
    use crate::dungeon::exclusive::effect::*;
    use MoveEffect::*;
    let has = |effect| statuses.exclusive.has(effect);
    let any_status = has(NO_STATUS);
    match effect {
        Poison | BadlyPoison | Burn => any_status,
        Paralyze => any_status || has(NO_PARALYSIS),
        Freeze => any_status || has(NO_FREEZE),
        Sleep | Yawn => any_status || has(NO_SLEEP),
        Confuse | Swagger | Flatter => any_status || has(NO_CONFUSION),
        Infatuate => has(NO_INFATUATION),
        TargetSpeedDown | Tickle => has(NO_STAT_DROPS),
        _ => lowers_a_target_stage(effect) && has(NO_STAT_DROPS),
    }
}

/// A stage of the target goes down.
fn lowers_a_target_stage(effect: MoveEffect) -> bool {
    use MoveEffect::*;
    matches!(effect, TargetAttack(by) | TargetDefense(by) | TargetAccuracy(by) | TargetEvasion(by) | TargetSpAttack(by) | TargetSpDefense(by) if by < 0)
}

/// What a shield stops: Safeguard the status problems, Mist anything that
/// would weaken the target's stats. What a move does to its own user is
/// never stopped.
fn shield_stops(shield: Shield, effect: MoveEffect) -> bool {
    use MoveEffect::*;
    match shield {
        Shield::Safeguard => matches!(
            effect,
            Sleep | Poison | BadlyPoison | Burn | Paralyze | Freeze | Cringe | Confuse | Yawn | LeechSeed | TriAttack | Swagger | Flatter | Cursed | Nightmare
        ),
        Shield::Mist => {
            lowers_a_target_stage(effect)
                || matches!(effect, Memento | Tickle | TargetSpeedDown)
                || matches!(effect, AttackMultiplier(by) | DefenseMultiplier(by) if by < 0x100)
        }
        _ => false,
    }
}

/// Whether anything keeps this effect off the monster.
fn kept_off(effect: MoveEffect, stats: &Stats, statuses: &Statuses) -> bool {
    use MoveEffect::*;
    let heals = matches!(effect, HealHalf | HealQuarter | HealBy(_) | HealFlat(_) | HealingWish { .. } | Swallow);
    let sleeps = matches!(effect, Sleep | Yawn | Nightmare);
    kept_by_exclusive(effect, statuses)
        || (heals && statuses.heal_block_turns > 0)
        || (sleeps && statuses.sleepless_turns > 0)
        || statuses.shield.is_some_and(|shield| shield_stops(shield, effect))
        || crate::dungeon::ability::keeps_off(stats, effect)
        || crate::dungeon::held::keeps_off(stats, effect)
        || (effect == Sleep && crate::dungeon::iq::has(stats, crate::dungeon::iq::NONSLEEPER))
}

/// Applies a move's effect to the monster it lands on. Returns false when
/// it fails: something keeps it off (Safeguard, Mist...), or there is
/// nothing for it to do.
pub fn apply_effect(effect: MoveEffect, rng: &mut DungeonRng, constants: &Constants, stats: &mut Stats, statuses: &mut Statuses) -> bool {
    if kept_off(effect, stats, statuses) {
        return false;
    }
    let simple = crate::dungeon::ability::has(stats, crate::dungeon::ability::SIMPLE);
    for (stat, by) in stage_changes(effect) {
        statuses.shift(stat, by, simple);
    }
    match speed_change(effect) {
        Some(true) => _ = statuses.boost_speed(rng, constants),
        Some(false) => _ = statuses.lower_speed(rng, constants),
        None => {}
    }
    inflict(effect, rng, constants, stats, statuses)
        && heal(effect, constants, stats, statuses)
        && protect(effect, rng, constants, statuses)
        && condition(effect, rng, constants, stats, statuses)
}

/// The status problems. False when the effect fails.
fn inflict(effect: MoveEffect, rng: &mut DungeonRng, constants: &Constants, stats: &Stats, statuses: &mut Statuses) -> bool {
    use crate::dungeon::ability;
    use MoveEffect::*;
    match effect {
        Sleep => {
            statuses.fall_asleep(rng, constants);
            // Early Birds wake up in half the time.
            if ability::has(stats, ability::EARLY_BIRD) {
                statuses.sleep_turns = (statuses.sleep_turns + 1) / 2;
            }
        }
        Poison => statuses.poison(),
        BadlyPoison => {
            statuses.poison();
            statuses.badly_poisoned = true;
        }
        Burn => statuses.burn(rng, constants),
        Paralyze => statuses.paralyze(rng, constants),
        Freeze => statuses.freeze(rng, constants),
        Cringe => {
            statuses.cringe(rng, constants);
            // Steadfast turns the fright into speed.
            if ability::has(stats, ability::STEADFAST) {
                statuses.boost_speed(rng, constants);
            }
        }
        Confuse | Swagger | Flatter => statuses.confuse(rng, constants),
        TriAttack => match rng.rand_int(3) {
            0 => statuses.burn(rng, constants),
            1 => statuses.freeze(rng, constants),
            _ => statuses.paralyze(rng, constants),
        },
        Cursed => {
            statuses.cursed_turns = Statuses::duration(rng, constants.curse_turns);
            statuses.curse_timer = 0;
        }
        LeechSeed => {
            statuses.leech_seed_turns = Statuses::duration(rng, constants.leech_seed_turns);
            statuses.leech_seed_timer = 0;
        }
        Yawn if statuses.asleep() || statuses.yawning_turns > 0 => return false,
        Yawn => statuses.yawning_turns = Statuses::duration(rng, constants.yawn_turns),
        Nightmare => {
            if !statuses.asleep() {
                statuses.sleep_turns = Statuses::duration(rng, constants.nightmare_turns);
            }
            statuses.nightmare = true;
        }
        Infatuate => statuses.infatuated_turns = Statuses::duration(rng, constants.infatuated_turns),
        _ => {}
    }
    true
}

/// HP back. False when the effect fails.
fn heal(effect: MoveEffect, constants: &Constants, stats: &mut Stats, statuses: &mut Statuses) -> bool {
    use MoveEffect::*;
    let amount = match effect {
        HealQuarter => stats.max_hp / 4,
        HealHalf => stats.max_hp / 2,
        HealBy(index) => constants.heal_amount(index, weather::CLEAR),
        HealFlat(amount) => amount,
        // Swallow: the more was stockpiled, the more comes back.
        Swallow => {
            let back = match statuses.stockpile {
                0 => return false,
                1 => stats.max_hp / 4,
                2 => stats.max_hp / 2,
                _ => stats.max_hp,
            };
            statuses.stockpile = 0;
            back
        }
        HealingWish { .. } => {
            statuses.cure_all();
            stats.max_hp
        }
        _ => return true,
    };
    stats.hp = (stats.hp + amount).min(stats.max_hp);
    true
}

/// The shields: a new one takes the place of the one up.
fn protect(effect: MoveEffect, rng: &mut DungeonRng, constants: &Constants, statuses: &mut Statuses) -> bool {
    use MoveEffect::*;
    let (shield, range) = match effect {
        Protect => (Shield::Protect, constants.protect_turns),
        Safeguard => (Shield::Safeguard, constants.safeguard_turns),
        Mist => (Shield::Mist, constants.mist_turns),
        Reflect => (Shield::Reflect, constants.reflect_turns),
        LightScreen => (Shield::LightScreen, constants.light_screen_turns),
        Endure => (Shield::Endure, constants.endure_turns),
        Wish => (Shield::Wish, constants.wish_turns),
        LuckyChant => (Shield::LuckyChant, constants.lucky_chant_turns),
        AquaRing => {
            statuses.heal_timer = 0;
            (Shield::AquaRing, constants.aqua_ring_turns)
        }
        _ => return true,
    };
    statuses.shield(shield, rng, range);
    true
}

/// Everything else that lasts on the monster: a timed condition, a mark
/// for the caller, a change of stats. False when the effect fails.
fn condition(effect: MoveEffect, rng: &mut DungeonRng, constants: &Constants, stats: &mut Stats, statuses: &mut Statuses) -> bool {
    use MoveEffect::*;
    let has_last_move = statuses.last_move.is_some();
    match effect {
        AttackMultiplier(by) => scale(&mut statuses.attack_multiplier, by),
        DefenseMultiplier(by) => scale(&mut statuses.defense_multiplier, by),
        Memento => {
            scale(&mut statuses.attack_multiplier, 0x40);
            scale(&mut statuses.sp_attack_multiplier, 0x40);
        }
        Ingrain => {
            statuses.ingrain_turns = Statuses::duration(rng, constants.ingrain_turns);
            statuses.ingrain_timer = 0;
        }
        // Rest: a nap, at the end of which the monster wakes healed and
        // cured (see the end of the turn).
        Rest => {
            statuses.sleep_turns = Statuses::duration(rng, constants.rest_turns);
            statuses.napping = true;
        }
        CureStatus => statuses.cure_all(),
        FocusEnergy => statuses.focused = true,
        Taunt => statuses.taunted_turns = Statuses::duration(rng, constants.taunt_turns),
        Expose => statuses.evasion_stage = statuses.evasion_stage.min(NEUTRAL_STAGE),
        BellyDrum => {
            if stats.hp * 2 <= stats.max_hp {
                return false;
            }
            stats.hp -= stats.max_hp / 2;
            statuses.attack_stage = STAGE_COUNT as i32 - 1;
        }
        Splash => return false,
        SureShot => statuses.sure_shot_turns = Statuses::duration(rng, constants.sure_shot_turns),
        Rage => statuses.raging = true,
        Stockpile if statuses.stockpile >= 3 => return false,
        Stockpile => statuses.stockpile += 1,
        Acupressure => {
            const STATS: [Stat; 6] = [Stat::Attack, Stat::Defense, Stat::SpAttack, Stat::SpDefense, Stat::Accuracy, Stat::Evasion];
            let simple = crate::dungeon::ability::has(stats, crate::dungeon::ability::SIMPLE);
            statuses.shift(STATS[rng.rand_int(6) as usize], 2, simple);
        }
        MagicCoat => statuses.bouncing_turns = Statuses::duration(rng, constants.magic_coat_turns),
        MirrorMove => statuses.bouncing_turns = Statuses::duration(rng, constants.mirror_move_turns),
        Snatch => statuses.snatching_turns = Statuses::duration(rng, constants.snatch_turns),
        Invisify => statuses.invisible_turns = Statuses::duration(rng, constants.invisible_turns),
        LayTrap(kind) => statuses.laying = Some(kind),
        Haze => statuses.reset_stages(),
        Charge => statuses.charged = true,
        MagnetRise => statuses.levitating_turns = Statuses::duration(rng, constants.magnet_rise_turns),
        Embargo => statuses.embargo_turns = Statuses::duration(rng, constants.embargo_turns),
        // Mean Look, Block: held in place about as long as a constriction.
        Immobilize => statuses.held_turns = Statuses::duration(rng, constants.constriction_turns),
        Spite | Torment | Encore if !has_last_move => return false,
        Spite => statuses.spited = true,
        Torment => statuses.tormented = statuses.last_move,
        Encore => statuses.encore_turns = Statuses::duration(rng, constants.encore_turns),
        Grudge => statuses.grudge = true,
        HealBlock => statuses.heal_block_turns = Statuses::duration(rng, constants.heal_block_turns),
        PowerTrick => {
            std::mem::swap(&mut statuses.attack_stage, &mut statuses.defense_stage);
            std::mem::swap(&mut statuses.attack_multiplier, &mut statuses.defense_multiplier);
        }
        PerishSong if statuses.perish_turns > 0 => return false,
        PerishSong => statuses.perish_turns = Statuses::duration(rng, constants.perish_turns),
        TrickRoom => return trick_room(rng, constants, statuses),
        Counter(kind) => {
            let range = if kind == CounterKind::Special { constants.mirror_coat_turns } else { constants.counter_turns };
            (statuses.counter, statuses.counter_turns) = (Some(kind), Statuses::duration(rng, range));
        }
        Decoy => (statuses.decoy, statuses.decoy_turns) = (true, Statuses::duration(rng, constants.decoy_turns)),
        Constrict => {
            statuses.constricted_turns = Statuses::duration(rng, constants.constriction_turns);
            statuses.constrict_timer = 0;
        }
        VitalThrow => statuses.vital_throw_turns = Statuses::duration(rng, constants.vital_throw_turns),
        FollowMe => statuses.drawing_turns = Statuses::duration(rng, constants.decoy_turns),
        Conversion2 => statuses.conversion2_turns = Statuses::duration(rng, constants.conversion2_turns),
        Wrap => statuses.paused_turns = Statuses::duration(rng, constants.wrap_turns),
        SetDamage => statuses.set_damage_turns = Statuses::duration(rng, constants.set_damage_turns),
        Pause => statuses.paused_turns = Statuses::duration(rng, constants.imprison_turns),
        // Uproar and Worry Seed wake it, and keep it awake for a while.
        Uproar | WorrySeed => {
            if statuses.sleep_turns != DEEP_SLEEP {
                statuses.sleep_turns = 0;
            }
            (statuses.nightmare, statuses.yawning_turns) = (false, 0);
            statuses.sleepless_turns = Statuses::duration(rng, constants.sleepless_turns);
        }
        _ => {}
    }
    true
}

/// Trick Room turns speed about: a fast monster slows down, a slow or
/// ordinary one speeds up.
fn trick_room(rng: &mut DungeonRng, constants: &Constants, statuses: &mut Statuses) -> bool {
    match statuses.speed_stage() > 1 {
        true => statuses.lower_speed(rng, constants),
        false => statuses.boost_speed(rng, constants),
    }
}
