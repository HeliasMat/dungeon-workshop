//! Fixed rooms: the floors laid out by hand (boss fights, mostly).
//!
//! `BALANCE/fixed.bin` is a SIR0 container whose header points to a list of
//! 32-bit offsets, one per room, ended by `AA AA AA AA`. A room is its
//! width and height (16 bits each), an unknown 16 bits, then pairs of 16
//! bits: an action and how many more times it repeats, row by row, until
//! every tile has one. The low 12 bits of an action are, under 16, a kind
//! of terrain; from 16 on, an entity (minus 16): an index into overlay
//! 29's `FIXED_ROOM_ENTITY_SPAWN_TABLE`, whose 12-byte entries are three
//! pointers (an item, a monster, a tile) into three other tables of the
//! overlay. A monster there is 4 bytes (which monster, which line of
//! overlay 10's `FIXED_ROOM_MONSTER_SPAWN_STATS_TABLE`, how it behaves);
//! overlay 10's `FIXED_ROOM_PROPERTIES_TABLE` says, at 0xA of its 12-byte
//! entries, whether the floor is left once its enemies are defeated.
//! Layout from skytemple-files (`dungeon_data/fixed_bin`,
//! `hardcoded/fixed_floor`, GPL-3) and pmdsky-debug's structures.
//! An action's top 4 bits are the direction its monster faces (1: down, then
//! turning left). The rooms from 165 on are set inside generated floors,
//! for jobs of sealed chambers, which this game has not.

use super::nds::NdsRom;
use crate::bytes::{u16_at, u32_at};
use crate::sym;

/// What a tile of a fixed room is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedTile {
    Floor,
    Wall,
    Secondary,
}

/// A monster a fixed room places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedMonster {
    /// Its number in the monster table.
    pub monster: u16,
    pub level: u16,
    pub hp: u16,
    pub exp: i16,
    pub attack: u8,
    pub sp_attack: u8,
    pub defense: u8,
    pub sp_defense: u8,
    /// How it behaves: 0 to 6 are enemies of several kinds, others stand
    /// with the team or only watch.
    pub behavior: u8,
    /// The way it faces, 0 down, turning as `DIRECTIONS`.
    pub facing: u8,
}

/// A fixed room: its tiles row by row, where the leader starts, and the
/// monsters it places.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FixedRoom {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<FixedTile>,
    /// What is around the room: the secondary terrain, not wall.
    pub secondary_around: bool,
    pub leader: Option<(usize, usize)>,
    pub monsters: Vec<((usize, usize), FixedMonster)>,
    /// What lies there; traps, by id, and whether they show.
    pub items: Vec<((usize, usize), u16)>,
    pub traps: Vec<((usize, usize), u8, bool)>,
    /// The warp zone, the way on.
    pub exit: Option<(usize, usize)>,
    /// The floor is left as soon as no enemy is left on it.
    pub ends_with_enemies: bool,
    pub moves_allowed: bool,
    pub orbs_allowed: bool,
    /// Its song (0: the floor's).
    pub music: u16,
}

/// Addresses, in the order EU, NA, JP.
const ENTITIES: [u32; 3] = sym::FIXED_ROOM_ENTITY_SPAWN_TABLE.address;
const ENTITY_COUNT: usize = 0xC9C / 12;
const MONSTERS: [u32; 3] = sym::FIXED_ROOM_MONSTER_SPAWN_TABLE.address;
const MONSTER_COUNT: usize = 0x1E0 / 4;
const STATS: [u32; 3] = sym::FIXED_ROOM_MONSTER_SPAWN_STATS_TABLE.address;
const STAT_COUNT: usize = 0x4A4 / 12;
const PROPERTIES: [u32; 3] = sym::FIXED_ROOM_PROPERTIES_TABLE.address;
/// A tile entity's trap when it has none, and its flag of secondary
/// terrain (`fixed_room_tile_spawn_entry`).
const NO_TRAP: u8 = 25;
const TILE_SHOWS: u8 = 1;
const TILE_SECONDARY: u8 = 8;

/// The terrain of an action under 16 (skytemple-files' `TileRuleType`):
/// the leader's start (4) and the warp zone (8) are floor.
fn terrain(action: u16) -> FixedTile {
    match action {
        1..=3 => FixedTile::Wall,
        5..=7 | 10 => FixedTile::Secondary,
        _ => FixedTile::Floor,
    }
}

/// Fixed room `id`, if the ROM has it.
pub fn load_room(rom: &NdsRom, id: u8) -> Option<FixedRoom> {
    let data = rom.file("BALANCE/fixed.bin")?;
    if data.get(0..4)? != b"SIR0" {
        return None;
    }
    let list = u32_at(data, 4)? as usize;
    let start = u32_at(data, list + id as usize * 4)?;
    // (The list ends on padding, never a room.)
    if start == 0xAAAA_AAAA || (0..=id as usize).any(|before| u32_at(data, list + before * 4) == Some(0xAAAA_AAAA)) {
        return None;
    }
    let start = start as usize;
    let (width, height) = (u16_at(data, start)? as usize, u16_at(data, start + 2)? as usize);
    let region = rom.region().ok()?;
    let (base29, overlay29) = rom.overlay(29)?;
    let (base10, overlay10) = rom.overlay(10)?;
    let at29 = |address: u32, size: usize| overlay29.get((address.checked_sub(base29)?) as usize..)?.get(..size);
    let at10 = |address: u32, size: usize| overlay10.get((address.checked_sub(base10)?) as usize..)?.get(..size);
    let entry_of = |entity: usize| (entity < ENTITY_COUNT).then(|| at29(ENTITIES[region] + entity as u32 * 12, 12)).flatten();
    let item_of = |entity: usize| u16_at(at29(u32_at(entry_of(entity)?, 0)?, 2)?, 0).filter(|&item| item != 0);
    let tile_of = |entity: usize| at29(u32_at(entry_of(entity)?, 8)?, 4);
    let monster_of = |entity: usize, facing: u8| -> Option<FixedMonster> {
        let entry = entry_of(entity)?;
        let spawn = at29(u32_at(entry, 4)?, 4)?;
        let index = (u32_at(entry, 4)?.checked_sub(MONSTERS[region])? / 4) as usize;
        let monster = u16_at(spawn, 0)?;
        if monster == 0 || index >= MONSTER_COUNT || spawn[2] as usize >= STAT_COUNT {
            return None;
        }
        let stats = at10(STATS[region] + spawn[2] as u32 * 12, 12)?;
        Some(FixedMonster {
            monster,
            level: u16_at(stats, 0)?,
            hp: u16_at(stats, 2)?,
            exp: u16_at(stats, 4)? as i16,
            attack: stats[6],
            sp_attack: stats[7],
            defense: stats[8],
            sp_defense: stats[9],
            behavior: spawn[3],
            facing,
        })
    };
    let mut room = FixedRoom { width, height, ..Default::default() };
    let mut cursor = start + 6;
    while room.tiles.len() < width * height {
        let (raw, more) = (u16_at(data, cursor)?, u16_at(data, cursor + 2)? as usize);
        let (action, facing) = (raw & 0xFFF, ((raw >> 12) as u8).saturating_sub(1));
        cursor += 4;
        for _ in 0..=more {
            let at = (room.tiles.len() % width.max(1), room.tiles.len() / width.max(1));
            room.secondary_around |= action == 7;
            let mut tile = terrain(action.min(15));
            match action {
                4 => room.leader = room.leader.or(Some(at)),
                8 => room.exit = room.exit.or(Some(at)),
                16.. => {
                    let entity = action as usize - 16;
                    room.monsters.extend(monster_of(entity, facing).map(|monster| (at, monster)));
                    room.items.extend(item_of(entity).map(|item| (at, item)));
                    let spawned = tile_of(entity).unwrap_or(&[NO_TRAP, 0, 0, 0]);
                    if spawned[0] != NO_TRAP {
                        room.traps.push((at, spawned[0], spawned[3] & TILE_SHOWS != 0));
                    }
                    tile = if spawned[3] & TILE_SECONDARY != 0 { FixedTile::Secondary } else { FixedTile::Floor };
                }
                _ => {}
            }
            room.tiles.push(tile);
        }
    }
    room.tiles.truncate(width * height);
    let properties = at10(PROPERTIES[region] + id as u32 * 12, 12)?;
    (room.music, room.moves_allowed, room.orbs_allowed) = (u16_at(properties, 0)?, properties[6] != 0, properties[7] != 0);
    room.ends_with_enemies = properties[0xA] != 0;
    Some(room)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real dump.
    #[test]
    #[ignore]
    fn real_rom_fixed_rooms_are_read() {
        let rom = crate::nds::test_rom();
        let rooms: Vec<(u8, FixedRoom)> = (1..165).filter_map(|id| Some((id, load_room(&rom, id)?))).collect();
        for (id, room) in rooms.iter().take(12) {
            let monsters: Vec<(u16, u16, u16, u8)> = room.monsters.iter().map(|(_, m)| (m.monster, m.level, m.hp, m.behavior)).collect();
            println!("room {id}: {}x{} leader {:?} ends with enemies {} monsters {monsters:?}", room.width, room.height, room.leader, room.ends_with_enemies);
        }
        assert!(rooms.len() > 100, "{}", rooms.len());
        // They fit a floor, have somewhere to stand and a start for the
        // leader; many are fights that end the floor.
        assert!(rooms.iter().all(|(_, room)| room.width <= 56 && room.height <= 32 && room.tiles.len() == room.width * room.height));
        // The first are the pit of Beach Cave (Koffing and Zubat) and the
        // peak of Mt. Bristle (Drowzee, level 12 with 100 HP).
        let first: Vec<u16> = rooms[0].1.monsters.iter().map(|(_, monster)| monster.monster).collect();
        assert_eq!(first, [109, 41]);
        let drowzee = rooms[1].1.monsters[0].1;
        assert_eq!((drowzee.monster, drowzee.level, drowzee.hp), (96, 12, 100));
        let fights = rooms.iter().filter(|(_, room)| room.ends_with_enemies && !room.monsters.is_empty()).count();
        let started = rooms.iter().filter(|(_, room)| room.leader.is_some()).count();
        println!("{} rooms, {fights} fights, {started} with a start", rooms.len());
        assert!(fights > 30 && started > 100);
    }
}
