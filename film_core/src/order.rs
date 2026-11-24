//! One word order: who acts comes first. A line of a word said of someone
//! starts with him, as a line said does (`pikachu walk to piplup`, like
//! `pikachu (happy): Hi`); a line where nobody acts starts with its word
//! (`music beach`, `wait 2s`, `frame pikachu piplup`).
//!
//! Which words are said of someone is read from their forms, never listed:
//! a form whose first part is one performer (`who:pokemon`, `who:code`, a
//! recipe's `a:pokemon`) or a team of one (`*who`). A line written the old
//! way, the word first (`walk pikachu to piplup`), is wrong
//! ([`crate::codes::OLD_ORDER`]), its fix writing it who first.

use crate::film::Film;
use crate::grammar::{Piece, performer_first};
use crate::words::plain;

/// The old order, written as it is now: the word's line `walk pikachu to b`,
/// who acts its second word, written `pikachu walk to b`. Told wrong with
/// its fix, if the line is the film's own as written there (`written`:
/// its words, each where it is), its first word that word, its second
/// someone of the film and no word of the language.
pub(crate) fn told_who_first(film: &mut Film, number: usize, written: &[(String, language::Span)], verb: &str) {
    let [(word, word_at), (who, who_at), ..] = written else { return };
    // (A word the vocabulary writes word first, nobody acting in it, `fight a vs b`: as it is.)
    if film.lexicon.who_first(verb) == Some(false) || plain(word) != verb || film.actor(who).is_none() || film.lexicon.is_word(who, &film.source) {
        return;
    }
    let rest: Vec<&str> = written[2..].iter().map(|(said, _)| said.as_str()).collect();
    let now = std::iter::once(format!("{who} {word}")).chain(rest.iter().map(|said| said.to_string())).collect::<Vec<_>>().join(" ");
    let before = written.iter().map(|(said, _)| said.as_str()).collect::<Vec<_>>().join(" ");
    let span = language::Span::new(word_at.start, who_at.end);
    let message = format!("“{before}”: who acts comes first, as in a line said: write “{now}”");
    let problem = engine::Diagnostic::at(number, message).on(Some((span.start, span.end))).coded(crate::codes::OLD_ORDER).fix(
        format!("write “{who} {word}”"),
        span,
        format!("{who} {word}"),
    );
    film.problems.push(problem);
}

/// Whether a line of a word is said of someone, by the first of that
/// family's forms that fits it: its first part one who acts (a team given
/// only one).
pub(crate) fn said_of_someone(film: &mut Film, family: &str, verb: &str, rest: &[String]) -> bool {
    let Some(word) = film.lexicon.word_of_group(family, verb) else { return false };
    for form in word.forms {
        let pieces = crate::grammar::read(form);
        let Ok(bound) = crate::grammar::bind(film, &pieces, rest) else { continue };
        return match pieces.first() {
            Some(Piece::Team(name)) => bound.words(name).len() == 1,
            _ => performer_first(&pieces),
        };
    }
    false
}
