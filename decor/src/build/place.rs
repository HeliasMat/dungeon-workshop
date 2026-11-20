//! A place being laid: its layers of every frame, the palettes that change
//! with time, its walls, its paths, what was laid where; a piece laid on it,
//! its shadow dropped on what is under.

use std::collections::{BTreeMap, BTreeSet};

use crate::cut::Cut;
use crate::fault::{Diagnostic, Location};
use crate::lang::Decor;
use crate::matte::{Shade, shaded};
use crate::pixels::{Grid, Mask, Picture, TILE};
use crate::weave::Workshop;

/// The most palettes changing with time a place of ours has.
const PALETTES: usize = 16;

/// A place being laid: its layers of every frame, which palette colour shows
/// where colours change with time, its walls, its paths, what was laid where.
#[derive(Clone)]
pub struct Place {
    pub decor: Decor,
    pub lower: Vec<Picture>,
    pub upper: Vec<Picture>,
    pub lower_index: Vec<Grid<u8>>,
    pub upper_index: Vec<Grid<u8>>,
    /// By tiles.
    pub walls: Mask,
    pub tree_walls: Mask,
    /// By pixels.
    pub path: Mask,
    pub paths: BTreeMap<usize, Mask>,
    pub shaded: Mask,
    /// Which thing laid each pixel; which laying put each pixel of the ground.
    pub owner: Grid<i32>,
    pub origin: Grid<i32>,
    pub layings: i32,
    /// The rows of tiles a band was laid on.
    pub banded: Vec<bool>,
    pub names: Vec<String>,
    /// (Level, palette) of a source -> our palette, in the order given.
    pub slots: Vec<((i64, u64), u64)>,
    pub cycles: Vec<crate::source::Cycle>,
    pub seconds: f64,
    pub remarks: Vec<Diagnostic>,
    /// Woven tiles that meet badly: line, x, y, cost.
    pub seams: Vec<(usize, usize, usize, i64)>,
    /// Pieces that meet badly: x0, y0, x1, y1, pixels.
    pub joins: Vec<(usize, usize, usize, usize, usize)>,
    pub join_mask: Mask,
    pub levels: BTreeSet<i64>,
    /// Where paths end: line, x, y.
    pub ends: Vec<(usize, i64, i64)>,
    pub trees: Vec<(usize, i64, i64)>,
}

/// What is laid on a layer: its frames, which palette colour each pixel
/// shows where its colours change with time, the place it comes from (-1:
/// one of the game's things).
struct Laid<'a> {
    frames: &'a [Picture],
    index: Option<&'a Vec<Grid<u8>>>,
    level: i64,
}

/// The two layers of a place.
#[derive(Clone, Copy, PartialEq)]
enum Layer {
    Lower,
    Upper,
}

impl Place {
    /// The place as the game shows it so many seconds in, its own size, a
    /// pixel of it a pixel (RGBA): its lower layer at the frame its
    /// animation is on, its upper one over it, its colours that change with
    /// time as they are then. For one who edits it, before it is written.
    pub fn picture_at(&self, seconds: f64) -> (usize, usize, Vec<u8>) {
        let (w, h, lower, upper) = self.layers_at(seconds);
        let rgba =
            lower.as_chunks::<4>().0.iter().zip(upper.as_chunks::<4>().0).flat_map(|(below, over)| if over[3] > 0 { over } else { below }).copied().collect();
        (w, h, rgba)
    }

    /// The same, its two layers apart: the lower one (under who passes),
    /// the upper one (over), each RGBA.
    pub fn layers_at(&self, seconds: f64) -> (usize, usize, Vec<u8>, Vec<u8>) {
        let Some(first) = self.lower.first() else {
            return (0, 0, Vec::new(), Vec::new());
        };
        let (w, h) = (first.w, first.h);
        let frame = (seconds / self.seconds.max(0.01)) as usize % self.lower.len();
        // (The colours of each palette that changes, at that moment.)
        let ticks = (seconds * 60.0).max(0.0) as usize;
        let mut shown: [Option<&[u8]>; 16] = [None; 16];
        for (palette, (each, steps)) in &self.cycles {
            if let Some(slot) = shown.get_mut(*palette as usize)
                && !steps.is_empty()
            {
                *slot = Some(steps[ticks / (*each).max(1) as usize % steps.len()].as_slice());
            }
        }
        let cycled = |index: Option<&Grid<u8>>, at: usize, rgba: &mut [u8; 4]| {
            let Some(index) = index.and_then(|grid| grid.cells.get(at)).copied().filter(|index| index & 0xF != 0) else {
                return;
            };
            if let Some(colours) = shown[(index >> 4) as usize] {
                let colour = (index & 0xF) as usize * 3;
                if let Some(said) = colours.get(colour..colour + 3) {
                    rgba[..3].copy_from_slice(said);
                }
            }
        };
        let (lower_index, upper_index) = (self.lower_index.get(frame), self.upper_index.get(frame));
        let upper = self.upper.get(frame % self.upper.len().max(1));
        let (mut below, mut above) = (Vec::with_capacity(w * h * 4), Vec::with_capacity(w * h * 4));
        for at in 0..w * h {
            let mut seen = self.lower[frame].cells.get(at).copied().unwrap_or(first.cells[at]);
            cycled(lower_index, at, &mut seen);
            below.extend_from_slice(&seen);
            let mut over = upper.and_then(|upper| upper.cells.get(at)).copied().unwrap_or_default();
            if over[3] > 0 {
                cycled(upper_index, at, &mut over);
            }
            above.extend_from_slice(&over);
        }
        (w, h, below, above)
    }

    pub(crate) fn new(decor: Decor, frames: usize) -> Place {
        let (w, h) = (decor.width, decor.height);
        Place {
            lower: vec![Picture::new(w, h); frames],
            upper: vec![Picture::new(w, h); frames],
            lower_index: vec![Grid::new(w, h); frames],
            upper_index: vec![Grid::new(w, h); frames],
            walls: Mask::new(w / TILE, h / TILE),
            tree_walls: Mask::new(w / TILE, h / TILE),
            path: Mask::new(w, h),
            paths: BTreeMap::new(),
            shaded: Mask::new(w, h),
            owner: Grid::new(w, h),
            origin: Grid::new(w, h),
            layings: 0,
            banded: vec![false; h / TILE],
            names: vec![String::new()],
            slots: Vec::new(),
            cycles: Vec::new(),
            seconds: 1.0 / 6.0,
            remarks: Vec::new(),
            seams: Vec::new(),
            joins: Vec::new(),
            join_mask: Mask::new(w, h),
            levels: BTreeSet::new(),
            ends: Vec::new(),
            trees: Vec::new(),
            decor,
        }
    }

    /// A dungeon room (`room`): one frame, no layer over; the colours that
    /// change with time and which each pixel shows; its walls by tiles.
    pub fn room(decor: Decor, picture: Picture, indices: Grid<u8>, cycles: Vec<crate::source::Cycle>, walls: Mask) -> Place {
        let mut place = Place::new(decor, 1);
        (place.lower, place.lower_index, place.cycles, place.walls) = (vec![picture], vec![indices], cycles, walls);
        place
    }

    /// The place cut to that size (laid on whole tiles, a side may be any):
    /// its pictures and what is by pixel; its walls stay by whole tiles.
    pub(crate) fn cut_to(&mut self, w: usize, h: usize) {
        if (w, h) == (self.decor.width, self.decor.height) {
            return;
        }
        for frame in self.lower.iter_mut().chain(self.upper.iter_mut()) {
            *frame = frame.crop(0, 0, w, h);
        }
        for frame in self.lower_index.iter_mut().chain(self.upper_index.iter_mut()) {
            *frame = frame.crop(0, 0, w, h);
        }
        for mask in [&mut self.path, &mut self.shaded, &mut self.join_mask].into_iter().chain(self.paths.values_mut()) {
            *mask = mask.crop(0, 0, w, h);
        }
        self.owner = self.owner.crop(0, 0, w, h);
        self.origin = self.origin.crop(0, 0, w, h);
        (self.decor.width, self.decor.height) = (w, h);
    }

    pub fn remark(&mut self, line: usize, said: String) {
        self.remarks.push(Diagnostic::at(line, said).as_remark());
    }

    pub(super) fn new_laying(&mut self) -> i32 {
        self.layings += 1;
        self.layings
    }

    /// Colours changing with time: each palette of a source gets one of ours.
    pub(super) fn table(&mut self, workshop: &Workshop, level: i64) -> Result<[u8; 256], Diagnostic> {
        let source = workshop.source(level)?;
        let mut out = [0u8; 256];
        for (palette, cycle) in &source.info.cycles {
            let slot = match self.slots.iter().find(|(known, _)| *known == (level, *palette)) {
                Some((_, slot)) => *slot,
                None => {
                    if self.slots.len() >= PALETTES {
                        return Err(Location::in_file(self.decor.file.clone(), 0).wrong(format!("more than {PALETTES} changing palettes")));
                    }
                    // (The first not taken: a dungeon's floor keeps its own.)
                    let slot = (0..).find(|slot| !self.slots.iter().any(|(_, known)| known == slot)).unwrap_or(0);
                    self.slots.push(((level, *palette), slot));
                    self.cycles.push((slot, cycle.clone()));
                    slot
                }
            };
            for colour in 1..16u64 {
                out[(palette << 4 | colour) as usize & 255] = (slot << 4 | colour) as u8;
            }
        }
        Ok(out)
    }

    /// The part of a picture at x, y inside the place, if any.
    pub fn clip(&self, x: i64, y: i64, w: usize, h: usize) -> Option<(usize, usize, usize, usize)> {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w as i64).min(self.decor.width as i64), (y + h as i64).min(self.decor.height as i64));
        (x0 < x1 && y0 < y1).then_some((x0 as usize, y0 as usize, x1 as usize, y1 as usize))
    }

    /// Frames of a picture over a layer at x, y, what is opaque of it; its
    /// colours that change with time with it. What of it was seen.
    fn lay(&mut self, workshop: &Workshop, layer: Layer, laid: Laid, (x, y): (i64, i64), laying: i32) -> Result<Mask, Diagnostic> {
        let Laid { frames, index, level } = laid;
        let (w, h) = (frames[0].w, frames[0].h);
        let Some((x0, y0, x1, y1)) = self.clip(x, y, w, h) else {
            return Ok(Mask::new(0, 0));
        };
        let table = match index {
            Some(_) => Some(self.table(workshop, level)?),
            None => None,
        };
        let (bw, bh) = (x1 - x0, y1 - y0);
        let mut any_seen = Mask::new(bw, bh);
        let count = self.lower.len();
        for frame in 0..count {
            let part = &frames[frame % frames.len()];
            for by in 0..bh {
                for bx in 0..bw {
                    let (px, py) = ((x0 + bx) as i64 - x, (y0 + by) as i64 - y);
                    let rgba = part.get(px as usize, py as usize);
                    if rgba[3] == 0 {
                        continue;
                    }
                    any_seen.set(bx, by, true);
                    let shown = match (&table, index) {
                        (Some(table), Some(index)) => table[index[frame % index.len()].get(px as usize, py as usize) as usize],
                        _ => 0,
                    };
                    let (pictures, indices) = match layer {
                        Layer::Lower => (&mut self.lower, &mut self.lower_index),
                        Layer::Upper => (&mut self.upper, &mut self.upper_index),
                    };
                    pictures[frame].set(x0 + bx, y0 + by, rgba);
                    indices[frame].set(x0 + bx, y0 + by, shown);
                }
            }
        }
        if level >= 0 {
            self.levels.insert(level);
        }
        if layer == Layer::Lower {
            for by in 0..bh {
                for bx in 0..bw {
                    if any_seen.get(bx, by) {
                        self.origin.set(x0 + bx, y0 + by, laying);
                    }
                }
            }
        }
        Ok(any_seen)
    }

    /// A shade over the lower layer at x, y: its ground made darker or lighter.
    pub(super) fn shade(&mut self, workshop: &Workshop, shade: &Shade, x: i64, y: i64) {
        let (w, h) = (shade.factor.w, shade.factor.h);
        let Some((x0, y0, x1, y1)) = self.clip(x, y, w, h) else {
            return;
        };
        if !shade.mask().any() {
            return;
        }
        let (rx, ry) = ((x0 as i64 - x) as usize, (y0 as i64 - y) as usize);
        let part = shade.part(rx, ry, rx + x1 - x0, ry + y1 - y0);
        let mask = part.mask();
        let (bw, bh) = (x1 - x0, y1 - y0);
        for by in 0..bh {
            for bx in 0..bw {
                if mask.get(bx, by) {
                    self.shaded.set(x0 + bx, y0 + by, true);
                }
            }
        }
        for frame in 0..self.lower.len() {
            let mut region: Vec<[u8; 4]> = (0..bw * bh).map(|at| self.lower[frame].get(x0 + at % bw, y0 + at / bw)).collect();
            shaded(&mut region, &part, &workshop.ramps);
            for (at, mut rgba) in region.into_iter().enumerate() {
                let (px, py) = (x0 + at % bw, y0 + at / bw);
                if mask.cells[at] {
                    rgba[3] = 255;
                    self.lower_index[frame].set(px, py, 0);
                }
                self.lower[frame].set(px, py, rgba);
            }
        }
    }

    /// Walls of a thing laid: a tile it covers whole is as the thing says;
    /// one it covers in part is a wall if a third of it is.
    pub(crate) fn wall(&mut self, seen: &Mask, solid: &Mask, (x0, y0): (usize, usize)) {
        let (w, h) = (seen.w, seen.h);
        for ty in y0 / TILE..(y0 + h).div_ceil(TILE) {
            for tx in x0 / TILE..(x0 + w).div_ceil(TILE) {
                let (top, bottom) = ((ty * TILE).saturating_sub(y0), ((ty + 1) * TILE - y0).min(h));
                let (left, right) = ((tx * TILE).saturating_sub(x0), ((tx + 1) * TILE - x0).min(w));
                if top >= bottom || left >= right {
                    continue;
                }
                let (mut covers, mut walled) = (0, 0);
                for y in top..bottom {
                    for x in left..right {
                        covers += usize::from(seen.get(x, y));
                        walled += usize::from(solid.get(x, y));
                    }
                }
                let walled = walled * 3 >= TILE * TILE;
                if covers == TILE * TILE {
                    self.walls.set(tx, ty, walled);
                } else if walled {
                    self.walls.set(tx, ty, true);
                }
            }
        }
    }

    /// A place of the game under all, at x, y, as it is: its layers, its
    /// colours that change with time, its walls (on the grid of 8).
    pub(super) fn lay_level(&mut self, workshop: &Workshop, level: i64, x: i64, y: i64) -> Result<(), Diagnostic> {
        let source = workshop.source(level)?;
        let laying = self.new_laying();
        self.lay(workshop, Layer::Lower, Laid { frames: &source.frames, index: source.indices.as_ref(), level }, (x, y), laying)?;
        if !source.upper.is_empty() {
            self.lay(workshop, Layer::Upper, Laid { frames: &source.upper, index: source.upper_indices.as_ref(), level }, (x, y), laying)?;
        }
        if let Some(walls) = &source.walls {
            let (dx, dy) = (x.div_euclid(TILE as i64), y.div_euclid(TILE as i64));
            for row in 0..walls.h {
                for column in 0..walls.w {
                    let (tx, ty) = (column as i64 + dx, row as i64 + dy);
                    if (0..self.walls.w as i64).contains(&tx) && (0..self.walls.h as i64).contains(&ty) {
                        self.walls.set(tx as usize, ty as usize, walls.get(column, row));
                    }
                }
            }
        }
        Ok(())
    }

    /// Holes in its lower layer filled with its ground (`fill_holes`).
    pub(super) fn fill_holes(&mut self, holes: &Mask) {
        fill_holes(&mut self.lower, &mut self.lower_index, holes);
    }

    /// A dungeon's floor under all, from its top left corner: its picture in
    /// every frame, its colours that change with time as they are, its walls.
    pub(super) fn lay_room(&mut self, made: &crate::room::Laid) {
        let (w, h) = (made.picture.w.min(self.decor.width), made.picture.h.min(self.decor.height));
        for frame in 0..self.lower.len() {
            for y in 0..h {
                for x in 0..w {
                    self.lower[frame].set(x, y, made.picture.get(x, y));
                    self.lower_index[frame].set(x, y, made.indices.get(x, y));
                }
            }
        }
        // (Its palettes keep their numbers: what each pixel shows is the tileset's.)
        for (palette, cycle) in &made.cycles {
            self.slots.push(((-1000 - made.tileset, *palette), *palette));
            self.cycles.push((*palette, cycle.clone()));
        }
        for row in 0..made.walls.h.min(self.walls.h) {
            for column in 0..made.walls.w.min(self.walls.w) {
                self.walls.set(column, row, made.walls.get(column, row));
            }
        }
    }

    /// One of our own pictures at x, y, as it is (never made larger): over
    /// the ground (`ground`, `objects`) or before who passes (`over`); no
    /// wall of its own (`wall` says where one may not walk).
    pub(super) fn lay_picture(&mut self, workshop: &Workshop, pictures: &[Picture], x: i64, y: i64, owner: i32, layer: Option<&str>) -> Result<(), Diagnostic> {
        let laying = self.new_laying();
        let on = if layer == Some("over") { Layer::Upper } else { Layer::Lower };
        let picture = &pictures[0];
        let seen = self.lay(workshop, on, Laid { frames: pictures, index: None, level: -1 }, (x, y), laying)?;
        if let Some((x0, y0, _, _)) = self.clip(x, y, picture.w, picture.h) {
            for by in 0..seen.h {
                for bx in 0..seen.w {
                    if seen.get(bx, by) {
                        self.owner.set(x0 + bx, y0 + by, owner);
                    }
                }
            }
        }
        Ok(())
    }

    /// A piece at x, y: its shade, its lower and upper layers (or all on one,
    /// `layer`), its walls.
    pub(super) fn lay_cut(&mut self, workshop: &Workshop, cut: &Cut, x: i64, y: i64, owner: i32, layer: Option<&str>) -> Result<(), Diagnostic> {
        let laying = self.new_laying();
        self.shade(workshop, &cut.shade, x, y);
        let (mut lower, mut upper) = (cut.lower.clone(), cut.upper.clone());
        let (mut lower_index, mut upper_index) = (cut.lower_index.as_ref(), cut.upper_index.as_ref());
        if let Some(layer) = layer {
            // (Its colours that change with time stay as they are seen in frame 0.)
            let blank = |frames: &Vec<Picture>| -> Vec<Picture> { frames.iter().map(|frame| Picture::new(frame.w, frame.h)).collect() };
            (lower, upper) = if layer == "over" { (blank(&cut.lower), merged(cut)) } else { (merged(cut), blank(&cut.upper)) };
            (lower_index, upper_index) = (None, None);
        }
        let level = cut.level;
        let seen_lower = self.lay(workshop, Layer::Lower, Laid { frames: &lower, index: lower_index, level }, (x, y), laying)?;
        let seen_upper = self.lay(workshop, Layer::Upper, Laid { frames: &upper, index: upper_index, level }, (x, y), laying)?;
        let (w, h) = (cut.solid.w, cut.solid.h);
        let Some((x0, y0, x1, y1)) = self.clip(x, y, w, h) else {
            return Ok(());
        };
        let (bw, bh) = (x1 - x0, y1 - y0);
        let mut seen = Mask::new(bw, bh);
        for part in [&seen_lower, &seen_upper] {
            if !part.cells.is_empty() {
                seen = seen.or(part);
            }
        }
        let (rx, ry) = ((x0 as i64 - x) as usize, (y0 as i64 - y) as usize);
        self.wall(&seen, &cut.solid.crop(rx, ry, rx + bw, ry + bh), (x0, y0));
        if owner != 0 {
            for by in 0..bh {
                for bx in 0..bw {
                    if seen.get(bx, by) {
                        self.owner.set(x0 + bx, y0 + by, owner);
                    }
                }
            }
        }
        Ok(())
    }
}

/// A piece's two layers as one, frame by frame.
fn merged(cut: &Cut) -> Vec<Picture> {
    let frames = cut.lower.len().max(cut.upper.len());
    (0..frames)
        .map(|frame| {
            let mut out = cut.lower[frame % cut.lower.len()].clone();
            let over = &cut.upper[frame % cut.upper.len()];
            for (cell, rgba) in out.cells.iter_mut().zip(&over.cells) {
                if rgba[3] > 0 {
                    *cell = *rgba;
                }
            }
            out
        })
        .collect()
}

pub(super) fn lcm(numbers: &[usize]) -> usize {
    fn gcd(a: usize, b: usize) -> usize {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    numbers.iter().fold(1, |out, number| out * number / gcd(out, *number))
}

/// Holes in the lower layer filled with its ground: each tile with a
/// hole takes, where it is a hole, the pixels of the nearest whole tile,
/// one seen elsewhere too if one is near (its ground, not a thing), in
/// every frame, with its colours that change with time.
pub(crate) fn fill_holes(lower: &mut [Picture], lower_index: &mut [Grid<u8>], holes: &Mask) {
    let (w, h) = (holes.w / TILE, holes.h / TILE);
    if w == 0 || h == 0 {
        return;
    }
    let holed = |column: usize, row: usize| (0..TILE).any(|dy| (0..TILE).any(|dx| holes.get(column * TILE + dx, row * TILE + dy)));
    let content =
        |column: usize, row: usize| -> Vec<[u8; 4]> { (0..TILE * TILE).map(|at| lower[0].get(column * TILE + at % TILE, row * TILE + at / TILE)).collect() };
    // (How often each whole tile is seen: a ground's are seen again and again.)
    let mut seen: std::collections::HashMap<Vec<[u8; 4]>, usize> = std::collections::HashMap::new();
    for row in 0..h {
        for column in 0..w {
            if !holed(column, row) {
                *seen.entry(content(column, row)).or_default() += 1;
            }
        }
    }
    let whole = |column: usize, row: usize| !holed(column, row) && content(column, row).iter().all(|pixel| pixel[3] > 0);
    let mut fills: Vec<((usize, usize), (usize, usize))> = Vec::new();
    for row in 0..h {
        for column in 0..w {
            if !holed(column, row) {
                continue;
            }
            let mut best: Option<((usize, usize), bool)> = None;
            'far: for far in 1..=24usize {
                for (dc, dr) in (0..=far as i64).flat_map(|a| {
                    let b = far as i64 - a;
                    [(a, b), (-a, b), (a, -b), (-a, -b)]
                }) {
                    let (c, r) = (column as i64 + dc, row as i64 + dr);
                    if !(0..w as i64).contains(&c) || !(0..h as i64).contains(&r) || !whole(c as usize, r as usize) {
                        continue;
                    }
                    let ground = seen.get(&content(c as usize, r as usize)).copied().unwrap_or(0) > 1;
                    if best.is_none_or(|(_, known)| ground && !known) {
                        best = Some(((c as usize, r as usize), ground));
                    }
                    if ground {
                        break 'far;
                    }
                }
            }
            if let Some((from, _)) = best {
                fills.push(((column, row), from));
            }
        }
    }
    for ((column, row), (from_column, from_row)) in fills {
        for at in 0..TILE * TILE {
            let (dx, dy) = (at % TILE, at / TILE);
            let (x, y) = (column * TILE + dx, row * TILE + dy);
            if !holes.get(x, y) {
                continue;
            }
            let (fx, fy) = (from_column * TILE + dx, from_row * TILE + dy);
            for picture in lower.iter_mut() {
                let pixel = picture.get(fx, fy);
                picture.set(x, y, pixel);
            }
            for index in lower_index.iter_mut() {
                let shown = index.get(fx, fy);
                index.set(x, y, shown);
            }
        }
    }
}
