//! Backgrounds of the overworld ("ground mode") maps, in `MAP_BG/`.
//!
//! `bg_list.dat` names, for each map, a BPL (palettes), a BPC (8x8 tiles and
//! the 3x3-tile chunks built from them) and a BMA (which chunk goes where,
//! plus collision), and up to eight BPA (animated tiles, drawn frame by
//! frame into [`MapBackground::frames`]). The BPL's animated palettes come
//! out as [`PaletteCycle`]s. File layouts and compression schemes follow
//! the format notes of the SkyTemple project.

use super::nds::{NdsRom, RomError};
use crate::bytes::u16_le;

/// Chunks are 3x3 tiles of 8x8 pixels.
pub(crate) const CHUNK_PIXELS: usize = 24;
const TILE_BYTES: usize = 32;
const LIST_ENTRY: usize = 88;
/// Animation frames drawn at most per map (each is a full picture).
const MAX_FRAMES: usize = 8;
const DEFAULT_FRAME_TICKS: u16 = 10;

/// Names of the files making up one map, without their extension.
#[derive(Clone, Debug)]
pub struct MapEntry {
    pub bpl: String,
    pub bpc: String,
    pub bma: String,
    /// Animated tile files (BPA), by slot; empty where a slot is unused.
    pub bpas: Vec<String>,
}

/// Parses `MAP_BG/bg_list.dat`: 88-byte entries of 8-byte names (BPL, BPC,
/// BMA, then eight BPA slots).
pub fn map_list(data: &[u8]) -> Vec<MapEntry> {
    let name = |bytes: &[u8]| {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).to_lowercase()
    };
    data.as_chunks::<LIST_ENTRY>()
        .0
        .iter()
        .map(|e| MapEntry {
            bpl: name(&e[0..8]),
            bpc: name(&e[8..16]),
            bma: name(&e[16..24]),
            bpas: e[24..].as_chunks::<8>().0.iter().map(|bytes| name(bytes)).collect(),
        })
        .collect()
}

/// A palette of a map whose colours change with time: every `ticks`
/// 60ths of a second it takes the next of `colours`, round and round
/// (SkyTemple's `graphics/bpl`; timed on the original's personality test).
#[derive(Clone)]
pub struct PaletteCycle {
    pub palette: usize,
    pub ticks: u16,
    pub colours: Vec<[[u8; 3]; 16]>,
}

/// What a map is made of, tile by tile (see [`MapBackground::tile_entries`]).
pub struct TileEntries {
    /// In tiles of 8 pixels.
    pub wide: usize,
    pub high: usize,
    /// Each layer's tile entries, the lower first; 0 where none.
    pub entries: Vec<Vec<u16>>,
    /// Each layer's chunk number for each tile.
    pub chunks: Vec<Vec<u16>>,
}

/// A decoded map background.
pub struct MapBackground {
    /// In pixels.
    pub width: usize,
    pub height: usize,
    /// The map drawn once per animation frame of its animated tiles (a
    /// single picture for maps without any): its lower layer, which
    /// characters walk on.
    pub frames: Vec<Vec<u8>>,
    /// The same for the upper layer of the maps that have two, which
    /// characters walk under (the front of a tent, tree tops...): empty
    /// for the others.
    pub upper: Vec<Vec<u8>>,
    /// How long each frame is shown, in 1/60 s.
    pub frame_ticks: u16,
    /// The palettes that change with time; `frames` show their first
    /// colours.
    pub cycles: Vec<PaletteCycle>,
    /// For maps with such palettes, which palette and colour each pixel
    /// of each of `frames` shows: `palette << 4 | colour`, 0 where nothing
    /// is drawn.
    pub indices: Vec<Vec<u8>>,
    pub upper_indices: Vec<Vec<u8>>,
    /// For those maps too: the same for each layer alone (the lower
    /// first), in the first of `frames`; and every palette as it starts.
    pub layers: Vec<Vec<u8>>,
    pub palettes: Vec<[[u8; 3]; 16]>,
    /// One flag per 8x8 tile: true where the map is solid.
    pub collision: Vec<bool>,
    /// Size of the collision grid, in tiles.
    pub tiles_wide: usize,
    pub tiles_high: usize,
}

impl MapBackground {
    pub fn load(rom: &NdsRom, entry: &MapEntry) -> Result<Self, RomError> {
        let file = |name: &str, extension: &str| rom.file(&format!("MAP_BG/{name}.{extension}")).ok_or(RomError::Truncated);
        let (palettes, cycles) = parse_bpl(file(&entry.bpl, "bpl")?)?;
        // Animated tiles (BPA), by slot: tile count, frame count, 4 bytes
        // per frame (duration, unknown), then every frame's tiles.
        let bpa = |slot: usize| -> Option<(usize, usize, &[u8])> {
            let name = entry.bpas.get(slot).filter(|name| !name.is_empty())?;
            let data = file(name, "bpa").ok()?;
            let (tiles, frames) = (u16_le(data, 0).ok()? as usize, u16_le(data, 2).ok()? as usize);
            (frames > 0).then_some((tiles, frames, data))
        };
        let slots: Vec<_> = (0..8).map(bpa).collect();
        let frame_count = slots.iter().flatten().map(|&(_, frames, _)| frames).max().unwrap_or(1).min(MAX_FRAMES);
        let frame_ticks = slots.iter().flatten().find_map(|&(_, _, data)| u16_le(data, 4).ok()).filter(|&ticks| ticks > 0).unwrap_or(DEFAULT_FRAME_TICKS);
        let map = parse_bma(file(&entry.bma, "bma")?)?;
        let (width, height) = (map.chunks_wide * CHUNK_PIXELS, map.chunks_high * CHUNK_PIXELS);

        let mut frames = Vec::with_capacity(frame_count);
        let (mut upper, mut upper_indices) = (Vec::new(), Vec::new());
        let (mut indices, mut apart) = (Vec::new(), Vec::new());
        let two = map.layers.len() > 1;
        for frame in 0..frame_count {
            // Each file loops over its own frames.
            let animated = |slot: usize| -> Option<&[u8]> {
                let (tiles, count, data) = (*slots.get(slot)?)?;
                let start = 4 + count * 4 + (frame % count) * tiles * TILE_BYTES;
                data.get(start..start + tiles * TILE_BYTES)
            };
            let layers = parse_bpc(file(&entry.bpc, "bpc")?, &animated)?;
            let mut rgba = vec![0u8; width * height * 4];
            let mut index = (!cycles.is_empty()).then(|| vec![0u8; width * height]);
            let mut over = two.then(|| vec![0u8; width * height * 4]);
            let mut over_index = (two && !cycles.is_empty()).then(|| vec![0u8; width * height]);
            // The BMA lists the lower layer first; the BPC lists the upper
            // one first. Draw from the bottom up.
            for (depth, chunk_map) in map.layers.iter().enumerate() {
                let layer = layers.len().checked_sub(depth + 1).and_then(|i| layers.get(i));
                let Some(layer) = layer else { continue };
                let mut alone = (frame == 0 && !cycles.is_empty()).then(|| vec![0u8; width * height]);
                for (position, &chunk) in chunk_map.iter().enumerate() {
                    let (cx, cy) = (position % map.chunks_wide, position / map.chunks_wide);
                    // The lower layer on one picture, the other on its own.
                    let (picture, whole) = match (depth > 0, over.as_deref_mut()) {
                        (true, Some(over)) => (over, over_index.as_deref_mut()),
                        _ => (rgba.as_mut_slice(), index.as_deref_mut()),
                    };
                    layer.draw_chunk(chunk as usize, &palettes, picture, [whole, alone.as_deref_mut()], width, cx, cy);
                }
                apart.extend(alone);
            }
            frames.push(rgba);
            indices.extend(index);
            upper.extend(over);
            upper_indices.extend(over_index);
        }
        Ok(Self {
            width,
            height,
            frames,
            upper,
            frame_ticks,
            cycles,
            indices,
            upper_indices,
            layers: apart,
            palettes,
            collision: map.collision,
            tiles_wide: map.tiles_wide,
            tiles_high: map.tiles_high,
        })
    }

    /// The chunks of a map's lower layer and how many make a row: what a
    /// map drawn with a dungeon's tiles says of its terrain.
    pub fn terrain_chunks(rom: &NdsRom, entry: &MapEntry) -> Result<(Vec<u16>, usize), RomError> {
        let file = rom.file(&format!("MAP_BG/{}.bma", entry.bma)).ok_or(RomError::Truncated)?;
        let map = parse_bma(file)?;
        let lower = map.layers.into_iter().next().ok_or(RomError::Truncated)?;
        Ok((lower, map.chunks_wide))
    }

    /// What the map is made of, tile by tile, layer by layer (the lower
    /// first): each tile's entry as the chunks give it (its tile number in
    /// the low 10 bits, flipped across 0x400, up and down 0x800, its
    /// palette in the high 4), and the chunk it is part of. The ROM's own
    /// account of what is drawn where: a tile seen once is one thing's, a
    /// palette is one sort of thing's.
    pub fn tile_entries(rom: &NdsRom, entry: &MapEntry) -> Result<TileEntries, RomError> {
        let file = |name: &str, extension: &str| rom.file(&format!("MAP_BG/{name}.{extension}")).ok_or(RomError::Truncated);
        let map = parse_bma(file(&entry.bma, "bma")?)?;
        let layers = parse_bpc(file(&entry.bpc, "bpc")?, &|_| None)?;
        let (wide, high) = (map.chunks_wide * 3, map.chunks_high * 3);
        let mut entries = Vec::new();
        let mut chunks = Vec::new();
        for (depth, chunk_map) in map.layers.iter().enumerate() {
            let Some(layer) = layers.len().checked_sub(depth + 1).and_then(|at| layers.get(at)) else {
                continue;
            };
            let (mut said, mut of) = (vec![0u16; wide * high], vec![0u16; wide * high]);
            for (position, &chunk) in chunk_map.iter().enumerate() {
                let (cx, cy) = (position % map.chunks_wide, position / map.chunks_wide);
                let Some(mapping) = layer.mappings.get(chunk as usize * 9..chunk as usize * 9 + 9) else {
                    continue;
                };
                for (cell, &value) in mapping.iter().enumerate() {
                    let at = (cy * 3 + cell / 3) * wide + cx * 3 + cell % 3;
                    said[at] = value;
                    of[at] = chunk;
                }
            }
            entries.push(said);
            chunks.push(of);
        }
        Ok(TileEntries { wide, high, entries, chunks })
    }

    /// Picture `frame` of the map with both its layers, as a screen showing
    /// it all at once draws it.
    pub fn flat(&self, frame: usize) -> Option<Vec<u8>> {
        let mut rgba = self.frames.get(frame)?.clone();
        if let Some(upper) = self.upper.get(frame) {
            for (under, over) in rgba.as_chunks_mut::<4>().0.iter_mut().zip(upper.as_chunks::<4>().0.iter()) {
                if over[3] != 0 {
                    under.copy_from_slice(over);
                }
            }
        }
        Some(rgba)
    }

    /// Whether the 8x8 tile containing pixel `(x, y)` is solid (outside the
    /// map counts as solid).
    pub fn solid_at(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return true;
        }
        let (tx, ty) = (x as usize / 8, y as usize / 8);
        tx >= self.tiles_wide || ty >= self.tiles_high || self.collision.get(ty * self.tiles_wide + tx).copied().unwrap_or(true)
    }
}

/// A palette: its colours.
type Palette = [[u8; 3]; 16];

/// Palettes of 16 colours: 15 stored as RGBx, the first is transparent.
/// Those that change with time come with their first colours, and all of
/// them apart.
fn parse_bpl(data: &[u8]) -> Result<(Vec<Palette>, Vec<PaletteCycle>), RomError> {
    let count = u16_le(data, 0)? as usize;
    let mut palettes = Vec::with_capacity(count);
    for p in 0..count {
        let mut palette = [[0u8; 3]; 16];
        for c in 0..15 {
            let o = 4 + (p * 15 + c) * 4;
            let colour = data.get(o..o + 3).ok_or(RomError::Truncated)?;
            palette[c + 1] = [colour[0], colour[1], colour[2]];
        }
        palettes.push(palette);
    }
    // Animated palettes (SkyTemple's `graphics/bpl`): after the palettes,
    // one (frame duration, frame count) per palette, then every frame of
    // every animated palette, 15 colours each.
    let mut cycles = Vec::new();
    if u16_le(data, 2)? > 0 {
        let specs = 4 + count * 15 * 4;
        let frames = specs + count * 4;
        let mut used = 0;
        for (p, palette) in palettes.iter_mut().enumerate() {
            let ticks = u16_le(data, specs + p * 4)?;
            let frame_count = u16_le(data, specs + p * 4 + 2)? as usize;
            if frame_count == 0 {
                continue;
            }
            let mut colours = Vec::with_capacity(frame_count);
            for frame in used..used + frame_count {
                let mut shown = *palette;
                for c in 0..15 {
                    let o = frames + (frame * 15 + c) * 4;
                    if let Some(colour) = data.get(o..o + 3) {
                        shown[c + 1] = [colour[0], colour[1], colour[2]];
                    }
                }
                colours.push(shown);
            }
            *palette = colours[0];
            cycles.push(PaletteCycle { palette: p, ticks, colours });
            used += frame_count;
        }
    }
    Ok((palettes, cycles))
}

struct TileLayer {
    /// 4bpp tiles; tile 0 is the null tile.
    tiles: Vec<u8>,
    /// 16-bit tile mappings, 9 per chunk; chunk 0 is the null chunk.
    mappings: Vec<u16>,
}

impl TileLayer {
    fn draw_chunk(&self, chunk: usize, palettes: &[[[u8; 3]; 16]], rgba: &mut [u8], mut indices: [Option<&mut [u8]>; 2], width: usize, cx: usize, cy: usize) {
        let Some(mapping) = self.mappings.get(chunk * 9..chunk * 9 + 9) else {
            return;
        };
        for (cell, &entry) in mapping.iter().enumerate() {
            let tile = (entry & 0x3FF) as usize;
            let (flip_x, flip_y) = (entry & 0x400 != 0, entry & 0x800 != 0);
            // Tiles past the layer's own and its animated (BPA) ones: skipped.
            let Some(pixels) = self.tiles.get(tile * TILE_BYTES..(tile + 1) * TILE_BYTES) else {
                continue;
            };
            let Some(palette) = palettes.get((entry >> 12) as usize) else {
                continue;
            };
            for y in 0..8 {
                for x in 0..8 {
                    let byte = pixels[y * 4 + x / 2];
                    let colour = if x % 2 == 0 { byte & 0xF } else { byte >> 4 } as usize;
                    if colour == 0 {
                        continue;
                    }
                    let px = cx * CHUNK_PIXELS + cell % 3 * 8 + if flip_x { 7 - x } else { x };
                    let py = cy * CHUNK_PIXELS + cell / 3 * 8 + if flip_y { 7 - y } else { y };
                    let o = (py * width + px) * 4;
                    rgba[o..o + 3].copy_from_slice(&palette[colour]);
                    rgba[o + 3] = 255;
                    for indices in indices.iter_mut().flatten() {
                        indices[py * width + px] = ((entry >> 12) as u8) << 4 | colour as u8;
                    }
                }
            }
        }
    }
}

/// Header: pointers to the upper and (optional) lower layer data, then a
/// 12-byte spec per layer: tile count + 1, four BPA slot sizes, chunk
/// count + 1. Each layer's data is its compressed tiles, then (on an even offset)
/// its compressed tile mappings.
///
/// The animated tiles of the layer's four BPA slots (`animated(slot)`, slots
/// 0-3 for the first layer and 4-7 for the second) are numbered right after
/// its own tiles.
fn parse_bpc<'a>(data: &[u8], animated: &dyn Fn(usize) -> Option<&'a [u8]>) -> Result<Vec<TileLayer>, RomError> {
    let pointers = [u16_le(data, 0)? as usize, u16_le(data, 2)? as usize];
    let mut layers = Vec::new();
    for (index, &pointer) in pointers.iter().enumerate() {
        if pointer == 0 {
            continue;
        }
        let spec = 4 + index * 12;
        let tile_count = (u16_le(data, spec)? as usize).saturating_sub(1);
        let chunk_count = (u16_le(data, spec + 10)? as usize).saturating_sub(1);
        let compressed = data.get(pointer..).ok_or(RomError::Truncated)?;
        let (tile_data, used) = decompress_tiles(compressed, tile_count * TILE_BYTES)?;
        let mut tiles = vec![0u8; TILE_BYTES];
        tiles.extend_from_slice(&tile_data);
        for slot in 0..4 {
            let count = u16_le(data, spec + 2 + slot * 2)? as usize;
            let frame = animated(index * 4 + slot).unwrap_or(&[]);
            let end = tiles.len() + count * TILE_BYTES;
            tiles.extend_from_slice(&frame[..frame.len().min(count * TILE_BYTES)]);
            // A missing or short file still takes up its tile numbers.
            tiles.resize(end, 0);
        }
        let rest = compressed.get((used + 1) & !1..).ok_or(RomError::Truncated)?;
        let mut mappings = vec![0u16; 9];
        mappings.extend(decompress_mappings(rest, chunk_count * 9)?);
        layers.push(TileLayer { tiles, mappings });
    }
    Ok(layers)
}

/// "BPC image" compression: copy commands (below 0x80) take bytes from the
/// input, pattern commands repeat one byte. 0x80.. loads a new pattern,
/// 0xC0.. reuses the current one, 0xE0.. swaps it with the previous one.
/// The last value of each range, and 0x7E/0x7F, read the length from the
/// input instead. Every command writes its length plus one bytes.
fn decompress_tiles(input: &[u8], size: usize) -> Result<(Vec<u8>, usize), RomError> {
    let mut out = Vec::with_capacity(size);
    let mut pos = 0;
    let next = |pos: &mut usize| {
        let b = input.get(*pos).copied().ok_or(RomError::Truncated);
        *pos += 1;
        b
    };
    let (mut pattern, mut previous) = (0u8, 0u8);
    while out.len() < size {
        let command = next(&mut pos)?;
        let length = match command {
            0x7F => next(&mut pos)? as usize | (next(&mut pos)? as usize) << 8,
            0x7E | 0xBF | 0xDF | 0xFF => next(&mut pos)? as usize,
            0x00..=0x7D => command as usize,
            0x80..=0xBE => (command - 0x80) as usize,
            0xC0..=0xDE => (command - 0xC0) as usize,
            0xE0..=0xFE => (command - 0xE0) as usize,
        } + 1;
        match command {
            0x00..=0x7F => {
                for _ in 0..length {
                    out.push(next(&mut pos)?);
                }
                continue;
            }
            0x80..=0xBF => {
                previous = pattern;
                pattern = next(&mut pos)?;
            }
            0xC0..=0xDF => {}
            0xE0..=0xFF => std::mem::swap(&mut pattern, &mut previous),
        }
        out.resize(out.len() + length, pattern);
    }
    out.truncate(size);
    Ok((out, pos))
}

/// "BPC tilemap" compression, in two passes over the same output: the first
/// writes the high byte of every 16-bit mapping, the second the low byte.
/// Commands below 0x80 write zeros (pass 1) or skip (pass 2), 0x80..0xBF
/// repeat one byte, 0xC0.. copy bytes; all run for their low bits + 1.
fn decompress_mappings(input: &[u8], count: usize) -> Result<Vec<u16>, RomError> {
    let mut out = vec![0u16; count];
    let mut pos = 0;
    let next = |pos: &mut usize| {
        let b = input.get(*pos).copied().ok_or(RomError::Truncated);
        *pos += 1;
        b
    };
    for shift in [8, 0] {
        let mut index = 0;
        while index < count {
            let command = next(&mut pos)?;
            let (run, mode) = match command {
                0x00..=0x7F => (command as usize + 1, 0),
                0x80..=0xBF => ((command - 0x80) as usize + 1, 1),
                0xC0..=0xFF => ((command - 0xC0) as usize + 1, 2),
            };
            let repeated = if mode == 1 { next(&mut pos)? } else { 0 };
            for _ in 0..run {
                let value = match mode {
                    0 => 0,
                    1 => repeated,
                    _ => next(&mut pos)?,
                };
                if let Some(entry) = out.get_mut(index) {
                    *entry |= (value as u16) << shift;
                }
                index += 1;
            }
        }
    }
    Ok(out)
}

struct ChunkMap {
    chunks_wide: usize,
    chunks_high: usize,
    tiles_wide: usize,
    tiles_high: usize,
    /// Chunk index of every position, lower layer first.
    layers: Vec<Vec<u16>>,
    collision: Vec<bool>,
}

/// 12-byte header (map size in tiles and in chunks, layer count, data layer
/// flag, collision layer count), then the layers and the collision, each
/// run-length compressed and XORed row against row.
fn parse_bma(data: &[u8]) -> Result<ChunkMap, RomError> {
    let header = data.get(0..12).ok_or(RomError::Truncated)?;
    let (tiles_wide, tiles_high) = (header[0] as usize, header[1] as usize);
    let (chunks_wide, chunks_high) = (header[4] as usize, header[5] as usize);
    let layer_count = u16_le(data, 6)? as usize;
    let has_data_layer = u16_le(data, 8)? != 0;
    let collision_layers = u16_le(data, 10)?;
    let mut pos = 12;

    // Rows are stored with an even number of chunks.
    let stored_wide = (chunks_wide + 1) & !1;
    let mut layers = Vec::new();
    for _ in 0..layer_count.min(2) {
        let values = decompress_chunk_map(data, &mut pos, stored_wide * chunks_high)?;
        let mut layer = vec![0u16; chunks_wide * chunks_high];
        let mut above = vec![0u16; stored_wide];
        for (y, row) in values.chunks_exact(stored_wide).enumerate() {
            for (x, &value) in row.iter().enumerate() {
                above[x] ^= value;
                if x < chunks_wide {
                    layer[y * chunks_wide + x] = above[x];
                }
            }
        }
        layers.push(layer);
    }

    // One byte per tile, purpose unknown: skipped.
    if has_data_layer {
        let mut written = 0;
        while written < tiles_wide * tiles_high {
            let command = *data.get(pos).ok_or(RomError::Truncated)?;
            pos += 1;
            // Same three command ranges as the chunk maps, on single bytes.
            match command {
                0x00..=0x7F => written += command as usize + 1,
                0x80..=0xBF => {
                    written += (command - 0x80) as usize + 1;
                    pos += 1;
                }
                0xC0..=0xFF => {
                    written += (command - 0xC0) as usize + 1;
                    pos += (command - 0xC0) as usize + 1;
                }
            }
        }
    }

    // Collision: runs of "solid" (commands from 0x80) or "free" tiles.
    let mut collision = vec![false; tiles_wide * tiles_high];
    if collision_layers > 0 {
        let mut index = 0;
        while index < collision.len() {
            let command = *data.get(pos).ok_or(RomError::Truncated)?;
            pos += 1;
            let (solid, run) = (command & 0x80 != 0, (command & 0x7F) as usize + 1);
            for _ in 0..run {
                if let Some(tile) = collision.get_mut(index) {
                    *tile = solid;
                }
                index += 1;
            }
        }
        for index in tiles_wide..collision.len() {
            collision[index] ^= collision[index - tiles_wide];
        }
    }
    Ok(ChunkMap { chunks_wide, chunks_high, tiles_wide, tiles_high, layers, collision })
}

/// Pairs of 12-bit values packed in 3 bytes. Commands below 0x80 write
/// `command + 1` null pairs, 0x80..0xBF repeat the next pair and 0xC0..
/// copy pairs, both for their low bits + 1 (worked out on the ROM's maps).
fn decompress_chunk_map(data: &[u8], pos: &mut usize, count: usize) -> Result<Vec<u16>, RomError> {
    let mut out = Vec::with_capacity(count + 1);
    let pair = |pos: &mut usize| -> Result<[u16; 2], RomError> {
        let b = data.get(*pos..*pos + 3).ok_or(RomError::Truncated)?;
        *pos += 3;
        Ok([b[0] as u16 | (b[1] as u16 & 0xF) << 8, (b[1] as u16) >> 4 | (b[2] as u16) << 4])
    };
    while out.len() < count {
        let command = *data.get(*pos).ok_or(RomError::Truncated)?;
        *pos += 1;
        match command {
            0x00..=0x7F => out.resize(out.len() + (command as usize + 1) * 2, 0),
            0x80..=0xBF => {
                let repeated = pair(pos)?;
                for _ in 0..=command - 0x80 {
                    out.extend(repeated);
                }
            }
            0xC0..=0xFF => {
                for _ in 0..=command - 0xC0 {
                    out.extend(pair(pos)?);
                }
            }
        }
    }
    out.truncate(count);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_title_moves() {
        let rom = crate::nds::test_rom();
        let list = map_list(rom.file("MAP_BG/bg_list.dat").unwrap());
        for name in ["s05p02c2", "s05p02c3", "s05p02a", "s05p02b"] {
            let Some(entry) = list.iter().find(|entry| entry.bma == name) else { continue };
            let map = MapBackground::load(&rom, entry).unwrap();
            println!(
                "{name}: {}x{}, {} frames every {} ticks, {} palette cycles {:?}",
                map.width,
                map.height,
                map.frames.len(),
                map.frame_ticks,
                map.cycles.len(),
                map.cycles.iter().map(|c| (c.palette, c.ticks, c.colours.len())).collect::<Vec<_>>()
            );
        }
        // The title's sprites: how many pictures each has (`extractor dump
        // title-sprites` writes them out).
        for name in ["a", "b", "c", "d", "e"].iter().flat_map(|letter| (1..=4).map(move |n| format!("s05p02{letter}{n}"))) {
            let Some(data) = rom.file(&format!("GROUND/{name}.wan")) else { continue };
            let Ok(wan) = crate::wan::Wan::parse(data) else {
                println!("{name}: not parsed");
                continue;
            };
            println!("{name}: {} meta-frames, groups {:?}", wan.meta_frame_count(), wan.group_sizes());
        }
    }

    #[test]
    fn tile_compression_copies_and_repeats() {
        // Copy 3 bytes, load pattern 7 and write 2, swap back to pattern 0
        // (the initial one) for 1, reuse it for 2.
        let input = [0x02, 1, 2, 3, 0x81, 7, 0xE0, 0xC1];
        let (out, used) = decompress_tiles(&input, 8).unwrap();
        assert_eq!(out, [1, 2, 3, 7, 7, 0, 0, 0]);
        assert_eq!(used, input.len());
    }

    #[test]
    fn mapping_compression_fills_high_then_low_bytes() {
        // High bytes: one zero, then 0x12 twice. Low bytes: skip one, then
        // copy 0x34 and 0x56.
        let input = [0x00, 0x81, 0x12, 0x00, 0xC1, 0x34, 0x56];
        assert_eq!(decompress_mappings(&input, 3).unwrap(), [0x0000, 0x1234, 0x1256]);
    }

    #[test]
    #[ignore]
    fn real_rom_palettes_that_change_are_read() {
        let rom = crate::nds::test_rom();
        let maps = map_list(rom.file("MAP_BG/bg_list.dat").unwrap());
        // The personality test's backdrop: one palette going through 63
        // sets of colours, 8 frames each, starting red.
        let entry = maps.iter().find(|entry| entry.bpl.eq_ignore_ascii_case("s02p01a")).unwrap();
        let map = MapBackground::load(&rom, entry).unwrap();
        assert_eq!(map.cycles.len(), 1);
        let cycle = &map.cycles[0];
        assert_eq!((cycle.palette, cycle.ticks, cycle.colours.len()), (0, 8, 63));
        assert_eq!(cycle.colours[0][1], [0xFF, 0, 0]);
        assert_eq!(cycle.colours[27][1][0], 0);
        // Every pixel drawn is known by its palette and colour.
        assert_eq!(map.indices.len(), map.frames.len());
        let (frame, indices) = (&map.frames[0], &map.indices[0]);
        assert_eq!(indices.len() * 4, frame.len());
        assert!(indices.iter().zip(frame.as_chunks::<4>().0.iter()).all(|(&index, pixel)| (index & 0xF != 0) == (pixel[3] != 0)));
        assert!(indices.iter().any(|&index| index >> 4 == 0 && index != 0));
        // Its two layers, each covering the whole map, and each on a
        // picture of its own.
        assert_eq!(map.upper.len(), map.frames.len());
        assert_eq!(map.upper_indices.len(), map.frames.len());
        assert!(map.upper[0].as_chunks::<4>().0.iter().all(|pixel| pixel[3] != 0));
        assert_eq!(map.layers.len(), 2);
        // The guild's entrance: the tent's front is on the upper layer,
        // which leaves most of the map to be seen through.
        let entry = maps.iter().find(|entry| entry.bma.eq_ignore_ascii_case("g01p01b")).unwrap();
        let guild = MapBackground::load(&rom, entry).unwrap();
        let over = guild.upper[0].as_chunks::<4>().0.iter().filter(|pixel| pixel[3] != 0).count();
        assert!(over > 1000 && over * 4 < guild.width * guild.height, "{over}");
        // A map of one layer has no upper one.
        let entry = maps.iter().find(|entry| entry.bma.eq_ignore_ascii_case("g01p07a")).unwrap();
        assert!(MapBackground::load(&rom, entry).unwrap().upper.is_empty());
        assert!(map.layers.iter().all(|layer| layer.len() == indices.len() && layer.iter().all(|&index| index & 0xF != 0)));
        assert_eq!(map.palettes.len(), 16);
        // Many maps have such palettes; those without keep no indices.
        let with = maps.iter().filter_map(|entry| MapBackground::load(&rom, entry).ok()).filter(|map| !map.cycles.is_empty()).count();
        println!("{with} maps with palettes that change");
        assert!(with > 100);
    }

    #[test]
    #[ignore]
    fn real_rom_maps_decode() {
        let rom = crate::nds::test_rom();
        let maps = map_list(rom.file("MAP_BG/bg_list.dat").unwrap());
        let (mut loaded, mut failed) = (0, Vec::new());
        let (mut animated, mut moving) = (0, 0);
        for (index, entry) in maps.iter().enumerate() {
            match MapBackground::load(&rom, entry) {
                Ok(map) => {
                    loaded += 1;
                    // Animated maps: several frames, not all identical.
                    if map.frames.len() > 1 {
                        animated += 1;
                        // (On either layer.)
                        let moves = |frames: &[Vec<u8>]| frames.iter().any(|f| *f != frames[0]);
                        moving += (moves(&map.frames) || moves(&map.upper)) as usize;
                        assert!(map.frame_ticks > 0, "{}", entry.bma);
                    }
                    let rgba = &map.flat(0).unwrap();
                    let opaque = rgba.as_chunks::<4>().0.iter().filter(|px| px[3] != 0).count();
                    if opaque * 4 < map.width * map.height {
                        failed.push((index, entry.bma.clone(), "mostly empty"));
                    }
                }
                Err(_) => failed.push((index, entry.bma.clone(), "error")),
            }
        }
        let errors = failed.iter().filter(|f| f.2 == "error").count();
        println!("{} maps listed, {loaded} decoded, {} mostly empty, {errors} errors", maps.len(), failed.len() - errors);
        println!("{animated} maps have several frames, {moving} of them actually change");
        assert!(maps.len() > 100);
        assert_eq!(errors, 0);
        assert!(animated > 20 && moving * 10 >= animated * 8);
        // Sparse maps exist (skies, interiors drawn on a small area).
        assert!(failed.len() * 4 < maps.len(), "too many maps are mostly empty");
    }
}
