//! Each word of a film's language, what it becomes for the engine, and, for
//! what depends on the scene, where it leaves those on the stage.

use ::scenes::vocabulary::POKEMON;
use engine::{Beat, Call, Director, Stage, Who};
use glam::Vec2;

use crate::Written;
use film_core::compile::with_lines;
use film_core::emit;
use film_core::film::Sortie;
use film_fights::Combat;

/// The repository's root, where the tests read the library and `data/` from.
pub(crate) fn root() -> std::path::PathBuf {
    assets::testing::root()
}

/// The project the tests compile in: the tests' smallest game
/// (`tests/games/test_village`: its places, its recipes), then the shared
/// library.
pub(crate) fn project() -> assets::project::Project {
    assets::testing::library().at(root().join("tests/games/test_village"))
}

/// The films the tests read as a corpus: those of the tests' games
/// (`tests/games/<game>/films`; the library's are the user's own, empty in
/// a new workshop), sorted.
pub(crate) fn corpus() -> Vec<std::path::PathBuf> {
    let games = std::fs::read_dir(root().join("tests/games")).into_iter().flatten().flatten().map(|game| game.path().join("films"));
    let mut films: Vec<std::path::PathBuf> = games
        .flat_map(|folder| std::fs::read_dir(folder).into_iter().flatten().flatten().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "film"))
        .collect();
    films.sort();
    films
}

/// The repository's tables (its `data/`, or `DUNGEON_DATA`), read once for all the tests.
pub(crate) fn tables() -> std::sync::Arc<film_core::tables::Tables> {
    static READ: std::sync::OnceLock<std::sync::Arc<film_core::tables::Tables>> = std::sync::OnceLock::new();
    READ.get_or_init(|| std::sync::Arc::new(film_core::tables::Tables::read(&project(), &assets::testing::data()))).clone()
}

/// A film being written with every word and the repository's tables.
pub(crate) fn film() -> film_core::Film {
    film_core::Film::with(&crate::LEXICON, tables())
}

/// A film's text compiled with the repository's tables.
pub(crate) fn compile(text: &str) -> Written {
    crate::compile_with(text, tables())
}

#[test]
fn what_a_rod_pulls_out_is_not_placed_at_the_start() {
    let lines = clean("uses fishing\nscene: town pikachu\npikachu cast\npikachu reel catch marill\n");
    let marill = lines.iter().position(|fields| starts(fields, &["putnear", "marill"])).expect("pulled out");
    assert!(!lines[..marill].iter().any(|fields| starts(fields, &["put", "marill"])), "{:?}", &lines[..marill]);
}

/// The fisher, his rod in his pictures.
fn fisher(lines: &[&str]) -> String {
    format!("scene: town pikachu_pecheur piplup\n{}\n", lines.join("\n"))
}

fn hunter(lines: &[&str]) -> String {
    format!("scene: town pikachu_chasseur piplup\n{}\n", lines.join("\n"))
}

fn scene(lines: &[&str]) -> String {
    format!("scene: town krabby piplup\n{}\n", lines.join("\n"))
}

thread_local! {
    /// The codes of the film last compiled, by the names it gives them.
    static NAMES: std::cell::RefCell<Vec<(String, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Where a line the engine reads names someone: a field read as one
/// (`target` and `m1` read there as two, not as what is left out).
fn who_places(fields: &[String]) -> Vec<usize> {
    if Beat::parse(fields, &POKEMON).is_err() {
        return Vec::new();
    }
    let as_who = |at: usize, who: &str| {
        let mut probe = fields.to_vec();
        probe[at] = who.to_string();
        Beat::parse(&probe, &POKEMON).ok()
    };
    (1..fields.len()).filter(|&at| matches!((as_who(at, "target"), as_who(at, "m1")), (Some(one), Some(other)) if one != other)).collect()
}

/// The verbs of the old scenes' blocks, sequences and names given (`cast`,
/// `define`, `do`, `each`…): never one beat, compared as written.
const PASSED_ON: [&str; 7] = ["cast", "spot", "group", "define", "enddef", "do", "each"];

/// A line as the tests compare it: the beat it is written as (`emit.rs`:
/// `sound shock` is `sound 8964`), someone told by the name the film gives.
fn as_compared(fields: &[String]) -> Vec<String> {
    if PASSED_ON.contains(&fields[0].as_str()) {
        return fields.to_vec();
    }
    NAMES.with(|names| {
        let names = names.borrow();
        // (A line the game reads, a line filled as it plays: someone by
        // their code wherever they are, told by their name.)
        if POKEMON.host_line(&fields[0]) || engine::played_filled(fields) {
            let named = |field: &String| names.iter().find(|(code, _)| code == field).map_or_else(|| field.clone(), |(_, name)| name.clone());
            return fields.iter().enumerate().map(|(at, field)| if at > 0 { named(field) } else { field.clone() }).collect();
        }
        let coded: Vec<String> = fields
            .iter()
            .enumerate()
            .map(|(at, field)| match names.iter().find(|(_, name)| name == field) {
                Some((code, _)) if at > 0 => code.clone(),
                _ => field.clone(),
            })
            .collect();
        let written = Beat::parse(&coded, &POKEMON).ok().and_then(|beat| beat.written(&POKEMON)).filter(|written| written.len() >= coded.len());
        let Some(written) = written else { return fields.to_vec() };
        let whom = who_places(&written);
        written
            .iter()
            .enumerate()
            .map(|(at, field)| match names.iter().find(|(code, _)| code == field) {
                Some((_, name)) if whom.contains(&at) => name.clone(),
                _ => field.clone(),
            })
            .collect()
    })
}

/// The film written, as the engine reads it, without its line numbers,
/// each line as the tests compare it (`as_compared`).
fn engine_lines(film: &str) -> (Vec<Vec<String>>, Vec<String>) {
    let written = compile(film);
    NAMES.with(|names| *names.borrow_mut() = written.names.clone());
    let lines = written.lines.iter().filter(|(_, fields)| !starts(fields, &["filmline"])).map(|(_, fields)| as_compared(fields)).collect();
    (lines, written.problems.iter().map(|problem| written.told_by_name(problem)).collect())
}

/// The lines said in the box, each as who says it, the face, the words and
/// how many frames it stays.
fn said_lines(lines: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut said = Vec::new();
    for fields in lines.iter().filter(|fields| fields[0] == "say") {
        let frames = fields.get(4).cloned().unwrap_or_default();
        said.push(vec![fields[1].clone(), fields.get(2).cloned().unwrap_or_default(), fields.get(3).cloned().unwrap_or_default(), frames]);
    }
    said
}

/// Whether a line starts with those words, its word as the engine names it
/// (the beats are written in the engine's words).
fn starts(fields: &[String], words: &[&str]) -> bool {
    fields.len() >= words.len() && fields.iter().zip(words).all(|(field, word)| field == word)
}

fn is_remark(problem: &str) -> bool {
    problem.starts_with("remark")
}

fn clean(film: &str) -> Vec<Vec<String>> {
    let (lines, problems) = engine_lines(film);
    let wrong: Vec<&String> = problems.iter().filter(|problem| !is_remark(problem)).collect();
    assert!(wrong.is_empty(), "{wrong:?}");
    lines
}

fn line(fields: &[&str]) -> Vec<String> {
    as_compared(&fields.iter().map(|field| field.to_string()).collect::<Vec<String>>())
}

fn wrote(film: &str, wanted: &[&[&str]]) {
    let (lines, problems) = engine_lines(film);
    assert!(problems.is_empty(), "{problems:?}");
    for fields in wanted {
        assert!(lines.contains(&line(fields)), "{fields:?} in {:?}", &lines[lines.len().saturating_sub(12)..]);
    }
}

fn has(film: &str, word: &str) {
    assert!(clean(film).iter().any(|fields| starts(fields, &[word])), "{word}");
}

fn told(film: &str, words: &str) {
    let (_, problems) = engine_lines(film);
    assert!(problems.iter().any(|problem| problem.contains(words)), "{words} : {problems:?}");
}

fn at_of(lines: &[Vec<String>], fields: &[&str]) -> usize {
    // (A whole line as the beat it is written as, or the start of one.)
    let whole = line(fields);
    lines.iter().position(|one| *one == whole || starts(one, fields)).unwrap_or_else(|| panic!("{fields:?}"))
}

/// Played on a real stage to its end: what the film leaves, and what was noted.
fn played(film: &str) -> (Written, Stage) {
    let written = compile(film);
    let wrong: Vec<String> = written.problems.iter().filter(|problem| !problem.is_remark()).map(|problem| problem.to_string()).collect();
    assert!(wrong.is_empty(), "{wrong:?}");
    let (events, problems) = written.events();
    assert!(problems.is_empty(), "{problems:?}");
    let mut director = Director::new(events, 0, &POKEMON);
    let (mut stage, mut focus) = (Stage::default(), None::<Vec2>);
    for frame in 0..30_000 {
        let calls = director.tick(&mut stage, 1.0, focus.unwrap_or(Vec2::ZERO));
        if calls.iter().any(|call| matches!(call, Call::Over)) {
            break;
        }
        if director.reading() && !director.line_timed() {
            director.read();
        }
        focus = stage.advance(1.0, frame as f32 / 60.0, focus).or(focus);
    }
    (written, stage)
}

fn point_of(played: &(Written, Stage), name: &str) -> Vec2 {
    let code = &played.0.names.iter().find(|(_, known)| known == name).unwrap_or_else(|| panic!("{name}")).0;
    let who = Who::parse(code).expect("someone");
    played.1.cast.get(&who).unwrap_or_else(|| panic!("{name} on the stage")).position
}

fn notes(played: &(Written, Stage)) -> Vec<String> {
    played.1.place.notes.iter().map(|note| played.0.by_name(&note.text, &note.mentions)).collect()
}

// ---- The film and the engine.

#[test]
fn each_line_of_the_film_tells_its_number() {
    let out = with_lines(
        vec![
            (1, Sortie::Scene { level: 3, name: "a".to_string() }),
            (1, Sortie::Beat(emit::label("a"))),
            (1, Sortie::Beat(emit::wait(2))),
            (1, Sortie::Beat(emit::wait(3))),
            (4, Sortie::Beat(emit::wait(5))),
        ],
        &film_core::time::Plan::default(),
    );
    let words: Vec<String> = out.iter().map(|(_, made)| made.fields().join(" ")).collect();
    assert_eq!(words, ["scene 3 a", "label a", "filmline 1 again", "t 2", "t 3", "filmline 4 again", "t 5"]);
}

#[test]
fn the_engine_is_told_by_names() {
    let written = compile("scene: town pikachu\npikachu: Yes.\n");
    let said = engine::Said::default().naming("species m25", 25, 0).words(" is not on the stage yet");
    let said = written.by_name(&said.text, &said.mentions);
    assert!(said.contains("pikachu is not on the stage yet"), "{said}");
    assert!(!said.contains("species"), "{said}");
}

#[test]
fn a_film_reads_and_plays_with_no_text_between() {
    let written = compile("scene: town krabby\nkrabby: Hi.\n");
    assert!(written.script().problems.is_empty());
    // (Krabby by its code: the film puts in the names itself.)
    let krabby = crate::tests::tables().monsters["krabby"].0.clone();
    assert!(written.text().contains(&format!("say\tm{krabby}\t")), "{}", written.text());
}

mod cast;
mod doc;
mod docs;
mod ds;
mod fights;
mod grammar;
mod hostile;
mod language;
mod lines;
mod nodes;
mod plot;
mod recipes;
mod replace;
mod scenes;
mod stage;
mod things;
mod time;
mod words;

#[test]
fn who_comes_in_later_is_not_there_before_even_named_at_the_head() {
    let lines = clean("scene: town krabby piplup\npiplup: Anyone there?\nkrabby enter via left to center\n");
    let comes = at_of(&lines, &["arrives", "krabby"]);
    assert!(!lines[..comes].iter().any(|fields| starts(fields, &["put", "krabby"])), "{:?}", &lines[..comes]);
}

#[test]
fn who_a_vision_brings_on_is_gone_when_it_ends() {
    let lines = clean("scene: town krabby\nmemory\npiplup appear right\npiplup: Time is cracking.\nend\nkrabby: what?\n");
    let shown = at_of(&lines, &["put", "piplup"]);
    let over = shown + at_of(&lines[shown..], &["grey", "off"]);
    assert!(lines[shown..over].iter().any(|fields| starts(fields, &["gone", "piplup"])), "{:?}", &lines[shown..]);
    // (Who was there before the vision stays.)
    assert!(!lines.iter().any(|fields| starts(fields, &["gone", "krabby"])), "{lines:?}");
}

#[test]
fn a_scene_that_says_no_song_plays_its_places() {
    let music = |film: &str| clean(film).into_iter().filter(|fields| starts(fields, &["music"])).collect::<Vec<_>>();
    assert_eq!(music("scene: guild krabby\nkrabby: Oh.\n"), [line(&["music", "guild"])]);
    // (Said, it is its own; the same place again goes on with what plays.)
    assert_eq!(music("scene: guild krabby\nmusic peace\nkrabby: Oh.\n"), [line(&["music", "peace"])]);
    assert_eq!(music("scene: guild krabby\nkrabby: Oh.\n\nscene: guild krabby\nkrabby: Again.\n").len(), 1);
}
