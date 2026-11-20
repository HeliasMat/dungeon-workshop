//! The pages written by hand (`workshop/wiki/*.txt`): for a word, what
//! it does said for an author, and an example, a little film of our own.
//!
//! ```text
//! == walk
//! > Someone walks to a place, step by step; the film waits until he is there.
//! scene: beach pikachu piplup
//! pikachu walk to right
//! ```
//!
//! A page begins with `== word`; its lines beginning with `> ` say what it
//! does (one paragraph); the others, the example, as a film is written. A
//! line `!retired` marks a word a film does not take as written (the game's
//! now, or an engine command a film's own words write): its page says what to
//! write instead, its line is refused.
//! Before the first page, the lines are the file's own comments.

use std::collections::HashMap;

/// What is written of a word: what it does, its example (if any).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Written {
    pub explained: String,
    pub example: String,
    pub retired: bool,
}

/// The files of the pages written, in the Workshop (`workshop/wiki/`), a family or two each.
pub const PAGES: [&str; 19] = [
    include_str!("../../wiki/speech.txt"),
    include_str!("../../wiki/moving.txt"),
    include_str!("../../wiki/gestures.txt"),
    include_str!("../../wiki/camera.txt"),
    include_str!("../../wiki/sound.txt"),
    include_str!("../../wiki/effects.txt"),
    include_str!("../../wiki/fights.txt"),
    include_str!("../../wiki/items.txt"),
    include_str!("../../wiki/fishing.txt"),
    include_str!("../../wiki/dungeon.txt"),
    include_str!("../../wiki/engine.txt"),
    include_str!("../../wiki/story.txt"),
    include_str!("../../wiki/core.txt"),
    include_str!("../../wiki/presence.txt"),
    include_str!("../../wiki/formations.txt"),
    include_str!("../../wiki/movement.txt"),
    include_str!("../../wiki/turning.txt"),
    include_str!("../../wiki/body.txt"),
    include_str!("../../wiki/emotions.txt"),
];

/// The pages of all the files written.
pub fn every_page() -> HashMap<String, Written> {
    PAGES.iter().flat_map(|text| read(text)).collect()
}

/// The pages of a text, by word (a word written twice: the last).
pub fn read(text: &str) -> HashMap<String, Written> {
    let mut pages: HashMap<String, Written> = HashMap::new();
    let mut current: Option<(String, Vec<String>, Vec<String>, bool)> = None;
    let mut done = |current: Option<(String, Vec<String>, Vec<String>, bool)>| {
        if let Some((word, said, film, retired)) = current {
            let example = film.join("\n").trim_end().to_string();
            let example = if example.is_empty() { example } else { format!("{example}\n") };
            pages.insert(word, Written { explained: said.join(" "), example, retired });
        }
    };
    for line in text.lines() {
        if let Some(word) = line.strip_prefix("== ") {
            done(current.take());
            current = Some((film::words::plain(word.trim()), Vec::new(), Vec::new(), false));
            continue;
        }
        let Some((_, said, film, retired)) = current.as_mut() else { continue };
        if line.trim() == "!retired" {
            *retired = true;
            continue;
        }
        match line.strip_prefix("> ") {
            Some(told) => said.push(told.trim().to_string()),
            None if film.is_empty() && line.trim().is_empty() => {}
            None => film.push(line.trim_end().to_string()),
        }
    }
    done(current);
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_are_read_by_word() {
        let pages = read("# the file's own\n== walk\n> Walks.\n> Waited for.\nscene: beach pikachu\npikachu walk to right\n\n== sky\n> The sky.\n!retired\n");
        let walk =
            Written { explained: "Walks. Waited for.".to_string(), example: "scene: beach pikachu\npikachu walk to right\n".to_string(), retired: false };
        assert_eq!(pages["walk"], walk);
        assert_eq!(pages["sky"].example, "");
        assert!(pages["sky"].retired);
        assert_eq!(pages.len(), 2);
    }
}
