//! The language of the films whole: its core (`film_core`, the text read,
//! the scenes compiled, what is written for the engine) and every group of
//! its words, gathered in their order (`LEXICON`). What a program asks of
//! the films' language it asks here: a film compiled, its words, the
//! language's service for editors.
//!
//! - the groups: the core's own (`film_core::verbs`), and those of a crate
//!   each;
//! - `service`: what an editor asks of a film's text (problems, hovers,
//!   completions, the outline);
//! - `steps`: a film's text shown as it goes through the language, step by step.

pub mod service;
pub mod steps;

use std::path::Path;

use engine::{Diagnostic, Lines, StoryEvent};

pub use film_core::{
    Context, Film, Lexicon, ReadAs, Told, Written, check, codes, coloured, context, doc, emit, formatted, grammar, internal, ir, lines, locate, order,
    performer_first, plot, scene_line, screen, shared_files, shared_path, tables, time, words,
};

/// The vocabulary of the language's words (`film_core::vocabulary`), each
/// in its family, as `LEXICON` declares them.
pub mod vocabulary {
    pub use film_core::vocabulary::*;

    use super::LEXICON;

    /// Every word of the vocabulary, in the order of the families.
    pub fn words() -> &'static [Planned] {
        LEXICON.vocabulary()
    }

    /// The word of the vocabulary of that name, if one.
    pub fn planned(word: &str) -> Option<&'static Planned> {
        LEXICON.planned(word)
    }

    /// The family a word is shown in (`Lexicon::family_of`).
    pub fn family_of(word: &str) -> &'static str {
        LEXICON.family_of(word)
    }

    /// The form a word of the old vocabulary is written in now (`Lexicon::replaced`).
    pub fn replaced(word: &str) -> Option<&'static str> {
        LEXICON.replaced(word)
    }

    /// The recipes and commands of `OTHERS` the language does not have.
    pub fn pending(known: &dyn Fn(&str) -> bool) -> Vec<&'static Planned> {
        LEXICON.pending(known)
    }
}

/// What each word becomes on the DS (`film_core::ds`), as `LEXICON`'s
/// words declare it.
pub mod ds {
    pub use film_core::ds::*;

    use super::LEXICON;

    /// How that word stands on the DS, if it is a word of the vocabulary.
    pub fn on_ds(word: &str) -> Option<OnDs> {
        LEXICON.on_ds(word)
    }

    /// How that word exports, if it is a word of the vocabulary.
    pub fn export_of(word: &str) -> Option<Export> {
        LEXICON.export_of(word)
    }

    /// How any word stands on the DS (none: an engine's command).
    pub fn status(word: &str) -> Export {
        LEXICON.ds_status(word)
    }

    /// What it is written as on the DS: the game's operations.
    pub fn how(word: &str) -> Option<&'static str> {
        on_ds(word).map(|ds| ds.how)
    }

    /// What a word written with less loses on the DS.
    pub fn lost(word: &str) -> Option<&'static str> {
        on_ds(word).and_then(|ds| ds.lost)
    }

    /// What a film `for ds` is told of a word that doesn't export.
    pub fn refused(word: &str) -> String {
        LEXICON.ds_refused(word)
    }
}

/// A form as an author writes it, who first if who acts (`Lexicon::shown_form`).
pub fn shown_form(word: &str, form: &str) -> String {
    LEXICON.shown_form(word, form)
}

/// The groups of the language's words, the narrower first: a line goes to
/// the first whose form of its word fits.
const GROUPS: [film_core::verbs::Group; 26] = [
    film_core::plot::GROUP,
    film_acting::GROUP,
    film_core::verbs::game::GROUP,
    film_fishing::GROUP,
    film_dungeon::GROUP,
    film_dungeon::guild::GROUP,
    film_core::verbs::things::GROUP,
    film_core::verbs::heard::GROUP,
    film_core::verbs::view::GROUP,
    film_core::verbs::sky::GROUP,
    film_core::verbs::effects::GROUP,
    film_fights::GROUP,
    film_core::verbs::items::GROUP,
    film_moving::swimming::GROUP,
    film_moving::flying::GROUP,
    film_moving::riding::GROUP,
    film_moving::contact::GROUP,
    film_moving::hiding::GROUP,
    film_moving::GROUP,
    film_moving::body::GROUP,
    film_core::verbs::pictures::GROUP,
    film_core::verbs::rituals::GROUP,
    film_core::verbs::crowds::GROUP,
    film_core::verbs::said::GROUP,
    film_core::verbs::timing::GROUP,
    film_core::verbs::nodes::GROUP,
];

/// The language: every group of its words, in their order.
pub static LEXICON: Lexicon = Lexicon::new(&GROUPS);

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

/// Every word of the films' language: its group, its word, its forms, what
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
