//! A film, or a dungeon, exported to Explorers of Sky (the DS): what
//! `scene_player export-ds` writes, and `tools/ds/patch_rom.py` puts in a
//! copy of the user's own ROM (docs/DS.md).
//!
//! - [`film()`]: a film as a cutscene of the original game, written in
//!   ExplorerScript (SkyTemple's language for Sky's scripts), each decor
//!   its scenes are in as a map of Sky's of its own, and each Pokémon
//!   SpriteCollab adds that it casts as its sheets in SpriteBot's layout,
//!   beside it.
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

use ::film::ds::room::{self, Room};
use ::film::ds::{Export, export_of};
use ::film::tables::{Tables, game_file};
use ::film::{Written, vocabulary};

use engine::{Beat, FlowBeat, Who};

mod dungeon;
mod map;
mod notes;
mod sky;
#[cfg(test)]
mod tests;
mod writer;

pub use dungeon::dungeon;
pub use notes::{Fate, Kind, Note, Report, Summary};

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
    /// A scene's sprites past the room (`film::ds::room`) kept, drawn as
    /// magenta squares, not split nor left out.
    pub squares: bool,
}

/// What an export wrote: how many actors, the first scene's place, what it
/// tells of the film (`Report`).
pub struct Exported {
    pub actors: usize,
    pub place: Option<String>,
    pub report: Report,
}

/// The film written to `out` (ExplorerScript), each decor its scenes are in
/// as a map beside it (`<out>.decor<level>.*`). `tell` is given, as it
/// goes, each note of the report (what was left out and what was changed
/// to fit the DS, a line each) and how the maps were made. Err: what
/// doesn't export (unless `partial`), or a file that can't be read or written.
pub fn film(film: &Film, options: Options, out: &Path, tell: &mut dyn FnMut(String)) -> Result<Exported, String> {
    let game = game_of(film);
    let mut writer = write(film, &game, options);
    if !writer.problems.is_empty() {
        let problems: Vec<String> = writer.problems.iter().map(Note::to_string).collect();
        return Err(format!("{} doesn't export to DS:\n  {}", film.name, problems.join("\n  ")));
    }
    let mut text = writer.finish(film.name);
    let report = report_of(film, &writer);
    for note in &report.notes {
        tell(format!("  {note}"));
    }
    for info in &writer.info {
        tell(format!("  {info}"));
    }
    if let Some(folder) = out.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    // The decors the scenes are in, each as a map beside the export; the
    // Pokémon SpriteCollab adds, each as its sheets.
    let maps =
        decor_maps(&game, film.tables, &text, out, tell)? + &added_monsters(&game, &writer.added, out, tell)? + &item_pictures(film, &writer, out, tell)?;
    if !maps.is_empty() {
        let at = text.find('\n').map_or(0, |end| end + 1);
        text.insert_str(at, &maps);
    }
    std::fs::write(out, text).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(Exported { actors: writer.actors.len(), place: writer.level.clone(), report })
}

/// What the film loses on DS, as `film` would tell it with `--partial`,
/// nothing written: the Workshop's DS Report and the Script's DS marks.
pub fn report(film: &Film) -> Report {
    let game = game_of(film);
    let mut writer = write(film, &game, Options { partial: true, ..Options::default() });
    let _ = writer.finish(film.name);
    report_of(film, &writer)
}

/// What the export knows of Sky for that film: the game's data, the
/// moves' types of its project, its decors.
fn game_of(film: &Film) -> Game {
    let move_types = film.art.map(|art| art.tables.move_slots.iter().map(|slot| slot.type_id).collect()).unwrap_or_default();
    let mut game = Game::read(&film.tables.data, move_types);
    game.move_effects = film.art.map(|art| art.tables.move_effects.clone()).unwrap_or_default();
    // (Two decors built on one level: the first by name.)
    for (name, (level, ..)) in &film.tables.decors {
        game.decors.entry(*level).or_insert_with(|| name.clone());
    }
    game
}

/// The film's beats written, each word looked up first; `partial`: what
/// keeps it from exporting made a note of what is left out.
fn write<'a>(film: &Film, game: &'a Game, options: Options) -> Writer<'a> {
    let written = film.written;
    let mut writer = Writer::new(game);
    writer.tall_asked = options.tall;
    writer.squares = options.squares;
    if let Some(art) = film.art {
        writer.fights = writer::fights_played(written, art, game);
    }
    let defined = defined(film.text);
    let mut engine_words = Vec::new();
    for (number, read) in written.read_as.iter().filter(|(_, read)| !defined.contains(&read.word.as_str())) {
        let line = *number as i32;
        let on_ds = ::film::ds::on_ds(&read.word);
        let instead = on_ds.and_then(|ds| ds.fix).map_or("left out".to_string(), |fix| format!("left out ({fix})"));
        match on_ds.map(|ds| ds.export) {
            Some(Export::Direct | Export::Close) => {}
            Some(Export::Work) => writer.problem(line, Kind::Word, Fate::Impossible, format!("“{}”: the export doesn't write it yet", read.word), instead),
            Some(Export::None) => {
                let why = on_ds.map_or("Sky has nothing for it", |ds| ds.how);
                writer.problem(line, Kind::Word, Fate::Impossible, format!("“{}”: {why}", read.word), instead);
            }
            // (An engine's command outside the vocabulary: trackpath, shadow, gone...;
            // told after the beats, unless it was on an item the export shows.)
            None => engine_words.push((line, read.word.clone(), instead)),
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
    // (Who transforms: a special actor each, its species changed as the script runs.)
    for beat in written.events.iter().flat_map(|event| &event.scenes).flat_map(|scene| &scene.beats) {
        if let Beat::Body(engine::BodyBeat::Mimic { who, .. }) = beat
            && !writer.shifters.contains(who)
        {
            writer.shifters.push(*who);
        }
    }
    // The typed beats the engine plays, each after the line it is of; a
    // scene over the sprites' room split at a moment where both parts fit.
    let mut number = 0;
    for scene in written.events.iter().flat_map(|event| &event.scenes) {
        let mut split = if options.squares { None } else { split_of(film, game, &writer.room, scene) };
        for beat in &scene.beats {
            if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
                number = *line;
                if let Some(plan) = split.take_if(|plan| *line > plan.line) {
                    writer.split_scene(&plan.first, &plan.second, &plan.moment, plan.line);
                }
            }
            let word = usize::try_from(number).ok().and_then(|line| written.read_as.get(&line)).map(|read| read.word.as_str());
            // (`DS_BEATS=1`: each beat as the export reads it.)
            if std::env::var_os("DS_BEATS").is_some() {
                eprintln!("{number:>4} {:<10} {beat:?}", word.unwrap_or(""));
            }
            writer.beat(number, word, beat);
        }
    }
    for (line, word, instead) in engine_words {
        if !writer.item_lines.contains(&line) {
            writer.problem(line, Kind::Engine, Fate::Impossible, format!("“{word}”: an engine's command, with no DS operation"), instead);
        }
    }
    if options.partial {
        let problems = std::mem::take(&mut writer.problems);
        writer.notes.extend(problems);
    }
    writer
}

/// Where a scene over the sprites' room splits into two DS scenes: the
/// moment's line and name, the species of each part (`film::ds::room`).
struct Split {
    line: i32,
    moment: String,
    first: Vec<u16>,
    second: Vec<u16>,
}

/// A scene's split, if its species are over the room and a moment splits
/// it so that both parts fit (the hero and the partner in both).
fn split_of(film: &Film, game: &Game, room: &Room, scene: &engine::Scene) -> Option<Split> {
    let moments: Vec<usize> = film.text.lines().enumerate().filter(|(_, line)| line.trim_start().starts_with("moment:")).map(|(at, _)| at + 1).collect();
    let mut always = Vec::new();
    let acting: Vec<&str> = film
        .written
        .names
        .iter()
        .filter(|(code, name)| {
            code.parse::<u32>().is_ok()
                && film.text.lines().any(|line| line.trim_start().strip_prefix(name.as_str()).is_some_and(|rest| rest.starts_with([' ', ':', '('])))
        })
        .map(|(code, _)| code.as_str())
        .collect();
    let usage = room::usage_of(&scene.beats, &moments, |field| {
        match Who::parse(field)? {
            Who::Monster(species) | Who::Another { species, .. } => Some(::film::ds::drawn_on_ds(&game.data, species).species(species)),
            // (A plain number is a character's code, and a place's too: the hero
            // or the partner only where a line of the film starts with their name.)
            Who::Character(number) if acting.contains(&field) => {
                always.extend(game.actors.get(usize::from(number)).map(|(_, kind)| *kind));
                None
            }
            _ => None,
        }
    });
    let always = room::once(&always);
    if room::fits(&game.data, room, &[always.clone(), usage.all()].concat(), 0) {
        return None;
    }
    let at = room::split_at(&game.data, room, &usage, &always, 0)?;
    let line = usage.lines[at];
    let moment = film.text.lines().nth(line - 1).and_then(|text| text.trim_start().strip_prefix("moment:")).unwrap_or("").trim().to_string();
    let part = |range| room::once(&[always.clone(), usage.part(range)].concat());
    Some(Split { line: line as i32, moment, first: part(0..at), second: part(at..usage.moments.len()) })
}

/// The words the film defines (`define greet`): their lines are written out already.
fn defined(text: &str) -> Vec<&str> {
    text.lines().filter_map(|line| line.trim().strip_prefix("define ")?.split_whitespace().next()).collect()
}

/// The writer's notes made the film's report: with them, each word's own
/// loss on its line (`film::ds::lost`), and each pose a sprite lacks on
/// each line it is asked.
fn report_of(film: &Film, writer: &Writer) -> Report {
    let mut notes = writer.notes.clone();
    let defined = defined(film.text);
    for (number, read) in film.written.read_as.iter().filter(|(_, read)| !defined.contains(&read.word.as_str())) {
        if export_of(&read.word) == Some(Export::Close) {
            let lost = ::film::ds::lost(&read.word).unwrap_or("less than the film shows");
            notes.push(Note::new(*number as i32, Kind::Word, Fate::Changed, format!("{}: {lost}", read.word), ""));
        }
    }
    for ((species, asked, shown), lines) in &writer.instead_poses {
        for line in lines {
            notes.push(Note::new(*line, Kind::Pose, Fate::Changed, format!("{species}'s Sky sprite has no {asked} pose"), format!("{shown} instead")));
        }
    }
    let mut lines: std::collections::BTreeSet<usize> = writer.lines_seen.iter().filter_map(|line| usize::try_from(*line).ok()).collect();
    lines.extend(film.written.read_as.keys().copied());
    Report::of(notes, lines.len(), writer.added.len())
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

/// Each Pokémon SpriteCollab adds that the film casts (`added`: its
/// species, who it is drawn as where it can't be put in the ROM), its
/// sheets and faces written beside `out` in SpriteBot's layout
/// (`<out>.monster<species>/sprite/AnimData.xml`..., `portrait/<Face>.png`,
/// `spritecollab::export`), for `tools/ds/add_monsters.py` to make a
/// monster of the ROM's of: its line, `// ds-export monster <species>
/// <instead> <folder> <name>`.
fn added_monsters(game: &Game, added: &BTreeMap<u16, u16>, out: &Path, tell: &mut dyn FnMut(String)) -> Result<String, String> {
    let mut lines = String::new();
    let places = spritecollab::Places::new(&game.data, Path::new(""));
    for (species, instead) in added {
        let folder = format!("{}.monster{species}", out.file_name().and_then(|name| name.to_str()).unwrap_or("film"));
        let at = out.with_file_name(&folder);
        // (Of an export before: what this one writes only.)
        let _ = std::fs::remove_dir_all(&at);
        let written = spritecollab::export(&places, u32::from(*species), &at)?;
        // (A form is named as its base species; a Pokémon of its own by its name.)
        let name = match ::film::ds::without_sheets(&game.data, *species) {
            ::film::ds::Drawn::Base(base) => game.name(base),
            _ => game.name(*species),
        };
        tell(format!("  {name} (species {species}): put in the ROM from SpriteCollab's sheets ({written}); drawn as {} where that fails", game.name(*instead)));
        let _ = writeln!(lines, "// ds-export monster {species} {instead} {folder} {name}");
    }
    Ok(lines)
}

/// Each picture of the film's own it shows as an item (a letter, a parcel
/// drawn by hand), copied beside `out` (`<out>.item<slot>.png`) for
/// `tools/ds/add_items.py` to make its object's sprite of: its line,
/// `// ds-export item OBJECT_N<slot> picture <file>`. Without the project's
/// art (or the file), told and left as the slot's own sprite.
fn item_pictures(film: &Film, writer: &Writer, out: &Path, tell: &mut dyn FnMut(String)) -> Result<String, String> {
    let mut lines = String::new();
    for (slot, file) in writer.item_lines_for_patch().1 {
        let Some(art) = film.art else {
            tell(format!("  the picture {file}: not found without the project's art, not shown on DS"));
            continue;
        };
        let from = shell::art::ours(&art.folder, &art.project, &file);
        let name = format!("{}.item{slot}.png", out.file_name().and_then(|name| name.to_str()).unwrap_or("film"));
        match std::fs::copy(&from, out.with_file_name(&name)) {
            Ok(_) => {
                let _ = writeln!(lines, "// ds-export item OBJECT_N{slot} picture {name}");
            }
            Err(error) => tell(format!("  the picture {file} ({}): {error}, not shown on DS", from.display())),
        }
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
