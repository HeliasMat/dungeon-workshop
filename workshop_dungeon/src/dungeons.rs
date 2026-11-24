//! The Dungeon space: the dungeons and the boss fights, listed side by side,
//! and their editors (on the whole window, each open one its tab in the tools bar).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use assets::project::Project;
use std::time::Instant;
use workshop_base::session::Session;

use eframe::egui::{self, RichText, Sense};
use shell::sound::Hears;

use crate::{DungeonMaking, Summary};
use workshop_base::done::Done;
use workshop_base::{tabs, theme};

/// What to open in the Dungeon space: a new one, one by its word (the open
/// game's own if it has one by that word, else the shared library's), or
/// one by its file (a dungeon's or a fight's, wherever it is).
pub enum Open {
    NewDungeon,
    NewFight,
    Dungeon(String),
    Fight(String),
    File(PathBuf),
}

/// The Dungeon space as the window keeps it.
pub struct Dungeons {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    /// The dungeons and fights open (each its tab), the one shown; the list shown instead.
    open: Vec<DungeonMaking>,
    at: usize,
    listing: bool,
    /// One closed while not written, asked once more.
    closing: Option<usize>,
    audio: Option<Box<dyn Hears>>,
    volume: f32,
    /// The dungeons and fights written, summed up (the open game's first); when they were read.
    lists: Option<(Instant, Vec<Summary>, Vec<Summary>)>,
    /// The game open in the Game space: its folder and its title (its dungeons and fights listed too).
    pub game: Option<(PathBuf, String)>,
    pub said: Option<String>,
}

impl Dungeons {
    pub fn new(session: &Arc<Session>) -> Dungeons {
        let mut dungeons = Dungeons {
            session: session.clone(),
            open: Vec::new(),
            at: 0,
            listing: true,
            closing: None,
            audio: None,
            volume: 0.8,
            lists: None,
            game: None,
            said: None,
        };
        // (`WORKSHOP_CREATE=dungeon:my_dungeon` or `fight:my_fight`: that one open, for a picture.)
        for kind in session.launch.create.clone().unwrap_or_default().split(',') {
            match kind {
                "dungeon" => dungeons.open(Open::NewDungeon),
                "fight" => dungeons.open(Open::NewFight),
                _ if kind.starts_with("dungeon:") => dungeons.open(Open::Dungeon(kind["dungeon:".len()..].to_string())),
                _ if kind.starts_with("fight:") => dungeons.open(Open::Fight(kind["fight:".len()..].to_string())),
                _ => {}
            }
        }
        dungeons
    }

    /// The game open's project, if one is.
    fn game_project(&self) -> Option<Project> {
        self.game.as_ref().map(|(folder, _)| self.session.library().at(folder))
    }

    /// The project a dungeon or a fight of that word is in: the open game's, if it has it, else the shared library.
    fn project_of(&self, word: &str, fight: bool) -> Project {
        let (kind, end) = if fight { ("fights", "fight") } else { ("dungeons", "dungeon") };
        self.game_project().filter(|game| game.kind(kind).join(format!("{word}.{end}")).is_file()).unwrap_or_else(|| self.session.library().clone())
    }

    /// One opened in its editor (already open and written: shown again).
    pub fn open(&mut self, what: Open) {
        let making = match what {
            Open::NewDungeon => DungeonMaking::new(&self.session, "new_dungeon"),
            Open::NewFight => DungeonMaking::new_fight(&self.session, "new_fight"),
            Open::Dungeon(word) => DungeonMaking::open_in(&self.session, &self.project_of(&word, false), &word),
            Open::Fight(word) => DungeonMaking::open_fight_in(&self.session, &self.project_of(&word, true), &word),
            Open::File(path) => {
                let word = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                // (`<project>/dungeons/<word>.dungeon`: the project is two folders up.)
                let folder = path.parent().and_then(Path::parent).map_or_else(|| self.session.library().folder().to_path_buf(), Path::to_path_buf);
                let project = self.session.library().at(folder);
                match path.extension().is_some_and(|end| end == "fight") {
                    true => DungeonMaking::open_fight_in(&self.session, &project, &word),
                    false => DungeonMaking::open_in(&self.session, &project, &word),
                }
            }
        };
        let same = self.open.iter().position(|open| open.saved && making.saved && open.path() == making.path());
        self.at = same.unwrap_or_else(|| {
            self.open.push(making);
            self.open.len() - 1
        });
        self.listing = false;
        self.closing = None;
    }

    /// How many open ones are not written yet.
    pub fn unsaved_count(&self) -> usize {
        self.open.iter().filter(|open| !open.saved).count()
    }

    /// A file restored (`workshop_base::safe`): the open one written in it read again.
    pub fn reload(&mut self, path: &std::path::Path) {
        for open in &mut self.open {
            if open.path() == path {
                let (session, project) = (&self.session, open.project.clone());
                *open = if open.fight {
                    DungeonMaking::open_fight_in(session, &project, &open.word)
                } else {
                    DungeonMaking::open_in(session, &project, &open.word)
                };
            }
        }
        self.lists = None;
    }

    fn close(&mut self, at: usize) {
        if at < self.open.len() {
            self.open.remove(at);
        }
        self.at = self.at.min(self.open.len().saturating_sub(1));
        self.closing = None;
        if self.open.is_empty() {
            self.listing = true;
        }
        self.lists = None;
    }
}

/// The tools bar: the list, each one open (a click shows it, × closes it), new ones;
/// Ctrl+Tab the next, Ctrl+Shift+Tab the one before.
pub fn tabs(ui: &mut egui::Ui, dungeons: &mut Dungeons) {
    let (before, next) = ui.input_mut(|input| {
        (input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Tab), input.consume_key(egui::Modifiers::CTRL, egui::Key::Tab))
    });
    let count = dungeons.open.len();
    if count > 0 && (next || before) {
        dungeons.at = if before { (dungeons.at + count - 1) % count } else { (dungeons.at + 1) % count };
        (dungeons.listing, dungeons.closing) = (false, None);
    }
    let here = dungeons.listing || dungeons.open.is_empty();
    let all = egui::Button::new(RichText::new("All Dungeons").strong().color(if here { theme::STRONG } else { theme::TEXT })).fill(if here {
        theme::HOVERED
    } else {
        egui::Color32::TRANSPARENT
    });
    if ui.add(all).on_hover_text("every dungeon and boss fight written (those open stay open)").clicked() {
        (dungeons.listing, dungeons.lists) = (true, None);
    }
    ui.separator();
    let mut closed = None;
    for at in 0..dungeons.open.len() {
        let chosen = !dungeons.listing && at == dungeons.at;
        let making = &dungeons.open[at];
        let colour = if making.fight { theme::WRONG } else { theme::DUNGEON };
        let shown = tabs::Tab {
            id: egui::Id::new(("dungeon tab", at)),
            kind: if making.fight { "Fight" } else { "Dungeon" },
            colour,
            word: &making.word,
            saved: making.saved,
            chosen,
            asking: dungeons.closing == Some(at),
            game: making.own().then(|| making.path().display().to_string()),
        };
        match tabs::tab(ui, shown) {
            tabs::Clicked::Show => (dungeons.at, dungeons.listing, dungeons.closing) = (at, false, None),
            tabs::Clicked::Close if dungeons.open[at].saved || dungeons.closing == Some(at) => closed = Some(at),
            tabs::Clicked::Close => dungeons.closing = Some(at),
            tabs::Clicked::Nothing => {}
        }
    }
    if let Some(at) = closed {
        dungeons.close(at);
    }
    if ui.button(RichText::new("+ New Dungeon").strong().color(theme::DUNGEON)).clicked() {
        dungeons.open(Open::NewDungeon);
    }
    if ui.button(RichText::new("+ New Boss Fight").strong().color(theme::WRONG)).clicked() {
        dungeons.open(Open::NewFight);
    }
}

/// The space itself: the list, or the editor of the one shown.
pub fn page(ui: &mut egui::Ui, dungeons: &mut Dungeons) {
    if dungeons.listing || dungeons.open.is_empty() {
        list(ui, dungeons);
        return;
    }
    let at = dungeons.at;
    let Some(making) = dungeons.open.get_mut(at) else {
        return;
    };
    match crate::make_dungeon(ui, making, &mut dungeons.audio, dungeons.volume) {
        Some(Done::Fight(word)) => dungeons.open(if word.is_empty() { Open::NewFight } else { Open::Fight(word) }),
        Some(Done::Closed) => {
            if dungeons.open[at].saved || dungeons.closing == Some(at) {
                dungeons.close(at);
            } else {
                dungeons.closing = Some(at);
                dungeons.said = Some("not saved: Close again (or × on its tab) to close it anyway".to_string());
            }
        }
        _ => {}
    }
}

/// Every dungeon and every boss fight written, side by side; a click opens one.
fn list(ui: &mut egui::Ui, dungeons: &mut Dungeons) {
    if dungeons.lists.as_ref().is_none_or(|(when, _, _)| when.elapsed().as_secs() >= 3) {
        let session = &dungeons.session;
        let (mut written, mut fights) = (Vec::new(), Vec::new());
        // (The open game's own first, then the shared library's.)
        if let Some(game) = dungeons.game_project() {
            written.extend(crate::summaries_in(session, &game));
            fights.extend(crate::fight_summaries_in(session, &game));
        }
        written.extend(crate::summaries(session));
        fights.extend(crate::fight_summaries(session));
        dungeons.lists = Some((Instant::now(), written, fights));
    }
    let game_title = dungeons.game.as_ref().map(|(_, title)| title.clone()).unwrap_or_default();
    let mut asked = None;
    let Some((_, written, fights)) = &dungeons.lists else {
        return;
    };
    ui.columns(2, |columns| {
        for (ui, fight) in columns.iter_mut().zip([false, true]) {
            let (title, about, items) = if fight {
                ("BOSS FIGHTS", "an arena, a boss, its guards, its phases; a dungeon names it at its end: `fight: <word>`", fights)
            } else {
                ("DUNGEONS", "floors played turn by turn, the Pokémon met there, the items; in a film: `dungeon <word>`", written)
            };
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(theme::text(title, theme::TITLE, theme::STRONG));
                let (word, colour) = if fight { ("+ New Boss Fight", theme::WRONG) } else { ("+ New Dungeon", theme::DUNGEON) };
                if ui.add(egui::Button::new(RichText::new(word).strong().color(theme::GROUND)).fill(colour)).clicked() {
                    asked = Some(if fight { Open::NewFight } else { Open::NewDungeon });
                }
            });
            ui.label(theme::weak(about));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().id_salt(("dungeon list", fight)).auto_shrink(false).show(ui, |ui| {
                if items.is_empty() {
                    ui.label(theme::weak(if fight { "no boss fight yet" } else { "no dungeon yet" }));
                }
                for item in items {
                    if card(ui, item, fight, &game_title).clicked() {
                        asked = Some(Open::File(item.path.clone()));
                    }
                }
            });
        }
    });
    if let Some(what) = asked {
        dungeons.open(what);
    }
}

/// One of them as the list shows it: its name, its word, what is in it (the open game's own: said).
fn card(ui: &mut egui::Ui, item: &Summary, fight: bool, game: &str) -> egui::Response {
    let frame = egui::Frame::new().fill(theme::GROUND).corner_radius(4).inner_margin(egui::Margin::symmetric(10, 8));
    let shown = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::text(if item.name.is_empty() { &item.word } else { &item.name }, theme::BODY + 1.0, theme::STRONG));
            ui.label(theme::text(&item.word, theme::SMALL, theme::FAINT));
            if item.own {
                ui.label(theme::text(format!("{game}'s own"), theme::SMALL, theme::GAME));
            }
        });
        let mut said = Vec::new();
        if !fight {
            said.push(match item.floors {
                0 => "its fight only".to_string(),
                floors => format!("{floors} floor{}", if floors == 1 { "" } else { "s" }),
            });
        }
        if !item.look.is_empty() {
            said.push(item.look.clone());
        }
        if !fight && !item.foes.is_empty() {
            said.push(format!("{} Pokémon", item.foes.len()));
        }
        if let Some(boss) = &item.boss {
            let mut boss_said = format!("boss: {} Lv. {}", boss.species, boss.level);
            if boss.guards > 0 {
                boss_said += &format!(", {} guard{}", boss.guards, if boss.guards == 1 { "" } else { "s" });
            }
            if boss.phases > 0 {
                boss_said += &format!(", {} phase{}", boss.phases, if boss.phases == 1 { "" } else { "s" });
            }
            said.push(boss_said);
        }
        ui.label(theme::weak(said.join(" · ")));
    });
    ui.add_space(4.0);
    let response = ui.interact(shown.response.rect, egui::Id::new(("dungeon card", fight, &item.path)), Sense::click());
    if response.hovered() {
        ui.painter().rect_stroke(
            shown.response.rect,
            4.0,
            egui::Stroke::new(1.0_f32, if fight { theme::WRONG } else { theme::DUNGEON }),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("open it in its editor")
}
