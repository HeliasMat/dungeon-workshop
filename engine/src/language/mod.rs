//! The engine's lines: fields, each line one beat. The engine reads no
//! language: the films are read by the language (`language`, the `film`
//! crate), which gives it these lines; only the game's story,
//! drawn from the ROM, is kept as a file of fields between tabs
//! (`story.txt`), read here.

use crate::vocabulary::Vocabulary;
use crate::who::Who;
use diagnostics::Diagnostic;

mod include;

pub use include::read_fields;

/// Who says a line (none: a narration, written `-`); an unseen spot says
/// nothing.
pub fn speaker(code: Option<&String>) -> Option<Who> {
    Who::parse(code?).filter(|who| !matches!(who, Who::Spot(_)))
}

/// A file's lines, each with its number and its fields.
pub type Lines = Vec<(usize, Vec<String>)>;

/// Lines as the engine plays them, each with its number, and what was
/// wrong on the way to them.
#[derive(Clone, Default)]
pub struct Script {
    pub lines: Lines,
    pub problems: Vec<Diagnostic>,
}

/// A file of the engine's lines (fields between tabs: `story.txt`), as the
/// engine reads it; `folder`: where its pictures are.
pub fn read_scene(text: &str, folder: Option<&std::path::Path>, words: &'static Vocabulary) -> Script {
    let mut script = Script::default();
    script.lines = read_fields(text, folder, &mut script.problems, words);
    script
}
