//! The type chart.

use super::*;

pub fn load_type_chart(rom: &NdsRom) -> Result<TypeChart, RomError> {
    let addresses = Addresses::of(rom.region()?);
    let (base, overlay) = rom.overlay(OVERLAY_ID).ok_or(RomError::Truncated)?;
    let at = |address: u32, len: usize| {
        let off = address.checked_sub(base).ok_or(RomError::Truncated)? as usize;
        overlay.get(off..off + len).ok_or(RomError::Truncated)
    };

    // 18 rows (attacking type) of 18 16-bit matchups (target type).
    let table = at(addresses.table, TYPE_COUNT * TYPE_COUNT * 2)?;
    let mut matchups = [[0u8; TYPE_COUNT]; TYPE_COUNT];
    for (i, entry) in table.as_chunks::<2>().0.iter().enumerate() {
        let value = u16::from_le_bytes([entry[0], entry[1]]);
        if value > 3 {
            return Err(RomError::Truncated);
        }
        matchups[i / TYPE_COUNT][i % TYPE_COUNT] = value as u8;
    }

    // Fixed-point numbers with 8 fraction bits.
    let mut multipliers = [0i32; 4];
    for (multiplier, address) in multipliers.iter_mut().zip(addresses.multipliers) {
        let b = at(address, 4)?;
        *multiplier = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    }

    let mut erratic = [0i32; 4];
    for (multiplier, address) in erratic.iter_mut().zip(addresses.erratic) {
        let b = at(address, 4)?;
        *multiplier = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    }

    // 4x4 32-bit matchups.
    let table = at(addresses.combinator, 16 * 4)?;
    let mut combinator = [[0u8; 4]; 4];
    for (i, entry) in table.as_chunks::<4>().0.iter().enumerate() {
        let value = u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]);
        if value > 3 {
            return Err(RomError::Truncated);
        }
        combinator[i / 4][i % 4] = value as u8;
    }

    // 2048 16-bit values.
    let (arm9_base, arm9) = rom.arm9().ok_or(RomError::Truncated)?;
    let off = addresses.ln_table.checked_sub(arm9_base).ok_or(RomError::Truncated)? as usize;
    let table = arm9.get(off..off + 0x800 * 2).ok_or(RomError::Truncated)?;
    let ln_table = table.as_chunks::<2>().0.iter().map(|entry| i16::from_le_bytes([entry[0], entry[1]])).collect();
    Ok(TypeChart { matchups, multipliers, erratic, combinator, ln_table })
}
