//! The Timeline panel: the film's scenes, each line when it plays, who is on
//! the stage when. Here the head, the time's axis (the wheel, the ruler, the
//! whole film small under it, the keys) and what the bars ask of the Script;
//! the bands under the ruler in `bands.rs`.

use eframe::egui::{self, Color32, FontId, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use preview::played::Timeline;

use super::bands::{Asked, bands};
use super::lines::{edited_lines, with_seconds};
use super::playing::clock;
use super::scene::toggle;
use super::{BarAct, Films, SECOND};
use crate::script::Scripts;
use workshop_base::theme;

/// What a line is, to colour its bar: its kind's index (`KINDS`).
pub(super) fn kind_of_line(line: &str) -> usize {
    let line = line.trim();
    let first: String = line.chars().take_while(|letter| letter.is_alphanumeric() || *letter == '_').collect();
    let rest = line[first.len()..].trim_start();
    if rest.starts_with(':') || (rest.starts_with('(') && rest.contains("):")) {
        return 0;
    }
    match first.as_str() {
        "wait" | "pause" | "pace" => 1,
        "music" | "sound" | "volume" | "ambient" | "fade" | "crossfade" => 2,
        "camera" | "frame" | "shot" | "zoom" | "glide" | "view" | "shake" | "quake" => 3,
        _ => 4,
    }
}

/// The kinds of lines, their names and colours.
pub(super) const KINDS: [(&str, Color32); 5] = [
    ("lines", Color32::from_rgb(0xE0, 0xA9, 0x3A)),
    ("waits", Color32::from_rgb(0x70, 0x76, 0x80)),
    ("sound", Color32::from_rgb(0x4A, 0xDE, 0x80)),
    ("camera", Color32::from_rgb(0xC0, 0x84, 0xFC)),
    ("actions", Color32::from_rgb(0x3B, 0x82, 0xC4)),
];

/// Who says a line, if it is one said.
pub(super) fn speaker_of(line: &str) -> Option<String> {
    let line = line.trim();
    let first: String = line.chars().take_while(|letter| letter.is_alphanumeric() || *letter == '_').collect();
    let rest = line[first.len()..].trim_start();
    (!first.is_empty() && (rest.starts_with(':') || (rest.starts_with('(') && rest.contains("):")))).then_some(first)
}

/// The time across the Timeline: where it begins on the screen, the frame
/// at its left edge, how wide a frame is, the film's last frame.
#[derive(Clone, Copy)]
pub(super) struct Axis {
    pub(super) content: Rect,
    pub(super) left: f32,
    pub(super) per_frame: f32,
    pub(super) end: usize,
}

impl Axis {
    /// Where a frame is on the screen.
    pub(super) fn x_of(self, frame: f32) -> f32 {
        self.content.min.x + (frame - self.left) * self.per_frame
    }

    /// The frame at a place of the screen (within the film).
    pub(super) fn frame_of(self, x: f32) -> usize {
        ((x - self.content.min.x) / self.per_frame + self.left).clamp(0.0, self.end as f32) as usize
    }
}

/// The ruler's step, in seconds: one that reads at that zoom (64 points at least).
pub(super) fn second_step(zoom: f32) -> f32 {
    [0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0].into_iter().find(|seconds: &f32| seconds * zoom >= 64.0).unwrap_or(120.0)
}

/// The zoom (points a second) at which so many frames fill that width.
pub(super) fn zoom_to_fill(width: f32, frames: usize) -> f32 {
    ((width - 20.0) / (frames.max(1) as f32 / SECOND as f32)).clamp(4.0, 400.0)
}

/// The next start of a line of the film's own (not beside it) after that frame, or the one before it.
pub(super) fn next_start(timeline: &Timeline, now: usize, forth: bool) -> Option<usize> {
    let mut starts: Vec<usize> = timeline.spans.iter().filter(|span| span.label.is_none() && span.thread == 0).map(|span| span.start).collect();
    starts.sort_unstable();
    starts.dedup();
    if forth { starts.iter().find(|start| **start > now).copied() } else { starts.iter().rev().find(|start| **start < now).copied() }
}

/// The Timeline: its head, the time's ruler, its bands, the whole film small; what they asked, done.
pub fn timeline(ui: &mut egui::Ui, films: &mut Films, scripts: &mut Scripts) {
    let Some(timeline) = films.timeline.clone() else {
        (scripts.lit_line, scripts.lit_who_track) = (None, None);
        ui.add_space(8.0);
        ui.label(theme::weak(if scripts.open.is_some() { "The film is playing…" } else { "No film open." }));
        return;
    };
    let end = films.end.max(1);
    let fit = head(ui, films, &timeline);
    let full = ui.available_rect_before_wrap();
    let names_wide = 84.0;
    let content = Rect::from_min_max(pos2(full.min.x + names_wide, full.min.y), full.max);
    if fit {
        films.zoom = zoom_to_fill(content.width(), end);
        films.left = 0.0;
    }
    let visible = content.width() / (films.zoom / SECOND as f32);
    let pointer = ui.input(|input| input.pointer.hover_pos()).filter(|at| full.contains(*at));
    wheel(ui, films, content, end, pointer);
    let axis = Axis { content, left: films.left, per_frame: films.zoom / SECOND as f32, end };
    let painter = ui.painter_at(full);
    painter.rect_filled(full, 0.0, Color32::from_rgb(0x1B, 0x1B, 0x1B));
    let ruler = Rect::from_min_max(full.min, pos2(full.max.x, full.min.y + 22.0));
    time_ruler(ui, &painter, films, &timeline, axis, full, ruler);
    let overview = Rect::from_min_max(pos2(content.min.x, full.max.y - 12.0), full.max);
    let body = Rect::from_min_max(pos2(full.min.x, ruler.max.y), pos2(full.max.x, overview.min.y));
    let asked = bands(ui, films, scripts, &timeline, axis, body);
    scripts.lit_line = asked.lit_line;
    scripts.lit_who_track = asked.lit_who_track.clone();
    if let Some((start, until)) = asked.fit_scene {
        films.zoom = zoom_to_fill(content.width(), until - start);
        films.left = start as f32;
    }
    bars_asked(films, scripts, &asked);
    whole_film(ui, films, &timeline, overview, end, visible);
    playhead_on_ruler(ui, films, axis, ruler, pointer);
    if let Some(frame) = asked.go {
        films.go(frame);
        films.playing = false;
    }
    if let (Some(line), Some(open)) = (asked.line_chosen, scripts.open.as_mut()) {
        open.go_to_line(line);
        films.followed_line = line;
    }
    if pointer.is_some() && !ui.ctx().wants_keyboard_input() {
        keys(ui, films, &timeline);
    }
    ui.allocate_rect(full, Sense::hover());
}

/// The head: what it holds, the kinds' colours (a click hides them), who it
/// is focused on, the zoom, See All, the playhead followed, the loop. Whether all is to be seen.
fn head(ui: &mut egui::Ui, films: &mut Films, timeline: &Timeline) -> bool {
    let mut fit = false;
    ui.horizontal(|ui| {
        ui.label(theme::text("Timeline", theme::BODY, theme::STRONG));
        ui.label(theme::weak(format!("{} scene(s) · {} line(s) · {}", timeline.scenes.len(), timeline.lines.len(), clock(films.end as f64))));
        ui.separator();
        for (index, (name, colour)) in KINDS.iter().enumerate() {
            let hidden = films.hidden_kinds[index];
            let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
            if hidden {
                ui.painter().rect_stroke(dot, 2.0, Stroke::new(1.0_f32, *colour), StrokeKind::Inside);
            } else {
                ui.painter().rect_filled(dot, 2.0, *colour);
            }
            let said = ui.add(egui::Label::new(theme::text(*name, theme::SMALL, if hidden { theme::FAINT } else { theme::WEAK })).sense(Sense::click()));
            if said.on_hover_text("a click: these lines hidden or shown").clicked() {
                films.hidden_kinds[index] = !hidden;
            }
        }
        if let Some(who) = films.focus_who.clone() {
            ui.separator();
            if ui.small_button(format!("✕ {who}")).on_hover_text("all the lines, not only theirs").clicked() {
                films.focus_who = None;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Slider::new(&mut films.zoom, 4.0..=400.0).logarithmic(true).suffix(" px/s").show_value(true));
            fit = ui.button("See All").on_hover_text("the whole film in the width").clicked();
            ui.checkbox(&mut films.follow_head, "Follow the Playhead");
            if let Some((a, b)) = films.loop_range {
                if ui.small_button("✕ loop").on_hover_text("stop looping this passage").clicked() {
                    films.loop_range = None;
                }
                ui.label(theme::text(format!("loop {} → {}", clock(a as f64), clock(b as f64)), theme::SMALL, theme::FILM));
            }
        });
    });
    fit
}

/// The playhead kept in sight while it plays; the wheel: across (Maj or not), closer or further with Ctrl, where the pointer is.
fn wheel(ui: &egui::Ui, films: &mut Films, content: Rect, end: usize, pointer: Option<egui::Pos2>) {
    let per_frame = films.zoom / SECOND as f32;
    let visible = content.width() / per_frame;
    if films.follow_head && films.playing {
        let frame = films.frame() as f32;
        if frame < films.left || frame > films.left + visible * 0.85 {
            films.left = (frame - visible * 0.15).max(0.0);
        }
    }
    let Some(at) = pointer else { return };
    let (command, wheel, wheel_x) = ui.input(|input| (input.modifiers.command, input.smooth_scroll_delta.y, input.smooth_scroll_delta.x));
    if command && wheel != 0.0 {
        let under = films.left + (at.x - content.min.x).max(0.0) / per_frame;
        films.zoom = (films.zoom * (wheel * 0.003).exp()).clamp(4.0, 400.0);
        let per = films.zoom / SECOND as f32;
        films.left = (under - (at.x - content.min.x).max(0.0) / per).max(0.0);
    } else if wheel_x != 0.0 || (ui.input(|input| input.modifiers.shift) && wheel != 0.0) {
        let by = if wheel_x != 0.0 { wheel_x } else { wheel };
        films.left = (films.left - by / per_frame).clamp(0.0, (end as f32 - visible * 0.5).max(0.0));
    }
}

/// The ruler: seconds, what is wrong, the passage played round; a click or
/// a drag goes there (Maj: a passage looped); its menu.
#[allow(clippy::too_many_arguments)]
fn time_ruler(ui: &egui::Ui, painter: &egui::Painter, films: &mut Films, timeline: &Timeline, axis: Axis, full: Rect, ruler: Rect) {
    let content = axis.content;
    painter.rect_filled(ruler, 0.0, theme::TABS);
    let step = second_step(films.zoom);
    let mut second = ((axis.left / SECOND as f32) / step).floor() * step;
    while axis.x_of(second * SECOND as f32) < full.max.x {
        let x = axis.x_of(second * SECOND as f32);
        if x >= content.min.x {
            painter.vline(x, ruler.max.y - 7.0..=ruler.max.y, Stroke::new(1.0_f32, theme::FAINT));
            painter.text(
                pos2(x + 3.0, ruler.min.y + 3.0),
                egui::Align2::LEFT_TOP,
                clock(f64::from(second) * SECOND),
                FontId::proportional(theme::SMALL - 1.5),
                theme::WEAK,
            );
            painter.vline(x, ruler.max.y..=full.max.y, Stroke::new(1.0_f32, Color32::from_white_alpha(7)));
        }
        second += step;
    }
    for (line, _) in &films.wrong {
        if let Some(frame) = timeline.lines.get(line) {
            let x = axis.x_of(*frame as f32);
            painter.add(egui::Shape::convex_polygon(
                vec![pos2(x - 4.0, ruler.max.y), pos2(x + 4.0, ruler.max.y), pos2(x, ruler.max.y - 7.0)],
                Color32::from_rgb(0xF1, 0x4C, 0x4C),
                Stroke::NONE,
            ));
        }
    }
    if let Some((a, b)) = films.loop_range {
        let band = Rect::from_min_max(pos2(axis.x_of(a as f32), ruler.min.y), pos2(axis.x_of(b as f32), full.max.y));
        painter.rect_filled(band.intersect(content), 0.0, theme::FILM.gamma_multiply(0.08));
        painter.rect_filled(Rect::from_min_max(pos2(band.min.x, ruler.max.y - 4.0), pos2(band.max.x, ruler.max.y)).intersect(content), 0.0, theme::FILM);
    }
    let on_ruler = ui.interact(Rect::from_min_max(pos2(content.min.x, ruler.min.y), ruler.max), ui.id().with("time ruler"), Sense::click_and_drag());
    let shift = ui.input(|input| input.modifiers.shift);
    if on_ruler.drag_started() && shift {
        films.loop_start = on_ruler.interact_pointer_pos().map(|at| axis.frame_of(at.x));
    }
    if let (Some(start), Some(at)) = (films.loop_start, on_ruler.interact_pointer_pos())
        && on_ruler.dragged()
    {
        let now = axis.frame_of(at.x);
        films.loop_range = Some((start.min(now), start.max(now).max(start.min(now) + 1)));
        films.looped = true;
    } else if (on_ruler.clicked() || on_ruler.dragged())
        && films.loop_start.is_none()
        && let Some(at) = on_ruler.interact_pointer_pos()
    {
        films.go(axis.frame_of(at.x));
    }
    if on_ruler.drag_stopped() {
        films.loop_start = None;
    }
    on_ruler.context_menu(|ui| {
        if ui.button("Stop Looping").clicked() {
            films.loop_range = None;
            ui.close();
        }
    });
}

/// What the bars asked of the Script: a line made longer or shorter, gone to,
/// a wait after it, doubled, deleted; a line looped.
fn bars_asked(films: &mut Films, scripts: &mut Scripts, asked: &Asked) {
    let Some(open) = scripts.open.as_mut() else { return };
    if let Some((line, seconds)) = asked.stretched
        && let Some(words) = open.text.split('\n').nth(line - 1)
    {
        let made = with_seconds(words, seconds);
        let text = edited_lines(&open.text, |lines| lines[line - 1] = made.clone());
        open.set_text(text);
        films.said = Some(format!("line {line}: {}", made.trim()));
    }
    match asked.bar_act {
        Some(BarAct::Script(line)) => open.go_to_line(line),
        Some(BarAct::Wait(line)) => {
            let text = edited_lines(&open.text, |lines| {
                let lead: String = lines[line - 1].chars().take_while(|letter| letter.is_whitespace()).collect();
                lines.insert(line, format!("{lead}wait 1s"));
            });
            open.set_text(text);
        }
        Some(BarAct::Twice(line)) => {
            let text = edited_lines(&open.text, |lines| lines.insert(line, lines[line - 1].clone()));
            open.set_text(text);
        }
        Some(BarAct::Gone(line)) => {
            let text = edited_lines(&open.text, |lines| {
                lines.remove(line - 1);
            });
            open.set_text(text);
        }
        Some(BarAct::Loop(start, stop)) => {
            films.loop_range = Some((start, stop.max(start + 1)));
            films.looped = true;
            films.go(start);
        }
        None => {}
    }
}

/// The whole film, small, under it all: its scenes, what is seen of it, the playhead; a click or a drag goes there.
fn whole_film(ui: &egui::Ui, films: &mut Films, timeline: &Timeline, overview: Rect, end: usize, visible: f32) {
    let small = ui.painter_at(overview);
    small.rect_filled(overview, 0.0, theme::TABS);
    let whole_x = |frame: f32| overview.min.x + frame / end as f32 * overview.width();
    for (at, scene) in timeline.scenes.iter().enumerate() {
        let until = timeline.scenes.get(at + 1).map_or(films.end, |next| next.start);
        small.rect_filled(
            Rect::from_min_max(pos2(whole_x(scene.start as f32), overview.min.y + 3.0), pos2(whole_x(until as f32) - 1.0, overview.max.y - 3.0)),
            1.0,
            if at % 2 == 0 { Color32::from_rgb(0x3A, 0x33, 0x22) } else { Color32::from_rgb(0x2E, 0x2A, 0x1F) },
        );
    }
    let seen =
        Rect::from_min_max(pos2(whole_x(films.left), overview.min.y + 1.0), pos2(whole_x(films.left + visible).min(overview.max.x), overview.max.y - 1.0));
    small.rect_stroke(seen, 2.0, Stroke::new(1.0_f32, Color32::from_white_alpha(140)), StrokeKind::Inside);
    small.vline(whole_x(films.frame() as f32), overview.y_range(), Stroke::new(1.5_f32, Color32::from_rgb(0xF1, 0x4C, 0x4C)));
    let on_overview = ui.interact(overview, ui.id().with("the whole film"), Sense::click_and_drag());
    if (on_overview.clicked() || on_overview.dragged())
        && let Some(at) = on_overview.interact_pointer_pos()
    {
        let frame = (at.x - overview.min.x) / overview.width() * end as f32;
        films.left = (frame - visible / 2.0).clamp(0.0, (end as f32 - visible * 0.5).max(0.0));
        films.follow_head = false;
    }
    on_overview.on_hover_text("the whole film: the white frame, what is seen of it; a click or a drag goes there");
}

/// The playhead's head on the ruler; the time where the pointer is on it.
fn playhead_on_ruler(ui: &egui::Ui, films: &Films, axis: Axis, ruler: Rect, pointer: Option<egui::Pos2>) {
    let content = axis.content;
    let x = axis.x_of(films.frame() as f32);
    if x >= content.min.x && x <= content.max.x {
        let red = Color32::from_rgb(0xF1, 0x4C, 0x4C);
        ui.painter().vline(x, ruler.y_range(), Stroke::new(1.5_f32, red));
        ui.painter().add(egui::Shape::convex_polygon(
            vec![pos2(x - 6.0, ruler.min.y), pos2(x + 6.0, ruler.min.y), pos2(x, ruler.min.y + 8.0)],
            red,
            Stroke::NONE,
        ));
    }
    if let Some(at) = pointer.filter(|at| at.x >= content.min.x && ruler.contains(*at)) {
        crate::decors::tag(ui.painter(), pos2(at.x + 4.0, ruler.max.y + 18.0), &clock(axis.frame_of(at.x) as f64), Color32::WHITE);
    }
}

/// The keys, the pointer over it: Space plays, ← → the line before, after, Home, End.
fn keys(ui: &egui::Ui, films: &mut Films, timeline: &Timeline) {
    let (space, home, end_key) = ui.input(|input| (input.key_pressed(egui::Key::Space), input.key_pressed(egui::Key::Home), input.key_pressed(egui::Key::End)));
    let (back, forth) = ui.input(|input| (input.key_pressed(egui::Key::ArrowLeft), input.key_pressed(egui::Key::ArrowRight)));
    if space {
        toggle(films);
    }
    if (back || forth)
        && let Some(frame) = next_start(timeline, films.frame(), forth)
    {
        films.go(frame);
        films.playing = false;
    }
    if home {
        films.left = 0.0;
        films.go(0);
    }
    if end_key {
        films.go(films.end);
    }
}
