//! `BALANCE/waza_p.bin`: move data and per-species learnsets.
//!
//! SIR0 container; the header points to the move table (26-byte entries,
//! layout from pmdsky-debug's `move_data`) and to the learnset pointer table
//! (three pointers per species: level-up, TM/HM, egg).

use super::nds::RomError;
use crate::bytes::offset_at;

const MOVE_SIZE: usize = 26;
const LEARNSET_PTRS_SIZE: usize = 12;

#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub base_power: u16,
    pub type_id: u8,
    pub category: u8,
    /// Raw target/range bitfield, not decoded yet.
    pub target_range: u16,
    /// The same bitfield as the AI sees it, with its condition in the third
    /// nibble.
    pub ai_target_range: u16,
    /// Weight of the move when the AI picks one at random.
    pub ai_weight: u8,
    /// Chance, in percent, that the AI accepts a target when the move's AI
    /// condition is "random".
    pub ai_chance: u8,
    pub pp: u8,
    pub accuracy: u8,
    pub accuracy2: u8,
    pub strikes: u8,
    pub crit_chance: u8,
    pub move_id: u16,
}

pub struct MoveData {
    moves: Vec<Move>,
    /// Per species (by entity id): `(move index, level)` in learning order.
    level_up: Vec<Vec<(u16, u8)>>,
    /// Per species: the moves its TMs and HMs can teach it.
    machines: Vec<Vec<u16>>,
}

impl MoveData {
    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        if data.get(0..4) != Some(b"SIR0") {
            return Err(RomError::Truncated);
        }
        let header = offset_at(data, 4)?;
        let move_table = offset_at(data, header)?;
        let learnsets = offset_at(data, header + 4)?;

        // Both tables are padded up to the next one.
        let moves = (0..(learnsets - move_table) / MOVE_SIZE)
            .map(|i| {
                let off = move_table + i * MOVE_SIZE;
                let b = &data[off..off + MOVE_SIZE];
                Move {
                    base_power: u16::from_le_bytes([b[0x00], b[0x01]]),
                    type_id: b[0x02],
                    category: b[0x03],
                    target_range: u16::from_le_bytes([b[0x04], b[0x05]]),
                    ai_target_range: u16::from_le_bytes([b[0x06], b[0x07]]),
                    ai_weight: b[0x09],
                    ai_chance: b[0x0C],
                    pp: b[0x08],
                    accuracy: b[0x0A],
                    accuracy2: b[0x0B],
                    strikes: b[0x0D],
                    crit_chance: b[0x0F],
                    move_id: u16::from_le_bytes([b[0x16], b[0x17]]),
                }
            })
            .collect();

        let mut level_up = Vec::new();
        let mut machines = Vec::new();
        for i in 0..(header - learnsets) / LEARNSET_PTRS_SIZE {
            let list = offset_at(data, learnsets + i * LEARNSET_PTRS_SIZE)?;
            // The dummy species 0 has null pointers.
            level_up.push(if list == 0 { Vec::new() } else { parse_level_up(data, list)? });
            let taught = offset_at(data, learnsets + i * LEARNSET_PTRS_SIZE + 4)?;
            machines.push(if taught == 0 { Vec::new() } else { parse_moves(data, taught)? });
        }
        Ok(Self { moves, level_up, machines })
    }

    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    pub fn get(&self, index: usize) -> Option<&Move> {
        self.moves.get(index)
    }

    /// The moves a monster learns by level: `(move, level)`. Learnsets are
    /// numbered like the first half of `BALANCE/monster.md` (one per
    /// monster and form), not by entity id.
    pub fn level_up(&self, monster: usize) -> &[(u16, u8)] {
        self.level_up.get(monster).map_or(&[], Vec::as_slice)
    }

    /// The moves TMs and HMs can teach a monster (numbered as
    /// [`Self::level_up`]).
    pub fn machines(&self, monster: usize) -> &[u16] {
        self.machines.get(monster).map_or(&[], Vec::as_slice)
    }

    /// The (up to four) most recent level-up moves known at `level`.
    // Used for the team; wild monsters draw theirs (`dungeon::wild_moves`).
    pub fn moves_at(&self, monster: usize, level: u8) -> Vec<u16> {
        let known: Vec<u16> = self.level_up(monster).iter().filter(|&&(_, l)| l <= level).map(|&(m, _)| m).collect();
        known[known.len().saturating_sub(4)..].to_vec()
    }
}

/// Pairs of variable-length integers `(move, level)`, closed by a zero byte.
fn parse_level_up(data: &[u8], mut pos: usize) -> Result<Vec<(u16, u8)>, RomError> {
    let mut list = Vec::new();
    loop {
        if *data.get(pos).ok_or(RomError::Truncated)? == 0 {
            return Ok(list);
        }
        let move_index = varint(data, &mut pos)?;
        let level = varint(data, &mut pos)?;
        list.push((move_index as u16, level as u8));
    }
}

/// Moves as variable-length integers, closed by a zero byte.
fn parse_moves(data: &[u8], mut pos: usize) -> Result<Vec<u16>, RomError> {
    let mut list = Vec::new();
    while *data.get(pos).ok_or(RomError::Truncated)? != 0 {
        list.push(varint(data, &mut pos)? as u16);
    }
    Ok(list)
}

/// Big-endian base-128 integer; the high bit marks "more bytes follow".
fn varint(data: &[u8], pos: &mut usize) -> Result<u32, RomError> {
    let mut value = 0u32;
    loop {
        let b = *data.get(*pos).ok_or(RomError::Truncated)?;
        *pos += 1;
        value = value << 7 | (b & 0x7F) as u32;
        if b & 0x80 == 0 {
            return Ok(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_spans_several_bytes() {
        let mut pos = 0;
        assert_eq!(varint(&[0x82, 0x05, 0x7F], &mut pos).unwrap(), 2 << 7 | 5);
        assert_eq!(pos, 2);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_regular_attack_is_a_plain_physical_hit() {
        let rom = crate::nds::test_rom();
        let moves = MoveData::parse(rom.file("BALANCE/waza_p.bin").unwrap()).unwrap();
        let regular = (0..moves.move_count()).filter_map(|index| moves.get(index)).find(|m| m.move_id == 355).unwrap();
        println!("regular attack: {regular:?}");
        // One strike, physical, typeless, with some power.
        assert_eq!((regular.strikes, regular.category, regular.type_id), (1, 0, 0));
        assert!((1..=20).contains(&regular.base_power));
        assert!(regular.accuracy >= 50);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_learnsets_are_consistent() {
        let rom = crate::nds::test_rom();
        let data = MoveData::parse(rom.file("BALANCE/waza_p.bin").unwrap()).unwrap();
        println!("{} moves, {} learnsets", data.move_count(), data.level_up.len());
        assert!(data.move_count() > 500);
        // Bulbasaur (1) learns Solarbeam (151) by TM, not Surf (219).
        println!("bulbasaur's TMs {:?}", data.machines(1));
        assert!(data.machines(1).contains(&151) && !data.machines(1).contains(&219));
        // What the AI reads: most moves can be drawn, self-heals are marked
        // "user, on the spot, at low HP", and the AI's idea of a move's
        // range mostly agrees with the real one.
        let all = || (0..data.move_count()).filter_map(|i| data.get(i));
        let weighted = all().filter(|m| m.ai_weight > 0).count();
        let heals = all().filter(|m| m.ai_target_range == 0x273).count();
        let agreeing = all().filter(|m| m.ai_target_range & 0xFF == m.target_range & 0xFF).count();
        let conditional = all().filter(|m| m.ai_target_range >> 8 & 0xF == 1).count();
        println!("{weighted} weighted, {heals} self-heals, {agreeing} agreeing, {conditional} random");
        assert!(weighted > data.move_count() / 2);
        assert!(heals > 0);
        assert!(agreeing > data.move_count() / 2);
        assert!(all().all(|m| m.ai_chance <= 100 && m.ai_target_range >> 8 <= 6));
        // The stored id matches the table index for most entries, but not
        // all (e.g. index 543 stores 414 in the EU ROM): look moves up by index.
        let matching = (0..data.move_count()).filter(|&i| data.get(i).unwrap().move_id as usize == i).count();
        println!("{matching} entries store their own index");
        assert!(matching > data.move_count() / 2);
        let mut non_empty = 0;
        let (mut unordered, mut out_of_range) = (Vec::new(), Vec::new());
        for (species, list) in data.level_up.iter().enumerate() {
            non_empty += !list.is_empty() as usize;
            if list.windows(2).any(|pair| pair[0].1 > pair[1].1) {
                unordered.push(species);
            }
            if list.iter().any(|&(m, l)| m as usize >= data.move_count() || !(1..=100).contains(&l)) {
                out_of_range.push(species);
            }
        }
        assert!(out_of_range.is_empty(), "out of range: {out_of_range:?}");
        assert!(unordered.is_empty(), "levels go backwards: {unordered:?}");
        assert!(non_empty > 400);
        assert!(data.moves_at(1, 5).len() <= 4 && !data.moves_at(1, 5).is_empty());
    }
}
