//! Species data (`BALANCE/monster.md`) and per-level growth
//! (`BALANCE/m_level.bin`).

use crate::bytes::offset_at;
use rules::dungeon::{LevelGain, Stats};

use super::nds::RomError;
use super::px::decompress_pkdpx;

const MD_ENTRY_SIZE: usize = 0x44;
const LEVEL_ENTRY_SIZE: usize = 12;
const MAX_LEVEL: usize = 100;

#[derive(Clone, Copy, Debug)]
pub struct Species {
    /// Shared by both gender entries of a species; indexes `m_level.bin`.
    pub entity_id: u16,
    /// The national Pokedex number (which sprites of other projects go by).
    pub dex: u16,
    pub type1: u8,
    pub type2: u8,
    pub exp_yield: u16,
    /// Chances out of 1000 that it asks to join the team once defeated;
    /// negative for those that never do.
    pub recruit_rate: i16,
    /// Index of the species' sprites in `MONSTER/monster.bin` and
    /// `MONSTER/m_attack.bin`; it is not the entity id.
    pub sprite_index: u16,
    /// How large its shadow on the ground is: 0 to 2.
    pub shadow_size: u8,
    /// What terrain the species can cross: 0 normal, 2 hovering, 3
    /// through walls, 4 lava, 5 water.
    pub movement_type: u8,
    /// Chance out of 100 that the species appears asleep.
    pub spawn_asleep_chance: u8,
    /// Half of the species' natural regeneration speed.
    pub hp_regeneration: u8,
    pub base_hp: u16,
    pub base_attack: u8,
    pub base_sp_attack: u8,
    pub base_defense: u8,
    pub base_sp_defense: u8,
    /// Its two abilities (see `dungeon::ability`).
    pub abilities: [u8; 2],
    /// Its IQ group (see `dungeon::iq`).
    pub iq_group: u8,
    /// The entry's gender (`enum monster_gender`: 1 male, 2 female, 3
    /// genderless).
    pub gender: u8,
    /// How heavy it is, as Low Kick and Grass Knot's damage multiplier (8
    /// fraction bits: `GetWeightMultiplier`).
    pub weight: u16,
    /// Its family (`GetFamilyIndex`), which exclusive items go by.
    pub family: u16,
    /// How it evolves: from what, by which method (`town::method`) and
    /// with what.
    pub evolution: rules::town::Evolution,
}

pub struct MonsterData {
    species: Vec<Species>,
    /// Decompressed growth tables: 100 levels of
    /// `exp: u32, hp: u16, atk, sp_atk, def, sp_def: u8, pad: u16`.
    growth: Vec<Vec<u8>>,
}

impl MonsterData {
    pub fn parse(md: &[u8], m_level: &[u8]) -> Result<Self, RomError> {
        if md.get(0..4) != Some(b"MD\0\0") {
            return Err(RomError::Truncated);
        }
        let count = offset_at(md, 4)?;
        let mut species = Vec::with_capacity(count);
        for i in 0..count {
            let off = 8 + i * MD_ENTRY_SIZE;
            let e = md.get(off..off + MD_ENTRY_SIZE).ok_or(RomError::Truncated)?;
            species.push(Species {
                entity_id: u16::from_le_bytes([e[0x00], e[0x01]]),
                dex: u16::from_le_bytes([e[0x04], e[0x05]]),
                sprite_index: u16::from_le_bytes([e[0x10], e[0x11]]),
                shadow_size: e[0x2E],
                abilities: [e[0x18], e[0x19]],
                iq_group: e[0x17],
                gender: e[0x12],
                weight: u16::from_le_bytes([e[0x28], e[0x29]]),
                family: u16::from_le_bytes([e[0x32], e[0x33]]),
                evolution: rules::town::Evolution {
                    from: u16::from_le_bytes([e[0x08], e[0x09]]),
                    method: e[0x0A],
                    param: u16::from_le_bytes([e[0x0C], e[0x0D]]),
                    extra: u16::from_le_bytes([e[0x0E], e[0x0F]]),
                },
                type1: e[0x14],
                type2: e[0x15],
                movement_type: e[0x16],
                exp_yield: u16::from_le_bytes([e[0x1C], e[0x1D]]),
                recruit_rate: i16::from_le_bytes([e[0x1E], e[0x1F]]),
                base_hp: u16::from_le_bytes([e[0x20], e[0x21]]),
                base_attack: e[0x24],
                base_sp_attack: e[0x25],
                base_defense: e[0x26],
                base_sp_defense: e[0x27],
                spawn_asleep_chance: e[0x2F],
                hp_regeneration: e[0x30],
            });
        }

        // Pack file: count, then (offset, length) pairs of SIR0-wrapped PKDPX.
        let tables = offset_at(m_level, 4)?;
        let mut growth = Vec::with_capacity(tables);
        for i in 0..tables {
            let (off, len) = (offset_at(m_level, 8 + i * 8)?, offset_at(m_level, 12 + i * 8)?);
            let sir0 = m_level.get(off..off + len).ok_or(RomError::Truncated)?;
            let content = offset_at(sir0, 4)?;
            let table = decompress_pkdpx(sir0.get(content..).ok_or(RomError::Truncated)?)?;
            if table.len() != MAX_LEVEL * LEVEL_ENTRY_SIZE {
                return Err(RomError::Truncated);
            }
            growth.push(table);
        }
        Ok(Self { species, growth })
    }

    pub fn species_count(&self) -> usize {
        self.species.len()
    }

    pub fn species(&self, md_index: usize) -> Option<&Species> {
        self.species.get(md_index)
    }

    /// What each level takes and gives a species, level 1 first.
    pub fn growth_table(&self, md_index: usize) -> Vec<LevelGain> {
        let table = self.species.get(md_index).and_then(|species| self.growth.get(species.entity_id as usize));
        let Some(table) = table else {
            return Vec::new();
        };
        table
            .as_chunks::<LEVEL_ENTRY_SIZE>()
            .0
            .iter()
            .map(|e| LevelGain {
                total_exp: u32::from_le_bytes([e[0], e[1], e[2], e[3]]) as i32,
                hp: u16::from_le_bytes([e[4], e[5]]) as i32,
                attack: e[6] as i32,
                sp_attack: e[7] as i32,
                defense: e[8] as i32,
                sp_defense: e[9] as i32,
            })
            .collect()
    }

    /// Stats of a freshly spawned monster: base stats plus every level-up
    /// gain up to `level`.
    pub fn stats_at(&self, md_index: usize, level: u8) -> Option<Stats> {
        let s = self.species.get(md_index)?;
        let table = self.growth.get(s.entity_id as usize)?;
        let (mut hp, mut attack, mut defense) = (s.base_hp as i32, s.base_attack as i32, s.base_defense as i32);
        let (mut sp_attack, mut sp_defense) = (s.base_sp_attack as i32, s.base_sp_defense as i32);
        for l in 1..(level as usize).min(MAX_LEVEL) {
            let e = &table[l * LEVEL_ENTRY_SIZE..(l + 1) * LEVEL_ENTRY_SIZE];
            hp += u16::from_le_bytes([e[4], e[5]]) as i32;
            attack += e[6] as i32;
            sp_attack += e[7] as i32;
            defense += e[8] as i32;
            sp_defense += e[9] as i32;
        }
        Some(Stats {
            level: level as i32,
            max_hp: hp,
            hp,
            attack,
            defense,
            sp_attack,
            sp_defense,
            types: (s.type1, s.type2),
            abilities: s.abilities,
            held: 0,
            iq: rules::dungeon::iq::STARTING_IQ,
            iq_group: s.iq_group,
            iq_skills: 0,
            female: s.gender == 2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_regeneration_speed_is_the_documented_one() {
        let rom = crate::nds::test_rom();
        let monsters = MonsterData::parse(rom.file("BALANCE/monster.md").unwrap(), rom.file("BALANCE/m_level.bin").unwrap()).unwrap();
        // Bulbasaur walks, Squirtle (7) swims, Gastly (92) goes through
        // walls, Charizard (6) hovers.
        let movement = |index: usize| monsters.species(index).unwrap().movement_type;
        println!("movement types: {:?}", [1, 6, 7, 92, 126, 130].map(movement));
        assert_eq!((movement(1), movement(7), movement(92)), (0, 5, 3));
        // Sprites have their own numbering: both genders of a species may
        // share one or not.
        let sprite = |index: usize| monsters.species(index).unwrap().sprite_index;
        assert_eq!((sprite(1), sprite(2), sprite(25), sprite(625)), (0, 1, 26, 25));
        // pmdsky-debug: 8 most of the time.
        let asleep = |index: usize| monsters.species(index).unwrap().spawn_asleep_chance;
        println!("asleep chances: {:?}", [1, 25, 143, 185].map(asleep));
        assert_eq!(asleep(1), 8);
        // pmdsky-debug: always 0x64.
        assert_eq!(monsters.species(1).unwrap().hp_regeneration, 0x64);
        assert_eq!(monsters.species(25).unwrap().hp_regeneration, 0x64);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_stats_grow_with_level() {
        let rom = crate::nds::test_rom();
        let data = MonsterData::parse(rom.file("BALANCE/monster.md").unwrap(), rom.file("BALANCE/m_level.bin").unwrap()).unwrap();
        assert!(data.species_count() > 1000);
        // Level 1 is free, every level asks for more, and the gains are
        // the ones `stats_at` adds up.
        let table = data.growth_table(1);
        println!("Bulbasaur: {:?} ... {:?}", &table[..3], table[99]);
        assert_eq!((table.len(), table[0].total_exp), (100, 0));
        assert!(table.windows(2).all(|pair| pair[0].total_exp < pair[1].total_exp));
        let hp: i32 = table[1..10].iter().map(|gain| gain.hp).sum();
        let (low, high) = (data.stats_at(1, 1).unwrap(), data.stats_at(1, 10).unwrap());
        assert_eq!(high.max_hp - low.max_hp, hp);
        let yields = [1, 3, 25, 150, 425, 483].map(|index| data.species(index).unwrap().exp_yield);
        println!("exp yields: {yields:?}");
        assert!(yields.iter().all(|value| (1..1000).contains(value)));
        for md_index in [1, 25, 425] {
            let low = data.stats_at(md_index, 1).unwrap();
            let high = data.stats_at(md_index, 50).unwrap();
            assert_eq!(low.max_hp, data.species(md_index).unwrap().base_hp as i32);
            assert!(high.max_hp > low.max_hp && high.attack > low.attack);
        }
    }
}
