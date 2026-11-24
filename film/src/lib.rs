//! The language of the films whole: its core (`film_core`, the text read,
//! the scenes compiled, what is written for the engine) and every family of
//! its words, gathered in their order (`LEXICON`). What a program asks of
//! the films' language it asks here: a film compiled, its words, the
//! language's service for editors.
//!
//! - the families: the core's own (`film_core::verbs`), and those of a crate
//!   each;
//! - `service`: what an editor asks of a film's text (problems, hovers,
//!   completions, the outline);
//! - `steps`: a film's text shown as it goes through the language, step by step.

#![forbid(unsafe_code)]

pub mod service;
pub mod steps;

use std::path::Path;

use engine::{Diagnostic, Lines, StoryEvent};

pub use film_core::{
    Context, Film, Lexicon, ReadAs, Told, Written, check, codes, coloured, context, doc, ds, emit, formatted, grammar, internal, ir, lines, locate, order,
    performer_first, plan, plot, scene_line, screen, shared_files, shared_path, shown_form, tables, time, words,
};

/// The families of the language's words, the narrower first: a line goes to
/// the first whose form of its word fits.
const FAMILIES: [film_core::verbs::Family; 26] = [
    film_core::plot::FAMILY,
    film_core::verbs::acting::FAMILY,
    film_core::verbs::game::FAMILY,
    film_fishing::FAMILY,
    film_dungeon::FAMILY,
    film_dungeon::guild::FAMILY,
    film_core::verbs::things::FAMILY,
    film_core::verbs::heard::FAMILY,
    film_core::verbs::view::FAMILY,
    film_core::verbs::sky::FAMILY,
    film_core::verbs::effects::FAMILY,
    film_fights::FAMILY,
    film_core::verbs::items::FAMILY,
    film_moving::swimming::FAMILY,
    film_moving::flying::FAMILY,
    film_moving::riding::FAMILY,
    film_moving::contact::FAMILY,
    film_moving::hiding::FAMILY,
    film_moving::FAMILY,
    film_moving::body::FAMILY,
    film_core::verbs::pictures::FAMILY,
    film_core::verbs::rituals::FAMILY,
    film_core::verbs::crowds::FAMILY,
    film_core::verbs::said::FAMILY,
    film_core::verbs::timing::FAMILY,
    film_core::verbs::nodes::FAMILY,
];

/// The language: every family of its words, in their order.
pub static LEXICON: Lexicon = Lexicon::new(&FAMILIES);

/// A film's text turned into lines, with these tables (its project's, its
/// data's, its language).
pub fn compile_with(text: &str, tables: std::sync::Arc<film_core::tables::Tables>) -> film_core::Written {
    film_core::compile::compile_with(&LEXICON, text, tables)
}

/// A scene the game makes of a place (`game::roam`), turned into lines with
/// those tables: the words only the game writes are known there.
pub fn compile_place_with(text: &str, tables: std::sync::Arc<film_core::tables::Tables>) -> film_core::Written {
    film_core::compile::compile_place_with(&LEXICON, text, tables)
}

/// Whether that is a word of the language (or a recipe the film writes).
pub fn is_word(word: &str, text: &str) -> bool {
    LEXICON.is_word(word, text)
}

/// Whether that word is said of someone first (`walk who:pokemon to where:place`).
pub fn takes_someone_first(word: &str, text: &str) -> bool {
    LEXICON.takes_someone_first(word, text)
}

/// Whether a line is written who first (`pikachu walk to right`).
pub fn subject_first(first: &str, second: Option<&str>, text: &str) -> bool {
    LEXICON.subject_first(first, second, text)
}

/// Every word of the films' language: its family, its word, its forms, what
/// it writes (`Lexicon::every_word`).
pub fn every_word() -> impl Iterator<Item = (&'static str, &'static str, &'static [&'static str], &'static str)> {
    LEXICON.every_word()
}

/// The engine's commands as words of the language: each its word, its forms, what it writes.
pub fn command_words() -> impl Iterator<Item = (&'static str, &'static [&'static str], &'static str)> {
    LEXICON.command_words()
}

/// The engine's commands a film may write (`Lexicon::shown_commands`).
pub fn shown_commands() -> impl Iterator<Item = (&'static str, &'static [&'static str], &'static str)> {
    LEXICON.shown_commands()
}

/// A line of that word, with what is given each name (`Lexicon::line_of`).
pub fn line_of(word: &str, given: &[(&str, &str)]) -> Result<String, String> {
    LEXICON.line_of(word, given)
}

/// The recipes a film has to call: its own, those of the files it uses, then the library's.
pub fn recipes_for(project: &assets::project::Project, text: &str) -> Vec<film_core::Told> {
    film_core::text::recipes::recipes_for(&LEXICON, project, text)
}

/// What the engine plays of a film or of the game's story (`story.txt`): its events, each
/// scene its beats, and what was wrong in reading them; a film compiled
/// with those tables.
pub fn read_any_with(
    path: &Path,
    words: &'static engine::Vocabulary,
    tables: std::sync::Arc<film_core::tables::Tables>,
) -> Result<(Vec<StoryEvent>, Vec<Diagnostic>), String> {
    if path.extension().is_some_and(|kind| kind == "film") {
        let written = film_core::Written::of_file_with(&LEXICON, path, tables)?;
        let mut problems = written.problems;
        problems.extend(written.engine.problems);
        return Ok((written.events, problems));
    }
    let (lines, problems) = lines_of(path, words, tables)?;
    Ok((engine::read_events(lines.into_iter().map(|(_, fields)| fields), words), problems))
}

/// The lines of a film or of the game's story, as the engine reads them, in
/// fields: to be written as a file of its own (`scene_player pack`).
pub fn lines_of(
    path: &Path,
    words: &'static engine::Vocabulary,
    tables: std::sync::Arc<film_core::tables::Tables>,
) -> Result<(Lines, Vec<Diagnostic>), String> {
    if path.extension().is_some_and(|kind| kind == "film") {
        let written = film_core::Written::of_file_with(&LEXICON, path, tables)?;
        let script = written.script();
        let mut problems = written.problems;
        problems.extend(script.problems);
        return Ok((script.lines, problems));
    }
    // (The game's story, `story.txt`.)
    story::lines_of(path, words)
}

#[cfg(test)]
mod tests;
