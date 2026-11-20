//! The game's text font (`FONT/kanji_rd.dat`).
//!
//! Format from SkyTemple's `graphics/fonts/font_dat` (GPL-3): a count, then
//! 28-byte entries: character code, table, width, bytes per row (unused),
//! and twelve rows of two bytes, one bit per pixel, lowest bit leftmost.
//! Table 0 holds the single-byte Western characters, table 0x81 a few of
//! two bytes (see `text::WIDE`).

use media::font::Font;

use super::nds::RomError;

const ENTRY_SIZE: usize = 0x1C;

pub fn parse(data: &[u8]) -> Result<Font, RomError> {
    let count = data.get(0..4).ok_or(RomError::Truncated)?;
    let count = u32::from_le_bytes([count[0], count[1], count[2], count[3]]) as usize;
    let entries = data.get(4..4 + count * ENTRY_SIZE).ok_or(RomError::Truncated)?;
    let mut font = Font::default();
    for e in entries.as_chunks::<ENTRY_SIZE>().0 {
        let rows = std::array::from_fn(|row| u16::from_le_bytes([e[4 + row * 2], e[5 + row * 2]]));
        font.add((e[1] as u16) << 8 | e[0] as u16, e[2] as usize, rows);
    }
    Ok(font)
}
