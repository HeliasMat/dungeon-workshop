//! Weighted random picks, as used by the ROM's monster, item and trap lists:
//! every entry stores a cumulative threshold out of 10000.

use super::combat::MoveSlot;
use super::floor::{Floor, Pos, Tile};
use super::rng::DungeonRng;

/// How much Poké a pile holds: an amount of `table` (the amounts money
/// comes in) no larger than `max`, the floor's most. Two are drawn and the
/// larger kept, so piles lean towards what the floor allows. Without an
/// amount that fits, the pile holds `max`.
pub fn money_amount(rng: &mut DungeonRng, table: &[i32], max: i32) -> i32 {
    let fitting: Vec<i32> = table.iter().copied().filter(|&amount| amount > 0 && amount <= max).collect();
    if fitting.is_empty() {
        return max.max(0);
    }
    let mut draw = || fitting[rng.rand_int(fitting.len() as i32) as usize];
    draw().max(draw())
}

/// The moves of a monster appearing at `level`: of the moves its species
/// has learnt by then, the three newest, and one of the older ones drawn
/// at random. They are given in learning order.
pub fn wild_moves(rng: &mut DungeonRng, learnset: &[(u8, MoveSlot)], level: i32) -> Vec<MoveSlot> {
    let mut learnt: Vec<MoveSlot> = Vec::new();
    for &(_, slot) in learnset.iter().take_while(|&&(at, _)| at as i32 <= level) {
        if !learnt.iter().any(|known| known.id == slot.id) {
            learnt.push(slot);
        }
    }
    if learnt.len() <= 4 {
        return learnt;
    }
    let newest = learnt.split_off(learnt.len() - 3);
    let older = learnt[rng.rand_int(learnt.len() as i32) as usize];
    std::iter::once(older).chain(newest).collect()
}

/// A free tile for a monster to appear on, drawn among the best kind there
/// is: a room tile out of the leader's sight, else a room tile 6 or more
/// tiles away, else any room tile, else any open tile (never the stairs).
///
/// `occupied` tells the tiles holding a monster, an item or a trap.
pub fn random_tile(rng: &mut DungeonRng, floor: &Floor, leader: Pos, occupied: impl Fn(Pos) -> bool) -> Option<Pos> {
    let free: Vec<(Pos, bool)> = Floor::positions()
        .filter_map(|p| match floor.tile(p) {
            Tile::Floor { room } if !occupied(p) => Some((p, room.is_some())),
            _ => None,
        })
        .collect();
    let unseen = |p: Pos| !super::ai::can_see(floor, leader, p);
    let far = |p: Pos| super::floor::chebyshev(leader, p) >= 6;
    let tiers: [&dyn Fn(Pos, bool) -> bool; 4] = [&|p, in_room| in_room && unseen(p), &|p, in_room| in_room && far(p), &|_, in_room| in_room, &|_, _| true];
    tiers.iter().find_map(|fits| {
        let tiles: Vec<Pos> = free.iter().filter(|&&(p, in_room)| fits(p, in_room)).map(|&(p, _)| p).collect();
        (!tiles.is_empty()).then(|| tiles[rng.rand_int(tiles.len() as i32) as usize])
    })
}

/// First entry whose threshold exceeds a roll in `0..10000`. Entries with a
/// weight of 0 never come up.
// (A weight of 0xFFFF marks an entry that is never drawn either.)
pub fn pick_weighted<T>(rng: &mut DungeonRng, entries: impl IntoIterator<Item = (T, u16)>) -> Option<T> {
    let roll = rng.rand_int(10000);
    entries.into_iter().find(|&(_, w)| w != 0 && w != u16::MAX && w as i32 > roll).map(|(entry, _)| entry)
}

/// Items that can lie on a floor.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ItemTable {
    /// `(category, weight)`.
    pub categories: Vec<(u8, u16)>,
    /// `(item id, category, weight)`; weights are cumulative per category.
    pub items: Vec<(u16, u8, u16)>,
}

impl ItemTable {
    /// Rolls a category, then an item of that category; a roll that finds
    /// nothing gives money.
    pub fn pick(&self, rng: &mut DungeonRng) -> Option<u16> {
        if self.categories.is_empty() {
            return None;
        }
        let Some(category) = pick_weighted(rng, self.categories.iter().copied()) else {
            return Some(super::status::item::POKE);
        };
        let of_category = self.items.iter().filter(|item| item.1 == category);
        Some(pick_weighted(rng, of_category.map(|&(id, _, weight)| (id, weight))).unwrap_or(super::status::item::POKE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_follow_the_thresholds() {
        let mut rng = DungeonRng::new(99);
        let mut counts = [0; 3];
        for _ in 0..10_000 {
            // 30 %, never, 70 %.
            let pick = pick_weighted(&mut rng, [(0, 3000), (1, 0), (2, 10000)]).unwrap();
            counts[pick] += 1;
        }
        assert_eq!(counts[1], 0);
        assert!((2700..3300).contains(&counts[0]), "{counts:?}");
    }

    #[test]
    fn item_comes_from_the_rolled_category() {
        let table = ItemTable { categories: vec![(3, 10000)], items: vec![(10, 2, 10000), (20, 3, 5000), (21, 3, 10000)] };
        let mut rng = DungeonRng::new(5);
        for _ in 0..100 {
            assert!(matches!(table.pick(&mut rng), Some(20 | 21)));
        }
    }
}

#[cfg(test)]
mod tile_tests {
    use super::*;

    #[test]
    fn money_piles_fit_the_floor() {
        let table: Vec<i32> = (0..100).map(|code| code * 4).collect();
        let mut rng = DungeonRng::new(8);
        let amounts: Vec<i32> = (0..500).map(|_| money_amount(&mut rng, &table, 100)).collect();
        assert!(amounts.iter().all(|&amount| (4..=100).contains(&amount)));
        // The larger of two draws: most piles are in the upper half.
        assert!(amounts.iter().filter(|&&amount| amount > 50).count() > 300);
        assert!(amounts.iter().any(|&amount| amount < 30));
        // Nothing small enough: the floor's most.
        assert_eq!(money_amount(&mut rng, &[500, 900], 100), 100);
    }

    #[test]
    fn wild_monsters_know_their_newest_moves() {
        let learnset: Vec<(u8, MoveSlot)> = [(1, 10), (1, 11), (4, 12), (7, 10), (9, 13), (12, 14), (20, 15)]
            .into_iter()
            .map(|(level, id)| (level, MoveSlot { id, ..MoveSlot::fallback_regular_attack() }))
            .collect();
        let ids = |moves: &[MoveSlot]| moves.iter().map(|slot| slot.id).collect::<Vec<_>>();
        let mut rng = DungeonRng::new(5);
        // All of them while there are four or fewer; a move is known once.
        assert_eq!(ids(&wild_moves(&mut rng, &learnset, 3)), vec![10, 11]);
        assert_eq!(ids(&wild_moves(&mut rng, &learnset, 9)), vec![10, 11, 12, 13]);
        // Past four, the three newest stay and one older joins them.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            let moves = ids(&wild_moves(&mut rng, &learnset, 12));
            assert_eq!(moves[1..], [12, 13, 14]);
            assert!(matches!(moves[0], 10 | 11));
            seen.insert(moves);
        }
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn monsters_appear_out_of_sight_first() {
        let mut floor = Floor::new_walled();
        let room = |floor: &mut Floor, xs: std::ops::Range<i32>, id: u8| {
            for y in 5..10 {
                for x in xs.clone() {
                    floor.set(Pos::new(x, y), Tile::Floor { room: Some(id) });
                }
            }
        };
        room(&mut floor, 5..25, 0);
        room(&mut floor, 32..38, 1);
        for x in 25..32 {
            floor.set(Pos::new(x, 7), Tile::Floor { room: None });
        }
        let mut rng = DungeonRng::new(3);
        let leader = Pos::new(6, 6);
        let room_of = |p: Pos| floor.tile(p).room();
        for _ in 0..100 {
            let tile = random_tile(&mut rng, &floor, leader, |_| false).unwrap();
            assert_eq!(room_of(tile), Some(1));
        }
        // The far room taken: far in the leader's room, then near, then a
        // hallway tile, then nothing.
        let other_room = |p: Pos| room_of(p) == Some(1);
        let far = |p: Pos| other_room(p) || (p.x - leader.x).abs().max((p.y - leader.y).abs()) >= 6;
        let tile = random_tile(&mut rng, &floor, leader, other_room).unwrap();
        assert!(room_of(tile) == Some(0) && far(tile));
        let near = random_tile(&mut rng, &floor, leader, far).unwrap();
        assert!(room_of(near) == Some(0) && !far(near));
        let hallway = random_tile(&mut rng, &floor, leader, |p| room_of(p).is_some()).unwrap();
        assert_eq!(floor.tile(hallway), Tile::Floor { room: None });
        assert_eq!(random_tile(&mut rng, &floor, leader, |_| true), None);
    }
}
