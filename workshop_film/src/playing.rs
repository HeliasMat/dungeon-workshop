//! The film played on a thread of its own: what is asked of it, what it
//! answers, the frames and steps of its timeline.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;
use preview::played::Timeline;

use super::{Answer, Ask, Frame, Played, SECOND};

/// The picture drawn so many times the game's pixels: a view that steps
/// back (zoom under 1) keeps every pixel of the place, shown smaller and
/// sharp, instead of skipping some.
const SHOWN_SCALE: usize = 3;

/// The frame a line first plays at (or the nearest line before it that plays).
pub fn line_frame(timeline: &Timeline, line: usize) -> Option<usize> {
    timeline.lines.range(..=line as i32).next_back().map(|(_, frame)| *frame)
}

/// Each start of a line on the film's own course, in the order they play:
/// its frame, its line (the lines played beside the film, `&`, and those of
/// a recipe left out).
pub(super) fn steps(timeline: &Timeline) -> Vec<(usize, usize)> {
    let mut steps: Vec<(usize, usize)> =
        timeline.spans.iter().filter(|span| span.label.is_none() && span.thread == 0).map(|span| (span.start, span.number as usize)).collect();
    steps.sort_by_key(|(frame, _)| *frame);
    steps
}

/// The start after the one stopped at (several lines may start on the same
/// frame: one after the other, the film standing still), or the first after
/// that frame.
pub(super) fn next_step(steps: &[(usize, usize)], frame: usize, stepped: Option<(usize, usize)>) -> Option<(usize, usize)> {
    if let Some(at) = stepped.filter(|(at, _)| *at == frame).and_then(|stepped| steps.iter().position(|step| *step == stepped)) {
        return steps.get(at + 1).copied();
    }
    steps.iter().find(|(at, _)| *at > frame).copied()
}

/// The film's thread: its session, its art, its sound; of the frames asked
/// meanwhile, the last only.
pub(super) fn work(own: &workshop_base::session::Session, asked: &Receiver<Ask>, answer: &Sender<Answer>, waker: &egui::Context) {
    let data = own.data().to_string();
    let send = |reply: Answer| {
        _ = answer.send(reply);
        waker.request_repaint();
    };
    let art = match preview::art::Art::open_in(&data, own.library().clone()) {
        Ok(art) => art,
        Err(why) => {
            send(Answer::Failed(why));
            return;
        }
    };
    let audio = shell::sound::Audio::open_in(&data, own.library().clone()).map(|audio| Box::new(audio) as Box<dyn shell::sound::Hears>);
    let mut worker = Worker {
        art: Some(art),
        tables: own.tables(),
        debug: own.launch.film_debug.clone(),
        audio,
        heard: true,
        paused: true,
        attached: false,
        session: None,
        shown: None,
        names: HashMap::new(),
        canvas: stage_view::ui::Canvas::default(),
        screen: shell::Screen::new(SHOWN_SCALE),
    };
    while let Ok(first) = asked.recv() {
        let mut waiting = vec![first];
        waiting.extend(asked.try_iter());
        let last_text = waiting.iter().rposition(|ask| matches!(ask, Ask::Text(..)));
        let last_show = waiting.iter().rposition(|ask| matches!(ask, Ask::Show(..)));
        for (at, ask) in waiting.into_iter().enumerate() {
            let reply = match ask {
                Ask::Text(revision, text) if Some(at) == last_text => worker.text(revision, &text),
                Ask::Heard(on) => {
                    worker.heard_on(on);
                    None
                }
                Ask::Playing(on) => {
                    worker.playing(on);
                    None
                }
                Ask::Show(frame, whole, staged) if Some(at) == last_show => worker.show(frame, whole, staged),
                Ask::Text(..) | Ask::Show(..) => None,
            };
            if let Some(reply) = reply {
                send(reply);
            }
        }
    }
}

/// What the film's thread keeps between two asks.
struct Worker {
    /// The art, while no session holds it.
    art: Option<preview::art::Art>,
    tables: Arc<film::tables::Tables>,
    /// The developer's options each film is played with (`DUNGEON_FILM_DEBUG`).
    debug: preview::debug::Debug,
    audio: Option<Box<dyn shell::sound::Hears>>,
    /// It plays, or is paused; its sound is on the film (attached), or apart, silent.
    heard: bool,
    paused: bool,
    attached: bool,
    session: Option<preview::session::Session>,
    shown: Option<usize>,
    names: HashMap<engine::Who, String>,
    canvas: stage_view::ui::Canvas,
    screen: shell::Screen,
}

/// Whether a frame shown after another was gone to, not played on to.
pub(super) fn jumped(shown: Option<usize>, frame: usize) -> bool {
    shown.is_none_or(|shown| frame != shown + 1 && frame != shown && frame != shown + 2)
}

impl Worker {
    /// The sound put on the film, going on.
    fn attach(&mut self) {
        if let Some(session) = self.session.as_mut()
            && !self.attached
        {
            if let Some(audio) = self.audio.as_mut() {
                audio.pause(false);
            }
            self.audio = session.hear_on(self.audio.take());
            self.attached = true;
        }
    }

    /// The sound taken off the film, silent.
    fn detach(&mut self) {
        if let Some(session) = self.session.as_mut()
            && self.attached
        {
            self.audio = session.hear_on(None).or(self.audio.take());
            self.attached = false;
        }
    }

    /// Its text changed by the least edit: played again only from there (all of it, the first time or when the
    /// edit cannot be made).
    fn text(&mut self, revision: u64, text: &str) -> Option<Answer> {
        self.detach();
        let made = self.session.as_mut().and_then(|session| session.doc().edit_to(text).map(|edit| session.edit(&edit)));
        if self.session.is_none() || made.as_ref().is_some_and(Result::is_err) {
            let art = self.art.take().or_else(|| self.session.take().map(preview::session::Session::into_art)).expect("what films play with");
            let mut session = preview::session::Session::new(text, art, self.tables.clone());
            session.debug = self.debug.clone();
            self.session = Some(session);
        }
        let session = self.session.as_mut()?;
        let report = session.play().clone();
        let wrong: Vec<(i32, String)> = report
            .read
            .iter()
            .filter(|problem| !problem.is_remark())
            .map(|problem| (if problem.span.file.is_none() { problem.line() as i32 } else { 0 }, problem.message.clone()))
            .chain(report.wrong.iter().map(|seen| (seen.line, seen.text.clone())))
            .collect();
        self.names = report.names.iter().cloned().collect();
        let mut cast: Vec<String> = report.names.iter().map(|(_, name)| name.clone()).collect();
        cast.sort();
        cast.dedup();
        // (Its first frame seen, looked for in its first minute.)
        let first_seen = session.first_seen(60 * super::SECOND as usize);
        let played =
            Played { revision, end: session.end().unwrap_or(0), timeline: report.timeline.clone(), wrong, fields: report.fields.clone(), cast, first_seen };
        self.shown = None;
        Some(Answer::Played(Box::new(played)))
    }

    fn heard_on(&mut self, on: bool) {
        self.heard = on;
        if self.heard && !self.paused {
            self.attach();
        } else {
            self.detach();
        }
    }

    fn playing(&mut self, on: bool) {
        self.paused = !on;
        let Some(session) = self.session.as_mut() else { return };
        if self.paused {
            // (Held where it is: it goes on from there.)
            session.pause_heard(true);
        } else if self.heard {
            if self.attached {
                session.pause_heard(false);
            } else {
                self.attach();
            }
        }
    }

    /// A frame drawn, and who is seen on it.
    fn show(&mut self, frame: usize, whole: bool, staged: bool) -> Option<Answer> {
        self.session.as_ref()?;
        // (Gone to, not played on to: silent there; heard from there if it plays.)
        if jumped(self.shown, frame) {
            self.detach();
            if let Some(session) = self.session.as_mut() {
                session.seek(frame);
            }
            if self.heard && !self.paused {
                self.attach();
            }
        }
        self.shown = Some(frame);
        let session = self.session.as_mut()?;
        let (canvas, screen) = (&mut self.canvas, &mut self.screen);
        let drawn = if staged {
            session.draw_staged(frame, whole, canvas, screen)
        } else if whole {
            session.draw_whole(frame, canvas, screen)
        } else {
            session.draw(frame, canvas, screen)
        };
        if !drawn {
            return None;
        }
        let film = session.seek(frame)?;
        let names = &self.names;
        let camera = film.view();
        let view = if whole { film.whole_view() } else { camera };
        // (Who is seen: the box around each, in the picture's pixels.)
        let stage = &film.play().sim.stage;
        let bodies: Vec<(String, [f32; 4])> = stage
            .cast
            .iter()
            .filter(|(_, actor)| actor.seen())
            .filter_map(|(who, actor)| {
                let feet = view.on_screen(actor.shown_at() - glam::Vec2::new(0.0, actor.shown_height()));
                let frame = film.play().drawn_box(*who, &view).unwrap_or_else(|| {
                    let size = stage.view.body(*who) * view.close;
                    [feet.x - size.x / 2.0, feet.y - size.y, feet.x + size.x / 2.0, feet.y]
                });
                Some((names.get(who)?.clone(), frame))
            })
            .collect();
        let feet: BTreeMap<String, [f32; 2]> = stage
            .cast
            .iter()
            .filter(|(_, actor)| actor.seen())
            .filter_map(|(who, actor)| Some((names.get(who)?.clone(), [actor.shown_at().x.round(), actor.shown_at().y.round()])))
            .collect();
        let half = camera.screen / 2.0 / camera.close;
        let camera_box = whole.then(|| {
            let (a, b) = (view.on_screen(camera.centre - half), view.on_screen(camera.centre + half));
            [a.x, a.y, b.x, b.y]
        });
        Some(Answer::Frame(Box::new(Frame {
            frame,
            staged,
            wide: screen.wide,
            high: screen.high,
            scale: screen.scale,
            rgba: screen.rgba.clone(),
            centre: [view.centre.x, view.centre.y],
            close: view.close,
            camera: camera_box,
            bodies,
            feet,
            numbers: film.play().sim.director.numbers_and_words().0.into_iter().collect(),
            state: preview::state::moment(film, &|who| names.get(&who).cloned().unwrap_or_else(|| film.play().sim.cast.who(who).1)),
        })))
    }
}

/// A time, as minutes and seconds.
pub(super) fn clock(frames: f64) -> String {
    let seconds = frames / SECOND;
    format!("{}:{:04.1}", (seconds / 60.0).floor() as i64, seconds % 60.0)
}
