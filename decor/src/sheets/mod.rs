//! The sheets, in target/decors/: a place, its walls, entrances and what it
//! remarks, beside the game's places it comes from; a grid to read places
//! off; all places together (here). The catalogue and pieces looked at
//! closely: `pieces`.

pub mod canvas;
pub mod pieces;

use std::path::PathBuf;

use crate::fault::Diagnostic;
use crate::pixels::{Picture, TILE};
use crate::place::Place;
use crate::source::picture_at;
use canvas::{Canvas, LETTER_HIGH, Rgb, text_wide};

const WALL: Rgb = [255, 0, 0];
const SEAM: Rgb = [0, 255, 255];
const JOIN: Rgb = [255, 0, 255];
const ENTRANCE: Rgb = [255, 255, 0];
/// Above a picture on a sheet: its title's room.
const TITLED: usize = LETTER_HIGH + 4;

/// Where a sheet goes: `<root>/target/decors/<name>`.
pub fn sheet_path(root: &std::path::Path, name: &str) -> Result<PathBuf, Diagnostic> {
    let folder = root.join("target").join("decors");
    std::fs::create_dir_all(&folder).map_err(|why| Diagnostic::said(format!("{}: {why}", folder.display())))?;
    Ok(folder.join(name))
}

/// What is seen of a place's first frame: the layer walked under over the other.
pub fn whole(place: &Place) -> Picture {
    let mut out = place.lower[0].clone();
    for (cell, over) in out.cells.iter_mut().zip(&place.upper[0].cells) {
        if over[3] > 0 {
            *cell = *over;
        }
    }
    out
}

/// A picture of the game's files (a place of the game: `backdrops/<n>.png`).
pub fn game_picture(data: &str, file: &str) -> Result<Canvas, Diagnostic> {
    picture_at(&assets::data::path(data, file)).map(|picture| Canvas::of(&picture)).ok_or_else(|| Diagnostic::said(format!("{file}: no image")))
}

/// The place with its walls (red lines), woven seams (cyan), joins that
/// show (magenta), entrances (a yellow cross and their name).
fn marked(picture: &Canvas, place: &Place) -> Canvas {
    let mut out = picture.clone();
    for row in 0..place.walls.h {
        for column in (0..place.walls.w).filter(|column| place.walls.get(*column, row)) {
            for y in (row * TILE..(row + 1) * TILE).step_by(3) {
                out.fill(((column * TILE) as i64, y as i64, ((column + 1) * TILE) as i64 - 1, y as i64), WALL);
            }
        }
    }
    for (_, x, y, _) in &place.seams {
        out.fill((*x as i64, *y as i64, *x as i64 + 1, (*y + TILE) as i64 - 1), SEAM);
    }
    for (cell, joined) in out.cells.iter_mut().zip(&place.join_mask.cells) {
        if *joined {
            *cell = JOIN;
        }
    }
    for (x0, y0, x1, y1, _) in &place.joins {
        out.frame((*x0 as i64 - 2, *y0 as i64 - 2, *x1 as i64 + 1, *y1 as i64 + 1), JOIN);
    }
    for (_, x, y, name) in &place.decor.entrances {
        out.fill((x - 5, y - 1, x + 5, *y), ENTRANCE);
        out.fill((x - 1, y - 5, *x, y + 5), ENTRANCE);
        out.text((x + 6).min(place.decor.width as i64 - text_wide(name) as i64), (y - 12).max(0), name, ENTRANCE);
    }
    out
}

/// Pictures side by side, each with its title above.
fn beside(pictures: &[(Canvas, String)]) -> Canvas {
    let wide = pictures.iter().map(|(picture, _)| picture.w + 8).sum::<usize>() - 8;
    let high = pictures.iter().map(|(picture, _)| picture.h).max().unwrap_or(0) + TITLED;
    let mut side = Canvas::new(wide, high, [255; 3]);
    let mut x = 0;
    for (picture, title) in pictures {
        side.paste(picture, x, TITLED as i64);
        side.text(x + 2, 2, title, [0; 3]);
        x += picture.w as i64 + 8;
    }
    side
}

/// The sheets of a place: itself, marked, beside the game's places it comes
/// from. Their paths.
pub fn look(root: &std::path::Path, data: &str, place: &Place) -> Result<Vec<PathBuf>, Diagnostic> {
    let name = &place.decor.name;
    let paths = [
        sheet_path(root, &format!("{name}.png"))?,
        sheet_path(root, &format!("{name}_walls.png"))?,
        sheet_path(root, &format!("{name}_side_by_side.png"))?,
        grid(root, place)?,
    ];
    let picture = Canvas::of(&whole(place));
    picture.save(&paths[0])?;
    marked(&picture, place).save(&paths[1])?;
    let compared: Vec<i64> = if place.decor.compared.is_empty() { place.levels.iter().take(2).copied().collect() } else { place.decor.compared.clone() };
    let mut pictures = vec![(picture, name.clone())];
    for other in compared {
        pictures.push((game_picture(data, &format!("backdrops/{other}.png"))?, format!("jeu {other}")));
    }
    beside(&pictures).save(&paths[2])?;
    Ok(paths.to_vec())
}

/// A place twice as large, a line at each step of its grid (`grid`, else
/// 16), a strong one every four with its coordinates; what it lays named at
/// its corner, its entrances and spots marked: to read its places off.
pub fn grid(root: &std::path::Path, place: &Place) -> Result<PathBuf, Diagnostic> {
    const SCALE: usize = 2;
    let (w, h) = (place.decor.width, place.decor.height);
    let step = place.decor.grid.unwrap_or(16).max(4);
    let strong = step * 4;
    let mut big = Canvas::of(&whole(place)).scaled(SCALE);
    let line = |at: usize| if at.is_multiple_of(strong) { [255, 255, 0] } else { [90, 90, 90] };
    for x in (0..w).step_by(step) {
        big.fill(((x * SCALE) as i64, 0, (x * SCALE) as i64, (h * SCALE) as i64), line(x));
    }
    for y in (0..h).step_by(step) {
        big.fill((0, (y * SCALE) as i64, (w * SCALE) as i64, (y * SCALE) as i64), line(y));
    }
    for x in (0..w).step_by(strong) {
        for y in (0..h).step_by(strong) {
            big.text((x * SCALE + 2) as i64, (y * SCALE + 2) as i64, &format!("{x},{y}"), [255; 3]);
        }
    }
    // (What is laid, by its name at its corner; the entrances and spots, a dot and their name.)
    for line in place.decor.lines.iter().filter(|line| matches!(line.verb.as_str(), "put" | "water" | "image")) {
        let at = crate::fault::Location::in_file(place.decor.file.clone(), line.number);
        for (x, y) in crate::lang::points_of(&at, &line.words[1..]).unwrap_or_default() {
            let (x, y) = (x * SCALE as i64, y * SCALE as i64);
            big.frame((x, y, x + 3, y + 3), [90, 210, 230]);
            big.text(x + 2, y + 5, &line.words[0], [90, 210, 230]);
        }
    }
    for (_, x, y, name) in &place.decor.entrances {
        let (x, y) = (x * SCALE as i64, y * SCALE as i64);
        big.fill((x - 2, y - 2, x + 2, y + 2), [120, 220, 120]);
        big.text(x + 4, y - 3, name, [150, 230, 150]);
    }
    let path = sheet_path(root, &format!("{}_grid.png", place.decor.name))?;
    big.save(&path)?;
    Ok(path)
}

/// Places two by two, each with its name: `target/decors/<out>.png`.
pub fn ensemble(root: &std::path::Path, places: &[(String, Picture)], out: &str) -> Result<PathBuf, Diagnostic> {
    const COLUMNS: usize = 2;
    let cells: Vec<Canvas> = places.iter().map(|(_, picture)| Canvas::of(picture)).collect();
    let wide = cells.iter().map(|cell| cell.w).max().unwrap_or(0);
    let heights: Vec<usize> = cells.chunks(COLUMNS).map(|row| row.iter().map(|cell| cell.h).max().unwrap_or(0) + TITLED + 4).collect();
    let mut sheet = Canvas::new(COLUMNS * (wide + 8), heights.iter().sum::<usize>().max(1), [28; 3]);
    let mut y = 0;
    for (row, high) in heights.iter().enumerate() {
        for column in 0..COLUMNS {
            let Some(cell) = cells.get(row * COLUMNS + column) else {
                continue;
            };
            let x = (column * (wide + 8)) as i64;
            sheet.text(x + 2, y + 3, &places[row * COLUMNS + column].0, [230; 3]);
            sheet.paste(cell, x, y + TITLED as i64 + 2);
        }
        y += *high as i64;
    }
    let path = sheet_path(root, &format!("{out}.png"))?;
    sheet.save(&path)?;
    Ok(path)
}

/// A place's terrains, as `terrain` paints them: each its letter, colour,
/// share and tile; the place made again of its own corners beside it as it
/// is (`terrains_<level>.png`). What is said of them.
pub fn terrains(workshop: &crate::workshop::Workshop, level: i64, letters: &str) -> Result<String, Diagnostic> {
    use crate::terrain::{NONE, of_the_place};
    let set = workshop.tileset(level)?;
    let source = workshop.source(level)?;
    let (columns, rows) = (set.corners.w - 1, set.corners.h - 1);
    let painted = of_the_place(&set, 0, 0, (0, 0, columns, rows));
    let made = crate::terrain::cut_of(&set, &painted);
    let first = &source.frames[0];
    let (w, h) = (first.w, first.h);
    let mut same = 0;
    let mut laid = 0;
    for row in 0..rows {
        for column in 0..columns {
            let tile = |picture: &crate::pixels::Picture| picture.crop(column * 8, row * 8, column * 8 + 8, row * 8 + 8).cells;
            let was = tile(first);
            if was.iter().all(|rgba| rgba[3] == 0) {
                continue;
            }
            laid += 1;
            same += usize::from(tile(&made.lower[0]) == was);
        }
    }
    let swatch = 24;
    let key_high = set.terrains.len() * (swatch + 4) + 8;
    let mut sheet = Canvas::new(w * 2 + 24, (h + 20).max(key_high + 20), [24, 24, 24]);
    sheet.text(4, 4, &format!("place {level}"), [230, 230, 230]);
    sheet.text(w as i64 + 16, 4, "made again from its corners", [230, 230, 230]);
    sheet.paste(&Canvas::of(first), 4, 18);
    sheet.paste(&Canvas::of(&made.lower[0]), w as i64 + 16, 18);
    let mut out = vec![format!(
        "place {level}: {} tiles, {} terrains; made again from its corners, {same} tiles of {laid} identical ({:.0} %)",
        set.tiles.len(),
        set.terrains.len(),
        100.0 * same as f64 / laid.max(1) as f64
    )];
    for terrain in &set.terrains {
        let solid = terrain.wall;
        out.push(format!(
            "  {}  #{:02X}{:02X}{:02X}  {:5.1} %{}",
            terrain.letter,
            terrain.colour[0],
            terrain.colour[1],
            terrain.colour[2],
            terrain.share * 100.0,
            if solid { "  (wall)" } else { "" }
        ));
    }
    let _ = NONE;
    // (Those letters asked: each set of corners of them, its tiles, the most seen first.)
    if !letters.is_empty() {
        let wanted: Vec<u8> = letters.chars().filter_map(crate::terrain::terrain_of).collect();
        let mut keys: Vec<(&[u8; 4], &Vec<u32>)> = set.keys().filter(|(key, _)| key.iter().all(|corner| wanted.contains(corner))).collect();
        keys.sort();
        let widest = keys.iter().map(|(_, ids)| ids.len().min(24)).max().unwrap_or(1);
        let mut board = Canvas::new(80 + widest * 28, keys.len() * 38 + 8, [24, 24, 24]);
        for (row, (key, ids)) in keys.iter().enumerate() {
            let y = 4 + row as i64 * 38;
            let said: String = key.iter().map(|corner| crate::terrain::letter(*corner)).collect();
            board.text(4, y + 8, &said, [230, 230, 230]);
            let mut ids: Vec<u32> = (*ids).clone();
            ids.sort_by_key(|id| std::cmp::Reverse(set.tiles[*id as usize].count));
            for (at, id) in ids.iter().take(24).enumerate() {
                let tile = &set.tiles[*id as usize];
                let big = Canvas::of(&tile.frames[0]);
                let x = 60 + at as i64 * 28;
                for py in 0..24 {
                    for px in 0..24 {
                        board.dot(x + px, y + py, big.get(px as usize / 3, py as usize / 3));
                    }
                }
                board.text(x, y + 24, &tile.count.min(99).to_string(), [160, 160, 160]);
            }
            out.push(format!("  {said}: {} tiles ({} seen)", ids.len(), ids.iter().map(|id| set.tiles[*id as usize].count).sum::<usize>()));
        }
        let path = workshop.project.root().join("target/decors").join(format!("terrains_{level}_{letters}.png"));
        board.save(&path)?;
        out.push(path.display().to_string());
    }
    if let Some(walls) = &source.walls {
        out.push(format!("walls {}x{} ({} full), tiles {columns}x{rows}", walls.w, walls.h, walls.count()));
    }
    let path = workshop.project.root().join("target/decors").join(format!("terrains_{level}.png"));
    std::fs::create_dir_all(path.parent().unwrap_or(&path)).map_err(|why| Diagnostic::said(why.to_string()))?;
    sheet.save(&path)?;
    out.push(path.display().to_string());
    Ok(out.join("\n"))
}
