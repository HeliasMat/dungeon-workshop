//! The smaller tables of the code.

use super::*;

pub fn load_recruit_boosts(rom: &NdsRom) -> Result<Vec<i32>, RomError> {
    let region = rom.region()?;
    let (base, overlay) = rom.overlay(OVERLAY_ID).ok_or(RomError::Truncated)?;
    let off = RECRUIT_LEVEL_BOOSTS[region].checked_sub(base).ok_or(RomError::Truncated)? as usize;
    let table = overlay.get(off..off + 102 * 2).ok_or(RomError::Truncated)?;
    Ok(table.as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes([b[0], b[1]]) as i32).collect())
}

/// The exclusive items' tables: who each is for (`BALANCE/item_s_p.bin`:
/// after its 16-byte header, a kind, a padding byte and a parameter each),
/// their effects and stat boosts (arm9).
pub fn load_exclusive_items(rom: &NdsRom) -> Result<rules::dungeon::exclusive::ExclusiveItems, RomError> {
    use rules::dungeon::exclusive::{END, FIRST};
    let region = rom.region()?;
    let count = (END - FIRST) as usize;
    let file = rom.file("BALANCE/item_s_p.bin").ok_or(RomError::Truncated)?;
    let kinds = (0..count).map_while(|at| file.get(16 + at * 4..20 + at * 4).map(|b| (b[0], i16::from_le_bytes([b[2], b[3]])))).collect();
    let effects = arm9_values(rom, EXCLUSIVE_ITEM_EFFECT_DATA[region], count * 2, 1)?;
    let boosts = arm9_values(rom, EXCLUSIVE_ITEM_STAT_BOOST_DATA[region], 15 * 4, 1)?;
    Ok(rules::dungeon::exclusive::ExclusiveItems {
        kinds,
        effects: effects.as_chunks::<2>().0.iter().map(|pair| (pair[0] as u8, pair[1] as u8)).collect(),
        boosts: boosts.as_chunks::<4>().0.iter().map(|row| [row[0] as i8, row[1] as i8, row[2] as i8, row[3] as i8]).collect(),
    })
}

pub fn load_sentry_visitors(rom: &NdsRom) -> Result<Vec<u16>, RomError> {
    Ok(arm9_values(rom, SENTRY_DUTY_MONSTER_IDS[rom.region()?], 102, 2)?.into_iter().map(|id| id as u16).filter(|&id| id != 0).collect())
}

/// The jobs' tables (see `JobTables`): the ranks by `RANK_UP_TABLE`, whose
/// 16-byte entries are a name, the points to go past it, the storage's room
/// and an item (at 0, 4, 8, 12); the floors' ranks by `MISSION_FLOOR_RANKS`,
/// a pointer a group of floors, to a rank a floor of the group (0 first).
/// `dungeons`: the group, first floor and number of floors of each.
pub fn load_job_tables(rom: &NdsRom, names: &[String], dungeons: &[(u8, u8, u8)]) -> Result<rules::jobs::JobTables, RomError> {
    use rules::jobs::{ExplorerRank, JobTables};
    const RANKS: usize = 13;
    const GROUPS: usize = 100;
    let region = rom.region()?;
    let ranks = arm9_values(rom, RANK_UP_TABLE[region], RANKS * 4, 4)?;
    let ranks = ranks.as_chunks::<4>().0.iter().enumerate().map(|(at, entry)| ExplorerRank {
        name: names.get(at).cloned().unwrap_or_default(),
        points: entry[1] as u32,
        storage: entry[2] as u32,
        prize: (entry[3] & 0xFFFF) as u16,
    });
    let weights = |address: u32| arm9_values(rom, address, 7, 2).map(|weights| std::array::from_fn(|at| weights[at] as u16));
    let pointers = arm9_values(rom, MISSION_FLOOR_RANKS[region], GROUPS, 4)?;
    let (base, arm9) = rom.arm9().ok_or(RomError::Truncated)?;
    let floor_rank = |group: usize, floor: usize| {
        let list = (*pointers.get(group)? as u32).checked_sub(base)? as usize;
        arm9.get(list + floor).copied()
    };
    let floors = |&(group, first, count): &(u8, u8, u8)| -> Vec<u8> {
        (0..count as usize).map(|floor| floor_rank(group as usize, first as usize + floor + 1).unwrap_or(0)).collect()
    };
    Ok(JobTables {
        ranks: ranks.collect(),
        rank_points: arm9_values(rom, MISSION_RANK_POINTS[region], 16, 4)?.iter().map(|&points| points as u32).collect(),
        floor_ranks: dungeons.iter().map(floors).collect(),
        rewards: [weights(REWARD_WEIGHTS[region])?, weights(OUTLAW_REWARD_WEIGHTS[region])?],
    })
}

pub fn load_recycle_trades(rom: &NdsRom) -> Result<Vec<rules::town::RecycleTrade>, RomError> {
    let bytes = arm9_values(rom, RECYCLE_SHOP_ITEM_LIST[rom.region()?], 24 * 36, 1)?;
    Ok(rules::town::RecycleTrade::parse(&bytes.iter().map(|&b| b as u8).collect::<Vec<u8>>()))
}

pub fn load_iq_tables(rom: &NdsRom) -> Result<rules::dungeon::iq::IqTables, RomError> {
    use rules::dungeon::iq::{SKILLS, TYPES};
    let region = rom.region()?;
    let square = |address: u32| -> Result<Vec<Vec<i16>>, RomError> {
        let values = arm9_values(rom, address, TYPES * TYPES, 2)?;
        Ok(values.chunks(TYPES).map(|row| row.iter().map(|&v| v as i16).collect()).collect())
    };
    Ok(rules::dungeon::iq::IqTables {
        required: arm9_values(rom, IQ_SKILLS[region], SKILLS, 4)?,
        groups: arm9_values(rom, IQ_GROUP_SKILLS[region], 16 * 25, 1)?.chunks(25).map(|group| group.iter().map(|&skill| skill as u8).collect()).collect(),
        restrictions: arm9_values(rom, IQ_SKILL_RESTRICTIONS[region], SKILLS, 2)?.iter().map(|&v| v as i16).collect(),
        gummi_iq: square(IQ_GUMMI_GAIN[region])?,
        gummi_belly: square(GUMMI_BELLY_RESTORE[region])?,
        wonder_iq: arm9_values(rom, WONDER_GUMMI_IQ[region], 1, 2)?[0],
    })
}

pub fn load_money_table(rom: &NdsRom) -> Result<Vec<i32>, RomError> {
    let region = match rom.game_code().as_bytes().get(3) {
        Some(b'P') => 0,
        Some(b'E') => 1,
        Some(b'J') => 2,
        _ => return Err(RomError::Truncated),
    };
    let (base, arm9) = rom.arm9().ok_or(RomError::Truncated)?;
    let off = MONEY_QUANTITY_TABLE[region].checked_sub(base).ok_or(RomError::Truncated)? as usize;
    let table = arm9.get(off..off + 100 * 4).ok_or(RomError::Truncated)?;
    Ok(table.as_chunks::<4>().0.iter().map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect())
}

pub fn load_episode_characters(rom: &NdsRom) -> Result<Vec<u16>, RomError> {
    let region = match rom.game_code().as_bytes().get(3) {
        Some(b'P') => 0,
        Some(b'E') => 1,
        Some(b'J') => 2,
        _ => return Err(RomError::Truncated),
    };
    let (base, arm9) = rom.arm9().ok_or(RomError::Truncated)?;
    let off = SPECIAL_EPISODE_MAIN_CHARACTERS[region].checked_sub(base).ok_or(RomError::Truncated)? as usize;
    let table = arm9.get(off..off + 100 * 2).ok_or(RomError::Truncated)?;
    Ok(table.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes([b[0], b[1]])).collect())
}
