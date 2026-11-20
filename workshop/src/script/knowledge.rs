//! What the text says of a word: where a name is given, what is known
//! there, a place's and a species' names, the picture a line shows, the
//! portraits.

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;

use crate::session::Session;

use super::highlight::names_of;
use super::text::{line_at, spot_of, word_at};

/// Where a name is given: in this film (its line), in a shared file (opened there), or what is said of it.
pub(super) enum Defined {
    Here(usize),
    File(PathBuf, usize),
    Said(String),
}

/// Where the word at the cursor is given: the `define` of a recipe (here, or
/// in the file it comes from), the file a `uses` or `include` names.
pub(super) fn defined(session: &Session, text: &str, cursor: usize) -> Defined {
    let (_, start, line) = line_at(text, cursor);
    let first = line.split_whitespace().next().map(film::words::plain).unwrap_or_default();
    if matches!(first.as_str(), "uses" | "include") {
        let name = line.split_whitespace().nth(1).unwrap_or_default().to_string();
        let _ = start;
        return match film::shared_path(session.library(), &name) {
            Some(path) => Defined::File(path, 1),
            None => Defined::Said(format!("\"{name}\": no shared file of that name")),
        };
    }
    match film::service::definition(text, spot_of(text, cursor), session.library()) {
        Some((None, line)) => Defined::Here(line),
        Some((Some(file), line)) => match film::shared_path(session.library(), &file).or_else(|| Some(PathBuf::from(&file)).filter(|path| path.is_file())) {
            Some(path) => Defined::File(path, line),
            None => Defined::Said(format!("given in {file}, line {line}")),
        },
        None => match word_at(text, cursor).map(|word| film::words::plain(&word)).and_then(|word| film::every_word().find(|(_, known, ..)| *known == word)) {
            Some((family, known, ..)) => Defined::Said(format!("\"{known}\": a word of the language ({family})")),
            None => Defined::Said("this name is given nowhere here".to_string()),
        },
    }
}

/// What may go where a word starts, known of the game and the project
/// rather than of the language: places after `scene:`, faces in a line
/// said, the shared files after `uses` or `include`, the marks after `at`.
pub(super) fn known_here(session: &Session, text: &str, head: &str, plain: &str) -> Option<Vec<(String, String)>> {
    let trimmed = head.trim_start();
    let keep = |word: &str| film::words::plain(word).starts_with(plain);
    if let Some(rest) = trimmed.strip_prefix("scene:") {
        if !rest.trim().is_empty() {
            return None;
        }
        // (By kind of place, `library/tables/place_kinds.txt`: each with its kind and its name.)
        let tables = session.tables();
        let kinds = crate::decors::place_kinds(session.library());
        let mut places: Vec<(decor::tags::PlaceKind, String, String)> = tables
            .decors
            .keys()
            .filter(|word| keep(word))
            .map(|word| {
                let kind = kinds.of_decor(word, None);
                (kind, word.clone(), format!("{} · the project's decor", kind.shown()))
            })
            .collect();
        places.extend(tables.levels.iter().filter(|(word, _)| keep(word)).map(|(word, level)| {
            let kind = kinds.of_level(i64::from(level.level));
            (kind, word.clone(), format!("{} · {}", kind.shown(), level.name))
        }));
        places.sort_by_key(|(kind, word, _)| (*kind, word.len(), word.clone()));
        places.dedup_by(|a, b| a.1 == b.1);
        return Some(places.into_iter().take(200).map(|(_, word, said)| (word, said)).collect());
    }
    if let Some(open) = head.rfind('(')
        && !head[open..].contains(')')
        && !head[..open].contains(':')
    {
        return Some(film::words::FACES.iter().filter(|(face, _)| keep(face)).map(|(face, _)| (face.to_string(), "face".to_string())).collect());
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    match words.as_slice() {
        [first] if head.ends_with(' ') && (*first == "uses" || *first == "include") => {
            Some(film::shared_files(session.library()).into_iter().filter(|name| keep(name)).map(|name| (name, "shared file".to_string())).collect())
        }
        ["a"] if head.ends_with(' ') => {
            Some(film::lines::marks(text).into_iter().map(|(_, name)| name).filter(|name| keep(name)).map(|name| (name, "mark".to_string())).collect())
        }
        _ => None,
    }
}

/// A place as the game names it (its own word if it is one of the project's).
pub(crate) fn place_said(session: &Session, word: &str) -> String {
    let plain = film::words::plain(word);
    session.tables().levels.get(&plain).map(|level| level.name.clone()).filter(|name| !name.is_empty()).unwrap_or_else(|| word.to_string())
}

/// Someone of the film's species (as the film names it with `=`), their number off.
pub(crate) fn species_of(session: &Session, text: &str, who: &str) -> String {
    let plain = film::words::plain(who);
    for line in text.lines().filter(|line| !line.trim_start().starts_with('#')) {
        if let Some((name, species)) = line.split_once('=').map(|(name, species)| (name.trim(), species.trim()))
            && film::words::plain(name) == plain
            && !species.is_empty()
            && !species.contains(' ')
        {
            return film::words::plain(species);
        }
    }
    let base = plain.trim_end_matches(|letter: char| letter.is_ascii_digit()).to_string();
    if session.tables().monsters.contains_key(&base) { base } else { "pikachu".to_string() }
}

/// What a word pointed at shows: someone (their species, their name in the game), a place (its word, its name).
pub(super) enum Pictured {
    Someone(String, String),
    Place(String, String),
}

/// The picture a word at that place has, if it is someone of the film, a Pokémon or a place.
pub(super) fn picture_of(session: &Session, text: &str, at: usize) -> Option<Pictured> {
    let word = word_at(text, at)?;
    let plain = film::words::plain(&word);
    if plain.chars().count() < 3 || film::is_word(&plain, text) {
        return None;
    }
    let tables = session.tables();
    let base = plain.trim_end_matches(|letter: char| letter.is_ascii_digit());
    if names_of(text).contains(&plain) || tables.monsters.contains_key(base) {
        let species = species_of(session, text, &plain);
        let name = tables.monsters.get(&species).map(|(_, name)| name.clone())?;
        let said = if species == base { name } else { format!("{plain} · {name}") };
        return Some(Pictured::Someone(species, said));
    }
    if let Some(level) = tables.levels.get(&plain) {
        return Some(Pictured::Place(plain.clone(), level.name.clone()));
    }
    tables.decors.contains_key(&plain).then(|| Pictured::Place(plain.clone(), format!("{plain} (the project's decor)")))
}

/// A species' portrait for a face word (`joyous`), read once (none of that face: its first).
pub(super) fn face_portrait(
    session: &Session,
    context: &egui::Context,
    kept: &mut HashMap<String, Option<egui::TextureHandle>>,
    species: &str,
    word: &str,
) -> Option<egui::TextureHandle> {
    let canonical = film::words::FACES.iter().find(|(said, _)| *said == word).map_or(word, |(_, face)| *face);
    let face = scenes::vocabulary::POKEMON.faces.iter().find(|(name, _)| *name == canonical).map_or(0, |(_, number)| *number);
    kept.entry(format!("{species}:{face}"))
        .or_insert_with(|| {
            let (number, _) = session.tables().monsters.get(species)?.clone();
            let folder = session.data();
            let path = [format!("portraits/{number}_{face}.png"), format!("portraits/{number}_0.png")]
                .into_iter()
                .map(|file| assets::data::path(folder, &file))
                .find(|path| std::path::Path::new(path).exists())?;
            let picture = image::open(path).ok()?.to_rgba8();
            let image = egui::ColorImage::from_rgba_unmultiplied([picture.width() as usize, picture.height() as usize], picture.as_raw());
            Some(context.load_texture(format!("portrait {species} {face}"), image, egui::TextureOptions::NEAREST))
        })
        .clone()
}

/// A species' portrait (its first face), read once.
pub(super) fn portrait(
    session: &Session,
    context: &egui::Context,
    kept: &mut HashMap<String, Option<egui::TextureHandle>>,
    species: &str,
) -> Option<egui::TextureHandle> {
    kept.entry(species.to_string())
        .or_insert_with(|| {
            let (number, _) = session.tables().monsters.get(species)?.clone();
            let path = assets::data::path(session.data(), &format!("portraits/{number}_0.png"));
            let picture = image::open(path).ok()?.to_rgba8();
            let image = egui::ColorImage::from_rgba_unmultiplied([picture.width() as usize, picture.height() as usize], picture.as_raw());
            Some(context.load_texture(format!("portrait {species}"), image, egui::TextureOptions::NEAREST))
        })
        .clone()
}
