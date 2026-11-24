//! Terrains painted, as the game lays its own: a place's tiles learned once
//! (its tileset), each tile's four corners told by their terrain (grass,
//! path, sand, water…), which tile the game puts beside which; a grid of
//! corners painted, the tiles that fit them found again, the game's own
//! neighbours preferred (corner-based autotiling, Wang tiles, of the place's
//! own pieces: no tile drawn, none guessed). The walls come from the tiles.
//!
//! A line of a decor: `terrain <level> x y columns rows <corners>`, its
//! corners `(columns+1) x (rows+1)` letters (`a` the place's most seen
//! terrain, `b` the next…; `.` nothing), rows apart by `/`, a letter followed
//! by how many times it is repeated (`a12b3.4`); then `murs <lettres>` (or
//! `murs aucun`): the terrains one does not walk on, if not those the place
//! itself never lets walk on (its water, its cliffs).

use std::collections::HashMap;
use std::rc::Rc;

use crate::matte::Shade;
use crate::piece::Cut;
use crate::pixels::{Grid, Mask, Picture, TILE};
use crate::random::Rng;
use crate::source::Source;

/// No terrain: nothing laid there.
pub const NONE: u8 = u8::MAX;
/// The most terrains a place is told into (`a` to `p`).
pub const MOST_TERRAINS: usize = 16;
/// How far apart two colours are still one terrain (RGB distance).
const SAME_TERRAIN: f64 = 44.0;
/// A terrain seen on less than this share of the corners: one of its
/// neighbours (a speck, a flower).
const LEAST_SHARE: f64 = 0.004;
/// The most of a place's tiles learned from (its largest places).
const MOST_CELLS: usize = 1 << 17;

/// A tile seen fewer times than this is of a thing (a house, a sign), not
/// of a ground: laid only where nothing of a ground fits.
const GROUND_SEEN: usize = 2;
/// A pixel this far from every terrain's colour is of none (of a thing).
const STRANGE: f64 = 36.0;
/// A terrain the place lets walk on less than this share of it: a wall.
const WALKED_LEAST: f64 = 0.04;

/// One of a place's tiles: each frame of it, which palette colour each
/// pixel shows (its colours that change), the terrain of its corners
/// (top left, top right, bottom left, bottom right), how often the game
/// lays it (a tile turned over: as often as the one it is turned of), how
/// purely its quarters are their terrains (1: wholly; less: a thing on the
/// ground, a rug's border).
pub struct Tile {
    pub frames: Vec<Picture>,
    pub index: Option<Vec<Grid<u8>>>,
    pub corners: [u8; 4],
    pub count: usize,
    pub pure: f64,
}

/// A grid turned over top to bottom.
fn upside_down<T: Copy + Default>(grid: &Grid<T>) -> Grid<T> {
    let mut out = Grid::new(grid.w, grid.h);
    for y in 0..grid.h {
        for x in 0..grid.w {
            out.set(x, grid.h - 1 - y, grid.get(x, y));
        }
    }
    out
}

/// A terrain of a place: its colour, how much of the place it is, the tile
/// that shows it best (all four corners of it), whether one walks on it
/// (the place letting walk on hardly any of it: its water, its cliffs).
#[derive(Clone, Debug)]
pub struct Terrain {
    pub letter: char,
    pub colour: [u8; 3],
    pub share: f64,
    pub swatch: Option<usize>,
    pub wall: bool,
}

/// What is learned of a place to paint with.
pub struct Tileset {
    pub level: i64,
    pub tiles: Vec<Tile>,
    pub terrains: Vec<Terrain>,
    pub frames: usize,
    pub seconds: f64,
    /// How often tile b is seen right of tile a; under it.
    right: Pairs,
    down: Pairs,
    by_corners: HashMap<[u8; 4], Vec<u32>>,
    /// The place's own corners, as it is: for a terrain begun from it.
    pub corners: Grid<u8>,
    nearest: std::cell::RefCell<HashMap<[u8; 4], Rc<Vec<u32>>>>,
}

/// How often each tile is seen beside another.
type Pairs = HashMap<(u32, u32), u32>;

/// The letter of a terrain, and back.
pub fn letter(terrain: u8) -> char {
    if terrain == NONE { '.' } else { (b'a' + terrain) as char }
}

pub fn terrain_of(letter: char) -> Option<u8> {
    match letter {
        '.' => Some(NONE),
        'a'..='p' => Some(letter as u8 - b'a'),
        _ => None,
    }
}

/// The mean colour of a 4x4 quarter of a tile.
fn quarter(picture: &Picture, x: usize, y: usize) -> Option<[f64; 3]> {
    let mut sum = [0.0; 3];
    let mut seen = 0.0;
    for dy in 0..TILE / 2 {
        for dx in 0..TILE / 2 {
            let rgba = picture.get(x + dx, y + dy);
            if rgba[3] > 0 {
                for (part, value) in sum.iter_mut().zip(rgba) {
                    *part += f64::from(value);
                }
                seen += 1.0;
            }
        }
    }
    (seen > 0.0).then(|| sum.map(|part| part / seen))
}

fn apart(a: [f64; 3], b: [f64; 3]) -> f64 {
    // (As the eye has it, roughly: green counts most.)
    let (r, g, b) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    (2.0 * r * r + 4.0 * g * g + 3.0 * b * b).sqrt() / 3.0
}

/// The quarters' colours told into terrains: grouped when near, refined,
/// the specks given to their nearest; the most seen first.
fn told(quarters: &[([f64; 3], usize)]) -> (Vec<[f64; 3]>, Vec<f64>) {
    let mut sorted: Vec<&([f64; 3], usize)> = quarters.iter().collect();
    sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let mut means: Vec<[f64; 3]> = Vec::new();
    for (colour, _) in &sorted {
        if !means.iter().any(|mean| apart(*mean, *colour) < SAME_TERRAIN) && means.len() < 64 {
            means.push(*colour);
        }
    }
    let total: f64 = quarters.iter().map(|(_, count)| *count as f64).sum::<f64>().max(1.0);
    let mut weights = vec![0.0; means.len()];
    for round in 0..8 {
        let mut sums = vec![([0.0; 3], 0.0); means.len()];
        for (colour, count) in quarters {
            let near = nearest_of(&means, *colour);
            let (sum, weight) = &mut sums[near];
            for (part, value) in sum.iter_mut().zip(colour) {
                *part += value * *count as f64;
            }
            *weight += *count as f64;
        }
        weights = sums.iter().map(|(_, weight)| weight / total).collect();
        means = sums.iter().zip(&means).map(|((sum, weight), was)| if *weight > 0.0 { sum.map(|part| part / weight) } else { *was }).collect();
        // (The specks, and all past the most there may be, given to their nearest.)
        if round >= 2 {
            let mut order: Vec<usize> = (0..means.len()).collect();
            order.sort_by(|a, b| weights[*b].total_cmp(&weights[*a]));
            let kept: Vec<usize> =
                order.into_iter().enumerate().filter(|(rank, at)| *rank < MOST_TERRAINS && weights[*at] >= LEAST_SHARE).map(|(_, at)| at).collect();
            means = kept.iter().map(|at| means[*at]).collect();
            weights = kept.iter().map(|at| weights[*at]).collect();
        }
    }
    let mut order: Vec<usize> = (0..means.len()).collect();
    order.sort_by(|a, b| weights[*b].total_cmp(&weights[*a]));
    (order.iter().map(|at| means[*at]).collect(), order.iter().map(|at| weights[*at]).collect())
}

fn nearest_of(means: &[[f64; 3]], colour: [f64; 3]) -> usize {
    (0..means.len()).min_by(|a, b| apart(means[*a], colour).total_cmp(&apart(means[*b], colour))).unwrap_or(0)
}

/// What a place's lower layer teaches: its tiles, their corners, their neighbours.
pub fn learned(source: &Source) -> Tileset {
    let (mut tiles, cells) = cut_tiles(source);
    let (means, shares) = tell_terrains(&mut tiles);
    let turned = turn_over(&mut tiles);
    let mut by_corners: HashMap<[u8; 4], Vec<u32>> = HashMap::new();
    for (id, tile) in tiles.iter().enumerate() {
        by_corners.entry(tile.corners).or_default().push(id as u32);
    }
    let (right, down) = cells.neighbours(&turned);
    let corners = cells.corners(&tiles);
    let walked = cells.walked(source, &tiles);
    let terrains = means
        .iter()
        .zip(&shares)
        .enumerate()
        .map(|(at, (mean, share))| {
            let at = at as u8;
            let swatch = by_corners.get(&[at; 4]).and_then(|ids| ids.iter().max_by_key(|id| tiles[**id as usize].count)).map(|id| *id as usize).or_else(|| {
                (0..tiles.len())
                    .filter(|id| tiles[*id].corners.contains(&at))
                    .max_by_key(|id| (tiles[*id].corners.iter().filter(|corner| **corner == at).count(), tiles[*id].count))
            });
            let (on, of) = walked[at as usize];
            Terrain {
                letter: letter(at),
                colour: mean.map(|part| part.round() as u8),
                share: *share,
                swatch,
                wall: of > 0 && (on as f64) < WALKED_LEAST * of as f64,
            }
        })
        .collect();
    Tileset {
        level: source.level,
        tiles,
        terrains,
        frames: source.frames.len(),
        seconds: f64::from(source.info.seconds),
        right,
        down,
        by_corners,
        corners,
        nearest: Default::default(),
    }
}

/// The place's cells, each the tile it shows (`u32::MAX`: none).
struct Cells {
    columns: usize,
    rows: usize,
    at_cell: Grid<u32>,
}

/// The place's tiles, each one once with how often it is seen, and which
/// is in each cell (one in `step` of them on a very large place).
fn cut_tiles(source: &Source) -> (Vec<Tile>, Cells) {
    let first = &source.frames[0];
    let (columns, rows) = (first.w / TILE, first.h / TILE);
    let step = ((columns * rows) / MOST_CELLS).max(1);
    let mut tiles: Vec<Tile> = Vec::new();
    let mut known: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut at_cell: Grid<u32> = Grid::filled(columns, rows, u32::MAX);
    for row in 0..rows {
        for column in 0..columns {
            if (row * columns + column) % step != 0 && step > 1 {
                continue;
            }
            let (x, y) = (column * TILE, row * TILE);
            let frames: Vec<Picture> = source.frames.iter().map(|frame| frame.crop(x, y, x + TILE, y + TILE)).collect();
            if frames[0].cells.iter().all(|rgba| rgba[3] == 0) {
                continue;
            }
            let index: Option<Vec<Grid<u8>>> = source.indices.as_ref().map(|all| all.iter().map(|grid| grid.crop(x, y, x + TILE, y + TILE)).collect());
            let mut key: Vec<u8> = frames.iter().flat_map(|frame| frame.cells.iter().flatten().copied()).collect();
            if let Some(index) = &index {
                key.extend(index.iter().flat_map(|grid| grid.cells.iter().copied()));
            }
            let id = *known.entry(key).or_insert_with(|| {
                tiles.push(Tile { frames, index, corners: [NONE; 4], count: 0, pure: 1.0 });
                (tiles.len() - 1) as u32
            });
            tiles[id as usize].count += 1;
            at_cell.set(column, row, id);
        }
    }
    (tiles, Cells { columns, rows, at_cell })
}

/// Each tile's quarters told into terrains: the terrains' colours and
/// shares, each tile's corners and how purely it shows them.
fn tell_terrains(tiles: &mut [Tile]) -> (Vec<[f64; 3]>, Vec<f64>) {
    let quarters_of = |tile: &Tile| -> [Option<[f64; 3]>; 4] { [(0, 0), (4, 0), (0, 4), (4, 4)].map(|(x, y)| quarter(&tile.frames[0], x, y)) };
    let mut quarters: Vec<([f64; 3], usize)> = Vec::new();
    for tile in tiles.iter() {
        for colour in quarters_of(tile).into_iter().flatten() {
            quarters.push((colour, tile.count));
        }
    }
    let (means, shares) = told(&quarters);
    for tile in tiles.iter_mut() {
        let quarters = quarters_of(tile);
        tile.corners = quarters.map(|colour| colour.filter(|_| !means.is_empty()).map_or(NONE, |colour| nearest_of(&means, colour) as u8));
        // (How far its quarters are from their terrains' colours, at most;
        // how many of its pixels are of no terrain (a rug's white border, a
        // sign); a hole in it (under a thing of the upper layer): no ground.)
        let off = quarters.iter().zip(&tile.corners).filter_map(|(colour, corner)| Some(apart((*colour)?, *means.get(*corner as usize)?))).fold(0.0, f64::max);
        let strange = tile.frames[0]
            .cells
            .iter()
            .filter(|rgba| rgba[3] > 0)
            .filter(|rgba| {
                let colour = [f64::from(rgba[0]), f64::from(rgba[1]), f64::from(rgba[2])];
                means.iter().all(|mean| apart(*mean, colour) > STRANGE)
            })
            .count();
        let holed = tile.frames[0].cells.iter().any(|rgba| rgba[3] == 0);
        tile.pure = (-(strange as f64) / 3.0).exp() / (1.0 + off / 16.0).powi(2) * if holed { 0.01 } else { 1.0 };
    }
    (means, shares)
}

/// Each tile turned over too, as the game turns its own: left to right,
/// top to bottom, both; its corners turned with it. For each way, the
/// tile each of the place's own becomes.
fn turn_over(tiles: &mut Vec<Tile>) -> [Vec<u32>; 3] {
    let originals = tiles.len();
    let mut known_turned: HashMap<Vec<u8>, u32> = HashMap::new();
    for (id, tile) in tiles.iter().enumerate() {
        known_turned.insert(tile.frames.iter().flat_map(|frame| frame.cells.iter().flatten().copied()).collect(), id as u32);
    }
    let mut turned: [Vec<u32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for id in 0..originals {
        for (way, made) in turned.iter_mut().enumerate() {
            let (left_right, top_bottom) = (way != 1, way != 0);
            let turn = |grid: &Picture| {
                let grid = if left_right { grid.mirrored() } else { grid.clone() };
                if top_bottom { upside_down(&grid) } else { grid }
            };
            let turn_index = |grid: &Grid<u8>| {
                let grid = if left_right { grid.mirrored() } else { grid.clone() };
                if top_bottom { upside_down(&grid) } else { grid }
            };
            let tile = &tiles[id];
            let frames: Vec<Picture> = tile.frames.iter().map(turn).collect();
            let key: Vec<u8> = frames.iter().flat_map(|frame| frame.cells.iter().flatten().copied()).collect();
            if let Some(known) = known_turned.get(&key) {
                made.push(*known);
                continue;
            }
            let [tl, tr, bl, br] = tile.corners;
            let corners = match (left_right, top_bottom) {
                (true, false) => [tr, tl, br, bl],
                (false, true) => [bl, br, tl, tr],
                _ => [br, bl, tr, tl],
            };
            let index = tile.index.as_ref().map(|all| all.iter().map(turn_index).collect());
            let (count, pure) = (tile.count, tile.pure);
            tiles.push(Tile { frames, index, corners, count, pure });
            known_turned.insert(key, (tiles.len() - 1) as u32);
            made.push((tiles.len() - 1) as u32);
        }
    }
    turned
}

impl Cells {
    /// Who is beside whom (right of, under), the turned tiles too.
    fn neighbours(&self, turned: &[Vec<u32>; 3]) -> (Pairs, Pairs) {
        let (columns, rows, at_cell) = (self.columns, self.rows, &self.at_cell);
        let (mut right, mut down): (Pairs, Pairs) = (HashMap::new(), HashMap::new());
        for row in 0..rows {
            for column in 0..columns {
                let here = at_cell.get(column, row);
                if here == u32::MAX {
                    continue;
                }
                if column + 1 < columns && at_cell.get(column + 1, row) != u32::MAX {
                    *right.entry((here, at_cell.get(column + 1, row))).or_default() += 1;
                }
                if row + 1 < rows && at_cell.get(column, row + 1) != u32::MAX {
                    *down.entry((here, at_cell.get(column, row + 1))).or_default() += 1;
                }
            }
        }
        // (Who is beside whom, turned over: left to right, the right one goes left.)
        let seen_right: Vec<((u32, u32), u32)> = right.iter().map(|(pair, count)| (*pair, *count)).collect();
        let seen_down: Vec<((u32, u32), u32)> = down.iter().map(|(pair, count)| (*pair, *count)).collect();
        let [h, v, both] = [0, 1, 2].map(|way| turned[way].as_slice());
        for ((a, b), count) in seen_right {
            *right.entry((h[b as usize], h[a as usize])).or_default() += count;
            *right.entry((v[a as usize], v[b as usize])).or_default() += count;
            *right.entry((both[b as usize], both[a as usize])).or_default() += count;
        }
        for ((a, b), count) in seen_down {
            *down.entry((h[a as usize], h[b as usize])).or_default() += count;
            *down.entry((v[b as usize], v[a as usize])).or_default() += count;
            *down.entry((both[b as usize], both[a as usize])).or_default() += count;
        }
        (right, down)
    }

    /// The place's own corners: each the terrain most of the four quarters around it say.
    fn corners(&self, tiles: &[Tile]) -> Grid<u8> {
        let (columns, rows) = (self.columns, self.rows);
        let mut corners = Grid::filled(columns + 1, rows + 1, NONE);
        for vy in 0..=rows {
            for vx in 0..=columns {
                let mut said = [0usize; MOST_TERRAINS];
                for (dx, dy, corner) in [(-1i64, -1i64, 3usize), (0, -1, 2), (-1, 0, 1), (0, 0, 0)] {
                    let (column, row) = (vx as i64 + dx, vy as i64 + dy);
                    if column < 0 || row < 0 || column >= columns as i64 || row >= rows as i64 {
                        continue;
                    }
                    let id = self.at_cell.get(column as usize, row as usize);
                    if let Some(tile) = tiles.get(id as usize)
                        && tile.corners[corner] != NONE
                    {
                        said[tile.corners[corner] as usize] += 1;
                    }
                }
                let (best, count) = said.iter().enumerate().max_by_key(|(at, count)| (**count, std::cmp::Reverse(*at))).unwrap_or((0, &0));
                if *count > 0 {
                    corners.set(vx, vy, best as u8);
                }
            }
        }
        corners
    }

    /// How much of each terrain the place lets walk on: (walkable, all).
    fn walked(&self, source: &Source, tiles: &[Tile]) -> [(usize, usize); MOST_TERRAINS] {
        let mut walked = [(0usize, 0usize); MOST_TERRAINS];
        if let Some(walls) = &source.walls {
            for row in 0..self.rows.min(walls.h) {
                for column in 0..self.columns.min(walls.w) {
                    let Some(tile) = tiles.get(self.at_cell.get(column, row) as usize) else {
                        continue;
                    };
                    for corner in tile.corners.iter().filter(|corner| **corner != NONE) {
                        let seen = &mut walked[*corner as usize];
                        seen.0 += usize::from(!walls.get(column, row));
                        seen.1 += 1;
                    }
                }
            }
        }
        walked
    }
}

impl Tileset {
    /// The tiles of those corners, or of the nearest corners there are (the
    /// most of them the same, then the most seen).
    fn fitting(&self, key: [u8; 4]) -> Rc<Vec<u32>> {
        if let Some(known) = self.nearest.borrow().get(&key) {
            return known.clone();
        }
        // (Those of these very corners, all: the drawing weighs them.)
        if let Some(ids) = self.by_corners.get(&key) {
            let made = Rc::new(ids.clone());
            self.nearest.borrow_mut().insert(key, made.clone());
            return made;
        }
        // (Else of the nearest corners, of a ground first: seen more than once.)
        let ground = |ids: &Vec<u32>| -> Vec<u32> { ids.iter().filter(|id| self.tiles[**id as usize].count >= GROUND_SEEN).copied().collect() };
        let alike = |other: &[u8; 4]| other.iter().zip(&key).filter(|(a, b)| a == b).count();
        let weight = |ids: &Vec<u32>| ids.iter().map(|id| self.tiles[*id as usize].count).sum::<usize>();
        let best = self
            .by_corners
            .iter()
            .map(|(other, ids)| (other, ground(ids)))
            .filter(|(_, ids)| !ids.is_empty())
            .max_by_key(|(other, ids)| (alike(other), weight(ids), **other))
            .map(|(_, ids)| ids)
            .or_else(|| self.by_corners.get(&key).cloned())
            .unwrap_or_default();
        let made = Rc::new(best);
        self.nearest.borrow_mut().insert(key, made.clone());
        made
    }

    /// The tiles of a grid of corners, cell after cell: those that fit its
    /// corners, of them those the game lays beside what is left and above,
    /// one drawn as often as the game lays it (the same each time for the
    /// same cell). `u32::MAX`: nothing there.
    pub fn solved(&self, corners: &Grid<u8>, seed: (i64, i64)) -> Grid<u32> {
        let (columns, rows) = (corners.w.saturating_sub(1), corners.h.saturating_sub(1));
        let mut out = Grid::filled(columns, rows, u32::MAX);
        for row in 0..rows {
            for column in 0..columns {
                let key = [corners.get(column, row), corners.get(column + 1, row), corners.get(column, row + 1), corners.get(column + 1, row + 1)];
                if key.contains(&NONE) {
                    continue;
                }
                let fitting = self.fitting(key);
                if fitting.is_empty() {
                    continue;
                }
                let left = if column > 0 { out.get(column - 1, row) } else { u32::MAX };
                let up = if row > 0 { out.get(column, row - 1) } else { u32::MAX };
                // (Each weighed by how often the game lays it, how purely it is
                // ground, how often after what is left and under what is above.)
                let after_left = |id: u32| if left == u32::MAX { 0.0 } else { self.right.get(&(left, id)).map_or(0.0, |count| f64::from(*count)) };
                let under_up = |id: u32| if up == u32::MAX { 0.0 } else { self.down.get(&(up, id)).map_or(0.0, |count| f64::from(*count)) };
                let among: Vec<(u32, f64)> = fitting
                    .iter()
                    .map(|id| {
                        let tile = &self.tiles[*id as usize];
                        (*id, tile.count as f64 * tile.pure * tile.pure * (0.5 + after_left(*id)) * (0.5 + under_up(*id)))
                    })
                    .collect();
                let mut rng = Rng::new(&[self.level as u64, (seed.0 + column as i64) as u64, (seed.1 + row as i64) as u64]);
                let total: f64 = among.iter().map(|(_, weight)| weight).sum();
                let mut drawn = rng.random() * total;
                let mut chosen = among[0].0;
                for (id, weight) in &among {
                    if drawn < *weight {
                        chosen = *id;
                        break;
                    }
                    drawn -= weight;
                }
                out.set(column, row, chosen);
            }
        }
        out
    }

    /// Each set of corners there are, and its tiles.
    pub fn keys(&self) -> impl Iterator<Item = (&[u8; 4], &Vec<u32>)> {
        self.by_corners.iter()
    }

    /// The picture of one tile (frame 0), RGBA.
    pub fn tile_picture(&self, id: usize) -> &Picture {
        &self.tiles[id].frames[0]
    }
}

/// A terrain as a line of a decor says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Painted {
    pub level: i64,
    pub x: i64,
    pub y: i64,
    /// Its corners: (columns+1) x (rows+1).
    pub corners: Grid<u8>,
    /// The terrains one does not walk on, if said (`walls …`).
    pub walls: Option<Vec<u8>>,
}

impl Painted {
    pub fn columns(&self) -> usize {
        self.corners.w.saturating_sub(1)
    }

    pub fn rows(&self) -> usize {
        self.corners.h.saturating_sub(1)
    }

    /// Of a line's words, after `terrain`.
    pub fn read(words: &[String]) -> Result<Painted, String> {
        let usage = "terrain <level> x y columns rows <corners> (letters a…p, . for nothing, the rows separated by /, a12: twelve a)";
        let (words, walls) = match words.iter().position(|word| word == "walls") {
            Some(at) => {
                let said = words[at + 1..].join("");
                let walls = if said == "none" {
                    Vec::new()
                } else {
                    said.chars()
                        .map(|one| terrain_of(one).filter(|terrain| *terrain != NONE).ok_or_else(|| format!("walls \"{one}\": letters from a to p, or none")))
                        .collect::<Result<Vec<u8>, String>>()?
                };
                (&words[..at], Some(walls))
            }
            None => (words, None),
        };
        let [level, x, y, columns, rows, said] = words else {
            return Err(usage.to_string());
        };
        let number = |word: &String| word.parse::<i64>().map_err(|_| format!("\"{word}\" is not a number: {usage}"));
        let (level, x, y, columns, rows) = (number(level)?, number(x)?, number(y)?, number(columns)?, number(rows)?);
        if !(1..=1024).contains(&columns) || !(1..=1024).contains(&rows) {
            return Err("from 1 to 1024 columns and rows".to_string());
        }
        let (w, h) = (columns as usize + 1, rows as usize + 1);
        let lines: Vec<&str> = said.split('/').collect();
        if lines.len() != h {
            return Err(format!("{h} rows of corners expected (rows + 1), {} written", lines.len()));
        }
        let mut corners = Grid::filled(w, h, NONE);
        for (row, line) in lines.iter().enumerate() {
            let mut cells: Vec<u8> = Vec::with_capacity(w);
            let mut chars = line.chars().peekable();
            while let Some(first) = chars.next() {
                let terrain = terrain_of(first).ok_or_else(|| format!("\"{first}\": a corner is a letter from a to p, or ."))?;
                let mut digits = String::new();
                while let Some(digit) = chars.peek().filter(|digit| digit.is_ascii_digit()) {
                    digits.push(*digit);
                    chars.next();
                }
                let times = if digits.is_empty() { 1 } else { digits.parse::<usize>().map_err(|_| usage.to_string())? };
                cells.extend(std::iter::repeat_n(terrain, times));
            }
            if cells.len() != w {
                return Err(format!("corner row {} has {} corners, {w} are needed (columns + 1)", row + 1, cells.len()));
            }
            corners.cells[row * w..(row + 1) * w].copy_from_slice(&cells);
        }
        Ok(Painted { level, x, y, corners, walls })
    }

    /// Its corners as written: each run a letter and how many.
    pub fn written_corners(&self) -> String {
        let mut rows = Vec::new();
        for row in self.corners.cells.chunks(self.corners.w.max(1)) {
            let mut out = String::new();
            let mut at = 0;
            while at < row.len() {
                let run = row[at..].iter().take_while(|cell| **cell == row[at]).count();
                out.push(letter(row[at]));
                if run > 1 {
                    out.push_str(&run.to_string());
                }
                at += run;
            }
            rows.push(out);
        }
        rows.join("/")
    }

    /// The whole line.
    pub fn line(&self) -> String {
        let walls = match &self.walls {
            None => String::new(),
            Some(walls) if walls.is_empty() => " walls none".to_string(),
            Some(walls) => format!(" walls {}", walls.iter().map(|terrain| letter(*terrain)).collect::<String>()),
        };
        format!("terrain {} {} {} {} {} {}{walls}", self.level, self.x, self.y, self.columns(), self.rows(), self.written_corners())
    }

    /// Nothing painted in it.
    pub fn is_empty(&self) -> bool {
        self.corners.cells.iter().all(|cell| *cell == NONE)
    }

    /// Grown (or cut) to hold those corners of the decor (columns, rows of
    /// 8 from the decor's corner), nothing painted on what it gains.
    pub fn holding(&self, (c0, r0): (i64, i64), (c1, r1): (i64, i64)) -> Painted {
        let (x0, y0) = (self.x.div_euclid(TILE as i64), self.y.div_euclid(TILE as i64));
        let (x1, y1) = (x0 + self.corners.w as i64 - 1, y0 + self.corners.h as i64 - 1);
        let (nx0, ny0, nx1, ny1) = (x0.min(c0), y0.min(r0), x1.max(c1), y1.max(r1));
        let (w, h) = ((nx1 - nx0 + 1) as usize, (ny1 - ny0 + 1) as usize);
        let mut corners = Grid::filled(w, h, NONE);
        for vy in 0..self.corners.h {
            for vx in 0..self.corners.w {
                corners.set((x0 - nx0) as usize + vx, (y0 - ny0) as usize + vy, self.corners.get(vx, vy));
            }
        }
        Painted { level: self.level, x: nx0 * TILE as i64, y: ny0 * TILE as i64, corners, walls: self.walls.clone() }
    }

    /// Cut to what is painted (one corner of nothing all around kept out).
    pub fn trimmed(&self) -> Painted {
        let (w, h) = (self.corners.w, self.corners.h);
        let painted = |vx: usize, vy: usize| self.corners.get(vx, vy) != NONE;
        let columns: Vec<usize> = (0..w).filter(|vx| (0..h).any(|vy| painted(*vx, vy))).collect();
        let rows: Vec<usize> = (0..h).filter(|vy| (0..w).any(|vx| painted(vx, *vy))).collect();
        let (Some(&cx0), Some(&cx1), Some(&ry0), Some(&ry1)) = (columns.first(), columns.last(), rows.first(), rows.last()) else {
            return Painted { level: self.level, x: self.x, y: self.y, corners: Grid::filled(2, 2, NONE), walls: self.walls.clone() };
        };
        let (cx1, ry1) = (cx1.max(cx0 + 1), ry1.max(ry0 + 1));
        let mut corners = Grid::filled(cx1 - cx0 + 1, ry1 - ry0 + 1, NONE);
        for vy in ry0..=ry1.min(h - 1) {
            for vx in cx0..=cx1.min(w - 1) {
                corners.set(vx - cx0, vy - ry0, self.corners.get(vx, vy));
            }
        }
        Painted { level: self.level, x: self.x + (cx0 * TILE) as i64, y: self.y + (ry0 * TILE) as i64, corners, walls: self.walls.clone() }
    }
}

/// A terrain as a piece to lay, of a tileset at hand (`Workshop::tileset`):
/// its tiles solved, every frame of them, their colours that change, their
/// walls.
pub fn cut_of(set: &Tileset, painted: &Painted) -> Cut {
    let solved = set.solved(&painted.corners, (painted.x.div_euclid(TILE as i64), painted.y.div_euclid(TILE as i64)));
    let (w, h) = ((solved.w * TILE).max(1), (solved.h * TILE).max(1));
    let frames = set.frames.max(1);
    let mut lower: Vec<Picture> = (0..frames).map(|_| Picture::new(w, h)).collect();
    let indexed = set.tiles.first().is_some_and(|tile| tile.index.is_some());
    let index_count = set.tiles.first().and_then(|tile| tile.index.as_ref()).map_or(0, Vec::len);
    let mut lower_index: Option<Vec<Grid<u8>>> = indexed.then(|| (0..index_count).map(|_| Grid::new(w, h)).collect());
    let mut solid = Mask::new(w, h);
    let walls: Vec<u8> =
        painted.walls.clone().unwrap_or_else(|| set.terrains.iter().enumerate().filter(|(_, terrain)| terrain.wall).map(|(at, _)| at as u8).collect());
    for row in 0..solved.h {
        for column in 0..solved.w {
            let id = solved.get(column, row);
            let Some(tile) = set.tiles.get(id as usize) else {
                continue;
            };
            for (frame, picture) in lower.iter_mut().enumerate() {
                let part = &tile.frames[frame % tile.frames.len()];
                for dy in 0..TILE {
                    for dx in 0..TILE {
                        picture.set(column * TILE + dx, row * TILE + dy, part.get(dx, dy));
                    }
                }
            }
            if let (Some(all), Some(own)) = (lower_index.as_mut(), tile.index.as_ref()) {
                for (grid, part) in all.iter_mut().zip(own) {
                    for dy in 0..TILE {
                        for dx in 0..TILE {
                            grid.set(column * TILE + dx, row * TILE + dy, part.get(dx, dy));
                        }
                    }
                }
            }
            // (Its walls: the quarters of a terrain one does not walk on.)
            for (quarter, corner) in tile.corners.iter().enumerate() {
                if *corner == NONE || !walls.contains(corner) {
                    continue;
                }
                let (qx, qy) = ((quarter % 2) * TILE / 2, (quarter / 2) * TILE / 2);
                for dy in 0..TILE / 2 {
                    for dx in 0..TILE / 2 {
                        solid.set(column * TILE + qx + dx, row * TILE + qy + dy, tile.frames[0].get(qx + dx, qy + dy)[3] > 0);
                    }
                }
            }
        }
    }
    Cut {
        upper: vec![Picture::new(w, h)],
        lower,
        solid,
        level: set.level,
        lower_index,
        upper_index: None,
        shade: Shade::none(w, h),
        foot: (0, 0),
        walked: false,
        seconds: set.seconds,
    }
}

/// A part of a place of the game as a terrain to paint on: its own corners
/// over those columns and rows of 8 (`terrain depuis`).
pub fn of_the_place(set: &Tileset, x: i64, y: i64, (c0, r0, columns, rows): (usize, usize, usize, usize)) -> Painted {
    let mut corners = Grid::filled(columns + 1, rows + 1, NONE);
    for vy in 0..=rows {
        for vx in 0..=columns {
            if c0 + vx < set.corners.w && r0 + vy < set.corners.h {
                corners.set(vx, vy, set.corners.get(c0 + vx, r0 + vy));
            }
        }
    }
    Painted { level: set.level, x, y, corners, walls: None }
}
