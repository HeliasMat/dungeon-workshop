//! The game's tables that scripts and scenes read: levels, entities,
//! objects, effects, the animations of moves, items and traps.

use super::*;
use crate::sym;

/// The level and entity tables, with the byte length of the level table
/// (pmdsky-debug's `EVENTS` and `ENTITIES`).
const LEVELS: sym::Symbol = sym::EVENTS;
const ENTITIES: [u32; 3] = sym::ENTITIES.address;
pub(super) const ENTITY_COUNT: usize = 386;

/// A place where scenes happen.
#[derive(Clone, Debug)]
pub struct Level {
    /// Internal name, e.g. `T00P01`; its scripts are in `SCRIPT/<name>/`.
    pub name: String,
    /// Index into `MAP_BG/bg_list.dat`.
    pub map_id: u16,
    /// Which place name the level shows (`Names::places`).
    pub place_id: u16,
    /// How its map is drawn (see `DUNGEON_TILES`, `FIXED_ROOM_MAP`).
    pub map_type: u16,
}

/// A character scripts can put in a scene.
#[derive(Clone, Debug)]
pub struct Entity {
    pub name: String,
    pub kind: u16,
    /// Index into `BALANCE/monster.md` (0 for none).
    pub monster: u16,
}

/// Addresses, in the order EU, NA, JP, of the table of the animations
/// `SetAnimation` plays (overlay 11) and of the places portraits can take
/// (pmdsky-debug's `SETANIMATION_TABLE` and `PORTRAIT_LAYOUTS`).
const SET_ANIMATIONS: [u32; 3] = sym::SETANIMATION_TABLE.address;
pub(super) const SET_ANIMATION_COUNT: usize = 84;
#[cfg(test)]
const PORTRAIT_LAYOUTS: [u32; 3] = sym::PORTRAIT_LAYOUTS.address;
#[cfg(test)]
pub(super) const PORTRAIT_LAYOUT_COUNT: usize = 32;

/// Addresses, in the order EU, NA, JP, of the numbers of the item lists
/// the two shops stock from, four for each shop.
// (pmdsky-debug's `KECLEON_SHOP_ITEM_TABLE_LISTS_1`, followed by `_2`.)
const SHOP_TABLES: [u32; 3] = sym::KECLEON_SHOP_ITEM_TABLE_LISTS_1.address;

/// Address, in the order EU, NA, JP, of the pointers to the 26 item lists
/// of the ARM9 binary.
const ITEM_TABLES: [u32; 3] = sym::ITEM_TABLES_PTRS_1.address;
const ITEM_TABLE_COUNT: usize = 26;

/// The 26 item lists of the game data.
pub fn load_item_tables(rom: &NdsRom) -> Result<Vec<crate::mappa::ItemList>, RomError> {
    let (base, binary) = rom.arm9().ok_or(RomError::Truncated)?;
    let start = (ITEM_TABLES[rom.region()?] - base) as usize;
    let table = |index: usize| {
        let pointer = u32_at(binary, start + index * 4).ok_or(RomError::Truncated)?;
        let offset = pointer.checked_sub(base).ok_or(RomError::Truncated)?;
        crate::mappa::parse_item_list(binary, offset as usize)
    };
    (0..ITEM_TABLE_COUNT).map(table).collect()
}

/// The numbers (from 1) of the item lists of the big shop, then of the
/// small one: four each.
pub fn load_shop_tables(rom: &NdsRom) -> Result<[[u32; 4]; 2], RomError> {
    let (base, binary) = rom.arm9().ok_or(RomError::Truncated)?;
    let start = (SHOP_TABLES[rom.region()?] - base) as usize;
    let bytes = binary.get(start..start + 32).ok_or(RomError::Truncated)?;
    let words: Vec<u32> = bytes.as_chunks::<4>().0.iter().map(|word| u32::from_le_bytes(*word)).collect();
    Ok([std::array::from_fn(|at| words[at]), std::array::from_fn(|at| words[4 + at])])
}

/// An animation scripts can give a character.
#[derive(Clone, Copy, Debug)]
pub struct SceneAnimation {
    /// Its group in the character's sprite.
    pub group: u8,
    pub looped: bool,
    /// Held on its first frame.
    pub frozen: bool,
}

/// The animations `SetAnimation` plays, by its parameter's low byte
/// (`struct animation_data`: a group, then speed in the two low bits, 3
/// for frozen, and bit 3 for a loop).
pub fn load_scene_animations(rom: &NdsRom) -> Result<Vec<SceneAnimation>, RomError> {
    let (base, binary) = rom.overlay(11).ok_or(RomError::Truncated)?;
    let start = (SET_ANIMATIONS[rom.region()?] - base) as usize;
    let table = binary.get(start..start + SET_ANIMATION_COUNT * 2).ok_or(RomError::Truncated)?;
    let animations = table.as_chunks::<2>().0.iter().map(|entry| SceneAnimation { group: entry[0], looped: entry[1] & 8 != 0, frozen: entry[1] & 3 == 3 });
    Ok(animations.collect())
}

/// Where portraits can be shown: the tile of their top-left corner on the
/// touch screen, and whether they are mirrored (`struct portrait_layout`).
#[cfg(test)]
pub(crate) fn load_portrait_layouts(rom: &NdsRom) -> Result<Vec<(i16, i16, bool)>, RomError> {
    let (base, binary) = rom.arm9().ok_or(RomError::Truncated)?;
    let start = (PORTRAIT_LAYOUTS[rom.region()?] - base) as usize;
    let table = binary.get(start..start + PORTRAIT_LAYOUT_COUNT * 6).ok_or(RomError::Truncated)?;
    let word = |e: &[u8], at: usize| i16::from_le_bytes([e[at], e[at + 1]]);
    Ok(table.as_chunks::<6>().0.iter().map(|e| (word(e, 0), word(e, 2), e[4] != 0)).collect())
}

/// Address, in the order EU, NA, JP, of the ARM9 binary's table of how
/// partners speak (pmdsky-debug's `PARTNER_TALK_KIND_TABLE`): entries of 8
/// bytes, a talk kind and a monster, closed by the entry of no monster,
/// whose kind is that of every partner the table does not name.
#[cfg(test)]
const PARTNER_TALK_KINDS: [u32; 3] = sym::PARTNER_TALK_KIND_TABLE.address;

#[cfg(test)]
pub(crate) fn load_partner_talk_kinds(rom: &NdsRom) -> Result<Vec<(i32, u16)>, RomError> {
    let (base, binary) = rom.arm9().ok_or(RomError::Truncated)?;
    let start = (PARTNER_TALK_KINDS[rom.region()?] - base) as usize;
    let mut kinds = Vec::new();
    loop {
        let at = start + kinds.len() * 8;
        let entry = binary.get(at..at + 8).ok_or(RomError::Truncated)?;
        let kind = i32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]);
        let monster = u16::from_le_bytes([entry[4], entry[5]]);
        kinds.push((kind, monster));
        if monster == 0 {
            return Ok(kinds);
        }
    }
}

pub fn load_levels(rom: &NdsRom) -> Result<Vec<Level>, RomError> {
    let (base, binary) = rom.arm9().ok_or(RomError::Truncated)?;
    let region = rom.region()?;
    let (address, length) = (LEVELS.address[region], LEVELS.length[region] as usize);
    let start = (address - base) as usize;
    let table = binary.get(start..start + length).ok_or(RomError::Truncated)?;
    // 12 bytes: map type, place id, map id, weather, name pointer.
    let levels =
        table.as_chunks::<12>().0.iter().map_while(|e| {
            Some(Level { map_type: u16_at(e, 0)?, place_id: u16_at(e, 2)?, map_id: u16_at(e, 4)?, name: c_string(base, binary, u32_at(e, 8)?)? })
        });
    Ok(levels.collect())
}

/// Map types of levels drawn with the tiles of a dungeon's floor: where
/// their map says floor and wall, and where a fixed room does.
pub const DUNGEON_TILES: u16 = 10;
pub const FIXED_ROOM_MAP: u16 = 11;
/// Address, in the order EU, NA, JP, of overlay 11's list of those levels
/// and their floors (pmdsky-debug's `LEVEL_TILEMAP_LIST`), 8 bytes each.
const LEVEL_TILEMAPS: [u32; 3] = sym::LEVEL_TILEMAP_LIST.address;
const LEVEL_TILEMAPS_LENGTH: usize = 0x288;

/// The dungeon and floor (from 1) a level takes its tiles from, if any.
pub fn level_floor(rom: &NdsRom, level: u16) -> Option<(u8, u8)> {
    let (base, overlay) = rom.overlay(11)?;
    let start = (LEVEL_TILEMAPS[rom.region().ok()?] - base) as usize;
    let table = overlay.get(start..start + LEVEL_TILEMAPS_LENGTH)?;
    let entry = table.as_chunks::<8>().0.iter().find(|entry| u16_at(*entry, 0) == Some(level))?;
    (entry[2] != 0).then_some((entry[2], entry[3]))
}

/// Address, in the order EU, NA, JP, of the table of effects in overlay
/// 10 (pmdsky-debug's `EFFECT_ANIMATION_INFO`: 700 entries of `struct
/// effect_animation`, 28 bytes each).
const EFFECTS: [u32; 3] = sym::EFFECT_ANIMATION_INFO.address;
pub(super) const EFFECT_COUNT: usize = 700;

/// An effect drawn with a sprite of `EFFECT/effect.bin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effect {
    /// The sprite's number in the pack, and which of its animations.
    pub file: usize,
    pub animation: usize,
    /// Played over and over until it is removed, instead of once.
    pub repeats: bool,
    /// The point of a monster's sprite it is shown on (0 head, 1 and 2
    /// its hands, 3 its centre, 255 its tile).
    pub point: u8,
}

/// The effect of that number, if it is one drawn with a sprite (kinds 1
/// to 5 of the table: the pack's first two sprites, or another; kind 4 is
/// a sprite drawn from a texture, kind 5 is said to be of the whole
/// screen but is a sprite like others). See [`effect_is_rain`] for kind 6.
pub fn load_effect(rom: &NdsRom, id: u16) -> Option<Effect> {
    // The first is "no effect", whatever its entry says.
    if id == 0 {
        return None;
    }
    let (base, binary) = rom.overlay(10)?;
    let start = (EFFECTS[rom.region().ok()?] - base) as usize;
    let entry = binary.get(start..start + EFFECT_COUNT * 28)?.as_chunks::<28>().0.get(id as usize)?;
    let file = match u32_at(entry, 0)? {
        1 => 0,
        2 => 1,
        3..=5 => u32_at(entry, 4)? as usize,
        _ => return None,
    };
    Some(Effect { file, animation: u32_at(entry, 12)? as usize, repeats: entry[27] != 0, point: entry[25] })
}

/// Address, in the order EU, NA, JP, of the table of what moves show in
/// overlay 10 (pmdsky-debug's `MOVE_ANIMATION_INFO`: 563 entries of 24
/// bytes; skytemple-files' `MoveAnim`).
const MOVE_ANIMATIONS: [u32; 3] = sym::MOVE_ANIMATION_INFO.address;
pub(super) const MOVE_ANIMATION_COUNT: usize = 563;

/// The effects a move shows, as numbers of the table of effects (0 for
/// none): on its user before it attacks, on its user as it strikes, on
/// each target, and flying from the user to the target.
// See [`load_move_motion`] for how its user moves, [`move_sound`] for its
// sound.
// (The game waits for them to end, `PlayMoveAnimation`: nothing to read.)
pub fn load_move_animation(rom: &NdsRom, move_id: u16) -> Option<[u16; 4]> {
    let entry = shown_entry(rom, MOVE_ANIMATIONS, MOVE_ANIMATION_COUNT, 24, move_id as usize)?;
    Some([u16_at(entry, 0)?, u16_at(entry, 2)?, u16_at(entry, 4)?, u16_at(entry, 6)?])
}

pub use rules::tables::MoveMotion;

/// Address, in the order EU, NA, JP, of the species' own move animations
/// (pmdsky-debug's `SPECIAL_MONSTER_MOVE_ANIMATION_INFO`: 7422 entries of
/// 6 bytes).
const SPECIAL_MOVE_ANIMATIONS: [u32; 3] = sym::SPECIAL_MONSTER_MOVE_ANIMATION_INFO.address;
const SPECIAL_MOVE_ANIMATION_COUNT: usize = 7422;

pub fn load_move_motion(rom: &NdsRom, move_id: u16) -> Option<MoveMotion> {
    let entry = shown_entry(rom, MOVE_ANIMATIONS, MOVE_ANIMATION_COUNT, 24, move_id as usize)?;
    let (count, first) = (u16_at(entry, 20)? as usize, u16_at(entry, 22)? as usize);
    let species = (first..first + count)
        .filter_map(|at| {
            let own = shown_entry(rom, SPECIAL_MOVE_ANIMATIONS, SPECIAL_MOVE_ANIMATION_COUNT, 6, at)?;
            Some((u16_at(own, 0)?, own[2], own[3], u16_at(own, 4)?))
        })
        .collect();
    let flight = match u32_at(entry, 12)? {
        1 => 2,
        2 => 3,
        _ => 6,
    };
    Some(MoveMotion { group: entry[16], point: entry[17], sound: u16_at(entry, 18)?, species, lob: entry[8] & 7 == 1, flight })
}

/// Addresses, in the order EU, NA, JP, of the tables of what traps and
/// items show in overlay 10 (pmdsky-debug's `TRAP_ANIMATION_INFO`, 26
/// entries of 2 bytes, and `ITEM_ANIMATION_INFO`, 1400 entries of 4).
const TRAP_ANIMATIONS: [u32; 3] = sym::TRAP_ANIMATION_INFO.address;
pub(super) const TRAP_ANIMATION_COUNT: usize = 26;
const ITEM_ANIMATIONS: [u32; 3] = sym::ITEM_ANIMATION_INFO.address;
pub(super) const ITEM_ANIMATION_COUNT: usize = 1400;

/// An entry of one of overlay 10's tables of what is shown in dungeons.
fn shown_entry(rom: &NdsRom, addresses: [u32; 3], count: usize, size: usize, index: usize) -> Option<&[u8]> {
    let (base, binary) = rom.overlay(10)?;
    let start = (addresses[rom.region().ok()?] - base) as usize;
    binary.get(start..start + count * size)?.chunks_exact(size).nth(index)
}

/// The effect a trap shows as it is sprung (0 for none).
pub fn load_trap_animation(rom: &NdsRom, trap: usize) -> Option<u16> {
    u16_at(shown_entry(rom, TRAP_ANIMATIONS, TRAP_ANIMATION_COUNT, 2, trap)?, 0)
}

/// The two effects an item shows (0 for none).
// The first is shown on who uses it (few have one); the second (8 to 10
// for most) is how it looks thrown, which is not used: its picture flies.
pub fn load_item_animation(rom: &NdsRom, item: u16) -> Option<[u16; 2]> {
    let entry = shown_entry(rom, ITEM_ANIMATIONS, ITEM_ANIMATION_COUNT, 4, item as usize)?;
    Some([u16_at(entry, 0)?, u16_at(entry, 2)?])
}

/// Address, in the order EU, NA, JP, of the places of the world map in
/// arm9 (pmdsky-debug's `MAP_MARKER_PLACEMENTS`: 310 entries of 8 bytes).
const MAP_MARKERS: [u32; 3] = sym::MAP_MARKER_PLACEMENTS.address;
pub(super) const MAP_MARKER_COUNT: usize = 310;

/// Where a place of the world map is (a dungeon by its number, then
/// other places): the level whose map it is on, and the point there. Some
/// stand where another does; some are nowhere.
pub fn load_map_marker(rom: &NdsRom, marker: u16) -> Option<(u16, i16, i16)> {
    let (base, binary) = rom.arm9()?;
    let start = (MAP_MARKERS[rom.region().ok()?] - base) as usize;
    let table = binary.get(start..start + MAP_MARKER_COUNT * 8)?;
    let entry = |marker: usize| table.as_chunks::<8>().0.get(marker);
    let mut found = entry(marker as usize)?;
    let other = u16_at(found, 2)? as i16;
    if other >= 0 {
        found = entry(other as usize)?;
    }
    let level = u16_at(found, 0)?;
    (level != u16::MAX).then_some((level, u16_at(found, 4)? as i16, u16_at(found, 6)? as i16))
}

/// The sound that goes with the effect of that number (see
/// [`crate::sedl`]; one whose low byte is 0 is none).
pub fn effect_sound(rom: &NdsRom, id: u16) -> Option<u16> {
    u16_at(shown_entry(rom, EFFECTS, EFFECT_COUNT, 28, id as usize)?, 16)
}

/// The sound of a move as its user starts it.
pub fn move_sound(rom: &NdsRom, move_id: u16) -> Option<u16> {
    u16_at(shown_entry(rom, MOVE_ANIMATIONS, MOVE_ANIMATION_COUNT, 24, move_id as usize)?, 0x12)
}

/// Whether the effect of that number is the rain (kind 6 of the table:
/// the game's only one, drawn with `EFFECT/ame.wba`).
pub fn effect_is_rain(rom: &NdsRom, id: u16) -> bool {
    let kind = || {
        let (base, binary) = rom.overlay(10)?;
        let start = (EFFECTS[rom.region().ok()?] - base) as usize;
        u32_at(binary.get(start..start + EFFECT_COUNT * 28)?.as_chunks::<28>().0.get(id as usize)?, 0)
    };
    kind() == Some(6)
}

/// The pictures of the rain (`EFFECT/ame.wba`), as RGBA with their width
/// and height: the game shows them one after the other, side by side all
/// over the screen. The file, worked out on the ROM's: 32-bit numbers
/// giving where and how many its parts are (the texture at the fifth, of
/// 4 bits a pixel; 16 colours at the ninth; the pictures' places in the
/// texture at the eleventh, as corner and size).
pub fn load_rain(rom: &NdsRom) -> Option<Vec<(usize, usize, Vec<u8>)>> {
    let data = rom.file("EFFECT/ame.wba")?;
    let word = |index: usize| u32_at(data, index * 4).map(|value| value as usize);
    let (texture, texture_size, colours, places, count) = (word(4)?, word(5)?, word(8)?, word(10)?, word(11)?);
    let place = |index: usize| {
        let number = |part: usize| u16_at(data, places + index * 8 + part * 2).map(|value| value as usize);
        Some((number(0)?, number(1)?, number(2)?, number(3)?))
    };
    // The texture is as wide as its pictures reach.
    let wide = (0..count).filter_map(place).map(|(x, _, width, _)| x + width).max()?;
    let mut pictures = Vec::with_capacity(count);
    for (x, y, width, height) in (0..count).filter_map(place) {
        let mut rgba = vec![0u8; width * height * 4];
        for (index, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let at = (y + index / width) * wide + x + index % width;
            let byte = *data.get(texture + at / 2).filter(|_| at / 2 < texture_size)?;
            let colour = (byte >> (at % 2 * 4) & 15) as usize;
            if colour != 0 {
                pixel[..3].copy_from_slice(data.get(colours + colour * 4..colours + colour * 4 + 3)?);
                pixel[3] = 255;
            }
        }
        pictures.push((width, height, rgba));
    }
    Some(pictures)
}

/// The names of the game's objects, by number: `GROUND/<name>.wan` is the
/// sprite of each (the name is empty for the objects that have none).
pub fn load_objects(rom: &NdsRom) -> Result<Vec<String>, RomError> {
    let (base, binary) = rom.overlay(11).ok_or(RomError::Truncated)?;
    // The table of the things scenes hold besides characters
    // (pmdsky-debug's `OBJECTS`, in overlay 11), 12 bytes an entry.
    let region = rom.region()?;
    let (address, count) = (sym::OBJECTS.address[region], sym::OBJECTS.length[region] as usize / 12);
    let start = (address - base) as usize;
    let table = binary.get(start..start + count * 12).ok_or(RomError::Truncated)?;
    // 12 bytes (`struct script_object`): four of flags, the name's
    // address, four more.
    let name = |e: &[u8]| u32_at(e, 4).and_then(|p| c_string(base, binary, p)).unwrap_or_default();
    Ok(table.as_chunks::<12>().0.iter().map(|entry| name(entry)).collect())
}

pub fn load_entities(rom: &NdsRom) -> Result<Vec<Entity>, RomError> {
    let (base, binary) = rom.arm9().ok_or(RomError::Truncated)?;
    let start = (ENTITIES[rom.region()?] - base) as usize;
    let table = binary.get(start..start + ENTITY_COUNT * 12).ok_or(RomError::Truncated)?;
    // 12 bytes: type, monster id, name pointer, 4 more bytes.
    let entities = table.as_chunks::<12>().0.iter().map(|e| Entity {
        kind: u16_at(e, 0).unwrap_or(0),
        monster: u16_at(e, 2).unwrap_or(0),
        name: u32_at(e, 4).and_then(|p| c_string(base, binary, p)).unwrap_or_default(),
    });
    Ok(entities.collect())
}
