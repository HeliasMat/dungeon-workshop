//! Who a scene's lines name, in the order they come, and who a line puts on
//! the stage by itself: those need no place at the start. Who a line brings
//! is its word's to say (`verbs::Word::brings`), never guessed here.

use crate::film::Film;
use crate::ir::{Instr, Shape};
use crate::verbs::{Comes, brought_by, named_by};
use crate::words::plain;

/// Who a line names, in the order they come.
fn named_in(film: &mut Film, item: &Instr) -> Vec<String> {
    // (A said line and a word as they were read; a line anchored, who it
    // waits for and what it plays; a fight's moment, its lines; the
    // engine's fields, by its word.)
    let (said, words, first) = match &item.shape {
        Shape::Said { who, .. } => (true, vec![who.clone()], String::new()),
        Shape::Word { verb, words, .. } => (false, words.clone(), verb.clone()),
        Shape::Engine(fields) => match fields.split_first() {
            Some((verb, rest)) => (false, rest.to_vec(), verb.clone()),
            None => return Vec::new(),
        },
        Shape::Arrives { who, then } => {
            let mut named: Vec<String> = who.iter().map(|one| plain(one)).filter(|one| film.actor(one).is_some()).collect();
            named.extend(then.iter().flat_map(|then| named_in(film, then)));
            return named;
        }
        Shape::AtMark { then, .. } => return then.iter().flat_map(|then| named_in(film, then)).collect(),
        Shape::WhenLife { who, lines, .. } => {
            let mut named: Vec<String> = std::iter::once(plain(who)).filter(|one| film.actor(one).is_some()).collect();
            named.extend(lines.iter().flat_map(|line| named_in(film, line)));
            return named;
        }
        _ => return Vec::new(),
    };
    // (A line played later names who it plays: `every 3s sound click` no one.)
    if !said && matches!(first.as_str(), "every" | "after") && words.len() > 1 {
        return named_in(film, &Instr::word(item.number, &words[1], &words[2..], true));
    }
    // (Who the line's word names, by its form: `verbs::named_by`; a said
    // line, its speaker; what no form reads, fields for the engine, each
    // word that is someone, but those its form says are a thing.)
    let derived = if said { None } else { named_by(film, &first, &words) };
    let mut found = match derived {
        Some(named) => named,
        None => {
            let things = crate::verbs::things_in(film, &first, &words);
            words
                .iter()
                .filter(|word| !film.thing_words.contains(&plain(word)) && !things.contains(word) && film.actor(word).is_some())
                .map(|word| plain(word))
                .collect()
        }
    };
    for inner in item.inner.iter().flatten() {
        found.extend(named_in(film, inner));
    }
    found
}

/// Everyone a scene's lines name, in the order they come.
pub fn who_acts(film: &mut Film, lines: &[Instr]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for item in lines {
        for who in named_in(film, item) {
            if !seen.contains(&who) {
                seen.push(who);
            }
        }
    }
    seen
}

/// A scene's lines, those of its blocks among them, without what says when.
fn alone(lines: &[Instr], out: &mut Vec<Instr>) {
    for item in lines {
        match &item.inner {
            Some(inner) if !item.text.to_lowercase().starts_with("fight") => alone(inner, out),
            _ => out.push(item.clone()),
        }
    }
}

/// Those a line brings onto the stage, each with how he comes, as its word
/// declares it (`verbs::Brings`): read from the values of its form.
fn brought(film: &mut Film, item: &Instr) -> Vec<(String, Comes)> {
    match &item.shape {
        Shape::Word { verb, words, .. } => brought_by(film, verb, words),
        _ => Vec::new(),
    }
}

/// Whether a line puts someone there or brings him in (`Comes::Put`,
/// `Comes::Arrives`): who it names comes by a line that says so.
pub fn puts_someone(film: &mut Film, item: &Instr) -> bool {
    brought(film, item).iter().any(|(_, comes)| *comes != Comes::New)
}

/// Those a line puts on the stage, who need no place at the start: as its
/// word declares it (`verbs::Brings`), named for the first time by that
/// line, put before the scene shows, or coming in. Who was already there
/// when a late `place` comes stays there until then. (`setting`: how many of
/// the lines come before it shows.)
pub fn placed_by_a_line(film: &mut Film, lines: &[Instr], listed: &[String], setting: usize) -> Vec<String> {
    let (mut placed, mut seen): (Vec<String>, Vec<String>) = (Vec::new(), listed.to_vec());
    // (Who a line of the scene has named so far: the head is not a line.)
    let mut acted: Vec<String> = Vec::new();
    let mut all = Vec::new();
    alone(lines, &mut all);
    for (at, item) in all.iter().enumerate() {
        for (one, comes) in brought(film, item) {
            let new = match comes {
                Comes::New => !seen.contains(&one),
                Comes::Put => at < setting || !seen.contains(&one),
                // (Who comes in in the middle of the scene is not there
                // before, even named at its head, unless a line had him act.)
                Comes::Arrives => at < setting || !acted.contains(&one),
            };
            if new && !placed.contains(&one) {
                placed.push(one);
            }
        }
        let named = named_in(film, item);
        acted.extend(named.iter().cloned());
        seen.extend(named);
    }
    placed
}
