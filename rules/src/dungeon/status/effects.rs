//! What a move does, by its number.

use super::*;

/// What a move without damage does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveEffect {
    /// Stat stage changes on the user.
    UserAttack(i32),
    UserDefense(i32),
    /// Stat stage changes on the targets.
    TargetAttack(i32),
    TargetDefense(i32),
    TargetAccuracy(i32),
    TargetEvasion(i32),
    UserEvasion(i32),
    /// Special attack and special defense stages.
    UserSpAttack(i32),
    UserSpDefense(i32),
    TargetSpAttack(i32),
    TargetSpDefense(i32),
    /// Raises the user's special attack and special defense by one stage.
    CalmMind,
    /// Raises the user's defense and special defense by one stage.
    CosmicPower,
    /// After Close Combat: the user's defense and special defense drop.
    CloseCombat,
    /// Confuses the target and raises its special attack by one stage.
    Flatter,
    /// Quarters the target's attack and special attack.
    Memento,
    /// The user's screens: physical or special damage taken is halved.
    Reflect,
    LightScreen,
    /// Movement speed, by one stage.
    UserSpeedUp,
    TargetSpeedDown,
    /// Multiplies the target's attack or defense (8-bit fixed point).
    AttackMultiplier(i32),
    DefenseMultiplier(i32),
    /// Confuses the target and raises its attack by two stages.
    Swagger,
    /// Lowers the target's attack and defense by one stage.
    Tickle,
    /// Raises the user's attack and defense by one stage.
    BulkUp,
    /// Raises the user's attack and speed by one stage.
    DragonDance,
    /// After Superpower: the user's attack and defense drop by one stage.
    Superpower,
    /// After Hammer Arm: the user slows down.
    UserSpeedDown,
    /// Raises the user's attack, defense and speed by one stage.
    BoostAll,
    /// The user recovers a quarter of its max HP.
    HealQuarter,
    /// The user recovers HP from `Constants::heal_tables[n]` (4: Heal
    /// Order), according to the weather.
    HealBy(usize),
    /// The user recovers this many HP.
    HealFlat(i32),
    /// Changes the floor's weather; the caller carries it out.
    Weather(u8),
    /// Sends the user to a random tile; the caller carries it out.
    Warp,
    /// Blows the target away from the user; the caller carries it out.
    Blowback,
    /// The user's natural regeneration changes while it lasts.
    Wish,
    /// The user heals regularly; Ingrain also roots it to its tile.
    Ingrain,
    AquaRing,
    /// The user survives any hit with 1 HP while it lasts.
    Endure,
    /// The user falls asleep, fully healed and cured.
    Rest,
    /// Curse, before the user's type settles what it does.
    Curse,
    /// Curse used by a Ghost type: the target loses a quarter of its max
    /// HP every few turns.
    Cursed,
    /// Curse used by any other type: attack and defense up, speed down.
    CurseSelf,
    /// Cures the user's status problems.
    CureStatus,
    /// The user's protections; they replace one another.
    Protect,
    Safeguard,
    Mist,
    Sleep,
    Poison,
    /// Toxic's.
    BadlyPoison,
    /// Focus Energy: its blows are critical hits.
    FocusEnergy,
    /// Leech Seed: HP lost every few turns.
    LeechSeed,
    /// Yawn: asleep a little later.
    Yawn,
    /// Taunt: no status moves for a while.
    Taunt,
    /// Foresight, Odor Sleuth: its evasion back to normal.
    Expose,
    /// Belly Drum: half its HP for the highest attack.
    BellyDrum,
    /// Splash: nothing.
    Splash,
    /// Mind Reader, Lock-On: its moves do not miss.
    SureShot,
    /// Attract: too smitten to attack.
    Infatuate,
    /// Rage: its attack rises each time it is hurt.
    Rage,
    /// Stockpile, and Swallow which heals by what was stockpiled.
    Stockpile,
    Swallow,
    /// Acupressure: one stat sharply up, at random.
    Acupressure,
    /// Magic Coat, Snatch (also the Rebound and Snatch Orbs'), the Invisify
    /// Orb's invisibility.
    MagicCoat,
    Snatch,
    Invisify,
    /// Spikes, Toxic Spikes: a trap of this kind under its user.
    LayTrap(usize),
    /// Mud Sport and Water Sport weaken Electric and Fire moves on the floor
    /// (for the caller to carry out: `water` for Water Sport).
    Sport {
        water: bool,
    },
    /// Worry Seed: awake, it cannot fall asleep for a while.
    WorrySeed,
    /// Tri Attack: burned, frozen or paralyzed, at random.
    TriAttack,
    /// Haze: its stat changes undone.
    Haze,
    /// Charge: its next Electric move hits twice as hard.
    Charge,
    /// Magnet Rise: it floats (over water and lava, out of reach of Ground
    /// moves).
    MagnetRise,
    /// Embargo: no items for a while.
    Embargo,
    /// Mean Look, Block: it cannot move away from its tile.
    Immobilize,
    /// Encore: it must use its last move again for a while.
    Encore,
    /// Psych Up: its user takes on its stat changes (for the caller to
    /// carry out: it reads one monster and changes another).
    PsychUp,
    /// Helping Hand: the user's teammates' Attack and Special Attack up a
    /// level (its description).
    HelpingHand,
    /// Lucky Chant: no critical hits for a while.
    LuckyChant,
    /// Healing Wish, Lunar Dance: the user faints, its teammates are healed
    /// of HP and status problems (and of PP: Lunar Dance; see the caller).
    HealingWish {
        pp: bool,
    },
    /// Nightmare: asleep, and hurt on waking up.
    Nightmare,
    /// Pain Split: the user's and the target's HP shared evenly (see the
    /// caller).
    PainSplit,
    /// Spite: the target's last move used loses all its PP.
    Spite,
    /// Grudge: whoever knocks the user out loses the PP of that move.
    Grudge,
    /// Heal Block: no HP restored for a while.
    HealBlock,
    /// Torment: the target's last move used cannot be used again on the
    /// floor.
    Torment,
    /// Uproar: awake, and kept from sleeping.
    Uproar,
    /// Guard Swap, Power Swap, Heart Swap: the stat changes of the user and
    /// the target swapped (see the caller).
    Swap(StatSwap),
    /// Power Trick: the user's Attack changes and Defense changes swapped.
    PowerTrick,
    /// Trick, Switcheroo: held items swapped, when both hold one (see the
    /// caller).
    Trick,
    /// Perish Song: it faints when the count runs out.
    PerishSong,
    /// Trick Room: the speed a level up or down, at random.
    TrickRoom,
    /// Counter, Payback, Pursuit, Mirror Coat, Metal Burst: damage taken comes back
    /// to who dealt it.
    Counter(CounterKind),
    /// Selfdestruct, Explosion: the user blows up, this far around (see
    /// the caller).
    Explode(i32),
    /// Rock Smash: the wall the user faces breaks (see the caller).
    SmashWall,
    /// Doom Desire, Future Sight: the user's damage is set for a while.
    SetDamage,
    /// Psycho Shift: the user's status problems go to the target (see the
    /// caller).
    PsychoShift,
    /// Mirror Move: moves used on it bounce back (as Magic Coat).
    MirrorMove,
    /// Imprison: the target is paused, unable to act.
    Pause,
    /// Role Play: the user takes the target's abilities; Skill Swap: the two
    /// swap them; Gastro Acid: the target's are gone (see the caller). For
    /// the floor only.
    RolePlay,
    SkillSwap,
    GastroAcid,
    /// Conversion: the user's type becomes one of its moves' (see the
    /// caller).
    Conversion,
    /// Mimic: the target's last move, used on it in turn; Sketch: learnt
    /// in Sketch's place (see the caller).
    Mimic,
    Sketch,
    /// Substitute: the target becomes a decoy its own side goes for.
    Decoy,
    /// Wrap: the user and the target are both held, unable to act (see
    /// the caller).
    Wrap,
    /// Strength, Fling: the monster in front is hurled at another of its
    /// side; Knock Off: what it holds falls to the ground; Beat Up: the
    /// user's teammates come around it (see the caller).
    Hurl,
    KnockOff,
    BeatUp,
    /// Camouflage: the user takes the type of the floor's ground (see the
    /// caller).
    Camouflage,
    /// Gravity: on the floor, Flying types and levitating monsters are
    /// struck by Ground moves (see the caller).
    Gravity,
    /// Destiny Bond: what the user loses, the target loses too, for a
    /// while (see the caller).
    DestinyBond,
    /// Transform: the user looks like a monster of the floor for a while
    /// (see the caller).
    Transform,
    /// Baton Pass: the user switches places with the others of its room
    /// (see the caller).
    SwitchPlaces,
    /// Recycle: the team's Used TMs are TMs again (see the caller).
    Recycle,
    /// Constriction (Whirlpool, Bind...): stuck in place, and hurt now and
    /// then.
    Constrict,
    /// Follow Me: the user draws its foes' blows for a while.
    FollowMe,
    /// Vital Throw: those who strike the user up close are hurled (see
    /// the caller), for a while.
    VitalThrow,
    /// Conversion 2: hit by a move, the user takes a type that resists it
    /// (see the caller), for a while.
    Conversion2,
    Burn,
    Paralyze,
    Freeze,
    Cringe,
    Confuse,
    /// The user recovers half of its max HP.
    HealHalf,
}

/// Which stat changes a swap takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatSwap {
    Defenses,
    Attacks,
    All,
}

/// What damage a Counter sends back, and how much of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterKind {
    /// Counter, Payback: physical damage, in full.
    Physical,
    /// Mirror Coat: special damage, in full.
    Special,
    /// Metal Burst: any damage, half of it.
    Any,
}

impl CounterKind {
    /// What comes back of damage of this category (0 physical).
    pub fn returned(self, category: u8, amount: i32) -> i32 {
        match (self, category) {
            (Self::Physical, 0) => amount,
            (Self::Special, 1) => amount,
            (Self::Any, 0 | 1) => amount / 2,
            _ => 0,
        }
    }
}

impl MoveEffect {
    pub fn on_user(self) -> bool {
        matches!(
            self,
            Self::UserAttack(_)
                | Self::UserDefense(_)
                | Self::UserEvasion(_)
                | Self::UserSpeedUp
                | Self::UserSpAttack(_)
                | Self::UserSpDefense(_)
                | Self::CalmMind
                | Self::CosmicPower
                | Self::CloseCombat
                | Self::Reflect
                | Self::LightScreen
                | Self::BulkUp
                | Self::DragonDance
                | Self::Superpower
                | Self::UserSpeedDown
                | Self::BoostAll
                | Self::HealHalf
                | Self::HealQuarter
                | Self::HealBy(_)
                | Self::HealFlat(_)
                | Self::Weather(_)
                | Self::Warp
                | Self::Endure
                | Self::Wish
                | Self::Ingrain
                | Self::AquaRing
                | Self::Rest
                | Self::CurseSelf
                | Self::CureStatus
                | Self::Protect
                | Self::Safeguard
                | Self::Mist
                | Self::FocusEnergy
                | Self::BellyDrum
                | Self::Splash
                | Self::SureShot
                | Self::Rage
                | Self::Stockpile
                | Self::Swallow
                | Self::Acupressure
                | Self::MagicCoat
                | Self::Snatch
                | Self::Invisify
                | Self::LayTrap(_)
                | Self::Sport { .. }
                | Self::Charge
                | Self::MagnetRise
                | Self::HelpingHand
                | Self::LuckyChant
                | Self::FollowMe
                | Self::VitalThrow
                | Self::Conversion2
                | Self::HealingWish { .. }
                | Self::Grudge
                | Self::Uproar
                | Self::PowerTrick
                | Self::Counter(_)
                | Self::Explode(_)
                | Self::SmashWall
                | Self::SetDamage
                | Self::MirrorMove
                | Self::Conversion
                | Self::Hurl
                | Self::KnockOff
                | Self::BeatUp
                | Self::Camouflage
                | Self::Gravity
                | Self::Transform
                | Self::SwitchPlaces
                | Self::Recycle
        )
    }
}

/// Effect of a status move, by the move's number in the game data.
// (The well-known effects of these moves, stage counts included; where a
// move is simplified, its variant says how. Each number is in one family
// alone.)
pub fn move_effect(move_id: u16) -> Option<MoveEffect> {
    stage_effect(move_id).or_else(|| guard_effect(move_id)).or_else(|| ailment_effect(move_id)).or_else(|| other_effect(move_id))
}

/// The moves that raise or lower stats, or trade them.
fn stage_effect(move_id: u16) -> Option<MoveEffect> {
    use MoveEffect::*;
    Some(match move_id {
        // Growl; FeatherDance.
        217 => TargetAttack(-1),
        267 => TargetAttack(-2),
        // Charm halves the attack, Memento quarters it, Screech quarters
        // the defense.
        11 => AttackMultiplier(0x80),
        43 => Memento,
        27 => DefenseMultiplier(0x40),
        // Swagger, Tickle, Flatter.
        24 => Swagger,
        85 => Tickle,
        45 => Flatter,
        // Growth; Tail Glow, Nasty Plot; Amnesia; Calm Mind; Metal Sound.
        148 => UserSpAttack(1),
        288 | 542 => UserSpAttack(2),
        215 => UserSpDefense(2),
        319 => CalmMind,
        83 => TargetSpDefense(-3),
        // Cosmic Power, Bulk Up, Dragon Dance.
        99 => CosmicPower,
        265 => BulkUp,
        343 => DragonDance,
        // Tail Whip, Leer.
        122 | 224 => TargetDefense(-1),
        // Sharpen, Howl, Meditate; Swords Dance.
        55 | 195 | 333 => UserAttack(1),
        181 => UserAttack(2),
        // Harden, Withdraw, Defense Curl; Iron Defense, Acid Armor, Barrier.
        58 | 70 | 302 => UserDefense(1),
        186 | 202 | 258 => UserDefense(2),
        // Smokescreen, Sand-Attack, Kinesis, Flash.
        37 | 142 | 146 | 273 => TargetAccuracy(-1),
        // Sweet Scent.
        10 => TargetEvasion(-1),
        // Double Team, Minimize.
        56 | 169 => UserEvasion(1),
        74 => FocusEnergy,
        // Defend Order, Defog (its evasion part).
        520 => CosmicPower,
        456 => TargetEvasion(-1),
        448 => Swap(StatSwap::Defenses),
        513 => Swap(StatSwap::Attacks),
        506 => Swap(StatSwap::All),
        514 => PowerTrick,
        257 => BellyDrum,
        492 => Acupressure,
        88 => Haze,
        115 => PsychUp,
        // Fake Tears, Captivate; String Shot; Rock Polish.
        33 => TargetSpDefense(-2),
        534 => TargetSpAttack(-2),
        23 => TargetSpeedDown,
        540 => UserSpeedUp,
        // Agility, Tailwind and the other moves of the speed boost effect.
        95 | 372 | 426 | 444 => UserSpeedUp,
        // Scary Face, Cotton Spore.
        106 | 351 => TargetSpeedDown,
        _ => return None,
    })
}

/// The moves that heal, shield or change the weather.
fn guard_effect(move_id: u16) -> Option<MoveEffect> {
    use MoveEffect::*;
    Some(match move_id {
        // Reflect, Light Screen.
        338 => Reflect,
        259 => LightScreen,
        // Softboiled, Milk Drink.
        165 | 318 => HealQuarter,
        // Morning Sun, Synthesis, Moonlight, Roost, Heal Order.
        6 => HealBy(0),
        94 => HealBy(1),
        173 => HealBy(2),
        509 => HealBy(3),
        449 => HealBy(4),
        // Wish, Ingrain, Aqua Ring.
        225 => Wish,
        233 => Ingrain,
        434 => AquaRing,
        // Endure, Rest, Curse.
        104 => Endure,
        232 => Rest,
        237 => Curse,
        // Rain Dance, Hail, Sandstorm, Sunny Day.
        12 => Weather(weather::RAIN),
        14 => Weather(weather::HAIL),
        141 => Weather(weather::SANDSTORM),
        223 => Weather(weather::SUNNY),
        // Aromatherapy, Heal Bell, Refresh.
        15 | 26 | 339 => CureStatus,
        // Protect, Detect; Safeguard; Mist.
        301 | 305 => Protect,
        134 => Safeguard,
        130 => Mist,
        298 => MagicCoat,
        334 => Snatch,
        212 => Sport { water: false },
        307 => Sport { water: true },
        128 => Charge,
        493 => MagnetRise,
        185 => HelpingHand,
        447 => LuckyChant,
        439 => HealingWish { pp: false },
        526 => HealingWish { pp: true },
        // Recover, Slack Off.
        117 | 218 => HealHalf,
        _ => return None,
    })
}

/// The moves that leave the target in a state.
fn ailment_effect(move_id: u16) -> Option<MoveEffect> {
    use MoveEffect::*;
    Some(match move_id {
        // Lovely Kiss, Sing, Spore, GrassWhistle, Hypnosis, Sleep Powder.
        4 | 34 | 79 | 84 | 111 | 231 => Sleep,
        // Poison Gas, PoisonPowder; Toxic.
        196 | 199 => Poison,
        197 => BadlyPoison,
        329 => LeechSeed,
        3 => Yawn,
        172 => Taunt,
        // Foresight, Odor Sleuth, Miracle Eye.
        315 | 54 | 528 => Expose,
        5 => Nightmare,
        35 => Spite,
        49 => Grudge,
        450 => HealBlock,
        22 => Torment,
        112 => Uproar,
        // Dark Void, Spider Web.
        483 => Sleep,
        86 => Immobilize,
        293 => PerishSong,
        464 => PsychoShift,
        438 => GastroAcid,
        294 => Wrap,
        326 => Infatuate,
        504 => WorrySeed,
        466 => Embargo,
        // Mean Look, Block.
        89 | 194 => Immobilize,
        17 => Encore,
        // Will-O-Wisp.
        47 => Burn,
        // Thunder Wave and the three moves of the plain paralysis effect.
        59 | 124 | 192 | 281 => Paralyze,
        // The plain confusion effect (Confuse Ray, Supersonic, Sweet Kiss,
        // Teeter Dance...).
        13 | 171 | 191 | 274 | 382 => Confuse,
        _ => return None,
    })
}

/// The rest: moves that move, copy, trade, hit back or change the floor.
fn other_effect(move_id: u16) -> Option<MoveEffect> {
    use MoveEffect::*;
    Some(match move_id {
        // Teleport and the other moves of the warp effect.
        187 | 369 | 424 => Warp,
        // Whirlwind, Roar and the blowback effect.
        268 | 284 | 368 => Blowback,
        21 => PainSplit,
        209 | 482 => Trick,
        499 => TrickRoom,
        // Counter, Payback, and Pursuit (which in these games strikes back
        // as Counter does).
        51 | 472 | 38 => Counter(CounterKind::Physical),
        316 => Counter(CounterKind::Special),
        532 => Counter(CounterKind::Any),
        // Selfdestruct, Explosion.
        123 => Explode(1),
        155 => Explode(2),
        29 => SmashWall,
        480 => LayTrap(trap::STEALTH_ROCK),
        256 | 317 => SetDamage,
        40 => MirrorMove,
        266 => Pause,
        220 => RolePlay,
        137 => SkillSwap,
        183 => Conversion,
        327 => Mimic,
        138 => Sketch,
        304 => Decoy,
        50 | 503 => Hurl,
        249 => KnockOff,
        269 => BeatUp,
        286 => Camouflage,
        478 => Gravity,
        313 => DestinyBond,
        282 => Transform,
        252 => SwitchPlaces,
        337 => Recycle,
        102 => FollowMe,
        7 => VitalThrow,
        184 => Conversion2,
        254 => Splash,
        // Mind Reader, Lock-On.
        98 | 349 => SureShot,
        19 => Rage,
        160 => Stockpile,
        236 => Swallow,
        295 => LayTrap(trap::SPIKED_TILE),
        496 => LayTrap(trap::TOXIC_SPIKES),
        _ => return None,
    })
}
