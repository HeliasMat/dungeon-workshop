//! The game's story's lines written out, and the passes that write out what
//! lines in the engine's words write short.

pub mod passes;

use engine::{Script, Vocabulary};

/// A text of fields between tabs (`story.txt`), its short forms written out.
pub fn read_text(text: &str, folder: Option<&std::path::Path>, words: &'static Vocabulary) -> Script {
    let mut problems = Vec::new();
    let lines = engine::read_fields(text, folder, &mut problems, words);
    passes::written_out(lines, problems, words)
}
