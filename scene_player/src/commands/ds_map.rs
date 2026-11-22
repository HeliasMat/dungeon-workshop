//! A place of the Workshop's own (a decor) as a map of Explorers of Sky's,
//! for `export-ds`: its two layers as the game keeps a map's (tiles of 8
//! pixels, each of one palette of 16 colours, colour 0 clear; 16 palettes
//! shared by both layers), its size in the game's chunks (3 by 3 tiles), its
//! walls. `tools/ds/patch_rom.py` makes the game's files of it (BPC, BPL,
//! BMA) and gives it a level.
//!
//! The decor's pictures are its built ones (`grounds/pictures/<level>_f0.png`
//! and `_u0.png`), the first frame: its animated tiles and its colours that
//! change with time stay as they first are. A tile with more than 15 colours,
//! or whose colours no palette left has room for, is brought to the nearest
//! of its palette's (requantized).

use std::collections::HashMap;
use std::path::Path;

/// A tile, and a chunk of the game's maps, in pixels.
pub const TILE: usize = 8;
pub const CHUNK: usize = 3 * TILE;
/// The palettes of a map, and the colours of each (its colour 0 is clear).
pub const PALETTES: usize = 16;
pub const COLOURS: usize = 15;

/// A colour as the DS shows it: 5 bits a channel.
type Colour = [u8; 3];
/// A tile that shows something: its layer, where (x, y in tiles), its colours by how many pixels.
type Shown = (usize, usize, usize, Vec<(Colour, usize)>);

/// A decor made a map: its size (whole chunks), each layer's pixels as
/// `palette * 16 + colour`, the palettes, the walls by tiles.
pub struct Map {
    pub width: usize,
    pub height: usize,
    pub lower: Vec<u8>,
    pub upper: Option<Vec<u8>>,
    pub palettes: Vec<Vec<Colour>>,
    pub walls: Vec<bool>,
    /// The tiles brought to their palette's nearest colours.
    pub requantized: usize,
}

/// A colour of 5 bits a channel at 8 bits, as the DS shows it.
fn wide(channel: u8) -> u8 {
    (channel << 3) | (channel >> 2)
}

fn distance(a: Colour, b: Colour) -> i32 {
    a.iter().zip(b).map(|(a, b)| (i32::from(*a) - i32::from(b)).pow(2)).sum()
}

/// The colour of that palette nearest to it (1 the first, 0 for none).
fn nearest(palette: &[Colour], colour: Colour) -> usize {
    palette.iter().enumerate().min_by_key(|(_, one)| distance(**one, colour)).map_or(0, |(at, _)| at + 1)
}

/// The map of a decor `w` by `h` pixels: its lower layer and its upper one
/// (RGBA, a pixel clear under half its alpha), where it is solid by tiles
/// (none known: nowhere). Padded right and down to whole chunks, clear and
/// solid there.
pub fn map_of(w: usize, h: usize, lower: &[u8], upper: Option<&[u8]>, solid: Option<&[Vec<bool>]>) -> Map {
    let (width, height) = (w.div_ceil(CHUNK) * CHUNK, h.div_ceil(CHUNK) * CHUNK);
    let pixel = |layer: &[u8], x: usize, y: usize| -> Option<Colour> {
        let at = (y * w + x) * 4;
        (x < w && y < h && layer.get(at + 3).is_some_and(|alpha| *alpha >= 128)).then(|| [layer[at] >> 3, layer[at + 1] >> 3, layer[at + 2] >> 3])
    };
    let layers: Vec<&[u8]> = std::iter::once(lower).chain(upper).collect();
    // Each tile that shows something: its layer, where, its colours by how many pixels.
    let mut tiles: Vec<Shown> = Vec::new();
    for (layer, pixels) in layers.iter().enumerate() {
        for ty in 0..height / TILE {
            for tx in 0..width / TILE {
                let mut counts: HashMap<Colour, usize> = HashMap::new();
                for y in ty * TILE..ty * TILE + TILE {
                    for x in tx * TILE..tx * TILE + TILE {
                        if let Some(colour) = pixel(pixels, x, y) {
                            *counts.entry(colour).or_default() += 1;
                        }
                    }
                }
                if counts.is_empty() {
                    continue;
                }
                let mut counts: Vec<(Colour, usize)> = counts.into_iter().collect();
                counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                tiles.push((layer, tx, ty, counts));
            }
        }
    }
    // The palettes: the tiles of most colours first, each in the palette it
    // adds the fewest to, a new one while there are fewer than 16, else the
    // one that already has most of its pixels.
    let mut order: Vec<usize> = (0..tiles.len()).collect();
    order.sort_by_key(|at| (std::cmp::Reverse(tiles[*at].3.len()), *at));
    let mut palettes: Vec<Vec<Colour>> = Vec::new();
    let mut chosen = vec![0; tiles.len()];
    let mut requantized = 0;
    for at in order {
        let counts = &tiles[at].3;
        let wanted: Vec<Colour> = counts.iter().take(COLOURS).map(|(colour, _)| *colour).collect();
        let missing = |palette: &Vec<Colour>| wanted.iter().filter(|colour| !palette.contains(colour)).count();
        let fits = palettes
            .iter()
            .enumerate()
            .filter(|(_, palette)| palette.len() + missing(palette) <= COLOURS)
            .min_by_key(|(_, palette)| (missing(palette), std::cmp::Reverse(palette.len())))
            .map(|(n, _)| n);
        let n = match fits {
            Some(n) => n,
            None if palettes.len() < PALETTES => {
                palettes.push(Vec::new());
                palettes.len() - 1
            }
            None => palettes
                .iter()
                .enumerate()
                .max_by_key(|(_, palette)| counts.iter().filter(|(colour, _)| palette.contains(colour)).map(|(_, count)| count).sum::<usize>())
                .map_or(0, |(n, _)| n),
        };
        for colour in &wanted {
            if palettes[n].len() < COLOURS && !palettes[n].contains(colour) {
                palettes[n].push(*colour);
            }
        }
        if counts.iter().any(|(colour, _)| !palettes[n].contains(colour)) {
            requantized += 1;
        }
        chosen[at] = n;
    }
    // Each pixel: its tile's palette, its colour or the nearest of it.
    let mut out = vec![vec![0u8; width * height]; layers.len()];
    for (at, (layer, tx, ty, _)) in tiles.iter().enumerate() {
        let palette = &palettes[chosen[at]];
        for y in ty * TILE..ty * TILE + TILE {
            for x in tx * TILE..tx * TILE + TILE {
                let colour = pixel(layers[*layer], x, y)
                    .map_or(0, |colour| palette.iter().position(|one| *one == colour).map_or_else(|| nearest(palette, colour), |at| at + 1));
                out[*layer][y * width + x] = (chosen[at] * 16 + colour) as u8;
            }
        }
    }
    let walls = (0..height / TILE)
        .flat_map(|ty| (0..width / TILE).map(move |tx| (tx, ty)))
        .map(|(tx, ty)| {
            if tx * TILE >= w || ty * TILE >= h {
                return true;
            }
            solid.is_some_and(|rows| rows.get(ty).and_then(|row| row.get(tx)).copied().unwrap_or(true))
        })
        .collect();
    let upper = (out.len() > 1).then(|| out.pop()).flatten();
    let lower = out.pop().unwrap_or_default();
    Map { width, height, lower, upper, palettes, walls, requantized }
}

impl Map {
    /// The 256 colours of the pictures: 16 palettes of 16, colour 0 black (clear).
    pub fn colours(&self) -> Vec<u8> {
        (0..PALETTES)
            .flat_map(|palette| (0..16).map(move |colour| (palette, colour)))
            .flat_map(|(palette, colour)| {
                let shown = if colour == 0 { None } else { self.palettes.get(palette).and_then(|one| one.get(colour - 1)) };
                shown.map_or([0, 0, 0], |one| [wide(one[0]), wide(one[1]), wide(one[2])])
            })
            .collect()
    }

    /// Written beside the export: `<base>.lower.png`, `.upper.png` (8 bits
    /// indexed, the same 256 colours), `.walls.txt` (a row of `0`/`1` a row of
    /// tiles). The names of the files written, as the export's line says them.
    pub fn write(&self, base: &Path) -> Result<Vec<String>, String> {
        let mut written = Vec::new();
        let colours = self.colours();
        for (part, pixels) in [("lower", Some(&self.lower)), ("upper", self.upper.as_ref())] {
            let path = base.with_file_name(format!("{}.{part}.png", base.file_name().and_then(|name| name.to_str()).unwrap_or("map")));
            let Some(pixels) = pixels else {
                written.push("-".to_string());
                continue;
            };
            let file = std::fs::File::create(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), self.width as u32, self.height as u32);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(colours.clone());
            let mut writer = encoder.write_header().map_err(|error| format!("{}: {error}", path.display()))?;
            writer.write_image_data(pixels).map_err(|error| format!("{}: {error}", path.display()))?;
            written.push(path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string());
        }
        let path = base.with_file_name(format!("{}.walls.txt", base.file_name().and_then(|name| name.to_str()).unwrap_or("map")));
        let rows: Vec<String> = self.walls.chunks(self.width / TILE).map(|row| row.iter().map(|solid| if *solid { '1' } else { '0' }).collect()).collect();
        std::fs::write(&path, rows.join("\n") + "\n").map_err(|error| format!("{}: {error}", path.display()))?;
        written.push(path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string());
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny decor: a tile of 20 colours, tiles of two colours each, a clear
    /// corner on the upper layer; padded to whole chunks, each tile of one palette.
    #[test]
    fn a_decor_becomes_tiles_of_one_palette_in_whole_chunks() {
        let (w, h) = (30, 10);
        let mut lower = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let at = (y * w + x) * 4;
                let colour: [u8; 3] = if x < 8 && y < 8 { [((x + y * 8) % 20 * 12) as u8, 40, 200] } else { [(x / 8 * 60) as u8, (y % 2 * 100) as u8, 10] };
                lower[at..at + 4].copy_from_slice(&[colour[0], colour[1], colour[2], 255]);
            }
        }
        let mut upper = vec![0u8; w * h * 4];
        upper[(3 * w + 12) * 4..(3 * w + 12) * 4 + 4].copy_from_slice(&[250, 0, 0, 255]);
        let map = map_of(w, h, &lower, Some(&upper), None);
        assert_eq!((map.width, map.height), (48, 24), "padded to chunks of 24");
        assert!(map.palettes.len() <= PALETTES && map.palettes.iter().all(|palette| palette.len() <= COLOURS));
        assert_eq!(map.requantized, 1, "only the tile of 20 colours");
        for pixels in [&map.lower, map.upper.as_ref().expect("an upper layer")] {
            for ty in 0..map.height / TILE {
                for tx in 0..map.width / TILE {
                    let first = pixels[ty * TILE * map.width + tx * TILE] >> 4;
                    for y in 0..TILE {
                        for x in 0..TILE {
                            assert_eq!(pixels[(ty * TILE + y) * map.width + tx * TILE + x] >> 4, first, "tile {tx},{ty} of one palette");
                        }
                    }
                }
            }
        }
        // (The upper layer clear but its one pixel; the padding solid.)
        let upper = map.upper.as_ref().expect("an upper layer");
        assert_eq!(upper.iter().filter(|index| *index & 0xF != 0).count(), 1);
        assert!(map.walls[map.width / TILE - 1] && !map.walls[0]);
        assert_eq!(map.colours().len(), 256 * 3);
    }
}
