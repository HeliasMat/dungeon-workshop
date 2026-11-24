//! The tabs of what is being made, and the studio it is made in.

use eframe::egui::{self, RichText};

use super::dropped::take_dropped_while_making;
use super::{Resources, Shelf};
use workshop_base::{tabs, theme};

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
        let shown = tabs::Tab {
            id: egui::Id::new(("making tab", at)),
            kind: maker.kind(),
            colour,
            word: maker.word(),
            saved: maker.saved(),
            chosen,
            asking: resources.closing == Some(at),
            game: None,
        };
        match tabs::tab(ui, shown) {
            tabs::Clicked::Show => {
                resources.making_at = at;
                resources.closing = None;
            }
            tabs::Clicked::Close if resources.makings[at].saved() || resources.closing == Some(at) => closed = Some(at),
            tabs::Clicked::Close => resources.closing = Some(at),
            tabs::Clicked::Nothing => {}
        }
    }
    if let Some(at) = closed {
        resources.close_making(at);
    }
    ui.menu_button(RichText::new("+ New").strong().color(theme::ASSETS), |ui| {
        for (said, kind) in [("An Image", "image"), ("An Animation", "animation"), ("A Sound", "sound"), ("A Song", "music")] {
            if ui.button(said).clicked() {
                resources.open_making(crate::Maker::new_of(&resources.session, kind));
                ui.close();
            }
        }
        let songs = workshop_music::songs(&resources.session);
        if !songs.is_empty() {
            ui.separator();
            ui.label(theme::text("Open a Song", theme::SMALL, theme::WEAK));
            for word in songs {
                if ui.button(&word).clicked() {
                    resources.open_making(crate::Maker::Music(Box::new(workshop_music::MusicMaking::open(&resources.session, &word))));
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
        crate::Maker::Picture(painting) => workshop_picture::paint(ui, painting),
        crate::Maker::Sound(making) => workshop_sound::make_sound(ui, making, &mut resources.audio, resources.volume),
        crate::Maker::Music(music) => workshop_music::make_music(ui, music, &mut resources.audio, resources.volume),
        crate::Maker::Dungeon(dungeon) => workshop_dungeon::make_dungeon(ui, dungeon, &mut resources.audio, resources.volume),
    };
    match done {
        Some(workshop_base::done::Done::Picture(word)) => {
            resources.lists.remove(&Shelf::Drawn);
            resources.pictures.remove(&format!("{}:{word}", Shelf::Drawn.key()));
            (resources.shelf, resources.chosen) = (Shelf::Drawn, Some(format!("{}:{word}", Shelf::Drawn.key())));
        }
        Some(workshop_base::done::Done::Sound(word)) => {
            resources.lists.remove(&Shelf::Sounds);
            (resources.shelf, resources.chosen) = (Shelf::Sounds, Some(format!("{}:{word}", Shelf::Sounds.key())));
        }
        Some(workshop_base::done::Done::Fight(word)) => {
            let fight = if word.is_empty() {
                workshop_dungeon::DungeonMaking::new_fight(&resources.session, "new_fight")
            } else {
                workshop_dungeon::DungeonMaking::open_fight(&resources.session, &word)
            };
            resources.open_making(crate::Maker::Dungeon(Box::new(fight)));
        }
        Some(workshop_base::done::Done::Closed) => {
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
