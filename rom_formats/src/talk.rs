//! What team members say by personality (`MESSAGE/tbl_talk.tlk`): in a
//! dungeon when healthy, at half life, in a pinch, on levelling up, while
//! waiting, and in town.
//!
//! A file is a list of 16-bit offsets, six per talk group (one per kind of
//! dialogue, each where its list ends; the first offset is where the lists
//! start); the lists are 16-bit dialogue numbers within that kind's block
//! of the text files. After them, one byte per monster: its talk group,
//! then 0x24 more for the story's own characters (the partners, Grovyle,
//! Chatot...). Layout from skytemple-files (`data/tbl_talk`, GPL-3).

use super::nds::RomError;
use crate::bytes::u16_le;

/// The kinds of dialogue, in the file's order.
pub const KINDS: [&str; 6] = ["healthy", "half_life", "pinch", "level_up", "wait", "ground_wait"];
/// The story's own characters, after the monsters.
pub const SPECIAL_COUNT: usize = 0x24;

/// Where each kind's block of strings starts in the text files, by region
/// (EU, NA, JP) and in `KINDS` order (skytemple-files' `pmd2data.xml`,
/// "Pokemon HEALTHY Dialogue"...).
const BLOCKS: [[usize; 6]; 3] = [[4630, 5740, 6085, 3940, 4285, 6430], [4628, 5738, 6083, 3938, 4283, 6428], [9367, 10477, 10822, 11167, 11512, 11857]];

pub struct Talk {
    /// By talk group, by kind (`KINDS`): the dialogue numbers.
    pub groups: Vec<[Vec<u16>; 6]>,
    /// Each monster's talk group, by its number in `monster.md`.
    pub monsters: Vec<u8>,
    /// The talk group of each of the story's own characters.
    pub special: Vec<u8>,
}

impl Talk {
    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        let mut last = u16_le(data, 0)? as usize;
        let group_count = (last / 2).saturating_sub(1) / KINDS.len();
        let mut groups = Vec::with_capacity(group_count);
        for group in 0..group_count {
            let mut kinds: [Vec<u16>; 6] = Default::default();
            for (kind, list) in kinds.iter_mut().enumerate() {
                let next = u16_le(data, 2 + (group * KINDS.len() + kind) * 2)? as usize;
                while last < next {
                    list.push(u16_le(data, last)?);
                    last += 2;
                }
            }
            groups.push(kinds);
        }
        let mut monsters = data.get(last..).ok_or(RomError::Truncated)?.to_vec();
        let special = monsters.split_off(monsters.len().saturating_sub(SPECIAL_COUNT));
        Ok(Self { groups, monsters, special })
    }
}

/// The index in the text files of dialogue `number` of a kind (`KINDS`).
pub fn string_index(region: usize, kind: usize, number: u16) -> Option<usize> {
    Some(BLOCKS.get(region)?.get(kind)? + number as usize)
}
