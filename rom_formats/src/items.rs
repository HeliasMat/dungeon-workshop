//! `BALANCE/item_p.bin`: item data (16-byte entries in a SIR0 container:
//! buying price, selling price, category, picture... as skytemple-files
//! reads them), and the item tables shops stock from.

use super::nds::RomError;
use crate::bytes::offset_at;

const ENTRY_SIZE: usize = 16;

/// Entry of `DUNGEON/dungeon.bin` holding the item pictures.
const ITEM_GRAPHICS_ENTRY: usize = 1022;
/// Items are 16x16 pixels, 4bpp.
pub(crate) const ITEM_PIXELS: usize = 16;
const SPRITE_BYTES: usize = ITEM_PIXELS * ITEM_PIXELS / 2;

pub struct ItemData {
    /// What each item costs, and what it sells for.
    prices: Vec<(u16, u16)>,
    categories: Vec<u8>,
    /// Picture and palette of each item in the item graphics.
    sprites: Vec<(u8, u8)>,
    /// The move an item uses (an orb's), by item (SkyTemple's `item_p`:
    /// the 16-bit value at 8).
    moves: Vec<u16>,
    /// The fewest and the most of a thrown item a pile holds, by item
    /// (`quantity_limit`, the bytes at 10 and 11).
    piles: Vec<(u8, u8)>,
}

impl ItemData {
    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        if data.get(0..4) != Some(b"SIR0") {
            return Err(RomError::Truncated);
        }
        // The entries sit between the content pointer and the SIR0 pointer list.
        let (start, end) = (offset_at(data, 4)?, offset_at(data, 8)?);
        let table = data.get(start..end).ok_or(RomError::Truncated)?;
        let price = |e: &[u8; ENTRY_SIZE]| (u16::from_le_bytes([e[0], e[1]]), u16::from_le_bytes([e[2], e[3]]));
        Ok(Self {
            prices: table.as_chunks::<ENTRY_SIZE>().0.iter().map(price).collect(),
            categories: table.as_chunks::<ENTRY_SIZE>().0.iter().map(|e| e[4]).collect(),
            sprites: table.as_chunks::<ENTRY_SIZE>().0.iter().map(|e| (e[5], e[0xC])).collect(),
            moves: table.as_chunks::<ENTRY_SIZE>().0.iter().map(|e| u16::from_le_bytes([e[8], e[9]])).collect(),
            piles: table.as_chunks::<ENTRY_SIZE>().0.iter().map(|e| (e[10], e[11])).collect(),
        })
    }

    pub fn item_count(&self) -> usize {
        self.categories.len()
    }

    /// `(buying price, selling price)` of an item.
    pub fn prices(&self, item_id: usize) -> Option<(u16, u16)> {
        self.prices.get(item_id).copied()
    }

    /// The fewest and the most a pile of the item holds.
    pub fn pile(&self, item_id: usize) -> (u8, u8) {
        self.piles.get(item_id).copied().unwrap_or((1, 1))
    }

    /// The move an item uses (0: none).
    pub fn item_move(&self, item_id: usize) -> u16 {
        self.moves.get(item_id).copied().unwrap_or(0)
    }

    pub fn category(&self, item_id: usize) -> Option<u8> {
        self.categories.get(item_id).copied()
    }

    /// `(picture, palette)` of an item.
    pub fn sprite(&self, item_id: usize) -> Option<(u8, u8)> {
        self.sprites.get(item_id).copied()
    }
}

/// The item pictures: a SIR0 container whose header gives the picture data,
/// the picture count and the palettes (16 colours of 4 bytes each). Pictures
/// are made of 8x8 tiles in row order, like the tileset graphics.
pub struct ItemGraphics<'a> {
    pictures: &'a [u8],
    palettes: &'a [u8],
}

impl<'a> ItemGraphics<'a> {
    pub fn load(dungeon_bin: &'a [u8]) -> Result<Self, RomError> {
        let data = super::tileset::pack_entry(dungeon_bin, ITEM_GRAPHICS_ENTRY)?;
        if data.get(0..4) != Some(b"SIR0") {
            return Err(RomError::Truncated);
        }
        let header = offset_at(data, 4)?;
        let (pictures, count, palettes) = (offset_at(data, header)?, offset_at(data, header + 4)?, offset_at(data, header + 8)?);
        Ok(Self {
            pictures: data.get(pictures..pictures + count * SPRITE_BYTES).ok_or(RomError::Truncated)?,
            palettes: data.get(palettes..header).ok_or(RomError::Truncated)?,
        })
    }

    /// 16x16 RGBA pixels; `None` if the picture or palette does not exist.
    pub fn render(&self, picture: u8, palette: u8) -> Option<Vec<u8>> {
        let start = picture as usize * SPRITE_BYTES;
        let pixels = self.pictures.get(start..start + SPRITE_BYTES)?;
        let colours = self.palettes.get(palette as usize * 64..palette as usize * 64 + 64)?;
        let mut rgba = vec![0u8; ITEM_PIXELS * ITEM_PIXELS * 4];
        for y in 0..ITEM_PIXELS {
            for x in 0..ITEM_PIXELS {
                let byte = pixels[(y / 8 * 2 + x / 8) * 32 + y % 8 * 4 + x % 8 / 2];
                let colour = if x % 2 == 0 { byte & 0xF } else { byte >> 4 } as usize;
                if colour != 0 {
                    let o = (y * ITEM_PIXELS + x) * 4;
                    rgba[o..o + 3].copy_from_slice(&colours[colour * 4..colour * 4 + 3]);
                    rgba[o + 3] = 255;
                }
            }
        }
        Some(rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mappa;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    /// Cross-checks the categories against the floors' item lists: within a
    /// category, spawn weights are cumulative, so they must never decrease.
    #[test]
    #[ignore]
    fn real_rom_categories_match_item_lists() {
        let rom = crate::nds::test_rom();
        let items = ItemData::parse(rom.file("BALANCE/item_p.bin").unwrap()).unwrap();
        let mappa = Mappa::parse(rom.file("BALANCE/mappa_s.bin").unwrap()).unwrap();
        println!("{} items", items.item_count());
        assert!(items.item_count() >= 364);
        let (mut floors, mut bad) = (0, 0);
        for g in 0..mappa.group_count() {
            for floor in mappa.floors(g) {
                floors += 1;
                let mut last = [0u16; 16];
                for &(id, weight) in &floor.items.items {
                    let category = items.category(id as usize).unwrap() as usize;
                    assert!(category < 16, "item {id}");
                    if weight == u16::MAX {
                        continue;
                    }
                    if weight < last[category] {
                        bad += 1;
                        break;
                    }
                    last[category] = weight;
                }
            }
        }
        println!("{bad} of {floors} floors have decreasing weights inside a category");
        assert_eq!(bad, 0);

        // Every item that can lie on a floor has a picture with visible pixels.
        let graphics = ItemGraphics::load(rom.file("DUNGEON/dungeon.bin").unwrap()).unwrap();
        for id in 1..364 {
            let (picture, palette) = items.sprite(id).unwrap();
            let rgba = graphics.render(picture, palette);
            assert!(rgba.is_some_and(|p| p.as_chunks::<4>().0.iter().filter(|px| px[3] != 0).count() > 20), "item {id}: picture {picture}, palette {palette}");
        }
    }
}
