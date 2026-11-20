//! Tilequant: any picture made to the console's rules. It is cut in tiles of
//! 8 by 8 pixels, each showing the colours of one palette of 16 (the first
//! one seen through), at most so many palettes (16, 12 for a dungeon's
//! tileset); every colour on 5 bits a part. Pixels stay crisp: none is
//! mixed with its neighbours, unless an ordered dither is asked.
//!
//! The tiles are grouped by the colours they show (k-means on their
//! colours' histograms), each group gets its palette (a median cut of all
//! its tiles' colours to 15), then each tile takes the palette that shows it
//! best, and the palettes are made again for those who took them.
//! (The idea after SkyTemple's tilequant, GPL-3; written anew.)

use std::collections::HashMap;

/// A tile's side, in pixels.
pub const TILE: usize = 8;

/// What is asked: how many palettes at most, and whether to dither.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Asked {
    pub palettes: usize,
    pub dither: bool,
}

impl Default for Asked {
    fn default() -> Self {
        Asked { palettes: 16, dither: false }
    }
}

/// A picture made to the rules: its pixels (RGBA, the seen-through ones all
/// zero), which palette and colour each shows (`palette * 16 + colour`, 0:
/// seen through), its palettes (colour 0 of each seen through).
pub struct Quantized {
    pub rgba: Vec<u8>,
    pub indices: Vec<u8>,
    pub palettes: Vec<[[u8; 3]; 16]>,
}

type Colour = [u8; 3];

/// A colour on 5 bits a part, as the console shows it.
fn console(colour: Colour) -> Colour {
    colour.map(|part| part & 0xF8)
}

fn distance(a: Colour, b: Colour) -> u32 {
    // (Green weighs more to the eye, blue less.)
    let part = |at: usize, weight: i32| (a[at] as i32 - b[at] as i32).pow(2) * weight;
    (part(0, 3) + part(1, 4) + part(2, 2)) as u32
}

fn nearest(colours: &[Colour], colour: Colour) -> (usize, u32) {
    colours.iter().enumerate().map(|(at, known)| (at, distance(*known, colour))).min_by_key(|(_, far)| *far).unwrap_or((0, u32::MAX))
}

/// The colours a tile shows (opaque ones), and how many pixels each.
fn histogram(rgba: &[u8], wide: usize, (x0, y0): (usize, usize), high: usize) -> Vec<(Colour, u32)> {
    let mut counted: HashMap<Colour, u32> = HashMap::new();
    for y in y0..(y0 + TILE).min(high) {
        for x in x0..(x0 + TILE).min(wide) {
            let pixel = &rgba[(y * wide + x) * 4..(y * wide + x) * 4 + 4];
            if pixel[3] >= 128 {
                *counted.entry(console([pixel[0], pixel[1], pixel[2]])).or_default() += 1;
            }
        }
    }
    let mut colours: Vec<(Colour, u32)> = counted.into_iter().collect();
    colours.sort();
    colours
}

/// What a tile is grouped by: its share of each of 64 boxes of colour (4
/// steps a part), then its mean colour and how far its colours spread.
const FEATURE: usize = 64 + 6;

fn feature(colours: &[(Colour, u32)]) -> [f32; FEATURE] {
    let mut boxes = [0.0f32; FEATURE];
    let total = colours.iter().map(|(_, count)| *count).sum::<u32>().max(1) as f32;
    for (colour, count) in colours {
        let at = (colour[0] as usize >> 6) * 16 + (colour[1] as usize >> 6) * 4 + (colour[2] as usize >> 6);
        let share = *count as f32 / total;
        boxes[at] += share * 0.5;
        for part in 0..3 {
            boxes[64 + part] += share * colour[part] as f32 / 128.0;
        }
    }
    for part in 0..3 {
        let mean = boxes[64 + part] * 128.0;
        let spread = colours.iter().map(|(colour, count)| (colour[part] as f32 - mean).abs() * *count as f32).sum::<f32>() / total;
        boxes[67 + part] = spread / 128.0;
    }
    boxes
}

fn apart(a: &[f32; FEATURE], b: &[f32; FEATURE]) -> f32 {
    a.iter().zip(b).map(|(a, b)| (a - b) * (a - b)).sum()
}

/// Up to `most` colours standing for those (weighed): themselves if few
/// enough, else a median cut.
pub fn median_cut(colours: &[(Colour, u32)], most: usize) -> Vec<Colour> {
    if colours.len() <= most {
        return colours.iter().map(|(colour, _)| *colour).collect();
    }
    let spread = |cut: &[(Colour, u32)]| -> (u64, usize) {
        // (How much it would gain split: its widest part's range by its weight.)
        let weight: u64 = cut.iter().map(|(_, count)| *count as u64).sum();
        (0..3)
            .map(|part| {
                let low = cut.iter().map(|(colour, _)| colour[part]).min().unwrap_or(0);
                let high = cut.iter().map(|(colour, _)| colour[part]).max().unwrap_or(0);
                ((high - low) as u64 * weight, part)
            })
            .max()
            .unwrap_or((0, 0))
    };
    let mut boxes: Vec<Vec<(Colour, u32)>> = vec![colours.to_vec()];
    while boxes.len() < most {
        let Some((at, (gain, part))) =
            boxes.iter().enumerate().filter(|(_, cut)| cut.len() > 1).map(|(at, cut)| (at, spread(cut))).max_by_key(|(_, (gain, _))| *gain)
        else {
            break;
        };
        if gain == 0 {
            break;
        }
        let mut cut = boxes.swap_remove(at);
        cut.sort_by_key(|(colour, _)| colour[part]);
        let half: u64 = cut.iter().map(|(_, count)| *count as u64).sum::<u64>() / 2;
        let mut sum = 0u64;
        let middle = cut.iter().position(|(_, count)| {
            sum += *count as u64;
            sum >= half
        });
        let split = (middle.unwrap_or(0) + 1).clamp(1, cut.len() - 1);
        let other = cut.split_off(split);
        boxes.push(cut);
        boxes.push(other);
    }
    let mut made: Vec<Colour> = boxes
        .iter()
        .map(|cut| {
            let weight: u64 = cut.iter().map(|(_, count)| *count as u64).sum::<u64>().max(1);
            let mean =
                |part: usize| ((cut.iter().map(|(colour, count)| colour[part] as u64 * *count as u64).sum::<u64>() / weight) as u8).saturating_add(4) & 0xF8;
            [mean(0), mean(1), mean(2)]
        })
        .collect();
    made.sort();
    made.dedup();
    made
}

/// What a tile loses shown with a palette.
fn loss(tile: &[(Colour, u32)], palette: &[Colour]) -> u64 {
    tile.iter().map(|(colour, count)| nearest(palette, *colour).1 as u64 * *count as u64).sum()
}

/// The palettes for tiles grouped so: a median cut of each group's colours.
fn palettes_of(tiles: &[Vec<(Colour, u32)>], group: &[usize], count: usize) -> Vec<Vec<Colour>> {
    (0..count)
        .map(|at| {
            let mut counted: HashMap<Colour, u32> = HashMap::new();
            for (tile, _) in tiles.iter().zip(group).filter(|(_, group)| **group == at) {
                for (colour, count) in tile {
                    *counted.entry(*colour).or_default() += count;
                }
            }
            let mut colours: Vec<(Colour, u32)> = counted.into_iter().collect();
            colours.sort();
            median_cut(&colours, 15)
        })
        .collect()
}

/// The 4 by 4 ordered dither's thresholds.
const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// A picture (`wide` by `high`, RGBA) made to the rules.
pub fn quantize(rgba: &[u8], wide: usize, high: usize, asked: Asked) -> Quantized {
    let most = asked.palettes.clamp(1, 16);
    let places: Vec<(usize, usize)> = (0..high.div_ceil(TILE)).flat_map(|y| (0..wide.div_ceil(TILE)).map(move |x| (x * TILE, y * TILE))).collect();
    let tiles: Vec<Vec<(Colour, u32)>> = places.iter().map(|at| histogram(rgba, wide, *at, high)).collect();
    let features: Vec<[f32; FEATURE]> = tiles.iter().map(|tile| feature(tile)).collect();
    let shown: Vec<usize> = (0..tiles.len()).filter(|at| !tiles[*at].is_empty()).collect();
    // --- The tiles in groups: centres the farthest apart first, then k-means ---
    let mut centres: Vec<[f32; FEATURE]> = Vec::new();
    if let Some(first) = shown.iter().max_by_key(|at| tiles[**at].len()) {
        centres.push(features[*first]);
    }
    while centres.len() < most {
        let far = shown
            .iter()
            .map(|at| (*at, centres.iter().map(|centre| apart(&features[*at], centre)).fold(f32::MAX, f32::min)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        match far {
            Some((at, far)) if far > 1e-6 => centres.push(features[at]),
            _ => break,
        }
    }
    let count = centres.len().max(1);
    let mut group = vec![0usize; tiles.len()];
    for _ in 0..8 {
        for at in &shown {
            group[*at] = centres
                .iter()
                .enumerate()
                .map(|(index, centre)| (index, apart(&features[*at], centre)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map_or(0, |(index, _)| index);
        }
        for (index, centre) in centres.iter_mut().enumerate() {
            let members: Vec<usize> = shown.iter().copied().filter(|at| group[*at] == index).collect();
            if members.is_empty() {
                continue;
            }
            *centre = [0.0; FEATURE];
            for at in &members {
                for (sum, part) in centre.iter_mut().zip(features[*at]) {
                    *sum += part / members.len() as f32;
                }
            }
        }
    }
    // --- Each tile to the palette that shows it best, the palettes made again ---
    let mut palettes = palettes_of(&tiles, &group, count);
    for _ in 0..3 {
        for at in &shown {
            group[*at] = palettes
                .iter()
                .enumerate()
                .filter(|(_, palette)| !palette.is_empty())
                .map(|(index, palette)| (index, loss(&tiles[*at], palette)))
                .min_by_key(|(_, lost)| *lost)
                .map_or(0, |(index, _)| index);
        }
        palettes = palettes_of(&tiles, &group, count);
    }
    // --- The pixels ---
    let mut made = vec![0u8; wide * high * 4];
    let mut indices = vec![0u8; wide * high];
    for ((x0, y0), palette_at) in places.iter().zip(&group) {
        let palette = &palettes[*palette_at];
        for y in *y0..(y0 + TILE).min(high) {
            for x in *x0..(x0 + TILE).min(wide) {
                let at = y * wide + x;
                let pixel = &rgba[at * 4..at * 4 + 4];
                if pixel[3] < 128 || palette.is_empty() {
                    continue;
                }
                let mut colour = [pixel[0], pixel[1], pixel[2]];
                if asked.dither {
                    let step = (BAYER[y % 4][x % 4] * 2 - 15) * 12 / 16;
                    colour = colour.map(|part| (part as i32 + step).clamp(0, 255) as u8);
                }
                let (chosen, _) = nearest(palette, colour);
                made[at * 4..at * 4 + 3].copy_from_slice(&palette[chosen]);
                made[at * 4 + 3] = 255;
                indices[at] = (*palette_at * 16 + chosen + 1) as u8;
            }
        }
    }
    let palettes = palettes
        .iter()
        .map(|palette| {
            let mut sixteen = [[0u8; 3]; 16];
            for (to, colour) in sixteen.iter_mut().skip(1).zip(palette) {
                *to = *colour;
            }
            sixteen
        })
        .collect();
    Quantized { rgba: made, indices, palettes }
}

/// Whether a picture keeps to the rules already: each tile of 8 within 15
/// colours of one of at most so many palettes (said roughly: the tiles'
/// colour sets merged greedily). How many palettes it takes.
pub fn palettes_needed(rgba: &[u8], wide: usize, high: usize) -> usize {
    let places: Vec<(usize, usize)> = (0..high.div_ceil(TILE)).flat_map(|y| (0..wide.div_ceil(TILE)).map(move |x| (x * TILE, y * TILE))).collect();
    let mut sets: Vec<Vec<Colour>> = Vec::new();
    for at in places {
        let colours: Vec<Colour> = histogram(rgba, wide, at, high).into_iter().map(|(colour, _)| colour).collect();
        if colours.is_empty() {
            continue;
        }
        if colours.len() > 15 {
            return usize::MAX;
        }
        match sets.iter_mut().find(|set| set.iter().chain(&colours).collect::<std::collections::HashSet<_>>().len() <= 15) {
            Some(set) => {
                set.extend(colours);
                set.sort();
                set.dedup();
            }
            None => sets.push(colours),
        }
    }
    sets.len()
}
