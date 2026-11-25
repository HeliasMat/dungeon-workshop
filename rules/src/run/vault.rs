//! Vaults: the small rooms the game sets inside a generated floor (its
//! fixed rooms from 165, `FloorExtras::vault`), an item behind a door the
//! Key opens.
//!
//! The game builds them into its grid as it lays the floor out; here, as
//! the generator's floor is kept seed for seed, one is set afterwards at the
//! middle of a room, the walls it covers opened up into the room. What the
//! game makes secondary terrain inside one is floor here, for its item to be
//! reached.

use crate::dungeon::{FLOOR_HEIGHT, FLOOR_WIDTH, Floor, Pos, Tile};
use crate::tables::{Fixed, KEY_DOOR};

/// What a vault sets on its floor: its doors, with the room they open
/// onto, and its items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Vault {
    pub doors: Vec<(Pos, Option<u8>)>,
    pub items: Vec<(Pos, u16)>,
}

/// The kinds of a fixed room's tiles (`Fixed::tiles`).
const WALL: u8 = 1;

/// Sets the vault at the middle of a room, the biggest it may go in (not
/// the shop's nor the Monster House's, the stairs and the leader's start
/// outside it), what it covers of the walls around opened up into the
/// room: its walls, its doors (walls until opened), what was to come there
/// moved out. `None`: no room for it.
pub(crate) fn set_vault(floor: &mut Floor, vault: &Fixed) -> Option<Vault> {
    let (width, height) = (vault.width as i32, vault.height as i32);
    let mut rooms = rooms(floor);
    rooms.sort_by_key(|&(room, x0, y0, x1, y1)| (std::cmp::Reverse((x1 - x0 + 1) * (y1 - y0 + 1)), room));
    let (room, left, top) = rooms.iter().find_map(|&(room, x0, y0, x1, y1)| {
        let left = ((x0 + x1 + 1 - width) / 2).clamp(2, FLOOR_WIDTH - 2 - width);
        let top = ((y0 + y1 + 1 - height) / 2).clamp(2, FLOOR_HEIGHT - 2 - height);
        let covered = (left..left + width).flat_map(|x| (top..top + height).map(move |y| Pos::new(x, y)));
        let fits = floor.monster_house != Some(room)
            && !covered
                .clone()
                .any(|tile| tile == floor.stairs || tile == floor.player_spawn || floor.in_shop(tile) || matches!(floor.tile(tile), Tile::Stairs { .. }))
            && covered.clone().any(|tile| floor.tile(tile) == Tile::Floor { room: Some(room) });
        fits.then_some((room, left, top))
    })?;
    let inside = |tile: &Pos| (left..left + width).contains(&tile.x) && (top..top + height).contains(&tile.y);
    for spawns in [&mut floor.item_spawns, &mut floor.trap_spawns, &mut floor.enemy_spawns, &mut floor.buried_item_spawns] {
        spawns.retain(|tile| !inside(tile));
    }
    let mut set = Vault::default();
    for (at, &kind) in vault.tiles.iter().enumerate() {
        let tile = Pos::new(left + (at % vault.width.max(1)) as i32, top + (at / vault.width.max(1)) as i32);
        match kind {
            WALL => floor.set(tile, Tile::Wall),
            KEY_DOOR => {
                floor.set(tile, Tile::Wall);
                set.doors.push((tile, Some(room)));
            }
            _ if !floor.tile(tile).is_walkable() => floor.set(tile, Tile::Floor { room: Some(room) }),
            _ => {}
        }
    }
    set.items = vault.items.iter().map(|&((x, y), item)| (Pos::new(left + x as i32, top + y as i32), item)).collect();
    Some(set)
}

/// Each room of the floor and the rectangle it spans, by room.
fn rooms(floor: &Floor) -> Vec<(u8, i32, i32, i32, i32)> {
    let mut rooms: Vec<(u8, i32, i32, i32, i32)> = Vec::new();
    for tile in Floor::positions() {
        let Some(room) = floor.tile(tile).room() else {
            continue;
        };
        match rooms.iter_mut().find(|known| known.0 == room) {
            Some(known) => *known = (room, known.1.min(tile.x), known.2.min(tile.y), known.3.max(tile.x), known.4.max(tile.y)),
            None => rooms.push((room, tile.x, tile.y, tile.x, tile.y)),
        }
    }
    rooms.sort_unstable();
    rooms
}

impl Fixed {
    /// Its key doors, on the floor it lays out.
    pub(crate) fn key_doors(&self) -> Vec<(Pos, Option<u8>)> {
        let width = self.width.max(1);
        self.tiles.iter().enumerate().filter(|(_, kind)| **kind == KEY_DOOR).map(|(at, _)| (Self::tile((at % width, at / width)), Some(0))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Room 181 of the game: a Miracle Chest behind its door.
    fn vault() -> Fixed {
        Fixed {
            width: 5,
            height: 5,
            tiles: vec![0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 1, 0, 0, 1, 3, 1, 0, 0, 0, 0, 0, 0],
            leader: None,
            monsters: Vec::new(),
            allies: Vec::new(),
            watchers: Vec::new(),
            items: vec![((2, 2), 66)],
            traps: Vec::new(),
            exit: None,
            ends_with_enemies: false,
            moves_allowed: true,
            orbs_allowed: true,
        }
    }

    fn floor() -> Floor {
        let mut floor = Floor::new_walled();
        let mut room = |room: u8, x0: i32, x1: i32| {
            for y in 4..11 {
                for x in x0..=x1 {
                    floor.set(Pos::new(x, y), Tile::Floor { room: Some(room) });
                }
            }
        };
        room(0, 3, 5);
        room(1, 10, 18);
        (floor.stairs, floor.player_spawn) = (Pos::new(3, 4), Pos::new(4, 4));
        floor.item_spawns = vec![Pos::new(14, 7), Pos::new(10, 4)];
        floor
    }

    #[test]
    fn a_vault_is_set_in_a_room_it_fits_in_its_door_shut() {
        let mut floor = floor();
        let set = set_vault(&mut floor, &vault()).expect("room 1 holds it");
        // In the middle of the second room (the first is too narrow).
        assert_eq!(set.items, [(Pos::new(14, 7), 66)]);
        assert_eq!(set.doors, [(Pos::new(14, 8), Some(1))]);
        assert_eq!(floor.tile(Pos::new(14, 8)), Tile::Wall);
        assert_eq!(floor.tile(Pos::new(13, 6)), Tile::Wall);
        assert_eq!(floor.tile(Pos::new(14, 7)), Tile::Floor { room: Some(1) });
        // What was to lie there went; what lies outside stays.
        assert_eq!(floor.item_spawns, [Pos::new(10, 4)]);
        // Nowhere it fits: no vault.
        let mut small = Floor::new_walled();
        assert_eq!(set_vault(&mut small, &vault()), None);
        assert_eq!(vault().key_doors(), [(Fixed::tile((2, 3)), Some(0))]);
    }
}
