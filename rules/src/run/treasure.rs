//! What a defeated foe drops where the dungeon lets it
//! (`Restrictions::treasure_boxes`), as the game's `TrySpawnEnemyItemDrop`:
//! an Unown its stone, else now and then a treasure box by its species'
//! weights, else Poké for a team with the exclusive item for it. Then what
//! Xatu finds in a box, in town.
//!
//! A box keeps what it holds the game's way only in part: the bits of its
//! value past the item's say which list of the floor's rank its item comes
//! from, or Nectar (Honey Gather); a box for an exclusive item gets one of
//! the game's at random as it is opened (the game's choice when the species
//! has none of its own).

use crate::dungeon::{DungeonRng, stack};
use crate::tables::Extras;

/// What Collector and the exclusive item for more treasure add, each, to
/// a species' weights where they are not 0 (nothing, an item, its first
/// exclusive item, its second); the game's, read from its overlay 10.
const BONUS: [i32; 4] = [0, 100, 30, 30];
/// Chances in 100, the game's (overlay 10): that a box's item is Nectar
/// with Honey Gather; that an Unown drops its stone; that Poké fall for the
/// exclusive item for more money.
const HONEY_CHANCE: i32 = 25;
const UNOWN_STONE_CHANCE: i32 = 33;
pub(crate) const MONEY_DROP_CHANCE: i32 = 10;
/// The Unown (A to ?) and their stones.
const UNOWN: std::ops::RangeInclusive<u16> = 201..=228;
const FIRST_STONE: u16 = 400;
/// The boxes: twelve kinds of three, the first holding an item, the
/// others an exclusive item (`town::item::FIRST_BOX`).
const BOX_KINDS: i32 = 12;
const FIRST_BOX: u16 = crate::town::item::FIRST_BOX;
/// What a box holds, in the bits of its value past the item's (the bag
/// keeps plain numbers): which rank list (rank `2 * code + 1`), or Nectar.
const CONTENT_SHIFT: u32 = 13;
const NECTAR_INSIDE: u16 = 7;
const NECTAR: u16 = crate::dungeon::status::item::NECTAR;
/// The species' exclusive items of the game, four by four from the first
/// (Bulba-Claw, Bulba-Fang...): the first two of each, those boxes give.
const FIRST_SPECIES_ITEM: u16 = 574;
const SPECIES_ITEM_GROUPS: usize = 110;

/// What a defeated foe drops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Drop {
    /// An item: an Unown's stone, a box (its value, what it holds in it).
    Item(u16),
    Money,
}

/// What brings it down, for what it drops.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Felled {
    /// Collector, and the exclusive item for more treasure: each counts.
    pub bonus: i32,
    pub honey_gather: bool,
    /// The exclusive item for more money.
    pub money: bool,
}

/// What a foe of that species drops on a floor of that job rank, if
/// anything.
pub(crate) fn drop_of(rng: &mut DungeonRng, extras: &Extras, species: u16, rank: u8, felled: Felled) -> Option<Drop> {
    if UNOWN.contains(&species) && rng.rand_int(100) < UNOWN_STONE_CHANCE {
        return Some(Drop::Item(FIRST_STONE + species - UNOWN.start()));
    }
    let weights = extras.drops.get(species as usize).map_or([0; 4], |(weights, _)| weights.map(i32::from));
    let weights: Vec<i32> = (0..4).map(|at| if weights[at] != 0 { weights[at] + felled.bonus * BONUS[at] } else { 0 }).collect();
    let total: i32 = weights.iter().sum();
    let roll = rng.rand_int(total.max(0));
    let tier = (0..4).find(|&at| roll < weights[..=at].iter().sum::<i32>()).unwrap_or(0);
    if weights[0] != 0 && tier > 0 {
        let code = match tier {
            1 if felled.honey_gather && rng.rand_int(100) < HONEY_CHANCE => NECTAR_INSIDE,
            1 => (rank.min(15) / 2).min(NECTAR_INSIDE as u8 - 1) as u16,
            _ => 0,
        };
        let kind = rng.rand_int(BOX_KINDS) as u16;
        return Some(Drop::Item((FIRST_BOX + kind * 3 + (tier as u16 - 1)) | (code << CONTENT_SHIFT)));
    }
    (felled.money && rng.rand_int(100) < MONEY_DROP_CHANCE).then_some(Drop::Money)
}

/// What a treasure box holds, opened (`None`: not a box, or nothing to
/// find). `items`: how many items the game has.
pub(crate) fn content(value: u16, extras: &Extras, items: usize, rng: &mut DungeonRng) -> Option<u16> {
    let (_, rank) = crate::town::treasure_box(stack::id(value))?;
    let code = value >> CONTENT_SHIFT;
    if rank == 1 {
        if code == NECTAR_INSIDE {
            return Some(NECTAR);
        }
        // (Rank `2 * code + 1`: its list is at `2 * code`.)
        let list = extras.rank_lists.get(2 * code as usize).or(extras.rank_lists.last())?;
        return list.pick(rng).filter(|&item| item != crate::dungeon::status::item::POKE && (item as usize) < items);
    }
    let group = rng.rand_int(SPECIES_ITEM_GROUPS as i32) as u16;
    Some(FIRST_SPECIES_ITEM + group * 4 + (rank - 2)).filter(|&item| (item as usize) < items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extras() -> Extras {
        let list = crate::dungeon::ItemTable { categories: vec![(0, 10000)], items: vec![(70, 0, 10000)] };
        let mut extras = Extras { drops: vec![([0; 4], [0; 2]); 230], rank_lists: vec![list; 15], ..Extras::default() };
        // (A foe that drops a box one time in ten, mostly with an item.)
        extras.drops[19] = ([900, 80, 10, 10], [0; 2]);
        extras
    }

    #[test]
    fn foes_drop_boxes_by_their_weights_and_unown_their_stones() {
        let (extras, mut rng) = (extras(), DungeonRng::new(7));
        let mut boxes = [0; 3];
        for _ in 0..10_000 {
            if let Some(Drop::Item(value)) = drop_of(&mut rng, &extras, 19, 5, Felled::default()) {
                let (_, rank) = crate::town::treasure_box(stack::id(value)).expect("a box");
                boxes[rank as usize - 1] += 1;
            }
        }
        // About 8 %, 1 % and 1 %.
        assert!((650..950).contains(&boxes[0]) && (50..160).contains(&boxes[1]) && (50..160).contains(&boxes[2]), "{boxes:?}");
        // Collector and the exclusive item: more boxes.
        let collector = Felled { bonus: 2, ..Felled::default() };
        let more = (0..10_000).filter(|_| drop_of(&mut rng, &extras, 19, 5, collector).is_some()).count();
        assert!(more > 1_200, "{more}");
        // Nothing from a species without weights; Poké with the item for it.
        assert_eq!(drop_of(&mut rng, &extras, 20, 5, Felled::default()), None);
        let money = (0..1_000).filter(|_| drop_of(&mut rng, &extras, 20, 5, Felled { money: true, ..Felled::default() }).is_some()).count();
        assert!((60..150).contains(&money), "{money}");
        // An Unown, its stone a third of the time: the Z-Stone for the 26th.
        let stones: Vec<Drop> = (0..300).filter_map(|_| drop_of(&mut rng, &extras, 226, 5, Felled::default())).collect();
        assert!((60..140).contains(&stones.len()) && stones.iter().all(|&drop| drop == Drop::Item(425)), "{}", stones.len());
    }

    #[test]
    fn a_box_holds_an_item_of_its_rank_nectar_or_an_exclusive_item() {
        let (extras, mut rng) = (extras(), DungeonRng::new(3));
        let honey = Felled { honey_gather: true, ..Felled::default() };
        let dropped: Vec<u16> = (0..20_000)
            .filter_map(|_| match drop_of(&mut rng, &extras, 19, 9, honey) {
                Some(Drop::Item(value)) => Some(value),
                _ => None,
            })
            .filter(|&value| crate::town::treasure_box(stack::id(value)).is_some_and(|(_, rank)| rank == 1))
            .collect();
        let opened: Vec<u16> = dropped.into_iter().filter_map(|value| content(value, &extras, 1400, &mut rng)).collect();
        let nectar = opened.iter().filter(|&&item| item == NECTAR).count();
        assert!(opened.iter().all(|&item| item == NECTAR || item == 70));
        assert!(nectar * 6 > opened.len() && nectar * 3 < opened.len(), "{nectar} of {}", opened.len());
        // (The second and third of a kind: the first or second exclusive item of a species.)
        let second = content(FIRST_BOX + 4, &extras, 1400, &mut rng).expect("an item");
        assert!(second >= FIRST_SPECIES_ITEM && (second - FIRST_SPECIES_ITEM).is_multiple_of(4));
        assert_eq!(content(FIRST_BOX - 1, &extras, 1400, &mut rng), None);
        // The box is the box whatever it holds: its name, its price.
        assert_eq!(stack::id(FIRST_BOX | (5 << CONTENT_SHIFT)), FIRST_BOX);
        assert_eq!(stack::count(FIRST_BOX | (5 << CONTENT_SHIFT)), 1);
    }
}
