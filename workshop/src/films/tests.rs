//! The tests of the film played: stepping line by line.

use super::playing::{jumped, next_step};

#[test]
fn stepping_goes_line_by_line_even_on_one_frame() {
    // (Lines 2 and 3 start on the same frame; line 5 later.)
    let steps = [(0, 1), (10, 2), (10, 3), (40, 5)];
    assert_eq!(next_step(&steps, 0, None), Some((10, 2)));
    assert_eq!(next_step(&steps, 10, Some((10, 2))), Some((10, 3)), "the next line of the same frame");
    assert_eq!(next_step(&steps, 10, Some((10, 3))), Some((40, 5)));
    assert_eq!(next_step(&steps, 12, Some((10, 3))), Some((40, 5)), "gone elsewhere: from where it stands");
    assert_eq!(next_step(&steps, 40, Some((40, 5))), None, "the last line");
}

/// The Timeline: the bars on rows that do not cover each other, the songs and sounds of the film, the time's axis, the ruler's step, the next line's start, what a line is.
#[test]
fn the_timeline_lays_out_its_bars() {
    use eframe::egui::{Rect, pos2, vec2};
    use preview::played::{LineSpan, Timeline};

    use super::bands::{rows_of, songs_and_sounds};
    use super::timeline::{Axis, kind_of_line, next_start, second_step, speaker_of, zoom_to_fill};
    let span = |number: i32, start: usize, end: usize| LineSpan { number, start, end, ..LineSpan::default() };
    let spans = vec![span(1, 0, 30), span(2, 10, 20), span(3, 32, 40), span(4, 31, 33)];
    let (placed, rows) = rows_of(&spans);
    let row_of = |number: i32| placed.iter().find(|(span, _)| span.number == number).map(|(_, row)| *row);
    assert_eq!(rows, 2);
    assert_eq!((row_of(1), row_of(2), row_of(4), row_of(3)), (Some(0), Some(1), Some(1), Some(0)), "a row free again 2 frames after the end");
    let lines = ["music calm", "pikachu: Hi", "sound bell", "music off"];
    let (songs, sounds) = songs_and_sounds(&lines, |line| Some(line * 10));
    assert_eq!(songs, vec![(10, "calm".to_string()), (40, "off".to_string())]);
    assert_eq!(sounds, vec![(30, "bell".to_string())]);
    let axis = Axis { content: Rect::from_min_size(pos2(100.0, 0.0), vec2(600.0, 10.0)), left: 60.0, per_frame: 2.0, end: 600 };
    assert_eq!(axis.x_of(60.0), 100.0);
    assert_eq!(axis.frame_of(300.0), 160);
    assert_eq!(axis.frame_of(5000.0), 600, "no further than the film");
    assert_eq!(second_step(100.0), 1.0);
    assert_eq!(second_step(10.0), 10.0);
    assert_eq!(zoom_to_fill(620.0, 600), 60.0);
    let timeline = Timeline {
        spans: vec![span(1, 0, 10), span(2, 30, 40), LineSpan { number: 3, start: 20, end: 25, thread: 1, ..LineSpan::default() }],
        ..Timeline::default()
    };
    assert_eq!(next_start(&timeline, 0, true), Some(30), "the lines beside the film skipped");
    assert_eq!(next_start(&timeline, 30, false), Some(0));
    assert_eq!(next_start(&timeline, 30, true), None);
    assert_eq!((kind_of_line("pikachu (joyous): Hi"), kind_of_line("wait 1s"), kind_of_line("camera 0 0"), kind_of_line("walk pikachu")), (0, 1, 3, 4));
    assert_eq!(speaker_of("piplup: Hey"), Some("piplup".to_string()));
    assert_eq!(speaker_of("walk piplup"), None);
}

/// The Inspector's sums: a line said written again, a number as a field writes it.
#[test]
fn the_inspector_writes_lines_again() {
    use super::inspector::{number_said, spoken_line};
    assert_eq!(spoken_line("  ", "riolu", "happy", "fast, top", "Hello\nthere "), "  riolu (happy, fast, top): Hello there");
    assert_eq!(spoken_line("", "riolu", "", " ", "Hi"), "riolu: Hi");
    assert_eq!((number_said(12.0), number_said(-0.5)), ("12".to_string(), "-0.5".to_string()));
}

#[test]
fn a_frame_far_from_the_last_shown_is_gone_to() {
    assert!(jumped(None, 0));
    assert!(!jumped(Some(10), 10));
    assert!(!jumped(Some(10), 12), "played on to, a frame or two later");
    assert!(jumped(Some(10), 13));
    assert!(jumped(Some(10), 9), "back");
}

/// A field the Inspector changes on a line written who first: written again
/// in place, who before the word, the rest after it.
#[test]
fn the_inspector_edits_a_field_of_a_line_written_who_first() {
    use film::grammar::{Kind, Value};

    use super::lines::with_field;
    let value = |name: &str, words: &[&str], kind: Kind, at: usize| Value {
        name: name.to_string(),
        words: words.iter().map(|word| word.to_string()).collect(),
        kind,
        at: Some(at),
        span: None,
    };
    // (Read as the word first: `walk pikachu to piplup slow`.)
    let values = vec![value("who", &["pikachu"], Kind::Someone, 0), value("where", &["piplup"], Kind::Place, 2), value("slow", &["slow"], Kind::Word, 3)];
    let read = film::ReadAs { word: "walk".to_string(), family: "moving", values };
    let text = "scene: beach pikachu piplup eevee\n  pikachu walk to piplup slow\n";
    let words = |said: &str| said.split_whitespace().map(str::to_string).collect::<Vec<String>>();
    assert_eq!(with_field(text, 2, &read, "who", &words("eevee")).as_deref(), Ok("scene: beach pikachu piplup eevee\n  eevee walk to piplup slow\n"));
    assert_eq!(
        with_field(text, 2, &read, "where", &words("front_of eevee")).as_deref(),
        Ok("scene: beach pikachu piplup eevee\n  pikachu walk to front_of eevee slow\n")
    );
    assert!(with_field("scene: beach\n  music calm\n", 2, &read, "who", &words("eevee")).is_err(), "another word's line");
}
