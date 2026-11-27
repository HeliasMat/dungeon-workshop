//! What is told of a line of the film's own text as it is written, before
//! it plays: a word of the old vocabulary (its fix the new form), a family's
//! word whose library is not taken in (`uses fishing`), and the word order.
//!
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
use crate::vocabulary::{library_of, literal_head};
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

/// A line of a word of the old vocabulary (`fadein 30`, `turn pikachu
/// left`): refused, with the form of the vocabulary that replaces it, and a fix
/// when the line can be written so word for word (its first words put for
/// the new ones, who first when the new form is).
pub(crate) fn told_replaced(film: &mut crate::film::Film, number: usize, written: &[(String, language::Span)], verb: &str) -> bool {
    let Some(now) = film.lexicon.replaced(verb) else { return false };
    let Some((first, first_at)) = written.first() else { return false };
    if crate::words::plain(first) != verb {
        // (Who first, `pikachu turn left`: the word is second.)
        if written.get(1).is_none_or(|(second, _)| crate::words::plain(second) != verb) {
            return false;
        }
    }
    let before = written.iter().map(|(said, _)| said.as_str()).collect::<Vec<_>>().join(" ");
    if now.is_empty() {
        let message = format!("“{verb}” is no word of the language any more: write the line another way (film/FILMS.md)");
        film.problems.push(engine::Diagnostic::at(number, message).on(Some((first_at.start, first_at.end))).coded(crate::codes::OLD_WORD));
        return true;
    }
    let (who_first, head) = literal_head(now);
    let word_first = crate::words::plain(first) == verb;
    let rest: Vec<&str> = written.iter().skip(1).map(|(said, _)| said.as_str()).collect();
    // (The one who acts: second when the old line was written word first.)
    let (who, after): (Option<&str>, &[&str]) = match (who_first, word_first, rest.split_first()) {
        (true, true, Some((who, after))) if film.actor(who).is_some() => (Some(who), after),
        (true, false, _) => (Some(first.as_str()), &rest[1..]),
        (false, true, _) => (None, &rest[..]),
        _ => (None, &[]),
    };
    // (Word for word only: no line said, `who think: text`; the form's values given, if it has any.)
    // (Its other values as many as the old line's other words, at most; those not optional, at least.)
    let tail: Vec<&str> = now.split_whitespace().skip(head.len() + usize::from(who_first)).collect();
    let (mut required, mut inside) = (0, 0);
    for value in &tail {
        if inside == 0 && !value.starts_with('[') {
            required += 1;
        }
        inside += value.matches('[').count();
        inside -= value.matches(']').count().min(inside);
    }
    let fits = after.len() <= tail.len() && after.len() >= required;
    let fixed = (!head.is_empty() && (!who_first || who.is_some()) && !now.contains(':') && fits)
        .then(|| who.into_iter().chain(head.iter().copied()).chain(after.iter().copied()).collect::<Vec<_>>().join(" "));
    let span = written.last().map_or(*first_at, |(_, last)| language::Span::new(first_at.start, last.end));
    let fixed = fixed.filter(|fixed| *fixed != before);
    let message = match &fixed {
        Some(fixed) => format!("“{verb}” is no word of the language any more: “{now}” says it now; write “{fixed}”"),
        None => format!("“{verb}” is no word of the language any more: “{now}” says it now"),
    };
    let mut problem = engine::Diagnostic::at(number, message).on(Some((span.start, span.end))).coded(crate::codes::OLD_WORD);
    if let Some(fixed) = fixed {
        problem = problem.fix(format!("write “{fixed}”"), span, fixed);
    }
    film.problems.push(problem);
    true
}

/// A word of a recipe file of the user's library (`library/recipes/`) said
/// in a film that does not use it: told, the `uses` line its fix. Whether
/// it was one.
pub(crate) fn asks_own_recipe(film: &mut crate::film::Film, line: &crate::verbs::Line) -> bool {
    let project = film.tables.project.clone();
    let said = std::iter::once(line.verb.as_str()).chain(line.rest.first().map(String::as_str));
    let Some((word, file)) = said.map(crate::words::plain).find_map(|word| crate::text::recipes::own_recipe_file_of(&project, &word).map(|(file, _)| (word, file)))
    else {
        return false;
    };
    let message = format!("“{word}” is a word of your library ({file}): “uses {file}” before the first scene");
    let problem = engine::Diagnostic::at(line.number, message).fix(format!("write “uses {file}”"), language::Span::new(0, 0), format!("uses {file}\n"));
    film.problems.push(problem);
    true
}

/// A word of an optional family said in a film that does not take its
/// library in (`bite` with no `uses fishing`): told, with the line that
/// takes it in as its fix (written first in the film).
pub(crate) fn asks_its_library(film: &mut crate::film::Film, number: usize, verb: &str) {
    let Some(library) = film.lexicon.planned(verb).and_then(|planned| library_of(planned.family)) else { return };
    if film.uses.iter().any(|used| used == library) {
        return;
    }
    let family = film.lexicon.planned(verb).map_or("", |planned| planned.family);
    let message = format!("“{verb}” is a word of the {family} family: “uses {library}” before the first scene");
    let problem = engine::Diagnostic::at(number, message).as_remark().coded(crate::codes::USES).fix(
        format!("write “uses {library}”"),
        language::Span::new(0, 0),
        format!("uses {library}\n"),
    );
    film.problems.push(problem);
}
