//! Cutting a piece out of the ground of its place, and its shadow dropped on
//! any ground, as the game draws them.
//!
//! Each tile of a piece is compared with the ground tiles of its place: a
//! pixel that is that ground, or one of the ground's own colours, is left
//! out; one of a colour the ground's shadows are drawn in keeps only how much
//! darker than the ground it is; the rest is the thing. No threshold on
//! colours: only the ground's own tiles and colours.

use std::collections::{BTreeMap, HashMap};

use crate::pixels::{Grid, Mask, Rgba, TILE, border, component, components, counted, grown, has, key, light, mean, unique};
use crate::source::{Source, tile_at};

/// Ground inside a thing of at least this many pixels is left out too.
const HOLE: usize = 6;
/// Bits of the thing smaller than this, alone in the ground, are left out;
/// and those smaller than this part of the largest bit.
const SPECK: f64 = 3.0;
const LESSER: f64 = 0.2;
/// The tiles round a piece looked at for its ground, and how often a tile is
/// seen in its place to be ground.
const RING: i64 = 3;
const COMMON: usize = 3;
/// A tile is ground where it is like a ground tile, if this many of its pixels are.
const FEW: usize = 8;
/// The colours of a ground that are its own: at least this part of its pixels.
const OWN: f64 = 0.04;
/// A shade this near the ground's light is a patch of it, not a shadow.
const FAINT: f32 = 0.1;
/// The rim of a thing in its ground's own colours is peeled so many times; a
/// hole in it is ground if this share of it is of those colours.
const PEEL: usize = 8;
const PLAIN_HOLE: f64 = 0.7;
/// How dark a shadow of the game's things is.
const THING_SHADOW: f32 = 0.7;
/// A shadow colour of the ground is taken when it is this near in lightness.
const RAMP_NEAR: f64 = 0.12;

/// A tile's colours, row by row (RGB): 192 bytes.
pub type Tile = Vec<u8>;

/// The pixels of a piece that are ground made darker or lighter: how much
/// (`factor`, 0 elsewhere), the colour they had, and the colours of the
/// ground they were taken from (sorted), where they keep it.
#[derive(Clone, Debug, PartialEq)]
pub struct Shade {
    pub factor: Grid<f32>,
    pub colour: Grid<[u8; 3]>,
    pub grounds: Vec<i64>,
}

impl Shade {
    pub fn none(w: usize, h: usize) -> Shade {
        Shade { factor: Grid::new(w, h), colour: Grid::new(w, h), grounds: Vec::new() }
    }

    pub fn mask(&self) -> Mask {
        self.factor.map(|factor| factor > 0.0)
    }

    pub fn mirrored(&self) -> Shade {
        Shade { factor: self.factor.mirrored(), colour: self.colour.mirrored(), grounds: self.grounds.clone() }
    }

    pub fn part(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Shade {
        Shade { factor: self.factor.crop(x0, y0, x1, y1), colour: self.colour.crop(x0, y0, x1, y1), grounds: self.grounds.clone() }
    }
}

/// A shadow of the game's things, which have no ground: as dark everywhere.
pub fn thing_shadow(mask: &Mask) -> Shade {
    Shade { factor: mask.map(|cell| if cell { THING_SHADOW } else { 0.0 }), colour: Grid::new(mask.w, mask.h), grounds: Vec::new() }
}

/// The shadow colours the game draws on each colour of its grounds, and how
/// dark each is against that ground as a whole (`shadow=` of the catalogue).
#[derive(Clone, Debug, Default)]
pub struct Ramps(pub HashMap<i64, (Vec<[u8; 3]>, Vec<f64>)>);

/// The nearest colour of the DS: 5 bits a channel, shown as 8k + 7.
fn snap(value: f64) -> u8 {
    let level = ((value - 7.0) / 8.0).round_ties_even().clamp(0.0, 31.0);
    (level * 8.0 + 7.0) as u8
}

/// The ground `under` with the shade over it: the pixels it darkens made so.
pub fn shaded(under: &mut [Rgba], shade: &Shade, ramps: &Ramps) {
    for (at, pixel) in under.iter_mut().enumerate() {
        let factor = shade.factor.cells[at];
        if factor <= 0.0 {
            continue;
        }
        let ground = [pixel[0], pixel[1], pixel[2]];
        // (Barely darker or lighter: a patch of its own ground, nothing on another.)
        let factor = if (factor - 1.0).abs() < FAINT { 1.0f32 } else { factor };
        let mut scaled = if factor == 1.0 { ground } else { ground.map(|part| snap(part as f64 * factor as f64)) };
        // (Where the ground is one of the game's, its own shadow colour as dark.)
        if let Some((shades, ratios)) = ramps.0.get(&key(ground)).filter(|_| factor != 1.0) {
            let distance = |ratio: f64| (ratio - factor as f64).abs();
            let best = (0..ratios.len()).fold(0, |best, at| if distance(ratios[at]) < distance(ratios[best]) { at } else { best });
            if distance(ratios[best]) <= RAMP_NEAR {
                scaled = shades[best];
            }
        }
        let kept = has(&shade.grounds, key(ground));
        let made = if kept { shade.colour.cells[at] } else { scaled };
        *pixel = [made[0], made[1], made[2], pixel[3]];
    }
}

/// The tiles round a rectangle of a place that are its plain ground: seen
/// often in the place, of the ground's colours only.
fn ring_tiles(source: &Source, rect: (i64, i64, i64, i64), palette: &[i64]) -> Vec<Tile> {
    let (x0, y0, x1, y1) = rect;
    let picture = source.picture();
    let (width, height) = (picture.w as i64, picture.h as i64);
    let tile = TILE as i64;
    let counts = source.tile_counts();
    let (left, top) = ((x0.div_euclid(tile) - RING).max(0) * tile, (y0.div_euclid(tile) - RING).max(0) * tile);
    let right = ((x1 + tile - 1).div_euclid(tile) + RING).min(width / tile) * tile;
    let bottom = ((y1 + tile - 1).div_euclid(tile) + RING).min(height / tile) * tile;
    let mut found = Vec::new();
    let mut ty = top;
    while ty < bottom {
        let mut tx = left;
        while tx < right {
            if !(x0 <= tx && tx < x1 && y0 <= ty && ty < y1) {
                let tile = tile_at(picture, tx as usize, ty as usize);
                if counts.get(&tile).unwrap_or(0) >= COMMON && tile.chunks(3).all(|rgb| has(palette, key([rgb[0], rgb[1], rgb[2]]))) {
                    found.push(tile);
                }
            }
            tx += TILE as i64;
        }
        ty += TILE as i64;
    }
    found
}

/// The ground under each pixel of a rectangle of a place: tile by tile, the
/// ground tile most like it. Returns the ground and where the place shows it.
fn ground_of(source: &Source, rect: (i64, i64, i64, i64), tiles: &[Tile], palette: &[i64]) -> (Grid<[u8; 3]>, Mask) {
    let (x0, y0, x1, y1) = rect;
    let tile = TILE as i64;
    let mut unique_tiles: BTreeMap<Tile, ()> = BTreeMap::new();
    for one in tiles.iter().cloned().chain(ring_tiles(source, rect, palette)) {
        unique_tiles.insert(one, ());
    }
    let stack: Vec<Tile> = unique_tiles.into_keys().collect();
    let picture = source.picture();
    let (left, top) = (x0.div_euclid(tile) * tile, y0.div_euclid(tile) * tile);
    let (right, bottom) = ((x1 + tile - 1).div_euclid(tile) * tile, (y1 + tile - 1).div_euclid(tile) * tile);
    let (w, h) = ((right - left) as usize, (bottom - top) as usize);
    let mut region: Grid<[u8; 3]> = Grid::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (left as usize + x, top as usize + y);
            if px < picture.w && py < picture.h {
                region.set(x, y, source.rgb(px, py));
            }
        }
    }
    let mut ground: Grid<[u8; 3]> = Grid::new(w, h);
    let mut trusted = Mask::filled(w, h, true);
    for ty in (0..h).step_by(TILE) {
        for tx in (0..w).step_by(TILE) {
            let alike = |one: &Tile| {
                (0..TILE * TILE).filter(|at| region.get(tx + at % TILE, ty + at / TILE) == [one[at * 3], one[at * 3 + 1], one[at * 3 + 2]]).count()
            };
            let (mut best, mut most) = (0, 0);
            for (at, one) in stack.iter().enumerate() {
                let count = alike(one);
                if count > most || at == 0 {
                    (best, most) = (at, count);
                }
            }
            for at in 0..TILE * TILE {
                let one = stack.get(best);
                let rgb = one.map_or([0; 3], |one| [one[at * 3], one[at * 3 + 1], one[at * 3 + 2]]);
                ground.set(tx + at % TILE, ty + at / TILE, rgb);
                if most < FEW {
                    trusted.set(tx + at % TILE, ty + at / TILE, false);
                }
            }
        }
    }
    let (cx, cy) = ((x0 - left) as usize, (y0 - top) as usize);
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let ground = ground.crop(cx, cy, cx + rw, cy + rh);
    let region = region.crop(cx, cy, cx + rw, cy + rh);
    let trusted = trusted.crop(cx, cy, cx + rw, cy + rh);
    let exact = Grid { w: rw, h: rh, cells: (0..rw * rh).map(|at| region.cells[at] == ground.cells[at] && trusted.cells[at]).collect() };
    (ground, exact)
}

/// The colours of a ground that are its own, not a flower or a leaf in one of
/// its tiles: those of `palette` seldom seen in `tiles` left out.
pub fn own_colours(tiles: &[Tile], palette: &[i64]) -> Vec<i64> {
    if tiles.is_empty() {
        return palette.to_vec();
    }
    let every: Vec<i64> = tiles.iter().flat_map(|tile| tile.chunks(3).map(|rgb| key([rgb[0], rgb[1], rgb[2]]))).collect();
    let total = every.len() as f64;
    let seldom: Vec<i64> = counted(every).into_iter().filter(|(_, count)| (*count as f64) < OWN * total).map(|(colour, _)| colour).collect();
    palette.iter().copied().filter(|colour| !has(&seldom, *colour)).collect()
}

/// Cuts the rectangle `rect` of a place out of its ground (`tiles`: the
/// ground tiles it may stand on, `palette`: their colours, `shadows`: the
/// colours of their shadows the game draws). What is the thing, its shade.
pub fn matte(source: &Source, rect: (i64, i64, i64, i64), tiles: &[Tile], palette: &[i64], shadows: &[i64], whole: bool) -> (Mask, Shade) {
    let (x0, y0, x1, y1) = rect;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let seen: Grid<[u8; 3]> = Grid { w, h, cells: (0..w * h).map(|at| source.rgb(x0 as usize + at % w, y0 as usize + at / w)).collect() };
    let (under, exact) = ground_of(source, rect, tiles, palette);
    // (How much darker than the ground as a whole.)
    let under_light: Vec<f64> = under.cells.iter().map(|rgb| light(*rgb)).collect();
    let whole_light = mean(&under_light).max(1.0);
    let factor: Vec<f64> = seen.cells.iter().map(|rgb| light(*rgb) / whole_light).collect();
    let keys: Vec<i64> = seen.cells.iter().map(|rgb| key(*rgb)).collect();
    let shadows = unique(shadows.iter().copied());
    let own = unique(own_colours(tiles, palette));
    let mask = |test: &dyn Fn(usize) -> bool| Grid { w, h, cells: (0..w * h).map(test).collect::<Vec<bool>>() };
    let said = mask(&|at| has(&shadows, keys[at]));
    let variant = mask(&|at| has(&own, keys[at]));
    let is_ground = mask(&|at| exact.cells[at] || (variant.cells[at] && !said.cells[at]));
    let is_shade = mask(&|at| !is_ground.cells[at] && said.cells[at]);
    let loose = is_ground.or(&is_shade);
    let edge = border(w, h);
    let mut outside = component(&loose, &edge);
    for part in components(&loose.minus(&outside)) {
        if part.and(&exact).count() >= HOLE {
            outside = outside.or(&part);
        }
    }
    let mut thing = outside.not();
    // (Specks of the thing alone in the ground are ground.)
    let parts = components(&thing);
    if let Some(largest) = parts.first().map(Mask::count) {
        for part in &parts[1..] {
            let least = if whole { SPECK } else { SPECK.max(LESSER * largest as f64) };
            if (part.count() as f64) < least {
                thing = thing.minus(part);
                outside = outside.or(part);
            }
        }
    }
    // (Its rim in the ground's own colours and shadows: peeled from the outside in.)
    let near = mask(&|at| has(&own, keys[at]) || said.cells[at]);
    for _ in 0..PEEL {
        let rim = thing.and(&grown(&component(&thing.not(), &edge), 1, 1));
        let peeled = rim.and(&near);
        if !peeled.any() {
            break;
        }
        thing = thing.minus(&peeled);
        outside = outside.or(&peeled);
    }
    // (A hole inside it is of it; unless it is the ground itself.)
    let holes = thing.not().minus(&component(&thing.not(), &edge));
    for part in components(&holes) {
        let cells: Vec<usize> = (0..w * h).filter(|at| part.cells[*at]).collect();
        let plain = cells.iter().filter(|at| has(&own, keys[**at]) || said.cells[**at]).count();
        if (plain as f64 / cells.len() as f64) < PLAIN_HOLE {
            thing = thing.or(&part);
            outside = outside.minus(&part);
        }
    }
    // (A shadow darkens, and is that of the thing kept: by it, or joined to it.)
    let mut shade_mask = mask(&|at| outside.cells[at] && is_shade.cells[at] && factor[at] < 1.0);
    for part in components(&shade_mask) {
        if !grown(&part, 2, 1).and(&thing).any() {
            shade_mask = shade_mask.minus(&part);
        }
    }
    let shade = Shade {
        factor: Grid { w, h, cells: (0..w * h).map(|at| if shade_mask.cells[at] { factor[at] as f32 } else { 0.0 }).collect() },
        colour: Grid { w, h, cells: (0..w * h).map(|at| if shade_mask.cells[at] { seen.cells[at] } else { [0; 3] }).collect() },
        grounds: unique(palette.iter().copied()),
    };
    (thing, shade)
}
