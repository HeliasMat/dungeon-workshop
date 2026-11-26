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
    let doing = chained(&workshop_base::session::Session::for_tests(), text, text.chars().count()).expect("what Pikachu may do");
    assert!(doing.choices.iter().any(|(word, _)| word == "walk"), "{:?}", &doing.choices[..doing.choices.len().min(10)]);
    let text = "scene: beach pikachu piplup\npikachu walk ";
    let rest = chained(&workshop_base::session::Session::for_tests(), text, text.chars().count()).expect("what comes after walk");
    assert!(rest.choices.iter().any(|(word, _)| word == "to" || word == "piplup" || word == "right"), "{:?}", rest.choices);
    assert_eq!(line_word(text, "pikachu walk to").as_deref(), Some("walk"));
    assert_eq!(
        species_of(
            &workshop_base::session::Session::for_tests(),
            "pinchy = krabby 
scene: beach pinchy",
            "pinchy"
        ),
        "krabby"
    );
    assert_eq!(species_of(&workshop_base::session::Session::for_tests(), "scene: beach krabby2", "krabby2"), "krabby");
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
    let session = workshop_base::session::Session::for_tests();
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
    // (A directed fight exports now, as blows: its model stays.)
    assert_eq!(models.iter().map(|(word, _)| word.as_str()).collect::<Vec<_>>(), ["fight", "scene"]);
}

/// An asset of the Assets panel written in the Script: each its line, under the line it is dropped on (a blank one
/// written over), as far in; a face on a spoken line, its other words kept; a place, a new scene with the same cast.
#[test]
fn an_asset_is_written_at_its_line() {
    use workshop_base::asset::Asset;
    let faces = ["normal", "happy", "sad", "surprised"];
    let text = "scene: town pikachu piplup\n  pikachu put 1 2\n  pikachu: Hi!\n\n  piplup (bubble, sad): Oh.";
    let line = |asset: &Asset, at: usize, who: Option<&str>| super::asset_written(text, at, asset, who, None, &faces);
    let nth = |made: &(String, usize)| made.0.split('\n').nth(made.1 - 1).unwrap_or_default().to_string();
    let music = line(&Asset::Music { word: "treasure_town".to_string() }, 2, None);
    assert_eq!((nth(&music), music.1), ("  music treasure_town".to_string(), 3));
    assert_eq!(nth(&line(&Asset::Sound { word: "click".to_string() }, 3, None)), "  sound click");
    assert_eq!(nth(&line(&Asset::Gesture { who: "piplup".to_string(), words: "sit".to_string() }, 3, None)), "  piplup sit");
    // (On the blank line: written over it.)
    let written = line(&Asset::Gesture { who: "pikachu".to_string(), words: "pose look_up".to_string() }, 4, None);
    assert_eq!(written, ("scene: town pikachu piplup\n  pikachu put 1 2\n  pikachu: Hi!\n  pikachu pose look_up\n  piplup (bubble, sad): Oh.".to_string(), 4));
    // (Things: held by who is chosen, or by the one the line is of; put down where dropped.)
    assert_eq!(nth(&line(&Asset::Thing { word: "Apple".to_string(), held: true }, 5, None)), "  piplup hold Apple");
    assert_eq!(nth(&line(&Asset::Thing { word: "Apple".to_string(), held: true }, 2, Some("piplup"))), "  piplup hold Apple");
    let put = super::asset_written(text, 2, &Asset::Thing { word: "Apple".to_string(), held: false }, None, Some([286.2, 234.0]), &faces);
    assert_eq!(nth(&put), "  put object Apple 286 234");
    assert_eq!(nth(&line(&Asset::Effect { name: "Thunderbolt".to_string() }, 3, None)), "  pikachu use Thunderbolt on piplup");
    // (A face: on a spoken line, in place of the face; elsewhere, a line to say.)
    let face = |at: usize| line(&Asset::Face { who: "piplup".to_string(), face: "happy".to_string() }, at, None);
    assert_eq!(nth(&face(3)), "  pikachu (happy): Hi!");
    assert_eq!(nth(&face(5)), "  piplup (bubble, happy): Oh.", "its bubble kept, its face made another");
    assert_eq!((nth(&face(2)), face(2).1), ("  piplup (happy): ".to_string(), 3));
    assert_eq!(super::assets::with_face("title: The Day", "happy", &faces), None, "a header is no one speaking");
    // (A place: a new scene, a blank line before it, the cast kept.)
    let place = line(&Asset::Place { word: "beach".to_string() }, 5, None);
    assert_eq!((place.0.ends_with("Oh.\n\nscene: beach pikachu piplup"), place.1), (true, 7));
}

/// The lines an asset writes are read by the language without a fault.
#[test]
fn the_lines_of_the_assets_are_good_lines() {
    let tables = workshop_base::session::Session::for_tests().tables();
    for line in [
        "pikachu sit",
        "pikachu pose look_up",
        "pikachu anim Attack",
        "pikachu hold Apple",
        "put object Apple 286 234",
        "pikachu use Thunderbolt on piplup",
        "music treasure_town",
        "sound click",
        "pikachu (happy): Hi!",
        "piplup put 320 250 facing pikachu",
    ] {
        let text = format!("scene: town pikachu piplup\npikachu put 300 242\n{line}\n");
        let wrong: Vec<String> =
            film::service::diagnostics(&text, tables.clone()).into_iter().filter(|told| told.is_error()).map(|told| told.message).collect();
        assert!(wrong.is_empty(), "{line}: {wrong:?}");
    }
}

/// An asset written is one change of the text (one step back with Ctrl+Z), the cursor at its end; a Pokémon's word
/// dropped in a line gets its spaces.
#[test]
fn an_asset_written_is_one_undo() {
    use workshop_base::asset::Asset;
    let before = "scene: town pikachu\npikachu: Hi!".to_string();
    let mut open = Script {
        path: std::path::PathBuf::from("t.film"),
        text: before.clone(),
        saved: before.clone(),
        revision: 1,
        asked: 0,
        told: Vec::new(),
        told_for: 0,
        changed: None,
        cursor: 0,
        jump: None,
        root: std::path::PathBuf::new(),
    };
    let said = open.put_asset(&Asset::Music { word: "treasure_town".to_string() }, Some(1), None, None);
    assert_eq!(said, "line 2: music treasure_town");
    assert_eq!(open.text, "scene: town pikachu\nmusic treasure_town\npikachu: Hi!");
    assert_eq!(open.cursor_line(), 2);
    // (As the text editor keeps its steps back: the text before, then the text written, a step between.)
    let mut undoer: egui::util::undoer::Undoer<String> = egui::util::undoer::Undoer::default();
    undoer.feed_state(0.0, &before);
    undoer.feed_state(0.1, &open.text);
    undoer.feed_state(5.0, &open.text);
    assert_eq!(undoer.undo(&open.text), Some(&before), "one undo: as it was");
    let mut word = Script { text: "pikachu:Hi".to_string(), ..open };
    word.insert_at(7, "eevee");
    assert_eq!(word.text, "pikachu eevee:Hi", "a space before it, none before the colon");
}
