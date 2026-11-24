//! The view. `view`, `frame`, `shot` and `camera` are one family: what the
//! view shows, how close, whether it stays. Here they are only translated
//! into the camera's beats (`engine/src/stage/camera/`): how a group is
//! framed, how close a shot comes for a body, how a glide eases, all of it
//! is the camera's.

use engine::{Cam, Ease, LIFT};

use film_core::film::Film;
use film_core::spots::routes::found;
use film_core::spots::{Aim, nowhere, point_said};
use film_core::verbs::{Line, Names, Word};
use film_core::words::{SHOTS, plain, spot};
use film_core::{emit, fields};

pub const WORDS: &[Word] = &[
    Word::new("frame", &["free", "members... snap", "members... [follows]"], "frame, waitcam; keepframe (follows); camera (snap: at once); a place: pan")
        .family("Camera")
        .ds_direct("MovePositionMark<performer 0> (the camera, camera_SetMyself) to between them; snap: SetPositionMark")
        .maybe(viewed),
    Word::new("glide", &["to where:place [in d:duration]", "where:word [d:duration]"], "pan to someone or a place, slowly at its ends")
        .family("Camera")
        .ds_direct("MovePositionMark<performer 0>(speed, Position), WaitExecutePerformer(0)")
        .maybe(viewed),
    Word::new("moment", &["title:text..."], "nothing for the engine: a moment of the scene (moment: title), and one framing alone in it")
        .family("Structure")
        .ds_direct("a @label")
        .names(Names::NoOne)
        .maybe(viewed),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("view", WORDS);

/// The words that frame the view: one in a moment.
const FRAMING_WORDS: [&str; 7] = ["camera", "view", "frame", "shot", "glide", "scroll", "dolly"];

/// The frames of a glide, a framing, a shot, by default.
const GLIDE: i32 = 30;
const FRAMING: i32 = 40;
const GLISSE: &str = "60";
/// The seconds of `camera from a to b` when it says none.
const CROSSING: f32 = 2.0;

/// What the view goes to, as a line says it: someone, or a place, as the
/// fields of a `pan`. None if it is neither, told.
pub fn aim(film: &mut Film, words: &[String], number: usize) -> Option<Vec<String>> {
    if words.len() == 1 && film.actor(&words[0]).is_some() {
        return Some(vec![plain(&words[0])]);
    }
    match point_said(film, words, number) {
        Some(Aim::At(x, y)) => Some(fields!["@", x, y]),
        Some(Aim::Found(spot)) => {
            let [x, y] = found(film, "-", spot, &["exact"]).fields();
            Some(fields!["@", x, y])
        }
        None => {
            nowhere(film, words, number, "where does the camera go?");
            None
        }
    }
}

/// Where the view goes, as `aim` gives it: someone, or a point (written,
/// or found as it plays: `{_place3_x}`); none for no one.
pub(super) fn cam_of(film: &mut Film, goal: &[String]) -> Option<Cam> {
    match goal {
        [at, x, y] if at == "@" => emit::point(x, y).map(Cam::At),
        [who] => film.other(who).map(Cam::Who),
        _ => None,
    }
}

/// The view glides there, staying if asked; the film waits for it if told.
fn pan(film: &mut Film, goal: Vec<String>, frames: impl ToString, stays: bool, wait: bool) {
    let frames = frames.to_string();
    match (cam_of(film, &goal), frames.parse::<f32>()) {
        (Some(to), Ok(frames)) => film.beat(emit::pan(to, frames, stays)),
        _ => film.unread(format!("{} {frames}", goal.join(" "))),
    }
    if wait {
        film.beat(emit::wait_cam());
    }
}

/// The view goes where they all show, no closer than `most` if said, and
/// stays.
fn frame(film: &mut Film, names: Vec<String>, most: Option<f32>, wait: bool) {
    framed_in(film, names, most, wait, FRAMING as f32);
}

/// The same, in so many frames (0: at once, a cut).
fn framed_in(film: &mut Film, names: Vec<String>, most: Option<f32>, wait: bool, frames: f32) {
    film.beat(emit::keep_frame(None));
    match film.bodies(&names).filter(|who| !who.is_empty()) {
        Some(who) => film.beat(emit::frame(who, frames, most)),
        None => film.not_a(names.join(" "), "Pokémon of the scene"),
    }
    if wait {
        film.beat(emit::wait_cam());
    }
}

/// `frame riolu ampharos lamp`: people and a place of the scene together,
/// the view where they all show (a place in numbers, the view framing the
/// people only if it is found as the film plays). False if the line does
/// not name both.
fn framed_with_a_place(film: &mut Film, line: &Line, snap: bool) -> bool {
    let members = line.args.words("members").to_vec();
    let (mut people, mut place) = (Vec::new(), Vec::new());
    for word in &members {
        match place.is_empty() && film.those(std::slice::from_ref(word)).is_some() {
            true => people.push(word.clone()),
            false => place.push(word.clone()),
        }
    }
    if people.is_empty() || place.is_empty() || line.args.has("follows") {
        return false;
    }
    let Some(goal) = aim(film, &place, line.number) else {
        return true;
    };
    let point = match goal.as_slice() {
        [at, x, y] if at == "@" => x.parse::<f32>().ok().zip(y.parse::<f32>().ok()).map(|(x, y)| glam::Vec2::new(x, y)),
        _ => None,
    };
    let names = film.those(&people).unwrap_or_default();
    film.beat(emit::keep_frame(None));
    match film.bodies(&names).filter(|who| !who.is_empty()) {
        Some(who) => film.beat(emit::frame_with(who, point.into_iter().collect(), if snap { 0.0 } else { FRAMING as f32 }, None)),
        None => film.not_a(names.join(" "), "Pokémon of the scene"),
    }
    if line.wait && !snap {
        film.beat(emit::wait_cam());
    }
    true
}

/// The view keeps them all in it as they move.
fn keep(film: &mut Film, names: Vec<String>) {
    match film.bodies(&names).filter(|who| !who.is_empty()) {
        Some(who) => film.beat(emit::keep_frame(Some(who))),
        None => film.not_a(names.join(" "), "Pokémon of the scene"),
    }
}

/// The size of the shot a line says (`wide`, `medium`, `close`), if one.
fn shot_size(line: &Line) -> Option<f32> {
    SHOTS.iter().find(|(word, _)| line.args.has(word)).map(|(_, close)| *close)
}

/// `shot medium a [follows]`: how much is seen round someone or a place, never
/// nearer than the view's own, further if its body asks for it; it stays on a,
/// or stays where it went. Clear of the box when a line shows.
fn shot(film: &mut Film, line: &Line, close: f32) {
    let words = line.args.words("target").to_vec();
    let follows = line.args.has("follows");
    let goal = if words.is_empty() { Some(fields!["@", film.middle.0, film.middle.1]) } else { aim(film, &words, line.number) };
    let Some(goal) = goal else {
        return;
    };
    let alone = goal[0] == "@";
    film.beat(emit::keep_frame(None));
    film.beat(emit::box_room(true));
    match if alone { Some(None) } else { film.other(&goal[0]).map(Some) } {
        Some(who) => film.beat(emit::shot(close, who)),
        None => film.not_a(&goal[0], "a Pokémon of the scene"),
    }
    pan(film, goal, FRAMING, !follows && !alone, line.wait);
}

/// `camera from a to b in 3s [smooth|linear|out]`: the view starts on one and
/// glides to the other, slowly at its ends (as the game's), evenly, or
/// quickly then slowing.
fn glide(film: &mut Film, line: &Line) {
    let frames = line.args.frames("d").unwrap_or((CROSSING * 60.0) as i32);
    let start = aim(film, line.args.words("start"), line.number);
    let goal = aim(film, line.args.words("goal"), line.number);
    if let (Some(start), Some(goal)) = (start, goal) {
        film.beat(emit::keep_frame(None));
        match cam_of(film, &start) {
            Some(to) => film.beat(emit::camera(Some(to))),
            None => film.unread(start.join(" ")),
        }
        // (Evenly, or quickly then slowing: told before the glide begins, as it was after.)
        let ease = [("linear", Ease::Linear), ("out", Ease::Out(engine::EaseFamily::Quad))].into_iter().find(|(word, _)| line.args.has(word));
        if let Some((_, ease)) = ease {
            film.beat(emit::ease(ease));
        }
        pan(film, goal, frames, false, line.wait);
        if ease.is_some() {
            film.beat(emit::ease(Ease::Smooth));
        }
    }
}

/// `camera ...`: on someone or a place, following someone or a group.
fn camera(film: &mut Film, line: &Line) -> bool {
    let (args, number) = (&line.args, line.number);
    let who = args.word("who").map(plain);
    if args.has("from") {
        glide(film, line);
    } else if args.has("on") {
        // (`camera on front_of a`, `camera on tile 12 7`, `camera on throne`.)
        match who {
            Some(who) => pan(film, vec![who], GLIDE, false, true),
            None => {
                if let Some(goal) = aim(film, args.words("where"), number) {
                    pan(film, goal, GLIDE, false, line.wait);
                }
            }
        }
    } else if args.has("follows") {
        match who {
            // (Before the scene shows, at once; then it glides there, and follows.)
            Some(who) if !film.staging.started => match film.other(&who) {
                Some(on) => film.beat(emit::camera(Some(Cam::Who(on)))),
                None => film.not_a(&who, "a Pokémon of the scene"),
            },
            Some(who) => {
                film.beat(emit::keep_frame(None));
                pan(film, vec![who], GLIDE, false, false);
            }
            // (`camera follows group a b c`, `camera follows onlookers`: the view keeps
            // them all in it as they move, as `view a b c`.)
            None => {
                let named = args.words("members").to_vec();
                match film.those(&named) {
                    Some(names) => keep(film, names),
                    None => film.wrong(number, format!("the camera follows whom? “{}”: Pokémon of the scene, or a crowd", named.join(" "))),
                }
            }
        }
    } else {
        return false;
    }
    true
}

fn view_word(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    match line.verb.as_str() {
        "camera" => return camera(film, line),
        "shot" => {
            let Some(close) = shot_size(line) else {
                return false;
            };
            // (`shot wide onlookers`: the whole crowd, never closer than the shot.)
            let aimed = args.words("target").to_vec();
            match film.names_a_crowd(&aimed) {
                true => {
                    if let Some(names) = film.those(&aimed) {
                        frame(film, names, Some(close), line.wait);
                    }
                }
                false => shot(film, line, close),
            }
        }
        // (`frame a b`: the view goes where they all show, and stays;
        // `frame a b follows`, as `view a b`, keeps them in it as they move; a
        // crowd's name is all of it.)
        // (At the head of a scene, before it shows, a cut: the scene opens on
        // its first framing.)
        "frame" => {
            let snap = args.has("snap") || !film.staging.started;
            if framed_with_a_place(film, line, snap) {
                return true;
            }
            let Some(names) = film.those(args.words("members")) else {
                // (`frame throne`, `frame left`: a place of the scene, the view still there.)
                let words = args.words("members").to_vec();
                // (`frame 30 a b`: the engine's, its frames first.)
                if words.len() > 2 && words[0].bytes().all(|byte| byte.is_ascii_digit()) {
                    return false;
                }
                let Some(goal) = aim(film, &words, line.number) else {
                    return true;
                };
                film.beat(emit::keep_frame(None));
                match (snap, cam_of(film, &goal)) {
                    (true, Some(on)) => film.beat(emit::camera(Some(on))),
                    _ => pan(film, goal, FRAMING, false, line.wait),
                }
                return true;
            };
            match (args.has("follows"), snap) {
                (true, _) => keep(film, names),
                (false, true) => framed_in(film, names, None, false, 0.0),
                (false, false) => frame(film, names, None, line.wait),
            }
        }
        "view" if args.has("free") => film.beat(emit::keep_frame(None)),
        // (A crowd's name: all of it.)
        "view" => {
            let named = args.words("members").to_vec();
            let names = named.iter().flat_map(|word| film.those(std::slice::from_ref(word)).unwrap_or_else(|| vec![word.clone()])).collect();
            keep(film, names);
        }
        // (`glide to piplup in 2s`, `glide to throne`: a light move of the view, slow at its ends.)
        "glide" if args.has("to") => {
            let words = args.words("where").to_vec();
            if let Some(goal) = aim(film, &words, line.number) {
                let goal = match goal.as_slice() {
                    [at, x, y] if at == "@" => fields!["@", x, y.parse::<i32>().map_or(y.clone(), |y| (y - LIFT as i32).to_string())],
                    _ => goal,
                };
                pan(film, goal, args.frames("d").unwrap_or(GLISSE.parse().unwrap_or(60)), false, line.wait);
            }
        }
        "glide" if args.word("where").is_some_and(|word| spot(&plain(word), film.middle).is_some()) => {
            let (x, y) = spot(&plain(args.word("where").unwrap_or_default()), film.middle).unwrap_or_default();
            pan(film, fields!["@", x, y - LIFT as i32], args.word("d").unwrap_or(GLISSE), false, line.wait);
        }
        // (`scroll 6s 300 700`: the view goes down a tall picture, y to y;
        // `dolly 6s 100 500`, along a wide one, x to x.)
        "scroll" | "dolly" => {
            let (x, y) = film.middle;
            let (from, to) = (args.word("from").unwrap_or_default().to_string(), args.word("to").unwrap_or_default().to_string());
            let ((x0, y0), (x1, y1)) =
                if line.verb == "scroll" { ((x.to_string(), from), (x.to_string(), to)) } else { ((from, y.to_string()), (to, y.to_string())) };
            match cam_of(film, &["@".to_string(), x0.clone(), y0.clone()]) {
                Some(on) => film.beat(emit::camera(Some(on))),
                None => film.not_a(format!("{x0} {y0}"), "a point"),
            }
            pan(film, fields!["@", x1, y1], args.frames("d").unwrap_or(0), false, false);
            let length = args.word("d").unwrap_or_default();
            match film_core::words::frames_of(length) {
                Some(frames) => film.wait(i64::from(frames)),
                None => film.not_a(length, "a duration"),
            }
        }
        _ => return false,
    }
    true
}

/// One family of words for one thing, whichever is written: `view follows a`,
/// `view on a`, `camera ...`; `view wide a`, `shot ...`; `shot a b`, `frame a
/// b`; `frame free`, `view free`. What it reads as, if it is not itself.
fn one_family(film: &Film, line: &Line) -> Option<&'static str> {
    let args = &line.args;
    match line.verb.as_str() {
        "view" if ["follows", "on", "from"].iter().any(|word| args.has(word)) => Some("camera"),
        "view" if shot_size(line).is_some() => Some("shot"),
        "frame" if args.has("free") => Some("view"),
        "shot" if args.has("members") => Some("frame"),
        "shot" if args.word("group").is_some_and(|group| film.groups.contains_key(&plain(group))) => Some("frame"),
        _ => None,
    }
}

/// A line of the view: one reader for all its words, each read as the one it
/// is (`view follows a` is `camera follows a`); a moment framed once.
fn viewed(film: &mut Film, line: &Line) -> bool {
    // (`moment: the gust`: a moment of the scene; its view is framed once.)
    if line.verb == "moment" {
        let title = line.args.words("title").join(" ");
        film_core::groups::story::moment_starts(film, &title);
        film.moment = Some((title, false));
        return true;
    }
    // (Read again as the word it is, by its own forms.)
    let played = match one_family(film, line) {
        Some(verb) => {
            let as_it_is = Line::word(film, "view", verb, line.args.said(), line);
            view_word(film, &as_it_is)
        }
        None => view_word(film, line),
    };
    if played
        && FRAMING_WORDS.contains(&line.verb.as_str())
        && let Some((title, framed)) = film.moment.as_mut()
        && std::mem::replace(framed, true)
    {
        let title = title.clone();
        film.remark(line.number, format!("“{}”: the moment “{title}” already has its framing; one a moment (a new “moment:” for another)", line.text));
    }
    played
}
