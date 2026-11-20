//! Dungeon tilesets, from the `DUNGEON/dungeon.bin` pack.
//!
//! For tileset `i` of 170: DMA (terrain + neighbours -> chunk) is entry
//! `170 + i`, DPC (chunks of 3x3 tiles) `340 + i`, DPCI (8x8 4bpp tiles)
//! `510 + i` and DPL (12 palettes of 16 colours) `680 + i`; its animated
//! palettes (DPLA) are entry `i` (see [`Tileset::cycles`]).
//!
//! "Tilesets" 170 and on are whole pictures (the floors of bosses): for
//! picture `i` of 29, DPC is entry `879 + i`, DPCI `908 + i`, the chunk
//! of each place of a 32x32 grid (DBG, compressed in SIR0) `937 + i` and
//! DPL `966 + i` (SkyTemple's `pmd2dungeondata.xml`).

use crate::bytes::offset_at;
use crate::sym;
use media::art_files::Cycle;

use super::nds::RomError;
use super::px::decompress_at4px;

pub const TILESET_COUNT: usize = 170;
/// Chunks are 3x3 tiles of 8x8 pixels.
pub(crate) const CHUNK_PIXELS: usize = 24;

const TILE_BYTES: usize = 32;
const CHUNK_BYTES: usize = 18;
const PALETTE_COUNT: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Wall = 0,
    /// Water, lava, chasm...
    Secondary = 1,
    Floor = 2,
}

/// Neighbours holding the same terrain, as DMA lookup bits.
pub mod neighbour {
    pub(crate) const SOUTH: u8 = 0x01;
    pub(crate) const SOUTH_EAST: u8 = 0x02;
    pub(crate) const EAST: u8 = 0x04;
    pub(crate) const NORTH_EAST: u8 = 0x08;
    pub(crate) const NORTH: u8 = 0x10;
    pub(crate) const NORTH_WEST: u8 = 0x20;
    pub(crate) const WEST: u8 = 0x40;
    pub(crate) const SOUTH_WEST: u8 = 0x80;
}

pub struct Tileset {
    palettes: Vec<[[u8; 3]; 16]>,
    /// Raw 4bpp tiles.
    tiles: Vec<u8>,
    /// Raw chunk mappings: 9 little-endian u16 per chunk.
    chunks: Vec<u8>,
    dma: Vec<u8>,
    /// The palettes that change with time: which, how long each of their
    /// steps lasts, their colours at each step.
    cycles: Vec<Cycle>,
}

impl Tileset {
    pub fn load(dungeon_bin: &[u8], index: usize) -> Result<Self, RomError> {
        if index >= TILESET_COUNT {
            return Err(RomError::Truncated);
        }
        let dma_sir0 = pack_entry(dungeon_bin, TILESET_COUNT + index)?;
        let dma_content = offset_at(dma_sir0, 4)?;
        let dma = decompress_at4px(dma_sir0.get(dma_content..).ok_or(RomError::Truncated)?)?;
        let chunks = decompress_at4px(pack_entry(dungeon_bin, 2 * TILESET_COUNT + index)?)?;
        let tiles = decompress_at4px(pack_entry(dungeon_bin, 3 * TILESET_COUNT + index)?)?;
        let dpl = pack_entry(dungeon_bin, 4 * TILESET_COUNT + index)?;
        if dma.len() < 3 * 0x300 || dpl.len() < PALETTE_COUNT * 16 * 4 {
            return Err(RomError::Truncated);
        }
        let palettes: Vec<[[u8; 3]; 16]> = (0..PALETTE_COUNT)
            .map(|p| {
                std::array::from_fn(|c| {
                    let o = (p * 16 + c) * 4;
                    [dpl[o], dpl[o + 1], dpl[o + 2]]
                })
            })
            .collect();
        let cycles = palette_cycles(pack_entry(dungeon_bin, index)?, &palettes);
        Ok(Self { palettes, tiles, chunks, dma, cycles })
    }

    /// A whole picture of a floor (tileset `TILESET_COUNT + index`): the
    /// tileset its chunks are drawn from, and the chunk at each place of
    /// its 32x32 grid, row by row.
    pub fn load_picture(dungeon_bin: &[u8], index: usize) -> Result<(Self, Vec<u16>), RomError> {
        const PICTURES: usize = 29;
        const FIRST: usize = 5 * TILESET_COUNT + PICTURES;
        if index >= PICTURES {
            return Err(RomError::Truncated);
        }
        let chunks = decompress_at4px(pack_entry(dungeon_bin, FIRST + index)?)?;
        let tiles = decompress_at4px(pack_entry(dungeon_bin, FIRST + PICTURES + index)?)?;
        let dbg_sir0 = pack_entry(dungeon_bin, FIRST + 2 * PICTURES + index)?;
        let dbg_content = offset_at(dbg_sir0, 4)?;
        let layout = super::px::decompress_pkdpx(dbg_sir0.get(dbg_content..).ok_or(RomError::Truncated)?)?;
        let dpl = pack_entry(dungeon_bin, FIRST + 3 * PICTURES + index)?;
        let palettes = (0..(dpl.len() / 64).clamp(1, 16))
            .map(|p| {
                std::array::from_fn(|c| {
                    let o = (p * 16 + c) * 4;
                    [dpl[o], dpl[o + 1], dpl[o + 2]]
                })
            })
            .collect();
        let layout = layout.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
        Ok((Self { palettes, tiles, chunks, dma: Vec::new(), cycles: Vec::new() }, layout))
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len() / CHUNK_BYTES
    }

    /// Chunk shown for `terrain` given which neighbours share it;
    /// `variation` (0..3) picks among the interchangeable looks.
    pub fn chunk_for(&self, terrain: Terrain, neighbours: u8, variation: usize) -> usize {
        self.dma[terrain as usize * 0x300 + neighbours as usize * 3 + variation % 3] as usize
    }

    /// The whole lookup table, indexed `terrain * 768 + neighbours * 3 + variation`.
    pub fn chunk_table(&self) -> Vec<u16> {
        self.dma[..3 * 0x300].iter().map(|&c| c as u16).collect()
    }

    /// A grid of terrain drawn as RGBA pixels, as the game draws a floor:
    /// `wide` cells a row, each chunk chosen by the cells around it.
    pub fn draw_terrain(&self, cells: &[Terrain], wide: usize) -> (Vec<u8>, Vec<u8>) {
        let high = cells.len() / wide.max(1);
        let mut rgba = vec![0u8; wide * high * CHUNK_PIXELS * CHUNK_PIXELS * 4];
        let mut indices = vec![0u8; wide * high * CHUNK_PIXELS * CHUNK_PIXELS];
        let mut drawn = std::collections::HashMap::new();
        for (at, &terrain) in cells.iter().enumerate() {
            let chunk = self.chunk_for(terrain, neighbours_of(cells, wide, at), 0);
            let (picture, colours) = drawn.entry(chunk).or_insert_with(|| self.chunk_pixels(chunk));
            for row in 0..CHUNK_PIXELS {
                let start = (at / wide * CHUNK_PIXELS + row) * wide * CHUNK_PIXELS + at % wide * CHUNK_PIXELS;
                rgba[start * 4..(start + CHUNK_PIXELS) * 4].copy_from_slice(&picture[row * CHUNK_PIXELS * 4..(row + 1) * CHUNK_PIXELS * 4]);
                indices[start..start + CHUNK_PIXELS].copy_from_slice(&colours[row * CHUNK_PIXELS..(row + 1) * CHUNK_PIXELS]);
            }
        }
        (rgba, indices)
    }

    /// The palettes that change with time: which, how long a step lasts,
    /// the colours at each step.
    pub fn cycles(&self) -> &[Cycle] {
        &self.cycles
    }

    /// 24x24 RGBA pixels of a chunk (colour 0 of a palette is transparent).
    pub fn render_chunk(&self, chunk: usize) -> Vec<u8> {
        self.chunk_pixels(chunk).0
    }

    /// Which palette and colour each of a chunk's 24x24 pixels shows
    /// (`palette << 4 | colour`): what the palettes that change recolour.
    pub fn chunk_colours(&self, chunk: usize) -> Vec<u8> {
        self.chunk_pixels(chunk).1
    }

    /// The same, and which palette and colour each pixel shows
    /// (`palette << 4 | colour`).
    fn chunk_pixels(&self, chunk: usize) -> (Vec<u8>, Vec<u8>) {
        let mut rgba = vec![0u8; CHUNK_PIXELS * CHUNK_PIXELS * 4];
        let mut indices = vec![0u8; CHUNK_PIXELS * CHUNK_PIXELS];
        let Some(mapping) = self.chunks.get(chunk * CHUNK_BYTES..(chunk + 1) * CHUNK_BYTES) else {
            return (rgba, indices);
        };
        for cell in 0..9 {
            let entry = u16::from_le_bytes([mapping[cell * 2], mapping[cell * 2 + 1]]);
            let tile = (entry & 0x3FF) as usize;
            let (flip_x, flip_y) = (entry & 0x400 != 0, entry & 0x800 != 0);
            let number = (entry >> 12) as usize % self.palettes.len();
            let palette = &self.palettes[number];
            let Some(pixels) = self.tiles.get(tile * TILE_BYTES..(tile + 1) * TILE_BYTES) else {
                continue;
            };
            for y in 0..8 {
                for x in 0..8 {
                    let byte = pixels[y * 4 + x / 2];
                    let colour = if x % 2 == 0 { byte & 0xF } else { byte >> 4 } as usize;
                    if colour == 0 {
                        continue;
                    }
                    let px = cell % 3 * 8 + if flip_x { 7 - x } else { x };
                    let py = cell / 3 * 8 + if flip_y { 7 - y } else { y };
                    let o = (py * CHUNK_PIXELS + px) * 4;
                    rgba[o..o + 3].copy_from_slice(&palette[colour]);
                    rgba[o + 3] = 255;
                    indices[py * CHUNK_PIXELS + px] = (number << 4 | colour) as u8;
                }
            }
        }
        (rgba, indices)
    }
}

/// The palettes of a tileset that change with time, from its DPLA: a SIR0
/// whose list points to 16 colours for palette 10, then 16 for palette 11,
/// each its duration and its colours one after the other. (Each colour has
/// its own time in the game: here the shortest, as SkyTemple draws it, at
/// least 16 frames.) Layout from skytemple-files (`graphics/dpla`, GPL-3).
fn palette_cycles(dpla: &[u8], palettes: &[[[u8; 3]; 16]]) -> Vec<Cycle> {
    let word = |at: usize| dpla.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let half = |at: usize| dpla.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let (Some(start), Some(end)) = (word(4), word(8)) else {
        return Vec::new();
    };
    let entry = |at: usize| -> Option<(usize, Vec<[u8; 3]>)> {
        let place = word(start + at * 4)?;
        let (count, duration) = (half(place)?, half(place + 2)?);
        let colours = dpla.get(place + 4..place + 4 + count * 4)?;
        Some((duration, colours.as_chunks::<4>().0.iter().map(|c| [c[0], c[1], c[2]]).collect()))
    };
    let entries: Vec<_> = (0..(end.saturating_sub(start) / 4).min(32)).map_while(entry).collect();
    let mut cycles = Vec::new();
    for (place, first) in [(10, 0), (11, 16)] {
        let own = entries.get(first..entries.len().min(first + 16)).unwrap_or(&[]);
        if own.first().is_none_or(|entry| entry.1.is_empty()) {
            continue;
        }
        let steps = own.iter().map(|entry| entry.1.len()).max().unwrap_or(0);
        let ticks = own.iter().map(|entry| entry.0).filter(|&ticks| ticks > 0).min().unwrap_or(16).max(16) as u16;
        let base = palettes[place];
        let colours_at = |step: usize| {
            std::array::from_fn(|colour| own.get(colour).filter(|entry| !entry.1.is_empty()).map_or(base[colour], |entry| entry.1[step % entry.1.len()]))
        };
        cycles.push((place, ticks, (0..steps).map(colours_at).collect()));
    }
    cycles
}

/// Which neighbours of a cell of a grid share its terrain, as DMA lookup
/// bits. Past the grid is solid, as the game takes it; a floor shares with
/// all that is not wall (SkyTemple's `get_tile_neighbors`).
fn neighbours_of(cells: &[Terrain], wide: usize, at: usize) -> u8 {
    use neighbour::*;
    let high = (cells.len() / wide.max(1)) as isize;
    let (x, y, terrain) = ((at % wide) as isize, (at / wide) as isize, cells[at]);
    let same = |(dx, dy): (isize, isize)| {
        let (x, y) = (x + dx, y + dy);
        let there = (x >= 0 && y >= 0 && x < wide as isize && y < high).then(|| cells[y as usize * wide + x as usize]);
        match (terrain, there) {
            (Terrain::Floor, there) => there.is_some_and(|there| there != Terrain::Wall),
            (_, None) => true,
            (_, Some(there)) => there == terrain,
        }
    };
    let around = [
        ((0, 1), SOUTH),
        ((1, 1), SOUTH_EAST),
        ((1, 0), EAST),
        ((1, -1), NORTH_EAST),
        ((0, -1), NORTH),
        ((-1, -1), NORTH_WEST),
        ((-1, 0), WEST),
        ((-1, 1), SOUTH_WEST),
    ];
    around.into_iter().filter(|&(place, _)| same(place)).fold(0, |bits, (_, bit)| bits | bit)
}

/// Pack entry holding the trap and stairs tiles.
const TRAP_GRAPHICS_ENTRY: usize = 1033;
/// Tile of the stairs in that entry (27 and 28 are the two stair pictures).
// Known gap: which of the two is "down" was judged by eye.
pub const STAIRS_TILE: usize = 27;

/// The 24x24 trap tiles as RGBA pixels, indexed by trap id; the tiles after
/// the traps are the stairs and other special floor tiles.
///
/// SIR0 container whose header gives the tile data, the tile count, the
/// palettes and the colour count. Tiles are 4bpp, made of 8x8 tiles in row
/// order like the tileset graphics.
// Known gap: every tile is drawn with the first of the two palettes.
/// `SECONDARY_TERRAIN_TYPES` (in the ARM9 binary) has one byte per tileset,
/// more entries than there are tilesets with graphics.
const SECONDARY_TERRAIN_ENTRIES: u8 = 200;

/// The secondary terrain of a tileset: water (0), lava (1) or chasm (2).
pub fn secondary_terrain_type(rom: &super::nds::NdsRom, tileset: u8) -> Option<u8> {
    let address = sym::SECONDARY_TERRAIN_TYPES.address[rom.region().ok()?];
    let (base, arm9) = rom.arm9()?;
    if tileset >= SECONDARY_TERRAIN_ENTRIES {
        return None;
    }
    let offset = address.checked_sub(base)? as usize + tileset as usize;
    arm9.get(offset).copied().filter(|&kind| kind <= 2)
}

/// `TILESET_PROPERTIES` and `NATURE_POWER_TABLE` in overlay 10: 12-byte
/// `tileset_property` (Camouflage's type at 6, Nature Power's variant at
/// 8), 8-byte `wildcard_move_desc` (the move first).
const TILESET_PROPERTY_ENTRIES: u8 = 199;

/// What Camouflage turns its user into on floors of this tileset (a type),
/// and what Nature Power becomes there (a move).
pub fn tileset_moves(rom: &super::nds::NdsRom, tileset: u8) -> Option<(u8, u16, u8)> {
    let region = rom.region().ok()?;
    let at = |table: &sym::Symbol| Some(table.address[region]);
    let (base, overlay) = rom.overlay(10)?;
    if tileset >= TILESET_PROPERTY_ENTRIES {
        return None;
    }
    let property = at(&sym::TILESET_PROPERTIES)?.checked_sub(base)? as usize + tileset as usize * 12;
    let kind = *overlay.get(property + 6)?;
    // (What Secret Power does there: `DoMoveSecretPower` reads this byte.)
    let secret_power = *overlay.get(property + 5)?;
    let variant = u16::from_le_bytes([*overlay.get(property + 8)?, *overlay.get(property + 9)?]) as usize;
    let nature = at(&sym::NATURE_POWER_TABLE)?.checked_sub(base)? as usize + variant.min(14) * 8;
    let nature_power = u16::from_le_bytes([*overlay.get(nature)?, *overlay.get(nature + 1)?]);
    Some((kind, nature_power, secret_power))
}

/// Where the weather colour maps are in `dungeon.bin`, depending on the
/// region.
const COLOR_MAP_ENTRIES: [usize; 2] = [1034, 1022];
const COLOR_MAP_BYTES: usize = 0x400;

/// The colour maps of each weather (clear first): 256 RGBX entries each,
/// through which every colour channel of the floor is looked up.
pub fn load_weather_colors(dungeon_bin: &[u8]) -> Result<Vec<Vec<u8>>, RomError> {
    for entry in COLOR_MAP_ENTRIES {
        let Ok(data) = pack_entry(dungeon_bin, entry) else { continue };
        if data.get(0..4) != Some(b"SIR0") {
            continue;
        }
        // The maps fill the file's content, up to its pointer list.
        let read = |off: usize| data.get(off..off + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        let (Some(content), Some(end)) = (read(4), read(8)) else { continue };
        let Some(maps) = data.get(content..end) else { continue };
        let maps: Vec<Vec<u8>> = maps.as_chunks::<COLOR_MAP_BYTES>().0.iter().map(|map| map.to_vec()).collect();
        // Eight weathers at least, and every fourth byte is a filler.
        let filler = |map: &Vec<u8>| map.as_chunks::<4>().0.iter().all(|entry| entry[3] == 0xFF);
        if maps.len() >= 8 && maps.iter().take(8).all(filler) {
            return Ok(maps);
        }
    }
    Err(RomError::Truncated)
}

pub fn load_trap_graphics(dungeon_bin: &[u8]) -> Result<Vec<Vec<u8>>, RomError> {
    let data = pack_entry(dungeon_bin, TRAP_GRAPHICS_ENTRY)?;
    if data.get(0..4) != Some(b"SIR0") {
        return Err(RomError::Truncated);
    }
    let header = offset_at(data, 4)?;
    let (tiles, count, palette) = (offset_at(data, header)?, offset_at(data, header + 4)?, offset_at(data, header + 8)?);
    const BYTES: usize = CHUNK_PIXELS * CHUNK_PIXELS / 2;
    let colours = data.get(palette..palette + 16 * 4).ok_or(RomError::Truncated)?;
    let mut pictures = Vec::with_capacity(count);
    for index in 0..count {
        let start = tiles + index * BYTES;
        let pixels = data.get(start..start + BYTES).ok_or(RomError::Truncated)?;
        let mut rgba = vec![0u8; CHUNK_PIXELS * CHUNK_PIXELS * 4];
        for y in 0..CHUNK_PIXELS {
            for x in 0..CHUNK_PIXELS {
                let byte = pixels[(y / 8 * 3 + x / 8) * TILE_BYTES + y % 8 * 4 + x % 8 / 2];
                let colour = if x % 2 == 0 { byte & 0xF } else { byte >> 4 } as usize;
                if colour != 0 {
                    let o = (y * CHUNK_PIXELS + x) * 4;
                    rgba[o..o + 3].copy_from_slice(&colours[colour * 4..colour * 4 + 3]);
                    rgba[o + 3] = 255;
                }
            }
        }
        pictures.push(rgba);
    }
    Ok(pictures)
}

/// Entry `index` of a pack file (`count` at 4, then `(offset, length)` pairs).
pub fn pack_entry(pack: &[u8], index: usize) -> Result<&[u8], RomError> {
    if index >= offset_at(pack, 4)? {
        return Err(RomError::Truncated);
    }
    let (off, len) = (offset_at(pack, 8 + index * 8)?, offset_at(pack, 12 + index * 8)?);
    pack.get(off..off + len).ok_or(RomError::Truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_secondary_terrain_types_are_known() {
        let rom = crate::nds::test_rom();
        let kinds: Vec<u8> = (0..200).map(|tileset| secondary_terrain_type(&rom, tileset).unwrap()).collect();
        // Beach Cave has water; the game also has lava and chasm tilesets.
        assert_eq!(kinds[1], 0);
        assert!(kinds.contains(&1) && kinds.contains(&2));
        assert_eq!(secondary_terrain_type(&rom, 200), None);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_weather_colors_load() {
        let rom = crate::nds::test_rom();
        let maps = load_weather_colors(rom.file("DUNGEON/dungeon.bin").unwrap()).unwrap();
        println!("{} colour maps", maps.len());
        // Clear weather only rounds colours to the console's 5 bits per
        // channel; rain darkens them and turns them blue; a sandstorm
        // turns them yellow.
        let clear = &maps[0];
        let near = |v: usize, c: usize| (clear[v * 4 + c] as i32 - v as i32).abs() <= 7;
        assert!((0..256).all(|v| (0..3).all(|c| near(v, c))));
        let (rain, sand) = (&maps[4], &maps[2]);
        assert!(rain[128 * 4] < 128 && rain[128 * 4 + 2] > rain[128 * 4]);
        assert!(sand[255 * 4 + 2] < sand[255 * 4]);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_trap_graphics_cover_every_trap_and_the_stairs() {
        let rom = crate::nds::test_rom();
        let pictures = load_trap_graphics(rom.file("DUNGEON/dungeon.bin").unwrap()).unwrap();
        // 25 trap kinds, then the stairs.
        assert!(pictures.len() > STAIRS_TILE);
        let opaque = |p: &Vec<u8>| p.as_chunks::<4>().0.iter().filter(|px| px[3] != 0).count();
        for (kind, picture) in pictures.iter().enumerate().take(25).skip(1) {
            assert!(opaque(picture) > 100, "trap {kind}");
        }
        assert!(opaque(&pictures[STAIRS_TILE]) > 100);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_tilesets_all_load() {
        let rom = crate::nds::test_rom();
        let pack = rom.file("DUNGEON/dungeon.bin").unwrap();
        for index in 0..TILESET_COUNT {
            let set = Tileset::load(pack, index).unwrap();
            assert!(set.chunk_count() > 0, "tileset {index}");
            assert_eq!(set.tiles.len() % TILE_BYTES, 0, "tileset {index}");
            // Every chunk the DMA can name must exist.
            for terrain in [Terrain::Wall, Terrain::Secondary, Terrain::Floor] {
                for n in 0..=255u8 {
                    for v in 0..3 {
                        assert!(set.chunk_for(terrain, n, v) < set.chunk_count(), "tileset {index}");
                    }
                }
            }
        }
    }
}
