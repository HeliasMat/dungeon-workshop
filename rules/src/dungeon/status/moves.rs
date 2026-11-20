//! What each move does past its effect: strikes, power, what follows.

use super::*;

/// Damaging moves whose chance of an effect on the target comes with the
/// extracted game data, in the order of `Constants::secondary_chances`
/// (the order the extractor reads them in).
pub const ROM_SECONDARY_EFFECTS: [(&[u16], MoveEffect); 32] = [
    (&[158], MoveEffect::Cringe),
    (&[42], MoveEffect::TargetAttack(-1)),
    (&[1], MoveEffect::TargetDefense(-1)),
    (&[164], MoveEffect::Poison),
    (&[62, 275], MoveEffect::TargetDefense(-1)),
    (&[484], MoveEffect::TargetSpDefense(-1)),
    (&[283], MoveEffect::Poison),
    (&[188], MoveEffect::Paralyze),
    (&[205], MoveEffect::Paralyze),
    (&[139], MoveEffect::Cringe),
    (&[522], MoveEffect::Cringe),
    (&[149], MoveEffect::Burn),
    (&[44], MoveEffect::TargetAccuracy(-1)),
    (&[468], MoveEffect::TargetSpDefense(-1)),
    (&[536], MoveEffect::TargetSpDefense(-1)),
    (&[461], MoveEffect::Freeze),
    (&[452], MoveEffect::Paralyze),
    (&[100], MoveEffect::Cringe),
    (&[461], MoveEffect::Cringe),
    (&[276], MoveEffect::Burn),
    (&[25], MoveEffect::Cringe),
    (&[332], MoveEffect::TargetDefense(-1)),
    (&[109], MoveEffect::TargetSpDefense(-1)),
    (&[530], MoveEffect::TargetSpDefense(-1)),
    (&[127], MoveEffect::TargetSpDefense(-1)),
    (&[16], MoveEffect::TargetSpeedDown),
    (&[198], MoveEffect::Poison),
    (&[129], MoveEffect::Paralyze),
    (&[162], MoveEffect::Cringe),
    (&[226], MoveEffect::Cringe),
    (&[452], MoveEffect::Cringe),
    (&[517], MoveEffect::Burn),
];

/// Everything a damaging move may also do to its target, each with its
/// chance in percent.
pub fn secondary_effects(move_id: u16, constants: &Constants) -> Vec<(i32, MoveEffect)> {
    const TRI_ATTACK: u16 = 207;
    let mut effects: Vec<(i32, MoveEffect)> = secondary_effect(move_id, constants).into_iter().collect();
    for ((moves, effect), chance) in ROM_SECONDARY_EFFECTS.iter().zip(constants.secondary_chances) {
        if moves.contains(&move_id) {
            effects.push((chance, *effect));
        }
    }
    if move_id == TRI_ATTACK {
        effects.push((constants.tri_attack_chance, MoveEffect::TriAttack));
    }
    effects
}

/// The extra effects of the damaging moves this engine knows by heart,
/// in groups: the moves, the chance in percent, the effect.
const EXTRA_EFFECTS: &[(&[u16], i32, MoveEffect)] = &[
    // Fire moves that may burn.
    (&[52, 53, 157, 230, 262, 292, 519], 10, MoveEffect::Burn),
    // Electric and body blows that may paralyse: the light ones, the
    // heavy ones, and those that always do.
    (&[120, 144, 235, 342], 10, MoveEffect::Paralyze),
    (&[64, 65, 507, 521], 30, MoveEffect::Paralyze),
    (&[193], 100, MoveEffect::Paralyze),
    // Ice moves that may freeze.
    (&[101, 270, 344, 345], 10, MoveEffect::Freeze),
    // Poison moves: the common ones, the nastier ones.
    (&[200, 279, 280], 20, MoveEffect::Poison),
    (&[147, 459, 485, 495], 30, MoveEffect::Poison),
    // Confusing blows.
    (&[107, 114, 234, 446, 541], 10, MoveEffect::Confuse),
    (&[264, 310], 20, MoveEffect::Confuse),
    (&[246], 100, MoveEffect::Confuse),
    // Blows that may make cringe, from the light to the fearsome.
    (&[133, 261, 289], 10, MoveEffect::Cringe),
    (&[63, 221, 271, 303], 20, MoveEffect::Cringe),
    (&[30, 46, 431, 436, 442, 473, 498], 30, MoveEffect::Cringe),
    // Muddy and murky blows that cloud the sight.
    (&[159, 501, 527], 30, MoveEffect::TargetAccuracy(-1)),
    (&[213], 100, MoveEffect::TargetAccuracy(-1)),
    // Moves that always slow (Rock Tomb, Icy Wind, Mud Shot), and the
    // bubbles that may.
    (&[73, 97, 299], 100, MoveEffect::TargetSpeedDown),
    (&[71, 255], 10, MoveEffect::TargetSpeedDown),
    // The chances of the three groups below come with the extracted
    // data (see `data_chance`).
    // Mist Ball.
    (&[309], 0, MoveEffect::TargetSpAttack(-1)),
    // Luster Purge, Energy Ball, Focus Blast.
    (&[335, 443, 454], 0, MoveEffect::TargetSpDefense(-1)),
    // Whirlpool, Clamp, Bind, Sand Tomb, Fire Spin, Magma Storm.
    (&[32, 69, 125, 143, 291, 524], 0, MoveEffect::Constrict),
];

/// The chance the extracted data sets for a move of [`EXTRA_EFFECTS`],
/// where it has one.
fn data_chance(move_id: u16, constants: &Constants) -> Option<i32> {
    match move_id {
        309 => Some(constants.mist_ball_chance),
        335 | 443 | 454 => Some(constants.luster_purge_chance),
        32 | 69 | 125 | 143 | 291 | 524 => Some(constants.constriction_chance),
        _ => None,
    }
}

/// What a damaging move may also do to its target, with the chance in
/// percent.
pub fn secondary_effect(move_id: u16, constants: &Constants) -> Option<(i32, MoveEffect)> {
    let &(_, chance, effect) = EXTRA_EFFECTS.iter().find(|(moves, _, _)| moves.contains(&move_id))?;
    Some((data_chance(move_id, constants).unwrap_or(chance), effect))
}

/// Damage of the moves that ignore stats and types, or `None` for the
/// others. 0 means the move fails.
pub fn fixed_damage(slot: &MoveSlot, rng: &mut DungeonRng, constants: &Constants, chart: Option<&TypeChart>, user: &Stats, target: &Stats) -> Option<i32> {
    const SUPER_FANG: u16 = 20;
    const PSYWAVE: u16 = 108;
    const SONICBOOM: u16 = 152;
    const DRAGON_RAGE: u16 = 341;
    const ENDEAVOR: u16 = 66;
    const RETURN: u16 = 48;
    const FRUSTRATION: u16 = 328;
    const MAGNITUDE: u16 = 296;
    const VACUUM_CUT: u16 = 394;
    const PRESENT: u16 = 277;
    Some(match slot.id {
        131 | 150 | 177 | 247 => one_hit_ko(slot, chart, user, target),
        SUPER_FANG => (target.hp / 2).max(1),
        // From half to one and a half times the user's level.
        PSYWAVE => (user.level * rng.rand_range(50, 150) / 100).clamp(1, 199),
        SONICBOOM => constants.sonicboom_damage,
        DRAGON_RAGE => constants.dragon_rage_damage,
        // Seismic Toss, Night Shade.
        170 | 216 => user.level,
        // The target down to the user's HP; nothing to take, it fails.
        ENDEAVOR => (target.hp - user.hp).max(0),
        RETURN => by_iq(&constants.return_damage, user.iq),
        FRUSTRATION => by_iq(&constants.frustration_damage, user.iq),
        // One of its strengths at random (twice as much on a monster
        // underground: see the caller).
        MAGNITUDE => constants.magnitude_damage[rng.rand_int(constants.magnitude_damage.len() as i32) as usize],
        VACUUM_CUT => constants.vacuum_cut_damage,
        PRESENT => present(rng, target),
        _ => return None,
    })
}

/// Fissure, Sheer Cold, Horn Drill, Guillotine: the target is knocked
/// out at once, unless it is of a higher level than the user, or immune to
/// the move's type (Fissure, a Ground move, does not reach a levitating
/// monster either).
fn one_hit_ko(slot: &MoveSlot, chart: Option<&TypeChart>, user: &Stats, target: &Stats) -> i32 {
    const GROUND: u8 = 9;
    let immune = chart.is_some_and(|chart| chart.immune(slot.type_id, target));
    let floats = slot.type_id == GROUND && target.abilities.contains(&crate::dungeon::ability::LEVITATE);
    if immune || floats || target.level > user.level { 0 } else { OHKO_DAMAGE }
}

/// Return's and Frustration's damage: the first row of the table for a
/// user whose IQ is below its bound (rows from a negative bound on are
/// unused); 1 past them all.
fn by_iq(table: &[(i32, i32)], iq: i32) -> i32 {
    let rows = table.iter().take_while(|(bound, _)| *bound >= 0);
    rows.clone().find(|(bound, _)| iq < *bound).map_or(1, |(_, damage)| *damage)
}

/// Present: a little gift of damage most of the time, a big one rarely,
/// and now and then HP for the target instead (a negative damage).
fn present(rng: &mut DungeonRng, target: &Stats) -> i32 {
    match rng.rand_int(10) {
        0 | 1 => -(target.max_hp.min(999) / 4).max(1),
        2..=5 => 20,
        6..=8 => 40,
        _ => 80,
    }
}

/// Which quarter of its HP a monster is in: 0 at a quarter or less, 3
/// above three quarters.
pub(super) fn hp_quarter(stats: &Stats) -> usize {
    let most = stats.max_hp.clamp(1, 999);
    ((stats.hp.max(1) * 4 - 1) / most).clamp(0, 3) as usize
}

/// What a move's damage is multiplied by for the state of its user and
/// target (8 fraction bits), its user's ability boost included.
/// `slowed`: the user's speed is down.
pub(crate) fn power_multiplier(move_id: u16, pp: i32, max_pp: i32, constants: &Constants, user: (&Stats, &Statuses, bool), target: (&Stats, &Statuses)) -> i32 {
    let (user, user_status, slowed) = user;
    let (target, target_status) = target;
    let boost = crate::dungeon::ability::power_boost(user, move_id);
    boost * by_state(move_id, (pp, max_pp), constants, (user, user_status, slowed), (target, target_status)) / ONE
}

const ONE: i32 = 0x100;

fn by_state(move_id: u16, (pp, max_pp): (i32, i32), constants: &Constants, user: (&Stats, &Statuses, bool), target: (&Stats, &Statuses)) -> i32 {
    let (user, user_status, slowed) = user;
    let (target, target_status) = target;
    let lasting = user_status.burn_turns > 0 || user_status.poisoned || user_status.paralysis_turns > 0;
    match move_id {
        // Facade: stronger when its user suffers from a lasting problem.
        67 if lasting => constants.facade_mult,
        // Brine, Assurance: twice as hard on a target at half its HP.
        469 | 488 if target.hp * 2 <= target.max_hp.min(999) => 2 * ONE,
        // Gyro Ball: twice as hard from a slowed user.
        475 if slowed => 2 * ONE,
        457 => trump_card(pp, max_pp),
        // Wake-Up Slap: twice as hard on a sleeper (it wakes: see the caller).
        531 if target_status.asleep() => 2 * ONE,
        // Low Kick, Grass Knot: by the target's weight.
        92 | 458 => target_status.weight,
        // Hi Jump Kick, Blast Burn.
        206 | 272 => 2 * ONE,
        // Spit Up: by what was stockpiled (none: it fails, see the caller).
        245 => user_status.stockpile * ONE,
        // Flail, Reversal; Water Spout; Eruption: by the user's HP. Wring
        // Out, Crush Grip: by the target's.
        77 | 121 => constants.reversal_mult[hp_quarter(user)],
        113 => constants.water_spout_mult[hp_quarter(user)],
        278 => constants.eruption_mult[hp_quarter(user)],
        474 | 505 => constants.wring_out_mult[hp_quarter(target)],
        _ => ONE,
    }
}

/// Trump Card: the fewer PP it has left, the harder it hits.
fn trump_card(pp: i32, max_pp: i32) -> i32 {
    match pp * 10 / max_pp.max(1) {
        ..=2 => 0x180,
        3..=5 => 0x120,
        _ => ONE,
    }
}

/// What the `strike`th hit (from 0) of a move striking several times is
/// multiplied by: Triple Kick harder each time, Rollout and Ice Ball
/// growing by their table.
pub(crate) fn strike_multiplier(move_id: u16, strike: usize, constants: &Constants) -> i32 {
    const TRIPLE_KICK: u16 = 210;
    match move_id {
        TRIPLE_KICK => (strike as i32 + 1) * ONE,
        2 | 105 => {
            let table = &constants.rollout_mult;
            table[strike.min(table.len() - 1)]
        }
        _ => ONE,
    }
}

/// The moves that leave their user out of breath: Hyper Beam, Frenzy
/// Plant, Hydro Cannon, Blast Burn, Rock Wrecker, Giga Impact, Roar of
/// Time.
pub(crate) const HYPER_BEAMS: [u16; 7] = [238, 239, 242, 272, 453, 455, 494];

/// What Secret Power may do, by the kind of ground the floor has (a number
/// from the floor's look): poison, sleep, slow, lower the attack, the
/// defense, the accuracy, confuse, make cringe, freeze, else paralyse.
pub(crate) fn secret_power_effect(kind: u8) -> MoveEffect {
    use MoveEffect::*;
    const BY_GROUND: [MoveEffect; 9] = [Poison, Sleep, TargetSpeedDown, TargetAttack(-1), TargetDefense(-1), TargetAccuracy(-1), Confuse, Cringe, Freeze];
    BY_GROUND.get(kind as usize).copied().unwrap_or(Paralyze)
}

pub fn is_fixed_damage(move_id: u16) -> bool {
    matches!(move_id, 20 | 48 | 66 | 108 | 131 | 150 | 152 | 170 | 177 | 216 | 247 | 277 | 296 | 328 | 341 | 394)
}

/// How many times a move strikes: its own count, or for a count of 0 two
/// to five times, the short runs likelier than the long.
pub fn strike_count(rng: &mut DungeonRng, strikes: i32) -> i32 {
    if strikes != 0 {
        return strikes;
    }
    match rng.rand_int(20) {
        0..=6 => 2,
        7..=13 => 3,
        14..=16 => 4,
        _ => 5,
    }
}

/// What a damaging move may do to its user once it has dealt damage, with
/// the chance in percent.
pub fn user_effect(move_id: u16, constants: &Constants) -> Option<(i32, MoveEffect)> {
    use MoveEffect::*;
    Some(match move_id {
        103 => (constants.meteor_mash_chance, UserAttack(1)),
        325 => (constants.metal_claw_chance, UserAttack(1)),
        244 => (constants.steel_wing_chance, UserDefense(1)),
        // Silver Wind, AncientPower, Ominous Wind.
        82 | 93 | 437 => (constants.silver_wind_chance, BoostAll),
        243 => (100, Superpower),
        // Overheat, Psycho Boost, Leaf Storm, Draco Meteor.
        41 | 110 | 537 | 538 => (100, UserSpAttack(-2)),
        489 => (constants.charge_beam_chance, UserSpAttack(1)),
        // Close Combat; Hammer Arm.
        440 => (100, CloseCombat),
        430 => (100, UserSpeedDown),
        // Petal Dance leaves its user dizzy.
        253 => (100, Confuse),
        _ => return None,
    })
}

/// What a damaging move does to its user once it has dealt damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aftermath {
    /// The user is hurt by a third of the damage dealt.
    Recoil,
    /// The user recovers half of the damage dealt.
    Drain,
}

pub fn aftermath(move_id: u16) -> Option<Aftermath> {
    // Submission, Take Down, Volt Tackle, Wood Hammer, Brave Bird,
    // Double-Edge, Flare Blitz, Head Smash.
    const RECOIL: [u16; 8] = [116, 203, 354, 441, 518, 140, 517, 533];
    // Giga Drain, Leech Life, Mega Drain, Drain Punch, Absorb, Dream Eater.
    const DRAIN: [u16; 6] = [76, 80, 320, 500, 135, 331];
    if RECOIL.contains(&move_id) {
        Some(Aftermath::Recoil)
    } else if DRAIN.contains(&move_id) {
        Some(Aftermath::Drain)
    } else {
        None
    }
}

impl Aftermath {
    /// Applies it to the user; returns the HP lost (negative when healed).
    pub fn apply(self, user: &mut Stats, damage_dealt: i32) -> i32 {
        let before = user.hp;
        match self {
            Aftermath::Recoil => user.hp -= (damage_dealt / 3).max(1),
            Aftermath::Drain => user.hp = (user.hp + (damage_dealt / 2).max(1)).min(user.max_hp),
        }
        before - user.hp
    }
}
