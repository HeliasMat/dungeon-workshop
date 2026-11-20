//! The tabs of what is being made, and the studio it is made in.

use eframe::egui::{self, RichText, Sense, vec2};

use super::dropped::take_dropped_while_making;
use super::{Resources, Shelf};
use crate::theme;

/// The tabs of what is being made, in the window's tools bar: back to the Library, each making
/// (a click shows it, × closes it), a new one; Ctrl+Tab the next, Ctrl+Shift+Tab the one before.
pub fn making_tabs(ui: &mut egui::Ui, resources: &mut Resources) {
    let (next, before) = ui.input_mut(|input| {
        (input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Tab), input.consume_key(egui::Modifiers::CTRL, egui::Key::Tab))
    });
    let count = resources.makings.len();
    if count > 0 && (next || before) {
        resources.making_at = if before { (resources.making_at + 1) % count } else { (resources.making_at + count - 1) % count };
        resources.closing = None;
    }
    if ui.button(RichText::new("◀ Library").strong()).on_hover_text("the resources (what is being made stays open: ▶ Open)").clicked() {
        resources.making_full = false;
        resources.scroll_wanted = true;
    }
    ui.separator();
    let mut closed = None;
    for at in 0..resources.makings.len() {
        let chosen = at == resources.making_at;
        let maker = &resources.makings[at];
        let colour = match maker.kind() {
            "Sound" => theme::FILM,
            "Music" => theme::HOME,
            "Animation" => theme::DECOR,
            _ => theme::ASSETS,
        };
        let frame = egui::Frame::new()
            .fill(if chosen { theme::GROUND } else { egui::Color32::TRANSPARENT })
            .stroke(egui::Stroke::new(1.0_f32, if chosen { colour } else { theme::HOVERED }))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(8, 2));
        let shown = frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                ui.painter().rect_filled(dot, 2.0, colour);
                ui.label(theme::text(maker.kind(), theme::SMALL - 1.0, theme::WEAK));
                let word = format!("{}{}", maker.word(), if maker.saved() { "" } else { " •" });
                let name = ui.add(egui::Label::new(RichText::new(word).strong().color(if chosen { theme::STRONG } else { theme::TEXT })).sense(Sense::click()));
                let asking = resources.closing == Some(at);
                let cross = ui.add(
                    egui::Button::new(RichText::new(if asking { "close without saving?" } else { "×" }).size(theme::SMALL).color(if asking {
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
        let whole = ui.interact(shown.response.rect, egui::Id::new(("making tab", at)), Sense::click());
        if (name.clicked() || whole.clicked()) && !cross.clicked() {
            resources.making_at = at;
            resources.closing = None;
        }
        if whole.middle_clicked() || cross.clicked() {
            if resources.makings[at].saved() || resources.closing == Some(at) {
                closed = Some(at);
            } else {
                resources.closing = Some(at);
            }
        }
        whole.on_hover_text(if resources.makings[at].saved() {
            "click: show it · middle click: close"
        } else {
            "• not saved yet · Ctrl+Tab: the next one"
        });
    }
    if let Some(at) = closed {
        resources.close_making(at);
    }
    ui.menu_button(RichText::new("+ New").strong().color(theme::ASSETS), |ui| {
        for (said, kind) in [("An Image", "image"), ("An Animation", "animation"), ("A Sound", "sound"), ("A Song", "music")] {
            if ui.button(said).clicked() {
                resources.open_making(crate::making::Maker::new_of(&resources.session, kind));
                ui.close();
            }
        }
        let songs = crate::music::songs(&resources.session);
        if !songs.is_empty() {
            ui.separator();
            ui.label(theme::text("Open a Song", theme::SMALL, theme::WEAK));
            for word in songs {
                if ui.button(&word).clicked() {
                    resources.open_making(crate::making::Maker::Music(Box::new(crate::music::MusicMaking::open(&resources.session, &word))));
                    ui.close();
                }
            }
        }
    });
}

/// What is being made, on the whole window; written, read again and chosen.
pub fn studio(ui: &mut egui::Ui, resources: &mut Resources) {
    take_dropped_while_making(resources, ui.ctx());
    let at = resources.making_at;
    let Some(maker) = resources.makings.get_mut(at) else {
        return;
    };
    let done = match maker {
        crate::making::Maker::Picture(painting) => crate::making::paint(ui, painting),
        crate::making::Maker::Sound(making) => crate::making::make_sound(ui, making, &mut resources.audio, resources.volume),
        crate::making::Maker::Music(music) => crate::music::make_music(ui, music, &mut resources.audio, resources.volume),
        crate::making::Maker::Dungeon(dungeon) => crate::dungeon::make_dungeon(ui, dungeon, &mut resources.audio, resources.volume),
    };
    match done {
        Some(crate::making::Done::Picture(word)) => {
            resources.lists.remove(&Shelf::Drawn);
            resources.pictures.remove(&format!("{}:{word}", Shelf::Drawn.key()));
            (resources.shelf, resources.chosen) = (Shelf::Drawn, Some(format!("{}:{word}", Shelf::Drawn.key())));
        }
        Some(crate::making::Done::Sound(word)) => {
            resources.lists.remove(&Shelf::Sounds);
            (resources.shelf, resources.chosen) = (Shelf::Sounds, Some(format!("{}:{word}", Shelf::Sounds.key())));
        }
        Some(crate::making::Done::Fight(word)) => {
            let fight = if word.is_empty() {
                crate::dungeon::DungeonMaking::new_fight(&resources.session, "new_fight")
            } else {
                crate::dungeon::DungeonMaking::open_fight(&resources.session, &word)
            };
            resources.open_making(crate::making::Maker::Dungeon(Box::new(fight)));
        }
        Some(crate::making::Done::Closed) => {
            if resources.makings[at].saved() || resources.closing == Some(at) {
                resources.close_making(at);
            } else {
                resources.closing = Some(at);
                resources.said = Some("not saved: Close again (or × on its tab) to close it anyway".to_string());
            }
        }
        None => {}
    }
}
