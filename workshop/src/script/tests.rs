//! The tests of the Script: numbers dragged, sounds heard, who answers in a
//! dialogue, the words offered.

use super::cast::{other_voice, voices_near};
use super::completion::{chained, line_word};
use super::heard::heard_on;
use super::text::number_at;
use super::*;

/// Writing a dialogue: who answers is the last other who spoke in the scene; the voices, the scene's.
#[test]
fn numbers_are_dragged_and_sounds_heard() {
    assert_eq!(number_at("put pikachu 235 275", 13), Some((12, 3, 235.0, 0)));
    assert_eq!(number_at("wait 1.5s", 8), Some((5, 3, 1.5, 1)));
    assert_eq!(number_at("krabby2: Oh", 6), None);
    assert_eq!(number_at("shift a -12 4", 9), Some((8, 3, -12.0, 0)));
    assert_eq!(heard_on("  sound star_whistle"), Some((false, "star_whistle".to_string())));
    assert_eq!(heard_on("music good_night # the night"), Some((true, "good_night".to_string())));
    assert_eq!(heard_on("music off"), None);
}

#[test]
fn the_other_answers_in_a_dialogue() {
    let text = "scene: beach pikachu piplup kecleon\npikachu: Hi!\npiplup: Hey!\npikachu (joyous): Shall we go?\n\nscene: forest eevee\neevee: Alone.";
    assert_eq!(voices_near(text, 3), vec!["pikachu", "piplup", "kecleon"]);
    assert_eq!(other_voice(text, 3, "pikachu", &voices_near(text, 3)), "piplup");
    assert_eq!(other_voice(text, 1, "pikachu", &voices_near(text, 1)), "piplup", "no one before: the next of the scene");
    assert_eq!(voices_near(text, 6), vec!["eevee"], "the other scene, its own voices");
}

/// Someone, what they do, the rest: each offered in turn, as one writes.
#[test]
fn someone_then_what_they_do_then_the_rest() {
    let text = "scene: beach pikachu piplup\npikachu ";
    let doing = chained(&crate::session::Session::for_tests(), text, text.chars().count()).expect("what Pikachu may do");
    assert!(doing.choices.iter().any(|(word, _)| word == "walk"), "{:?}", &doing.choices[..doing.choices.len().min(10)]);
    let text = "scene: beach pikachu piplup\npikachu walk ";
    let rest = chained(&crate::session::Session::for_tests(), text, text.chars().count()).expect("what comes after walk");
    assert!(rest.choices.iter().any(|(word, _)| word == "to" || word == "piplup" || word == "right"), "{:?}", rest.choices);
    assert_eq!(line_word(text, "pikachu walk to").as_deref(), Some("walk"));
    assert_eq!(
        species_of(
            &crate::session::Session::for_tests(),
            "pinchy = krabby 
scene: beach pinchy",
            "pinchy"
        ),
        "krabby"
    );
    assert_eq!(species_of(&crate::session::Session::for_tests(), "scene: beach krabby2", "krabby2"), "krabby");
}

/// The keys that change lines: commented and back, doubled, moved, indented, gone, cut, a new line begun.
#[test]
fn the_keys_change_lines() {
    use super::edits::{chosen_letters, comments_toggled, line_cut, line_doubled, lines_chosen, lines_gone, lines_moved, new_line, tabbed, untabbed};
    let commented = comments_toggled("a\n  b\n\nc", (0, 1));
    assert_eq!(commented, "# a\n  # b\n\nc");
    assert_eq!(comments_toggled(&commented, (0, 1)), "a\n  b\n\nc", "all commented: taken back");
    assert_eq!(line_doubled("one\ntwo", 5), ("one\ntwo\ntwo".to_string(), 9));
    assert_eq!(lines_moved("a\nb\nc", (1, 1), true, (2, 2)), Some(("b\na\nc".to_string(), (0, 0))));
    assert_eq!(lines_moved("a\nb\nc", (1, 1), false, (2, 2)), Some(("a\nc\nb".to_string(), (4, 4))));
    assert_eq!(lines_moved("a\nb", (0, 0), true, (0, 0)), None, "the first line goes no higher");
    assert_eq!(lines_moved("a\nb", (1, 1), false, (2, 2)), None, "the last goes no lower");
    assert_eq!(tabbed("ab", (0, 0), (1, 1)), ("a  b".to_string(), (3, 3)));
    assert_eq!(untabbed("  a\n b", (0, 1), (5, 5)), ("a\nb".to_string(), (3, 3)));
    assert_eq!(lines_gone("a\nb\nc", (1, 1)), ("a\nc".to_string(), 2));
    assert_eq!(line_cut("a\nb\nc", 1), ("a\nc".to_string(), 2));
    assert_eq!(line_cut("x", 0), (String::new(), 0));
    assert_eq!(lines_chosen("ab\ncd\nef", (0, 1)), (6, 0));
    assert_eq!(chosen_letters("hello", (4, 1)), "ell");
    assert_eq!(new_line("wait 1s", (7, 7)), ("wait 1s\n".to_string(), 8));
    assert_eq!(new_line("  together", (10, 10)), ("  together\n    \n  end".to_string(), 15), "a block opened: its end under it");
}

/// Going through the problems and marks, one cursor more, the ruler: where each goes.
#[test]
fn spots_cursors_and_the_ruler() {
    use super::keys::{cursor_beside, next_spot};
    use super::margin::{line_on_ruler, lines_naming};
    let spots = [Spot { line: 2, column: 1 }, Spot { line: 5, column: 3 }];
    assert_eq!(next_spot(&spots, 3, true).map(|spot| spot.line), Some(5));
    assert_eq!(next_spot(&spots, 5, true).map(|spot| spot.line), Some(2), "after the last: the first again");
    assert_eq!(next_spot(&spots, 2, false).map(|spot| spot.line), Some(5), "before the first: the last");
    assert_eq!(next_spot(&[], 2, false), None);
    assert_eq!(cursor_beside("ab\ncdef\ng", &[5], true), Some(2), "as far along as the line above allows");
    assert_eq!(cursor_beside("ab\ncdef\ng", &[5], false), Some(9));
    assert_eq!(cursor_beside("ab\ncdef\ng", &[1], true), None);
    assert_eq!(line_on_ruler(50.0, 0.0, 100.0, 10), 6);
    assert_eq!(line_on_ruler(100.0, 0.0, 100.0, 10), 10);
    assert_eq!(lines_naming("pikachu: hi\n# pikachu\neevee pikachu2", "pikachu"), BTreeSet::from([1]));
}

/// What the body works out before drawing: the box's height, the lines folded away, the word whole, a number dragged, the status.
#[test]
fn what_the_body_works_out() {
    use super::body::{hidden_lines, said_box_height, scrubbed, whole_word_found};
    use super::problems::{problems_said, status_said};
    assert_eq!(said_box_height(None), 0.0);
    assert_eq!(said_box_height(Some(&["Hello".to_string()])), 31.0);
    assert_eq!(hidden_lines(&[(2, 4), (6, 7)], &BTreeSet::from([2])), BTreeSet::from([3, 4]));
    assert_eq!(whole_word_found("pika pikachu pika", "pika"), vec![0..4, 13..17]);
    assert_eq!(scrubbed(235.0, 0, 9.0), "238");
    assert_eq!(scrubbed(1.5, 1, -12.0), "1.3");
    assert_eq!(status_said("scene: a\nb", Spot { line: 1, column: 2 }, None), "Line 1, column 2   ·   2 lines · 1 scene");
    assert_eq!(problems_said(0, 2).1, "2 remarks");
    assert_eq!(problems_said(1, 0).1, "1 problem");
}

/// The versions' comparison counts the lines gone and come.
#[test]
fn a_comparison_is_counted() {
    use crate::workspace::Diff;
    let compared = [Diff::Same("a".into()), Diff::Gone("b".into()), Diff::Came("c".into()), Diff::Came("d".into())];
    assert_eq!(super::windows::counted(&compared), (1, 2));
}

/// In DS Mode (`for ds`), what is offered exports: a word with a loss said
/// so, one the DS lacks not offered; without the line, everything.
#[test]
fn in_ds_mode_only_what_exports_is_offered() {
    let session = crate::session::Session::for_tests();
    let text = "scene: beach pikachu piplup\npikachu d";
    let all = super::completion::offered(&session, text, text.chars().count(), true).expect("what Pikachu may do");
    let ds = format!("for ds\n{text}");
    let exported = super::completion::offered(&session, &ds, ds.chars().count(), true).expect("what Pikachu may do on DS");
    assert!(all.choices.iter().any(|(word, _)| word == "drop"), "{:?}", all.choices);
    assert!(!exported.choices.iter().any(|(word, _)| word == "drop"), "no held item on DS: {:?}", exported.choices);
    let dodge = exported.choices.iter().find(|(word, _)| word == "dodge").expect("dodge exports");
    assert!(dodge.1.contains("DS with loss"), "{dodge:?}");
    let models =
        super::completion::for_ds(vec![("fight".to_string(), "model — a directed fight".to_string()), ("scene".to_string(), "model — a scene".to_string())]);
    assert_eq!(models.iter().map(|(word, _)| word.as_str()).collect::<Vec<_>>(), ["scene"]);
}
