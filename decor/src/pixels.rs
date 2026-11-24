//! What places are made of: grids of cells (pictures, palette colours,
//! masks), colours as numbers, masks grown and parts joined.

use std::collections::VecDeque;

/// The size of a tile of a place.
pub const TILE: usize = 8;

/// A grid of cells, row after row: `cells[y * w + x]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Grid<T> {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<T>,
}

/// A colour and how much of it is seen (0: none).
pub type Rgba = [u8; 4];
pub type Picture = Grid<Rgba>;
pub type Mask = Grid<bool>;

impl<T: Copy + Default> Grid<T> {
    pub fn new(w: usize, h: usize) -> Self {
        Grid { w, h, cells: vec![T::default(); w * h] }
    }

    pub fn filled(w: usize, h: usize, value: T) -> Self {
        Grid { w, h, cells: vec![value; w * h] }
    }

    pub fn get(&self, x: usize, y: usize) -> T {
        self.cells[y * self.w + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) {
        self.cells[y * self.w + x] = value;
    }

    pub fn at(&mut self, x: usize, y: usize) -> &mut T {
        &mut self.cells[y * self.w + x]
    }

    /// The part from x0, y0 to x1, y1 (not included).
    pub fn crop(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Self {
        let mut out = Grid::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            out.cells[(y - y0) * out.w..(y - y0 + 1) * out.w].copy_from_slice(&self.cells[y * self.w + x0..y * self.w + x1]);
        }
        out
    }

    /// The same cells, each made another.
    pub fn map<U: Copy + Default>(&self, made: impl Fn(T) -> U) -> Grid<U> {
        Grid { w: self.w, h: self.h, cells: self.cells.iter().map(|cell| made(*cell)).collect() }
    }

    /// Turned over left to right.
    pub fn mirrored(&self) -> Self {
        let mut out = self.clone();
        for y in 0..self.h {
            out.cells[y * self.w..(y + 1) * self.w].reverse();
        }
        out
    }
}

impl Mask {
    pub fn any(&self) -> bool {
        self.cells.iter().any(|cell| *cell)
    }

    pub fn count(&self) -> usize {
        self.cells.iter().filter(|cell| **cell).count()
    }

    pub fn and(&self, other: &Mask) -> Mask {
        Grid { w: self.w, h: self.h, cells: self.cells.iter().zip(&other.cells).map(|(a, b)| *a && *b).collect() }
    }

    pub fn or(&self, other: &Mask) -> Mask {
        Grid { w: self.w, h: self.h, cells: self.cells.iter().zip(&other.cells).map(|(a, b)| *a || *b).collect() }
    }

    pub fn not(&self) -> Mask {
        self.map(|cell| !cell)
    }

    pub fn minus(&self, other: &Mask) -> Mask {
        Grid { w: self.w, h: self.h, cells: self.cells.iter().zip(&other.cells).map(|(a, b)| *a && !*b).collect() }
    }

    /// The first cell held, row by row: x, y.
    pub fn first(&self) -> Option<(usize, usize)> {
        self.cells.iter().position(|cell| *cell).map(|at| (at % self.w, at / self.w))
    }
}

/// Each colour as one number: `r << 16 | g << 8 | b`.
pub fn key(rgb: [u8; 3]) -> i64 {
    (rgb[0] as i64) << 16 | (rgb[1] as i64) << 8 | rgb[2] as i64
}

pub fn key_of(rgba: Rgba) -> i64 {
    key([rgba[0], rgba[1], rgba[2]])
}

pub fn rgb_of(key: i64) -> [u8; 3] {
    [(key >> 16) as u8, (key >> 8 & 255) as u8, (key & 255) as u8]
}

/// `rrggbb` as a colour.
pub fn colour(text: &str) -> Option<[u8; 3]> {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 {
        return None;
    }
    let part = |at: usize| u8::from_str_radix(&text[at..at + 2], 16).ok();
    Some([part(0)?, part(2)?, part(4)?])
}

/// A mask grown by `by` cells, `steps` times, side by side (none round the edges).
pub fn grown(mask: &Mask, steps: usize, by: usize) -> Mask {
    let mut out = mask.clone();
    for _ in 0..steps {
        let was = out.clone();
        for y in 0..mask.h {
            for x in 0..mask.w {
                if was.get(x, y) {
                    continue;
                }
                let near = (y >= by && was.get(x, y - by))
                    || (y + by < mask.h && was.get(x, y + by))
                    || (x >= by && was.get(x - by, y))
                    || (x + by < mask.w && was.get(x + by, y));
                if near {
                    out.set(x, y, true);
                }
            }
        }
    }
    out
}

/// The cells of `mask` joined to any of `starts` (x, y), side by side.
pub fn component(mask: &Mask, starts: &[(usize, usize)]) -> Mask {
    let mut seen = Mask::new(mask.w, mask.h);
    let mut waiting = VecDeque::new();
    for &(x, y) in starts {
        if mask.get(x, y) && !seen.get(x, y) {
            seen.set(x, y, true);
            waiting.push_back((x, y));
        }
    }
    while let Some((x, y)) = waiting.pop_front() {
        let near = [(x, y + 1), (x, y.wrapping_sub(1)), (x + 1, y), (x.wrapping_sub(1), y)];
        for (nx, ny) in near {
            if nx < mask.w && ny < mask.h && mask.get(nx, ny) && !seen.get(nx, ny) {
                seen.set(nx, ny, true);
                waiting.push_back((nx, ny));
            }
        }
    }
    seen
}

/// Each group of joined cells of a mask, largest first (those as large, in
/// the order they are found, row by row).
pub fn components(mask: &Mask) -> Vec<Mask> {
    let mut left = mask.clone();
    let mut found = Vec::new();
    while let Some(start) = left.first() {
        let part = component(&left, &[start]);
        left = left.minus(&part);
        found.push(part);
    }
    found.sort_by_key(|part| std::cmp::Reverse(part.count()));
    found
}

/// The cells on the edge of a rectangle (x, y).
pub fn border(w: usize, h: usize) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for y in 0..h {
        cells.push((0, y));
        cells.push((w - 1, y));
    }
    for x in 0..w {
        cells.push((x, 0));
        cells.push((x, h - 1));
    }
    cells
}

/// The mean of numbers (0 for none).
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
}

/// How light a colour is.
pub fn light(rgb: [u8; 3]) -> f64 {
    rgb[0] as f64 * 0.299 + rgb[1] as f64 * 0.587 + rgb[2] as f64 * 0.114
}

/// Numbers sorted, each once, and how many times each is.
pub fn counted(values: impl IntoIterator<Item = i64>) -> Vec<(i64, usize)> {
    let mut all: Vec<i64> = values.into_iter().collect();
    all.sort_unstable();
    let mut out: Vec<(i64, usize)> = Vec::new();
    for value in all {
        match out.last_mut() {
            Some((last, count)) if *last == value => *count += 1,
            _ => out.push((value, 1)),
        }
    }
    out
}

/// Numbers sorted, each once.
pub fn unique(values: impl IntoIterator<Item = i64>) -> Vec<i64> {
    counted(values).into_iter().map(|(value, _)| value).collect()
}

/// Whether a sorted list of numbers has one.
pub fn has(sorted: &[i64], value: i64) -> bool {
    sorted.binary_search(&value).is_ok()
}

/// How deep an object is worn away to find its heart; how many pixels a
/// heart has at least (a speck worn away is no object of its own).
const WORN: usize = 2;
const HEART: usize = 20;

/// The objects of a piece of a picture (`own`, near pieces together), one
/// each: its hearts (what is left of it worn `WORN` pixels deep, of `HEART`
/// pixels at least) each an object, every pixel to the nearest heart within
/// a few pixels' reach (a sign's post to its board, a flower's petals to it);
/// then two that touch pixel to pixel, of the same colours (`colour`), one
/// (a fence's posts). One object if it has one heart or none.
pub fn objects(own: &Mask, colour: &dyn Fn(usize, usize) -> [u8; 3]) -> Vec<Mask> {
    let (label, count) = object_labels(own, colour);
    let mut found = vec![Mask::new(own.w, own.h); count];
    for (cell, at) in label.cells.iter().enumerate() {
        if *at > 0 {
            found[*at as usize - 1].cells[cell] = true;
        }
    }
    found.retain(Mask::any);
    found
}

/// The same, as a number for each pixel of `own` (its object, from 1; 0
/// outside it), and how many: worked out on the room `own` takes only, its
/// hearts numbered on one grid (a forest's canopy has hundreds).
pub fn object_labels(own: &Mask, colour: &dyn Fn(usize, usize) -> [u8; 3]) -> (Grid<u32>, usize) {
    let mut out: Grid<u32> = Grid::new(own.w, own.h);
    let Some((x0, y0, x1, y1)) = bounds(own) else {
        return (out, 0);
    };
    // (Its room, and a few pixels round it for what grows.)
    const ROOM: usize = 4;
    let (x0, y0, x1, y1) = (x0.saturating_sub(ROOM), y0.saturating_sub(ROOM), (x1 + ROOM).min(own.w), (y1 + ROOM).min(own.h));
    let piece = own.crop(x0, y0, x1, y1);
    let (w, h) = (piece.w, piece.h);
    let colour = |x: usize, y: usize| colour(x + x0, y + y0);
    let worn = grown(&piece.not(), WORN, 1).not().and(&piece);
    // (Its hearts, numbered on one grid; the specks among them forgotten.)
    let mut heart: Grid<u32> = Grid::new(w, h);
    let mut sizes: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if !worn.cells[start] || heart.cells[start] > 0 {
            continue;
        }
        sizes.push(0);
        let number = sizes.len() as u32;
        let mut waiting = VecDeque::from([start]);
        heart.cells[start] = number;
        while let Some(cell) = waiting.pop_front() {
            sizes[number as usize - 1] += 1;
            let (x, y) = (cell % w, cell / w);
            for (nx, ny) in [(x + 1, y), (x.wrapping_sub(1), y), (x, y + 1), (x, y.wrapping_sub(1))] {
                if nx < w && ny < h && worn.get(nx, ny) && heart.get(nx, ny) == 0 {
                    heart.set(nx, ny, number);
                    waiting.push_back(ny * w + nx);
                }
            }
        }
    }
    let mut kept: Vec<u32> = vec![0; sizes.len() + 1];
    let mut count = 0usize;
    for (at, size) in sizes.iter().enumerate() {
        if *size >= HEART {
            count += 1;
            kept[at + 1] = count as u32;
        }
    }
    if count <= 1 {
        for y in 0..h {
            for x in 0..w {
                if piece.get(x, y) {
                    out.set(x + x0, y + y0, 1);
                }
            }
        }
        return (out, 1);
    }
    // (Each pixel to the nearest heart, through what is within reach.)
    let reach = grown(&piece, 3, 1);
    let mut label: Grid<u32> = Grid::new(w, h);
    let mut waiting = VecDeque::new();
    for cell in 0..w * h {
        let number = kept[heart.cells[cell] as usize];
        if number > 0 {
            label.cells[cell] = number;
            waiting.push_back(cell);
        }
    }
    while let Some(cell) = waiting.pop_front() {
        let (x, y) = (cell % w, cell / w);
        for (nx, ny) in [(x + 1, y), (x.wrapping_sub(1), y), (x, y + 1), (x, y.wrapping_sub(1))] {
            if nx < w && ny < h && reach.get(nx, ny) && label.get(nx, ny) == 0 {
                label.set(nx, ny, label.cells[cell]);
                waiting.push_back(ny * w + nx);
            }
        }
    }
    // (Those touching, of the same colours: one. The colours of each heart:
    // what it grew to may hold a neighbour's, a fence's rail.)
    let mut colours: Vec<std::collections::BTreeSet<[u8; 3]>> = vec![Default::default(); count];
    for cell in (0..w * h).filter(|cell| worn.cells[*cell] && label.cells[*cell] > 0) {
        // (Its own colours: not the dark of the outlines, that all have.)
        let [red, green, blue] = colour(cell % w, cell / w);
        if red as u32 * 3 + green as u32 * 6 + blue as u32 >= 10 * 70 {
            colours[label.cells[cell] as usize - 1].insert([red, green, blue]);
        }
    }
    let mut parent: Vec<usize> = (0..count).collect();
    fn root(parent: &mut [usize], mut at: usize) -> usize {
        while parent[at] != at {
            at = parent[at];
        }
        at
    }
    for cell in (0..w * h).filter(|cell| piece.cells[*cell]) {
        let (x, y) = (cell % w, cell / w);
        for (nx, ny) in [(x + 1, y), (x, y + 1)] {
            if nx >= w || ny >= h || !piece.get(nx, ny) {
                continue;
            }
            let (a, b) = (label.cells[cell], label.get(nx, ny));
            if a == 0 || b == 0 || a == b {
                continue;
            }
            let (a, b) = (root(&mut parent, a as usize - 1), root(&mut parent, b as usize - 1));
            if a == b {
                continue;
            }
            let shared = colours[a].intersection(&colours[b]).count();
            if shared * 2 >= colours[a].len().min(colours[b].len()) {
                parent[a] = b;
                let moved = std::mem::take(&mut colours[a]);
                colours[b].extend(moved);
            }
        }
    }
    // (Numbered anew from 1, those reached by no heart the first.)
    let mut numbers: std::collections::BTreeMap<usize, u32> = std::collections::BTreeMap::new();
    for cell in (0..w * h).filter(|cell| piece.cells[*cell]) {
        let group = match label.cells[cell] {
            0 => usize::MAX,
            at => root(&mut parent, at as usize - 1),
        };
        let next = numbers.len() as u32 + 1;
        let number = *numbers.entry(group).or_insert(next);
        out.set(cell % w + x0, cell / w + y0, number);
    }
    let count = numbers.len();
    (out, count)
}

/// The smallest rectangle round what a mask holds: x0, y0, x1, y1 (the
/// last two past the end); none if it holds nothing.
pub fn bounds(mask: &Mask) -> Option<(usize, usize, usize, usize)> {
    let (mut x0, mut y0, mut x1, mut y1) = (mask.w, mask.h, 0, 0);
    for (cell, held) in mask.cells.iter().enumerate() {
        if *held {
            let (x, y) = (cell % mask.w, cell / mask.w);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
    }
    (x1 > x0).then_some((x0, y0, x1, y1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_are_found_largest_first_and_masks_grown_side_by_side() {
        let mut mask = Mask::new(6, 3);
        for (x, y) in [(0, 0), (1, 0), (4, 1), (4, 2), (5, 2)] {
            mask.set(x, y, true);
        }
        let parts = components(&mask);
        assert_eq!(parts.iter().map(Mask::count).collect::<Vec<_>>(), vec![3, 2]);
        assert!(parts[0].get(4, 1));
        assert_eq!(grown(&mask, 1, 1).count(), 12);
        assert!(!grown(&mask, 1, 1).get(2, 1), "not across corners");
    }

    #[test]
    fn objects_stuck_together_are_apart_and_one_of_its_colours_is_one() {
        // (Two squares joined by a line a pixel thick.)
        let mut own = Mask::new(30, 12);
        for y in 1..11 {
            for x in (1..11).chain(19..29) {
                own.set(x, y, true);
            }
        }
        for x in 11..19 {
            own.set(x, 5, true);
        }
        let apart = |x: usize, _: usize| if x < 15 { [200, 60, 60] } else { [60, 200, 60] };
        assert_eq!(objects(&own, &apart).len(), 2, "two colours: two objects");
        let alike = |_: usize, _: usize| [200, 160, 90];
        assert_eq!(objects(&own, &alike).len(), 1, "the same colours, touching: one fence");
        let mut alone = Mask::new(12, 12);
        for y in 1..11 {
            for x in 1..11 {
                alone.set(x, y, true);
            }
        }
        assert_eq!(objects(&alone, &apart).len(), 1);
    }
}
