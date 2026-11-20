//! The Script laid out: the text beside what it shows, its windows.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, FontId, Key, Modifiers, Rect, RichText, Sense, Stroke, pos2, vec2};

use super::body::script_body;
use super::highlight::{Lights, painted};
use super::plan::plan;
use super::text::spot_of;
use super::wiki::wiki_window;
use super::{GUTTER, PLAIN, Scripts, Seen};
use crate::theme;

/// The Script; `playing`: the line the film plays now. The line whose number was clicked (the film goes there).
pub fn script(ui: &mut egui::Ui, scripts: &mut Scripts, path: Option<&PathBuf>, seen: &Seen) -> Option<usize> {
    let Some(file) = path else {
        return script_body(ui, scripts, path, seen);
    };
    scripts.open(file);
    if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F11)) {
        (scripts.concentration, scripts.centered) = (!scripts.concentration, None);
    }
    // (Concentration: the text alone over the whole window, a column in its middle; F11 or Esc: back.)
    if scripts.concentration {
        let screen = ui.ctx().screen_rect();
        let mut asked = None;
        egui::Area::new(egui::Id::new("concentration")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ui.ctx(), |ui| {
            ui.set_min_size(screen.size());
            ui.painter().rect_filled(screen, 0.0, Color32::from_rgb(0x1E, 0x1E, 0x1E));
            let wide = (screen.width() - 80.0).min(scripts.size * 0.62 * 96.0 + GUTTER + 40.0);
            let column = Rect::from_center_size(screen.center(), vec2(wide, screen.height() - 40.0));
            let corner = ui.painter().text(
                pos2(screen.max.x - 16.0, screen.min.y + 12.0),
                egui::Align2::RIGHT_TOP,
                "focus · F11: back",
                FontId::proportional(theme::SMALL - 1.0),
                theme::FAINT,
            );
            if ui.interact(corner, egui::Id::new("leave the focus"), Sense::click()).clicked() {
                scripts.concentration = false;
            }
            let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(column).layout(egui::Layout::top_down(egui::Align::Min)));
            asked = script_body(&mut inside, scripts, path, seen);
        });
        if ui.input(|input| input.key_pressed(Key::Escape)) && scripts.completion.is_none() && scripts.extra.is_empty() {
            scripts.concentration = false;
        }
        ui.allocate_rect(ui.available_rect_before_wrap(), Sense::hover());
        return asked;
    }
    if !scripts.plan_shown && !scripts.wiki_shown && scripts.beside.is_none() {
        return script_body(ui, scripts, path, seen);
    }
    // (The Layout on the left, the text beside it; the Wiki in a window of its own.)
    let full = ui.available_rect_before_wrap();
    let plan_wide = if scripts.plan_shown { 190.0 } else { 0.0 };
    if scripts.plan_shown {
        let side = Rect::from_min_max(full.min, pos2(full.min.x + plan_wide, full.max.y));
        ui.painter().rect_filled(side, 0.0, theme::PANEL);
        ui.painter().vline(side.max.x, side.y_range(), Stroke::new(1.0_f32, theme::GAP));
        let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(side.shrink(6.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        plan(&mut inside, scripts, &seen.timing, &seen.numbers);
    }
    if scripts.wiki_shown {
        wiki_window(ui.ctx(), scripts);
    }
    let mut middle = Rect::from_min_max(pos2(full.min.x + plan_wide + 1.0, full.min.y), full.max);
    // (Another film beside: read there; a double-click on one of its lines opens it at that line.)
    if let Some(other) = scripts.beside.clone() {
        let half = Rect::from_min_max(pos2(middle.center().x + 1.0, middle.min.y), middle.max);
        middle.max.x = middle.center().x - 1.0;
        ui.painter().rect_filled(half, 0.0, theme::GROUND);
        ui.painter().vline(half.min.x, half.y_range(), Stroke::new(1.0_f32, theme::GAP));
        let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(half.shrink(4.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        if let Some(line) = beside_view(&mut inside, &other, scripts.size) {
            scripts.open_asked = Some((other.clone(), line));
        }
    }
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(middle).layout(egui::Layout::top_down(egui::Align::Min)));
    let asked = script_body(&mut inside, scripts, path, seen);
    ui.allocate_rect(full, Sense::hover());
    asked
}

/// A film read beside the one open (as its words are coloured): a double-click on a line, that line asked.
fn beside_view(ui: &mut egui::Ui, path: &Path, size: f32) -> Option<usize> {
    let text = std::fs::read_to_string(path).unwrap_or_default().replace("\r\n", "\n");
    ui.horizontal(|ui| {
        ui.label(RichText::new(path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned())).strong());
        ui.label(theme::text("beside · a double-click on a line: open it there", theme::SMALL, theme::FAINT));
    });
    let font = FontId::monospace(size);
    let mut asked = None;
    egui::ScrollArea::both().id_salt(("beside", path)).auto_shrink(false).show(ui, |ui| {
        let job = painted(&text, &font, f32::INFINITY, &BTreeMap::new(), &Lights::new(), &BTreeSet::new());
        let galley = ui.fonts(|fonts| fonts.layout_job(job));
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
        ui.painter().galley(rect.min, galley.clone(), PLAIN);
        if response.double_clicked()
            && let Some(at) = response.interact_pointer_pos()
        {
            let index = galley.cursor_from_pos(at - rect.min).index;
            asked = Some(spot_of(&text, index).line);
        }
    });
    asked
}
