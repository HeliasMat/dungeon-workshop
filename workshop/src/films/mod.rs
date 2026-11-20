//! The Film space played: the film of the Script played on a thread of its
//! own (`preview::session::Session`, as `check` plays it: a change plays
//! again only from the line changed), any of its frames drawn there at
//! once; heard as it plays. Its panels:
//! - the Scene: the frame as the game shows it, its pixels whole; a bar to
//!   play it, go anywhere in it, faster or slower, heard or not, following
//!   the Script's cursor;
//! - the Timeline: its scenes, each line when it plays (a bar from its
//!   start to its end), who is on the stage when; a click goes there (and
//!   to the line in the Script);
//! - the Inspector: the line of the Script's cursor, when it plays, what
//!   the language says of it;
//! - the State: who is where at the moment shown, the camera, the objects
//!   (`preview::state`, as `scene_player state` tells it).
//!
//! Stepping by line, the film stops at the start of each line of its own
//! course as it plays; "Next Line" plays on to the next one.

use std::sync::Arc;

use crate::session::Session;
use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui::{self, Vec2, vec2};
use preview::played::Timeline;

use super::script::Scripts;
use playing::{line_frame, next_step, steps, work};

mod bands;
mod inspector;
mod lines;
mod playing;
mod scene;
mod state;
#[cfg(test)]
mod tests;
mod timeline;

pub use inspector::inspector;
pub use scene::scene;
pub use state::state;
pub use timeline::timeline;

/// Frames a second, as the game goes.
const SECOND: f64 = 60.0;
/// How long after the last change the film is played again.
const PLAYED_AFTER: Duration = Duration::from_millis(400);

/// What the window asks of the film's thread.
enum Ask {
    /// The film's text, of that revision: played (again).
    Text(u64, String),
    /// That frame drawn (the whole place, or what the view shows).
    Show(usize, bool),
    /// Heard or not.
    Heard(bool),
    /// Playing, or paused (what is heard paused with it).
    Playing(bool),
}

/// What the film's thread answers.
enum Answer {
    Played(Box<Played>),
    Frame(Box<Frame>),
    Failed(String),
}

/// A frame drawn: its pixels; how the place lies on it (the point of the
/// place at its middle, how many of its pixels one of the place takes); the
/// camera's frame on it (the whole place drawn); who is seen (their names,
/// the box around each, in its pixels).
pub struct Frame {
    frame: usize,
    wide: usize,
    high: usize,
    /// So many of its pixels for one of the game's (wide and high are its own).
    scale: usize,
    rgba: Vec<u8>,
    centre: [f32; 2],
    close: f32,
    camera: Option<[f32; 4]>,
    bodies: Vec<(String, [f32; 4])>,
    /// Where each one stands in the place (their feet), by name.
    feet: BTreeMap<String, [f32; 2]>,
    /// The film's numbers at that frame.
    numbers: BTreeMap<String, f64>,
    /// Who is where then, the camera, the objects (the State panel).
    state: preview::state::Moment,
}

/// A film played: of that revision, its last frame, when things are, what
/// is wrong, what each line was read as, who is in it.
struct Played {
    revision: u64,
    end: usize,
    timeline: Option<Timeline>,
    wrong: Vec<(i32, String)>,
    fields: BTreeMap<usize, film::ReadAs>,
    cast: Vec<String>,
}

/// The film played, as the window knows it.
pub struct Films {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    asks: Sender<Ask>,
    answers: Receiver<Answer>,
    /// The Script's revision sent to be played, the one played; its last frame, when things are.
    sent: u64,
    played: u64,
    pub end: usize,
    pub timeline: Option<Timeline>,
    /// What playing it found wrong (its line, what).
    pub wrong: Vec<(i32, String)>,
    pub failed: Option<String>,
    /// The frame shown, and its picture; the frame asked last.
    shown: Option<usize>,
    texture: Option<egui::TextureHandle>,
    picture_size: Vec2,
    asked: Option<usize>,
    /// Where it stands (in frames), whether it plays, how fast, since when.
    pub at: f64,
    pub playing: bool,
    /// The film's numbers at the frame shown; the breakpoint it last stopped at.
    pub numbers: BTreeMap<String, f64>,
    stopped_at: Option<usize>,
    pub speed: f64,
    last_tick: Option<Instant>,
    pub heard: bool,
    /// Whether the thread was told it plays.
    told_playing: bool,
    /// The film goes where the Script's cursor is.
    pub follow: bool,
    followed_line: usize,
    /// The Timeline: pixels a second.
    pub zoom: f32,
    /// The Scene: the frame shown as it was drawn (its pixels kept, to be written out);
    /// the whole place drawn, the names shown, the grid; played round and round.
    drawn: Option<Box<Frame>>,
    pub whole: bool,
    pub names: bool,
    pub grid: bool,
    pub looped: bool,
    said: Option<String>,
    /// The Timeline: the frame at its left edge; it follows the playhead;
    /// a passage played round (and where its drag began).
    left: f32,
    pub follow_head: bool,
    /// What each line was read as when the film last played; who is in it;
    /// a field to be chosen on the Scene (its line, its name); fields being typed.
    pub fields: BTreeMap<usize, film::ReadAs>,
    pub cast: Vec<String>,
    pub picking: Option<(usize, String)>,
    typing: std::collections::HashMap<(usize, String), String>,
    pub loop_range: Option<(usize, usize)>,
    loop_start: Option<usize>,
    /// The Scene looked at closer (1: fitted) and moved; what the mouse holds there.
    look: f32,
    pan: Vec2,
    held: Option<Held>,
    /// The Timeline: the kinds of lines hidden; the one whose lines stand out;
    /// a bar's end held (its line, its start); the bar a menu is opened on (line, start, end).
    hidden_kinds: [bool; 5],
    focus_who: Option<String>,
    stretching: Option<(usize, usize)>,
    menu_bar: Option<(usize, usize, usize)>,
    /// Step by line: the film stops at the start of each line of its own
    /// course; the start it plays to (its frame, its line); the one it stopped at.
    pub step: bool,
    target: Option<(usize, usize)>,
    stepped: Option<(usize, usize)>,
}

/// What a bar's menu does.
enum BarAct {
    Script(usize),
    Loop(usize, usize),
    Wait(usize),
    Twice(usize),
    Gone(usize),
}

/// What a drag on the Scene holds: someone (their name, from their feet to
/// where they were taken), the picture itself, or a measure (from that point).
enum Held {
    Someone(String, [f32; 2]),
    Picture,
    Measure([f32; 2]),
}

impl Films {
    pub fn new(session: &Arc<Session>, context: &egui::Context) -> Films {
        let (asks, asked) = channel::<Ask>();
        let (answer, answers) = channel::<Answer>();
        let waker = context.clone();
        let worked = session.clone();
        std::thread::spawn(move || work(&worked, &asked, &answer, &waker));
        Films {
            session: session.clone(),
            asks,
            answers,
            sent: 0,
            played: 0,
            end: 0,
            timeline: None,
            wrong: Vec::new(),
            failed: None,
            shown: None,
            texture: None,
            picture_size: vec2(256.0, 192.0),
            asked: None,
            at: 0.0,
            playing: false,
            numbers: BTreeMap::new(),
            stopped_at: None,
            speed: 1.0,
            last_tick: None,
            heard: true,
            told_playing: false,
            follow: true,
            followed_line: 0,
            zoom: 60.0,
            drawn: None,
            whole: false,
            names: false,
            grid: false,
            looped: false,
            said: None,
            left: 0.0,
            follow_head: true,
            fields: BTreeMap::new(),
            cast: Vec::new(),
            picking: None,
            typing: std::collections::HashMap::new(),
            loop_range: None,
            loop_start: None,
            look: 1.0,
            pan: Vec2::ZERO,
            held: None,
            hidden_kinds: [false; 5],
            focus_who: None,
            stretching: None,
            menu_bar: None,
            step: session.launch.step.as_deref() == Some("1"),
            target: None,
            stepped: None,
        }
    }

    /// (`WORKSHOP_INSTANT=secondes`: the film there; `WORKSHOP_SCENE=entier,noms,grille`: so shown; for a picture.)
    pub fn asked_instant(&mut self) {
        if let Some(seconds) = self.session.launch.instant {
            (self.at, self.follow) = (seconds * SECOND, false);
        }
        // (`WORKSHOP_STEP=1`: stepping by line, played from where it is: stopped at the next line.)
        if self.step && self.session.launch.step.is_some() {
            (self.playing, self.follow) = (true, false);
        }
        let shown = self.session.launch.scene.clone().unwrap_or_default();
        (self.whole, self.names, self.grid) = (shown.contains("whole"), shown.contains("names"), shown.contains("grid"));
    }

    /// The frame it stands at.
    pub fn frame(&self) -> usize {
        (self.at.max(0.0) as usize).min(self.end)
    }

    /// Gone to that frame (stopped there if it played past the end).
    pub fn go(&mut self, frame: usize) {
        self.at = frame.min(self.end) as f64;
    }

    /// Each frame: what the thread answered taken; the Script's text sent to
    /// be played a moment after it changed; the film going on as it plays;
    /// the frame it stands at asked.
    pub fn tick(&mut self, context: &egui::Context, scripts: &mut Scripts) {
        while let Ok(answer) = self.answers.try_recv() {
            match answer {
                Answer::Played(played) => {
                    let Played { revision, end, timeline, wrong, fields, cast } = *played;
                    let first = self.played == 0;
                    (self.played, self.end, self.timeline, self.wrong, self.failed) = (revision, end, timeline, wrong, None);
                    (self.fields, self.cast) = (fields, cast);
                    self.asked = None;
                    if first {
                        // (Played the first time: where the Script's cursor is, as it follows it.)
                        self.followed_line = 0;
                        self.asked_instant();
                    }
                }
                Answer::Frame(drawn) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied([drawn.wide, drawn.high], &drawn.rgba);
                    let options = egui::TextureOptions {
                        magnification: egui::TextureFilter::Nearest,
                        minification: egui::TextureFilter::Linear,
                        // (Drawn larger than shown: smaller, still clean.)
                        mipmap_mode: Some(egui::TextureFilter::Linear),
                        ..egui::TextureOptions::NEAREST
                    };
                    match &mut self.texture {
                        Some(texture) => texture.set(image, options),
                        None => self.texture = Some(context.load_texture("the film", image, options)),
                    }
                    (self.shown, self.picture_size) = (Some(drawn.frame), vec2((drawn.wide / drawn.scale) as f32, (drawn.high / drawn.scale) as f32));
                    self.numbers = drawn.numbers.clone();
                    self.drawn = Some(drawn);
                }
                Answer::Failed(why) => self.failed = Some(why),
            }
        }
        // (Played with other values ("Play with These Values"), or as the text has them again.)
        if let Some(text) = scripts.override_text.take() {
            self.sent = scripts.open.as_ref().map_or(0, |open| open.revision());
            _ = self.asks.send(Ask::Text(self.sent, text));
        }
        // (A breakpoint reached as it plays: stopped there, once; F10: on to the next line.)
        if self.playing
            && let Some(line) = self.playing_line()
        {
            if scripts.breakpoints.contains(&line) && self.stopped_at != Some(line) {
                self.playing = false;
                self.stopped_at = Some(line);
                scripts.said = Some(format!("stopped at line {line} (breakpoint; F10: the next line, Space: the rest)"));
            } else if !scripts.breakpoints.contains(&line) {
                self.stopped_at = None;
            }
        }
        if std::mem::take(&mut scripts.step_asked) {
            self.next_line();
        }
        let Some(open) = &scripts.open else { return };
        // (Played again a moment after the text changed.)
        if open.revision() != self.sent && open.quiet() >= PLAYED_AFTER {
            self.sent = open.revision();
            _ = self.asks.send(Ask::Text(self.sent, open.text.clone()));
        }
        // (Following the Script: its cursor's line, when it plays.)
        let line = open.cursor_line();
        if self.follow && line != self.followed_line && !self.playing {
            self.followed_line = line;
            if let Some(frame) = self.timeline.as_ref().and_then(|timeline| line_frame(timeline, line)) {
                self.go(frame);
            }
        }
        // (Going on, as it plays.)
        let now = Instant::now();
        // (Step by line: played on to the next line's start, from where it stands.)
        if self.playing && self.step && self.target.is_none() {
            self.target = self.next_step();
        }
        if self.playing {
            let since = self.last_tick.map_or(0.0, |last| now.duration_since(last).as_secs_f64());
            self.at += since * SECOND * self.speed;
            // (A passage played round: back to its start at its end.)
            if let Some((a, b)) = self.loop_range.filter(|_| self.looped)
                && self.at >= b as f64
            {
                self.at = a as f64;
            }
            if self.at >= self.end as f64 {
                // (Round and round, or stopped at its end.)
                if self.looped && self.end > 0 {
                    self.at = 0.0;
                } else {
                    (self.at, self.playing) = (self.end as f64, false);
                }
            }
            // (The next line reached: stopped at its start, that line lit in the Script.)
            if let Some((frame, line)) = self.target
                && self.playing
                && self.at >= frame as f64
            {
                (self.at, self.playing) = (frame as f64, false);
                (self.stepped, self.stopped_at) = (Some((frame, line)), Some(line));
            }
            context.request_repaint();
        }
        if !self.playing {
            self.target = None;
        }
        self.last_tick = Some(now);
        // (Its sound with it: paused when it is, going on when it plays.)
        if self.playing != self.told_playing {
            self.told_playing = self.playing;
            _ = self.asks.send(Ask::Playing(self.playing));
        }
        let frame = self.frame();
        if self.played > 0 && self.asked != Some(frame) {
            self.asked = Some(frame);
            _ = self.asks.send(Ask::Show(frame, self.whole));
        }
    }

    pub fn hear(&mut self, on: bool) {
        self.heard = on;
        _ = self.asks.send(Ask::Heard(on));
    }

    /// The line the film plays at the frame it stands at (the last begun).
    pub fn playing_line(&self) -> Option<usize> {
        let frame = self.frame();
        let timeline = self.timeline.as_ref()?;
        timeline
            .spans
            .iter()
            .filter(|span| span.start <= frame && frame <= span.end.max(span.start) && span.label.is_none())
            .max_by_key(|span| span.start)
            .map(|span| span.number as usize)
    }

    /// When each line plays the first time and how long (on the film's own course), in seconds.
    pub fn timing(&self) -> BTreeMap<usize, (f64, f64)> {
        let mut out = BTreeMap::new();
        for span in self.timeline.iter().flat_map(|timeline| &timeline.spans).filter(|span| span.label.is_none()) {
            out.entry(span.number as usize).or_insert((span.start as f64 / SECOND, (span.end as f64 - span.start as f64) / SECOND));
        }
        out
    }

    /// The film gone to where that line plays (stopped there).
    pub fn go_to_line(&mut self, line: usize) {
        if let Some(frame) = self.timeline.as_ref().and_then(|timeline| line_frame(timeline, line)) {
            self.go(frame);
            self.playing = false;
            self.followed_line = line;
        }
    }

    /// Someone chosen (their lines brought out in the Timeline, their frame
    /// on the Scene, their row in the State), or no longer if they were.
    pub fn choose(&mut self, who: String) {
        self.focus_who = if self.focus_who.as_ref() == Some(&who) { None } else { Some(who) };
    }

    /// Played until the next line of the film's own course starts, stopped
    /// there ("Next Line", F10).
    pub fn next_line(&mut self) {
        self.target = self.next_step();
        self.playing = self.target.is_some();
    }

    /// The next start of a line after the one it stands at.
    fn next_step(&self) -> Option<(usize, usize)> {
        let timeline = self.timeline.as_ref()?;
        next_step(&steps(timeline), self.frame(), self.stepped)
    }

    /// The line it stopped at (stepping by line, "Next Line"), while it stands there.
    pub fn stepped_line(&self) -> Option<usize> {
        self.stepped.filter(|(frame, _)| !self.playing && *frame == self.frame()).map(|(_, line)| line)
    }

    /// Whether the film shown is of the Script as it is.
    pub fn fresh(&self, scripts: &Scripts) -> bool {
        scripts.open.as_ref().is_some_and(|open| open.revision() == self.played)
    }
}
