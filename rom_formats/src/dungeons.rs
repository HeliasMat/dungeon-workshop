//! The list of dungeons and the dungeon music tables, read from the game's
//! code. Addresses come from the pmdsky-debug project (`DUNGEON_DATA_LIST`
//! in the ARM9 binary, `MUSIC_ID_TABLE` and `RANDOM_MUSIC_ID_TABLE` in
//! overlay 10) and differ per region.

use crate::sym;
pub use rules::tables::NO_MUSIC;
use rules::tables::{Restrictions, Rules};

use super::nds::{NdsRom, RomError};

pub(crate) const DUNGEON_COUNT: usize = 180;
const MUSIC_COUNT: usize = 170;
const RANDOM_MUSIC_ROWS: usize = 30;

// In the order EU, NA, JP.
const DUNGEON_DATA_LIST: [u32; 3] = sym::DUNGEON_DATA_LIST.address;
/// `DUNGEON_RESTRICTIONS`: 12 bytes per dungeon; bit 0 of the first says
/// the dungeon goes up (floors 1F, 2F...) rather than down (B1F, B2F...).
const DUNGEON_RESTRICTIONS: [u32; 3] = sym::DUNGEON_RESTRICTIONS.address;
const MUSIC_ID_TABLE: [u32; 3] = sym::MUSIC_ID_TABLE.address;
const RANDOM_MUSIC_ID_TABLE: [u32; 3] = sym::RANDOM_MUSIC_ID_TABLE.address;

fn parse_rules(entry: &[u8]) -> Rules {
    let flags = u16::from_le_bytes([entry[0], entry[1]]);
    let bit = |n: u32| flags >> n & 1 != 0;
    Rules {
        goes_up: bit(0),
        enemies_give_exp: bit(2),
        recruitment: bit(3),
        level_one: bit(4),
        money_allowed: bit(5),
        iq_disabled: bit(8),
        max_items: entry[5],
        max_party: entry[6],
        turn_limit: u16::from_le_bytes([entry[8], entry[9]]),
        options: Default::default(),
        more: Restrictions {
            enemies_evolve: bit(1),
            leader_change: bit(6),
            enter_without_saving: bit(7),
            no_trap_uncovering: bit(9),
            treasure_boxes: bit(10),
            max_rescues: entry[4],
            random_movement: i16::from_le_bytes([entry[10], entry[11]]),
        },
    }
}

/// One dungeon: a run of floors inside a floor group of `mappa_s.bin`.
#[derive(Clone, Copy, Debug)]
pub struct DungeonEntry {
    pub floors: u8,
    /// Floor group in `mappa_s.bin`.
    pub group: u8,
    /// Floors of the group that belong to earlier dungeons.
    pub first_floor: u8,
}

pub struct DungeonData {
    pub dungeons: Vec<DungeonEntry>,
    /// Whether each dungeon goes up.
    pub goes_up: Vec<bool>,
    /// Each dungeon's rules.
    pub rules: Vec<Rules>,
    /// Song of each floor music value; values with the high bit set name a
    /// row of `random_music` instead.
    music: Vec<u16>,
    random_music: Vec<[u16; 4]>,
}

impl DungeonData {
    pub fn load(rom: &NdsRom) -> Result<Self, RomError> {
        let region = match rom.game_code().as_bytes().get(3) {
            Some(b'P') => 0,
            Some(b'E') => 1,
            Some(b'J') => 2,
            _ => return Err(RomError::Truncated),
        };
        let read = |(base, binary): (u32, &[u8]), address: u32, len: usize| {
            let off = address.checked_sub(base).ok_or(RomError::Truncated)? as usize;
            binary.get(off..off + len).map(<[u8]>::to_vec).ok_or(RomError::Truncated)
        };
        let arm9 = rom.arm9().ok_or(RomError::Truncated)?;
        let overlay10 = rom.overlay(10).ok_or(RomError::Truncated)?;

        let list = read(arm9, DUNGEON_DATA_LIST[region], DUNGEON_COUNT * 4)?;
        let dungeons = list.as_chunks::<4>().0.iter().map(|e| DungeonEntry { floors: e[0], group: e[1], first_floor: e[2] }).collect();
        let rules: Vec<Rules> =
            read(arm9, DUNGEON_RESTRICTIONS[region], DUNGEON_COUNT * 12)?.as_chunks::<12>().0.iter().map(|rules| parse_rules(rules)).collect();
        let goes_up = rules.iter().map(|rules| rules.goes_up).collect();
        let words = |bytes: Vec<u8>| -> Vec<u16> { bytes.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes([b[0], b[1]])).collect() };
        let music = words(read(overlay10, MUSIC_ID_TABLE[region], MUSIC_COUNT * 2)?);
        let random = words(read(overlay10, RANDOM_MUSIC_ID_TABLE[region], RANDOM_MUSIC_ROWS * 8)?);
        let random_music = random.as_chunks::<4>().0.iter().map(|r| [r[0], r[1], r[2], r[3]]).collect();
        Ok(Self { dungeons, goes_up, rules, music, random_music })
    }

    /// The songs a floor with this music value can play: one, or the four
    /// candidates of a random group.
    pub fn songs(&self, music_value: u8) -> Vec<u16> {
        match self.music.get(music_value as usize) {
            Some(&id) if id & 0x8000 != 0 => self.random_music.get((id & 0x7FFF) as usize).map_or(Vec::new(), |row| row.to_vec()),
            Some(&id) => vec![id],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mappa;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_dungeon_list_fits_the_floor_data() {
        let rom = crate::nds::test_rom();
        let data = DungeonData::load(&rom).unwrap();
        let mappa = Mappa::parse(rom.file("BALANCE/mappa_s.bin").unwrap()).unwrap();
        assert_eq!(data.dungeons.len(), DUNGEON_COUNT);
        // Beach Cave (1): experience, a turn limit; most dungeons have one.
        println!("Beach Cave's rules: {:?}", data.rules[1]);
        assert!(data.rules[1].enemies_give_exp && (100..=5000).contains(&data.rules[1].turn_limit));
        assert!(data.rules.iter().filter(|rules| rules.turn_limit > 0).count() > 100);
        let (mut playable, mut overflowing) = (0, 0);
        for (id, d) in data.dungeons.iter().enumerate() {
            if d.floors == 0 {
                continue;
            }
            playable += 1;
            // The dungeon's floors exist in its group...
            let group = mappa.floors(d.group as usize);
            let end = d.first_floor as usize + d.floors as usize;
            if end > group.len() {
                // (Two entries name floors past their group, read as none here: no
                // story or job goes there: 71 and 173 in the EU ROM.)
                println!("dungeon {id}: {d:?} vs {} floors", group.len());
                overflowing += 1;
                continue;
            }
            // ...carry on the group's floor numbering (a dungeon's second
            // part continues where the first stopped), and all have songs
            // that exist.
            let floors = &group[d.first_floor as usize..end];
            // (One single-floor group numbers its floor 0.)
            let number = floors[0].layout.floor_number;
            assert!(number == d.first_floor + 1 || number == 0, "dungeon {id}");
            for floor in floors {
                let songs = data.songs(floor.layout.music);
                assert!(!songs.is_empty(), "dungeon {id}");
                for song in songs.into_iter().filter(|&song| song != NO_MUSIC) {
                    let name = format!("SOUND/BGM/bgm{song:04}.smd");
                    assert!(rom.file(&name).is_some(), "dungeon {id}: no {name}");
                }
            }
        }
        println!("{playable} dungeons with floors, {overflowing} overflowing their group");
        assert!(playable > 100 && overflowing <= 5);
        // Beach Cave: dungeon 1.
        println!("dungeon 1: {:?}", data.dungeons[1]);
    }
}
