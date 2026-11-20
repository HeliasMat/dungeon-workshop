//! What is wrong in a film, as it reads: what its words said, then what the
//! engine's lines do, with the sizes of the game's places.

use engine::{Checker, Diagnostic};
use scenes::commands::Command;
use scenes::vocabulary::POKEMON;

/// The first word of a line, as the language compares them.
pub fn first_word(line: &str) -> String {
    film::words::plain(line.split_whitespace().next().unwrap_or_default())
}

/// The engine's checker, with the sizes of the places of the game's files.
pub fn checker(data: &str) -> Checker {
    let sizes = assets::tables::backdrops(data).into_iter().map(|backdrop| (backdrop.level, (backdrop.width as f32, backdrop.height as f32)));
    Checker::new(&POKEMON).with_sizes(sizes.collect()).with_host(|fields| Command::parse(fields).is_some())
}

/// All that is wrong in a film's words, at once: what they said and what
/// the lines they make do.
pub fn read_through(written: &film::Written, data: &str) -> Vec<Diagnostic> {
    let mut problems = written.problems.clone();
    problems.extend(checker(data).check_script(written.script()));
    problems
}
