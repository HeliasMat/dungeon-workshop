//! Nintendo DS cartridge image: header + NitroFS (FNT/FAT) file system.

use crate::bytes::{u16_le, u32_le};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// Game codes of Explorers of Sky (the 4th letter is the region).
const SKY_GAME_CODE_PREFIX: &str = "C2S";

#[derive(Debug)]
pub enum RomError {
    Io(std::io::Error),
    Truncated,
    NotExplorersOfSky(String),
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomError::Io(e) => write!(f, "cannot read ROM: {e}"),
            RomError::Truncated => write!(f, "ROM is truncated or corrupt"),
            RomError::NotExplorersOfSky(code) => {
                write!(f, "game code {code:?} is not Explorers of Sky (C2S*)")
            }
        }
    }
}

impl std::error::Error for RomError {}

impl From<std::io::Error> for RomError {
    fn from(e: std::io::Error) -> Self {
        RomError::Io(e)
    }
}

pub struct NdsRom {
    data: Vec<u8>,
    game_code: String,
    /// Full path (e.g. `BALANCE/mappa_s.bin`) -> byte range in `data`.
    // (In the order of their paths: what goes through them finds the same first.)
    files: BTreeMap<String, (usize, usize)>,
    fat_off: usize,
    /// Offset and size of the ARM9 overlay table.
    overlay_table: (usize, usize),
}

/// The ROM the tests on the game's files read: `DUNGEON_ROM` names it (they are
/// ignored without it: `cargo test -p rom -- --ignored`).
#[cfg(test)]
pub(crate) fn test_rom() -> NdsRom {
    let path = std::env::var("DUNGEON_ROM").expect("DUNGEON_ROM: the path of the ROM these tests read");
    NdsRom::open(path).expect("the ROM opens")
}

impl NdsRom {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RomError> {
        Self::from_bytes(std::fs::read(path)?)
    }

    pub(crate) fn from_bytes(data: Vec<u8>) -> Result<Self, RomError> {
        let game_code = String::from_utf8_lossy(slice(&data, 0x0C, 4)?).into_owned();
        if !game_code.starts_with(SKY_GAME_CODE_PREFIX) {
            return Err(RomError::NotExplorersOfSky(game_code));
        }
        let fnt_off = u32_le(&data, 0x40)? as usize;
        let fat_off = u32_le(&data, 0x48)? as usize;

        let mut files = BTreeMap::new();
        // Directory ids are 0xF000 | index; the root is 0xF000.
        let mut pending = vec![(0xF000u16, String::new())];
        while let Some((dir_id, prefix)) = pending.pop() {
            let entry = fnt_off + (dir_id as usize & 0xFFF) * 8;
            let mut pos = fnt_off + u32_le(&data, entry)? as usize;
            let mut file_id = u16_le(&data, entry + 4)? as usize;
            loop {
                let len_byte = *data.get(pos).ok_or(RomError::Truncated)?;
                if len_byte == 0 {
                    break;
                }
                let len = (len_byte & 0x7F) as usize;
                let name = String::from_utf8_lossy(slice(&data, pos + 1, len)?);
                let full = format!("{prefix}{name}");
                pos += 1 + len;
                if len_byte & 0x80 != 0 {
                    pending.push((u16_le(&data, pos)?, format!("{full}/")));
                    pos += 2;
                } else {
                    let fat = fat_off + file_id * 8;
                    let start = u32_le(&data, fat)? as usize;
                    let end = u32_le(&data, fat + 4)? as usize;
                    if start > end || end > data.len() {
                        return Err(RomError::Truncated);
                    }
                    files.insert(full, (start, end));
                    file_id += 1;
                }
            }
        }
        let overlay_table = (u32_le(&data, 0x50)? as usize, u32_le(&data, 0x54)? as usize);
        Ok(Self { data, game_code, files, fat_off, overlay_table })
    }

    /// The main ARM9 binary: the address it is loaded at and its contents.
    pub(crate) fn arm9(&self) -> Option<(u32, &[u8])> {
        let offset = u32_le(&self.data, 0x20).ok()? as usize;
        let address = u32_le(&self.data, 0x28).ok()?;
        let size = u32_le(&self.data, 0x2C).ok()? as usize;
        Some((address, self.data.get(offset..offset + size)?))
    }

    /// ARM9 overlay `id`: the address it is loaded at and its contents.
    /// `None` if it is missing or stored compressed.
    pub fn overlay(&self, id: u32) -> Option<(u32, &[u8])> {
        let (table, size) = self.overlay_table;
        // 32-byte entries: id, load address, ..., file id at 0x18, flags at 0x1F.
        let entry = (0..size / 32).map(|i| table + i * 32).find(|&e| u32_le(&self.data, e).ok() == Some(id))?;
        if self.data.get(entry + 0x1F)? & 1 != 0 {
            return None;
        }
        let address = u32_le(&self.data, entry + 4).ok()?;
        let fat = self.fat_off + u32_le(&self.data, entry + 0x18).ok()? as usize * 8;
        let start = u32_le(&self.data, fat).ok()? as usize;
        let end = u32_le(&self.data, fat + 4).ok()? as usize;
        Some((address, self.data.get(start..end)?))
    }

    pub fn game_code(&self) -> &str {
        &self.game_code
    }

    /// Which of the game's releases it is, in the order the tables of
    /// addresses go by: 0 Europe, 1 North America, 2 Japan.
    pub fn region(&self) -> Result<usize, RomError> {
        match self.game_code.as_bytes().get(3) {
            Some(b'P') => Ok(0),
            Some(b'E') => Ok(1),
            Some(b'J') => Ok(2),
            _ => Err(RomError::NotExplorersOfSky(self.game_code.clone())),
        }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// File contents by NitroFS path, e.g. `BALANCE/mappa_s.bin`.
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        let &(start, end) = self.files.get(path)?;
        Some(&self.data[start..end])
    }
}

fn slice(data: &[u8], off: usize, len: usize) -> Result<&[u8], RomError> {
    data.get(off..off + len).ok_or(RomError::Truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal image: root holds `a.bin` and directory `D` holding `b.bin`.
    fn tiny_rom(code: &[u8; 4]) -> Vec<u8> {
        let mut rom = vec![0u8; 0x200];
        rom[0x0C..0x10].copy_from_slice(code);
        let fnt = 0x100usize;
        let fat = 0x180usize;
        rom[0x40..0x44].copy_from_slice(&(fnt as u32).to_le_bytes());
        rom[0x48..0x4C].copy_from_slice(&(fat as u32).to_le_bytes());
        // Main table: root (sub-table at 16, first file 0), D (sub-table at 27, first file 1).
        rom[fnt..fnt + 4].copy_from_slice(&16u32.to_le_bytes());
        rom[fnt + 6..fnt + 8].copy_from_slice(&2u16.to_le_bytes());
        rom[fnt + 8..fnt + 12].copy_from_slice(&27u32.to_le_bytes());
        rom[fnt + 12..fnt + 14].copy_from_slice(&1u16.to_le_bytes());
        let root_sub = [&[5u8][..], b"a.bin", &[0x81], b"D", &[0x01, 0xF0, 0x00]].concat();
        rom[fnt + 16..fnt + 16 + root_sub.len()].copy_from_slice(&root_sub);
        let d_sub = [&[5u8][..], b"b.bin", &[0x00]].concat();
        rom[fnt + 27..fnt + 27 + d_sub.len()].copy_from_slice(&d_sub);
        for (i, (s, e)) in [(0x1F0u32, 0x1F2u32), (0x1F2, 0x1F5)].into_iter().enumerate() {
            rom[fat + i * 8..fat + i * 8 + 4].copy_from_slice(&s.to_le_bytes());
            rom[fat + i * 8 + 4..fat + i * 8 + 8].copy_from_slice(&e.to_le_bytes());
        }
        rom[0x1F0..0x1F5].copy_from_slice(&[1, 2, 3, 4, 5]);
        rom
    }

    #[test]
    fn reads_nested_files() {
        let rom = NdsRom::from_bytes(tiny_rom(b"C2SE")).unwrap();
        assert_eq!(rom.file_count(), 2);
        assert_eq!(rom.file("a.bin"), Some(&[1u8, 2][..]));
        assert_eq!(rom.file("D/b.bin"), Some(&[3u8, 4, 5][..]));
    }

    #[test]
    fn rejects_other_games() {
        assert!(matches!(NdsRom::from_bytes(tiny_rom(b"ABCD")), Err(RomError::NotExplorersOfSky(_))));
    }
}
