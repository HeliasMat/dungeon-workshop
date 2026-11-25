//! A dungeon room for films, from a layout in letters: `library/decors/<name>.room`,
//! laid with the tileset of a floor of the game's (`scene_player dungeons` tells
//! which, in data/extra/dungeon_floors.txt), each tile chosen as the game chooses it,
//! from its ground and the neighbours like it. Written as the decors are
//! (data/extra/, never committed), and played as they are: `scene: <name>`.
//!
//! The layout, one letter a tile of 24 pixels:
//!
//! ```text
//! dungeon: cave_room Beach Cave 3     # name, dungeon (its name or its number), floor
//! layout
//! ##########
//! #...a....#      # wall, . ground, ~ water, lava (the floor's second ground),
//! #..~~~...#      a small letter: a spot of the ground with that name
//! ##########      (for the films: `put pikachu a`), on ground
//! ```
//!
//! - `generate`: a whole floor's layout, made as the game makes one.

pub mod generate;

use std::path::{Path, PathBuf};

use crate::fault::{Diagnostic, Location};
use crate::lang::{Decor, closest, plain};
use crate::pixels::{Grid, Mask, Picture, TILE};
use crate::place::Place;
use crate::source::{Cycle, Game};

/// A tile of a dungeon, in pixels; the chunks in a row of a tileset's atlas.
pub const CHUNK: usize = 24;
const IN_ROW: usize = 32;
/// The game's: the neighbours of a tile, as bits: S, SE, E, NE, N, NW, W, SW.
const NEIGHBOURS: [(i64, i64); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];
const WALL: usize = 0;
const SECOND: usize = 1;
const GROUND: usize = 2;

/// The rooms of the project (beside its places), by name.
pub fn names(project: &assets::project::Project) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(project.decors())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(".room").map(str::to_string))
        .collect();
    names.sort();
    names
}

/// Each floor of each dungeon, in the order written: id, name, floor, tileset.
fn floors(data: &str) -> Result<Vec<(i64, String, i64, i64)>, Diagnostic> {
    let path = Path::new(data).join("extra").join("dungeon_floors.txt");
    let text = std::fs::read_to_string(&path).map_err(|_| Diagnostic::said("data/extra/dungeon_floors.txt is missing: scene_player dungeons".to_string()))?;
    let mut out: Vec<(i64, String, i64, i64)> = Vec::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let row: Vec<&str> = line.split('\t').collect();
        if row.len() < 4 {
            continue;
        }
        let number = |field: &str| field.trim().parse::<i64>().map_err(|_| Diagnostic::said(format!("{}: \"{line}\"", path.display())));
        let (id, floor, tileset) = (number(row[0])?, number(row[2])?, number(row[3])?);
        // (The same floor twice: the last said.)
        match out.iter_mut().find(|(known, name, at, _)| *known == id && name == row[1] && *at == floor) {
            Some(known) => known.3 = tileset,
            None => out.push((id, row[1].to_string(), floor, tileset)),
        }
    }
    Ok(out)
}

/// The tileset of a dungeon's floor (by its name or number), and the dungeon's name.
fn tileset_of(data: &str, dungeon: &str, floor: i64, at: &Location) -> Result<(i64, String), Diagnostic> {
    let known = floors(data)?;
    if let Some((_, name, _, tileset)) =
        known.iter().find(|(number, name, at, _)| *at == floor && (number.to_string() == dungeon || plain(name) == plain(dungeon)))
    {
        return Ok((*tileset, name.clone()));
    }
    let mut names: Vec<&str> = known.iter().map(|(_, name, ..)| name.as_str()).collect();
    names.sort();
    names.dedup();
    Err(at.wrong(format!("no floor {floor} in \"{dungeon}\".{} (decor dungeons)", closest(dungeon, names))))
}

/// The dungeons whose name has that word, and their floors: one line each.
pub fn list(word: &str, data: &str) -> Result<Vec<String>, Diagnostic> {
    let word = plain(word);
    let mut shown: Vec<((i64, String), i64)> = Vec::new();
    for (number, name, floor, _) in floors(data)? {
        if !plain(&name).contains(&word) {
            continue;
        }
        match shown.iter_mut().find(|(known, _)| *known == (number, name.clone())) {
            Some((_, most)) => *most = (*most).max(floor),
            None => shown.push(((number, name), floor)),
        }
    }
    shown.sort();
    Ok(shown.into_iter().map(|((number, name), most)| format!("{number:4} {name}: floors 1 to {most}")).collect())
}

/// A tileset: its chunks (an atlas), what chooses one (by ground, the
/// neighbours' bits and a variation, or the chunk of each place of a whole
/// picture, a boss's floor, 32 by 32); which palette and colour each pixel
/// shows, and the palettes that change with time (the water, the lava), if
/// the extract wrote them.
struct Tileset {
    atlas: image::RgbaImage,
    chunk_for: Vec<u16>,
    colours: Option<image::GrayImage>,
    cycles: Vec<Cycle>,
}

fn read_tileset(data: &str, number: i64) -> Result<Tileset, Diagnostic> {
    // (Wherever the game's data has them: `rom/`, or the folder itself.)
    let file = |name: String| std::path::PathBuf::from(assets::data::path(data, &format!("tilesets/{name}")));
    let failed = |path: &Path, why: &dyn std::fmt::Display| Diagnostic::said(format!("{}: {why}", path.display()));
    let path = file(format!("{number}.bin"));
    let bytes = std::fs::read(&path).map_err(|why| failed(&path, &why))?;
    let length = bytes.get(8..16).map(|field| u64::from_le_bytes(field.try_into().expect("8")) as usize).ok_or_else(|| failed(&path, &"too short"))?;
    let chunk_for =
        bytes.get(16..16 + 2 * length).ok_or_else(|| failed(&path, &"too short"))?.as_chunks::<2>().0.iter().map(|two| u16::from_le_bytes(*two)).collect();
    let path = file(format!("{number}.png"));
    let atlas = image::open(&path).map_err(|why| failed(&path, &why))?.to_rgba8();
    let path = file(format!("{number}_i.png"));
    let colours = if path.exists() { Some(image::open(&path).map_err(|why| failed(&path, &why))?.to_luma8()) } else { None };
    let path = file(format!("{number}.cycles"));
    let cycles = if path.exists() {
        read_cycles(&std::fs::read(&path).map_err(|why| failed(&path, &why))?).ok_or_else(|| failed(&path, &"unreadable"))?
    } else {
        Vec::new()
    };
    Ok(Tileset { atlas, chunk_for, colours, cycles })
}

/// `<id>.cycles` (bincode: palette, ticks, its colours at each step) as the
/// places' info has them.
fn read_cycles(data: &[u8]) -> Option<Vec<Cycle>> {
    let u64_at = |at: usize| data.get(at..at + 8).map(|field| u64::from_le_bytes(field.try_into().expect("8")));
    let mut cycles: Vec<Cycle> = Vec::new();
    let mut at = 8;
    for _ in 0..u64_at(0)? {
        let palette = u64_at(at)?;
        let ticks = u16::from_le_bytes(data.get(at + 8..at + 10)?.try_into().ok()?);
        let steps = u64_at(at + 10)? as usize;
        at += 18;
        let colours = (0..steps).map(|step| data.get(at + 48 * step..at + 48 * (step + 1)).map(<[u8]>::to_vec)).collect::<Option<Vec<_>>>()?;
        at += 48 * steps;
        cycles.retain(|(known, _)| *known != palette);
        cycles.push((palette, (ticks, colours)));
    }
    Some(cycles)
}

/// A room's file read: its name, dungeon, floor and that line; the name it
/// shows; its plan, each row with its line, as wide as the widest.
struct Plan {
    path: String,
    name: String,
    dungeon: String,
    floor: i64,
    line: usize,
    shown: Option<String>,
    rows: Vec<(usize, Vec<char>)>,
}

fn read(project: &assets::project::Project, name: &str) -> Result<Plan, Diagnostic> {
    let path = project.decors().join(format!("{name}.room"));
    let shown_path = path.display().to_string();
    let text = std::fs::read_to_string(&path).map_err(|_| Diagnostic::said(format!("no room {name}: {shown_path}")))?;
    plan_of(&text, &shown_path)
}

/// A room's layout from its text; its faults said on the lines of `file`.
fn plan_of(text: &str, file: &str) -> Result<Plan, Diagnostic> {
    let shown_path = file.to_string();
    let (mut head, mut shown, mut rows, mut planned) = (None, None, Vec::new(), false);
    for (number, line) in text.lines().enumerate().map(|(at, line)| (at + 1, line)) {
        if planned {
            if !line.trim().is_empty() {
                rows.push((number, line.trim_end().chars().collect::<Vec<char>>()));
            }
            continue;
        }
        let bare = line.split("  #").next().unwrap_or_default().trim_end();
        let bare = if bare.starts_with('#') { "" } else { bare.split('#').next().unwrap_or_default().trim_end() };
        if bare.is_empty() {
            continue;
        }
        if let Some(rest) = bare.strip_prefix("dungeon:") {
            let words: Vec<&str> = rest.split_whitespace().collect();
            match words.as_slice() {
                [word, dungeon @ .., floor] if !dungeon.is_empty() && !floor.is_empty() && floor.chars().all(|digit| digit.is_ascii_digit()) => {
                    head = Some((word.to_string(), dungeon.join(" "), floor.parse::<i64>().unwrap_or_default(), number));
                }
                _ => return Err(Location::in_file(shown_path, number).wrong("dungeon: <name> <dungeon> <floor>")),
            }
        } else if let Some(rest) = bare.strip_prefix("name ") {
            shown = Some(rest.trim().to_string());
        } else if bare == "layout" {
            planned = true;
        } else {
            return Err(Location::in_file(shown_path, number).wrong(format!("\"{bare}\": dungeon:, name, layout")));
        }
    }
    let Some((word, dungeon, floor, line)) = head.filter(|_| !rows.is_empty()) else {
        return Err(Location::in_file(shown_path, 0).wrong("a dungeon: line and a layout are needed"));
    };
    squared(&mut rows);
    Ok(Plan { path: shown_path, name: word, dungeon, floor, line, shown, rows })
}

/// A room built: where it is, what it is laid with, and the place written.
pub struct Room {
    pub place: Place,
    pub level: i64,
    pub tileset: i64,
}

impl Room {
    /// What is seen of it: its picture.
    pub fn picture(&self) -> &Picture {
        &self.place.lower[0]
    }
}

/// A room built and written into `extra`: what `data`'s game says of its floor.
pub fn build(project: &assets::project::Project, name: &str, data: &str, extra: &Path) -> Result<Room, Diagnostic> {
    built(project, name, read(project, name)?, data, extra)
}

/// A room's floor laid from its layout: its picture, which colour each pixel
/// shows, the palettes that change with time, its walls by tiles of 8, its
/// spots (line, x, y, letter), the name it shows if it says none, its tileset.
pub struct Laid {
    pub picture: Picture,
    pub indices: Grid<u8>,
    pub cycles: Vec<Cycle>,
    pub walls: Mask,
    pub spots: Vec<(usize, i64, i64, String)>,
    pub shown: String,
    pub tileset: i64,
}

impl Laid {
    /// Laid under all of a place, from its top left corner: its picture in
    /// every frame, its colours that change with time as they are, its walls.
    pub(crate) fn lay_on(&self, place: &mut Place) {
        let (w, h) = (self.picture.w.min(place.decor.width), self.picture.h.min(place.decor.height));
        for frame in 0..place.lower.len() {
            for y in 0..h {
                for x in 0..w {
                    place.lower[frame].set(x, y, self.picture.get(x, y));
                    place.lower_index[frame].set(x, y, self.indices.get(x, y));
                }
            }
        }
        // (Its palettes keep their numbers: what each pixel shows is the tileset's.)
        for (palette, cycle) in &self.cycles {
            place.slots.push(((-1000 - self.tileset, *palette), *palette));
            place.cycles.push((*palette, cycle.clone()));
        }
        for row in 0..self.walls.h.min(place.walls.h) {
            for column in 0..self.walls.w.min(place.walls.w) {
                place.walls.set(column, row, self.walls.get(column, row));
            }
        }
    }
}

/// A layout's rows as wide as the widest (a wall where one is short).
pub fn squared(rows: &mut [(usize, Vec<char>)]) {
    let width = rows.iter().map(|(_, row)| row.len()).max().unwrap_or(0);
    for (_, row) in rows {
        row.resize(width, '#');
    }
}

/// A dungeon's floor laid from a layout in letters (`base dungeon` in a
/// decor, or a room's `.room`), its faults on the lines of `file`.
pub fn laid(data: &str, dungeon: &str, floor: i64, rows: &[(usize, Vec<char>)], file: &str, line: usize) -> Result<Laid, Diagnostic> {
    if rows.is_empty() {
        return Err(Location::in_file(file, line)
            .wrong("a dungeon base wants its layout: \"layout\" then its rows (# wall, . ground, ~ water or lava, a small letter: a spot)"));
    }
    let (number, dungeon_name) = tileset_of(data, dungeon, floor, &Location::in_file(file, line))?;
    let tileset = read_tileset(data, number)?;
    let plan = Plan { path: file.to_string(), name: String::new(), dungeon: dungeon.to_string(), floor, line, shown: None, rows: rows.to_vec() };
    let (rows, columns) = (plan.rows.len(), plan.rows[0].1.len());
    let mut ground = Grid::filled(columns, rows, WALL);
    let mut spots = Vec::new();
    for (y, (at, row)) in plan.rows.iter().enumerate() {
        for (x, letter) in row.iter().enumerate() {
            let here = match letter {
                '#' => WALL,
                '~' => SECOND,
                '.' => GROUND,
                letter if letter.is_lowercase() => {
                    spots.push((*at, (x * CHUNK + CHUNK / 2) as i64, (y * CHUNK + CHUNK / 2 + 4) as i64, letter.to_string()));
                    GROUND
                }
                letter => {
                    return Err(
                        Location::in_file(plan.path.clone(), *at).wrong(format!("\"{letter}\": # wall, . ground, ~ water or lava, a small letter: a spot"))
                    );
                }
            };
            ground.set(x, y, here);
        }
    }
    let terrain = |x: i64, y: i64| if (0..columns as i64).contains(&x) && (0..rows as i64).contains(&y) { ground.get(x as usize, y as usize) } else { WALL };
    let (width, height) = (columns * CHUNK, rows * CHUNK);
    let mut picture: Picture = Grid::new(width, height);
    let mut indices: Grid<u8> = Grid::new(width, height);
    let whole = tileset.chunk_for.len() == 32 * 32;
    for y in 0..rows {
        for x in 0..columns {
            let here = ground.get(x, y);
            let bits: usize =
                NEIGHBOURS.iter().enumerate().filter(|(_, (dx, dy))| terrain(x as i64 + dx, y as i64 + dy) == here).map(|(bit, _)| 1 << bit).sum();
            let index = if whole { (x < 32 && y < 32).then_some(y * 32 + x) } else { Some(here * 768 + bits * 3 + (x * 7 + y * 13) % 3) };
            let Some(at) = index.and_then(|index| tileset.chunk_for.get(index)).map(|at| *at as usize) else {
                continue;
            };
            let (from_x, from_y) = (at % IN_ROW * CHUNK, at / IN_ROW * CHUNK);
            if from_x + CHUNK > tileset.atlas.width() as usize || from_y + CHUNK > tileset.atlas.height() as usize {
                return Err(Diagnostic::said(format!("tileset {number}: no chunk {at}")));
            }
            for dy in 0..CHUNK {
                for dx in 0..CHUNK {
                    let (px, py) = ((from_x + dx) as u32, (from_y + dy) as u32);
                    picture.set(x * CHUNK + dx, y * CHUNK + dy, tileset.atlas.get_pixel(px, py).0);
                    if let Some(colours) = &tileset.colours {
                        indices.set(x * CHUNK + dx, y * CHUNK + dy, colours.get_pixel(px, py).0[0]);
                    }
                }
            }
        }
    }
    picture.cells.iter_mut().for_each(|rgba| rgba[3] = 255);
    // (Its walls, tile by tile of 8 pixels; the water and the lava move as
    // the game's: their palettes go round.)
    let mut walls = Mask::new(width / TILE, height / TILE);
    for (at, solid) in walls.cells.iter_mut().enumerate() {
        let (column, row) = (at % (width / TILE), at / (width / TILE));
        *solid = ground.get(column * TILE / CHUNK, row * TILE / CHUNK) != GROUND;
    }
    let indices = if tileset.cycles.is_empty() { Grid::new(width, height) } else { indices };
    Ok(Laid { picture, indices, cycles: tileset.cycles, walls, spots, shown: format!("{dungeon_name} {}F", plan.floor), tileset: number })
}

fn built(project: &assets::project::Project, name: &str, plan: Plan, data: &str, extra: &Path) -> Result<Room, Diagnostic> {
    if plan.name != name {
        return Err(Location::in_file(plan.path.clone(), plan.line).wrong(format!("the room is called {name}, not {}", plan.name)));
    }
    let made = laid(data, &plan.dungeon, plan.floor, &plan.rows, &plan.path, plan.line)?;
    let (width, height) = (made.picture.w, made.picture.h);
    let room = Decor {
        name: name.to_string(),
        width,
        height,
        shown: Some(plan.shown.unwrap_or(made.shown)),
        entrances: made.spots,
        file: plan.path,
        ..Decor::default()
    };
    let place = Place::room(room, made.picture, made.indices, made.cycles, made.walls);
    let game = Game::new(data);
    let level = crate::write::level_of(&game, extra, &place.decor, &project.decors());
    crate::write::write(&game, &place, level, extra)?;
    Ok(Room { place, level, tileset: made.tileset })
}

/// A room's text (`.room`) as a decor's (`.decor`, `base dungeon`): its
/// notes, its name and size, its level if it has one, the name it shows,
/// its dungeon and floor, its layout as it was.
pub fn as_decor(name: &str, text: &str, level: Option<i64>) -> Result<String, Diagnostic> {
    let plan = plan_of(text, &format!("{name}.room"))?;
    let notes: Vec<&str> = text.lines().take_while(|line| line.trim().is_empty() || line.trim_start().starts_with('#')).collect();
    let mut made: Vec<String> = notes.iter().map(|line| line.to_string()).collect();
    let (rows, columns) = (plan.rows.len(), plan.rows[0].1.len());
    made.push(format!("decor: {} {} {}", plan.name, columns * CHUNK, rows * CHUNK));
    if let Some(level) = level {
        made.push(format!("level {level}"));
    }
    if let Some(shown) = &plan.shown {
        made.push(format!("name {shown}"));
    }
    made.push(format!("base dungeon {} {}", plan.dungeon, plan.floor));
    made.push("layout".to_string());
    let raw: Vec<&str> = text.lines().collect();
    for (line, _) in &plan.rows {
        made.push(raw[line - 1].trim_end().to_string());
    }
    Ok(made.join("\n") + "\n")
}

/// What is told of a room built.
pub fn told(name: &str, room: &Room) -> String {
    let decor = &room.place.decor;
    let spots: Vec<String> = decor.entrances.iter().map(|(_, x, y, letter)| format!("{letter} ({x}, {y})")).collect();
    let spots = if spots.is_empty() { "none".to_string() } else { spots.join(", ") };
    let moving = if room.place.cycles.is_empty() { String::new() } else { format!(", {} moving palette(s)", room.place.cycles.len()) };
    format!("{name}: level {}, {}x{}, tileset {}{moving}, spots: {spots}\n", room.level, decor.width, decor.height, room.tileset)
}

/// A floor made as the game makes one (`generate <name> <dungeon> <floor> [seed]
/// [width height]`: the dungeon's name runs to the first number, its
/// floor), written as a room of that name and built: its file, the room.
pub fn generated(project: &assets::project::Project, words: &[String], data: &str, extra: &Path) -> Result<(PathBuf, Room), Diagnostic> {
    let usage = || Diagnostic::said("generate <name> <dungeon> <floor> [seed] [width height]".to_string());
    let digits = |word: &str| !word.is_empty() && word.chars().all(|digit| digit.is_ascii_digit());
    let at = words.iter().enumerate().position(|(at, word)| at > 1 && digits(word)).ok_or_else(usage)?;
    let (name, dungeon, floor) = (&words[0], words[1..at].join(" "), words[at].parse::<i64>().map_err(|_| usage())?);
    let numbers: Vec<i64> = words[at + 1..].iter().filter(|word| digits(word)).filter_map(|word| word.parse().ok()).collect();
    let seed = numbers.first().copied().unwrap_or(1);
    let (columns, rows) = if numbers.len() >= 3 { (numbers[1] as usize, numbers[2] as usize) } else { (32, 20) };
    tileset_of(data, &dungeon, floor, &Location::in_file("generate", 0))?;
    let path = project.decors().join(format!("{name}.room"));
    if path.exists() {
        return Err(Diagnostic::said(format!("{} already exists: another name, or delete it first", path.display())));
    }
    let text = generate::make(name, &dungeon, floor, seed, columns, rows)?;
    // (Written as a decor, `base dungeon`: built as the others are.)
    let path = project.decors().join(format!("{name}.decor"));
    if path.exists() {
        return Err(Diagnostic::said(format!("{} already exists: another name, or delete it first", path.display())));
    }
    let level = crate::write::free_level(&Game::new(data), extra, &project.decors());
    std::fs::write(&path, as_decor(name, &text, Some(level))?).map_err(|why| Diagnostic::said(format!("{}: {why}", path.display())))?;
    let workshop = crate::workshop(project, data)?;
    let (place, level) = crate::make(&workshop, crate::load(project, name, None)?, extra)?;
    Ok((path, Room { place, level, tileset: -1 }))
}
