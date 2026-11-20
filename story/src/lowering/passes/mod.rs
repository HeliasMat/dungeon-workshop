//! The game's story's lines (`story.txt`) written out as the engine plays
//! them: sequences played where they are called (`define` … `enddef`,
//! `do`, the `d` of a line said, `each`), groups and names put in
//! (`group`, `cast`, `@spot`), what is written short written out (`once`,
//! `switch`, `when`, `repeat`, `line`, `narrate`, `chat`…), paths that
//! cross told. Then each line is written again as the one beat it is
//! (`Beat::written`), so what the engine gets reads no language of its own.
//! `lowering::read_text` calls them.

use std::collections::{HashMap, HashSet};

use engine::{Beat, Diagnostic, Lines, Script, Vocabulary};

mod crossings;
mod names;
mod sequences;
mod shorthands;
#[cfg(test)]
mod tests;

use crossings::crossings;
use names::{groups, names};
use sequences::sequences;
use shorthands::shorthands;

/// Lines of fields, their short forms written out, each then written as
/// its beat where it is one.
pub fn written_out(lines: Lines, problems: Vec<Diagnostic>, words: &'static Vocabulary) -> Script {
    let Script { lines, problems } = passes(lines, problems, words);
    let lines = lines.into_iter().map(|(number, fields)| (number, as_beat(fields, words))).collect();
    Script { lines, problems }
}

/// The short forms written out, the lines as the passes leave them.
fn passes(lines: Lines, mut problems: Vec<Diagnostic>, words: &'static Vocabulary) -> Script {
    let lines = sequences(lines, &mut problems);
    let lines = groups(lines, words);
    let lines = shorthands(lines, &mut problems, words);
    let lines = names(lines, &mut problems, words);
    let lines = crossings(lines, &mut problems);
    Script { lines, problems }
}

/// A line of the engine's, written as the beat it reads as: the same beat,
/// in its own words. A line that is no one beat (a block's `if`, `endif`,
/// a line the engine refuses, one filled as it plays) as it was, for the engine to read or tell.
fn as_beat(fields: Vec<String>, words: &'static Vocabulary) -> Vec<String> {
    // (A line the engine fills as it plays, `put roc {x} {y}`: kept for then.)
    if engine::played_filled(&fields) {
        return fields;
    }
    match Beat::parse(&fields, words) {
        Ok(beat) => beat.written(words).unwrap_or(fields),
        Err(_) => fields,
    }
}

/// A text of fields between tabs, its short forms written out (the passes
/// alone, for their tests).
#[cfg(test)]
fn read_text(text: &str, folder: Option<&std::path::Path>, words: &'static Vocabulary) -> Script {
    let mut problems = Vec::new();
    let lines = engine::read_fields(text, folder, &mut problems, words);
    passes(lines, problems, words)
}

/// One line of the engine's words written out if it is a short form
/// (`narrate`, `cheer`, `bolt`, `each`, `waitfor a b`…): the lines it
/// gives; none if it is no short form (the line as it is).
pub fn written_short(number: usize, fields: Vec<String>, problems: &mut Vec<Diagnostic>, words: &'static Vocabulary) -> Option<Lines> {
    let line = vec![(number, fields)];
    let out = shorthands(line.clone(), problems, words);
    (out != line).then_some(out)
}
