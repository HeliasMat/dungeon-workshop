//! The one reader: forms read, lines bound to them, what is said when they
//! do not fit.

use super::*;
use film_core::grammar::{bind, place_words, read, read_recipe, shown};

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_string).collect()
}

/// A line bound to a form, without anyone of the scene in it.
fn bound(form: &str, line: &str) -> Result<film_core::grammar::Bound, String> {
    bind(&mut crate::tests::film(), &read(form), &words(line))
}

#[test]
fn a_form_is_read_and_written_back() {
    let form = "where:place [kind:word] [in d:duration] off|on text...";
    assert_eq!(shown(&read(form)), "where:place [kind:word] [in d:duration] off|on text...");
    assert_eq!(shown(&read_recipe("who where:place turns:number=4 object...=Rare_Fossil")), "who where:place [turns:number=4] [object...=Rare_Fossil]");
}

#[test]
fn places_take_as_many_words_as_they_say() {
    let at = |line: &str| place_words(&words(line), 0);
    assert_eq!(at("300 120 fast"), 2);
    assert_eq!(at("front_of eevee fast"), 2);
    assert_eq!(at("left_of eevee fast"), 2);
    assert_eq!(at("between a b"), 3);
    assert_eq!(at("3 tiles from b"), 4);
    assert_eq!(at("tile 12 7"), 3);
    assert_eq!(at("{_px} {_py}"), 2);
    assert_eq!(at("m"), 1);
}

#[test]
fn a_line_gives_each_name_its_words() {
    let line = bound("where:place [kind:word] [in d:duration]", "300 120 steam in 1.5s").expect("it fits");
    assert_eq!(line.point("where"), Some((300, 120)));
    assert_eq!(line.word("kind"), Some("steam"));
    assert_eq!(line.frames("d"), Some(90));
    assert_eq!(line.places().collect::<Vec<_>>(), vec![(0, 2)]);
    // (What is left out has nothing, or its default.)
    let line = bound("where:place [kind:word] [turns:number=4]", "m").expect("it fits");
    assert!(!line.has("kind"));
    assert_eq!(line.number("turns"), Some(4.0));
    // (Given by its name, anywhere.)
    let line = bound("where:place [kind:word] [turns:number=4]", "turns=6 m").expect("it fits");
    assert_eq!(line.number("turns"), Some(6.0));
    // (The word said among choices is kept.)
    let line = bound("thing... [over|under] [hidden]", "Rare Fossil under").expect("it fits");
    assert_eq!(line.words("thing"), ["Rare", "Fossil"]);
    assert!(line.has("under") && !line.has("hidden"));
}

#[test]
fn words_before_a_place_stop_where_it_starts() {
    let line = bound("thing... where:place target:word", "Perfect Apple 100 180 rattata").expect("it fits");
    assert_eq!(line.words("thing"), ["Perfect", "Apple"]);
    assert_eq!(line.point("where"), Some((100, 180)));
    assert_eq!(line.word("target"), Some("rattata"));
}

#[test]
fn what_does_not_fit_is_told() {
    assert!(bound("where:place d:duration", "300 120 fast").unwrap_err().contains("is not a duration"));
    assert!(bound("n:number", "").unwrap_err().contains("n is missing"));
    assert!(bound("n:number", "3 4").unwrap_err().contains("too many"));
    assert!(bound("up|down", "far").unwrap_err().contains("up or down expected"));
    // (A number alone is not a place.)
    assert!(bound("where:place", "300").is_err());
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn someone_is_one_of_the_scene() {
    assert!(!tables().monsters.is_empty(), "the extracted data/");
    let line = bound("*who to where:place", "pikachu eevee to front_of eevee").expect("it fits");
    assert_eq!(line.words("who"), ["pikachu", "eevee"]);
    assert_eq!(line.words("where"), ["front_of", "eevee"]);
    assert!(bound("who:pokemon", "pikachou").unwrap_err().contains("is not someone of the scene"));
}

#[test]
fn a_line_is_written_by_its_word_s_forms_and_reads_back() {
    let line = |word: &str, given: &[(&str, &str)]| crate::line_of(word, given).unwrap_or_else(|why| panic!("{why}"));
    let written = [
        line("put", &[("who", "eevee"), ("where", "320 190")]),
        line("effect", &[("move", "Iron Tail"), ("target", "pikachu")]),
        line("emote", &[("who", "pikachu"), ("kind", "joy")]),
        line("particles", &[("kind", "fireflies"), ("x", "300"), ("y", "200")]),
        line("music", &[("name", "peace")]),
        line("put", &[("what", "Letter 280 200")]),
    ];
    assert_eq!(written[0], "eevee put 320 190");
    assert_eq!(written[1], "effect Iron Tail on pikachu");
    assert_eq!(written[3], "particles fireflies @ 300 200");
    assert_eq!(written[5], "put object Letter 280 200");
    assert!(crate::line_of("put", &[("nobody", "eevee")]).is_err());
    if crate::tests::tables().monsters.is_empty() {
        return;
    }
    let film = format!("{}\n{}\n", crate::scene_line("beach pikachu eevee"), written.join("\n"));
    let read = crate::tests::compile(&film);
    assert!(read.problems.iter().all(|problem| problem.is_remark()), "{:?}", read.problems);
}
