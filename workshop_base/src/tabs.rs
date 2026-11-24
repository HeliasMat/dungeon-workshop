//! The tab of one open making, in a space's tools bar (the Dungeon space's
//! dungeons and fights, the Resources' images, sounds, songs): a frame, a
//! dot of its colour, its kind, its word (`•` while not saved) and a `×`
//! that asks "close without saving?" first when it is not saved.

use eframe::egui::{self, Color32, RichText, Sense, vec2};

use crate::theme;

/// What a tab shows.
pub struct Tab<'a> {
    /// Its egui id, one per tab of the bar.
    pub id: egui::Id,
    /// What it is ("Dungeon", "Sound"...), before its word.
    pub kind: &'a str,
    pub colour: Color32,
    pub word: &'a str,
    pub saved: bool,
    /// The one shown.
    pub chosen: bool,
    /// Its `×` clicked once while not saved: it asks before closing.
    pub asking: bool,
    /// The open game's own: "game" shown, this file on hover.
    pub game: Option<String>,
}

/// What was done to a tab this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clicked {
    Nothing,
    /// To show it (a click on it, not on its `×`).
    Show,
    /// To close it (its `×` or a middle click): closed if saved or asked
    /// already, else asked.
    Close,
}

/// Draws one tab, and says what was done to it.
pub fn tab(ui: &mut egui::Ui, tab: Tab) -> Clicked {
    let frame = egui::Frame::new()
        .fill(if tab.chosen { theme::GROUND } else { Color32::TRANSPARENT })
        .stroke(egui::Stroke::new(1.0_f32, if tab.chosen { tab.colour } else { theme::HOVERED }))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(8, 2));
    let shown = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
            ui.painter().rect_filled(dot, 2.0, tab.colour);
            ui.label(theme::text(tab.kind, theme::SMALL - 1.0, theme::WEAK));
            if let Some(path) = &tab.game {
                ui.label(theme::text("game", theme::SMALL - 1.0, theme::GAME)).on_hover_text(path);
            }
            let word = format!("{}{}", tab.word, if tab.saved { "" } else { " •" });
            let name = ui.add(egui::Label::new(RichText::new(word).strong().color(if tab.chosen { theme::STRONG } else { theme::TEXT })).sense(Sense::click()));
            let cross = ui.add(
                egui::Button::new(RichText::new(if tab.asking { "close without saving?" } else { "×" }).size(theme::SMALL).color(if tab.asking {
                    theme::WRONG
                } else {
                    theme::FAINT
                }))
                .frame(false),
            );
            (name, cross)
        })
        .inner
    });
    let (name, cross) = shown.inner;
    let whole = ui.interact(shown.response.rect, tab.id, Sense::click());
    let clicked = if whole.middle_clicked() || cross.clicked() {
        Clicked::Close
    } else if name.clicked() || whole.clicked() {
        Clicked::Show
    } else {
        Clicked::Nothing
    };
    whole.on_hover_text(if tab.saved { "click: show it · middle click: close" } else { "• not saved yet · Ctrl+Tab: the next one" });
    clicked
}
