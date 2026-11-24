//! The Hierarchy (the layers, shown or hidden, locked; then each line's
//! things, the one in front first) and the Inspector (the brush in hand;
//! else what is chosen: several lined up, kept as a model; one piece,
//! someone who lives there, a landmark, a terrain; else what to add) of the
//! Decor space, and the History; the rows and badges they share.

use eframe::egui::{self, Color32, Sense, vec2};

use super::decors::Decors;
use super::edit::{Item, Kind};
use workshop_base::theme;

mod fields;
mod hierarchy;
mod history;
mod inspector;
mod landmark;
mod layers;
mod piece;
mod resident;
#[cfg(test)]
mod tests;

pub(crate) use fields::typed;
pub use hierarchy::hierarchy;
pub use history::history;
pub use inspector::inspector;

/// A row of the Hierarchy: its rectangle and how it answers.
fn row(ui: &mut egui::Ui, chosen: bool) -> (egui::Rect, egui::Response) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::click());
    let fill = if chosen {
        theme::CHOSEN
    } else if response.hovered() {
        theme::HOVERED
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    (rect, response)
}

/// The badge of what a line is: a letter, a colour, what it says.
fn badge_of(decors: &Decors, one: Option<&Item>, terrain: bool) -> (&'static str, Color32, &'static str) {
    if terrain {
        return ("T", Color32::from_rgb(0x5B, 0xBF, 0x5A), "painted terrain");
    }
    let Some(one) = one else {
        return ("?", theme::FAINT, "");
    };
    match one.kind {
        Kind::Lives => ("R", Color32::from_rgb(0xF6, 0xD3, 0x3C), "resident"),
        Kind::Spot => ("M", theme::DECOR, "a films' spot"),
        Kind::Entrance => ("E", Color32::from_rgb(0x4A, 0xDE, 0x80), "entrance"),
        Kind::Exit => ("X", theme::FILM, "exit"),
        Kind::Zone => ("Z", theme::GAME, "zone (on zone … end)"),
        Kind::Piece => match decors.pieces.iter().find(|piece| piece.name == one.name).map(|piece| piece.kind) {
            Some('a') => ("A", Color32::from_rgb(0x38, 0xC6, 0xD9), "the game's animated tiles"),
            Some('o') => ("S", theme::ASSETS, "scene thing"),
            _ if one.name.starts_with('r') => ("F", theme::WEAK, "remnant of a game place"),
            _ => ("O", Color32::from_rgb(0x8A, 0xA4, 0xFF), "object of a game place"),
        },
    }
}

/// A badge drawn in a square.
fn paint_badge(painter: &egui::Painter, rect: egui::Rect, (letter, colour, _): (&str, Color32, &str)) {
    painter.rect_filled(rect, 3.0, colour.gamma_multiply(0.28));
    painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0_f32, colour), egui::StrokeKind::Inside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, letter, egui::FontId::proportional(theme::SMALL - 1.5), colour);
}

/// A small cross one clicks (taken out, closed).
fn cross_button(ui: &mut egui::Ui, said: &str) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    workshop_base::icons::button(ui, rect, said, workshop_base::icons::cross)
}
