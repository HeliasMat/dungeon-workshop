//! A line written at a time of the Timeline: a double click on a track opens
//! a small window there, with what that track can do (a gesture, a line
//! said, a walk for one of the cast; a glide or a zoom for the camera; a
//! fade for the screen; a wait), or any line typed; beside the film or in
//! sequence. Chosen, it is written where it starts then (`edits.rs`).

use eframe::egui::{self, Pos2};

use super::Films;
use super::lanes::Lane;
use super::playing::clock;
use workshop_base::theme;

/// The gestures offered for one of the cast (after their name).
pub(super) const GESTURES: [&str; 9] = ["nod", "hop happy", "jump", "laugh", "sigh", "look around", "bow", "sulk", "sit"];

/// The small window of a line to write: on which track, at which frame,
/// where on the screen; what is typed in it; beside the film or not; just opened.
pub(crate) struct Inserting {
    lane: Lane,
    frame: usize,
    at: Pos2,
    said: String,
    line: String,
    beside: bool,
    fresh: bool,
    /// Asked for a picture: put where its time is once the Timeline is laid out.
    unplaced: bool,
}

impl Inserting {
    pub(super) fn new(lane: Lane, frame: usize, at: Pos2) -> Inserting {
        Inserting { lane, frame, at, said: String::new(), line: String::new(), beside: false, fresh: true, unplaced: false }
    }

    /// To be put where its time is on the Timeline (`placed`).
    pub(super) fn unplaced(self) -> Inserting {
        Inserting { unplaced: true, ..self }
    }

    /// Put where its time is, if it was not.
    pub(super) fn placed(&mut self, at: impl Fn(usize) -> Pos2) {
        if std::mem::take(&mut self.unplaced) {
            self.at = at(self.frame);
        }
    }
}

/// The quick lines a track offers: what each button says, the line it writes.
pub(super) fn offered(lane: &Lane, feet: Option<[f32; 2]>, camera: Option<[f32; 2]>) -> Vec<(String, String)> {
    let mut offered: Vec<(String, String)> = Vec::new();
    match lane {
        Lane::Who(who) => {
            offered.extend(GESTURES.iter().map(|gesture| (gesture.to_string(), format!("{who} {gesture}"))));
            if let Some([x, y]) = feet {
                offered.push(("walk".to_string(), format!("{who} walk to {} {}", (x + 24.0).round(), y.round())));
            }
        }
        Lane::Camera => {
            if let Some([x, y]) = camera {
                offered.push(("glide here".to_string(), format!("glide to {} {} in 1s", x.round(), y.round())));
            }
            offered.push(("zoom in".to_string(), "zoom 1.5 1s".to_string()));
            offered.push(("zoom back".to_string(), "zoom 1 1s".to_string()));
            offered.push(("shake".to_string(), "shake 0.5s".to_string()));
        }
        Lane::Sound => {}
        Lane::Screen => {
            offered.push(("fade out".to_string(), "transition fade out 1s".to_string()));
            offered.push(("fade in".to_string(), "transition fade in 1s".to_string()));
        }
        Lane::Other => {}
    }
    offered.push(("wait 1s".to_string(), "wait 1s".to_string()));
    offered
}

/// The window, if open: the line chosen in it (its frame, its words, beside the film or not).
pub(super) fn adding(context: &egui::Context, films: &mut Films) -> Option<(usize, String, bool)> {
    let feet = films.inserting.as_ref().and_then(|open| open.lane.who()).and_then(|who| films.drawn.as_ref()?.feet.get(who).copied());
    let camera = films.drawn.as_ref().map(|drawn| [drawn.state.camera.centre.x, drawn.state.camera.centre.y]);
    let open = films.inserting.as_mut()?;
    let mut chosen: Option<String> = None;
    let mut closed = context.input(|input| input.key_pressed(egui::Key::Escape));
    let shown = egui::Area::new(egui::Id::new("timeline: a line written there")).fixed_pos(open.at + egui::vec2(4.0, 4.0)).order(egui::Order::Foreground).show(
        context,
        |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(300.0);
                ui.label(theme::text(format!("{} · at {}", open.lane.name(), clock(open.frame as f64)), theme::BODY, theme::STRONG));
                ui.horizontal_wrapped(|ui| {
                    for (said, line) in offered(&open.lane, feet, camera) {
                        if ui.small_button(said).on_hover_text(&line).clicked() {
                            chosen = Some(line);
                        }
                    }
                });
                if let Some(who) = open.lane.who() {
                    ui.horizontal(|ui| {
                        ui.label(theme::weak("Say"));
                        let typed = ui.add(egui::TextEdit::singleline(&mut open.said).hint_text("what they say, Enter").desired_width(220.0));
                        if open.fresh {
                            typed.request_focus();
                        }
                        if typed.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) && !open.said.trim().is_empty() {
                            chosen = Some(format!("{who}: {}", open.said.trim()));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label(theme::weak("Line"));
                    let hint = match &open.lane {
                        Lane::Who(who) => format!("{who} …, Enter"),
                        Lane::Sound => "sound bell, music calm…, Enter".to_string(),
                        _ => "any line, Enter".to_string(),
                    };
                    let typed = ui.add(egui::TextEdit::singleline(&mut open.line).hint_text(hint).desired_width(220.0));
                    if open.fresh && open.lane.who().is_none() {
                        typed.request_focus();
                    }
                    if typed.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) && !open.line.trim().is_empty() {
                        chosen = Some(open.line.trim().to_string());
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut open.beside, "Beside the film (&)").on_hover_text("nothing after it moves: it plays alongside");
                    if ui.small_button("Cancel").clicked() {
                        closed = true;
                    }
                });
            });
        },
    );
    let pressed_out = context.input(|input| input.pointer.any_pressed() && input.pointer.interact_pos().is_some_and(|at| !shown.response.rect.contains(at)));
    if pressed_out && !open.fresh {
        closed = true;
    }
    open.fresh = false;
    let made = chosen.map(|words| (open.frame, words, open.beside));
    if made.is_some() || closed {
        films.inserting = None;
    }
    made
}
