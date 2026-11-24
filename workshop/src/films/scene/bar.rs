//! Under the film's picture: the line it plays; the bar (play, where, how
//! fast, heard, step by line); the keys over the panel.

use eframe::egui::{self, FontId, Rect, RichText, Sense, Stroke, vec2};

use super::super::playing::clock;
use super::super::{Films, SECOND};
use crate::script::Scripts;
use workshop_base::theme;

/// The line played (a click: it in the Script), what was last said.
pub(super) fn line_played(ui: &egui::Ui, films: &Films, scripts: &mut Scripts, under: Rect) {
    if let (Some(line), Some(open)) = (films.stepped_line().or(films.playing_line()), scripts.open.as_ref()) {
        let words = open.text.lines().nth(line - 1).unwrap_or_default().trim().to_string();
        let written = ui.painter().with_clip_rect(under).text(
            under.left_center(),
            egui::Align2::LEFT_CENTER,
            format!("line {line}  ·  {words}"),
            FontId::monospace(theme::SMALL),
            theme::WEAK,
        );
        if ui.interact(written.intersect(under), ui.id().with("line played"), Sense::click()).on_hover_text("a click: this line in the Script").clicked()
            && let Some(open) = scripts.open.as_mut()
        {
            open.go_to_line(line);
        }
    }
    if let Some(said) = &films.said {
        ui.painter().with_clip_rect(under).text(under.right_center(), egui::Align2::RIGHT_CENTER, said, FontId::proportional(theme::SMALL), theme::FAINT);
    }
}

/// The bar: where (each scene's start marked), its buttons.
pub(super) fn bar(ui: &mut egui::Ui, films: &mut Films, bar: Rect) {
    ui.painter().rect_filled(bar, 0.0, theme::TOOLS);
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(10.0, 6.0))));
    let end = films.end.max(1);
    let mut at = films.frame() as f64;
    inside.spacing_mut().slider_width = inside.available_width();
    let slider = inside.add(egui::Slider::new(&mut at, 0.0..=end as f64).show_value(false).trailing_fill(true));
    if slider.changed() {
        films.go(at as usize);
    }
    // (Where each scene begins, on the bar.)
    if let Some(timeline) = &films.timeline {
        let track = slider.rect.shrink2(vec2(8.0, 0.0));
        for scene in timeline.scenes.iter().skip(1) {
            let x = track.min.x + scene.start as f32 / end as f32 * track.width();
            inside.painter().vline(x, slider.rect.y_range(), Stroke::new(2.0_f32, theme::FILM));
        }
    }
    inside.horizontal(|ui| buttons(ui, films));
}

/// Play, a frame back or on, the next scene, step by line, the time, how fast, heard.
fn buttons(ui: &mut egui::Ui, films: &mut Films) {
    if ui.button("⏮").on_hover_text("to the start (Home)").clicked() {
        films.go(0);
    }
    if ui.small_button("‹").on_hover_text("one frame back (Shift+←)").clicked() {
        films.go(films.frame().saturating_sub(1));
    }
    let word = if films.playing { "⏸" } else { "▶" };
    if ui
        .add(egui::Button::new(RichText::new(word).color(theme::GROUND)).fill(theme::FILM).min_size(vec2(34.0, 0.0)))
        .on_hover_text("play, stop (Space)")
        .clicked()
    {
        toggle(films);
    }
    if ui.small_button("›").on_hover_text("one frame on (Shift+→)").clicked() {
        films.go(films.frame() + 1);
    }
    if ui.button("⏭").on_hover_text("to the next scene").clicked() {
        let frame = films.frame();
        let next = films.timeline.as_ref().and_then(|timeline| timeline.scenes.iter().map(|scene| scene.start).find(|start| *start > frame + 1));
        films.go(next.unwrap_or(films.end));
    }
    ui.separator();
    ui.toggle_value(&mut films.step, "Step").on_hover_text("step by line: playing, the film stops at the start of each line (F6)");
    if ui.button("Next Line").on_hover_text("played until the next line starts, stopped there (F10)").clicked() {
        films.next_line();
    }
    ui.separator();
    ui.label(RichText::new(format!("{} / {}", clock(films.frame() as f64), clock(films.end as f64))).monospace().color(theme::TEXT));
    ui.separator();
    for speed in [0.25, 0.5, 1.0, 2.0] {
        if ui.selectable_label(films.speed == speed, format!("{speed}×")).clicked() {
            films.speed = speed;
        }
    }
    ui.separator();
    let mut heard = films.heard;
    if ui.checkbox(&mut heard, "Sound").changed() {
        films.hear(heard);
    }
}

/// The keys: F6 and F10 wherever the pointer is; over the panel, Space
/// plays, ← → a second (Shift: a frame), `,` `.` a frame, Home.
pub(super) fn keys(ui: &egui::Ui, films: &mut Films, full: Rect) {
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F6)) {
        films.step = !films.step;
    }
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F10)) {
        films.next_line();
    }
    let pointed = ui.rect_contains_pointer(full);
    if !pointed || ui.ctx().wants_keyboard_input() {
        return;
    }
    let (space, left, right, home, shift) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::Space) || input.key_pressed(egui::Key::K),
            input.key_pressed(egui::Key::ArrowLeft) || input.key_pressed(egui::Key::J),
            input.key_pressed(egui::Key::ArrowRight) || input.key_pressed(egui::Key::L),
            input.key_pressed(egui::Key::Home),
            input.modifiers.shift,
        )
    });
    let (comma, period) = ui.input(|input| (input.key_pressed(egui::Key::Comma), input.key_pressed(egui::Key::Period)));
    let step = if shift { 1 } else { SECOND as usize };
    if space {
        toggle(films);
    }
    if left {
        films.go(films.frame().saturating_sub(step));
    }
    if right {
        films.go(films.frame() + step);
    }
    if comma {
        films.go(films.frame().saturating_sub(1));
    }
    if period {
        films.go(films.frame() + 1);
    }
    if home {
        films.go(0);
    }
}

pub(in super::super) fn toggle(films: &mut Films) {
    if !films.playing && films.frame() >= films.end {
        films.go(0);
    }
    films.playing = !films.playing;
}
