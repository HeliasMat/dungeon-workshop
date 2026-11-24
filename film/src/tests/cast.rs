//! Who a line brings onto the stage, as its word declares it
//! (`verbs::Word::brings`), read from the values of its form.

use film_core::ir::Instr;
use film_core::text::cast::{placed_by_a_line, who_acts};

/// The lines, each a word's line as the language reads it: its word first
/// (`walk pikachu to piplup`, written `pikachu walk to piplup`).
fn lines_of(lines: &[&str]) -> Vec<Instr> {
    lines
        .iter()
        .enumerate()
        .map(|(at, line)| {
            let words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
            Instr::word(at + 1, &words[0], &words[1..], true)
        })
        .collect()
}

/// Who the lines name, `chest` someone of the film.
fn named(lines: &[&str]) -> Vec<String> {
    let mut film = crate::tests::film();
    film.give_name("chest".to_string(), "m1".to_string());
    who_acts(&mut film, &lines_of(lines))
}

/// Who a line names is read from its form's values: who it is said of,
/// those its places are said by; never a thing, nor words said.
#[test]
fn a_line_names_those_its_form_gives_someone() {
    assert_eq!(named(&["walk pikachu to piplup"]), ["pikachu", "piplup"]);
    assert_eq!(named(&["walk pikachu to front_of piplup"]), ["pikachu", "piplup"]);
    assert!(named(&["sound click"]).is_empty());
    assert!(named(&["put object chest"]).is_empty());
    assert_eq!(named(&["think pikachu eevee is late"]), ["pikachu"]);
}

/// Those the lines put on the stage, the head naming those listed, so many
/// of the lines setting the scene; `chest` someone of the film.
fn placed(lines: &[&str], listed: &[&str], setting: usize) -> Vec<String> {
    let mut film = crate::tests::film();
    film.give_name("chest".to_string(), "m1".to_string());
    let lines = lines_of(lines);
    let listed: Vec<String> = listed.iter().map(|one| one.to_string()).collect();
    placed_by_a_line(&mut film, &lines, &listed, setting)
}

#[test]
fn a_word_declaring_an_entry_brings_its_parameter() {
    // (`_haul`, the reel's: its catch, new to the scene; not the fisher, at its head.)
    assert_eq!(placed(&["_haul pikachu marill"], &["pikachu"], 0), ["marill"]);
    // (Not one named before.)
    assert!(placed(&["look marill left", "_haul pikachu marill"], &["pikachu"], 0).is_empty());
    // (`enter`: in, even named at the head, unless a line had him act.)
    assert_eq!(placed(&["look pikachu left", "enter piplup via left"], &["pikachu", "piplup"], 0), ["piplup"]);
    assert!(placed(&["look piplup left", "enter piplup via left"], &["piplup"], 0).is_empty());
    // (`jump a water`: out of the water; `jump a`: no one comes.)
    assert_eq!(placed(&["jump magikarp water 300 200"], &[], 0), ["magikarp"]);
    assert!(placed(&["jump magikarp wave"], &["pikachu"], 0).is_empty());
}

#[test]
fn putting_an_object_brings_no_one() {
    assert!(placed(&["put object chest"], &[], 0).is_empty());
    assert!(placed(&["put object chest"], &[], 1).is_empty());
    // (But someone of that name put there is.)
    assert_eq!(placed(&["put chest 300 200"], &[], 0), ["chest"]);
}

#[test]
fn a_formation_brings_all_its_members() {
    assert_eq!(placed(&["form row pikachu piplup bulbasaur 300 200"], &[], 0), ["pikachu", "piplup", "bulbasaur"]);
    assert_eq!(placed(&["enter triangle pikachu piplup bulbasaur"], &[], 0), ["pikachu", "piplup", "bulbasaur"]);
    assert_eq!(placed(&["face to face pikachu piplup"], &[], 0), ["pikachu", "piplup"]);
    // (Before the scene shows, those at its head too.)
    assert_eq!(placed(&["form column pikachu piplup"], &["pikachu", "piplup"], 1), ["pikachu", "piplup"]);
    // (`face a b` is no formation.)
    assert!(placed(&["face pikachu piplup"], &[], 0).is_empty());
}
