//! How the log tells of attacks: what an effect did, a hit, its aftermath.

use crate::dungeon::{Damage, MoveEffect, matchup};

use super::{Book, Log};

/// What a status move did to `name`, for the message log.
pub(crate) fn effect_message(book: &Book, effect: MoveEffect, name: &str, applied: bool) -> String {
    if !applied {
        return format!("{name} is protected!");
    }
    if let Some(said) = book.effect_phrase(effect, name) {
        return said;
    }
    let change = |by: i32| if by > 0 { "rose" } else { "fell" };
    match effect {
        MoveEffect::UserAttack(by) | MoveEffect::TargetAttack(by) => {
            format!("{name}'s Attack {}.", change(by))
        }
        MoveEffect::UserDefense(by) | MoveEffect::TargetDefense(by) => {
            format!("{name}'s Defense {}.", change(by))
        }
        MoveEffect::TargetAccuracy(by) => format!("{name}'s accuracy {}.", change(by)),
        MoveEffect::UserSpAttack(by) | MoveEffect::TargetSpAttack(by) => {
            format!("{name}'s Special Attack {}.", change(by))
        }
        MoveEffect::UserSpDefense(by) | MoveEffect::TargetSpDefense(by) => {
            format!("{name}'s Special Defense {}.", change(by))
        }
        MoveEffect::CalmMind => format!("{name}'s Sp. Atk. and Sp. Def. rose."),
        MoveEffect::CosmicPower => format!("{name}'s Defense and Sp. Def. rose."),
        MoveEffect::CloseCombat => format!("{name}'s Defense and Sp. Def. fell."),
        MoveEffect::Flatter => format!("{name} is confused, its Special Attack rose."),
        MoveEffect::Memento => format!("{name}'s Attack and Special Attack plummeted."),
        MoveEffect::Reflect => format!("{name} is protected by Reflect."),
        MoveEffect::LightScreen => format!("{name} is protected by Light Screen."),
        MoveEffect::AttackMultiplier(_) => format!("{name}'s Attack plummeted."),
        MoveEffect::DefenseMultiplier(_) => format!("{name}'s Defense plummeted."),
        MoveEffect::Swagger => format!("{name} is confused, its Attack rose sharply."),
        MoveEffect::Tickle | MoveEffect::Superpower => {
            format!("{name}'s Attack and Defense fell.")
        }
        MoveEffect::BulkUp => format!("{name}'s Attack and Defense rose."),
        MoveEffect::DragonDance => format!("{name}'s Attack and Speed rose."),
        MoveEffect::BoostAll => format!("{name}'s stats rose."),
        MoveEffect::UserSpeedDown => format!("{name}'s Speed fell."),
        MoveEffect::HealQuarter | MoveEffect::HealBy(_) | MoveEffect::HealFlat(_) => {
            format!("{name} recovered HP.")
        }
        MoveEffect::Weather(_) => format!("{name} changed the weather."),
        MoveEffect::Warp => format!("{name} warped."),
        MoveEffect::Blowback => format!("{name} was blown away!"),
        MoveEffect::Endure => format!("{name} braced itself."),
        MoveEffect::Wish => format!("{name} made a wish."),
        MoveEffect::Ingrain => format!("{name} planted its roots."),
        MoveEffect::AquaRing => format!("{name} surrounded itself with a veil of water."),
        MoveEffect::Rest => format!("{name} fell asleep and recovered."),
        MoveEffect::Curse | MoveEffect::CurseSelf => {
            format!("{name}'s Attack and Defense rose, its Speed fell.")
        }
        MoveEffect::Cursed => format!("{name} is cursed."),
        MoveEffect::CureStatus => format!("{name} is cured."),
        MoveEffect::Protect => format!("{name} protected itself."),
        MoveEffect::Safeguard => format!("{name} is under Safeguard."),
        MoveEffect::Mist => format!("{name} is shrouded in Mist."),
        MoveEffect::UserSpeedUp => format!("{name}'s Speed rose."),
        MoveEffect::TargetSpeedDown => format!("{name}'s Speed fell."),
        MoveEffect::Burn => format!("{name} is burned."),
        MoveEffect::Paralyze => format!("{name} is paralyzed."),
        MoveEffect::Freeze => format!("{name} is frozen."),
        MoveEffect::Cringe => format!("{name} cringed."),
        MoveEffect::Confuse => format!("{name} is confused."),
        MoveEffect::TargetEvasion(by) | MoveEffect::UserEvasion(by) => {
            format!("{name}'s evasion {}.", change(by))
        }
        MoveEffect::Sleep => format!("{name} fell asleep."),
        MoveEffect::Poison => format!("{name} is poisoned."),
        MoveEffect::BadlyPoison => format!("{name} is badly poisoned."),
        MoveEffect::FocusEnergy => format!("{name} is focusing: its hits will be critical."),
        MoveEffect::LeechSeed => format!("{name} was seeded by Leech Seed."),
        MoveEffect::Yawn => format!("{name} began to yawn."),
        MoveEffect::Taunt => format!("{name} is taunted."),
        MoveEffect::Expose => format!("{name} is exposed."),
        MoveEffect::BellyDrum => format!("{name} cut its HP: its Attack is maxed out!"),
        MoveEffect::Splash => "But nothing happened.".to_string(),
        MoveEffect::SureShot => format!("{name} takes aim: its hits will not miss."),
        MoveEffect::Infatuate => format!("{name} fell in love!"),
        MoveEffect::Rage => format!("{name} flew into a rage."),
        MoveEffect::Stockpile => format!("{name} stockpiled energy."),
        MoveEffect::Swallow => format!("{name} swallowed its stockpile and recovered HP."),
        MoveEffect::Acupressure => format!("One of {name}'s stats rose sharply!"),
        MoveEffect::MagicCoat => format!("{name} is protected by Magic Coat!"),
        MoveEffect::Snatch => format!("{name} waits to snatch the foes' effects!"),
        MoveEffect::Invisify => format!("{name} became invisible!"),
        MoveEffect::LayTrap(_) => format!("{name} laid a trap."),
        MoveEffect::Sport { water: false } => "Electricity's power was weakened!".to_string(),
        MoveEffect::Sport { water: true } => "Fire's power was weakened!".to_string(),
        MoveEffect::WorrySeed => format!("{name} can no longer sleep."),
        MoveEffect::TriAttack => format!("{name} was hit by Tri Attack."),
        MoveEffect::Haze => format!("{name}'s stat changes were removed."),
        MoveEffect::Charge => format!("{name} is charged with electricity."),
        MoveEffect::MagnetRise => format!("{name} levitates with magnetism!"),
        MoveEffect::Embargo => format!("{name} can no longer use items!"),
        MoveEffect::Immobilize => format!("{name} can no longer move!"),
        MoveEffect::Encore => format!("{name} must repeat its last move!"),
        MoveEffect::PsychUp => format!("{name}'s stat changes were copied."),
        MoveEffect::HelpingHand => format!("{name} is cheered on: its Attack and Special Attack rose!"),
        MoveEffect::LuckyChant => format!("{name} is protected from critical hits!"),
        MoveEffect::HealingWish { .. } => format!("{name} is fully healed!"),
        MoveEffect::Nightmare => format!("{name} fell into a nightmare!"),
        MoveEffect::PainSplit => format!("{name} shared its HP."),
        MoveEffect::Spite => format!("{name} fell victim to Spite!"),
        MoveEffect::Grudge => format!("{name} bears a grudge."),
        MoveEffect::HealBlock => format!("{name} can no longer recover HP!"),
        MoveEffect::Torment => format!("{name} is tormented!"),
        MoveEffect::Uproar => format!("{name} can no longer sleep!"),
        MoveEffect::Swap(_) => format!("{name} swapped its stat changes."),
        MoveEffect::PowerTrick => format!("{name} swapped its Attack and Defense."),
        MoveEffect::Trick => format!("{name} swapped its item."),
        MoveEffect::PerishSong => format!("{name} heard the Perish Song!"),
        MoveEffect::TrickRoom => format!("{name}'s Speed changed."),
        MoveEffect::Counter(_) => format!("{name} is ready to strike back."),
        MoveEffect::Explode(_) => format!("{name} exploded!"),
        MoveEffect::SmashWall => format!("{name} struck the wall."),
        MoveEffect::SetDamage => format!("{name}'s damage is set."),
        MoveEffect::PsychoShift => format!("{name} received the status problems."),
        MoveEffect::MirrorMove => format!("{name} will reflect moves back."),
        MoveEffect::Pause => format!("{name} is frozen in place!"),
        MoveEffect::RolePlay => format!("{name} copied an ability."),
        MoveEffect::SkillSwap => format!("{name} swapped its ability."),
        MoveEffect::GastroAcid => format!("{name} lost its ability!"),
        MoveEffect::Conversion => format!("{name}'s type changed!"),
        MoveEffect::Mimic => format!("{name} was mimicked."),
        MoveEffect::Sketch => format!("{name} was sketched."),
        MoveEffect::Decoy => format!("{name} became a decoy: its allies take it for a foe!"),
        MoveEffect::Wrap => format!("{name} is bound!"),
        MoveEffect::Hurl => format!("{name} was hurled!"),
        MoveEffect::KnockOff => format!("{name} dropped its item!"),
        MoveEffect::BeatUp => format!("{name}'s team rushes in!"),
        MoveEffect::Camouflage => format!("{name}'s type changed!"),
        MoveEffect::Gravity => "Gravity intensified!".to_string(),
        MoveEffect::DestinyBond => format!("{name} is bound by destiny!"),
        MoveEffect::Transform => format!("{name} transformed!"),
        MoveEffect::SwitchPlaces => format!("{name} switched places."),
        MoveEffect::Recycle => "The used TMs are like new again!".to_string(),
        MoveEffect::FollowMe => format!("{name} draws attention!"),
        MoveEffect::VitalThrow => format!("{name} is ready to throw!"),
        MoveEffect::Constrict => format!("{name} is bound!"),
        MoveEffect::Conversion2 => format!("{name} is about to change its type!"),
        MoveEffect::HealHalf => format!("{name} recovered HP."),
    }
}

/// Recoil (HP lost) or drain (HP gained) after a hit.
pub(crate) fn log_aftermath(log: &mut Log, user: &str, lost: i32) {
    if lost > 0 {
        log.push(format!("{user} took {lost} HP of recoil damage."));
    } else if lost < 0 {
        log.push(format!("{user} recovered {} HP.", -lost));
    }
}

/// Reports a hit the way the original does: critical, matchup, amount.
pub(crate) fn log_damage(book: &Book, log: &mut Log, target: &str, damage: Damage) {
    use crate::strings::log_text;
    if damage.critical {
        log.push(book.phrase(log_text::CRITICAL, &[]).unwrap_or_else(|| "A critical hit!".to_string()));
    }
    match damage.matchup {
        matchup::SUPER_EFFECTIVE => log.push(book.phrase(log_text::SUPER_EFFECTIVE, &[]).unwrap_or_else(|| "It's super effective!".to_string())),
        matchup::NOT_VERY_EFFECTIVE => log.push(book.phrase(log_text::NOT_VERY_EFFECTIVE, &[]).unwrap_or_else(|| "It's not very effective...".to_string())),
        matchup::IMMUNE => log.push("It had little effect...".to_string()),
        _ => {}
    }
    if damage.amount == 0 {
        log.push(format!("{target} took no damage."));
    } else if damage.amount < 0 {
        // (Present's gift.)
        log.push(format!("{target} recovered {} HP.", -damage.amount));
    } else {
        let amount = damage.amount.to_string();
        log.push(
            book.phrase(log_text::TOOK_DAMAGE, &[("string:1", target), ("digits_c:0", &amount)]).unwrap_or_else(|| format!("{target} took {amount} damage.")),
        );
    }
}
