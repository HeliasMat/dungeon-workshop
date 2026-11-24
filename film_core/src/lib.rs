//! The language of the films, whatever its words: a film written in plain
//! English turned into what the engine plays, by the groups of words it is
//! given (`Lexicon`; `film` gathers them all).
//!
//! - `text`: the text read by the language, its recipes played out, who each
//!   scene names;
//! - `compile`: a film's scenes, and what is said in them;
//! - `verbs`: what a word is (`Word`, `Group`, `Lexicon`), a line played
//!   by the group whose form fits; the groups the language leans on, a
//!   file by theme;
//! - `names`: what a film calls those it shows, things, places, ways;
//! - `spots`: where a line says, routes, formations;
//! - `tables`: the game's files as a film reads them;
//! - `ds`: what each word becomes on the DS (the export's table);
//! - `words`, `screen`, `check`: the small words, words on the screen, what
//!   is wrong and the nearest names.
//!
//! Only what the words say is known here. What depends on the scene as it
//! stands is written as such (`Spot`, `PlaceBeat`) and found on the stage.

pub mod check;
pub mod codes;
pub mod compile;
pub mod context;
pub mod doc;
pub mod ds;
pub mod emit;
pub mod film;
pub mod grammar;
pub mod ir;
pub mod lines;
pub mod names;
pub mod order;
pub mod plot;
pub mod screen;
pub mod spots;
pub mod tables;
pub mod temps;
pub mod text;
pub mod time;
pub mod verbs;
pub mod vocabulary;
pub mod words;

use std::path::{Path, PathBuf};

use engine::{Diagnostic, Script, StoryEvent};

pub use compile::{ReadAs, Written};
pub use context::Context;
pub use film::Film;
pub use lines::formatted;
pub use order::{performer_first, shown_form};
pub use screen::coloured;
pub use text::recipes::{Told, shared_files, shared_path};
pub use verbs::{Group, Lexicon, Word, internal, scene_line};

/// A film by a path, or a name of the project's films (the shared
/// library's: `library/films`), else of a game's `films/`
/// (`games/<game>/films`, the games in order).
pub fn locate(project: &assets::project::Project, name: &str) -> Option<PathBuf> {
    let folder = project.films();
    let wanted = [PathBuf::from(name), PathBuf::from(format!("{name}.film")), folder.join(name), folder.join(format!("{name}.film"))];
    wanted.into_iter().find(|path| path.is_file()).or_else(|| {
        let file = format!("{}.film", name.trim_end_matches(".film"));
        let games = project.game_folders();
        games.into_iter().map(|game| game.join("films").join(&file)).find(|path| !name.contains(['/', '\\']) && path.is_file())
    })
}

impl Written {
    /// A film's file turned into lines, with those words and those tables
    /// (its project's).
    pub fn of_file_with(lexicon: &'static Lexicon, path: &Path, tables: std::sync::Arc<tables::Tables>) -> Result<Written, String> {
        let text = std::fs::read_to_string(path).map_err(|error| format!("{} : {error}", path.display()))?;
        Ok(compile::compile_with(lexicon, &text, tables))
    }

    /// As the engine reads it.
    pub fn script(&self) -> Script {
        self.engine.clone()
    }

    /// What the engine plays: the film's events, each scene its beats, and
    /// what was wrong in reading them.
    pub fn events(&self) -> (Vec<StoryEvent>, Vec<Diagnostic>) {
        (self.events.clone(), self.engine.problems.clone())
    }

    /// As a scene file, to be sent or looked at.
    pub fn text(&self) -> String {
        let mut text = self.head.clone();
        for (_, fields) in &self.lines {
            text.push_str(&fields.join("\t"));
            text.push('\n');
        }
        text
    }

    /// What the engine says, those it names (typed: `engine::Mention`) said
    /// by the names the film gives them; the others as it wrote them.
    pub fn by_name(&self, text: &str, mentions: &[engine::Mention]) -> String {
        engine::Said { text: text.to_string(), mentions: mentions.to_vec() }.by(&|mention| self.name_of(mentioned(mention)))
    }

    /// A problem as the engine tells it, those it names said by the film's names.
    pub fn told_by_name(&self, problem: &Diagnostic) -> String {
        let message = problem.message_by(&|mention| self.name_of(mentioned(mention)));
        Diagnostic { message, mentions: Vec::new(), ..problem.clone() }.to_string()
    }

    /// The name the film gives someone, if it gives one.
    pub fn name_of(&self, who: engine::Who) -> Option<String> {
        let code = who.to_string();
        self.names.iter().find(|(known, _)| *known == code).map(|(_, name)| name.clone())
    }
}

/// Who the engine names.
fn mentioned(mention: &engine::Mention) -> engine::Who {
    match mention.copy {
        0 => engine::Who::Monster(mention.species),
        copy => engine::Who::Another { species: mention.species, copy },
    }
}
