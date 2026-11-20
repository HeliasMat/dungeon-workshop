//! The dungeon tilesets of one's own (`library/tilesets/<word>.png`),
//! drawn on the template of 47 cells (`media::template`): the template
//! blank, a tileset of the game drawn on it to start from, a picture made
//! one (each tile of 8 one palette, 12 at most: `media::quant`). Also any
//! picture made to the console's rules alone (`<name>_quant.png`).

use std::path::{Path, PathBuf};

use media::quant::Asked;
use media::template;

use crate::fault::Diagnostic;

/// The palettes a dungeon's tileset has at most (the others are the game's).
pub const PALETTES: usize = 12;

/// Where a project's are drawn (the shared library's: `library/tilesets`).
pub fn folder(project: &assets::project::Project) -> PathBuf {
    project.tilesets()
}

/// Their words (their versions, `<word>.1.png`, left out).
pub fn names(project: &assets::project::Project) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder(project))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(".png").map(str::to_string))
        .filter(|name| !name.contains('.'))
        .collect();
    names.sort();
    names
}

fn wrong(why: String) -> Diagnostic {
    Diagnostic::said(why)
}

fn save(path: &Path, rgba: &[u8], (wide, high): (usize, usize)) -> Result<(), Diagnostic> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|why| wrong(format!("{}: {why}", parent.display())))?;
    }
    image::save_buffer(path, rgba, wide as u32, high as u32, image::ColorType::Rgba8).map_err(|why| wrong(format!("{}: {why}", path.display())))
}

/// The template, blank, written there.
pub fn write_blank(path: &Path) -> Result<(), Diagnostic> {
    save(path, &template::blank(), (template::WIDE, template::HIGH))
}

/// A tileset of the game (by its number) drawn on the template, written
/// there (its other versions beside it).
pub fn export_game(data: &str, number: u8, path: &Path) -> Result<(), Diagnostic> {
    let tileset = media::tileset::read_tileset(data, number).ok_or_else(|| wrong(format!("no tileset {number} of the game in {data}")))?;
    let pictures = template::export(&tileset).ok_or_else(|| wrong(format!("tileset {number} is a whole picture (a boss fight's)")))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|why| wrong(format!("{}: {why}", parent.display())))?;
    }
    template::write(path, &pictures).map_err(wrong)
}

/// A picture drawn on the template (and its versions beside it) made a
/// tileset of one's own under that word: each tile of 8 one palette of 16,
/// 12 at most. Where it was written.
pub fn import(project: &assets::project::Project, source: &Path, word: &str, dither: bool) -> Result<PathBuf, Diagnostic> {
    let pictures = template::pictures(source).map_err(wrong)?;
    let made = template::quantized(&pictures, PALETTES, dither);
    write_made(project, word, &made)
}

/// Pictures on the template, already to the rules, written as the tileset
/// of that word (versions of an older one under that word, not this one's, go).
pub fn write_made(project: &assets::project::Project, word: &str, made: &[Vec<u8>]) -> Result<PathBuf, Diagnostic> {
    let folder = folder(project);
    let path = folder.join(format!("{word}.png"));
    std::fs::create_dir_all(&folder).map_err(|why| wrong(format!("{}: {why}", folder.display())))?;
    for (at, version) in template::versions_of(&path).into_iter().enumerate() {
        if at + 1 >= made.len() && version.exists() {
            let _ = std::fs::remove_file(version);
        }
    }
    template::write(&path, made).map_err(wrong)?;
    Ok(path)
}

/// A picture made to the console's rules; it and what it became (RGBA, sizes).
pub struct Converted {
    pub size: (usize, usize),
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    /// How many palettes it uses.
    pub palettes: usize,
}

/// Any picture made to the console's rules (`media::quant`).
pub fn convert(source: &Path, asked: Asked) -> Result<Converted, Diagnostic> {
    let picture = image::open(source).map_err(|why| wrong(format!("{}: {why}", source.display())))?.to_rgba8();
    let size = (picture.width() as usize, picture.height() as usize);
    let before = picture.into_raw();
    let made = media::quant::quantize(&before, size.0, size.1, asked);
    let palettes = made.indices.iter().filter(|index| **index > 0).map(|index| (index - 1) / 16).collect::<std::collections::HashSet<u8>>().len();
    Ok(Converted { size, before, after: made.rgba, palettes })
}

/// Where a picture made to the rules is written: `<name>_quant.png` beside it.
pub fn quant_path(source: &Path) -> PathBuf {
    let stem = source.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    source.with_file_name(format!("{stem}_quant.png"))
}

/// The same, written beside it.
pub fn convert_file(source: &Path, asked: Asked) -> Result<(PathBuf, Converted), Diagnostic> {
    let made = convert(source, asked)?;
    let path = quant_path(source);
    save(&path, &made.after, made.size)?;
    Ok((path, made))
}

// --- An example drawn by our own code ---

/// A little hash of a place, for pixel noise that is the same each time.
fn noise(x: usize, y: usize, salt: u32) -> u32 {
    let mut said = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ salt.wrapping_mul(0xC2B2_AE3D);
    said ^= said >> 15;
    said = said.wrapping_mul(0x2C1B_3C6D);
    said ^= said >> 12;
    said
}

/// A tileset drawn by code on the template, of no game's: mossy stone walls,
/// clear water with a light edge, grass with flowers; each cell's tile drawn
/// from which of its neighbours are like it (an edge where they are not), so
/// every one of the 47 ways joins its neighbours. Three versions, the flowers
/// and stones moved. Pictures on the template (RGBA), not yet to the rules.
pub fn drawn_example() -> Vec<Vec<u8>> {
    use media::tileset::CHUNK;
    const S: u8 = 1;
    const SE: u8 = 2;
    const E: u8 = 4;
    const NE: u8 = 8;
    const N: u8 = 16;
    const NW: u8 = 32;
    const W: u8 = 64;
    const SW: u8 = 128;
    // (A part's colours: its edge, its body dark, light, and a spot.)
    let wall = [[0x2A, 0x24, 0x30], [0x58, 0x52, 0x5E], [0x78, 0x72, 0x80], [0x4A, 0x7A, 0x3A]];
    let water = [[0xC8, 0xE8, 0xF8], [0x28, 0x58, 0xB0], [0x40, 0x78, 0xD0], [0x80, 0xB8, 0xF0]];
    let grass = [[0x3A, 0x6A, 0x2A], [0x5A, 0x9A, 0x3A], [0x6E, 0xB0, 0x48], [0xF0, 0xE0, 0x60]];
    let parts = [wall, water, grass];
    (0..3u32)
        .map(|version| {
            let mut rgba = vec![0u8; template::WIDE * template::HIGH * 4];
            for (part, colours) in parts.iter().enumerate() {
                for (cell, rule) in template::RULES.iter().enumerate() {
                    let Some(rule) = *rule else {
                        continue;
                    };
                    let (x0, y0) = ((part * template::COLUMNS + cell % template::COLUMNS) * CHUNK, cell / template::COLUMNS * CHUNK);
                    for y in 0..CHUNK {
                        for x in 0..CHUNK {
                            // (How far from each side the pixel is, an edge where the neighbour there differs.)
                            let edge = 3;
                            let near = |side: u8, far: usize| rule & side == 0 && far < edge;
                            let (left, right, top, bottom) = (x, CHUNK - 1 - x, y, CHUNK - 1 - y);
                            let corner = |corner: u8, a: usize, b: usize| rule & corner == 0 && a < edge && b < edge;
                            let on_edge = near(W, left)
                                || near(E, right)
                                || near(N, top)
                                || near(S, bottom)
                                || corner(NW, left, top)
                                || corner(NE, right, top)
                                || corner(SW, left, bottom)
                                || corner(SE, right, bottom);
                            // (The same noise in every cell of a part and version: tiles join.)
                            let said = noise(x, y, part as u32 * 7 + version * 131);
                            let colour = if on_edge {
                                colours[0]
                            } else {
                                match part {
                                    // (Stones: bricks of 8 by 6, mortar dark, moss here and there.)
                                    0 => {
                                        let shift = if (y / 6) % 2 == 0 { 0 } else { 4 };
                                        if y % 6 == 0 || (x + shift) % 8 == 0 {
                                            colours[0]
                                        } else if said.is_multiple_of(23) {
                                            colours[3]
                                        } else if said.is_multiple_of(3) {
                                            colours[2]
                                        } else {
                                            colours[1]
                                        }
                                    }
                                    // (Water: ripples in rows.)
                                    1 => {
                                        if (x + y / 4 * 3 + version as usize * 2) % 9 < 2 && y % 4 == 1 {
                                            colours[3]
                                        } else if (y / 2) % 3 == 0 {
                                            colours[2]
                                        } else {
                                            colours[1]
                                        }
                                    }
                                    // (Grass: blades, a flower now and then.)
                                    _ => {
                                        if said.is_multiple_of(97) {
                                            colours[3]
                                        } else if said.is_multiple_of(4) {
                                            colours[2]
                                        } else {
                                            colours[1]
                                        }
                                    }
                                }
                            };
                            let to = ((y0 + y) * template::WIDE + x0 + x) * 4;
                            rgba[to..to + 4].copy_from_slice(&[colour[0], colour[1], colour[2], 255]);
                        }
                    }
                }
            }
            rgba
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drawn example keeps to the rules once made to them: 12 palettes at most.
    #[test]
    fn drawn_example_keeps_to_the_rules() {
        let made = template::quantized(&drawn_example(), PALETTES, false);
        assert_eq!(made.len(), 3);
        let tileset = template::tileset_of(&made);
        assert_eq!(tileset.chunk_for.len(), 3 * 768);
        assert!(media::quant::palettes_needed(&made[0], template::WIDE, template::HIGH) <= 16);
    }
}
