//! The History: the changes made to the decor, gone back to.

use eframe::egui::{self, vec2};

use super::row;
use crate::Decors;
use workshop_base::theme;

/// Each change, the first first: a click goes back (or forth) to it.
pub fn history(ui: &mut egui::Ui, decors: &mut Decors) {
    let Some(open) = &mut decors.open else {
        ui.label(theme::weak("No decor open."));
        return;
    };
    let (done, undone) = open.history();
    let mut go: Option<(bool, usize)> = None;
    egui::ScrollArea::vertical().auto_shrink(false).stick_to_bottom(true).show(ui, |ui| {
        let (rect, response) = row(ui, done.is_empty());
        ui.painter().text(rect.left_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, "Opening", egui::FontId::proportional(theme::BODY), theme::TEXT);
        if response.clicked() && !done.is_empty() {
            go = Some((false, done.len()));
        }
        for (at, said) in done.iter().enumerate() {
            let (rect, response) = row(ui, at + 1 == done.len());
            ui.painter().text(rect.left_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, said, egui::FontId::proportional(theme::BODY), theme::TEXT);
            if response.clicked() && at + 1 < done.len() {
                go = Some((false, done.len() - at - 1));
            }
        }
        for (at, said) in undone.iter().enumerate() {
            let (rect, response) = row(ui, false);
            ui.painter().text(rect.left_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, said, egui::FontId::proportional(theme::BODY), theme::FAINT);
            if response.on_hover_text("undone: a click redoes it (up to it)").clicked() {
                go = Some((true, at + 1));
            }
        }
    });
    if let Some((forth, count)) = go {
        for _ in 0..count {
            if forth {
                open.redo();
            } else {
                open.undo();
            }
        }
    }
}
