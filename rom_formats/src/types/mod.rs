//! Type matchups and balance constants, read from the game's code overlay 10.
//!
//! Addresses come from the pmdsky-debug project (`TYPE_MATCHUP_TABLE`, the
//! `MATCHUP_*_MULTIPLIER` constants and a few others) and differ per region.

mod chart;
pub use chart::*;
mod symbols;
use symbols::*;
mod tables;
pub use tables::*;
mod constants;
pub use constants::*;

use rules::dungeon::status::{CHARGED_MOVES, STAGE_COUNT};
use rules::dungeon::{Constants, TYPE_COUNT, TypeChart};

use super::nds::{NdsRom, RomError};
use crate::sym;

const OVERLAY_ID: u32 = 10;

struct Addresses {
    table: u32,
    /// `TYPE_MATCHUP_COMBINATOR_TABLE` (overlay 10).
    combinator: u32,
    /// `NATURAL_LOG_VALUE_TABLE` (ARM9).
    ln_table: u32,
    /// Indexed by matchup: immune, not very effective, neutral, super effective.
    multipliers: [u32; 4],
    /// The same, for an attacker with the Erratic Player IQ skill
    /// (`MATCHUP_*_MULTIPLIER_ERRATIC_PLAYER`).
    erratic: [u32; 4],
}

impl Addresses {
    /// In one release (see `NdsRom::region`).
    fn of(region: usize) -> Self {
        let at = |symbol: sym::Symbol| symbol.address[region];
        Self {
            table: at(sym::TYPE_MATCHUP_TABLE),
            combinator: at(sym::TYPE_MATCHUP_COMBINATOR_TABLE),
            ln_table: at(sym::NATURAL_LOG_VALUE_TABLE),
            multipliers: [
                sym::MATCHUP_IMMUNE_MULTIPLIER,
                sym::MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER,
                sym::MATCHUP_NEUTRAL_MULTIPLIER,
                sym::MATCHUP_SUPER_EFFECTIVE_MULTIPLIER,
            ]
            .map(at),
            erratic: [
                sym::MATCHUP_IMMUNE_MULTIPLIER_ERRATIC_PLAYER,
                sym::MATCHUP_NOT_VERY_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER,
                sym::MATCHUP_NEUTRAL_MULTIPLIER_ERRATIC_PLAYER,
                sym::MATCHUP_SUPER_EFFECTIVE_MULTIPLIER_ERRATIC_PLAYER,
            ]
            .map(at),
        }
    }
}

/// `count` little-endian values of `size` bytes at `address` of the ARM9
/// binary, signed.
/// 18 rows of 18, from a table read row by row.
fn square18(values: Vec<i32>) -> [[i32; 18]; 18] {
    std::array::from_fn(|row| std::array::from_fn(|column| values.get(row * 18 + column).copied().unwrap_or(0)))
}

fn arm9_values(rom: &NdsRom, address: u32, count: usize, size: usize) -> Result<Vec<i32>, RomError> {
    let (base, arm9) = rom.arm9().ok_or(RomError::Truncated)?;
    let off = address.checked_sub(base).ok_or(RomError::Truncated)? as usize;
    let table = arm9.get(off..off + count * size).ok_or(RomError::Truncated)?;
    Ok(table
        .chunks_exact(size)
        .map(|b| match size {
            1 => b[0] as i32,
            2 => i16::from_le_bytes([b[0], b[1]]) as i32,
            _ => i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        })
        .collect())
}

#[cfg(test)]
mod tests;
