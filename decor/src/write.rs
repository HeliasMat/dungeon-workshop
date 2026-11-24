//! A place built, written into data/extra as the game reads its places: the
//! pictures of each frame, the layer walked under, the colours that change
//! with time, its info; its line in decor_backdrops.txt, its block in
//! decor_town.txt (walls, music, who lives there), its line in decors.txt
//! (its name, middle and entrances, for films) and in decor_ground.txt.
//! Those files are decor's alone, read over the game's lists of the same
//! name (`assets::data`, see `data/DATA.md`).

use std::path::{Path, PathBuf};

use crate::fault::Diagnostic;
use crate::lang::{Decor, plain};
use crate::pixels::TILE;
use crate::place::Place;
use crate::source::{Game, Info};

/// The first level a place of ours may take (the game's go to 458).
pub const FIRST_LEVEL: i64 = 459;
/// decor's own lists, read over the game's `backdrops.txt`, `town.txt` and
/// `ground.txt` (`assets::data::DECORS`).
const BACKDROPS: &str = "decor_backdrops.txt";
const TOWN: &str = "decor_town.txt";
const GROUND: &str = "decor_ground.txt";
/// The ways a resident faces, as the game counts them.
const FACES: [(&str, i64); 8] = [("down", 1), ("downright", 2), ("right", 3), ("upright", 4), ("up", 5), ("upleft", 6), ("left", 7), ("downleft", 8)];

fn failed(path: &Path, why: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::said(format!("{} : {why}", path.display()))
}

/// The lines of one of data/extra's lists, as written (with their ends).
fn lines_of(extra: &Path, file: &str) -> Vec<String> {
    std::fs::read_to_string(extra.join(file)).map(|text| text.split_inclusive('\n').map(str::to_string).collect()).unwrap_or_default()
}

fn write_lines(extra: &Path, file: &str, lines: &[String]) -> Result<(), Diagnostic> {
    let path = extra.join(file);
    std::fs::write(&path, lines.concat()).map_err(|why| failed(&path, why))
}

/// The places already made: name, level.
fn registered(extra: &Path) -> Vec<(String, i64)> {
    lines_of(extra, "decors.txt")
        .iter()
        .filter_map(|line| {
            let fields: Vec<&str> = line.trim_end_matches('\n').split('\t').collect();
            Some((fields.first()?.to_string(), fields.get(1)?.parse().ok()?))
        })
        .collect()
}

/// The levels the places and rooms say themselves (`level 459` in their
/// file), each with its name: no other may take them.
fn claimed(decors: &Path) -> Vec<(String, i64)> {
    let files = std::fs::read_dir(decors).into_iter().flatten().flatten();
    let mut claimed = Vec::new();
    for file in files {
        let path = file.path();
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string) else { continue };
        if !path.extension().is_some_and(|kind| kind == "decor" || kind == "room") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let said = text.lines().find_map(|line| line.trim().strip_prefix("niveau ").and_then(|level| level.trim().parse::<i64>().ok()));
        claimed.extend(said.map(|level| (name, level)));
    }
    claimed
}

/// The level a place of that name was made at, if it was (data/extra/decors.txt).
pub fn known_level(extra: &Path, name: &str) -> Option<i64> {
    registered(extra).into_iter().find(|(known, _)| known == name).map(|(_, level)| level)
}

/// Its level: as it says, as it had (unless another says it is its own), or
/// the first free (neither the game's, nor one made, nor one another says).
pub fn level_of(game: &Game, extra: &Path, decor: &Decor, decors: &Path) -> i64 {
    if let Some(level) = decor.level {
        return level;
    }
    let made = registered(extra);
    let claimed = claimed(decors);
    let theirs = |level: i64| claimed.iter().any(|(name, own)| *own == level && *name != decor.name);
    if let Some((_, level)) = made.iter().find(|(name, level)| *name == decor.name && !theirs(*level)) {
        return *level;
    }
    first_free(game, &made, &claimed)
}

/// The first level no place has: neither the game's, nor one made, nor one
/// a place or room of `decors` says is its own (a new place's, `level` in its file).
pub fn free_level(game: &Game, extra: &Path, decors: &Path) -> i64 {
    first_free(game, &registered(extra), &claimed(decors))
}

fn first_free(game: &Game, made: &[(String, i64)], claimed: &[(String, i64)]) -> i64 {
    let taken: Vec<i64> = assets::tables::backdrops(&game.folder)
        .iter()
        .map(|backdrop| backdrop.level as i64)
        .chain(made.iter().map(|(_, level)| *level))
        .chain(claimed.iter().map(|(_, level)| *level))
        .collect();
    let mut level = FIRST_LEVEL;
    while taken.contains(&level) {
        level += 1;
    }
    level
}

/// Where a group stands: the free tile nearest the middle, or as said.
pub fn middle_of(place: &Place) -> (i64, i64) {
    let decor = &place.decor;
    if let Some(middle) = decor.middle {
        return middle;
    }
    let target = (decor.height as f64 / 2.0 / TILE as f64, decor.width as f64 / 2.0 / TILE as f64);
    let mut best: Option<(f64, usize, usize)> = None;
    for row in 0..place.walls.h {
        for column in 0..place.walls.w {
            if place.walls.get(column, row) {
                continue;
            }
            let (dr, dc) = (row as f64 - target.0, column as f64 - target.1);
            let far = dr * dr + dc * dc;
            if best.is_none_or(|(least, ..)| far < least) {
                best = Some((far, row, column));
            }
        }
    }
    match best {
        Some((_, row, column)) => ((column * TILE + TILE / 2) as i64, (row * TILE + TILE / 2) as i64),
        None => (decor.width as i64 / 2, decor.height as i64 / 2),
    }
}

fn save_rgba(path: &Path, w: usize, h: usize, pixels: Vec<u8>) -> Result<(), Diagnostic> {
    saved(path, w, h, &pixels, image::ExtendedColorType::Rgba8)
}

fn save_gray(path: &Path, w: usize, h: usize, pixels: Vec<u8>) -> Result<(), Diagnostic> {
    saved(path, w, h, &pixels, image::ExtendedColorType::L8)
}

/// A picture written as a PNG, quickly: built again at each change of a
/// decor, read at once (a smaller file is not worth the wait).
fn saved(path: &Path, w: usize, h: usize, pixels: &[u8], colour: image::ExtendedColorType) -> Result<(), Diagnostic> {
    use image::ImageEncoder;
    let file = std::fs::File::create(path).map_err(|why| failed(path, why))?;
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        std::io::BufWriter::new(file),
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    );
    encoder.write_image(pixels, w as u32, h as u32, colour).map_err(|why| failed(path, why))
}

/// Its pictures; what is seen of frame 0, RGB.
fn write_pictures(game: &Game, place: &Place, level: i64, extra: &Path) -> Result<(), Diagnostic> {
    let decor = &place.decor;
    let frames = place.lower.len();
    let has_upper = place.upper.iter().any(|frame| frame.cells.iter().any(|rgba| rgba[3] > 0));
    let moving = !place.cycles.is_empty();
    let folder = extra.join("grounds").join("pictures");
    std::fs::create_dir_all(&folder).map_err(|why| failed(&folder, why))?;
    std::fs::create_dir_all(extra.join("grounds").join("info")).map_err(|why| failed(extra, why))?;
    std::fs::create_dir_all(extra.join("backdrops")).map_err(|why| failed(extra, why))?;
    let mine = format!("{level}_");
    for entry in std::fs::read_dir(&folder).map_err(|why| failed(&folder, why))?.flatten() {
        if entry.file_name().to_string_lossy().starts_with(&mine) {
            std::fs::remove_file(entry.path()).map_err(|why| failed(&entry.path(), why))?;
        }
    }
    let (w, h) = (decor.width, decor.height);
    let flat = |frame: &crate::pixels::Picture| -> Vec<u8> { frame.cells.iter().flat_map(|rgba| *rgba).collect() };
    // (Each frame by itself, all at once: a large decor has a score of pictures.)
    let written: Vec<Result<(), Diagnostic>> = std::thread::scope(|scope| {
        let flat = &flat;
        let folder = &folder;
        let each: Vec<_> = (0..frames)
            .map(|frame| {
                scope.spawn(move || -> Result<(), Diagnostic> {
                    save_rgba(&folder.join(format!("{level}_f{frame}.png")), w, h, flat(&place.lower[frame]))?;
                    if has_upper {
                        save_rgba(&folder.join(format!("{level}_u{frame}.png")), w, h, flat(&place.upper[frame]))?;
                    }
                    if moving {
                        save_gray(&folder.join(format!("{level}_i{frame}.png")), w, h, place.lower_index[frame].cells.clone())?;
                        if has_upper {
                            save_gray(&folder.join(format!("{level}_v{frame}.png")), w, h, place.upper_index[frame].cells.clone())?;
                        }
                    }
                    Ok(())
                })
            })
            .collect();
        each.into_iter().map(|one| one.join().unwrap_or_else(|_| Err(Diagnostic::said("an image could not be written")))).collect()
    });
    written.into_iter().collect::<Result<Vec<()>, Diagnostic>>()?;
    let count = frames as u64;
    let counts = [count, if has_upper { count } else { 0 }, if moving { count } else { 0 }, if moving && has_upper { count } else { 0 }, 0];
    let first = place.levels.iter().next().copied().unwrap_or(323);
    let palettes = game.source(first).map(|source| source.info.palettes.clone())?;
    let info = Info { width: w as u64, height: h as u64, seconds: place.seconds as f32, counts, palettes, cycles: place.cycles.clone() };
    let path = extra.join("grounds").join("info").join(format!("{level}.bin"));
    std::fs::write(&path, info.bytes()).map_err(|why| failed(&path, why))?;
    let mut whole = Vec::with_capacity(w * h * 3);
    for at in 0..w * h {
        let (below, above) = (place.lower[0].cells[at], place.upper[0].cells[at]);
        let seen = if above[3] > 0 { above } else { below };
        whole.extend_from_slice(&seen[..3]);
    }
    let path = extra.join("backdrops").join(format!("{level}.png"));
    saved(&path, w, h, &whole, image::ExtendedColorType::Rgb8)
}

/// A Pokémon's number, by its name as films write it.
fn species_of(game: &Game, name: &str, number: usize) -> Result<i64, Diagnostic> {
    for row in game.rows("monsters.txt") {
        if row.len() > 1
            && let Ok(species) = row[0].parse::<i64>()
            && plain(&row[1]) == name
        {
            return Ok(species);
        }
    }
    Err(Diagnostic::said(format!("line {number}: unknown Pokémon \"{name}\"")))
}

/// A line said, cut for the game's box: 34 letters at most a line.
fn said(text: &str) -> String {
    let (mut lines, mut now) = (Vec::new(), String::new());
    for word in text.split_whitespace() {
        if !now.is_empty() && now.chars().count() + 1 + word.chars().count() > 34 {
            lines.push(std::mem::take(&mut now));
            now = word.to_string();
        } else {
            now = format!("{now} {word}").trim().to_string();
        }
    }
    lines.push(now);
    lines.join("\\n")
}

/// The lines of town.txt for who lives in a place, what each says, its ways
/// out and in: as the game's own places have them.
fn lived_in(game: &Game, decor: &Decor) -> Result<Vec<String>, Diagnostic> {
    let mut out = Vec::new();
    for (x, y, wide, high, to) in &decor.exits {
        out.push(format!("area\t{x}\t{y}\t{wide}\t{high}\t0\n"));
        out.push(format!("ground\t{to}\n"));
    }
    for (came, x, y, wide, high) in &decor.links {
        out.push(format!("link\t{came}\t{x}\t{y}\t{wide}\t{high}\n"));
    }
    for one in &decor.residents {
        let code = format!("m{}", species_of(game, &one.name, one.number)?);
        let face = FACES.iter().find(|(name, _)| *name == one.facing).map_or(1, |(_, face)| *face);
        let animation = one.animation.as_deref().unwrap_or("-");
        out.push(format!("actor\t{code}\t{}\t{}\t{face}\t0\t{}\t{animation}\t{}\n", one.x, one.y, one.wanders, u8::from(one.still)));
        for text in &one.lines {
            out.push(format!("say\t{code}\t0\t {}\n", said(text)));
        }
    }
    Ok(out)
}

/// A place's name as shown: as it says, or its file's, its first letter big.
fn shown_name(decor: &Decor) -> String {
    decor.shown.clone().unwrap_or_else(|| {
        let spaced = decor.name.replace('_', " ");
        let mut letters = spaced.chars();
        letters.next().map(|first| first.to_uppercase().chain(letters.flat_map(char::to_lowercase)).collect()).unwrap_or_default()
    })
}

/// The place built, written: its pictures, its lines in the lists.
pub fn write(game: &Game, place: &Place, level: i64, extra: &Path) -> Result<(), Diagnostic> {
    std::fs::create_dir_all(extra).map_err(|why| failed(extra, why))?;
    write_pictures(game, place, level, extra)?;
    let decor = &place.decor;
    let mine = format!("{level}\t");
    let mut backdrops: Vec<String> = lines_of(extra, BACKDROPS).into_iter().filter(|line| !line.starts_with(&mine)).collect();
    backdrops.push(format!("{level}\t{}\t{}\t{}\n", decor.name.to_uppercase(), decor.width, decor.height));
    write_lines(extra, BACKDROPS, &backdrops)?;
    let (mut blocks, mut keep) = (Vec::new(), true);
    for line in lines_of(extra, TOWN) {
        if line.starts_with("place\t") {
            keep = line.split('\t').nth(1) != Some(level.to_string().as_str());
        }
        if keep {
            blocks.push(line);
        }
    }
    let rows: Vec<String> =
        (0..place.walls.h).map(|row| (0..place.walls.w).map(|column| if place.walls.get(column, row) { '1' } else { '0' }).collect()).collect();
    let shown = shown_name(decor);
    blocks.push(format!("place\t{level}\t{shown}\t{}\t{}\n", rows[0].len(), rows.len()));
    if let Some(music) = decor.music {
        blocks.push(format!("music\t{music}\n"));
    }
    if decor.departure {
        blocks.push("departure\n".to_string());
    }
    blocks.extend(rows.iter().map(|row| format!("solid\t{row}\n")));
    blocks.extend(lived_in(game, decor)?);
    write_lines(extra, TOWN, &blocks)?;
    let (x, y) = middle_of(place);
    // (Its entrances, `x,y,name;...`, after its name: films may read them.)
    let doors: Vec<String> = decor.entrances.iter().map(|(_, ex, ey, name)| format!("{ex},{ey},{name}")).collect();
    let mut decors: Vec<String> = lines_of(extra, "decors.txt").into_iter().filter(|line| line.split('\t').next() != Some(decor.name.as_str())).collect();
    let doors = if doors.is_empty() { String::new() } else { format!("\t{}", doors.join(";")) };
    decors.push(format!("{}\t{level}\t{x}\t{y}\t{shown}{doors}\n", decor.name));
    write_lines(extra, "decors.txt", &decors)?;
    // (Its walls in the ground too, as the game's grid: the other places' left as they are.)
    let mut grounds: Vec<String> = lines_of(extra, GROUND).into_iter().filter(|line| !line.starts_with(&mine)).collect();
    grounds.push(format!("{level}\tjeu\t{}\n", rows.join("/")));
    grounds.sort_by_key(|line| line.split('\t').next().and_then(|level| level.parse::<i64>().ok()).unwrap_or(i64::MAX));
    write_lines(extra, GROUND, &grounds)
}

/// Where data/extra is, for the game's files in that folder.
pub fn extra_of(folder: &str) -> PathBuf {
    Path::new(folder).join("extra")
}
