//! Places for films, written like films (`library/decors/<name>.decor`),
//! built from the pieces of the game's own places (`library/tables/pieces.txt`)
//! into data/extra (never committed), as the game reads its places; and the
//! dungeon rooms (`<name>.room`), laid with a floor's own tiles.
//!
//! - `fault`: what goes wrong, and on which line of which file;
//! - `lang`: a place's file read;
//! - `catalogue`: the pieces, in the order they are written;
//! - `source`: the game's places as the decors read them;
//! - `matte`: a piece cut out of its ground, a shadow dropped on another;
//! - `cut`: the pieces cut; `weave`: grounds, bands and paths woven;
//! - `terrain`: terrains painted, of a place's own tiles (autotiling);
//! - `build`: a place laid, line after line; `remarks`: what it remarks;
//! - `write`: written into data/extra;
//! - `room`: the dungeon rooms, and whole floors made as the game makes them;
//! - `inventory`: every thing of the game's places, found by itself;
//!   `sort`: those offered, one of each; `tags`: what our eyes say of them
//!   (a category, a name, tags) and of the places (their kind);
//! - `sheets`: what is looked at, in target/decors/;
//! - `pixels`, `random`: grids, colours and masks; chance from a seed.

pub mod animate;
pub mod build;
pub mod catalogue;
pub mod cut;
pub mod doc;
pub mod fault;
pub mod grounds;
pub mod inventory;
pub mod lang;
pub mod matte;
pub mod pixels;
pub mod random;
pub mod remarks;
pub mod room;
pub mod sheets;
pub mod sort;
pub mod source;
pub mod tags;
pub mod terrain;
pub mod tilesets;
pub mod weave;
pub mod write;

#[cfg(test)]
mod tests;

use crate::fault::Diagnostic;
use assets::project::Project;
use std::path::{Path, PathBuf};

use build::{Builder, Place};
use catalogue::Catalogue;
use lang::{Decor, closest, parse};
use source::Game;
use weave::Workshop;

/// The pieces the ROM keeps apart (`decor inventory`), made from the user's
/// own copy at extraction, in that data folder's `extra/`.
pub fn game_pieces(data: &str) -> PathBuf {
    write::extra_of(data).join("game_pieces.txt")
}

/// Where the pieces the places are made of are listed (`library/tables`:
/// `pieces.txt`, and `set_aside.txt`).
pub fn pieces_folder(project: &Project) -> PathBuf {
    project.root().join("library").join("tables")
}

/// Where a place's file is: the project's (`library/decors`), else the
/// first game's that has it (`games/<game>/decors`).
pub fn file_of(project: &Project, name: &str) -> PathBuf {
    let file = format!("{name}.decor");
    let own = project.decors().join(&file);
    if own.is_file() {
        return own;
    }
    let games = project.game_folders().into_iter().map(|game| game.join("decors").join(&file));
    games.into_iter().find(|path| path.is_file()).unwrap_or(own)
}

/// The places of the project (the shared library's: `library/decors`), by name.
pub fn names(project: &Project) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(project.decors())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(".decor").map(str::to_string))
        .collect();
    names.sort();
    names
}

/// The places only a game has (`games/<name>/decors/`, a name the shared
/// library has not: a game's copy of a library place looks as the library's
/// does): each its game's project and its name.
pub fn games_own_places(library: &Project) -> Vec<(Project, String)> {
    let shared = names(library);
    let mut found = Vec::new();
    for folder in library.game_folders() {
        let game = library.at(folder);
        found.extend(names(&game).into_iter().filter(|name| !shared.contains(name)).map(|name| (game.clone(), name)));
    }
    found
}

/// Every place built again into `extra` (`decor all`, and the Workshop's first
/// launch): the library's, those only a game has, the dungeon rooms, then the
/// ground of the game's own places. Each told as it is built (`Ok`), or why
/// it could not be (`Err`: the others still are); the names of those not built.
pub fn build_all(library: &Project, data: &str, extra: &Path, tell: &mut dyn FnMut(Result<String, String>)) -> Result<Vec<String>, String> {
    let workshop = workshop(library, data).map_err(|why| why.to_string())?;
    let mut failed = Vec::new();
    let places = names(library).into_iter().map(|name| (library.clone(), name)).chain(games_own_places(library));
    for (project, name) in places {
        match load(&project, &name, None).and_then(|decor| make(&workshop, decor, extra)) {
            Ok((place, level)) => tell(Ok(told(&place, level))),
            Err(why) => {
                tell(Err(why.to_string()));
                failed.push(name);
            }
        }
    }
    for name in room::names(library) {
        match room::build(library, &name, data, extra) {
            Ok(made) => tell(Ok(room::told(&name, &made))),
            Err(why) => {
                tell(Err(why.to_string()));
                failed.push(name);
            }
        }
    }
    let count = grounds::write(data, extra, library.root())?;
    tell(Ok(format!(
        "{}: the ground of {count} places of the game
",
        extra.join("ground.txt").display()
    )));
    Ok(failed)
}

/// A new place written into the project's decors, to be made its own (its
/// level fixed): its file.
pub fn new_place(project: &Project, name: &str, width: usize, height: usize, level: Option<i64>) -> Result<PathBuf, Diagnostic> {
    let folder = project.decors();
    let path = folder.join(format!("{name}.decor"));
    if path.exists() {
        return Err(Diagnostic::said(format!("{name}.decor already exists: decor view {name}")));
    }
    if name.is_empty() || name != name.to_lowercase() || !name.chars().all(|letter| letter.is_alphanumeric() || letter == '_') {
        return Err(Diagnostic::said(format!("\"{name}\": a name in small letters, letters, digits and _")));
    }
    let (least, most) = lang::SIDES;
    if ![width, height].iter().all(|side| (least..=most).contains(&(*side as i64))) {
        return Err(Diagnostic::said(format!("width and height from {least} to {most} pixels")));
    }
    std::fs::create_dir_all(&folder)
        .and_then(|()| std::fs::write(&path, doc::made(name, "", [width, height], level, "herbe_bourg", None, None)))
        .map_err(|why| Diagnostic::said(format!("{}: {why}", path.display())))?;
    Ok(path)
}

/// A place of the game brought in to be made one's own (`decor import`):
/// a decor as large, with it under all (`base level`), its level fixed.
pub fn imported(project: &Project, data: &str, extra: &Path, level: i64, name: &str) -> Result<PathBuf, Diagnostic> {
    let path = project.decors().join(format!("{name}.decor"));
    if path.exists() {
        return Err(Diagnostic::said(format!("{name}.decor already exists: decor view {name}")));
    }
    if name.is_empty() || name != name.to_lowercase() || !name.chars().all(|letter| letter.is_alphanumeric() || letter == '_') {
        return Err(Diagnostic::said(format!("\"{name}\": a name in small letters, letters, digits and _")));
    }
    let game = Game::new(data);
    let source = game.source(level)?;
    let own = write::free_level(&game, extra, &project.decors());
    let shown = doc::made(name, "", [1, 1], None, "", None, None).lines().find_map(|line| line.strip_prefix("name ").map(str::to_string)).unwrap_or_default();
    let text = format!(
        "# Place {level} of the game, as it is, to make one's own on top.\ndecor: {name} {} {}\nlevel {own}\nname {shown}\nlike {level}\nbase level {level}\n",
        source.info.width, source.info.height
    );
    std::fs::write(&path, text).map_err(|why| Diagnostic::said(format!("{}: {why}", path.display())))?;
    Ok(path)
}

/// The elements of a place of the game, as the catalogue has them: those
/// found whole (`e<level>_<n>`) and the things found alone (`l<level>_<n>`),
/// not the place whole nor its grounds.
pub fn elements_of(catalogue: &Catalogue, level: i64) -> Vec<&catalogue::Piece> {
    let ours = |name: &str| {
        let rest = name.strip_prefix('e').or_else(|| name.strip_prefix('l')).or_else(|| name.strip_prefix('a')).unwrap_or_default();
        rest.split_once('_').is_some_and(|(of, number)| of == level.to_string() && !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
    };
    // (Its animated tiles too, `a…`: laid apart, moved like the others.)
    let laid = |piece: &catalogue::Piece| !piece.has("base") || piece.has("tiles");
    catalogue
        .pieces
        .iter()
        .filter(|piece| piece.level == catalogue::Level::Place(level) && piece.kind == "object" && laid(piece) && ours(&piece.name))
        .collect()
}

/// Whether a piece is one the ROM itself tells apart, offered to lay: an
/// object of a place's upper layer (`e…`, `over`), a stretch of its
/// animated tiles (`a…`, `tiles`), a thing the scenes lay (`o…`).
pub fn of_the_rom(piece: &catalogue::Piece) -> bool {
    match piece.name.chars().next() {
        Some('e') => piece.has("over"),
        Some('a') => piece.has("tiles"),
        Some('o') => matches!(piece.level, catalogue::Level::Thing(_)),
        _ => false,
    }
}

/// The elements the Workshop offers, in the catalogue's order: those the
/// ROM tells apart (`of_the_rom`), none set aside (`set_aside.txt`, read by
/// `sort::set_aside`) but those only almost like another; the same to the
/// pixel as one of them are its twins (`sort::twins`), not offered again.
pub fn offered<'a>(catalogue: &'a Catalogue, aside: &std::collections::HashMap<String, String>) -> Vec<&'a catalogue::Piece> {
    catalogue.pieces.iter().filter(|piece| of_the_rom(piece) && sort::offered(aside.get(&piece.name).map(String::as_str))).collect()
}

/// The text of a decor that makes a place of the game again, as it is, of
/// pieces of the catalogue only: its rest (`r<level>`: the place without
/// its elements), then each element laid where the game has it, without a
/// shadow of its own (the place's stays in its rest). The same as the game's
/// to the pixel if, and only if, the pieces are right; each may then be
/// moved, taken out, laid again elsewhere.
pub fn recreated(project: &Project, data: &str, level: i64, name: &str, title: &str) -> Result<String, Diagnostic> {
    let game = Game::new(data);
    let source = game.source(level)?;
    let catalogue = Catalogue::read(&pieces_folder(project), data)?;
    let elements = elements_of(&catalogue, level);
    let rest = format!("r{level}");
    if catalogue.get(&rest).is_none() {
        return Err(Diagnostic::said(format!("no piece \"{rest}\": decor inventory {level}")));
    }
    let mut lines = vec![
        format!(
            "# Place {level} of the game, made again of pieces alone: its rest ({rest}, the place without its elements) and its {} elements,",
            elements.len()
        ),
        "# each in its place. Identical to the game if the pieces are right; then to change (move, remove, lay elsewhere).".to_string(),
        format!("decor: {name} {} {}", source.info.width, source.info.height),
        format!("name {title}"),
        format!("like {level}"),
        // (Its rest locked: a click takes the element over it, not the whole place.)
        format!("put {rest} 0 0 shadow off locked"),
    ];
    // (Its animated tiles first, on its ground; then its objects, the lowest last.)
    let mut sorted = elements;
    sorted.sort_by_key(|piece| (!piece.name.starts_with('a'), piece.rect.3, piece.rect.0));
    for piece in sorted {
        lines.push(format!("put {} {} {} shadow off", piece.name, piece.rect.0, piece.rect.1));
    }
    Ok(lines.join("\n") + "\n")
}

/// Every place and room of library/decors written as now (`decor
/// migrate`): each `.room` made a `.decor` (`base dungeon`, its layout), each
/// `.decor` its level said, its layers named as now. What changed.
pub fn migrate(project: &Project, data: &str, extra: &Path) -> Result<Vec<String>, Diagnostic> {
    let game = Game::new(data);
    let failed = |path: &Path, why: std::io::Error| Diagnostic::said(format!("{}: {why}", path.display()));
    let mut told = Vec::new();
    let folder = project.decors();
    for name in room::names(project) {
        let from = folder.join(format!("{name}.room"));
        let to = folder.join(format!("{name}.decor"));
        if to.exists() {
            return Err(Diagnostic::said(format!("{} and {}: one of the two too many", from.display(), to.display())));
        }
        let text = std::fs::read_to_string(&from).map_err(|why| failed(&from, why))?;
        let level = write::known_level(extra, &name).unwrap_or_else(|| write::free_level(&game, extra, &folder));
        std::fs::write(&to, room::as_decor(&name, &text, Some(level))?).map_err(|why| failed(&to, why))?;
        std::fs::remove_file(&from).map_err(|why| failed(&from, why))?;
        told.push(format!("{name}.room → {name}.decor (level {level})"));
    }
    for name in names(project) {
        let path = folder.join(format!("{name}.decor"));
        let text = std::fs::read_to_string(&path).map_err(|why| failed(&path, why))?;
        let level = write::known_level(extra, &name).unwrap_or_else(|| write::free_level(&game, extra, &folder));
        let made = doc::migrated(&text, level);
        if made != text {
            std::fs::write(&path, &made).map_err(|why| failed(&path, why))?;
            told.push(format!("{name}.decor"));
        }
    }
    Ok(told)
}

/// A place's file read: that name's in the project (or the text of that
/// file), errors with their line.
pub fn load(project: &Project, name: &str, path: Option<&Path>) -> Result<Decor, Diagnostic> {
    let path = path.map_or_else(|| file_of(project, name), Path::to_path_buf);
    let Ok(text) = std::fs::read_to_string(&path) else {
        let known = names(project);
        return Err(Diagnostic::said(format!("no place \"{name}\" in {}.{}", project.decors().display(), closest(name, known.iter().map(String::as_str)))));
    };
    read(name, &text)
}

/// The text of the place of that name read: it must say that name.
pub fn read(name: &str, text: &str) -> Result<Decor, Diagnostic> {
    let file = format!("{name}.decor");
    let decor = parse(text, &file)?;
    if decor.name != name {
        return Err(fault::Location::in_file(file, 1).wrong(format!("it is called \"{}\", its file \"{name}\"", decor.name)));
    }
    Ok(decor)
}

/// What places are made with, read from the game's files in that folder,
/// for that project's places (its pictures, the levels its places say).
pub fn workshop(project: &Project, data: &str) -> Result<Workshop, Diagnostic> {
    let catalogue = Catalogue::read(&pieces_folder(project), data)?;
    Workshop::new(Game::new(data), catalogue, project.clone())
}

/// A piece as a decor lays it, for one who draws it himself (an editor):
/// cut from its place, a place's rest hollowed of its elements; kept by the
/// workshop, cut once.
pub fn laid_cut(workshop: &Workshop, name: &str) -> Result<std::rc::Rc<cut::Cut>, Diagnostic> {
    if let Some(known) = workshop.cuts.borrow().get(name).cloned() {
        return Ok(known);
    }
    let piece = workshop.catalogue.get(name).ok_or_else(|| Diagnostic::said(format!("unknown piece \"{name}\"")))?.clone();
    let mut made = cut::cut(workshop, &piece)?;
    if piece.has("rest") {
        made = Builder::new(workshop, Decor::default()).rest_of(&piece, made);
    }
    let made = std::rc::Rc::new(made);
    workshop.cuts.borrow_mut().insert(name.to_string(), made.clone());
    Ok(made)
}

/// A place built and written into `extra`: what was laid, its level.
pub fn make(workshop: &Workshop, decor: Decor, extra: &Path) -> Result<(Place, i64), Diagnostic> {
    let place = built(workshop, decor)?;
    let level = written(workshop, &place, extra)?;
    Ok((place, level))
}

/// A place built, in memory only: shown at once by one who edits it
/// (`Place::picture_at`), written out afterwards.
pub fn built(workshop: &Workshop, decor: Decor) -> Result<Place, Diagnostic> {
    // (Of the game's own pieces only: built simply; else, by the full builder.)
    if build::compose::simple(&decor) {
        return build::compose::composed(workshop, decor);
    }
    Builder::new(workshop, decor).build()
}

/// A place built, written into `extra` as the game reads its places: its level.
pub fn written(workshop: &Workshop, place: &Place, extra: &Path) -> Result<i64, Diagnostic> {
    let level = write::level_of(&workshop.game, extra, &place.decor, &workshop.project.decors());
    write::write(&workshop.game, place, level, extra)?;
    Ok(level)
}

/// A string as JSON writes it.
pub(crate) fn quoted(said: &str) -> String {
    let mut out = String::from("\"");
    for letter in said.chars() {
        match letter {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            letter if (letter as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", letter as u32)),
            letter => out.push(letter),
        }
    }
    out.push('"');
    out
}

/// What is told of a place built, for an agent (`decor build --json`):
/// one line of JSON, its name, level, size, frames, walls, middle, spots
/// and entrances, what it lays by layer, its remarks by line.
pub fn told_json(place: &Place, level: i64) -> String {
    let decor = &place.decor;
    let (x, y) = write::middle_of(place);
    let spots: Vec<String> =
        decor.entrances.iter().map(|(line, x, y, name)| format!("{{\"name\":{},\"x\":{x},\"y\":{y},\"line\":{line}}}", quoted(name))).collect();
    let remarks: Vec<String> = place.remarks.iter().map(|remark| format!("{{\"line\":{},\"says\":{}}}", remark.line(), quoted(&remark.message))).collect();
    let laid = |layer: Option<&str>| {
        decor.lines.iter().filter(|line| matches!(line.verb.as_str(), "put" | "water" | "image") && line.layer.as_deref() == layer).count()
    };
    format!(
        "{{\"name\":{},\"level\":{level},\"title\":{},\"width\":{},\"height\":{},\"grid\":{},\"images\":{},\"wall_tiles\":{},\"center\":[{x},{y}],\"layers\":{{\"ground\":{},\"objects\":{},\"over\":{}}},\"spots\":[{}],\"remarks\":[{}]}}",
        quoted(&decor.name),
        quoted(decor.shown.as_deref().unwrap_or("")),
        decor.width,
        decor.height,
        decor.grid.unwrap_or(doc::GRID),
        place.lower.len(),
        place.walls.count(),
        laid(Some("ground")),
        laid(None),
        laid(Some("over")),
        spots.join(","),
        remarks.join(",")
    )
}

/// What is told of a place built (`decor build`).
pub fn told(place: &Place, level: i64) -> String {
    let decor = &place.decor;
    let (x, y) = write::middle_of(place);
    let changing = if place.cycles.is_empty() { "" } else { "changing colours, " };
    let mut out = format!(
        "{}: level {level}, {}x{}, {} image(s), {changing}{} wall tiles, center {x} {y}\n",
        decor.name,
        decor.width,
        decor.height,
        place.lower.len(),
        place.walls.count()
    );
    for remark in &place.remarks {
        out.push_str(&format!("  {}: {}\n", fault::Location::in_file(decor.file.clone(), remark.line()), remark.message));
    }
    if place.remarks.is_empty() {
        out.push_str("  nothing to say\n");
    }
    out
}
