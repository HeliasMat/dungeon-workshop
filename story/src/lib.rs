//! The game's story as it was extracted from the ROM (`story.txt`, fields
//! between tabs), read into the lines the engine plays: the file is read
//! here, split into its fields by `engine::read_fields`, its short forms
//! written out by `lowering`. A film (`.film`) is `film`'s.

use std::path::Path;

use engine::{Diagnostic, Lines, StoryEvent, Vocabulary};

pub mod lowering;

/// The lines of a story kept as fields (`story.txt`), as the engine reads
/// them, and what is wrong.
pub fn lines_of(path: &Path, words: &'static Vocabulary) -> Result<(Lines, Vec<Diagnostic>), String> {
    if path.extension().is_some_and(|kind| kind == "film") {
        return Err(format!("{}: a film is read by film (scene_player plays it)", path.display()));
    }
    if path.extension().is_some_and(|kind| kind == "scene") {
        return Err(format!("{}: `.scene` files are no longer read: write a film (`.film`)", path.display()));
    }
    // (The engine's lines as the game's story is kept, `story.txt`.)
    let text = std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let script = lowering::read_text(&text, path.parent(), words);
    Ok((script.lines, script.problems))
}

/// The events of a story kept as fields (`story.txt`), and what is wrong.
pub fn read_events(path: &Path, words: &'static Vocabulary) -> Result<(Vec<StoryEvent>, Vec<Diagnostic>), String> {
    let (lines, problems) = lines_of(path, words)?;
    Ok((engine::read_events(lines.into_iter().map(|(_, fields)| fields), words), problems))
}
