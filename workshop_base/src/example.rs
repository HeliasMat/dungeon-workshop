//! A line played apart, as an example: a little film around it (Pikachu
//! and Eevee on the beach, framed, the line, a moment after), played on a
//! thread of its own, its frames kept; shown in a loop where a word is
//! chosen (the words by family, a move offered after `move`).

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use eframe::egui;

/// Frames a second, as the game goes; one kept in so many; at most so many kept.
const SECOND: f64 = 60.0;
const EVERY: usize = 3;
const KEPT: usize = 90;
/// Examples kept played, at most.
const REMEMBERED: usize = 12;

/// What the thread gives back: the line, and its frames (or why not).
type Played = (String, Result<Vec<egui::ColorImage>, String>);
/// What is asked of the thread: the film, whether its last picture only, the line its frames start a moment before (0: from the start).
type Asked = (String, bool, usize);

/// An example played: its frames, made pictures.
struct Clip {
    frames: Vec<egui::TextureHandle>,
    used: Instant,
}

/// The examples: asked (played, or their last picture only), played, kept (so many at most).
pub struct Examples {
    asks: Sender<Asked>,
    answers: Receiver<Played>,
    asked: Option<String>,
    waiting: HashSet<String>,
    clips: HashMap<String, Clip>,
    failed: HashMap<String, String>,
    began: Instant,
    room: usize,
}

impl Examples {
    pub fn new(session: &std::sync::Arc<crate::session::Session>, context: &egui::Context) -> Examples {
        Examples::with_room(session, context, REMEMBERED)
    }

    /// Kept, so many at most (the pictures of a list: many).
    pub fn with_room(session: &std::sync::Arc<crate::session::Session>, context: &egui::Context, room: usize) -> Examples {
        let (asks, asked) = channel::<Asked>();
        let (answer, answers) = channel::<Played>();
        let waker = context.clone();
        let session = session.clone();
        std::thread::spawn(move || work(&session, &asked, &answer, &waker));
        Examples { asks, answers, asked: None, waiting: HashSet::new(), clips: HashMap::new(), failed: HashMap::new(), began: Instant::now(), room }
    }

    /// What the thread played meanwhile, made pictures.
    fn receive(&mut self, context: &egui::Context) {
        while let Ok((said, played)) = self.answers.try_recv() {
            self.waiting.remove(&said);
            match played {
                Ok(images) => {
                    let options = egui::TextureOptions::NEAREST;
                    let frames = images.into_iter().enumerate().map(|(at, image)| context.load_texture(format!("example {at}"), image, options)).collect();
                    self.clips.insert(said, Clip { frames, used: Instant::now() });
                    if self.clips.len() > self.room
                        && let Some(oldest) = self.clips.iter().min_by_key(|(_, clip)| clip.used).map(|(said, _)| said.clone())
                    {
                        self.clips.remove(&oldest);
                    }
                }
                Err(why) => {
                    self.failed.insert(said, why);
                }
            }
        }
    }

    /// A film's last picture, if it is made (asked to be made if not); why not, if it cannot be.
    pub fn picture(&mut self, context: &egui::Context, film: &str) -> Result<Option<egui::TextureHandle>, String> {
        self.receive(context);
        if let Some(clip) = self.clips.get_mut(film).filter(|clip| !clip.frames.is_empty()) {
            clip.used = Instant::now();
            return Ok(clip.frames.last().cloned());
        }
        if let Some(why) = self.failed.get(film) {
            return Err(why.clone());
        }
        if self.waiting.insert(film.to_string()) {
            _ = self.asks.send((film.to_string(), true, 0));
        }
        Ok(None)
    }

    /// That film's example shown, at its size (asked to be played if it is not yet).
    pub fn show(&mut self, ui: &mut egui::Ui, film: &str, width: f32) {
        self.shown(ui, film, width, Some(8));
    }

    /// A film shown playing from a moment before that line of it (1: its start), at its size.
    pub fn show_from(&mut self, ui: &mut egui::Ui, film: &str, line: usize, width: f32) {
        self.shown(ui, film, width, Some(line));
    }

    /// That film's last picture (a place, as it is seen), at its size.
    pub fn still(&mut self, ui: &mut egui::Ui, film: &str, width: f32) {
        self.shown(ui, film, width, None);
    }

    /// Its frames from a moment before that line (`None`: its last picture only).
    fn shown(&mut self, ui: &mut egui::Ui, film: &str, width: f32, from_line: Option<usize>) {
        let moving = from_line.is_some();
        self.receive(ui.ctx());
        let size = egui::vec2(width, width * 0.75);
        if let Some(clip) = self.clips.get_mut(film).filter(|clip| !clip.frames.is_empty()) {
            clip.used = Instant::now();
            let at = if moving { (self.began.elapsed().as_secs_f64() * SECOND / EVERY as f64) as usize % clip.frames.len() } else { clip.frames.len() - 1 };
            ui.add(egui::Image::new(&clip.frames[at]).fit_to_exact_size(size));
            if moving {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(1000 * EVERY as u64 / 60));
            }
            return;
        }
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(0x14, 0x14, 0x14));
        let said = match self.failed.get(film) {
            Some(why) => why.clone(),
            None => {
                if self.asked.as_deref() != Some(film) {
                    self.asked = Some(film.to_string());
                    _ = self.asks.send((film.to_string(), !moving, from_line.unwrap_or(0)));
                }
                "the example is playing…".to_string()
            }
        };
        let galley = ui.painter().layout(said, egui::FontId::proportional(crate::theme::SMALL), crate::theme::FAINT, rect.width() - 16.0);
        let at = rect.center() - galley.size() / 2.0;
        ui.painter().with_clip_rect(rect).galley(at, galley, crate::theme::FAINT);
    }
}

/// The little film a line makes, said of someone: Pikachu and Eevee side by side on the beach, framed.
pub fn film_of(line: &str) -> String {
    format!(
        "scene: beach pikachu eevee\npikachu put center facing eevee\neevee put right_of pikachu facing pikachu\nframe pikachu eevee\nwait 0.4s\n{line}\nwait 1s\n"
    )
}

/// The little film a move makes: that one (a species) uses it on a Krabby (or on Pikachu, a Krabby's own).
pub fn film_of_move(user: &str, name: &str) -> String {
    let other = if user == "krabby" { "pikachu" } else { "krabby" };
    format!(
        "scene: beach {user} {other}\n{user} put center facing {other}\n{other} put right_of {user} facing {user}\nframe {user} {other}\nwait 0.4s\n{user} attack {name} at {other}\nwait 1s\n"
    )
}

/// The thread: each film asked played, its frames drawn (its last only, for a still);
/// of the films to be played asked meanwhile, the last only, the stills each in turn.
fn work(session: &crate::session::Session, asked: &Receiver<Asked>, answer: &Sender<Played>, waker: &egui::Context) {
    let data = session.data().to_string();
    let mut art = match preview::art::Art::open_in(&data, session.library().clone()) {
        Ok(art) => Some(art),
        Err(_) => return,
    };
    let tables = session.tables();
    let (mut canvas, mut screen) = (stage_view::ui::Canvas::default(), shell::Screen::new(1));
    let mut stills: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut moving: Option<(String, usize)> = None;
    loop {
        if stills.is_empty() && moving.is_none() {
            match asked.recv() {
                Ok((film, true, _)) => stills.push_back(film),
                Ok((film, false, line)) => moving = Some((film, line)),
                Err(_) => return,
            }
        }
        while let Ok((film, still, line)) = asked.try_recv() {
            if still {
                if !stills.contains(&film) {
                    stills.push_back(film);
                }
            } else {
                moving = Some((film, line));
            }
        }
        // (What plays first: the film to be watched; else the next still.)
        let (film, still, from_line) = match moving.take() {
            Some((film, line)) => (film, false, line),
            None => match stills.pop_front() {
                Some(film) => (film, true, 0),
                None => continue,
            },
        };
        let Some(taken) = art.take() else { return };
        let mut session = preview::session::Session::new(&film, taken, tables.clone());
        let mut play = || {
            let report = session.play().clone();
            if let Some(problem) = report.read.iter().find(|problem| !problem.is_remark()) {
                return Err(problem.message.clone());
            }
            let end = session.end().unwrap_or(0);
            if still {
                // (A moment before its end: the end shows "The End" on black.)
                let shown = end.saturating_sub(20);
                return Ok(session
                    .draw(shown, &mut canvas, &mut screen)
                    .then(|| egui::ColorImage::from_rgba_unmultiplied([screen.wide, screen.high], &screen.rgba))
                    .into_iter()
                    .collect());
            }
            // (From a moment before the line, to the end.)
            let from = report.timeline.as_ref().and_then(|timeline| timeline.lines.get(&(from_line as i32)).copied()).unwrap_or(0).saturating_sub(10);
            let mut frames = Vec::new();
            let mut frame = from;
            while frame <= end && frames.len() < KEPT {
                if session.draw(frame, &mut canvas, &mut screen) {
                    frames.push(egui::ColorImage::from_rgba_unmultiplied([screen.wide, screen.high], &screen.rgba));
                }
                frame += EVERY;
            }
            Ok(frames)
        };
        let played = play();
        art = Some(session.into_art());
        if answer.send((film, played)).is_err() {
            return;
        }
        waker.request_repaint();
    }
}
