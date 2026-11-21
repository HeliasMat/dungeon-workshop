//! A session goes to a moment as the film played to it from the start, and
//! says of which text it tells. (Those that need the extracted `data/` are
//! ignored: `cargo test -- --ignored`.)

use shell::Screen;
use stage_view::ui::Canvas;

use crate::art::Art;
use crate::session::{KEPT_EVERY, Session};
use film::doc::Edit;

/// The repository's shared library.
fn project() -> assets::project::Project {
    assets::testing::library()
}

/// The repository's data folder.
fn data() -> String {
    assets::testing::data()
}

/// The game's files, if they are there.
fn art() -> Option<Art> {
    let data = data();
    shell::data::read(&data, "dungeons.bin").map(|_| Art::open_in(&data, project()).expect("the game's files, read"))
}

/// The tables films are compiled with, read once for these tests.
fn tables() -> std::sync::Arc<film::tables::Tables> {
    static READ: std::sync::OnceLock<std::sync::Arc<film::tables::Tables>> = std::sync::OnceLock::new();
    READ.get_or_init(|| std::sync::Arc::new(film::tables::Tables::read(&project(), &data()))).clone()
}

const FILM: &str = "scene: beach pikachu piplup\npikachu put 300 200\npiplup put 360 210\npikachu walk to 420 200\npikachu: Shall we go?\npiplup jump\npiplup: Come on!\npiplup walk to 200 220\nwait all\n";

/// That frame of the session, drawn: its pixels.
fn drawn(session: &mut Session, frame: usize) -> Vec<u8> {
    let (mut canvas, mut screen) = (Canvas::default(), Screen::new(1));
    assert!(session.draw(frame, &mut canvas, &mut screen), "it plays");
    screen.rgba
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_moment_gone_to_is_the_moment_played_to() {
    let art = art().expect("the extracted data/");
    let mut session = Session::new(FILM, art, tables());
    let report = session.play();
    assert!(report.read.iter().all(|problem| problem.is_remark()), "{:?}", report.read);
    let end = session.end().expect("played");
    assert!(end > KEPT_EVERY * 2, "{end}");
    // (Played on from the start, frame by frame; then gone back to, straight.)
    let moments = [KEPT_EVERY - 1, KEPT_EVERY + 37, end - 5];
    let on: Vec<Vec<u8>> = moments.iter().map(|frame| drawn(&mut session, *frame)).collect();
    for (at, frame) in moments.iter().enumerate().rev() {
        assert!(drawn(&mut session, *frame) == on[at], "at frame {frame}, gone back to, it is no longer the same");
    }
    // (Drawn twice, the same: the drawing keeps nothing.)
    assert!(drawn(&mut session, moments[1]) == drawn(&mut session, moments[1]));
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn what_it_tells_is_of_the_text_it_played() {
    let art = art().expect("the extracted data/");
    let mut session = Session::new(FILM, art, tables());
    session.play();
    assert!(session.fresh());
    let tracks = &session.report().timeline.as_ref().expect("played").tracks;
    assert!(tracks.values().any(|track| !track.moving.is_empty()), "someone walks");
    session.edit(&Edit::Words { number: 4, at: 1, long: 2, with: vec!["240".to_string(), "200".to_string()] }).expect("an edit");
    assert!(!session.fresh(), "the text changed: what it says is of the old one");
    session.play();
    assert!(session.fresh());
    session.edit(&Edit::Line { number: 6, text: "nobody jump".to_string() }).expect("an edit");
    let report = session.play();
    assert!(report.read.iter().any(|problem| !problem.is_remark()), "a wrong word is told");
    assert!(report.timeline.is_none() && session.end().is_none(), "it does not play");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_field_is_set_where_its_form_read_it() {
    let art = art().expect("the extracted data/");
    let mut session = Session::new(FILM, art, tables());
    session.play();
    let names: Vec<&str> = session.fields(2).iter().map(|value| value.name.as_str()).collect();
    assert!(names.contains(&"where"), "put has its fields: {names:?}");
    session.set_field(2, "where", &["320".to_string(), "190".to_string()]).expect("set");
    assert_eq!(session.doc().line(2), Some("pikachu put 320 190"));
    assert!(session.set_field(2, "where", &["1".to_string(), "2".to_string()]).is_err(), "the text changed since it played");
    session.play();
    assert!(session.report().read.iter().all(|problem| problem.is_remark()));
    assert!(session.set_field(2, "nobody", &[]).is_err());
}

/// A film long enough to keep a few moments: three scenes, people walking,
/// speaking, waiting.
const LONG: &str = "scene: beach pikachu piplup\npikachu put 300 200\npiplup put 360 210\npikachu walk to 420 200\npikachu: Shall we go?\nwait 2s\npiplup: Come on!\npiplup walk to 200 220\nwait all\n\nscene: town pikachu piplup\npikachu: The town!\npikachu walk to right\nwait 2s\npiplup: Wait for me!\nwait 2s\n\nscene: beach_evening pikachu\npikachu: Evening falls.\npikachu jump\nwait 2s\npikachu: Let's go home.\n";

/// What a play told, said whole (to be compared).
fn told(session: &Session) -> String {
    let report = session.report();
    format!("{:?}\n{:?}\n{:?}\n{:?}\n{:?}", session.end(), report.timeline, report.wrong, report.remarks, report.views)
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_change_late_in_a_film_plays_from_the_last_moment_still_true_as_if_played_from_its_start() {
    let art = art().expect("the extracted data/");
    let mut session = Session::new(LONG, art, tables());
    session.play();
    let end = session.end().expect("played");
    assert!(end > KEPT_EVERY * 4, "{end}");
    // (A line of the last scene changed: played on from a moment kept.)
    let number = LONG.lines().position(|line| line == "pikachu: Let's go home.").expect("its line") + 1;
    for (edit, late) in [
        (Edit::Line { number, text: "pikachu: Let's hurry home, it's cold!".to_string() }, true),
        (Edit::Line { number: 2, text: "pikachu put 280 200".to_string() }, false),
    ] {
        session.edit(&edit).expect("an edit");
        session.play();
        assert_eq!(session.began() > KEPT_EVERY, late, "taken up from frame {}", session.began());
        let changed = session.doc().text();
        let again = self::art().expect("the extracted data/");
        let mut whole = Session::new(&changed, again, tables());
        whole.play();
        assert_eq!(told(&session), told(&whole), "played from a moment kept, it tells something else than played from the start");
        let last = session.end().expect("played");
        for frame in [KEPT_EVERY / 2, last / 2, last - 3] {
            assert!(drawn(&mut session, frame) == drawn(&mut whole, frame), "at frame {frame}, not the same picture");
        }
    }
}
