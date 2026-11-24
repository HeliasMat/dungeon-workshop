//! The text of a film read, before any word is played:
//!
//! - `front`: the text read by the language, written as the tree of lines the words play
//!   (a recipe kept as the language wrote it, and written for each of its calls);
//! - `recipes`: the recipes (`define`, `repeat`, `uses`, `include`) played out on that
//!   tree, each line read on from the one before (`there`, `team`);
//! - `cast`: who a scene's lines name, and who a line puts there.
//!
//! Here too, what a line is: who says it, when it says it happens.

pub mod cast;
pub mod checked;
pub mod front;
pub mod recipes;

use crate::verbs::Word;
use crate::words::plain;

/// The words of the text itself, read before any line is played.
pub const WORDS: &[Word] = &[
    Word::new("title", &["text..."], "typecaption before the first scene").family("Structure").ds_direct("back_SetTitleBanner(56, title): alone on black before the scene, as Sky titles its story"),
    Word::new("scene", &["place:word text..."], "scene, backdrop, ground, spread").family("Structure").ds_direct("back_SetGround(LEVEL_…), supervision_Acting(0), camera_SetMyself<performer 0>; the top screen's picture as Sky pairs it with the place (back2_SetMode(4), back2_SetGround), else black as most of Sky's scenes; who stands there goes in the .ssa"),
    Word::new(
        "transition",
        &["kind:fade|white|boss|wipe|iris|cut"],
        "how the next scene comes: whitein, iris, wipe, fadein (and their way out); with in or out, now (verbs/pictures.rs)",
    ).family("Structure").ds_close("fade: screen_FadeIn/Out; white: screen_WhiteOut, then screen_FadeIn back (as Sky); boss: CallCommon(CORO_BOSS_WIPE_FUNC), Sky's own; wipe and iris: a fade (Sky has neither)", "a wipe or an iris becomes a fade"),
    Word::new("define", &["name:word text..."], "nothing: its lines in the place of its name").family("Structure").ds_direct("nothing: recipes are written out before the export"),
    Word::new("repeat", &["n:number"], "nothing: its lines so many times").family("Structure").ds_direct("nothing: its turns are written out before the export"),
    Word::new("else", &[""], "else: the other branch of if … endif (engine commands)").family("Structure").ds_direct("} else {"),
    Word::new(
        "end",
        &["text..."],
        "closes a block (define, repeat, fight directed), or the last one open: a picture's shot, dimensional_scream, memory, choice, fight",
    ).family("Structure").ds_direct("nothing (closes a block)"),
];

/// A line's first word, and what follows it trimmed.
pub fn head(line: &str) -> (&str, &str) {
    let line = line.trim_start();
    let end = line.find(char::is_whitespace).unwrap_or(line.len());
    (&line[..end], line[end..].trim_start())
}

/// The first word of a line, as compared.
pub fn first_word(line: &str) -> String {
    plain(head(line).0)
}

/// Whether a word is a time in seconds: `3s`, `0.5s`.
pub fn in_seconds(word: &str) -> bool {
    word.strip_suffix('s').is_some_and(|number| !number.is_empty() && number.chars().all(|letter| letter.is_ascii_digit() || letter == '.'))
}

/// `who (face): words` at the start of a line: who, the brackets, the words.
pub fn spoken(line: &str) -> Option<(&str, Option<&str>, &str)> {
    let end = line.find(|letter: char| !(letter.is_alphanumeric() || letter == '_' || letter == '-'))?;
    let (name, rest) = line.split_at(end);
    if name.is_empty() {
        return None;
    }
    let rest = rest.trim_start();
    let (brackets, rest) = match rest.strip_prefix('(') {
        Some(inner) => {
            let (brackets, after) = inner.split_once(')')?;
            (Some(brackets), after.trim_start())
        }
        None => (None, rest),
    };
    Some((name, brackets, rest.strip_prefix(':')?.trim_start()))
}

/// Who says the lines without being on the stage.
pub const TELLERS: [&str; 2] = ["narrator", "voice"];

/// Whether a name is the narrator's.
pub fn told(name: &str) -> bool {
    TELLERS.contains(&plain(name).as_str())
}
