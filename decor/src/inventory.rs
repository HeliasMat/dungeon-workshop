//! The pieces of the game's places that the ROM itself tells apart, and only
//! those, found by themselves:
//! - `e<level>_<n>`: each object of a place's upper layer (what is walked
//!   under: a roof, a windmill, flowers), whose see-through pixels are truly
//!   so; `element=x,y` (a pixel of it: the cut keeps that object only),
//!   `moves` if the game animates it; nothing of the lower layer with it;
//! - `a<level>_<n>`: each stretch of the lower layer's tiles that the game
//!   animates (BPA tiles, cycled palettes: a mill, flowers, water), its tiles
//!   only (`tiles=c,r;…`);
//! - `o<n>`: each thing the scenes' scripts lay (a chest, a sign), its own picture;
//! - `r<level>`: the rest of each place, without its upper objects, backstage
//!   (the Workshop offers none): with them, the place as the game has it.
//!
//! All of it in `data/extra/game_pieces.txt`, made from the user's own copy at
//! extraction (never in the repository), never written by hand.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::fault::Diagnostic;
use crate::pixels::{Mask, Picture, TILE, components, grown};
use crate::source::Source;
use crate::weave::Workshop;

/// What the file says first.
const HEAD: [&str; 6] = [
    "# The pieces of the game's places that the ROM itself keeps apart, found on their own by decor inventory: this file is made again, it is not written by hand.",
    "# e<level>_<n>: an object of the upper layer (element=x,y;…: one pixel of each; moves: the game animates it);",
    "# a<level>_<n>: animated tiles of the ground layer (tiles=c,r;…: its tiles);",
    "# o<n>: an object laid by the scenes, its image;",
    "# r<level>: the rest of the place, without its upper objects, backstage.",
    "# The rest and its objects, each in its place: the place as the game has it (decor recreate).",
];

/// A piece of the upper layer of so few pixels is a speck; pieces so near
/// are one (a flower's petals, a sign and its post).
const UPPER_FEWEST: usize = 24;
const UPPER_NEAR: usize = 3;
/// A piece wider or higher than this is a stretch of its place, not a thing to lay.
const LARGEST: usize = 384;

/// An object of the upper layer: its rectangle, a pixel of it (from the
/// rectangle's corner), whether it moves.
pub struct Upper {
    pub rect: (usize, usize, usize, usize),
    pub seed: (usize, usize),
    pub moves: bool,
}

/// Whether pictures differ, frame to frame, in a rectangle.
fn moves_in(frames: &[Picture], (x0, y0, x1, y1): (usize, usize, usize, usize)) -> bool {
    frames.iter().skip(1).any(|frame| (y0..y1).any(|y| (x0..x1).any(|x| frame.get(x, y) != frames[0].get(x, y))))
}

/// The rectangle on whole tiles round the pixels of a mask, within it.
fn tile_rect(mask: &Mask) -> (usize, usize, usize, usize) {
    let (mut x0, mut y0, mut x1, mut y1) = (mask.w, mask.h, 0, 0);
    for at in (0..mask.cells.len()).filter(|at| mask.cells[*at]) {
        let (x, y) = (at % mask.w, at / mask.w);
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
    }
    let up = |value: usize, most: usize| value.div_ceil(TILE).saturating_mul(TILE).min(most);
    (x0 / TILE * TILE, y0 / TILE * TILE, up(x1, mask.w), up(y1, mask.h))
}

/// Each object of the upper layer: its pixels seen in any frame, near ones
/// together, those stuck together apart.
pub fn upper_objects(source: &Source) -> Vec<Upper> {
    let first = &source.upper[0];
    let (w, h) = (first.w, first.h);
    let mut seen = Mask::new(w, h);
    for frame in &source.upper {
        for (cell, rgba) in seen.cells.iter_mut().zip(&frame.cells) {
            *cell |= rgba[3] > 0;
        }
    }
    if !seen.any() {
        return Vec::new();
    }
    let joined = grown(&seen, UPPER_NEAR, 1);
    // (A pixel's colour: as the first frame that shows it has it.)
    let colour = |x: usize, y: usize| {
        let shown = source.upper.iter().map(|frame| frame.get(x, y)).find(|rgba| rgba[3] > 0).unwrap_or_default();
        [shown[0], shown[1], shown[2]]
    };
    let pieces = components(&joined).into_iter().map(|part| part.and(&seen)).filter(|own| own.count() >= UPPER_FEWEST);
    let mut found = Vec::new();
    for own in pieces.flat_map(|own| crate::pixels::objects(&own, &colour)) {
        let Some((sx, sy)) = own.first() else {
            continue;
        };
        let rect = tile_rect(&own);
        if rect.2 - rect.0 > LARGEST || rect.3 - rect.1 > LARGEST {
            continue;
        }
        let moves = moves_in(&source.upper, rect);
        found.push(Upper { rect, seed: (sx - rect.0, sy - rect.1), moves });
    }
    found.sort_by_key(|upper| (upper.rect.1, upper.rect.0));
    found
}

/// Each stretch of the lower layer's tiles the game animates (tiles side by
/// side): its tiles (column, row) in the place.
pub fn animated_tiles(source: &Source) -> Vec<Vec<(usize, usize)>> {
    let frames = &source.frames;
    let (w, h) = (frames[0].w, frames[0].h);
    let (columns, rows) = (w / TILE, h / TILE);
    let mut moving = Mask::new(columns, rows);
    for row in 0..rows {
        for column in 0..columns {
            let rect = (column * TILE, row * TILE, (column + 1) * TILE, (row + 1) * TILE);
            moving.set(column, row, moves_in(frames, rect));
        }
    }
    let mut found = Vec::new();
    for part in components(&moving) {
        let tiles: Vec<(usize, usize)> = (0..part.cells.len()).filter(|at| part.cells[*at]).map(|at| (at % columns, at / columns)).collect();
        let (c0, r0) = (tiles.iter().map(|tile| tile.0).min().unwrap_or(0), tiles.iter().map(|tile| tile.1).min().unwrap_or(0));
        let (c1, r1) = (tiles.iter().map(|tile| tile.0 + 1).max().unwrap_or(0), tiles.iter().map(|tile| tile.1 + 1).max().unwrap_or(0));
        if (c1 - c0) * TILE > LARGEST || (r1 - r0) * TILE > LARGEST {
            continue;
        }
        found.push(tiles);
    }
    found.sort_by_key(|tiles| tiles.iter().map(|(column, row)| (*row, *column)).min());
    found
}

/// The catalogue's lines of a place: its upper objects, its animated tiles,
/// then its rest.
fn lines_of(workshop: &Workshop, level: i64) -> Result<Vec<String>, Diagnostic> {
    // (A place the game does not draw whole: none.)
    let Ok(source) = workshop.source(level) else {
        return Ok(Vec::new());
    };
    let mut lines = Vec::new();
    for (at, upper) in upper_objects(&source).iter().enumerate() {
        let (x0, y0, x1, y1) = upper.rect;
        let moves = if upper.moves { " moves" } else { "" };
        let (sx, sy) = upper.seed;
        lines.push(format!("e{level}_{:<3}  object  {level}  {x0} {y0} {x1} {y1} over upper element={sx},{sy}{moves}", at + 1));
    }
    for (at, tiles) in animated_tiles(&source).iter().enumerate() {
        let (c0, r0) = (tiles.iter().map(|tile| tile.0).min().unwrap_or(0), tiles.iter().map(|tile| tile.1).min().unwrap_or(0));
        let (c1, r1) = (tiles.iter().map(|tile| tile.0 + 1).max().unwrap_or(0), tiles.iter().map(|tile| tile.1 + 1).max().unwrap_or(0));
        let said: Vec<String> = tiles.iter().map(|(column, row)| format!("{},{}", column - c0, row - r0)).collect();
        lines.push(format!(
            "a{level}_{:<3}  object  {level}  {} {} {} {} base upper tiles={} moves",
            at + 1,
            c0 * TILE,
            r0 * TILE,
            c1 * TILE,
            r1 * TILE,
            said.join(";")
        ));
    }
    // (Its rest: the place without its upper objects, backstage.)
    let picture = source.picture();
    lines.push(format!("r{level}  object  {level}  0 0 {} {} base upper rest", picture.w, picture.h));
    Ok(lines)
}

/// The things the scenes' scripts lay, each its first picture: o<n>.
fn props_lines(workshop: &Workshop) -> Vec<String> {
    let folder = &workshop.game.folder;
    let mut found = BTreeSet::new();
    for row in workshop.game.rows("props.txt") {
        let Some(number) = row.first().filter(|_| row.len() > 3 && row[1] == "0").and_then(|number| number.parse::<u32>().ok()) else {
            continue;
        };
        // (Its sheet holds all its pictures, each `w` by `h`: the first, cropped to what it draws.)
        let (Ok(cell_w), Ok(cell_h)) = (row[2].parse::<u32>(), row[3].parse::<u32>()) else {
            continue;
        };
        let Ok(sheet) = image::open(assets::data::path(folder, &format!("props/{number}_0.png"))) else {
            continue;
        };
        let sheet = sheet.to_rgba8();
        let (x0, y0, x1, y1) = (0..cell_h.min(sheet.height()))
            .flat_map(|y| (0..cell_w.min(sheet.width())).map(move |x| (x, y)))
            .filter(|(x, y)| sheet.get_pixel(*x, *y)[3] > 0)
            .fold((u32::MAX, u32::MAX, 0, 0), |(x0, y0, x1, y1), (x, y)| (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)));
        if x1 > x0 {
            found.insert((number, x0, y0, x1, y1));
        }
    }
    found.into_iter().map(|(number, x0, y0, x1, y1)| format!("o{number:<5}  object  o{number}  {x0} {y0} {x1} {y1}")).collect()
}

/// The places the game draws, by their numbers.
fn levels(folder: &str) -> Vec<i64> {
    let first = assets::data::path(folder, "grounds/pictures/0_f0.png");
    let pictures = Path::new(&first).parent().map(Path::to_path_buf).unwrap_or_default();
    let names = std::fs::read_dir(pictures).into_iter().flatten().flatten().map(|file| file.file_name().to_string_lossy().into_owned());
    let found: BTreeSet<i64> = names.filter_map(|name| name.split('_').next().and_then(|level| level.parse().ok())).collect();
    found.into_iter().collect()
}

/// The level a line of the file is of, if it is a place's.
fn level_of(line: &str) -> Option<i64> {
    let word = line.split(' ').next().unwrap_or_default();
    let rest = word.strip_prefix('e').or_else(|| word.strip_prefix('a'));
    match rest {
        Some(rest) => rest.split_once('_').and_then(|(level, _)| level.parse().ok()),
        None => word.strip_prefix('r').and_then(|level| level.parse().ok()),
    }
}

/// Every place and thing of the scenes, written into `out`; or only the
/// places asked, the others' lines kept as `out` has them. What is told.
pub fn inventory(workshop: &Workshop, asked: &[i64], out: &Path) -> Result<String, Diagnostic> {
    let mut by_level: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    if !asked.is_empty() {
        for line in std::fs::read_to_string(out).unwrap_or_default().lines() {
            if let Some(level) = level_of(line) {
                by_level.entry(level).or_default().push(line.to_string());
            }
        }
    }
    let levels = if asked.is_empty() { levels(&workshop.game.folder) } else { asked.to_vec() };
    for level in levels {
        by_level.insert(level, lines_of(workshop, level)?);
        workshop.forget();
    }
    let lines: Vec<String> = HEAD.iter().map(|line| line.to_string()).chain(by_level.into_values().flatten()).chain(props_lines(workshop)).collect();
    std::fs::write(out, lines.join("\n") + "\n").map_err(|why| Diagnostic::said(format!("{} : {why}", out.display())))?;
    Ok(format!("{}: {} pieces", out.display(), lines.len() - HEAD.len()))
}
