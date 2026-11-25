//! `BALANCE/mappa_s.bin`: per-floor dungeon properties.
//!
//! SIR0 container whose header points to five tables: floor lists, floor
//! layouts, item lists, monster spawn lists, trap lists; all five are
//! read (see [`FloorData`]). Layout offsets follow pmdsky-debug's `floor_properties`.

use crate::bytes::{offset_at, u16_le};
use rules::dungeon::FloorProperties;

use super::nds::RomError;

const FLOOR_ENTRY_SIZE: usize = 18;
const LAYOUT_SIZE: usize = 32;
pub(crate) const TRAP_COUNT: usize = 25;
const ITEM_CATEGORY_COUNT: usize = 16;
/// Item lists only cover item ids 0..=363 (every list in the EU ROM lands
/// exactly on slot 16 + 364).
const ITEM_COUNT: usize = 364;
/// Item list values from here up encode "skip (value - 30000) slots".
const ITEM_SKIP_BASE: u16 = 30000;

#[derive(Clone, Copy, Debug)]
pub struct FloorLayout {
    pub structure: u8,
    /// Negative: exact room count. Positive: count before a small random bonus.
    pub room_density: i8,
    pub tileset: u8,
    pub music: u8,
    pub weather: u8,
    pub floor_connectivity: u8,
    pub initial_enemy_density: i8,
    pub kecleon_shop_chance: u8,
    pub monster_house_chance: u8,
    pub maze_room_chance: u8,
    pub sticky_item_chance: u8,
    pub dead_ends: bool,
    pub itemless_monster_house_chance: u8,
    /// Despite the name: the maximum number of secondary structures (0x0C).
    pub secondary_terrain_type: u8,
    pub terrain_settings: u8,
    pub item_density: u8,
    pub trap_density: u8,
    pub floor_number: u8,
    pub fixed_room_id: u8,
    pub extra_hallways: u8,
    pub buried_item_density: u8,
    pub secondary_terrain_density: u8,
    pub visibility_range: u8,
    /// A fifth of the most Poké one pile can hold on the floor.
    pub max_money_div_5: u8,
    pub enemy_iq: i16,
    /// Which tiles of a Kecleon shop always have an item, the others a
    /// chance (`enum shop_item_positions`).
    pub shop_item_positions: u8,
    /// Where hidden stairs lead (0 the Secret Bazaar, 1 the Secret Room,
    /// 255 either), and their chance in 100.
    pub hidden_stairs_type: u8,
    pub hidden_stairs_chance: u8,
}

impl FloorLayout {
    fn parse(b: &[u8]) -> Self {
        Self {
            structure: b[0x00],
            room_density: b[0x01] as i8,
            tileset: b[0x02],
            music: b[0x03],
            weather: b[0x04],
            floor_connectivity: b[0x05],
            initial_enemy_density: b[0x06] as i8,
            kecleon_shop_chance: b[0x07],
            monster_house_chance: b[0x08],
            maze_room_chance: b[0x09],
            sticky_item_chance: b[0x0A],
            dead_ends: b[0x0B] != 0,
            secondary_terrain_type: b[0x0C],
            terrain_settings: b[0x0D],
            item_density: b[0x0F],
            trap_density: b[0x10],
            floor_number: b[0x11],
            fixed_room_id: b[0x12],
            extra_hallways: b[0x13],
            buried_item_density: b[0x14],
            secondary_terrain_density: b[0x15],
            visibility_range: b[0x16],
            max_money_div_5: b[0x17],
            shop_item_positions: b[0x18],
            itemless_monster_house_chance: b[0x19],
            hidden_stairs_type: b[0x1A],
            hidden_stairs_chance: b[0x1B],
            enemy_iq: i16::from_le_bytes([b[0x1C], b[0x1D]]),
        }
    }

    pub fn properties(&self) -> FloorProperties {
        FloorProperties {
            layout: self.structure,
            room_density: self.room_density as i32,
            floor_connectivity: self.floor_connectivity as i32,
            dead_ends: self.dead_ends,
            extra_hallways: self.extra_hallways as i32,
            flags: self.terrain_settings,
            secondary_density: self.secondary_terrain_density as i32,
            kecleon_shop_chance: self.kecleon_shop_chance as i32,
            monster_house_chance: self.monster_house_chance as i32,
            maze_room_chance: self.maze_room_chance as i32,
            empty_monster_house_chance: self.itemless_monster_house_chance as i32,
            secondary_structures: self.secondary_terrain_type as i32,
            enemy_density: self.initial_enemy_density as i32,
            item_density: self.item_density as i32,
            buried_item_density: self.buried_item_density as i32,
            trap_density: self.trap_density as i32,
            disarmed_lone_house: false,
        }
    }
}

/// One candidate of a floor's monster spawn list.
#[derive(Clone, Copy, Debug)]
pub struct MonsterSpawn {
    pub level: u8,
    /// Cumulative threshold out of 10000; 0 means "never spawns normally".
    pub weight: u16,
    /// Index into `BALANCE/monster.md`.
    pub md_index: u16,
}

/// Spawn weights of an item list. Weights are cumulative thresholds out of
/// 10000, first over the categories, then over the items of a category.
#[derive(Clone, Debug, Default)]
pub struct ItemList {
    /// `(category, weight)`.
    pub categories: Vec<(u8, u16)>,
    /// `(item id, weight)`.
    pub items: Vec<(u16, u16)>,
}

#[derive(Clone, Debug)]
pub struct FloorData {
    pub layout: FloorLayout,
    pub spawns: Vec<MonsterSpawn>,
    /// Weight of each of the 25 trap kinds.
    pub traps: [u16; TRAP_COUNT],
    /// Items lying on the floor (monster house and buried lists are not
    /// read yet).
    pub items: ItemList,
    /// Those a Kecleon shop sells there (the list after the floor's in the
    /// floor's entry).
    pub shop_items: ItemList,
    /// The Secret Bazaar's grab bags and the Secret Room's chests, for
    /// the floor's hidden stairs (the entry's last two lists).
    pub grab_bag: ItemList,
    pub chests: ItemList,
}

pub struct Mappa {
    /// `groups[g][f]` is floor `f + 1` of floor-list group `g`.
    groups: Vec<Vec<FloorData>>,
}

impl Mappa {
    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        if data.get(0..4) != Some(b"SIR0") {
            return Err(RomError::Truncated);
        }
        let header = offset_at(data, 4)?;
        let floor_lists = offset_at(data, header)?;
        let layouts = offset_at(data, header + 4)?;
        let item_lists = offset_at(data, header + 8)?;
        let monster_lists = offset_at(data, header + 12)?;
        let trap_lists = offset_at(data, header + 16)?;

        // The group pointer table runs up to the layout table.
        let starts: Vec<usize> = (floor_lists..layouts).step_by(4).map(|o| offset_at(data, o)).collect::<Result<_, _>>()?;
        // A group's floors run up to the next group's start (or the pointer table).
        let mut bounds = starts.clone();
        bounds.push(floor_lists);
        bounds.sort_unstable();

        let mut groups = Vec::with_capacity(starts.len());
        for &start in &starts {
            let end = bounds.iter().copied().find(|&b| b > start).unwrap_or(floor_lists);
            let count = (end - start) / FLOOR_ENTRY_SIZE;
            let mut floors = Vec::new();
            // Entry 0 of every group is a null placeholder.
            for f in 1..count {
                let entry = start + f * FLOOR_ENTRY_SIZE;
                let layout_idx = u16_le(data, entry)? as usize;
                let off = layouts + layout_idx * LAYOUT_SIZE;
                let bytes = data.get(off..off + LAYOUT_SIZE).ok_or(RomError::Truncated)?;
                let list_idx = u16_le(data, entry + 2)? as usize;
                let list = offset_at(data, monster_lists + list_idx * 4)?;
                let trap_idx = u16_le(data, entry + 4)? as usize;
                let trap_list = offset_at(data, trap_lists + trap_idx * 4)?;
                let mut traps = [0; TRAP_COUNT];
                for (t, weight) in traps.iter_mut().enumerate() {
                    *weight = u16_le(data, trap_list + t * 2)?;
                }
                let item_idx = u16_le(data, entry + 6)? as usize;
                let item_list = offset_at(data, item_lists + item_idx * 4)?;
                let shop_idx = u16_le(data, entry + 8)? as usize;
                let shop_list = offset_at(data, item_lists + shop_idx * 4)?;
                let list_at = |at: usize| -> Result<ItemList, RomError> {
                    let index = u16_le(data, entry + at)? as usize;
                    parse_items(data, offset_at(data, item_lists + index * 4)?)
                };
                floors.push(FloorData {
                    layout: FloorLayout::parse(bytes),
                    spawns: parse_spawns(data, list)?,
                    traps,
                    items: parse_items(data, item_list)?,
                    shop_items: parse_items(data, shop_list).unwrap_or_default(),
                    grab_bag: list_at(14).unwrap_or_default(),
                    chests: list_at(16).unwrap_or_default(),
                });
            }
            groups.push(floors);
        }
        Ok(Self { groups })
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn floors(&self, group: usize) -> &[FloorData] {
        self.groups.get(group).map_or(&[], Vec::as_slice)
    }
}

/// 8-byte entries `level << 9, weight, monster-house weight, md index`,
/// closed by an all-zero entry.
fn parse_spawns(data: &[u8], mut off: usize) -> Result<Vec<MonsterSpawn>, RomError> {
    let mut spawns = Vec::new();
    loop {
        let (level, weight, md_index) = (u16_le(data, off)?, u16_le(data, off + 2)?, u16_le(data, off + 6)?);
        if level == 0 && md_index == 0 {
            return Ok(spawns);
        }
        spawns.push(MonsterSpawn { level: (level >> 9) as u8, weight, md_index });
        off += 8;
    }
}

/// Run of u16 covering 16 category slots then every item id: a value is
/// either the weight of the current slot or a number of slots to skip.
/// An item list starting at `offset` of `data`, in the format of the
/// floors' lists.
pub(crate) fn parse_item_list(data: &[u8], offset: usize) -> Result<ItemList, RomError> {
    parse_items(data, offset)
}

fn parse_items(data: &[u8], mut off: usize) -> Result<ItemList, RomError> {
    let total = ITEM_CATEGORY_COUNT + ITEM_COUNT;
    let mut list = ItemList::default();
    let mut slot = 0;
    while slot < total {
        let value = u16_le(data, off)?;
        off += 2;
        // 0xFFFF is a regular slot value (seen in 9 lists of the EU ROM), not a skip.
        if (ITEM_SKIP_BASE..u16::MAX).contains(&value) {
            slot += (value - ITEM_SKIP_BASE) as usize;
        } else {
            if slot < ITEM_CATEGORY_COUNT {
                list.categories.push((slot as u8, value));
            } else {
                list.items.push(((slot - ITEM_CATEGORY_COUNT) as u16, value));
            }
            slot += 1;
        }
    }
    // A well-formed list lands exactly on the last slot.
    if slot == total { Ok(list) } else { Err(RomError::Truncated) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_floor_numbers_are_sequential() {
        let rom = crate::nds::test_rom();
        let mappa = Mappa::parse(rom.file("BALANCE/mappa_s.bin").unwrap()).unwrap();
        assert!(mappa.group_count() > 50);
        let mut total = 0;
        for g in 0..mappa.group_count() {
            for floor in mappa.floors(g) {
                // The only zero in the EU ROM is a one-floor group (63).
                assert!(floor.layout.floor_number >= 1 || mappa.floors(g).len() == 1, "group {g}");
                assert!(floor.spawns.iter().all(|s| s.level <= 100 && s.weight <= 10000));
                assert!(floor.traps.iter().all(|&w| w <= 10000), "group {g}");
                assert!(!floor.items.categories.is_empty(), "group {g}");
                let valid = |w: u16| w <= 10000 || w == u16::MAX;
                assert!(floor.items.categories.iter().all(|&(_, w)| valid(w)), "group {g}");
                assert!(floor.items.items.iter().all(|&(_, w)| valid(w)), "group {g}");
                total += 1;
            }
        }
        println!("{} groups, {total} floors", mappa.group_count());
        for (i, f) in mappa.floors(1).iter().enumerate() {
            println!("group 1 floor {}: {f:?}", i + 1);
        }
    }
}
