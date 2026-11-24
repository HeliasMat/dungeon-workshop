//! The steps of an animation: added, removed, moved, played, and their strip
//! under the canvas.

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use image::RgbaImage;

use super::Painting;
use super::canvas::{checkered, fitted};
use workshop_base::theme;

impl Painting {
    /// A step (of every layer) added after this one: empty, or the same.
    pub(super) fn add_step(&mut self, same: bool) {
        self.keep(if same { "duplicate the image" } else { "new image" });
        let (wide, high) = (self.wide() as u32, self.high() as u32);
        let at = self.at;
        for layer in &mut self.layers {
            let cel = if same { layer.cels[at].clone() } else { RgbaImage::new(wide, high) };
            layer.cels.insert(at + 1, cel);
        }
        self.ticks.insert(at + 1, self.ticks[at]);
        self.at += 1;
        self.all_changed();
    }

    pub(super) fn remove_step(&mut self) {
        if self.count() < 2 {
            return;
        }
        self.keep("remove the image");
        let at = self.at;
        for layer in &mut self.layers {
            layer.cels.remove(at);
        }
        self.ticks.remove(at);
        self.settle();
    }

    fn move_step(&mut self, later: bool) {
        let other = if later { self.at + 1 } else { self.at.wrapping_sub(1) };
        if other >= self.count() {
            return;
        }
        self.keep("move the image");
        let at = self.at;
        for layer in &mut self.layers {
            layer.cels.swap(at, other);
        }
        self.ticks.swap(at, other);
        self.at = other;
        self.all_changed();
    }

    /// The step shown while it plays.
    pub(super) fn playing_step(&self) -> usize {
        let count = self.count();
        let order: Vec<usize> = if self.back_and_forth && count > 2 { (0..count).chain((1..count - 1).rev()).collect() } else { (0..count).collect() };
        let total: u32 = order.iter().map(|at| u32::from(self.ticks[*at]).max(1)).sum();
        let mut now = ((self.began.elapsed().as_secs_f64() * 60.0 * f64::from(self.speed)) as u64 % u64::from(total.max(1))) as u32;
        for at in order {
            let length = u32::from(self.ticks[at]).max(1);
            if now < length {
                return at;
            }
            now -= length;
        }
        0
    }
}

/// The steps under the canvas: each its picture and its time; played.
pub(super) fn steps_panel(ui: &mut egui::Ui, painting: &mut Painting, context: &egui::Context) {
    ui.horizontal_wrapped(|ui| {
        let count = painting.count();
        ui.label(theme::text(format!("IMAGES  {} / {count}", painting.at + 1), theme::SMALL, theme::WEAK));
        if ui.selectable_label(painting.playing, if painting.playing { "⏸ Stop" } else { "▶ Play" }).on_hover_text("on the canvas").clicked() {
            painting.playing = !painting.playing;
            painting.began = Instant::now();
            painting.drop_floating();
        }
        if ui.selectable_label(!painting.back_and_forth, "Loop").clicked() {
            painting.back_and_forth = false;
        }
        if ui.selectable_label(painting.back_and_forth, "Back and Forth").clicked() {
            painting.back_and_forth = true;
        }
        for speed in [0.5, 1.0, 2.0] {
            if ui.selectable_label(painting.speed == speed, format!("×{speed}")).clicked() {
                painting.speed = speed;
            }
        }
        ui.separator();
        if ui.small_button("+ empty").clicked() {
            painting.add_step(false);
        }
        if ui.small_button("Duplicate").clicked() {
            painting.add_step(true);
        }
        if count > 1 && ui.small_button("Remove").clicked() {
            painting.remove_step();
        }
        if ui.small_button("◀").on_hover_text("earlier").clicked() {
            painting.move_step(false);
        }
        if ui.small_button("▶").on_hover_text("later").clicked() {
            painting.move_step(true);
        }
        ui.separator();
        let mut all = painting.ticks[painting.at];
        ui.label(theme::text("All", theme::SMALL, theme::WEAK));
        if ui.add(egui::DragValue::new(&mut all).range(1..=240).suffix("/60")).on_hover_text("the same time for all").changed() {
            painting.keep("timing");
            painting.ticks = vec![all; count];
            painting.saved = false;
        }
    });
    egui::ScrollArea::horizontal().id_salt("image strip").show(ui, |ui| {
        ui.horizontal(|ui| {
            for at in 0..painting.count() {
                ui.vertical(|ui| {
                    let (rect, response) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::click());
                    checkered(ui.painter(), rect, 8.0);
                    let texture = painting.composite_texture(context, at);
                    let inside = fitted(texture.size_vec2(), rect.shrink(2.0));
                    ui.painter().image(texture.id(), inside, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
                    let playing_here = painting.playing && painting.playing_step() == at;
                    if at == painting.at || playing_here {
                        ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0_f32, if playing_here { theme::FILM } else { theme::ASSETS }), StrokeKind::Outside);
                    }
                    ui.painter().text(
                        rect.left_top() + vec2(3.0, 2.0),
                        egui::Align2::LEFT_TOP,
                        format!("{}", at + 1),
                        FontId::proportional(theme::SMALL - 1.0),
                        Color32::WHITE,
                    );
                    if response.clicked() {
                        painting.drop_floating();
                        painting.at = at;
                    }
                    let mut ticks = painting.ticks[at];
                    if ui
                        .add(egui::DragValue::new(&mut ticks).range(1..=240).suffix("/60"))
                        .on_hover_text("how long it stays (sixtieths of a second)")
                        .changed()
                    {
                        painting.ticks[at] = ticks;
                        painting.saved = false;
                    }
                });
            }
        });
    });
}
