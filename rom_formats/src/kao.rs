//! Portraits (`FONT/kaomado.kao`): for each monster, up to 40 pictures of
//! 40x40 pixels, one per emotion and its mirrored variant.
//!
//! Layout as read by skytemple-files (`graphics/kao`): a table of 40
//! pointers per monster (the first monster is empty; negative pointers
//! stand for missing pictures); a picture is a 16-colour palette (3 bytes
//! each) followed by AT4PX-compressed 4-bit pixels, in 8x8 tiles, low
//! nibble first.

use super::px::decompress_at4px;

pub const SIZE: usize = 40;
const PER_MONSTER: usize = 40;
const PALETTE_BYTES: usize = 48;

/// Picture `sub` (emotion times two, plus one for the mirrored variant) of
/// monster `index`, as RGBA.
pub fn portrait(kao: &[u8], index: usize, sub: usize) -> Option<Vec<u8>> {
    if sub >= PER_MONSTER {
        return None;
    }
    let entry = (index * PER_MONSTER + sub) * 4;
    let pointer = i32::from_le_bytes(kao.get(entry..entry + 4)?.try_into().ok()?);
    let start = usize::try_from(pointer).ok().filter(|&start| start > 0)?;
    let palette = kao.get(start..start + PALETTE_BYTES)?;
    let pixels = decompress_at4px(kao.get(start + PALETTE_BYTES..)?).ok()?;
    let mut rgba = vec![0; SIZE * SIZE * 4];
    for at in 0..SIZE * SIZE {
        let byte = *pixels.get(at / 2)?;
        let color = (if at % 2 == 0 { byte & 0xF } else { byte >> 4 }) as usize;
        // Tiles follow each other; pixels follow each other inside a tile.
        let (tile, inside) = (at / 64, at % 64);
        let (x, y) = (tile % 5 * 8 + inside % 8, tile / 5 * 8 + inside / 8);
        let out = (y * SIZE + x) * 4;
        rgba[out..out + 3].copy_from_slice(&palette[color * 3..color * 3 + 3]);
        rgba[out + 3] = 255;
    }
    Some(rgba)
}

/// The picture mirrored left to right.
pub fn mirrored(rgba: &[u8]) -> Vec<u8> {
    let mut flipped = vec![0; rgba.len()];
    for (row, out) in rgba.as_chunks::<{ SIZE * 4 }>().0.iter().zip(flipped.as_chunks_mut::<{ SIZE * 4 }>().0.iter_mut()) {
        for (pixel, to) in row.as_chunks::<4>().0.iter().rev().zip(out.as_chunks_mut::<4>().0.iter_mut()) {
            to.copy_from_slice(pixel);
        }
    }
    flipped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirroring_swaps_the_sides() {
        let mut picture = vec![0; SIZE * SIZE * 4];
        picture[0] = 9;
        let flipped = mirrored(&picture);
        assert_eq!(flipped[(SIZE - 1) * 4], 9);
        assert_eq!(mirrored(&flipped), picture);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`
    /// (`extractor dump portraits <rom> <folder>` writes a few as raw RGBA).
    #[test]
    #[ignore]
    fn real_rom_portraits_decode() {
        let rom = crate::nds::test_rom();
        let kao = rom.file("FONT/kaomado.kao").unwrap();
        // The first few hundred monsters all have a normal portrait.
        let with = (1..=400).filter(|&index| portrait(kao, index, 0).is_some()).count();
        println!("{with} of the first 400 monsters have a portrait");
        assert!(with > 380);
        assert!(portrait(kao, 0, 0).is_none());
        let picture = portrait(kao, 25, 0).unwrap();
        // Not a flat picture.
        let colors: std::collections::HashSet<&[u8; 4]> = picture.as_chunks::<4>().0.iter().collect();
        assert!(colors.len() > 8);
    }
}
