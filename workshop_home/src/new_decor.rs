//! The New Decor window: a title, and what it starts from: one of the game's
//! places (all of it, each element to move or take out, or its ground
//! alone, to lay one's own), or an empty page of a size. Its brush then
//! paints with the place's own tiles (`workshop_decor::Decors::open`).

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, RichText, vec2};

use workshop_base::session::Session;
use workshop_base::theme;

use crate::new_film::ui::{Done, PlaceGrid, file_named};

/// What a new decor starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum From {
    /// A place of the game, with its elements (`true`) or its ground alone.
    Place(bool),
    /// An empty page.
    Empty,
}

/// The sizes an empty decor is offered in (pixels: the DS's screen, twice, three times).
pub const SIZES: [(i64, i64); 3] = [(256, 192), (512, 384), (768, 576)];

/// The window, while open.
pub struct NewDecor {
    session: Arc<Session>,
    pub title: String,
    pub from: From,
    /// The place chosen (its level), and the size of an empty one.
    pub level: Option<i64>,
    pub size: (i64, i64),
    sought: String,
    grid: PlaceGrid,
    said: Option<String>,
}

/// A decor of a place's ground alone, its size: the place's picture without its elements, locked under all one lays.
pub fn ground_only(name: &str, title: &str, level: i64, size: (i64, i64)) -> String {
    format!("decor: {name} {} {}\nname {title}\nlike {level}\n{}\n", size.0, size.1, workshop_decor::edit::backdrop_line(level))
}

/// An empty decor.
pub fn empty(name: &str, title: &str, size: (i64, i64)) -> String {
    format!("decor: {name} {} {}\nname {title}\n", size.0, size.1)
}

impl NewDecor {
    pub fn new(session: &Arc<Session>) -> NewDecor {
        NewDecor {
            session: session.clone(),
            title: String::new(),
            from: From::Place(true),
            level: None,
            size: SIZES[1],
            sought: String::new(),
            grid: PlaceGrid::new(session, true),
            said: None,
        }
    }

    /// As a launch option asks (`WORKSHOP_NEW=decor[:<place's word>]`), for a picture.
    pub fn asked(session: &Arc<Session>, said: &str) -> NewDecor {
        let mut new = NewDecor::new(session);
        if let Some(word) = said.split(':').nth(1) {
            new.level = new.grid.cards.iter().find(|card| card.word == word).and_then(|card| card.level);
            new.title = "My Place".to_string();
        }
        new
    }

    fn path(&self) -> Option<PathBuf> {
        file_named(&self.session.library().decors(), &self.title, "decor")
    }

    /// The size of a place of the game.
    fn size_of(&self, level: i64) -> (i64, i64) {
        let tables = self.session.tables();
        tables.levels.values().find(|known| known.level as i64 == level).map_or(SIZES[1], |known| (known.size.0 as i64, known.size.1 as i64))
    }

    /// Its text, or what is wrong.
    fn text(&self, name: &str) -> Result<String, String> {
        let title = self.title.trim();
        match (self.from, self.level) {
            (From::Empty, _) => Ok(empty(name, title, self.size)),
            (From::Place(_), None) => Err("choose a place".to_string()),
            (From::Place(false), Some(level)) => Ok(ground_only(name, title, level, self.size_of(level))),
            (From::Place(true), Some(level)) => {
                decor::recreated(self.session.library(), self.session.data(), level, name, title).map_err(|why| why.message.clone())
            }
        }
    }

    /// The window drawn; what it did.
    pub fn ui(&mut self, context: &egui::Context, pictures: &mut workshop_base::example::Examples) -> Done {
        let mut open = true;
        let mut create = false;
        let size = vec2(860.0, 560.0);
        egui::Window::new(RichText::new("New Decor").strong())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                ui.set_min_size(size);
                ui.horizontal(|ui| {
                    ui.label(theme::weak("Title"));
                    let field = ui.add(egui::TextEdit::singleline(&mut self.title).hint_text("My Harbour").desired_width(260.0));
                    if self.title.is_empty() && !field.has_focus() {
                        field.request_focus();
                    }
                    let file = self.path().map_or("—".to_string(), |path| path.file_name().unwrap_or_default().to_string_lossy().into_owned());
                    ui.label(theme::weak(format!("library/decors/{file}")));
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(theme::weak("Start from"));
                    for (from, word, said) in [
                        (From::Place(true), "A Place of the Game", "the place as the game has it: each house, tree, rock an element to move or take out"),
                        (From::Place(false), "Its Ground Only", "the place without its elements: its ground, to lay one's own on"),
                        (From::Empty, "An Empty Page", "nothing: paint terrains, lay backdrops and elements from the Library"),
                    ] {
                        if ui.selectable_label(self.from == from, word).on_hover_text(said).clicked() {
                            self.from = from;
                        }
                    }
                });
                ui.add_space(6.0);
                egui::Frame::new().show(ui, |ui| {
                    ui.set_height(size.y - 110.0);
                    ui.set_width(size.x);
                    match self.from {
                        From::Empty => {
                            ui.label(theme::weak("Its size, in pixels (the DS's screen is 256 × 192); changed later in the Inspector."));
                            ui.horizontal(|ui| {
                                for size in SIZES {
                                    if ui.selectable_label(self.size == size, format!("{} × {}", size.0, size.1)).clicked() {
                                        self.size = size;
                                    }
                                }
                            });
                        }
                        From::Place(_) => {
                            ui.horizontal(|ui| {
                                ui.label(theme::text("Which place?", theme::TITLE, theme::STRONG));
                                ui.add(egui::TextEdit::singleline(&mut self.sought).hint_text("Search the places").desired_width(300.0));
                            });
                            let chosen =
                                self.level.and_then(|level| self.grid.cards.iter().find(|card| card.level == Some(level))).map(|card| card.word.clone());
                            if let Some((at, twice)) = self.grid.show(ui, chosen.as_deref(), &self.sought, pictures, theme::DECOR) {
                                self.level = self.grid.cards[at].level;
                                if self.title.trim().is_empty() {
                                    self.title = format!("My {}", self.grid.cards[at].name);
                                }
                                create |= twice;
                            }
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let free = self.path().is_some_and(|path| !path.exists());
                    let ready = free && (self.from == From::Empty || self.level.is_some());
                    let button = egui::Button::new(RichText::new("Create the Decor").strong().color(theme::GROUND)).fill(theme::DECOR);
                    create |= ui.add_enabled(ready, button).clicked();
                    let why = if self.path().is_none() {
                        "a title first"
                    } else if !free {
                        "a decor of this name already exists"
                    } else if !ready {
                        "choose a place"
                    } else {
                        "it opens in the Decor space, its brush on the place's own tiles"
                    };
                    ui.label(theme::text(why, theme::SMALL, theme::WEAK));
                    if let Some(said) = &self.said {
                        ui.label(theme::text(said, theme::SMALL, theme::WRONG));
                    }
                });
            });
        if create && let Some(path) = self.path().filter(|path| !path.exists()) {
            let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            match self.text(&name).and_then(|text| std::fs::write(&path, text).map_err(|why| format!("{}: {why}", path.display()))) {
                Ok(()) => return Done::Made(path),
                Err(why) => self.said = Some(why),
            }
        }
        if open { Done::Open } else { Done::Closed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Its ground alone: the place's size, like it, its picture laid locked under all; empty: its size alone.
    #[test]
    fn a_new_decor_starts_from_a_place_or_empty() {
        let text = ground_only("my_beach", "My Beach", 11, (792, 384));
        assert_eq!(text, "decor: my_beach 792 384\nname My Beach\nlike 11\nput r11 0 0 shadow off layer ground locked\n");
        assert_eq!(workshop_decor::edit::backdrop_of(&text), Some((4, 11)));
        assert_eq!(workshop_decor::own_place(&text), Some(11), "its brush on the beach's tiles");
        assert_eq!(empty("a", "A", (256, 192)), "decor: a 256 192\nname A\n");
        assert_eq!(workshop_decor::own_place(&empty("a", "A", (256, 192))), None);
    }

    /// A place of the game made again whole (with the game's files): its elements laid, its name and title.
    #[test]
    fn a_place_of_the_game_is_made_again_whole() {
        let data = assets::testing::data();
        if !assets::testing::extracted(&data) {
            return;
        }
        let text = decor::recreated(&assets::testing::library(), &data, 11, "my_beach", "My Beach").expect("made");
        assert!(text.contains("decor: my_beach ") && text.contains("name My Beach\nlike 11\n"), "{text}");
        assert!(text.lines().filter(|line| line.starts_with("put e11_")).count() > 0, "{text}");
    }
}
