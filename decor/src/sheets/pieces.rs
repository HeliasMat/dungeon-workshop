//! The pieces looked at: the catalogue by kind and by place (`contact`); one
//! or several on black and on white, large, with the grid of their tiles
//! (`element`); the things of a game's place put back where they were, on
//! black (`reassemble`); a game's place with its grid, to read a rectangle off
//! (`level_grid`); what is left of a ground on each piece, measured
//! (`check`): a rim of grass round it, holes in it; each ground woven
//! alone (`grounds`).
//!
//! A piece well cut shows nothing but itself on black as on white: its ground
//! shows on one or the other, its holes on both.

use std::path::PathBuf;

use super::canvas::{Canvas, LETTER_HIGH, Rgb};
use super::{game_picture, sheet_path};
use crate::build::Builder;
use crate::catalogue::{KINDS, Level, Piece};
use crate::cut::cut;
use crate::fault::Diagnostic;
use crate::lang::parse;
use crate::matte::shaded;
use crate::piece::Cut;
use crate::pixels::{Grid, Mask, Rgba, TILE, border, component, has, key_of, unique};
use crate::weave::shadow_colours;
use crate::workshop::{Workshop, named_colours, seen_in};

/// The grey between the two halves, the grid on them, the labels.
const GAP: Rgb = [128; 3];
const LINE: Rgb = [70; 3];
const COLUMN_MARK: Rgb = [255, 0, 255];
const ROW_MARK: Rgb = [0, 255, 255];
/// A sheet of several pieces this wide, at most.
const SHEET_WIDE: usize = 1400;
/// A piece whose outline has this share in its ground's colours is said to keep a rim of it.
const RIM: f64 = 0.1;
/// A swatch of a ground woven alone, and how many go across a sheet.
const SWATCH: (usize, usize) = (128, 96);
const SWATCHES_ACROSS: usize = 6;

/// A piece as seen, its two layers together: colours, and where it is.
fn shown(made: &Cut) -> (Canvas, Mask) {
    let (w, h) = made.size();
    let (lower, upper) = (&made.lower[0], &made.upper[0]);
    let mut colours = Canvas::new(w, h, [0; 3]);
    let mut seen = Mask::new(w, h);
    for at in 0..w * h {
        let [r, g, b, a] = if upper.cells[at][3] > 0 { upper.cells[at] } else { lower.cells[at] };
        colours.cells[at] = [r, g, b];
        seen.cells[at] = a > 0 || lower.cells[at][3] > 0;
    }
    (colours, seen)
}

/// A piece on black then on white, side by side, `scale` times as large.
fn on_black_and_white(made: &Cut, scale: usize) -> Canvas {
    let (colours, seen) = shown(made);
    let (w, h) = (seen.w, seen.h);
    let mut pair = Canvas::new(w * 2 + 2, h, GAP);
    for (left, ground) in [(0, 0), (w + 2, 255)] {
        for y in 0..h {
            for x in 0..w {
                let colour = if seen.get(x, y) { colours.get(x, y) } else { [ground; 3] };
                pair.cells[y * pair.w + left + x] = colour;
            }
        }
    }
    pair.scaled(scale)
}

/// A grid of the tiles over the black half, columns and rows numbered as
/// `without=` and `keep=` count them.
fn gridded(sheet: &mut Canvas, wide: usize, high: usize, scale: usize) {
    for x in (0..=wide).step_by(TILE) {
        sheet.fill(((x * scale) as i64, 0, (x * scale) as i64, (high * scale) as i64), LINE);
    }
    for y in (0..=high).step_by(TILE) {
        sheet.fill((0, (y * scale) as i64, (wide * scale) as i64, (y * scale) as i64), LINE);
    }
    for column in 0..wide / TILE {
        sheet.text((column * TILE * scale + 2) as i64, 1, &column.to_string(), COLUMN_MARK);
    }
    for row in 0..high / TILE {
        sheet.text(2, (row * TILE * scale + 10) as i64, &row.to_string(), ROW_MARK);
    }
}

fn piece_named<'a>(workshop: &'a Workshop, name: &str) -> Result<&'a Piece, Diagnostic> {
    workshop.catalogue.get(name).ok_or_else(|| Diagnostic::said(format!("no piece \"{name}\".{}", crate::lang::closest(name, workshop.catalogue.names()))))
}

/// One piece, large with its grid; or several side by side, twice as large:
/// `target/decors/element_<name>.png`.
pub fn element(workshop: &Workshop, names: &[String]) -> Result<PathBuf, Diagnostic> {
    let pieces = names.iter().map(|name| piece_named(workshop, name)).collect::<Result<Vec<_>, _>>()?;
    if let [piece] = pieces.as_slice() {
        let made = cut(workshop, piece)?;
        let (w, h) = made.size();
        let scale = (480 / w.max(h)).clamp(2, 8);
        let mut sheet = on_black_and_white(&made, scale);
        gridded(&mut sheet, w, h, scale);
        let path = sheet_path(workshop.project.root(), &format!("element_{}.png", piece.name))?;
        sheet.save(&path)?;
        return Ok(path);
    }
    let mut rows: Vec<Vec<(&str, Canvas)>> = vec![Vec::new()];
    let mut x = 0;
    for piece in pieces {
        let shown = on_black_and_white(&cut(workshop, piece)?, 2);
        let row = rows.last_mut().expect("a row");
        if !row.is_empty() && x + shown.w > SHEET_WIDE {
            rows.push(Vec::new());
            x = 0;
        }
        x += shown.w + 12;
        rows.last_mut().expect("a row").push((&piece.name, shown));
    }
    let high = |row: &Vec<(&str, Canvas)>| row.iter().map(|(_, shown)| shown.h).max().unwrap_or(0) + LETTER_HIGH + 6;
    let mut sheet = Canvas::new(SHEET_WIDE, rows.iter().map(high).sum(), [60; 3]);
    let mut y = 0;
    for row in &rows {
        let mut x = 0;
        for (name, shown) in row {
            sheet.text(x + 2, y + 2, name, [255, 255, 0]);
            sheet.paste(shown, x, y + LETTER_HIGH as i64 + 4);
            x += shown.w as i64 + 12;
        }
        y += high(row) as i64;
    }
    let path = sheet_path(workshop.project.root(), &format!("element_{}_et_{}.png", names[0], names.len() - 1))?;
    sheet.save(&path)?;
    Ok(path)
}

/// Whether a piece is a thing cut from its place (not a ground, nor the
/// place whole, nor a piece kept with its ground).
fn a_thing(piece: &Piece) -> bool {
    ["object", "water", "tree"].contains(&piece.kind.as_str()) && !piece.has("base") && !piece.name.ends_with("_tout")
}

fn turned_over(piece: &Piece) -> bool {
    piece.option("flip").is_some_and(|turn| turn.contains('h'))
}

/// The things of a game's place, each cut and put back where it was, on
/// black, twice as large, each with its name: what is not on black is not of
/// them. `target/decors/reassembled_<level>.png`.
pub fn put_back_sheet(workshop: &Workshop, level: i64) -> Result<PathBuf, Diagnostic> {
    let place = workshop.source(level)?;
    let (w, h) = (place.picture().w, place.picture().h);
    let mut canvas = Canvas::new(w, h, [0; 3]);
    let mut labels = Vec::new();
    for piece in workshop.catalogue.pieces.iter().filter(|piece| piece.place() == Some(level) && a_thing(piece)) {
        let (mut colours, mut seen) = shown(&cut(workshop, piece)?);
        if turned_over(piece) {
            colours = colours.mirrored();
            seen = seen.mirrored();
        }
        let (x0, y0, ..) = piece.rect;
        for y in 0..seen.h {
            for x in (0..seen.w).filter(|x| seen.get(*x, y)) {
                canvas.dot(x0 + x as i64, y0 + y as i64, colours.get(x, y));
            }
        }
        labels.push((&piece.name, x0, y0));
    }
    let mut sheet = canvas.scaled(2);
    for (name, x, y) in labels {
        sheet.text(x * 2 + 2, y * 2 + 1, name, COLUMN_MARK);
    }
    let path = sheet_path(workshop.project.root(), &format!("reassembled_{level}.png"))?;
    sheet.save(&path)?;
    Ok(path)
}

/// A game's place, or a rectangle of it, four times as large (twice, whole),
/// its tiles in grey, every 32 pixels in magenta with its coordinate: to
/// read a piece's rectangle off. `target/decors/grille_<level>[_rect].png`.
pub fn level_grid(root: &std::path::Path, data: &str, level: i64, rect: Option<(i64, i64, i64, i64)>) -> Result<PathBuf, Diagnostic> {
    let picture = game_picture(data, &format!("backdrops/{level}.png"))?;
    let (x0, y0, x1, y1) = rect.unwrap_or((0, 0, picture.w as i64, picture.h as i64));
    let mut part = Canvas::new((x1 - x0).max(0) as usize, (y1 - y0).max(0) as usize, [0; 3]);
    for y in 0..part.h {
        for x in 0..part.w {
            let (from_x, from_y) = (x0 + x as i64, y0 + y as i64);
            if (0..picture.w as i64).contains(&from_x) && (0..picture.h as i64).contains(&from_y) {
                part.cells[y * part.w + x] = picture.get(from_x as usize, from_y as usize);
            }
        }
    }
    let scale = if rect.is_some() { 4 } else { 2 };
    let mut big = part.scaled(scale as usize);
    let (wide, high) = (big.w as i64, big.h as i64);
    let mark = |at: i64| if at % 32 == 0 { COLUMN_MARK } else { LINE };
    for x in (x0 - x0.rem_euclid(TILE as i64)..x1).step_by(TILE) {
        big.fill(((x - x0) * scale, 0, (x - x0) * scale, high), mark(x));
    }
    for y in (y0 - y0.rem_euclid(TILE as i64)..y1).step_by(TILE) {
        big.fill((0, (y - y0) * scale, wide, (y - y0) * scale), mark(y));
    }
    for x in (x0 + (-x0).rem_euclid(32)..x1).step_by(32) {
        big.text((x - x0) * scale + 2, 2, &x.to_string(), [255; 3]);
    }
    for y in (y0 + (-y0).rem_euclid(32)..y1).step_by(32) {
        big.text(2, (y - y0) * scale + 2, &y.to_string(), [255; 3]);
    }
    let said = rect.map_or(String::new(), |(x0, y0, x1, y1)| format!("_{x0}_{y0}_{x1}_{y1}"));
    let path = sheet_path(root, &format!("grille_{level}{said}.png"))?;
    big.save(&path)?;
    Ok(path)
}

/// The colours of the grounds and paths the catalogue names that a place
/// shows (not its bands, which hold its rocks too): their own, which a
/// piece's outline should not be; and with their shadows, which a hole in it
/// may be (outlines share those).
fn grounds_of_place(workshop: &Workshop, level: i64) -> Result<(Vec<i64>, Vec<i64>), Diagnostic> {
    let mut own = Vec::new();
    let mut shadows = Vec::new();
    for ground in seen_in(workshop, level)?.iter().filter(|ground| ground.kind != "edge") {
        own.extend(named_colours(workshop, ground)?);
        shadows.extend(shadow_colours(ground));
    }
    let shaded = unique(own.iter().copied().chain(shadows));
    Ok((unique(own), shaded))
}

/// What of its ground a piece keeps, as the game drew it there: the share of
/// its outline in its place's ground colours (a rim of grass round it), and
/// the pixels of its holes the game drew in no ground's colour (a hole in
/// it, not the grass between a fence's rails).
fn ground_left(workshop: &Workshop, piece: &Piece, made: &Cut) -> Result<(f64, usize), Diagnostic> {
    let (colours, seen) = shown(made);
    let (w, h) = (seen.w, seen.h);
    let outside = component(&seen.not(), &border(w, h));
    let mut holes = seen.not().minus(&outside);
    let Some(level) = piece.place().filter(|_| !piece.has("extract")) else {
        return Ok((0.0, holes.count()));
    };
    let (grounds, shaded) = grounds_of_place(workshop, level)?;
    let out = |x: i64, y: i64| x < 0 || y < 0 || x >= w as i64 || y >= h as i64 || outside.get(x as usize, y as usize);
    let (mut rim, mut grassy) = (0, 0);
    for y in 0..h {
        for x in (0..w).filter(|x| seen.get(*x, y)) {
            let (cx, cy) = (x as i64, y as i64);
            if out(cx, cy - 1) || out(cx, cy + 1) || out(cx - 1, cy) || out(cx + 1, cy) {
                rim += 1;
                let [r, g, b] = colours.get(x, y);
                grassy += usize::from(has(&grounds, key_of([r, g, b, 0])));
            }
        }
    }
    let source = workshop.source(level)?;
    let (x0, y0, ..) = piece.rect;
    for y in 0..h {
        for x in 0..w {
            let from_x = if turned_over(piece) { w - 1 - x } else { x };
            let drawn: Rgba = source.picture().get(x0 as usize + from_x, y0 as usize + y);
            if has(&shaded, key_of(drawn)) {
                holes.set(x, y, false);
            }
        }
    }
    Ok((grassy as f64 / rim.max(1) as f64, holes.count()))
}

/// The pieces chosen, the worst first: the share of their outline in their
/// ground's colours, their holes. The lines to print.
pub fn outline_check(workshop: &Workshop, chosen: &[&Piece]) -> Vec<String> {
    let mut rows: Vec<(f64, usize, &str, String)> = Vec::new();
    for piece in chosen.iter().filter(|piece| a_thing(piece)) {
        match cut(workshop, piece).and_then(|made| ground_left(workshop, piece, &made)) {
            Ok((full, holes)) => rows.push((full, holes, &piece.name, String::new())),
            Err(why) => rows.push((2.0, 0, &piece.name, format!("at fault: {why}"))),
        }
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    let mut lines = Vec::new();
    for (full, holes, name, said) in &rows {
        if !said.is_empty() {
            lines.push(format!("  {name:28} {said}"));
            continue;
        }
        let flag = if *full > RIM {
            "  <- a rim of ground"
        } else if *holes > 0 {
            "  <- holes"
        } else {
            ""
        };
        lines.push(format!("  {name:28} outline on ground {:>3.0}%   holes {holes:4}{flag}", full * 100.0));
    }
    let worst = rows.iter().filter(|(full, holes, _, said)| !said.is_empty() || *full > RIM || *holes > 0).count();
    lines.push(format!("{} pieces, {worst} to look over again (decor element <name>)", rows.len()));
    lines
}

/// Each ground named woven alone on a swatch, twice as large, side by side
/// with its name: to choose one. target/decors/grounds_<...>.png.
pub fn ground_swatches(workshop: &Workshop, names: &[String]) -> Result<PathBuf, Diagnostic> {
    let mut cells = Vec::new();
    for name in names {
        let decor = parse(&format!("decor: trial {} {}\nground {name}\n", SWATCH.0, SWATCH.1), "trial.decor")?;
        let woven = Builder::new(workshop, decor).build()?;
        cells.push((name, Canvas::of(&woven.lower[0]).scaled(2)));
    }
    let (wide, high) = (SWATCH.0 * 2 + 8, SWATCH.1 * 2 + LETTER_HIGH + 8);
    let rows = cells.len().div_ceil(SWATCHES_ACROSS);
    let mut sheet = Canvas::new(cells.len().min(SWATCHES_ACROSS) * wide, rows * high, [50; 3]);
    for (at, (name, swatch)) in cells.iter().enumerate() {
        let (x, y) = ((at % SWATCHES_ACROSS * wide) as i64, (at / SWATCHES_ACROSS * high) as i64);
        sheet.paste(swatch, x + 4, y + LETTER_HIGH as i64 + 6);
        sheet.text(x + 4, y + 2, name, [255, 255, 0]);
    }
    let more = if names.len() > 1 { format!("_and_{}", names.len() - 1) } else { String::new() };
    let path = sheet_path(workshop.project.root(), &format!("grounds_{}{more}.png", names[0]))?;
    sheet.save(&path)?;
    Ok(path)
}

/// A grey checker, what shows where a piece is cut out.
fn checker(w: usize, h: usize) -> Vec<Rgba> {
    (0..w * h).map(|at| if (at % w / 8 + at / w / 8) % 2 == 1 { [225, 225, 225, 255] } else { [200, 200, 200, 255] }).collect()
}

/// What a piece looks like alone: a ground's tiles side by side; a path or a
/// band as in its place; a thing on a checker, its shadow on it. Bare: a
/// thing on black and on white side by side, without its shadow: what is
/// left of its ground shows on one or the other.
pub fn piece_picture(workshop: &Workshop, piece: &Piece, bare: bool) -> Result<Canvas, Diagnostic> {
    if piece.kind == "ground" {
        let loom = workshop.loom(piece)?;
        let across = loom.tiles.len().min(12);
        let rows = loom.tiles.len().div_ceil(across.max(1)).min(8);
        let mut picture = Canvas::new(across * TILE, rows * TILE, [0; 3]);
        for (at, tile) in loom.tiles.iter().take(across * rows).enumerate() {
            for (pixel, [r, g, b, _]) in tile.frames[0].iter().enumerate() {
                picture.dot((at % across * TILE + pixel % TILE) as i64, (at / across * TILE + pixel / TILE) as i64, [*r, *g, *b]);
            }
        }
        return Ok(picture);
    }
    if piece.kind == "edge" || piece.kind == "path" {
        let (x0, y0, x1, y1) = piece.rect;
        let source = workshop.source(piece.place().unwrap_or_default())?;
        return Ok(Canvas::of(&source.picture().crop(x0 as usize, y0 as usize, x1 as usize, y1 as usize)));
    }
    let made = cut(workshop, piece)?;
    let (w, h) = made.size();
    let over = |under: &mut Vec<Rgba>| {
        for layer in [&made.lower[0], &made.upper[0]] {
            for (cell, rgba) in under.iter_mut().zip(&layer.cells) {
                if rgba[3] > 0 {
                    *cell = *rgba;
                }
            }
        }
    };
    if bare {
        let mut pair = Canvas::new(w * 2 + 2, h, GAP);
        for (left, ground) in [(0, 0), (w + 2, 255)] {
            let mut half = vec![[ground, ground, ground, 255]; w * h];
            over(&mut half);
            pair.paste(&Canvas::of(&Grid { w, h, cells: half }), left as i64, 0);
        }
        return Ok(pair);
    }
    let mut picture = checker(w, h);
    shaded(&mut picture, &made.shade, &workshop.ramps);
    over(&mut picture);
    Ok(Canvas::of(&Grid { w, h, cells: picture }))
}

/// A level as the catalogue says it: `11`, `o123`.
fn level_said(level: &Level) -> String {
    match level {
        Level::Place(level) => level.to_string(),
        Level::Thing(thing) => format!("o{thing}"),
    }
}

/// Where a piece comes from, as a sheet groups them.
fn source_name(level: &Level) -> String {
    match level {
        Level::Place(level) => format!("place {level}"),
        Level::Thing(thing) => format!("object {thing}"),
    }
}

/// A piece on the catalogue's sheet: its number, itself, as shown, its size said.
type Shown<'a> = (usize, &'a Piece, Canvas, String);

/// The catalogue, grouped by kind and by place, each piece numbered with its
/// name and size: the sheet, and the lines to print.
pub fn contact(workshop: &Workshop, chosen: &[&Piece], bare: bool) -> (Canvas, Vec<String>) {
    const MOST: usize = 168;
    const COLUMNS: usize = 6;
    let mut chosen = chosen.to_vec();
    chosen.sort_by_key(|piece| (KINDS.iter().position(|kind| *kind == piece.kind), format!("{:0>5}", level_said(&piece.level)), piece.line));
    let mut blocks: Vec<(String, Vec<Shown>)> = Vec::new();
    let mut lines = Vec::new();
    let mut number = 0;
    for piece in chosen {
        let picture = match piece_picture(workshop, piece, bare && !["ground", "edge", "path"].contains(&piece.kind.as_str())) {
            Ok(picture) => picture,
            Err(why) => {
                lines.push(format!("     {}: {why}", piece.name));
                continue;
            }
        };
        number += 1;
        let group = format!("{} - {}", piece.kind, source_name(&piece.level));
        if blocks.last().is_none_or(|(last, _)| *last != group) {
            blocks.push((group, Vec::new()));
        }
        let (w, h) = (picture.w, picture.h);
        // (Small pieces shown bigger, pixel for pixel; large ones smaller.)
        let (shown, scale) = if w.max(h) > MOST {
            let factor = w.max(h).div_ceil(MOST);
            (picture.every(factor), format!("1/{factor}"))
        } else {
            let factor = (MOST / w.max(h).max(1)).clamp(1, 3);
            (picture.scaled(factor), format!("x{factor}"))
        };
        blocks.last_mut().expect("a block").1.push((number, piece, shown, format!("{w}x{h}  {scale}")));
        lines.push(format!("{number:3}. {:24} {:7} {:5} {w:4}x{h:<4} {}", piece.name, piece.kind, level_said(&piece.level), piece.note));
    }
    let width = COLUMNS * (MOST + 12);
    let rows: Vec<(&String, &[Shown])> = blocks.iter().flat_map(|(group, items)| items.chunks(COLUMNS).map(move |row| (group, row))).collect();
    let row_high = |row: &[Shown]| row.iter().map(|item| item.2.h).max().unwrap_or(0) + 2 * LETTER_HIGH + 8;
    let height = 18 * blocks.len() + rows.iter().map(|(_, row)| row_high(row)).sum::<usize>();
    let mut sheet = Canvas::new(width, height.max(1), [255; 3]);
    let (mut y, mut last) = (0, None);
    for (group, row) in rows {
        if last != Some(group) {
            sheet.fill((0, y, width as i64, y + 15), [60; 3]);
            sheet.text(4, y + 3, group, [255; 3]);
            (y, last) = (y + 18, Some(group));
        }
        for (at, (number, piece, picture, size)) in row.iter().enumerate() {
            let x = (at * (MOST + 12) + 4) as i64;
            sheet.text(x, y + 2, &format!("{number}. {}", piece.name), [0; 3]);
            sheet.text(x, y + LETTER_HIGH as i64 + 4, size, [90; 3]);
            sheet.paste(picture, x, y + 2 * LETTER_HIGH as i64 + 6);
        }
        y += row_high(row) as i64;
    }
    (sheet, lines)
}

/// How many elements a labelling sheet holds, across and in all; how large each is shown at most.
const LABEL_ACROSS: usize = 10;
const LABEL_MOST: usize = 100;
const LABEL_CELL: usize = 104;

/// The elements named, on sheets to label them by eye (`decor labels`): a
/// hundred a sheet, each shown at most 104 pixels (small ones twice or three
/// times as large), its name and size under it; `<out>/labels_<n>.png`.
/// The sheets written, and the names not cut.
pub fn label_sheets(workshop: &Workshop, names: &[String], out: &std::path::Path) -> Result<(Vec<PathBuf>, Vec<String>), Diagnostic> {
    std::fs::create_dir_all(out).map_err(|why| Diagnostic::said(format!("{}: {why}", out.display())))?;
    let mut written = Vec::new();
    let mut failed = Vec::new();
    let high = LABEL_CELL + 2 * LETTER_HIGH + 8;
    for (at, chunk) in names.chunks(LABEL_MOST).enumerate() {
        let rows = chunk.len().div_ceil(LABEL_ACROSS);
        let mut sheet = Canvas::new(LABEL_ACROSS * (LABEL_CELL + 8), rows * high, [96, 96, 112]);
        for (number, name) in chunk.iter().enumerate() {
            let (x, y) = ((number % LABEL_ACROSS * (LABEL_CELL + 8) + 4) as i64, (number / LABEL_ACROSS * high + 2) as i64);
            let picture = workshop
                .catalogue
                .get(name)
                .ok_or_else(|| Diagnostic::said(format!("no piece \"{name}\"")))
                .and_then(|piece| piece_picture(workshop, piece, false));
            let Ok(picture) = picture else {
                failed.push(name.clone());
                continue;
            };
            let side = picture.w.max(picture.h).max(1);
            let shown = if side > LABEL_CELL { picture.every(side.div_ceil(LABEL_CELL)) } else { picture.scaled((LABEL_CELL / side).clamp(1, 3)) };
            sheet.paste(&shown, x + ((LABEL_CELL - shown.w.min(LABEL_CELL)) / 2) as i64, y + (LABEL_CELL - shown.h.min(LABEL_CELL)) as i64);
            sheet.text(x, y + LABEL_CELL as i64 + 2, name, [255, 255, 255]);
            sheet.text(x, y + (LABEL_CELL + LETTER_HIGH) as i64 + 4, &format!("{}x{}", picture.w, picture.h), [200, 200, 200]);
        }
        let path = out.join(format!("labels_{:02}.png", at + 1));
        sheet.save(&path)?;
        written.push(path);
        workshop.forget();
    }
    Ok((written, failed))
}

/// The game's places named, on sheets to tell their kinds by eye (`decor
/// labels <folder> places`): thirty a sheet, each its picture made small,
/// its level and code under it; `<out>/places_<n>.png`.
pub fn place_sheets(data: &str, places: &[(i64, String)], out: &std::path::Path) -> Result<Vec<PathBuf>, Diagnostic> {
    const ACROSS: usize = 6;
    const CELL: (usize, usize) = (200, 150);
    std::fs::create_dir_all(out).map_err(|why| Diagnostic::said(format!("{}: {why}", out.display())))?;
    let mut written = Vec::new();
    let high = CELL.1 + LETTER_HIGH + 8;
    for (at, chunk) in places.chunks(30).enumerate() {
        let mut sheet = Canvas::new(ACROSS * (CELL.0 + 8), chunk.len().div_ceil(ACROSS) * high, [40; 3]);
        for (number, (level, code)) in chunk.iter().enumerate() {
            let (x, y) = ((number % ACROSS * (CELL.0 + 8) + 4) as i64, (number / ACROSS * high + 2) as i64);
            if let Ok(picture) = game_picture(data, &format!("backdrops/{level}.png")) {
                let by = picture.w.div_ceil(CELL.0).max(picture.h.div_ceil(CELL.1)).max(1);
                sheet.paste(&picture.every(by), x, y);
            }
            sheet.text(x, y + CELL.1 as i64 + 2, &format!("{level} {code}"), [255; 3]);
        }
        let path = out.join(format!("places_{:02}.png", at + 1));
        sheet.save(&path)?;
        written.push(path);
    }
    Ok(written)
}
