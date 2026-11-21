//! The docs say the words as the code has them: the table of every word in
//! `film/FILMS.md` is `every_word()` (`GOLDEN=write` writes it anew),
//! each word is in the film skill, and each command a film writes is in
//! `engine/docs/COMMANDS.md`.

use std::collections::BTreeSet;

use crate::tests::{corpus, root, tables};
use crate::verbs::every_word;

const START: &str = "<!-- words: written by film/src/tests/docs.rs -->";
const END: &str = "<!-- end of words -->";

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|_| panic!("{path}")).replace("\r\n", "\n")
}

/// The table of every word, as FILMS.md has it: its forms, where it is
/// read, what it writes.
fn table() -> String {
    let mut out = format!("{START}\n| Word | Forms | Read in | Time | Written for the engine |\n|---|---|---|---|---|\n");
    for (family, word, forms, writes) in every_word() {
        let forms: Vec<String> = forms.iter().map(|form| format!("`{}`", crate::shown_form(word, form).replace('|', "\\|"))).collect();
        let time = crate::verbs::word_time(family, word).said();
        out.push_str(&format!("| `{word}` | {} | `{family}` | {time} | {writes} |\n", forms.join("; ")));
    }
    out.push_str(END);
    out
}

const TIMES_START: &str = "<!-- engine times: written by film/src/tests/docs.rs -->";
const TIMES_END: &str = "<!-- end of engine times -->";

/// The engine's commands that last, as the registry has them.
fn times_table() -> String {
    let mut out = format!("{TIMES_START}\n| Command | Time |\n|---|---|\n");
    for (word, time) in crate::verbs::engine_times() {
        out.push_str(&format!("| `{word}` | {} |\n", time.said()));
    }
    out.push_str(TIMES_END);
    out
}

#[test]
fn the_times_of_the_engine_s_commands_are_in_films_md_as_the_registry_has_them() {
    kept_as_made(TIMES_START, TIMES_END, &times_table(), "of the engine times");
}

/// Every word of a family has a form at least, for the one reader.
#[test]
fn every_word_has_its_forms() {
    let families = crate::verbs::FAMILIES.iter().flat_map(|family| family.words.iter().map(move |word| (family.name, word)));
    let text = crate::text::WORDS.iter().map(|word| ("text", word));
    let missing: Vec<String> =
        families.chain(text).filter(|(_, word)| word.forms.is_empty()).map(|(family, word)| format!("{} ({family})", word.word)).collect();
    assert!(missing.is_empty(), "these words have no form: {missing:?}");
}

/// Every word of a family plays by its own entry: none is left to be read
/// elsewhere (the text's words are read before, the engine's go as they are).
#[test]
fn every_word_of_a_family_plays() {
    let missing: Vec<String> = crate::verbs::FAMILIES
        .iter()
        .flat_map(|family| family.words.iter().filter(|word| word.play.is_none()).map(move |word| format!("{} ({})", word.word, family.name)))
        .collect();
    assert!(missing.is_empty(), "these words do not say how they play (.does or .maybe): {missing:?}");
}

/// Whether a text names a word in backquotes: `` `word` `` or `` `word ...` ``.
fn names(text: &str, word: &str) -> bool {
    [format!("`{word}`"), format!("`{word} "), format!("`{word}:")].iter().any(|form| text.contains(form.as_str()))
}

#[test]
fn every_word_is_in_the_table_of_films_md() {
    let path = "film/FILMS.md";
    let text = read(path);
    let (Some(start), Some(end)) = (text.find(START), text.find(END)) else {
        panic!("{path}: no table of words ({START} ... {END})");
    };
    let kept = &text[start..end + END.len()];
    if std::env::var("GOLDEN").is_ok_and(|asked| asked == "write") {
        std::fs::write(root().join(path), text.replace(kept, &table())).expect("written");
        return;
    }
    assert_eq!(kept, table(), "the table of words of {path} is no longer the code's: GOLDEN=write cargo test -p film docs");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn every_command_a_film_writes_is_in_commands_md() {
    // (No game files: nothing a film names is known.)
    assert!(!tables().monsters.is_empty(), "the extracted data/");
    let commands = read("engine/docs/COMMANDS.md");
    let mut written = BTreeSet::new();
    for path in corpus() {
        let film = crate::Written::of_file_with(&path, tables()).expect("a film");
        written.extend(film.lines.into_iter().map(|(_, fields)| fields[0].clone()));
    }
    let missing: Vec<&String> = written.iter().filter(|command| !names(&commands, command)).collect();
    assert!(missing.is_empty(), "COMMANDS.md does not say: {missing:?}");
}

/// The film words that are also the name of an engine command: a film
/// word goes first, so the engine's is only reached in its own shape (see
/// each family's guards). A new one is a choice to make, not an accident.
const SHARED_WITH_THE_ENGINE: &[&str] = &[
    "advance",
    "after",
    "ambient",
    "anim",
    "answer",
    "appear",
    "approach",
    "attach",
    "banner",
    "bubble",
    "camera",
    "cast",
    "chase",
    "cheer",
    "collisions",
    "credits",
    "crowd",
    "define",
    "detach",
    "dungeon",
    "effect",
    "else",
    "emote",
    "end",
    "face",
    "fade",
    "fall",
    "fight",
    "flash",
    "flee",
    "follow",
    "frame",
    "game",
    "give",
    "go",
    "hold",
    "hop",
    "jingle",
    "jump",
    "map",
    "measure",
    "move",
    "music",
    "narrate",
    "node",
    "open",
    "orbit",
    "particles",
    "path",
    "popup",
    "pose",
    "prop",
    "put",
    "question",
    "react",
    "recruit",
    "repeat",
    "ride",
    "run",
    "runpath",
    "screen",
    "set",
    "shadow",
    "shake",
    "shot",
    "shout",
    "sound",
    "spot",
    "status",
    "thing",
    "think",
    "throw",
    "train",
    "tremble",
    "turn",
    "vanish",
    "volume",
    "wait",
    "walk",
    "wander",
    "weather",
    "when",
];

#[test]
fn no_new_word_shadows_an_engine_command() {
    let shared: BTreeSet<&str> = every_word().map(|(_, word, ..)| word).filter(|word| engine::is_command(word, &scenes::vocabulary::POKEMON)).collect();
    let new: Vec<&&str> = shared.iter().filter(|word| !SHARED_WITH_THE_ENGINE.contains(word)).collect();
    assert!(new.is_empty(), "these film words hide an engine command: {new:?} (rename, or keep the engine's form and add them to SHARED_WITH_THE_ENGINE)");
}

const PROPS_START: &str = "<!-- properties: written by film/src/tests/docs.rs, from engine/src/props/ -->";
const PROPS_END: &str = "<!-- end of properties -->";

/// The registry's table, as FILMS.md has it.
fn props_table() -> String {
    format!("{PROPS_START}\n{}{PROPS_END}", engine::props::table())
}

/// A table of FILMS.md between its marks is the code's (`GOLDEN=write`
/// writes it anew). (The table of the codes is `scene_player`'s to check:
/// only it sees every block's codes.)
fn kept_as_made(start: &str, end: &str, made: &str, what: &str) {
    let path = "film/FILMS.md";
    let text = read(path);
    let (Some(from), Some(to)) = (text.find(start), text.find(end)) else {
        panic!("{path}: no table {what} ({start} ... {end})");
    };
    let kept = &text[from..to + end.len()];
    if std::env::var("GOLDEN").is_ok_and(|asked| asked == "write") {
        std::fs::write(root().join(path), text.replace(kept, made)).expect("written");
        return;
    }
    assert_eq!(kept, made, "the table {what} of {path} is no longer the code's: GOLDEN=write cargo test -p film docs");
}

#[test]
fn the_properties_are_in_films_md_as_the_registry_has_them() {
    kept_as_made(PROPS_START, PROPS_END, &props_table(), "of the properties");
}

const RANKS_START: &str = "<!-- z ranks: written by film/src/tests/docs.rs, from engine/src/stage/layers.rs -->";
const RANKS_END: &str = "<!-- end of z ranks -->";

#[test]
fn the_ranks_are_in_films_md_as_the_engine_draws_them() {
    kept_as_made(
        RANKS_START,
        RANKS_END,
        &format!(
            "{RANKS_START}
{}{RANKS_END}",
            engine::z_table()
        ),
        "of the z ranks",
    );
}
