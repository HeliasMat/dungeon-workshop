//! The tests of what the Timeline writes: a bar moved in time (its wait),
//! alongside or in sequence, its time pulled, onto another's track, a line
//! written at a time; one change to undo; the films written still read.

use std::sync::Arc;

use super::edits::{
    Course, asset_line, frames_of, inserted_beside, inserted_in_sequence, moved_beside, retimed, time_said, toggled_beside, with_duration, with_performer,
};
use super::lanes::{Lane, bar_words, lane_of};
use super::timeline::bar_moved;

/// A film: lines 1 to 10, the title, the scene, two put, a wait, a hop, a wait, a nod, a line said, a wait.
const FILM: &str = "title: Test\nscene: town pikachu piplup\npikachu put 300 242 facing down\npiplup put 340 242 facing down\nwait 1s\npikachu hop\nwait 2s\npiplup nod\npikachu: Hello.\nwait 1s";

/// When each of its lines played (frames).
fn course() -> Vec<Course> {
    [(1, 0, 0), (2, 0, 0), (3, 0, 0), (4, 0, 0), (5, 0, 60), (6, 60, 90), (7, 90, 210), (8, 210, 240), (9, 240, 400), (10, 400, 460)]
        .into_iter()
        .map(|(line, start, end)| Course { line, start, end })
        .collect()
}

/// The film's text with its lines from `from` (from 1) made these.
fn with_lines(from: usize, made: &[&str], gone: usize) -> String {
    let mut lines: Vec<String> = FILM.split('\n').map(str::to_string).collect();
    lines.splice(from - 1..from - 1 + gone, made.iter().map(|line| line.to_string()));
    lines.join("\n")
}

fn tables() -> Arc<film::tables::Tables> {
    static READ: std::sync::OnceLock<Arc<film::tables::Tables>> = std::sync::OnceLock::new();
    READ.get_or_init(|| Arc::new(film::tables::Tables::read(&assets::testing::library(), &assets::testing::data()))).clone()
}

/// What reading a film finds wrong (its remarks left aside).
fn wrong(text: &str) -> Vec<String> {
    film::compile_with(text, tables()).problems.into_iter().filter(|problem| !problem.is_remark()).map(|problem| problem.message).collect()
}

#[test]
fn times_are_read_and_written_as_films_write_them() {
    assert_eq!((frames_of("2s"), frames_of("1.5s"), frames_of("18f"), frames_of("leaf"), frames_of("s")), (Some(120), Some(90), Some(18), None, None));
    assert_eq!((time_said(60), time_said(90), time_said(18 + 1), time_said(6)), ("1s".to_string(), "1.5s".to_string(), "19f".to_string(), "0.1s".to_string()));
}

/// A bar moved later or earlier: the wait before it, the one after it the other way (Shift: not).
#[test]
fn a_bar_moved_rewrites_the_wait_before_it() {
    let (made, line, said) = retimed(FILM, 6, 30, false).unwrap();
    assert_eq!(made, with_lines(5, &["wait 1.5s", "pikachu hop", "wait 1.5s"], 3), "the rest stays");
    assert_eq!((line, said.as_str()), (6, "line 6: 0.5s later"));
    let (made, ..) = retimed(FILM, 6, 30, true).unwrap();
    assert_eq!(made, with_lines(5, &["wait 1.5s"], 1), "Shift: the rest moves with it");
    let (made, line, _) = retimed(FILM, 6, -12, false).unwrap();
    assert_eq!((made, line), (with_lines(5, &["wait 0.8s", "pikachu hop", "wait 2.2s"], 3), 6));
    // (Earlier than its wait: the wait taken away, as early as it can.)
    let (made, line, said) = retimed(FILM, 6, -90, false).unwrap();
    assert_eq!(made, with_lines(5, &["pikachu hop", "wait 3s"], 3));
    assert_eq!((line, said.as_str()), (5, "line 5: 1s earlier"));
    // (No wait after it: the rest moves; no wait before it: one written.)
    assert_eq!(retimed(FILM, 8, 12, false).unwrap().0, with_lines(7, &["wait 2.2s"], 1));
    let (made, line, _) = retimed(FILM, 9, 30, false).unwrap();
    assert_eq!((made, line), (with_lines(9, &["wait 0.5s", "pikachu: Hello.", "wait 0.5s"], 2), 10));
    assert!(retimed(FILM, 9, -30, false).is_err(), "nothing before it to shorten");
    assert!(retimed(FILM, 6, 2, false).is_err(), "less than a step: not moved");
}

/// Alongside (`&`) or back in sequence, by Alt+drag or the bar's menu.
#[test]
fn a_line_runs_alongside_or_in_sequence() {
    let (made, beside) = toggled_beside(FILM, 6).unwrap();
    assert_eq!((made.as_str(), beside), (with_lines(6, &["pikachu hop &"], 1).as_str(), true));
    let (back, beside) = toggled_beside(&made, 6).unwrap();
    assert_eq!((back.as_str(), beside), (FILM, false));
    let named = with_lines(6, &["pikachu hop & as hopping"], 1);
    assert_eq!(toggled_beside(&named, 6).unwrap().0, FILM, "& as name taken away too");
    assert!(toggled_beside(FILM, 5).is_err(), "a wait runs in sequence");
    assert!(toggled_beside(FILM, 2).is_err(), "the scene's line");
}

/// A bar's end pulled: its time written again, in its own unit; `in Ns` added to a walk.
#[test]
fn a_bar_end_pulled_writes_its_time() {
    assert_eq!(with_duration("  wait 18f", 30), "  wait 30f");
    assert_eq!(with_duration("wait 1s", 40), "wait 0.7s", "to a tenth");
    assert_eq!(with_duration("pikachu walk to 1 2", 90), "pikachu walk to 1 2 in 1.5s");
    assert_eq!(with_duration("glide to 1 2 in 1s &", 120), "glide to 1 2 in 2s &");
}

/// A bar onto another's track: they do it (a line said, a gesture; a word-first line by its form's `who`).
#[test]
fn a_bar_onto_another_track_changes_who_does_it() {
    assert_eq!(with_performer(FILM, 6, None, "pikachu", "piplup").unwrap(), with_lines(6, &["piplup hop"], 1));
    assert_eq!(with_performer(FILM, 9, None, "pikachu", "piplup").unwrap(), with_lines(9, &["piplup: Hello."], 1));
    assert!(with_performer(FILM, 8, None, "pikachu", "piplup").is_err(), "not pikachu's line");
    let read = film::ReadAs {
        word: "walk".to_string(),
        group: "moving",
        values: vec![film::grammar::Value {
            name: "who".to_string(),
            words: vec!["pikachu".to_string()],
            kind: film::grammar::Kind::Someone,
            at: Some(0),
            span: None,
        }],
    };
    let walking = "scene: town pikachu piplup\nwalk pikachu to 1 2";
    assert_eq!(with_performer(walking, 2, Some(&read), "pikachu", "piplup").unwrap().trim_end(), "scene: town pikachu piplup\nwalk piplup to 1 2");
    // (Both at once: onto piplup's track, half a second later; one change.)
    let lane = |who: &str| Lane::Who(who.to_string());
    let (made, line, said) = bar_moved(FILM, &course(), None, 6, 60, 30, false, &lane("pikachu"), &lane("piplup")).unwrap();
    assert_eq!(made, with_lines(5, &["wait 1.5s", "piplup hop", "wait 1.5s"], 3));
    assert_eq!((line, said.as_str()), (6, "line 6: piplup does it; line 6: 0.5s later"));
}

/// A line written at a time: in a wait, the wait split round it; after the
/// film's end, a wait to reach it; in a line still playing, after it; beside
/// the film, before the line starting then, a `beside` block waiting until then.
#[test]
fn a_line_is_written_at_a_time() {
    let (made, line, _) = inserted_in_sequence(FILM, &course(), 150, "pikachu nod").unwrap();
    assert_eq!((made, line), (with_lines(7, &["wait 1s", "pikachu nod", "wait 1s"], 1), 8));
    let (made, line, _) = inserted_in_sequence(FILM, &course(), 500, "pikachu nod").unwrap();
    assert_eq!((made, line), (with_lines(11, &["wait 0.7s", "pikachu nod"], 0), 12));
    let (made, line, said) = inserted_in_sequence(FILM, &course(), 300, "pikachu nod").unwrap();
    assert_eq!((made, line), (with_lines(10, &["pikachu nod"], 0), 10));
    assert!(said.contains("when line 9 ends"), "{said}");
    let (made, line) = inserted_beside(FILM, &course(), 150, "pikachu nod").unwrap();
    assert_eq!((made, line), (with_lines(7, &["beside", "  wait 1s", "  pikachu nod", "end"], 0), 9));
    let (made, line) = inserted_beside(FILM, &course(), 210, "piplup hop").unwrap();
    assert_eq!((made, line), (with_lines(8, &["piplup hop &"], 0), 8));
    // (A line beside the film moved: taken from where it was, written where it starts then.)
    let alongside = with_lines(6, &["pikachu hop &"], 1);
    let (made, line) = moved_beside(&alongside, &course(), 6, 150).unwrap();
    assert_eq!((made, line), (with_lines(6, &["wait 2s"], 2).replace("wait 1s\nwait 2s", "wait 1s\nbeside\n  wait 1s\n  pikachu hop\nend\nwait 2s"), 8));
    assert_eq!(
        asset_line(&workshop_base::asset::Asset::Gesture { who: "eevee".to_string(), words: "sit".to_string() }, Some("pikachu")).as_deref(),
        Some("pikachu sit")
    );
    assert_eq!(asset_line(&workshop_base::asset::Asset::Place { word: "beach".to_string() }, Some("pikachu")), None);
}

/// Each line on the track of who does it; the words of its bar there.
#[test]
fn each_line_goes_on_the_track_of_who_does_it() {
    let cast = vec!["pikachu".to_string(), "piplup".to_string(), "pichu".to_string()];
    let lane = |line: &str| lane_of(line, None, &cast);
    assert_eq!(lane("pikachu hop"), Lane::Who("pikachu".to_string()));
    assert_eq!(lane("  piplup (happy): Hi!"), Lane::Who("piplup".to_string()));
    assert_eq!(
        (lane("glide to 1 2 in 1s"), lane("music calm"), lane("transition fade out 1s"), lane("wait 1s")),
        (Lane::Camera, Lane::Sound, Lane::Screen, Lane::Other)
    );
    assert_eq!(lane("put object Apple 1 2"), Lane::Other);
    let read = film::ReadAs {
        word: "walk".to_string(),
        group: "moving",
        values: vec![film::grammar::Value {
            name: "who".to_string(),
            words: vec!["piplup".to_string()],
            kind: film::grammar::Kind::Someone,
            at: Some(0),
            span: None,
        }],
    };
    assert_eq!(lane_of("walk piplup to 1 2", Some(&read), &cast), Lane::Who("piplup".to_string()), "word first: by its form");
    assert_eq!(bar_words("pikachu hop happy &", &Lane::Who("pikachu".to_string())), "∥ hop happy");
    assert_eq!(bar_words("pichu (bubble, surprised): Whoa...", &Lane::Who("pichu".to_string())), "(surprised) “Whoa...”");
    assert_eq!(bar_words("wait 1s", &Lane::Other), "wait 1s");
}

/// Each edit is one change of the Script: one step back gives the text as it was.
#[test]
fn each_timeline_edit_is_one_undo() {
    let (made, ..) = bar_moved(FILM, &course(), None, 6, 60, 30, false, &Lane::Who("pikachu".to_string()), &Lane::Who("piplup".to_string())).unwrap();
    let mut open = workshop_script::Script::of_text(FILM);
    open.set_text(made.clone());
    assert_eq!(open.text, made);
    let mut undoer: eframe::egui::util::undoer::Undoer<String> = eframe::egui::util::undoer::Undoer::default();
    undoer.feed_state(0.0, &FILM.to_string());
    undoer.feed_state(0.1, &open.text);
    undoer.feed_state(5.0, &open.text);
    assert_eq!(undoer.undo(&open.text).map(String::as_str), Some(FILM), "one undo: as it was");
}

/// What each edit writes reads with nothing wrong, as the quick lines a track offers do.
#[test]
fn the_films_written_still_read() {
    let lane = |who: &str| Lane::Who(who.to_string());
    let written = [
        retimed(FILM, 6, 30, false).unwrap().0,
        retimed(FILM, 6, -90, false).unwrap().0,
        retimed(FILM, 9, 30, false).unwrap().0,
        toggled_beside(FILM, 6).unwrap().0,
        toggled_beside(FILM, 9).unwrap().0,
        bar_moved(FILM, &course(), None, 6, 60, 30, false, &lane("pikachu"), &lane("piplup")).unwrap().0,
        with_performer(FILM, 9, None, "pikachu", "piplup").unwrap(),
        inserted_in_sequence(FILM, &course(), 150, "pikachu nod").unwrap().0,
        inserted_in_sequence(FILM, &course(), 500, "piplup: Bye.").unwrap().0,
        inserted_beside(FILM, &course(), 150, "pikachu nod").unwrap().0,
        moved_beside(&toggled_beside(FILM, 6).unwrap().0, &course(), 6, 150).unwrap().0,
    ];
    assert_eq!(wrong(FILM), Vec::<String>::new());
    assert!(!wrong(&format!("{FILM}\npikachu frobnicate_twice")).is_empty(), "a word the language does not know is found");
    for text in &written {
        assert_eq!(wrong(text), Vec::<String>::new(), "{text}");
    }
    let mut offered: Vec<String> = Vec::new();
    for track in [lane("pikachu"), Lane::Camera, Lane::Screen, Lane::Other] {
        offered.extend(super::adding::offered(&track, Some([300.0, 242.0]), Some([300.0, 200.0])).into_iter().map(|(_, line)| line));
    }
    let film = format!("{FILM}\n{}\ntransition fade in 1s", offered.join("\n"));
    assert_eq!(wrong(&film), Vec::<String>::new(), "{film}");
}
