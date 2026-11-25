//! Items used and eaten.

use super::*;

pub mod item {
    /// Thrown items come first: sticks, thorns and spikes (damage from
    /// their power) and rocks (fixed damage).
    pub(crate) const LAST_PROJECTILE: u16 = 10;
    pub const ROCKS: [u16; 3] = [7, 8, 10];
    /// Money: it goes to the purse, not the bag.
    pub const POKE: u16 = 183;
    pub(crate) const HEAL_SEED: u16 = 69;
    pub(crate) const ORAN_BERRY: u16 = 70;
    pub(crate) const SITRUS_BERRY: u16 = 71;
    pub(crate) const REVIVER_SEED: u16 = 73;
    pub(crate) const LIFE_SEED: u16 = 77;
    pub(crate) const RAWST_BERRY: u16 = 78;
    pub(crate) const HUNGER_SEED: u16 = 79;
    pub(crate) const QUICK_SEED: u16 = 80;
    pub(crate) const PECHA_BERRY: u16 = 81;
    pub(crate) const CHERI_BERRY: u16 = 82;
    pub(crate) const TOTTER_SEED: u16 = 83;
    pub(crate) const SLEEP_SEED: u16 = 84;
    pub(crate) const PLAIN_SEED: u16 = 85;
    pub(crate) const BLAST_SEED: u16 = 87;
    pub(crate) const WARP_SEED: u16 = 86;
    pub(crate) const JOY_SEED: u16 = 89;
    pub(crate) const CHESTO_BERRY: u16 = 90;
    pub(crate) const GOLDEN_SEED: u16 = 93;
    pub(crate) const VILE_SEED: u16 = 94;
    pub(crate) const VIOLENT_SEED: u16 = 96;
    pub(crate) const MAX_ELIXIR: u16 = 99;
    pub(crate) const EYEDROP_SEED: u16 = 72;
    pub(crate) const BLINKER_SEED: u16 = 74;
    pub(crate) const DOOM_SEED: u16 = 75;
    pub(crate) const GINSENG: u16 = 88;
    pub(crate) const STUN_SEED: u16 = 91;
    pub(crate) const PURE_SEED: u16 = 95;
    pub(crate) const VANISH_SEED: u16 = 97;
    pub(crate) const NECTAR: u16 = 103;
    /// The lookalikes of other seeds, drinks and food.
    pub(crate) const DROPEYE_SEED: u16 = 104;
    pub(crate) const REVISER_SEED: u16 = 105;
    pub(crate) const SLIP_SEED: u16 = 106;
    pub(crate) const VIA_SEED: u16 = 107;
    pub(crate) const MIX_ELIXIR: u16 = 116;
    pub(crate) const DOUGH_SEED: u16 = 118;
    pub(crate) const GRAVELYROCK: u16 = 137;
    pub(crate) const GONE_PEBBLE: u16 = 167;
    pub(crate) const WANDER_GUMMI: u16 = 168;
    /// Opens the locked doors of a dungeon's rooms laid out by hand.
    pub(crate) const KEY: u16 = 182;
    pub(crate) const X_EYE_SEED: u16 = 76;
    pub(crate) const PROTEIN: u16 = 100;
    pub(crate) const CALCIUM: u16 = 101;
    pub(crate) const IRON: u16 = 102;
    pub(crate) const ZINC: u16 = 108;
    pub(crate) const OREN_BERRY: u16 = 117;
    pub(crate) const APPLE: u16 = 109;
    pub(crate) const BIG_APPLE: u16 = 110;
    pub(crate) const GRIMY_FOOD: u16 = 111;
    pub(crate) const HUGE_APPLE: u16 = 112;
    pub(crate) const GOLDEN_APPLE: u16 = 115;
    pub(crate) const WHITE_GUMMI: u16 = 119;
    /// The TMs, the last of them HMs, which are not used up.
    pub(crate) const FIRST_TM: u16 = 188;
    pub(crate) const FIRST_HM: u16 = 285;
    pub(crate) const LAST_TM: u16 = 292;
    pub(crate) const SILVER_GUMMI: u16 = 135;
    pub const WONDER_GUMMI: u16 = 136;
}

/// The most a stat, the max HP and the IQ can grow to.
const MAX_STAT: i32 = 255;
const MAX_HP: i32 = 999;
const MAX_IQ: i32 = 999;

/// HP back, Wise Healer adding its share, up to the max.
fn heal(stats: &mut Stats, constants: &Constants, amount: i32) {
    let wise = crate::dungeon::iq::has(stats, crate::dungeon::iq::WISE_HEALER);
    let extra = if wise { amount * constants.wise_healer_percent / 100 } else { 0 };
    stats.hp = (stats.hp + amount + extra).min(stats.max_hp);
}

/// More max HP, and the HP that go with it.
fn grow(stats: &mut Stats, by: i32) {
    stats.max_hp = (stats.max_hp + by).min(MAX_HP);
    stats.hp = (stats.hp + by).min(stats.max_hp);
}

/// Eats or drinks an item. Returns false, leaving everything untouched, for
/// items without an effect yet. (The Reviver Seed works on its own, when
/// its owner faints.)
pub(crate) fn use_item(item: u16, constants: &Constants, stats: &mut Stats, statuses: &mut Statuses, belly: &mut Belly, moves: &mut [MoveSlot]) -> bool {
    match item {
        item::ORAN_BERRY => heal(stats, constants, constants.oran_berry_hp),
        item::SITRUS_BERRY => heal(stats, constants, constants.sitrus_berry_hp),
        item::PECHA_BERRY => (statuses.poisoned, statuses.badly_poisoned) = (false, false),
        item::HEAL_SEED => statuses.cure_all(),
        // Apples: belly back, and on a belly already full a bigger belly.
        item::APPLE => belly.eat_food(50, 3),
        item::BIG_APPLE => belly.eat_food(100, 6),
        item::MAX_ELIXIR => moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp),
        _ => return false,
    }
    true
}

/// What eating an item did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemUse {
    /// No effect yet in this engine: the item is kept.
    Nothing,
    Done,
    /// The eater is sent elsewhere on the floor.
    Warp,
    /// The eater gains this many levels.
    Levels(i32),
    /// The eater loses a level (a Doom Seed).
    LoseLevel,
    /// The eater is sent to the stairs (a Pure Seed).
    ToStairs,
    /// The floor's traps are shown (an Eyedrop Seed).
    SeeTraps,
    /// More Poké lie on the team's next floor (a Dough Seed).
    MorePoke,
    /// It works for some species only ([`fussy_eaters`]), the caller
    /// knowing who ate it: a Mix Elixir, a Gravelyrock.
    Fussy(u16),
}

/// The species (by their first number) an item of [`ItemUse::Fussy`]
/// works for: Linoone for the Mix Elixir, Sudowoodo and Bonsly for the
/// Gravelyrock.
pub fn fussy_eaters(item: u16) -> &'static [u16] {
    match item {
        item::MIX_ELIXIR => &[292],
        item::GRAVELYROCK => &[185, 480],
        _ => &[],
    }
}

/// Eats or drinks an item: `use_item`, and the seeds, berries and drinks
/// whose effect is one this engine already has.
pub fn eat_item(
    item: u16,
    rng: &mut DungeonRng,
    constants: &Constants,
    stats: &mut Stats,
    statuses: &mut Statuses,
    belly: &mut Belly,
    moves: &mut [MoveSlot],
) -> ItemUse {
    // A berry, a seed or a vitamin is a mouthful for the belly too.
    const MOUTHFUL: i32 = 3;
    let small = constants.small_food.get(item as usize / 64).is_some_and(|bits| bits >> (item % 64) & 1 != 0);
    if small {
        belly.eat(MOUTHFUL);
    }
    if let Some(outcome) = eaten_elsewhere(item) {
        return outcome;
    }
    let full = stats.hp >= stats.max_hp;
    let affect = |effect, rng: &mut DungeonRng, stats: &mut Stats, statuses: &mut Statuses| {
        apply_effect(effect, rng, constants, stats, statuses);
    };
    let [protein, calcium, iron, zinc] = constants.vitamin_boosts;
    let raise = |stat: &mut i32, by: i32| *stat = (*stat + by).min(MAX_STAT);
    match item {
        // Berries eaten with nothing to heal make the eater sturdier.
        item::ORAN_BERRY if full => grow(stats, constants.oran_berry_full_hp_boost),
        item::SITRUS_BERRY if full => grow(stats, constants.sitrus_berry_full_hp_boost),
        item::LIFE_SEED => grow(stats, constants.life_seed_hp_boost),
        item::RAWST_BERRY => statuses.burn_turns = 0,
        item::CHERI_BERRY => statuses.paralysis_turns = 0,
        item::CHESTO_BERRY => statuses.sleep_turns = 0,
        item::HUNGER_SEED => belly.empty(),
        item::PLAIN_SEED => {}
        item::QUICK_SEED => affect(MoveEffect::UserSpeedUp, rng, stats, statuses),
        item::TOTTER_SEED => affect(MoveEffect::Confuse, rng, stats, statuses),
        item::SLEEP_SEED => affect(MoveEffect::Sleep, rng, stats, statuses),
        // (Blinker, from a Blinker Seed or a Wander Gummi's gunk, is an
        // accuracy drop here.)
        item::BLINKER_SEED | item::WANDER_GUMMI => affect(MoveEffect::TargetAccuracy(-2), rng, stats, statuses),
        // Dropeye: dark around it, even in rooms; Slip: it walks on water.
        // (Both until the floor is left.)
        item::DROPEYE_SEED => statuses.dropeye = true,
        item::SLIP_SEED => statuses.slipping = true,
        // A Reviser Seed revives nobody: eaten, it is a Plain Seed.
        item::REVISER_SEED => {}
        // A Gone Pebble brings tears: Enduring.
        item::GONE_PEBBLE => affect(MoveEffect::Endure, rng, stats, statuses),
        item::VANISH_SEED => affect(MoveEffect::Invisify, rng, stats, statuses),
        item::VIOLENT_SEED => {
            let top = STAGE_COUNT as i32 - 1;
            (statuses.attack_stage, statuses.sp_attack_stage) = (top, top);
        }
        item::VILE_SEED => (statuses.defense_stage, statuses.sp_defense_stage) = (0, 0),
        item::PROTEIN => raise(&mut stats.attack, protein),
        item::CALCIUM => raise(&mut stats.sp_attack, calcium),
        item::IRON => raise(&mut stats.defense, iron),
        item::ZINC => raise(&mut stats.sp_defense, zinc),
        item::OREN_BERRY => stats.hp -= constants.oren_berry_damage,
        item::HUGE_APPLE => belly.eat_food(MAX_HP, 10),
        item::GOLDEN_APPLE => belly.eat_food(MAX_HP, 30),
        item::GRIMY_FOOD => grimy_food(rng, constants, stats, statuses, belly),
        item::WONDER_GUMMI | item::WHITE_GUMMI..=item::SILVER_GUMMI => gummi(item, rng, constants, stats, belly),
        // (This project's own figures: the descriptions only say "IQ up"
        // and "a move boosted".)
        item::NECTAR => stats.iq = (stats.iq + 5).min(MAX_IQ),
        item::GINSENG => {
            if let Some(first) = moves.first_mut() {
                first.power += 2;
            }
        }
        item::X_EYE_SEED => statuses.cross_eyed_turns = Statuses::duration(rng, constants.cross_eyed_turns),
        // Turned to stone: it stands still until struck.
        item::STUN_SEED => statuses.sleep_turns = DEEP_SLEEP,
        _ if use_item(item, constants, stats, statuses, belly, moves) => {}
        _ => return ItemUse::Nothing,
    }
    ItemUse::Done
}

/// The items whose whole effect is the caller's.
fn eaten_elsewhere(item: u16) -> Option<ItemUse> {
    Some(match item {
        // (A Via Seed: "See ya!")
        item::WARP_SEED | item::VIA_SEED => ItemUse::Warp,
        item::DOUGH_SEED => ItemUse::MorePoke,
        item::MIX_ELIXIR | item::GRAVELYROCK => ItemUse::Fussy(item),
        item::DOOM_SEED => ItemUse::LoseLevel,
        item::PURE_SEED => ItemUse::ToStairs,
        item::EYEDROP_SEED => ItemUse::SeeTraps,
        item::JOY_SEED => ItemUse::Levels(1),
        item::GOLDEN_SEED => ItemUse::Levels(5),
        _ => return None,
    })
}

/// Grimy Food: it fills a little, and may upset the eater's stomach (not
/// a Survivalist's nor a Glutton's): poisoned, paralysed, burned, or
/// weakened.
fn grimy_food(rng: &mut DungeonRng, constants: &Constants, stats: &mut Stats, statuses: &mut Statuses, belly: &mut Belly) {
    const SURVIVALIST: u8 = 37;
    const GLUTTONY: u8 = 89;
    const ILLS: [MoveEffect; 4] = [MoveEffect::Poison, MoveEffect::Paralyze, MoveEffect::Burn, MoveEffect::TargetAttack(-2)];
    belly.eat(25);
    let spared = crate::dungeon::iq::has(stats, SURVIVALIST) || crate::dungeon::ability::has(stats, GLUTTONY);
    if !spared && rng.rand_int(100) < constants.grimy_food_chance {
        let ill = ILLS[rng.rand_int(ILLS.len() as i32) as usize];
        apply_effect(ill, rng, constants, stats, statuses);
    }
}

/// A Gummi: IQ and belly, the more as its colour suits one of the eater's
/// types (a Wonder Gummi suits everyone, but does not fill). One it really
/// likes may also make it stronger: a stat at random, or once in a while
/// all four.
fn gummi(item: u16, rng: &mut DungeonRng, constants: &Constants, stats: &mut Stats, belly: &mut Belly) {
    let (iq, food) = match item {
        item::WONDER_GUMMI => (constants.wonder_gummi_iq, 0),
        _ => {
            let colour = (item - item::WHITE_GUMMI + 1) as usize;
            let liking = |table: &[[i32; 18]; 18]| {
                let of = |kind: u8| table.get(kind as usize).map_or(0, |row| row[colour]);
                of(stats.types.0).max(of(stats.types.1))
            };
            (liking(&constants.gummi_iq), liking(&constants.gummi_belly))
        }
    };
    stats.iq = (stats.iq + iq).min(MAX_IQ);
    belly.eat(food);
    const LOVED: i32 = 3;
    if iq < LOVED || rng.rand_int(100) >= constants.gummi_boost_chance {
        return;
    }
    let all = rng.rand_int(constants.gummi_all_stats_odds.max(1)) == 0;
    let one = rng.rand_int(4) as usize;
    let by = constants.gummi_stat_boost;
    for (at, stat) in [&mut stats.attack, &mut stats.sp_attack, &mut stats.defense, &mut stats.sp_defense].into_iter().enumerate() {
        if all || at == one {
            *stat = (*stat + by).min(MAX_STAT);
        }
    }
}
