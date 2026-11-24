//! The Story family's names: the logic of a story told, as Ink and Yarn
//! Spinner have it. Read by the language (`language/`: `option "text" [if
//! test] [once]`, `{if test: words | other}`, `{value}`, a line's `#id`)
//! and written by `text/front/`; here what each moment and scene is called,
//! the number that counts how many times it was played (`seen(<name>)`:
//! `seen_<name>`, counted for the run of a film alone; kept by the game's
//! story across plays when a game plays it: [`counters`], `game/src/lib.rs`),
//! and the film's map of them (`StoryMap`, made by `compile`). Its word,
//! `go`, is a group's: `verbs/story.rs`.
//!
//! A film that does none of it is written as before: no label, no count.

use std::collections::{BTreeSet, HashSet};

use crate::words::plain;

/// A moment's or a scene's name as compared: its words plain, joined by `_`
/// (`moment: The bet` is `the_bet`).
pub fn slug(name: &str) -> String {
    name.split_whitespace().map(plain).filter(|word| !word.is_empty()).collect::<Vec<_>>().join("_")
}

/// The number that counts how many times that moment or scene was played.
pub fn seen_var(name: &str) -> String {
    format!("seen_{}", slug(name))
}

/// Whether a film's text goes from one part to another or counts them.
pub fn uses_story(text: &str) -> bool {
    text.lines().map(str::trim_start).any(|line| line.starts_with("go moment ") || line.starts_with("go scene ") || line.contains("seen("))
}

/// The numbers a film counts its moments and scenes by (`seen_the_bet`), if
/// it counts them: those a game keeps for it from one play to the next.
pub fn counters(text: &str) -> BTreeSet<String> {
    if !uses_story(text) {
        return BTreeSet::new();
    }
    names(text).into_iter().map(|name| format!("seen_{name}")).collect()
}

/// A film's moments and scenes, by name (as `slug` writes them), read by the
/// language alone.
pub fn names(text: &str) -> Vec<String> {
    names_of(&language::parse(text).file)
}

/// The same, of a text read.
pub fn names_of(file: &language::ast::File) -> Vec<String> {
    use language::ast::{Kind, Stmt};
    fn moments(stmts: &[Stmt], into: &mut Vec<String>) {
        for stmt in stmts {
            match &stmt.kind {
                Kind::Moment(name) => into.push(slug(name)),
                Kind::Block(block) => {
                    moments(&block.body, into);
                    for part in &block.parts {
                        moments(&part.body, into);
                    }
                }
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    for scene in &file.scenes {
        if let Some(place) = scene.args.first() {
            found.push(slug(&place.text));
        }
        moments(&scene.body, &mut found);
    }
    let mut seen = HashSet::new();
    found.retain(|name| seen.insert(name.clone()));
    found
}

/// What the film knows of its moments and scenes, to go to them and count them.
#[derive(Clone, Debug, Default)]
pub struct StoryMap {
    /// Whether the film goes from one to another or counts them.
    pub used: bool,
    /// Each moment by name, the scene it is in (its rank).
    pub moments: Vec<(String, usize)>,
    /// Each scene's place, by rank.
    pub scenes: Vec<String>,
    /// The moments gone to from another scene: asked for as their scene opens.
    pub reached_from_afar: Vec<usize>,
    /// The scene at hand.
    pub at: usize,
}
