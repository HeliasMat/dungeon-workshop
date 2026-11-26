//! A film, or a dungeon, exported to Explorers of Sky (the DS): what
//! `scene_player export-ds` writes, and `tools/ds/patch_rom.py` puts in a
//! copy of the user's own ROM (docs/DS.md).
//!
//! - [`film()`]: a film as a cutscene of the original game, written in
//!   ExplorerScript (SkyTemple's language for Sky's scripts), and each decor
//!   its scenes are in as a map of Sky's of its own, beside it.
//! - [`dungeon()`]: a dungeon of the Workshop (and the fight it ends with) as
//!   the rules read it, in JSON, for `tools/ds/dungeon.py`.
//! - [`table()`]: every word of the vocabulary and what it becomes on the DS.
//!
//! The export reads what the film has the engine play, its typed beats
//! (`Written::events`: places, Pokémon, emotes and sounds are already the
//! game's numbers), and writes each beat as the game's operations. First,
//! each line's word is looked up in its DS status (`film::ds::export_of`,
//! declared with each word in `film_core`): a word that has no Sky operation
//! yet is an error, never dropped in silence. A few beats only our engine
//! needs (the ground grid, its light, the title) are left out, and told.
//!
//! The game's names (levels, actors) are read from the data folder at run
//! time; nothing of the game is written here.
//!
//! Inside: `writer` writes the beats (`writer/`, a file for each kind of
//! beat), `sky` is what it knows of Sky (its data read, its tables), `map`
//! makes a decor a map of Sky's, `dungeon` writes a dungeon.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use ::film::ds::{Export, export_of};
use ::film::tables::{Tables, game_file};
use ::film::{Written, vocabulary};

use engine::{Beat, FlowBeat, Who};

mod dungeon;
mod map;
mod sky;
#[cfg(test)]
mod tests;
mod writer;

pub use dungeon::dungeon;

use sky::{DECOR_LEVEL, Game};
use writer::Writer;

/// A film to export: what it is called, its text, what it compiled to, the
/// tables it compiled with, and its project's art (to stage its fights by
/// their rules; without it, a short exchange each).
pub struct Film<'a> {
    pub name: &'a str,
    pub text: &'a str,
    pub written: &'a Written,
    pub tables: &'a Tables,
    pub art: Option<&'a preview::art::Art>,
}

/// How to export it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// What doesn't export is left out (and told), not an error.
    pub partial: bool,
    /// The top screen shows the place above the bottom one, where it is tall enough.
    pub tall: bool,
}

/// What an export wrote: how many actors, the first scene's place.
pub struct Exported {
    pub actors: usize,
    pub place: Option<String>,
}

/// The film written to `out` (ExplorerScript), each decor its scenes are in
/// as a map beside it (`<out>.decor<level>.*`). `tell` is given, as it
/// goes, what was left out and what was changed to fit the DS. Err: what
/// doesn't export (unless `partial`), or a file that can't be read or written.
pub fn film(film: &Film, options: Options, out: &Path, tell: &mut dyn FnMut(String)) -> Result<Exported, String> {
    let (name, written) = (film.name, film.written);
    let move_types = film.art.map(|art| art.tables.move_slots.iter().map(|slot| slot.type_id).collect()).unwrap_or_default();
    let mut game = Game::read(&film.tables.data, move_types);
    // (Two decors built on one level: the first by name.)
    for (name, (level, ..)) in &film.tables.decors {
        game.decors.entry(*level).or_insert_with(|| name.clone());
    }
    let mut writer = Writer::new(&game);
    writer.tall_asked = options.tall;
    if let Some(art) = film.art {
        writer.fights = writer::fights_played(written, art, &game);
    }
    // (A word the film defines, `define greet`: its lines are written out already.)
    let defined: Vec<&str> = film.text.lines().filter_map(|line| line.trim().strip_prefix("define ")?.split_whitespace().next()).collect();
    for (number, read) in written.read_as.iter().filter(|(_, read)| !defined.contains(&read.word.as_str())) {
        match export_of(&read.word) {
            Some(Export::Direct | Export::Close) => {}
            Some(_) => writer.problems.push(format!("line {number}: “{}” doesn't export to DS yet", read.word)),
            None => writer.problems.push(format!("line {number}: “{}” (not a word of the vocabulary) doesn't export to DS yet", read.word)),
        }
    }
    for (number, read) in &written.read_as {
        let words = read.values.iter().flat_map(|value| value.words.iter().cloned()).collect();
        writer.words.insert(*number as i32, words);
    }
    // (The film's names of its Pokémon, and the faces of its lines said in bubbles.)
    for (code, name) in &written.names {
        if let Some(who) = Who::parse(code) {
            writer.film_names.entry(who).or_insert_with(|| name.clone());
        }
    }
    for (at, line) in film.text.lines().enumerate() {
        if let Some(face) = bubble_face(line) {
            writer.bubble_faces.insert(at as i32 + 1, face);
        }
    }
    // The typed beats the engine plays, each after the line it is of.
    let mut number = 0;
    for scene in written.events.iter().flat_map(|event| &event.scenes) {
        for beat in &scene.beats {
            if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
                number = *line;
            }
            let word = usize::try_from(number).ok().and_then(|line| written.read_as.get(&line)).map(|read| read.word.as_str());
            // (`DS_BEATS=1`: each beat as the export reads it.)
            if std::env::var_os("DS_BEATS").is_some() {
                eprintln!("{number:>4} {:<10} {beat:?}", word.unwrap_or(""));
            }
            writer.beat(number, word, beat);
        }
    }
    if options.partial {
        let problems = std::mem::take(&mut writer.problems);
        writer.notes.extend(problems);
    }
    if !writer.problems.is_empty() {
        return Err(format!("{name} doesn't export to DS:\n  {}", writer.problems.join("\n  ")));
    }
    let mut text = writer.finish(name);
    for note in &writer.notes {
        tell(format!("  left out: {note}"));
    }
    for told in &writer.told {
        tell(format!("  {told}"));
    }
    for ((species, asked, shown), lines) in &writer.instead_poses {
        tell(format!("  {species}'s Sky sprite has no {asked}: {shown} instead on DS ({})", on_lines(lines)));
    }
    if let Some(folder) = out.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    // The decors the scenes are in, each as a map beside the export.
    let maps = decor_maps(&game, film.tables, &text, out, tell)?;
    if !maps.is_empty() {
        let at = text.find('\n').map_or(0, |end| end + 1);
        text.insert_str(at, &maps);
    }
    std::fs::write(out, text).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(Exported { actors: writer.actors.len(), place: writer.level.clone() })
}

/// The face a line said in a bubble is given by its brackets (`pichu
/// (bubble, joyous): …`): none if it is not said in a bubble, its normal
/// one if they give none.
fn bubble_face(line: &str) -> Option<u16> {
    let (head, _) = line.trim().split_once(':')?;
    let brackets = head.trim_end().strip_suffix(')')?.split_once('(')?.1;
    let words: Vec<String> = brackets.split(',').map(::film::words::plain).collect();
    if !words.iter().any(|word| word == "bubble" || word == "top") {
        return None;
    }
    let face = words.iter().find_map(|word| ::film::words::of(&::film::words::FACES, word)).unwrap_or("normal");
    scenes::vocabulary::POKEMON.face(face).or(Some(0))
}

/// Lines told by their numbers: `line 4`, `lines 4, 9 and 12`.
fn on_lines(lines: &[i32]) -> String {
    let mut lines = lines.to_vec();
    lines.dedup();
    match lines.as_slice() {
        [one] => format!("line {one}"),
        [rest @ .., last] => format!("lines {} and {last}", rest.iter().map(i32::to_string).collect::<Vec<_>>().join(", ")),
        [] => String::new(),
    }
}

/// Each decor the export's scenes are in (`LEVEL_DECOR_<n>`), written as a
/// map beside `out` (`map`): its line for `patch_rom.py`, `// ds-export
/// map LEVEL_DECOR_<n> <name> <width> <height> <lower> <upper> <walls>`.
fn decor_maps(game: &Game, tables: &Tables, text: &str, out: &Path, tell: &mut dyn FnMut(String)) -> Result<String, String> {
    let mut levels: Vec<u16> = text
        .match_indices(DECOR_LEVEL)
        .filter_map(|(at, _)| {
            let rest = &text[at + DECOR_LEVEL.len()..];
            rest[..rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len())].parse().ok()
        })
        .collect();
    levels.sort_unstable();
    levels.dedup();
    let mut lines = String::new();
    for level in levels {
        let name = game.decors.get(&level).cloned().unwrap_or_else(|| level.to_string());
        let picture = |part: &str| -> Result<Option<(usize, usize, Vec<u8>)>, String> {
            let path = game_file(&game.data, &format!("grounds/pictures/{level}_{part}0.png"));
            if !path.exists() {
                return Ok(None);
            }
            let image = image::open(&path).map_err(|error| format!("{}: {error}", path.display()))?.to_rgba8();
            Ok(Some((image.width() as usize, image.height() as usize, image.into_raw())))
        };
        let (w, h, lower) = picture("f")?.ok_or(format!("{name}: the decor is not built (no grounds/pictures/{level}_f0.png): build it first"))?;
        let upper = picture("u")?.filter(|(uw, uh, _)| (*uw, *uh) == (w, h)).map(|(_, _, pixels)| pixels);
        let solid = tables.grounds.get(&level).map(|ground| ground.rows().to_vec());
        let map = map::map_of(w, h, &lower, upper.as_deref(), solid.as_deref());
        let files = map.write(Path::new(&format!("{}.decor{level}", out.display())))?;
        tell(format!(
            "  {name}: a map of its own, {}x{} pixels ({}x{} chunks), {} palettes{}; its animations and changing colours as they first are, the top screen black",
            map.width,
            map.height,
            map.width / map::CHUNK,
            map.height / map::CHUNK,
            map.palettes.len(),
            if map.requantized > 0 { format!(", {} tiles brought to their palette's nearest colours", map.requantized) } else { String::new() },
        ));
        let _ = writeln!(
            lines,
            "// ds-export map {DECOR_LEVEL}{level} {} {} {} {}",
            name.replace(char::is_whitespace, "_"),
            map.width,
            map.height,
            files.join(" ")
        );
    }
    Ok(lines)
}

/// The table (`film::ds`), every word of the vocabulary, and how many of
/// the core's and of the others' map directly, closely, need work, or can't.
pub fn table() -> String {
    let mut out = String::new();
    let mut counts: BTreeMap<(bool, &str), (usize, usize)> = BTreeMap::new();
    for word in vocabulary::words() {
        let kind = export_of(word.word).map_or("missing", Export::kind);
        let how = ::film::ds::how(word.word).unwrap_or("");
        let lost = ::film::ds::lost(word.word).map_or(String::new(), |lost| format!(" (lost: {lost})"));
        let _ = writeln!(out, "{:<11} {:<12} {:<7} {how}{lost}", word.family, word.word, kind);
        let core = vocabulary::is_core(word.family);
        let count = counts.entry((!core, kind)).or_default();
        count.0 += 1;
        if matches!(word.made, vocabulary::Made::Word | vocabulary::Made::Command) {
            count.1 += 1;
        }
    }
    let _ = writeln!(out);
    for ((other, kind), (all, words)) in counts {
        let of = if other { "optional and advanced" } else { "core" };
        let _ = writeln!(out, "{of} {kind}: {all} ({words} of them words, the rest syntax and recipes)");
    }
    out
}
