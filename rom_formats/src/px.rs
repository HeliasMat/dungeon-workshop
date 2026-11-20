//! "PX" compression and its two containers, PKDPX and AT4PX, used all over
//! the ROM.

use super::nds::RomError;

/// Decompresses a PKDPX container found at the start of `data`.
pub(crate) fn decompress_pkdpx(data: &[u8]) -> Result<Vec<u8>, RomError> {
    if data.get(0..5) != Some(b"PKDPX") || data.len() < 0x14 {
        return Err(RomError::Truncated);
    }
    let container_len = u16::from_le_bytes([data[5], data[6]]) as usize;
    let out_len = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
    let input = data.get(0x14..container_len).ok_or(RomError::Truncated)?;
    decompress(input, &data[7..16], out_len)
}

/// Decompresses an AT4PX container found at the start of `data`.
pub(crate) fn decompress_at4px(data: &[u8]) -> Result<Vec<u8>, RomError> {
    if data.get(0..5) != Some(b"AT4PX") || data.len() < 0x12 {
        return Err(RomError::Truncated);
    }
    let container_len = u16::from_le_bytes([data[5], data[6]]) as usize;
    let out_len = u16::from_le_bytes([data[16], data[17]]) as usize;
    let input = data.get(0x12..container_len).ok_or(RomError::Truncated)?;
    decompress(input, &data[7..16], out_len)
}

fn decompress(input: &[u8], flags: &[u8], out_len: usize) -> Result<Vec<u8>, RomError> {
    let mut out = Vec::with_capacity(out_len);
    let mut pos = 0;
    let next = |pos: &mut usize| {
        let b = input.get(*pos).copied().ok_or(RomError::Truncated);
        *pos += 1;
        b
    };
    'blocks: while out.len() < out_len {
        let command = next(&mut pos)?;
        for bit in (0..8).rev() {
            if out.len() >= out_len {
                break 'blocks;
            }
            if command >> bit & 1 == 1 {
                out.push(next(&mut pos)?);
                continue;
            }
            let b = next(&mut pos)?;
            let (high, low) = (b >> 4, b & 0xF);
            if let Some(idx) = flags.iter().position(|&f| f == high) {
                out.extend_from_slice(&nibble_pattern(idx, low));
            } else {
                // Back-reference into the last 0x1000 bytes written.
                let distance = 0x1000 - ((low as usize) << 8 | next(&mut pos)? as usize);
                let start = out.len().checked_sub(distance).ok_or(RomError::Truncated)?;
                for i in 0..high as usize + 3 {
                    out.push(out[start + i]);
                }
            }
        }
    }
    out.truncate(out_len);
    Ok(out)
}

/// Two bytes made of four nibbles that are all equal, or all equal but one.
fn nibble_pattern(idx: usize, low: u8) -> [u8; 2] {
    let mut base = low;
    match idx {
        1 => base = base.wrapping_add(1),
        5 => base = base.wrapping_sub(1),
        _ => {}
    }
    let mut n = [base; 4];
    match idx {
        0 => {}
        1..=4 => n[idx - 1] = n[idx - 1].wrapping_sub(1),
        _ => n[idx - 5] = n[idx - 5].wrapping_add(1),
    }
    [(n[0] & 0xF) << 4 | (n[1] & 0xF), (n[2] & 0xF) << 4 | (n[3] & 0xF)]
}
