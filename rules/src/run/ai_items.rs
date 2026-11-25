//! What a monster's AI does with the item it holds, with Item Master on:
//! eats or uses it when that does it good, throws it at a foe in line or
//! at a teammate it helps, as the game's `AiDecideUseItem` and
//! `GetAiUseItemProbability` do. (The game's leader also goes through the
//! bag; here the leader is the player's.)

use crate::dungeon::status::item;
use crate::dungeon::{DungeonRng, Floor, Pos, ai, iq, stack};

use super::attack::chebyshev;
use super::{Dungeon, Monster, MonsterId, Side};

/// The items' AI flags, from the ROM's `item_p.bin` (byte 14, bits 7, 6,
/// 5): eaten or used by their holder, thrown at a teammate, thrown at a
/// foe. (Rollcall Orb, the one orb used, is left to the leader's orbs.)
const USED: [u16; 27] = [69, 70, 71, 72, 77, 78, 80, 81, 82, 89, 90, 92, 96, 97, 99, 100, 101, 102, 106, 108, 109, 110, 112, 116, 118, 137, 167];
const AT_TEAMMATES: [u16; 50] = [
    69, 70, 71, 72, 77, 78, 80, 81, 82, 88, 89, 90, 92, 93, 96, 97, 99, 100, 101, 102, 103, 106, 108, 109, 110, 112, 115, 116, 118, 119, 120, 121, 122, 123,
    124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 167, 179,
];
const AT_FOES: [u16; 29] = [1, 2, 3, 4, 5, 6, 7, 8, 14, 15, 20, 34, 47, 48, 74, 75, 76, 79, 83, 84, 86, 87, 91, 94, 104, 107, 111, 117, 168];

/// How far the AI throws, as its ranged moves reach (the game's
/// `RANGED_ATTACK_RANGE`).
const THROW_REACH: i32 = 10;

/// A stat past which a vitamin is not worth it.
const STAT_TOP: i32 = 250;

/// What the AI does with its held item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AiItem {
    Eat,
    /// Thrown that way; a rock at that tile.
    Throw((i32, i32), Option<Pos>),
}

/// Who an item's weight is for: its holder (`other` false), or the one it
/// is thrown at, a teammate of the thrower or not.
#[derive(Clone, Copy)]
struct For {
    other: bool,
    teammate: bool,
    beside_foe: bool,
}

/// Whether two monsters are on one side (a shopkeeper is on none).
fn same_side(a: &Monster, b: &Monster) -> bool {
    a.team() == b.team()
}

fn fights(a: &Monster, b: &Monster) -> bool {
    !same_side(a, b) && a.side != Side::Keeper && b.side != Side::Keeper
}

/// A foe of it right next to it.
fn beside_foe(dungeon: &Dungeon, monster: &Monster) -> bool {
    dungeon.monsters.iter().any(|other| other.alive() && chebyshev(other.at, monster.at) == 1 && fights(monster, other))
}

/// The chance in 100 the AI uses the item on `on`: the game's
/// `GetAiUseItemProbability`, item by item, with this engine's statuses
/// (no blinding nor enduring here: never blinded, never enduring).
fn weight(item: u16, on: &Monster, how: For) -> i32 {
    let For { other, teammate, beside_foe } = how;
    let (stats, status) = (&on.stats, &on.status);
    let near = |close: i32, far: i32| if beside_foe { close } else { far };
    let belly = on.belly.map_or(100, |belly| belly.points());
    let species = on.species.unwrap_or(0);
    match item {
        1..=10 if other => 70,
        34 if other && belly > 0 => 50,
        47 | 48 if other => 50,
        20 => 40,
        item::ORAN_BERRY | item::SITRUS_BERRY if stats.hp < stats.max_hp && stats.hp <= stats.max_hp / 4 => match other {
            false => near(100, 50),
            true => 50,
        },
        item::MAX_ELIXIR | item::MIX_ELIXIR => {
            const LINOONE: u16 = 292;
            if item == item::MIX_ELIXIR && species % 600 != LINOONE {
                return 0;
            }
            let wanting = on.moves.iter().map(|slot| if slot.pp == 0 { 30 } else { 0 } + if slot.pp != slot.max_pp { 6 } else { 0 });
            wanting.sum::<i32>().min(99)
        }
        item::HEAL_SEED | 92 if status.ailing() => 80,
        item::PROTEIN if stats.attack < STAT_TOP => 100,
        item::CALCIUM if stats.sp_attack < STAT_TOP => 100,
        item::IRON if stats.defense < STAT_TOP => 100,
        item::ZINC if stats.sp_defense < STAT_TOP => 100,
        item::LIFE_SEED if !other => near(10, 100),
        item::EYEDROP_SEED => near(80, 5),
        item::QUICK_SEED if status.speed_stage() < crate::dungeon::status::MAX_SPEED_STAGE => near(80, 5),
        item::X_EYE_SEED if status.cross_eyed_turns == 0 => near(80, 5),
        item::CHERI_BERRY if status.paralysis_turns > 0 => near(80, 30),
        item::TOTTER_SEED if status.confused_turns == 0 => near(80, 15),
        item::PECHA_BERRY if status.poisoned => near(100, 50),
        item::BLINKER_SEED => near(80, 5),
        item::WARP_SEED if !teammate => near(40, 5),
        item::WARP_SEED if stats.hp < stats.max_hp && stats.hp < 20 => match other {
            false => near(100, 50),
            true => 50,
        },
        item::SLEEP_SEED | item::STUN_SEED if !status.asleep() => near(80, 5),
        item::CHESTO_BERRY if status.sleepless_turns == 0 => 5,
        item::JOY_SEED if stats.level < 99 => 80,
        item::GINSENG => 80,
        item::RAWST_BERRY if status.burn_turns > 0 => 50,
        item::HUNGER_SEED if belly > 0 => 50,
        item::DOOM_SEED if stats.level > 1 => 80,
        item::BLAST_SEED => near(80, 30),
        item::DOUGH_SEED if on.team() => 100,
        // (Those that walk, or cross lava, and do not walk on water yet.)
        item::SLIP_SEED if matches!(status.movement, 0 | 4) && !status.slipping => near(10, 100),
        item::GONE_PEBBLE => near(80, 0),
        item::VILE_SEED if other => 70,
        item::VIOLENT_SEED if status.sp_attack_stage < 20 => 80,
        item::VANISH_SEED if status.invisible_turns == 0 => 80,
        // (The game's own bug: the female forms are not looked at.)
        item::GRAVELYROCK if crate::dungeon::status::fussy_eaters(item).contains(&species) => match other {
            true => 70,
            false => 100,
        },
        item::APPLE | item::BIG_APPLE | item::HUGE_APPLE if belly < 10 => 100,
        item::GRIMY_FOOD => 30,
        14 | 15 | item::VIA_SEED | item::OREN_BERRY | item::WANDER_GUMMI if other => 80,
        item::DROPEYE_SEED if other && !status.dropeye => 80,
        _ => 0,
    }
}

/// Whether the AI's roll of that chance in 100 comes up.
fn rolls(rng: &mut DungeonRng, chance: i32) -> bool {
    chance > 0 && rng.rand_int(100) < chance
}

/// The first one in line from `from` that way, within reach, short of a
/// wall.
fn first_in_line(floor: &Floor, dungeon: &Dungeon, from: Pos, (dx, dy): (i32, i32)) -> Option<MonsterId> {
    let mut at = from;
    for _ in 0..THROW_REACH {
        if !floor.can_step(at, dx, dy) {
            return None;
        }
        at = Pos::new(at.x + dx, at.y + dy);
        if let Some(there) = dungeon.monsters.iter().find(|monster| monster.alive() && monster.at == at) {
            return Some(there.id);
        }
    }
    None
}

/// What the monster does with the item it holds this turn, if anything
/// (Item Master on, nothing stopping it): eaten if it does it good, else
/// thrown at a teammate it helps or a foe in line, by the weights.
pub(crate) fn choose(dungeon: &mut Dungeon, who: MonsterId) -> Option<AiItem> {
    let me = dungeon.monster(who)?.clone();
    let held = me.stats.held;
    if held == 0 || stack::sticky(held) || !iq::has(&me.stats, iq::ITEM_MASTER) || me.status.embargo_turns > 0 || me.status.cannot_act() {
        return None;
    }
    let item = stack::id(held);
    let mine = For { other: false, teammate: true, beside_foe: beside_foe(dungeon, &me) };
    if USED.contains(&item) {
        let chance = weight(item, &me, mine);
        if rolls(&mut dungeon.rng, chance) {
            return Some(AiItem::Eat);
        }
    }
    for (thrown_at, teammates) in [(&AT_TEAMMATES[..], true), (&AT_FOES[..], false)] {
        if !thrown_at.contains(&item) {
            continue;
        }
        let seen = |other: &&Monster| {
            other.alive()
                && other.id != who
                && other.side != Side::Keeper
                && same_side(&me, other) == teammates
                && ai::can_see(&dungeon.floor, me.at, other.at)
                && (teammates || other.status.invisible_turns == 0)
        };
        let candidates: Vec<(MonsterId, Pos, i32)> = dungeon
            .monsters
            .iter()
            .filter(seen)
            .map(|target| (target.id, target.at, weight(item, target, For { other: true, teammate: teammates, beside_foe: beside_foe(dungeon, target) })))
            .collect();
        // Rocks: any foe in sight within reach the roll picks, one of
        // those at random.
        if item::ROCKS.contains(&item) {
            let near = candidates.iter().filter(|&&(_, at, _)| chebyshev(me.at, at) <= THROW_REACH);
            let picked: Vec<Pos> = near.filter(|&&(_, _, chance)| rolls(&mut dungeon.rng, chance)).map(|&(_, at, _)| at).collect();
            if picked.is_empty() {
                continue;
            }
            let at = picked[dungeon.rng.rand_int(picked.len() as i32) as usize];
            let toward = ((at.x - me.at.x).signum(), (at.y - me.at.y).signum());
            return Some(AiItem::Throw(toward, Some(at)));
        }
        // Others: one way for each that stands first in line, each way
        // rolled in turn.
        let mut ways: Vec<((i32, i32), i32)> = Vec::new();
        for &(id, at, chance) in &candidates {
            let (dx, dy) = (at.x - me.at.x, at.y - me.at.y);
            let lined = dx == 0 || dy == 0 || dx.abs() == dy.abs();
            let way = (dx.signum(), dy.signum());
            if lined && !ways.iter().any(|&(taken, _)| taken == way) && first_in_line(&dungeon.floor, dungeon, me.at, way) == Some(id) {
                ways.push((way, chance));
            }
        }
        if let Some(&(way, _)) = ways.iter().find(|&&(_, chance)| rolls(&mut dungeon.rng, chance)) {
            return Some(AiItem::Throw(way, None));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dungeon::{MoveSlot, Stats, Statuses};

    fn monster(side: Side, hp: i32) -> Monster {
        let stats = Stats {
            level: 10,
            max_hp: 40,
            hp,
            attack: 9,
            defense: 9,
            sp_attack: 9,
            sp_defense: 9,
            types: (1, 0),
            abilities: [0, 0],
            held: 0,
            iq: 1,
            iq_group: 0,
            iq_skills: 0,
            female: false,
        };
        let moves = vec![MoveSlot { pp: 10, max_pp: 10, ..MoveSlot::fallback_regular_attack() }];
        Monster::new(MonsterId(1), side, "It".into(), None, stats, moves, Statuses::default(), Pos::new(5, 5))
    }

    #[test]
    fn the_weights_are_the_game_s() {
        let mine = |beside_foe| For { other: false, teammate: true, beside_foe };
        let thrown = For { other: true, teammate: false, beside_foe: false };
        // An Oran Berry once a quarter of the HP is left, more so with a foe
        // beside.
        assert_eq!(weight(item::ORAN_BERRY, &monster(Side::Foe, 30), mine(false)), 0);
        assert_eq!(weight(item::ORAN_BERRY, &monster(Side::Foe, 10), mine(false)), 50);
        assert_eq!(weight(item::ORAN_BERRY, &monster(Side::Foe, 10), mine(true)), 100);
        // Sticks are only thrown; a Sleep Seed not at who sleeps already.
        assert_eq!((weight(1, &monster(Side::Leader, 40), mine(false)), weight(1, &monster(Side::Leader, 40), thrown)), (0, 70));
        let mut asleep = monster(Side::Leader, 40);
        asleep.status.sleep_turns = 3;
        assert_eq!((weight(item::SLEEP_SEED, &asleep, thrown), weight(item::SLEEP_SEED, &monster(Side::Leader, 40), thrown)), (0, 5));
        // An apple when the belly is nearly empty.
        let mut hungry = monster(Side::Ally, 40);
        hungry.belly = Some(crate::dungeon::Belly::full());
        assert_eq!(weight(item::APPLE, &hungry, mine(false)), 0);
        hungry.belly.as_mut().unwrap().lose(95);
        assert_eq!(weight(item::APPLE, &hungry, mine(false)), 100);
    }

    #[test]
    fn the_rom_s_flags_are_kept_in_order() {
        assert!(USED.iter().all(|item| AT_TEAMMATES.contains(item)));
        assert!(AT_FOES.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
