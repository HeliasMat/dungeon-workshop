//! Each recipe checked once, alone, where it is written: the film's own
//! (`define`), those of a file taken in, those of the files it uses. Its
//! lines are written as for a call (`front::called`), each name standing for
//! a value of its kind, then each word's line is read against its forms:
//! the word must be known, and one of its forms must fit.
//!
//! What each name stands for: someone (`who:pokemon`, `*who`, `where:place`)
//! by the name itself, made someone of the scene; a duration `30f`; a
//! number `2`; a property `opacity`; a word (`who`, `who:word`, `thing`) a
//! word that could be anything (`grammar::any`); words (`rest...`) as many
//! of those as a form wants. A recipe with names it may be left out is
//! written twice: all of them given, then none of those (`if other`, `if
//! not other`, their defaults).
//!
//! What is wrong is told once, on the recipe's own line: of the film, or
//! of the file taken in; for a file it uses, on the `uses` line, with the
//! file and its line (“in fishing/line.film, line 12: …”).

use std::collections::HashMap;

use super::front::{CallOf, CallValue, called};
use super::recipes::{INCLUDED, Recipe, library};
use crate::film::Film;
use crate::grammar::{self, Kind, Piece, any};
use crate::ir::{Instr, Shape};
use crate::words::{frames_of, plain};

/// A recipe the film writes or uses, and the `uses` line that takes it in
/// (none: written in the film, or in a file it takes in).
#[derive(Clone, Debug)]
pub struct Defined {
    pub recipe: Recipe,
    pub uses: Option<usize>,
}

/// The recipes a recipe's lines may call, by name.
pub type Known = HashMap<String, Vec<Recipe>>;

/// How many words a recipe's `rest...` may stand for, at most, in a line.
const MOST_WORDS: usize = 4;

/// The recipes the film writes and uses, each checked alone: what is wrong
/// in them told where they are written.
pub fn check(film: &mut Film) {
    let defined = std::mem::take(&mut film.defined);
    if defined.is_empty() {
        return;
    }
    let mut known = Known::new();
    for recipe in defined.iter().map(|one| &one.recipe).chain(library()) {
        known.entry(recipe.name.clone()).or_default().push(recipe.clone());
    }
    for one in &defined {
        // (Those the film names, its groups: known as the recipe is read.)
        let mut alone = Film::with(film.tables.clone());
        alone.actors = film.actors.clone();
        alone.groups = film.groups.clone();
        for (line, message) in wrong_in(&mut alone, &one.recipe, &known) {
            told(film, one, line, message);
        }
    }
    film.defined = defined;
}

/// What is wrong in a recipe read alone, each on its own line in its text.
pub fn wrong_in(alone: &mut Film, recipe: &Recipe, known: &Known) -> Vec<(usize, String)> {
    let mut found: Vec<(usize, String)> = Vec::new();
    let source = language::Source::new(&recipe.text.text);
    let leaves_out = left_out(&recipe.form);
    for given in [true, false].into_iter().take(if leaves_out { 2 } else { 1 }) {
        let mut values = Vec::new();
        standing(alone, &recipe.form, given, false, &mut values);
        let call = CallOf { id: 0, number: 0, values, file: recipe.file.clone(), apart: true };
        let read = called(&recipe.text, call, &alone.tables.language);
        for (_, wrong) in read.diagnostics.into_iter().filter(|(_, wrong)| wrong.severity == language::Severity::Error) {
            found.push((source.position(wrong.bytes().start).line, wrong.message));
        }
        let mut seen = Vec::new();
        for line in &read.lines {
            walk(alone, line, known, &source, &mut seen, &mut found);
        }
    }
    // (What its call writes first, a place found: on its first line.)
    let first = source.position(recipe.text.span.start).line;
    found.extend(literals_as_names(recipe, &source, first, known));
    for (line, _) in &mut found {
        *line = (*line).max(first);
    }
    let mut seen = std::collections::HashSet::new();
    found.retain(|one| seen.insert(one.clone()));
    found.sort_by_key(|(line, _)| *line);
    found
}

/// The words its first line writes as they are (`define joy who`: the word
/// “who” itself) that its lines use as a name: in a reckoning (`{who}`), or
/// as a word of a line whose word writes no such word itself (`emote who
/// joy`; not `walk who to there`, `to` being walk's). Its value is never
/// given: the author meant `who:word`.
fn literals_as_names(recipe: &Recipe, source: &language::Source, first: usize, known: &Known) -> Vec<(usize, String)> {
    // (A word that may be left out, or one of a choice, is a name: `if miss`.)
    let mut literals: Vec<String> = recipe
        .form
        .iter()
        .filter_map(|piece| match piece {
            Piece::Choice(words) if words.len() == 1 => Some(plain(&words[0])),
            _ => None,
        })
        .collect();
    literals.retain(|word| word.len() > 1 && word.chars().all(|letter| letter.is_ascii_alphanumeric() || letter == '_'));
    let last = source.position(recipe.text.span.end).line.min(source.lines());
    let mut found = Vec::new();
    for word in literals {
        for line in first + 1..=last {
            let text = source.line_text(line);
            let code = text.split('#').next().unwrap_or("");
            let braced = code
                .split('{')
                .skip(1)
                .filter_map(|after| after.split_once('}').map(|(inside, _)| inside))
                .any(|inside| inside.split(|letter: char| !(letter.is_ascii_alphanumeric() || letter == '_')).any(|part| part == word));
            let parts: Vec<String> = code.split_whitespace().map(|part| plain(part.trim_end_matches([',', ':', '&']))).collect();
            // (Its word first, or second when who acts is written first.)
            let verb_at = usize::from(parts.first().is_some_and(|first| !crate::verbs::is_word(first, "") && !known.contains_key(first.as_str())));
            let verb = parts.get(verb_at).cloned().unwrap_or_default();
            let bare = parts.iter().enumerate().any(|(at, part)| at != verb_at && *part == word) && !verb_literals(&verb, known).contains(&word);
            if braced || bare {
                found.push((
                    line,
                    format!("“{word}” is written as it is in the recipe's first line, yet its lines use it as a name: to give it a value, write “{word}:word” (or “{word}:pokemon”, “{word}:number”…)"),
                ));
                break;
            }
        }
    }
    found
}

/// The words a word's forms write as they are (or a recipe's of that name).
fn verb_literals(verb: &str, known: &Known) -> Vec<String> {
    let mut out = Vec::new();
    let forms = crate::verbs::all_words().filter(|(_, word, ..)| *word == verb).flat_map(|(_, _, forms, _)| forms.iter());
    let commands = crate::verbs::command_words().filter(|(word, ..)| *word == verb).flat_map(|(_, forms, _)| forms.iter());
    for form in forms.chain(commands) {
        grammar::literal_words(&grammar::read(form), &mut out);
    }
    for recipe in known.get(verb).into_iter().flatten() {
        grammar::literal_words(&recipe.form, &mut out);
    }
    out
}

/// Whether some of a form's names may be left out.
fn left_out(form: &[Piece]) -> bool {
    form.iter().any(|piece| matches!(piece, Piece::Maybe(_) | Piece::One { default: Some(_), .. } | Piece::Words { default: Some(_), .. }))
}

/// What each name of a form stands for: a value of its kind; those that
/// may be left out (`optional`, or with a default), left out unless `given`.
fn standing(alone: &mut Film, form: &[Piece], given: bool, optional: bool, out: &mut Vec<CallValue>) {
    for piece in form {
        let (name, words) = match piece {
            Piece::One { name, kind, default } => {
                let words = match default {
                    _ if given || !(optional || default.is_some()) => of_kind(alone, name, *kind),
                    Some(default) if *kind == Kind::Time => {
                        default.iter().map(|word| frames_of(word).map_or_else(|| word.clone(), |frames| format!("{frames}f"))).collect()
                    }
                    Some(default) => default.clone(),
                    None => Vec::new(),
                };
                (name, words)
            }
            Piece::Team(name) => (name, if given || !optional { vec![someone(alone, name), "partner".to_string()] } else { Vec::new() }),
            Piece::Words { name, kind, default, .. } => {
                let said = given || !(optional || default.is_some());
                let words = match default {
                    _ if said && *kind == Kind::Code => vec![someone(alone, name)],
                    _ if said => vec![any(&format!("{name}..."))],
                    Some(default) => default.clone(),
                    None => Vec::new(),
                };
                (name, words)
            }
            Piece::Maybe(inner) => {
                standing(alone, inner, given, true, out);
                continue;
            }
            // (A word that may be left out: said when the others are given,
            // for `if miss` to be read both ways.)
            Piece::Choice(words) if optional && given => {
                let word = plain(&words[0]);
                out.push(CallValue { name: word.clone(), shown: word.clone(), words: vec![word] });
                continue;
            }
            Piece::Choice(_) => continue,
        };
        out.push(CallValue { name: name.clone(), shown: words.join(" "), words });
    }
}

/// A value of that kind, for the name it stands for.
fn of_kind(alone: &mut Film, name: &str, kind: Kind) -> Vec<String> {
    match kind {
        Kind::Someone | Kind::Code | Kind::Place => vec![someone(alone, name)],
        Kind::Time => vec!["30f".to_string()],
        Kind::Number | Kind::Coordinate => vec!["2".to_string()],
        Kind::Property => vec!["opacity".to_string()],
        Kind::Word | Kind::Thing => vec![any(name)],
    }
}

/// Someone of the scene by the name he stands for (`who`), if it can be one;
/// else the hero.
fn someone(alone: &mut Film, name: &str) -> String {
    let key = plain(name);
    if key.bytes().any(|letter| letter.is_ascii_lowercase()) && !key.contains(['.', '{', '=']) {
        if alone.actor(&key).is_none() {
            alone.give_name(key.clone(), "0".to_string());
        }
        return key;
    }
    "hero".to_string()
}

/// Each word's line of the lines written, and of the blocks they hold: what
/// is wrong with it, on its own line. The words of the lines before (`seen`)
/// may be someone a line brought on: a bobber (`bobber who`), a thing put
/// down (`place_thing crosshair …`), named so as the film plays.
fn walk(alone: &mut Film, line: &Instr, known: &Known, source: &language::Source, seen: &mut Vec<String>, found: &mut Vec<(usize, String)>) {
    match &line.shape {
        Shape::Word { verb, words, .. } => {
            let mut wrong = wrong_line(alone, line, verb, words, known, source);
            let brought: Vec<String> = words.iter().map(|word| plain(word)).filter(|word| seen.contains(word)).collect();
            if wrong.is_some() && brought.iter().any(|word| alone.actor(word).is_none()) {
                for word in brought {
                    if alone.actor(&word).is_none() {
                        alone.give_name(word, "0".to_string());
                    }
                }
                wrong = wrong_line(alone, line, verb, words, known, source);
            }
            if let Some(wrong) = wrong {
                found.push((line.number, wrong));
            }
            seen.extend(std::iter::once(verb.clone()).chain(words.iter().map(|word| plain(word))));
        }
        Shape::Arrives { then: Some(then), .. } | Shape::AtMark { then: Some(then), .. } => walk(alone, then, known, source, seen, found),
        Shape::WhenLife { lines, .. } => {
            for line in lines {
                walk(alone, line, known, source, seen, found);
            }
        }
        _ => {}
    }
    for inner in line.inner.iter().flatten() {
        walk(alone, inner, known, source, seen, found);
    }
}

/// What is wrong with a word's line a recipe writes: its word not known, or
/// none of its forms (or of the recipes of that name) fitting.
fn wrong_line(alone: &mut Film, line: &Instr, verb: &str, words: &[String], known: &Known, source: &language::Source) -> Option<String> {
    if matches!(verb, "end" | "else" | "uses" | "include" | "for") {
        return None;
    }
    let first = line.text.split_whitespace().next().filter(|first| plain(first) == verb).unwrap_or(verb).to_string();
    // (As the line is written in the recipe, to be found there.)
    let text = if line.number == 0 { String::new() } else { source.line_text(line.number).trim().to_string() };
    let text = if text.is_empty() { line.text.clone() } else { text };
    let mut words = stood_for(words);
    // (`… & as wind`: beside the film, named.)
    if words.len() >= 3 && words[words.len() - 3] == "&" && plain(&words[words.len() - 2]) == "as" {
        words.truncate(words.len() - 3);
    }
    let tries = spread(&words);
    match known.get(verb) {
        Some(recipes) => {
            let mut why = Vec::new();
            for words in &tries {
                for recipe in recipes {
                    match grammar::bind(alone, &recipe.form, words) {
                        Ok(_) => return None,
                        Err(reason) if why.len() < recipes.len() => why.push(format!("“{}”: {reason}", recipe.shape().trim_end())),
                        Err(_) => {}
                    }
                }
            }
            Some(format!("“{text}”: {}", why.join("; ")))
        }
        None => {
            let mut wrong = None;
            for words in &tries {
                // (One way that fits: nothing wrong.)
                let said = crate::verbs::wrong_form(alone, verb, &first, words, &text)?;
                wrong.get_or_insert(said);
            }
            wrong
        }
    }
}

/// A line's words with `team` and `there` put for what they may stand for:
/// a team, a point (as the film reads them, `recipes::shortened`).
fn stood_for(words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let before = at.checked_sub(1).map(|at| plain(&words[at]));
        match plain(word).as_str() {
            "there" if !matches!(before.as_deref(), Some("of" | "from")) => out.extend(["160".to_string(), "120".to_string()]),
            "team" => out.extend(["hero".to_string(), "partner".to_string()]),
            _ => out.push(word.clone()),
        }
    }
    out
}

/// The line's words as many ways as its `rest...` may be read: none of
/// them, one, two… each a word that could be anything.
fn spread(words: &[String]) -> Vec<Vec<String>> {
    let many = |word: &String| grammar::stands_for_any(word) && word.ends_with("...\u{27e9}");
    if !words.iter().any(many) {
        return vec![words.to_vec()];
    }
    (0..=MOST_WORDS)
        .map(|long| {
            words
                .iter()
                .flat_map(
                    |word| if many(word) { vec![any(word.trim_start_matches('\u{27e8}').trim_end_matches("...\u{27e9}")); long] } else { vec![word.clone()] },
                )
                .collect()
        })
        .collect()
}

/// What is wrong in a recipe, told where it is written: on its own line of
/// the film (its words there), of the file taken in, or on the `uses` line
/// with the file and its line.
fn told(film: &mut Film, one: &Defined, line: usize, message: String) {
    match one.uses {
        Some(uses) => film.wrong(uses, format!("in {}, line {line}: {message}", one.recipe.file)),
        None => {
            let base = one.recipe.text.number / INCLUDED * INCLUDED;
            film.wrong(base + line, message);
            // (The film's own: the line as it is written there.)
            if base == 0
                && let Some(problem) = film.problems.last_mut()
            {
                let source = language::Source::new(&film.source);
                let written = source.line_text(line);
                let start = source.byte(language::Position { line, column: 1 }) + written.len() - written.trim_start().len();
                problem.span.bytes = Some(language::Span::new(start, start + written.trim().len()));
            }
        }
    }
}
