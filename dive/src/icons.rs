//! The icons over monsters for their statuses, taking turns (the original's
//! `UpdateStatusIconFlags`, for the statuses this game has).

use rules::dungeon::status::{CounterKind, NEUTRAL_STAGE, Shield};
use rules::dungeon::{Stats, Statuses};

/// The frames of an icon before the next.
pub const TURN: u32 = 64;
/// The frames of a step of an icon.
pub const FRAME: u32 = 4;
/// Pixels above a monster's tile's middle its icons are.
pub const ABOVE: f32 = 20.0;

pub fn names(stats: &Stats, status: &Statuses) -> Vec<&'static str> {
    // (As `status_icon_flags` has them: Lucky Chant and Counter under
    // Reflect's, Mirror Coat under Safeguard's, Magic Coat under Light
    // Screen's.)
    let shield = match status.shield {
        Some(Shield::Protect) => Some("protect"),
        Some(Shield::Safeguard) => Some("safeguard"),
        Some(Shield::Mist | Shield::Reflect | Shield::LuckyChant) => Some("reflect"),
        Some(Shield::LightScreen) => Some("light_screen"),
        Some(Shield::Endure) => Some("endure"),
        _ => None,
    };
    let counter = match status.counter {
        Some(CounterKind::Special) => Some("safeguard"),
        Some(_) => Some("reflect"),
        None => None,
    };
    let lowered = [status.attack_stage, status.defense_stage, status.sp_attack_stage, status.sp_defense_stage, status.accuracy_stage, status.evasion_stage]
        .iter()
        .any(|&stage| stage < NEUTRAL_STAGE);
    [
        (status.sleep_turns > 0).then_some("sleep"),
        (status.sleepless_turns > 0).then_some("sleepless"),
        (status.burn_turns > 0).then_some("burn"),
        match (status.badly_poisoned, status.poisoned) {
            (true, _) => Some("toxic"),
            (false, true) => Some("poison"),
            _ => None,
        },
        (status.cursed_turns > 0).then_some("curse"),
        (status.confused_turns > 0).then_some("confused"),
        (status.cross_eyed_turns > 0).then_some("cross_eyed"),
        (status.cringe_turns > 0).then_some("cringe"),
        (status.taunted_turns > 0).then_some("taunt"),
        (status.encore_turns > 0).then_some("encore"),
        (status.embargo_turns > 0).then_some("embargo"),
        (status.heal_block_turns > 0).then_some("heal_block"),
        (status.sure_shot_turns > 0).then_some("sure_shot"),
        (status.set_damage_turns > 0).then_some("set_damage"),
        status.focused.then_some("focus_energy"),
        status.grudge.then_some("grudge"),
        (status.vital_throw_turns > 0).then_some("protect"),
        (status.bouncing_turns > 0).then_some("light_screen"),
        shield,
        counter,
        (stats.hp * 4 <= stats.max_hp).then_some("low_hp"),
        lowered.then_some("lowered"),
    ]
    .into_iter()
    .flatten()
    .collect()
}
