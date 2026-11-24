//! What is remarked once a place is laid: paths that end on nothing,
//! entrances, free tiles out of reach, woven seams, and joins between pieces
//! that show (two things side by side whose colours never touch in the
//! places they come from, along a line of the grid).

use crate::fault::Diagnostic;
use crate::pixels::{Grid, Mask, TILE, border, component, components, grown, has, key_of, unique};
use crate::place::Place;
use crate::workshop::Workshop;

/// A join shows when its colours never touch in the game along this many pixels.
const RUN: usize = 6;
/// Free tiles out of reach are told from this many; between trees, from this many.
const ALONE: usize = 4;
const ALONE_IN_TREES: usize = 12;

pub fn remark_all(workshop: &Workshop, place: &mut Place) -> Result<(), Diagnostic> {
    path_ends(place);
    entrances(place);
    reach(place);
    let seams = place.seams.clone();
    for (number, x, y, cost) in seams.iter().take(6) {
        place.remark(*number, format!("seam at {x} {y} (gap {cost})"));
    }
    if seams.len() > 6 {
        place.remark(seams[6].0, format!("and {} more seams (see the drawing)", seams.len() - 6));
    }
    place.join_mask = joins(workshop, place)?;
    place.joins = grouped(&place.join_mask);
    if !place.joins.is_empty() {
        let told: Vec<String> = place.joins.iter().take(4).map(|(x0, y0, _, _, pixels)| format!("{x0} {y0} ({pixels} px)")).collect();
        let more = if place.joins.len() > 4 { format!(" and {} more", place.joins.len() - 4) } else { String::new() };
        let count = place.joins.len();
        place.remark(0, format!("{count} visible join(s) between pieces: {}{more} (in magenta on _walls.png)", told.join(", ")));
    }
    Ok(())
}

/// Where paths end: at an edge, by something, or by another path.
fn path_ends(place: &mut Place) {
    let (width, height) = (place.decor.width as i64, place.decor.height as i64);
    let tile = TILE as i64;
    for (number, x, y) in place.ends.clone() {
        let at_edge = x <= tile || y <= tile || x >= width - tile || y >= height - tile;
        let near = place.clip(x - 32, y - 32, 64, 64);
        let within = |mask: &dyn Fn(usize, usize) -> bool| near.is_some_and(|(x0, y0, x1, y1)| (y0..y1).any(|py| (x0..x1).any(|px| mask(px, py))));
        let by_thing = within(&|px, py| place.owner.get(px, py) > 0);
        let by_path = place.paths.iter().filter(|(other, _)| **other != number).any(|(_, path)| within(&|px, py| path.get(px, py)));
        if !(at_edge || by_thing || by_path) {
            place.remark(number, format!("the path ends at {x} {y} without meeting anything"));
        }
    }
}

/// Each entrance on a free tile of the place.
fn entrances(place: &mut Place) {
    for (number, x, y, name) in place.decor.entrances.clone() {
        if !(0 <= x && x < place.decor.width as i64 && 0 <= y && y < place.decor.height as i64) {
            place.remark(number, format!("the entrance \"{name}\" at {x} {y} is outside the place"));
        } else if place.walls.get(x as usize / TILE, y as usize / TILE) {
            place.remark(number, format!("the entrance \"{name}\" at {x} {y} is on a wall"));
        }
    }
}

/// Who comes in comes by an edge (or an entrance): free tiles that cannot be
/// reached are told, unless they are only a gap between trees.
pub(crate) fn reach(place: &mut Place) {
    let walls = place.walls.clone();
    let free = walls.not();
    let mut starts: Vec<(usize, usize)> = border(walls.w, walls.h).into_iter().filter(|(x, y)| free.get(*x, *y)).collect();
    for (_, x, y, _) in &place.decor.entrances {
        if 0 <= *x && *x < place.decor.width as i64 && 0 <= *y && *y < place.decor.height as i64 && free.get(*x as usize / TILE, *y as usize / TILE) {
            starts.push((*x as usize / TILE, *y as usize / TILE));
        }
    }
    if starts.is_empty() {
        place.remark(0, "no entrance by an edge: one only arrives here by appearing (or say \"entrance x y\")".to_string());
        return;
    }
    let alone = free.minus(&component(&free, &starts));
    let mut told = 0;
    for part in components(&alone) {
        let size = part.count();
        let round_it = grown(&part, 1, 1).and(&walls);
        let in_trees = round_it.any() && round_it.and(&place.tree_walls).count() * 2 >= round_it.count();
        if size < if in_trees { ALONE_IN_TREES } else { ALONE } {
            continue;
        }
        told += 1;
        if told <= 3
            && let Some((column, row)) = part.first()
        {
            place.remark(0, format!("{size} free tiles out of reach from the edges, near {} {}", column * TILE, row * TILE));
        }
    }
}

/// Every pair of colours side by side in the places of the game used.
fn known_pairs(workshop: &Workshop, levels: &std::collections::BTreeSet<i64>) -> Result<Vec<i64>, Diagnostic> {
    let mut found = Vec::new();
    for level in levels {
        let source = workshop.source(*level)?;
        let picture = source.picture();
        let key = |x: usize, y: usize| key_of(picture.get(x, y));
        for y in 0..picture.h {
            for x in 0..picture.w {
                if x + 1 < picture.w {
                    found.push(key(x, y) << 24 | key(x + 1, y));
                    found.push(key(x + 1, y) << 24 | key(x, y));
                }
                if y + 1 < picture.h {
                    found.push(key(x, y) << 24 | key(x, y + 1));
                    found.push(key(x, y + 1) << 24 | key(x, y));
                }
            }
        }
    }
    Ok(unique(found))
}

/// Where `strange` holds along columns `length` times or more in a row.
fn runs(strange: &Mask, length: usize) -> Mask {
    let mut out = Mask::new(strange.w, strange.h);
    let mut count = vec![0usize; strange.w];
    for row in 0..strange.h {
        for (column, counted) in count.iter_mut().enumerate() {
            *counted = if strange.get(column, row) { *counted + 1 } else { 0 };
            if *counted >= length {
                for back in 0..length {
                    out.set(column, row - back, true);
                }
            }
        }
    }
    out
}

/// The pixels where two things laid meet along a line of the grid and their
/// colours never touch in the game.
pub(crate) fn joins(workshop: &Workshop, place: &Place) -> Result<Mask, Diagnostic> {
    let picture = &place.lower[0];
    let (w, h) = (picture.w, picture.h);
    let key = |x: usize, y: usize| key_of(picture.get(x, y));
    // (A path's edge is drawn to meet the ground; a thing meets its own shadow.)
    let near_path = grown(&place.path, 5, 1);
    let origin = |x: usize, y: usize| if near_path.get(x, y) || place.shaded.get(x, y) { 0 } else { place.origin.get(x, y) };
    let known = known_pairs(workshop, &place.levels)?;
    let meets = |a: i32, b: i32| a != b && a > 0 && b > 0;
    let mut mask = Mask::new(w, h);
    // (Across: between columns 8k - 1 and 8k.)
    let xs: Vec<usize> = (TILE..w).step_by(TILE).collect();
    if !xs.is_empty() {
        let strange = Grid {
            w: xs.len(),
            h,
            cells: (0..h)
                .flat_map(|y| xs.iter().map(move |x| (*x, y)))
                .map(|(x, y)| meets(origin(x - 1, y), origin(x, y)) && !has(&known, key(x - 1, y) << 24 | key(x, y)))
                .collect(),
        };
        let found = runs(&strange, RUN);
        for (at, x) in xs.iter().enumerate() {
            for y in 0..h {
                if found.get(at, y) {
                    mask.set(x - 1, y, true);
                    mask.set(*x, y, true);
                }
            }
        }
    }
    // (Down: between rows; runs along them.)
    let ys: Vec<usize> = (TILE..h).step_by(TILE).collect();
    if !ys.is_empty() {
        let strange = Grid {
            w: ys.len(),
            h: w,
            cells: (0..w)
                .flat_map(|x| ys.iter().map(move |y| (x, *y)))
                .map(|(x, y)| meets(origin(x, y - 1), origin(x, y)) && !has(&known, key(x, y - 1) << 24 | key(x, y)))
                .collect(),
        };
        let found = runs(&strange, RUN);
        for (at, y) in ys.iter().enumerate() {
            for x in 0..w {
                if found.get(at, x) {
                    mask.set(x, y - 1, true);
                    mask.set(x, *y, true);
                }
            }
        }
    }
    Ok(mask)
}

/// Joins near each other as one: x0, y0, x1, y1, pixels; largest first.
pub(crate) fn grouped(mask: &Mask) -> Vec<(usize, usize, usize, usize, usize)> {
    let (columns, rows) = (mask.w / TILE, mask.h / TILE);
    let tiles = Grid {
        w: columns,
        h: rows,
        cells: (0..rows * columns)
            .map(|at| (0..TILE * TILE).any(|cell| mask.get((at % columns) * TILE + cell % TILE, (at / columns) * TILE + cell / TILE)))
            .collect(),
    };
    let mut out = Vec::new();
    for part in components(&grown(&tiles, 1, 1)) {
        let held: Vec<(usize, usize)> = (0..rows * columns).filter(|at| part.cells[*at] && tiles.cells[*at]).map(|at| (at % columns, at / columns)).collect();
        let (x0, y0) = (held.iter().map(|(x, _)| *x).min().unwrap_or(0) * TILE, held.iter().map(|(_, y)| *y).min().unwrap_or(0) * TILE);
        let (x1, y1) = ((held.iter().map(|(x, _)| *x).max().unwrap_or(0) + 1) * TILE, (held.iter().map(|(_, y)| *y).max().unwrap_or(0) + 1) * TILE);
        let pixels = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|(x, y)| mask.get(*x, *y)).count();
        out.push((x0, y0, x1, y1, pixels));
    }
    out.sort_by_key(|(x0, y0, _, _, pixels)| (std::cmp::Reverse(*pixels), *y0, *x0));
    out
}
