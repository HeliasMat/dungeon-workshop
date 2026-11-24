//! The wiki of the films' language: a page for every word (the language's,
//! the library's recipes, the engine's commands), each saying what it does
//! for an author, its forms and what each part of them means, a little film
//! as an example, the words it goes with. Written by hand where
//! `workshop_wiki/wiki/*.txt` has the word (`written`), made from its forms
//! otherwise (`made`); searched by name, family and what it does (`search`).
//! The Script shows it beside the text (`script/wiki.rs`).
//!
//! A crate of Dungeon Workshop: the pages (`pages`, `page_of`...) and the
//! words' catalogue they are made of (`catalogue`). It uses `workshop_base`
//! (the session, which keeps both) and `film` (the language's words, plan and
//! recipes); the Script (`workshop_script`) shows it.

use std::sync::Arc;

use crate::catalogue::{Entry, Origin};
use workshop_base::session::Session;

pub mod made;
pub mod search;
mod written;

pub use made::Form;

/// A word's page.
#[derive(Clone, Debug, Default)]
pub struct Page {
    pub word: String,
    /// Its family, of `catalogue::FAMILIES`.
    pub family: usize,
    /// Where it comes from (none: a page of the tests).
    pub origin: Option<Origin>,
    /// What it does, said for an author.
    pub explained: String,
    pub forms: Vec<Form>,
    /// Its example: a little film; the line of it where the word is (from 1); written by hand, or made.
    pub example: String,
    pub focus: usize,
    pub written: bool,
    /// A word a film does not take as written (the game's now, or an engine command a film's words write): its page
    /// says what to write instead.
    pub retired: bool,
}

impl Page {
    /// The example's line where the word is.
    pub fn focus_line(&self) -> &str {
        self.example.lines().nth(self.focus.saturating_sub(1)).unwrap_or_default()
    }

    /// What it does in a line: its first sentence, cut short if long.
    pub fn one_line(&self) -> String {
        let first = self.explained.split_inclusive(". ").next().unwrap_or_default().trim();
        if first.chars().count() > 110 { format!("{}…", first.chars().take(108).collect::<String>().trim_end()) } else { first.to_string() }
    }

    /// Where it comes from, said: a recipe's file, the engine.
    pub fn origin_said(&self) -> String {
        match &self.origin {
            Some(Origin::Recipe { from, .. }) => format!("a recipe of film/recipes/{from}.film"),
            Some(Origin::Command) => "a command of the engine".to_string(),
            _ => "a word of the language".to_string(),
        }
    }
}

/// Every page, by family then name (made once, kept by the session).
pub fn pages(session: &Session) -> Arc<Vec<Page>> {
    session.kept.get(|| {
        let written = written::every_page();
        crate::catalogue::catalogue(session).iter().map(|entry| page(entry, written.get(&entry.word))).collect()
    })
}

/// A word's page, by its name (plain).
pub fn page_of(session: &Session, word: &str) -> Option<Page> {
    let word = film::words::plain(word);
    pages(session).iter().find(|page| page.word == word).cloned()
}

/// A word's page, from its entry and what is written of it (if anything).
pub fn page(entry: &Entry, written: Option<&written::Written>) -> Page {
    let mut forms: Vec<Form> = entry.forms.iter().map(|form| made::form(&entry.word, form, entry.subject)).collect();
    // (Said of someone: the forms that start with someone first, `who walk to where` before the formations.)
    forms.sort_by_key(|form| !form.someone_first);
    // (A form that reads the whole rest itself, `walk rest…`, says nothing more than the others.)
    if forms.len() > 1 {
        forms.retain(|form| !form.shown.ends_with(&format!("{} rest…", entry.word)));
    }
    let explained = written.map(|written| written.explained.clone()).filter(|said| !said.is_empty()).unwrap_or_else(|| explained(entry));
    let (example, focus, by_hand) = match written.filter(|written| !written.example.is_empty()) {
        Some(written) => (written.example.clone(), focus_of(&written.example, &entry.word), true),
        None => {
            let (film, focus) = made::example(&entry.word, &entry.forms, entry.subject);
            (film, focus, false)
        }
    };
    let retired = written.is_some_and(|written| written.retired);
    Page { word: entry.word.clone(), family: entry.family, origin: Some(entry.origin.clone()), explained, forms, example, focus, written: by_hand, retired }
}

/// How a word of the language with no page written is said: by what it writes for the engine.
const UNWRITTEN: &str = "Written for the engine as: ";

/// What a word does, when nothing is written of it: a recipe's comment, an
/// engine command's line of the docs, else what it writes for the engine.
fn explained(entry: &Entry) -> String {
    let doc = entry.doc.trim();
    let said = match &entry.origin {
        Origin::Command => doc.strip_prefix(&format!("{}:", entry.word)).unwrap_or(doc).trim().to_string(),
        Origin::Recipe { .. } => doc.to_string(),
        Origin::Word(_) => format!("{UNWRITTEN}{doc}"),
    };
    sentence(&said)
}

/// A capital first, a full stop last.
fn sentence(said: &str) -> String {
    let said = said.trim();
    let mut letters = said.chars();
    let Some(first) = letters.next() else { return String::new() };
    let mut made: String = first.to_uppercase().chain(letters).collect();
    if !made.ends_with(['.', '!', '?', ')', '`']) {
        made.push('.');
    }
    made
}

/// The line of an example where the word is said (its first or second word,
/// past the head); else its last line.
pub fn focus_of(example: &str, word: &str) -> usize {
    let lines: Vec<&str> = example.lines().collect();
    let says = |line: &str| {
        let mut words = line.split_whitespace().map(|said| film::words::plain(said.trim_end_matches(':')));
        let (first, second) = (words.next(), words.next());
        first.as_deref() == Some(word) || second.as_deref() == Some(word)
    };
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| says(line))
        .or_else(|| lines.iter().enumerate().find(|(_, line)| says(line)))
        .map_or(lines.len().max(1), |(at, _)| at + 1)
}

/// Whether two pages come from the same place: the same family of the code, the same file of recipes.
fn same_source(one: &Page, other: &Page) -> bool {
    match (&one.origin, &other.origin) {
        (Some(Origin::Recipe { from: a, .. }), Some(Origin::Recipe { from: b, .. })) => a == b,
        (Some(Origin::Word(a)), Some(Origin::Word(b))) => a == b,
        (Some(Origin::Command), Some(Origin::Command)) => true,
        _ => false,
    }
}

/// The words a page goes with: those its example uses, then those of its
/// family whose name shares a part with it, then those of its family that
/// come from the same place, then its family's others, so many at most.
pub fn related(pages: &[Page], page: &Page, most: usize) -> Vec<String> {
    let known = |word: &str| pages.iter().any(|other| other.word == word);
    let mut related: Vec<String> = Vec::new();
    let add = |word: &str, related: &mut Vec<String>| {
        if word != page.word && word != "scene" && word != "put" && known(word) && !related.iter().any(|said| said == word) {
            related.push(word.to_string());
        }
    };
    for line in page.example.lines().skip(1) {
        let words: Vec<String> = line.split_whitespace().take(2).map(film::words::plain).collect();
        for word in &words {
            add(word, &mut related);
        }
    }
    let parts: Vec<&str> = page.word.split('_').filter(|part| part.len() > 2).collect();
    let family: Vec<&Page> = pages.iter().filter(|other| other.family == page.family).collect();
    for other in &family {
        if other.word.split('_').any(|part| parts.contains(&part)) {
            add(&other.word, &mut related);
        }
    }
    for other in family.iter().filter(|other| same_source(page, other)) {
        add(&other.word, &mut related);
    }
    for other in &family {
        add(&other.word, &mut related);
    }
    related.truncate(most);
    related
}

/// The source of a recipe of the library, from the comment over its
/// `define` to its `end` (none if it is not found).
pub fn recipe_source(session: &Session, from: &str, word: &str) -> Option<String> {
    let path = film::shared_path(session.library(), from)?;
    let text = std::fs::read_to_string(path).ok()?.replace("\r\n", "\n");
    recipe_in(&text, word)
}

/// A recipe's lines in a text: its comment, `define word …` to its `end`.
pub fn recipe_in(text: &str, word: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|line| line.trim_start().strip_prefix("define ").is_some_and(|rest| rest.split_whitespace().next() == Some(word)))?;
    // (Its blocks may not be indented: it ends at the last `end` before the next `define`.)
    let next = (start + 1..lines.len()).find(|&at| lines[at].trim_start().starts_with("define ")).unwrap_or(lines.len());
    let end = (start..next).rev().find(|&at| lines[at].trim() == "end").unwrap_or(next - 1);
    let mut first = start;
    while first > 0 && lines[first - 1].trim_start().starts_with('#') {
        first -= 1;
    }
    Some(lines[first..=end].join("\n"))
}

/// The families in their order, for those who show them.
pub fn family_name(family: usize) -> String {
    crate::catalogue::family_shown(family)
}

pub mod catalogue;
#[cfg(test)]
mod tests;
