//! A dungeon's tileset of one's own, drawn on one picture after a template of
//! 47 cells (after skytemple-dtef's DTEF, GPL-3): three parts side by side,
//! the walls, the secondary ground (water, lava), the ground, each of 6 by 8
//! cells of 24 pixels. Each cell is a tile for one way its 8 neighbours are
//! like it or not; the 256 ways of the game's table are each one of those
//! 47 (a corner's neighbour counts only when both sides of it do). Two more
//! pictures (`<word>.1.png`, `<word>.2.png`) may give other versions of the
//! cells, an empty cell meaning the version before.

use std::path::{Path, PathBuf};

use crate::images::Frame;
use crate::tileset::{CHUNK, Tileset};

/// The neighbours' bits, as the game's table has them.
const S: u8 = 1;
const SE: u8 = 2;
const E: u8 = 4;
const NE: u8 = 8;
const N: u8 = 16;
const NW: u8 = 32;
const W: u8 = 64;
const SW: u8 = 128;

/// The cells of a part, row by row, 6 a row: which neighbours are like the
/// tile in each (none: the cell not used, where the part's name is written).
pub const RULES: [Option<u8>; 48] = [
    Some(E | S | SE),
    Some(W | E | SW | S | SE),
    Some(W | SW | S),
    Some(E | S),
    Some(W | E),
    Some(W | S),
    Some(N | NE | E | S | SE),
    Some(255),
    Some(NW | N | W | SW | S),
    Some(N | S),
    Some(0),
    Some(N | W),
    Some(N | NE | E),
    Some(NW | N | NE | W | E),
    Some(NW | N | W),
    Some(N | E),
    Some(S),
    None,
    Some(NW | N | W | E | SW | S),
    Some(N | NE | W | E | S | SE),
    Some(W | E | S),
    Some(E),
    Some(N | W | E | S),
    Some(W),
    Some(N | W | E | SW | S | SE),
    Some(NW | N | NE | W | E | S),
    Some(N | W | E),
    Some(N | E | S),
    Some(N),
    Some(N | W | S),
    Some(NW | N | NE | W | E | SW | S),
    Some(NW | N | NE | W | E | S | SE),
    Some(N | NE | E | S),
    Some(NW | N | W | S),
    Some(W | E | SW | S),
    Some(W | E | S | SE),
    Some(NW | N | W | E | SW | S | SE),
    Some(N | NE | W | E | SW | S | SE),
    Some(N | E | S | SE),
    Some(N | W | SW | S),
    Some(NW | N | W | E),
    Some(N | NE | W | E),
    Some(N | W | E | S | SE),
    Some(N | W | E | SW | S),
    Some(N | NE | W | E | S),
    Some(NW | N | W | E | S),
    Some(N | NE | W | E | SW | S),
    Some(NW | N | W | E | S | SE),
];

/// Cells a row and rows of a part; the parts (walls, secondary, ground).
pub const COLUMNS: usize = 6;
pub const ROWS: usize = 8;
pub const PARTS: usize = 3;
/// The template's size, in pixels.
pub const WIDE: usize = COLUMNS * PARTS * CHUNK;
pub const HIGH: usize = ROWS * CHUNK;
/// The cell left unused in each part.
const UNUSED: usize = 17;
/// Versions of a tile the game's table has.
const VERSIONS: usize = 3;

/// A way of the neighbours as one of the 47: a corner kept only with both its sides.
pub fn base_rule(bits: u8) -> u8 {
    let mut rule = bits;
    for (corner, one, other) in [(NW, N, W), (NE, N, E), (SW, S, W), (SE, S, E)] {
        if rule & corner != 0 && (rule & one == 0 || rule & other == 0) {
            rule &= !corner;
        }
    }
    rule
}

/// The cell of the template a way of the neighbours is drawn in.
pub fn cell_of(bits: u8) -> usize {
    let base = base_rule(bits);
    RULES.iter().position(|rule| *rule == Some(base)).expect("every reduced way is one of the 47")
}

/// Where a cell of a part is in the template, its top left corner in pixels.
fn corner(part: usize, cell: usize) -> (usize, usize) {
    ((part * COLUMNS + cell % COLUMNS) * CHUNK, cell / COLUMNS * CHUNK)
}

/// Letters 3 by 5 for the parts' names.
fn letter(said: char) -> [&'static str; 5] {
    match said {
        'W' => ["101", "101", "101", "111", "101"],
        'A' => ["010", "101", "111", "101", "101"],
        'L' => ["100", "100", "100", "100", "111"],
        'T' => ["111", "010", "010", "010", "010"],
        'E' => ["111", "100", "110", "100", "111"],
        'R' => ["110", "101", "110", "101", "101"],
        'F' => ["111", "100", "110", "100", "100"],
        'O' => ["010", "101", "101", "101", "010"],
        _ => ["000"; 5],
    }
}

/// The template, blank: in each cell the tile in white, its neighbours black
/// when like it, blue when not; the unused cell red, the part's name on it
/// (WALL, WATER, FLOOR). RGBA, `WIDE` by `HIGH`.
pub fn blank() -> Vec<u8> {
    let mut rgba = vec![0u8; WIDE * HIGH * 4];
    let mut put = |(x, y): (usize, usize), colour: [u8; 4]| rgba[(y * WIDE + x) * 4..(y * WIDE + x) * 4 + 4].copy_from_slice(&colour);
    const THIRD: usize = CHUNK / 3;
    // (The neighbours by thirds of the cell: their bits, row by row.)
    let around = [[NW, N, NE], [W, 0, E], [SW, S, SE]];
    for part in 0..PARTS {
        for (cell, rule) in RULES.iter().enumerate() {
            let (x0, y0) = corner(part, cell);
            for y in 0..CHUNK {
                for x in 0..CHUNK {
                    let colour = match rule {
                        // (The cells' edges, to tell them apart.)
                        _ if x == 0 || y == 0 => [0x80, 0x80, 0x80, 255],
                        None => [0xC0, 0x20, 0x20, 255],
                        Some(_) if (x / THIRD, y / THIRD) == (1, 1) => [0xFF, 0xFF, 0xFF, 255],
                        Some(rule) if rule & around[y / THIRD][x / THIRD] != 0 => [0x10, 0x10, 0x10, 255],
                        Some(_) => [0x30, 0x50, 0xE0, 255],
                    };
                    put((x0 + x, y0 + y), colour);
                }
            }
        }
        let (x0, y0) = corner(part, UNUSED);
        for (at, said) in ["WALL", "WATER", "FLOOR"][part].chars().enumerate() {
            for (row, line) in letter(said).iter().enumerate() {
                for (column, dot) in line.chars().enumerate() {
                    if dot == '1' {
                        put((x0 + 2 + at * 4 + column, y0 + 10 + row), [0xFF, 0xFF, 0xFF, 255]);
                    }
                }
            }
        }
    }
    rgba
}

/// A tileset of the game drawn on the template: a picture for each version of
/// its tiles (a cell empty where a version is the one before). None for a
/// whole picture (a boss's floor).
pub fn export(tileset: &Tileset) -> Option<Vec<Vec<u8>>> {
    if tileset.chunk_for.len() == Tileset::PICTURE {
        return None;
    }
    let mut pictures = vec![vec![0u8; WIDE * HIGH * 4]; VERSIONS];
    for part in 0..PARTS {
        for (cell, rule) in RULES.iter().enumerate() {
            let Some(rule) = rule else {
                continue;
            };
            let mut drawn: Vec<u16> = Vec::new();
            for (version, picture) in pictures.iter_mut().enumerate() {
                let Some(chunk) = tileset.chunk_for.get(part * 768 + *rule as usize * 3 + version) else {
                    continue;
                };
                if drawn.contains(chunk) {
                    continue;
                }
                drawn.push(*chunk);
                let Some(frame) = tileset.chunks.get(*chunk as usize) else {
                    continue;
                };
                let (x0, y0) = corner(part, cell);
                for y in 0..CHUNK {
                    let to = ((y0 + y) * WIDE + x0) * 4;
                    picture[to..to + CHUNK * 4].copy_from_slice(&frame.rgba[y * CHUNK * 4..(y + 1) * CHUNK * 4]);
                }
            }
        }
    }
    Some(pictures)
}

/// A picture as the template is (an exact multiple of it, drawn larger, made
/// back to its size), or why not.
pub fn fitted(rgba: Vec<u8>, wide: usize, high: usize) -> Result<Vec<u8>, String> {
    if (wide, high) == (WIDE, HIGH) {
        return Ok(rgba);
    }
    let times = wide / WIDE;
    if times == 0 || wide != WIDE * times || high != HIGH * times {
        return Err(format!("{wide}×{high}: the template is {WIDE}×{HIGH} (3 parts of 6×8 cells of {CHUNK} pixels)"));
    }
    let mut made = Vec::with_capacity(WIDE * HIGH * 4);
    for y in 0..HIGH {
        for x in 0..WIDE {
            let from = (y * times * wide + x * times) * 4;
            made.extend_from_slice(&rgba[from..from + 4]);
        }
    }
    Ok(made)
}

/// A tileset from pictures drawn on the template (the first, then the other
/// versions): the same the game draws its floors with.
pub fn tileset_of(pictures: &[Vec<u8>]) -> Tileset {
    let mut chunks: Vec<Frame> = Vec::new();
    // (The chunk of each part, cell and version.)
    let mut of = vec![[[0u16; VERSIONS]; 48]; PARTS];
    for (part, cells) in of.iter_mut().enumerate() {
        for (cell, versions) in cells.iter_mut().enumerate() {
            if RULES[cell].is_none() {
                continue;
            }
            let (x0, y0) = corner(part, cell);
            for version in 0..VERSIONS {
                let rgba: Option<Vec<u8>> = pictures
                    .get(version)
                    .map(|picture| (0..CHUNK).flat_map(|y| picture[((y0 + y) * WIDE + x0) * 4..((y0 + y) * WIDE + x0 + CHUNK) * 4].iter().copied()).collect());
                let empty = rgba.as_ref().is_none_or(|rgba| rgba.iter().skip(3).step_by(4).all(|alpha| *alpha == 0));
                versions[version] = match rgba {
                    _ if version > 0 && empty => versions[version - 1],
                    rgba => {
                        let rgba = rgba.unwrap_or_else(|| vec![0; CHUNK * CHUNK * 4]);
                        chunks.push(Frame::still(CHUNK, CHUNK, (CHUNK as i32 / 2, CHUNK as i32 / 2), rgba));
                        (chunks.len() - 1) as u16
                    }
                };
            }
        }
    }
    let mut chunk_for = vec![0u16; PARTS * 768];
    for (part, cells) in of.iter().enumerate() {
        for bits in 0..=255u8 {
            let cell = cell_of(bits);
            for version in 0..VERSIONS {
                chunk_for[part * 768 + bits as usize * 3 + version] = cells[cell][version];
            }
        }
    }
    // (Its pixels on 5 bits a part, as the console shows them.)
    for chunk in &mut chunks {
        crate::images::console_colours(&mut chunk.rgba);
    }
    Tileset { chunk_for, chunks }
}

/// Where the other versions of a tileset of one's own are: `<word>.1.png`, `<word>.2.png`.
pub fn versions_of(path: &Path) -> Vec<PathBuf> {
    let stem = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    (1..VERSIONS).map(|version| path.with_file_name(format!("{stem}.{version}.png"))).collect()
}

/// A picture read as RGBA, and its size.
fn read_picture(path: &Path) -> Result<(Vec<u8>, usize, usize), String> {
    let picture = image::open(path).map_err(|why| format!("{}: {why}", path.display()))?.to_rgba8();
    let (wide, high) = (picture.width() as usize, picture.height() as usize);
    Ok((picture.into_raw(), wide, high))
}

/// The pictures of a tileset of one's own (its other versions if it has
/// them), each as the template is.
pub fn pictures(path: &Path) -> Result<Vec<Vec<u8>>, String> {
    let (rgba, wide, high) = read_picture(path)?;
    let mut pictures = vec![fitted(rgba, wide, high).map_err(|why| format!("{}: {why}", path.display()))?];
    for version in versions_of(path).into_iter().take_while(|version| version.exists()) {
        let (rgba, wide, high) = read_picture(&version)?;
        pictures.push(fitted(rgba, wide, high).map_err(|why| format!("{}: {why}", version.display()))?);
    }
    Ok(pictures)
}

/// A tileset of one's own, from its picture (`library/tilesets/<word>.png`).
pub fn read(path: &Path) -> Result<Tileset, String> {
    Ok(tileset_of(&pictures(path)?))
}

/// Writes pictures on the template: the first at `path`, the others as its versions.
pub fn write(path: &Path, pictures: &[Vec<u8>]) -> Result<(), String> {
    let mut paths = vec![path.to_path_buf()];
    paths.extend(versions_of(path));
    for (picture, to) in pictures.iter().zip(paths) {
        // (A version with no cell of its own is not written.)
        if to != path && picture.iter().skip(3).step_by(4).all(|alpha| *alpha == 0) {
            continue;
        }
        image::save_buffer(&to, picture, WIDE as u32, HIGH as u32, image::ColorType::Rgba8).map_err(|why| format!("{}: {why}", to.display()))?;
    }
    Ok(())
}

/// Pictures on the template made to the console's rules: each tile of 8 one
/// palette of 16, at most `palettes` of them for all (12 for a dungeon), the
/// versions side by side so they share them.
pub fn quantized(pictures: &[Vec<u8>], palettes: usize, dither: bool) -> Vec<Vec<u8>> {
    let count = pictures.len();
    let wide = WIDE * count;
    let mut whole = vec![0u8; wide * HIGH * 4];
    for (at, picture) in pictures.iter().enumerate() {
        for y in 0..HIGH {
            let to = (y * wide + at * WIDE) * 4;
            whole[to..to + WIDE * 4].copy_from_slice(&picture[y * WIDE * 4..(y + 1) * WIDE * 4]);
        }
    }
    let made = crate::quant::quantize(&whole, wide, HIGH, crate::quant::Asked { palettes, dither });
    (0..count).map(|at| (0..HIGH).flat_map(|y| made.rgba[(y * wide + at * WIDE) * 4..(y * wide + at * WIDE + WIDE) * 4].iter().copied()).collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 256 ways each fall in one of the 47 cells, the 47 their own.
    #[test]
    fn every_way_has_its_cell() {
        let used: std::collections::HashSet<usize> = (0..=255u8).map(cell_of).collect();
        assert_eq!(used.len(), 47);
        for (cell, rule) in RULES.iter().enumerate() {
            if let Some(rule) = rule {
                assert_eq!(cell_of(*rule), cell);
            }
        }
    }
}
