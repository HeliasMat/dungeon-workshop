//! Weaving grounds, paths and bands from the game's own tiles.
//!
//! A ground (`ground`) and a band (`edge`) are woven tile by tile, each tile the
//! one of the source whose edges best meet what is round it, going on from
//! its neighbour as in the game where it can. A path (`path`) is woven by
//! its shape: each tile of the source knows what of it is inside the path and
//! what is its fringe; each tile of the path wanted takes the source tile of
//! the most alike shape, then of the best edges. The looms are learned from
//! the game's places once, by the `Workshop` (`workshop`).

use std::collections::HashMap;
use std::rc::Rc;

use crate::catalogue::Piece;
use crate::fault::Diagnostic;
use crate::matte::{Shade, Tile};
use crate::pixels::{Grid, Mask, Rgba, TILE, colour, counted, key};
use crate::random::Rng;
use crate::source::Source;

/// How much a seam costs against going on as in the source.
pub const GOING_ON: f64 = 40.0;
/// What a pixel of a path's shape costs, wrong inside or wrong in its fringe.
const SHAPE_INSIDE: f64 = 40.0;
const SHAPE_FRINGE: f64 = 4.0;
/// How wide the fringe of a path is, in pixels.
pub const FRINGE: usize = 3;
/// The pixels of a tile.
pub(crate) const CELLS: usize = TILE * TILE;

/// The small things of a place's grounds: the colours each shows, where.
pub type Crumbs = Rc<Vec<(Grid<[u8; 3]>, Mask)>>;

/// A tile of a loom: its frames (alpha 0 where its ground is cut out), the
/// palette colours of each frame, its shade, where it is in its source (x, y,
/// turn), its row in a band, and for a path what of it is inside and fringe.
#[derive(Clone, Debug)]
pub struct LoomTile {
    pub frames: Vec<Vec<Rgba>>,
    pub indices: Option<Vec<Vec<u8>>>,
    pub place: (i64, i64, String),
    pub mark: i64,
    pub shade: Vec<f32>,
    pub shade_colour: Vec<[u8; 3]>,
    pub inside: Vec<bool>,
    pub fringe: Vec<bool>,
}

impl LoomTile {
    /// Its colours, frame 0, row by row.
    pub fn rgb(&self) -> Tile {
        self.frames[0].iter().flat_map(|rgba| [rgba[0], rgba[1], rgba[2]]).collect()
    }
}

/// The tiles a ground, a path or a band is woven from.
pub struct Loom {
    pub piece: Piece,
    pub tiles: Vec<LoomTile>,
    pub by_place: HashMap<(i64, i64, String), usize>,
    pub seconds: f64,
    pub grounds: Vec<i64>,
}

impl Loom {
    /// Whether all its tiles are whole and shade nothing.
    pub fn opaque(&self) -> bool {
        self.tiles.iter().all(|tile| tile.frames[0].iter().all(|rgba| rgba[3] > 0))
            && !self.tiles.iter().any(|tile| tile.shade.iter().any(|factor| *factor != 0.0))
    }

    pub fn frames(&self) -> usize {
        self.tiles.first().map_or(1, |tile| tile.frames.len())
    }

    pub fn tile_shade(&self, at: usize) -> Shade {
        let tile = &self.tiles[at];
        Shade {
            factor: Grid { w: TILE, h: TILE, cells: tile.shade.clone() },
            colour: Grid { w: TILE, h: TILE, cells: tile.shade_colour.clone() },
            grounds: self.grounds.clone(),
        }
    }
}

/// The colours a piece's option says (`rrggbb,...`).
pub fn colours_said(text: &str) -> Vec<i64> {
    text.split(',').filter(|word| !word.is_empty()).filter_map(colour).map(key).collect()
}

/// The colours of a rectangle of a place's picture, as keys.
pub(crate) fn rect_keys(source: &Source, (x0, y0, x1, y1): (i64, i64, i64, i64)) -> Vec<i64> {
    let mut out = Vec::new();
    for y in y0.max(0)..y1.min(source.picture().h as i64) {
        for x in x0.max(0)..x1.min(source.picture().w as i64) {
            out.push(key(source.rgb(x as usize, y as usize)));
        }
    }
    out
}

/// Those of a rectangle's colours seen in at least one part out of `of`.
pub(crate) fn common(every: Vec<i64>, of: usize) -> Vec<i64> {
    let total = every.len();
    counted(every).into_iter().filter(|(_, count)| count * of >= total).map(|(colour, _)| colour).collect()
}

/// The colours of the shadows the game draws on a ground (`shadow=`).
pub(crate) fn shadow_colours(piece: &Piece) -> Vec<i64> {
    colours_said(piece.option("shadow").unwrap_or_default())
}

/// The grounds a piece stands on: their tiles, colours (none: it names
/// none) and shadow colours.
pub type Grounds = (Vec<Tile>, Option<Vec<i64>>, Vec<i64>);

/// A path's shape, tile by tile: what of each is inside it, what its fringe.
pub type Shape = (Grid<Vec<bool>>, Grid<Vec<bool>>);

/// A tile's cells (8 by 8) turned as `turn` says: across (t), left to right
/// (h), upside down (v), in that order.
pub fn turned<T: Copy>(cells: &[T], turn: &str) -> Vec<T> {
    let mut out = cells.to_vec();
    let at = |x: usize, y: usize| y * TILE + x;
    if turn.contains('t') {
        out = (0..CELLS).map(|cell| out[at(cell / TILE, cell % TILE)]).collect();
    }
    if turn.contains('h') {
        out = (0..CELLS).map(|cell| out[at(TILE - 1 - cell % TILE, cell / TILE)]).collect();
    }
    if turn.contains('v') {
        out = (0..CELLS).map(|cell| out[at(cell % TILE, TILE - 1 - cell / TILE)]).collect();
    }
    out
}

/// A line through points, its turns rounded (cut at a quarter, again and again).
pub fn rounded(points: &[(i64, i64)]) -> Vec<(f64, f64)> {
    let mut points: Vec<(f64, f64)> = points.iter().map(|(x, y)| (*x as f64, *y as f64)).collect();
    for _ in 0..3 {
        if points.len() < 3 {
            break;
        }
        let mut out = vec![points[0]];
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            out.push((a.0 * 0.75 + b.0 * 0.25, a.1 * 0.75 + b.1 * 0.25));
            out.push((a.0 * 0.25 + b.0 * 0.75, a.1 * 0.25 + b.1 * 0.75));
        }
        out.push(points[points.len() - 1]);
        // (The ends stay where they are said: the first and last cut go.)
        let mut kept = vec![out[0]];
        kept.extend_from_slice(&out[2..out.len() - 2]);
        kept.push(out[out.len() - 1]);
        points = kept;
    }
    points
}

/// Which pixels of a place are inside a path along these points.
pub fn path_shape(points: &[(f64, f64)], width: i64, w: usize, h: usize) -> Mask {
    let mut inside = Mask::new(w, h);
    let reach = (width as f64 / 2.0) * (width as f64 / 2.0);
    for pair in points.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let length = (dx * dx + dy * dy).max(1e-9);
        for y in 0..h {
            for x in 0..w {
                let (px, py) = (x as f64, y as f64);
                let t = (((px - ax) * dx + (py - ay) * dy) / length).clamp(0.0, 1.0);
                let (ex, ey) = (px - ax - t * dx, py - ay - t * dy);
                if ex * ex + ey * ey <= reach {
                    inside.set(x, y, true);
                }
            }
        }
    }
    inside
}

/// A mask of a place as its tiles: each one's 64 cells, row by row.
pub fn as_tiles(mask: &Mask) -> Grid<Vec<bool>> {
    let (columns, rows) = (mask.w / TILE, mask.h / TILE);
    let mut out: Grid<Vec<bool>> = Grid { w: columns, h: rows, cells: vec![Vec::new(); columns * rows] };
    for row in 0..rows {
        for column in 0..columns {
            out.cells[row * columns + column] = (0..CELLS).map(|cell| mask.get(column * TILE + cell % TILE, row * TILE + cell / TILE)).collect();
        }
    }
    out
}

/// The colours of a picture, as numbers (frame 0 of a place, as woven over).
type Canvas = Grid<[i32; 3]>;

/// What a tile looks like over a ground, roughly: where it is cut out, the
/// ground, shaded as it says.
fn laid_over(tile: &LoomTile, under: &[[i32; 3]]) -> Vec<[i32; 3]> {
    (0..CELLS)
        .map(|at| {
            let rgba = tile.frames[0][at];
            if rgba[3] > 0 {
                return [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32];
            }
            let factor = tile.shade[at];
            let factor = if factor > 0.0 { factor as f64 } else { 1.0 };
            under[at].map(|part| (part as f64 * factor) as i32)
        })
        .collect()
}

fn block(picture: &Canvas, x: usize, y: usize) -> Vec<[i32; 3]> {
    (0..CELLS).map(|at| picture.get(x + at % TILE, y + at / TILE)).collect()
}

fn difference(a: [i32; 3], b: [i32; 3]) -> i64 {
    (0..3).map(|part| (a[part] - b[part]).abs() as i64).sum()
}

/// The choice of each tile of a weaving (-1: none), and what each cost.
pub type Woven = (Grid<i64>, Grid<f64>);

/// Chooses a tile of the loom for each tile of `wanted` (-1 none, else the
/// mark it must have): the one whose edges best meet what is round it
/// already (`canvas`, frame 0 of the place) and going on from its neighbours
/// as in the game; for a path, `shape` (inside, fringe) says each tile's shape.
pub fn weave(loom: &Loom, wanted: &Grid<i64>, canvas: &Grid<Rgba>, rng: &mut Rng, shape: Option<&Shape>) -> Woven {
    let (rows, columns) = (wanted.h, wanted.w);
    let mut chosen: Grid<i64> = Grid::filled(columns, rows, -1);
    let mut costs: Grid<f64> = Grid::new(columns, rows);
    let tiles: Vec<Vec<[i32; 3]>> =
        loom.tiles.iter().map(|tile| tile.frames[0].iter().map(|rgba| [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]).collect()).collect();
    let clear = loom.opaque();
    let mut by_mark: Vec<(i64, Vec<usize>)> = Vec::new();
    for (at, tile) in loom.tiles.iter().enumerate() {
        match by_mark.iter_mut().find(|(mark, _)| *mark == tile.mark) {
            Some((_, found)) => found.push(at),
            None => by_mark.push((tile.mark, vec![at])),
        }
    }
    let everything: Vec<usize> = (0..tiles.len()).collect();
    let mut picture: Canvas = canvas.map(|rgba| [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]);
    for row in 0..rows {
        for column in 0..columns {
            let mark = wanted.get(column, row);
            if mark < 0 {
                continue;
            }
            let candidates: &Vec<usize> = match shape {
                Some(_) => &everything,
                None => match by_mark.iter().find(|(known, _)| *known == mark) {
                    Some((_, found)) => found,
                    // (No tile of that kind in the source: the nearest kind.)
                    None => &by_mark.iter().min_by_key(|(other, _)| ((other ^ mark).count_ones(), *other)).expect("a kind").1,
                },
            };
            let (y, x) = (row * TILE, column * TILE);
            let under = block(&picture, x, y);
            let seen: Vec<Vec<[i32; 3]>> = candidates.iter().map(|at| if clear { tiles[*at].clone() } else { laid_over(&loom.tiles[*at], &under) }).collect();
            let mut sums = vec![0i64; candidates.len()];
            let mut sides = 0;
            // (Left, above: laid by now; right, below: only what is there and not to be woven.)
            let edge = |sums: &mut Vec<i64>, cells: &dyn Fn(usize) -> usize, there: &dyn Fn(usize) -> [i32; 3]| {
                for (slot, one) in sums.iter_mut().zip(&seen) {
                    *slot += (0..TILE).map(|at| difference(one[cells(at)], there(at))).sum::<i64>();
                }
            };
            if column > 0 && (chosen.get(column - 1, row) >= 0 || wanted.get(column - 1, row) < 0) {
                edge(&mut sums, &|at| at * TILE, &|at| picture.get(x - 1, y + at));
                sides += 1;
            }
            if row > 0 && (chosen.get(column, row - 1) >= 0 || wanted.get(column, row - 1) < 0) {
                edge(&mut sums, &|at| at, &|at| picture.get(x + at, y - 1));
                sides += 1;
            }
            if column + 1 < columns && wanted.get(column + 1, row) < 0 {
                edge(&mut sums, &|at| at * TILE + TILE - 1, &|at| picture.get(x + TILE, y + at));
                sides += 1;
            }
            if row + 1 < rows && wanted.get(column, row + 1) < 0 {
                edge(&mut sums, &|at| (TILE - 1) * TILE + at, &|at| picture.get(x + at, y + TILE));
                sides += 1;
            }
            let mut cost: Vec<f64> = sums.iter().map(|sum| *sum as f64 / sides.max(1) as f64 / TILE as f64).collect();
            let raw = cost.clone();
            if let Some((inside, fringe)) = shape {
                let (inside, fringe) = (&inside.cells[row * columns + column], &fringe.cells[row * columns + column]);
                for (slot, at) in cost.iter_mut().zip(candidates) {
                    let tile = &loom.tiles[*at];
                    *slot += SHAPE_INSIDE * (0..CELLS).filter(|cell| tile.inside[*cell] != inside[*cell]).count() as f64;
                    *slot += SHAPE_FRINGE * (0..CELLS).filter(|cell| tile.fringe[*cell] != fringe[*cell]).count() as f64;
                }
            }
            for (dr, dc, dx, dy) in [(0i64, -1i64, TILE as i64, 0i64), (-1, 0, 0, TILE as i64)] {
                let (r, c) = (row as i64 + dr, column as i64 + dc);
                if r < 0 || c < 0 || chosen.get(c as usize, r as usize) < 0 {
                    continue;
                }
                let (px, py, turn) = &loom.tiles[chosen.get(c as usize, r as usize) as usize].place;
                if turn.contains('t') {
                    continue;
                }
                let next = (if turn.contains('h') { px - dx } else { px + dx }, if turn.contains('v') { py - dy } else { py + dy }, turn.clone());
                if let Some(following) = loom.by_place.get(&next) {
                    for (slot, at) in cost.iter_mut().zip(candidates) {
                        if at == following {
                            *slot -= GOING_ON;
                        }
                    }
                }
            }
            for (slot, drawn) in cost.iter_mut().zip(rng.randoms(candidates.len())) {
                *slot += drawn * 0.5;
            }
            let best = (0..cost.len()).fold(0, |best, at| if cost[at] < cost[best] { at } else { best });
            chosen.set(column, row, candidates[best] as i64);
            costs.set(column, row, raw[best]);
            for (at, rgb) in seen[best].iter().enumerate() {
                picture.set(x + at % TILE, y + at / TILE, *rgb);
            }
        }
    }
    (chosen, costs)
}

/// Where a band lies, in tiles: its first row, how many rows, how many
/// columns; whether it has a ground above it, below it, to meet.
#[derive(Clone, Copy, Debug)]
pub struct Band {
    pub top: usize,
    pub rows: usize,
    pub columns: usize,
    pub above: bool,
    pub below: bool,
}

/// A band: whole columns of the source side by side, each the one whose left
/// edge best meets the column before it, going on as in the game where it
/// can, and whose edges best meet the bands above and below it.
pub fn weave_columns(loom: &Loom, band: Band, canvas: &Grid<Rgba>, rng: &mut Rng) -> Result<Woven, Diagnostic> {
    let Band { top, rows, columns, above, below } = band;
    let mut by_column: Vec<(i64, Vec<(i64, usize)>)> = Vec::new();
    for (at, tile) in loom.tiles.iter().enumerate() {
        let x = tile.place.0;
        let marks = match by_column.iter_mut().find(|(known, _)| *known == x) {
            Some((_, marks)) => marks,
            None => {
                by_column.push((x, Vec::new()));
                &mut by_column.last_mut().expect("just put").1
            }
        };
        match marks.iter_mut().find(|(mark, _)| *mark == tile.mark) {
            Some((_, kept)) => *kept = at,
            None => marks.push((tile.mark, at)),
        }
    }
    let mut xs: Vec<i64> = by_column.iter().filter(|(_, marks)| marks.len() == rows).map(|(x, _)| *x).collect();
    xs.sort_unstable();
    if xs.is_empty() {
        return Err(loom.piece.where_().wrong(format!("\"{}\" has no whole column", loom.piece.name)));
    }
    let stacks: Vec<Vec<usize>> = xs
        .iter()
        .map(|x| {
            let marks = &by_column.iter().find(|(known, _)| known == x).expect("a column").1;
            (0..rows).map(|row| marks.iter().find(|(mark, _)| *mark == row as i64).map_or(0, |(_, at)| *at)).collect()
        })
        .collect();
    let rgb = |at: usize, cell: usize| -> [i32; 3] {
        let rgba = loom.tiles[at].frames[0][cell];
        [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]
    };
    let picture: Canvas = canvas.map(|rgba| [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]);
    let height = canvas.h / TILE;
    let mut chosen: Grid<i64> = Grid::filled(columns, height, -1);
    let mut costs: Grid<f64> = Grid::new(columns, height);
    let mut previous: Option<usize> = None;
    for column in 0..columns {
        let x = column * TILE;
        let mut cost = vec![0.0f64; xs.len()];
        for (slot, stack) in cost.iter_mut().zip(&stacks) {
            if above {
                let sum: i64 = (0..TILE).map(|at| difference(rgb(stack[0], at), picture.get(x + at, top * TILE - 1))).sum();
                *slot += sum as f64 / TILE as f64;
            }
            if below {
                let sum: i64 = (0..TILE).map(|at| difference(rgb(stack[rows - 1], (TILE - 1) * TILE + at), picture.get(x + at, (top + rows) * TILE))).sum();
                *slot += sum as f64 / TILE as f64;
            }
        }
        let mut raw = vec![0.0f64; xs.len()];
        if let Some(before) = previous {
            for (at, stack) in stacks.iter().enumerate() {
                let sum: i64 = (0..rows)
                    .flat_map(|row| (0..TILE).map(move |y| (row, y)))
                    .map(|(row, y)| difference(rgb(stack[row], y * TILE), rgb(stacks[before][row], y * TILE + TILE - 1)))
                    .sum();
                raw[at] = sum as f64 / (rows * TILE) as f64;
                cost[at] += raw[at];
            }
            if let Some(next) = xs.iter().position(|x| *x == xs[before] + TILE as i64) {
                cost[next] -= GOING_ON;
            }
        }
        for (slot, drawn) in cost.iter_mut().zip(rng.randoms(xs.len())) {
            *slot += drawn * 0.5;
        }
        let best = (0..cost.len()).fold(0, |best, at| if cost[at] < cost[best] { at } else { best });
        for (row, stacked) in stacks[best].iter().take(rows).enumerate() {
            chosen.set(column, top + row, *stacked as i64);
            costs.set(column, top + row, raw[best]);
        }
        previous = Some(best);
    }
    Ok((chosen, costs))
}
