//! The New Film window: four steps on one window (the place, the cast, the
//! start, the title), each chosen with a click, Back and Next under them;
//! Create writes the film (its cast placed by playing its scene once, on a
//! thread) and asks the window to open it in the Film space.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use eframe::egui::{self, Color32, FontId, Rect, RichText, Sense, Stroke, StrokeKind, vec2};

use workshop_base::ask::Ask;
use workshop_base::session::Session;
use workshop_base::theme;

use super::{FEWEST, MOST, Plan, Start};

/// The steps, in order.
const STEPS: [&str; 4] = ["Place", "Cast", "Start", "Title"];

/// A place offered: its word, its name, the shelf it is on, its level (a place of the game's picture).
pub struct PlaceCard {
    pub word: String,
    pub name: String,
    pub shelf: String,
    pub level: Option<i64>,
    /// A decor's own place (`like`, its backdrop): its picture shown while the decor's own is made.
    pub looks: Option<i64>,
}

/// A Pokémon offered: its word, its name, its portrait's number, its Pokédex number, a variant or not.
struct Person {
    word: String,
    name: String,
    number: i64,
    dex: i64,
    variant: bool,
}

/// The window, while open.
pub struct NewFilm {
    session: Arc<Session>,
    step: usize,
    sought: String,
    pub place: Option<String>,
    pub cast: Vec<String>,
    pub start: Start,
    pub title: String,
    grid: PlaceGrid,
    people: Vec<Person>,
    /// SpriteCollab's Pokémon not in the game nor imported: their Pokédex numbers, names.
    collab: Vec<(u32, String)>,
    faces: HashMap<i64, Option<egui::TextureHandle>>,
    making: Option<Receiver<String>>,
    said: Option<String>,
}

/// What the window did this frame.
pub enum Done {
    Open,
    Closed,
    /// The film written there.
    Made(PathBuf),
}

/// The file a new film or decor of that title will be (`the_big_day`), in that folder.
pub fn file_named(folder: &std::path::Path, title: &str, end: &str) -> Option<PathBuf> {
    let words: Vec<String> = title
        .split_whitespace()
        .map(|word| word.to_lowercase().chars().filter(|letter| letter.is_alphanumeric()).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect();
    (!words.is_empty()).then(|| folder.join(format!("{}.{end}", words.join("_"))))
}

impl NewFilm {
    pub fn new(session: &Arc<Session>) -> NewFilm {
        NewFilm {
            session: session.clone(),
            step: 0,
            sought: String::new(),
            place: None,
            cast: Vec::new(),
            start: Start::Talk,
            title: String::new(),
            grid: PlaceGrid::new(session, false),
            people: people(session),
            collab: Vec::new(),
            faces: HashMap::new(),
            making: None,
            said: None,
        }
    }

    /// As a launch option asks (`WORKSHOP_NEW=film:<step>:<place>:<a,b>:<start>`), for a picture.
    pub fn asked(session: &Arc<Session>, said: &str) -> NewFilm {
        let mut new = NewFilm::new(session);
        let parts: Vec<&str> = said.split(':').collect();
        new.step = parts.get(1).and_then(|step| step.parse::<usize>().ok()).map_or(0, |step| step.clamp(1, 4) - 1);
        new.place = parts.get(2).filter(|place| !place.is_empty()).map(|place| place.to_string());
        new.cast = parts.get(3).map(|cast| cast.split(',').filter(|word| !word.is_empty()).map(str::to_string).collect()).unwrap_or_default();
        new.start = match parts.get(4).copied() {
            Some("arrival") => Start::Arrival,
            Some("chase") => Start::Chase,
            Some("moments") => Start::TwoMoments,
            Some("empty") => Start::Empty,
            _ => Start::Talk,
        };
        if new.step == 3 {
            new.title = new.suggested_title();
        }
        new
    }

    fn plan(&self) -> Option<Plan> {
        let tables = self.session.tables();
        Some(Plan {
            title: self.title.trim().to_string(),
            place: self.place.clone()?,
            cast: self.cast.iter().map(|word| (word.clone(), super::shown_name(&tables, word))).collect(),
            start: self.start,
        })
    }

    fn path(&self) -> Option<PathBuf> {
        file_named(&self.session.library().films(), &self.title, "film")
    }

    /// Whether a step's choice is made (and the steps after it may be gone to).
    fn ready(&self, step: usize) -> bool {
        match step {
            0 => self.place.is_some(),
            1 => (FEWEST..=MOST).contains(&self.cast.len()),
            2 => true,
            _ => self.path().is_some_and(|path| !path.exists()),
        }
    }

    fn suggested_title(&self) -> String {
        let tables = self.session.tables();
        let names: Vec<String> = self.cast.iter().take(2).map(|word| super::shown_name(&tables, word)).collect();
        match self.start {
            Start::Arrival => format!("{} Is Late", names.last().cloned().unwrap_or_default()),
            Start::Chase => format!("Catch Me, {}", names.get(1).cloned().unwrap_or_default()),
            Start::TwoMoments => format!("A Day with {}", names.join(" and ")),
            _ => names.join(" and "),
        }
    }

    /// The window drawn; what it did.
    pub fn ui(&mut self, context: &egui::Context, pictures: &mut workshop_base::example::Examples) -> Done {
        if let Some(made) = self.making.as_ref().and_then(|making| making.try_recv().ok()) {
            self.making = None;
            return match self.path() {
                Some(path) => match std::fs::write(&path, made) {
                    Ok(()) => Done::Made(path),
                    Err(why) => {
                        self.said = Some(format!("{}: {why}", path.display()));
                        Done::Open
                    }
                },
                None => Done::Open,
            };
        }
        let mut open = true;
        let mut create = false;
        let size = vec2(860.0, 560.0);
        egui::Window::new(RichText::new("New Film").strong())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                ui.set_min_size(size);
                self.steps_bar(ui);
                ui.add_space(6.0);
                let body = size.y - 86.0;
                egui::Frame::new().show(ui, |ui| {
                    ui.set_height(body);
                    ui.set_width(size.x);
                    match self.step {
                        0 => self.places_step(ui, pictures),
                        1 => self.cast_step(ui),
                        2 => self.start_step(ui),
                        _ => create |= self.title_step(ui),
                    }
                });
                ui.separator();
                create |= self.buttons(ui);
            });
        if create && self.making.is_none() && (0..4).all(|step| self.ready(step)) {
            self.make();
        }
        if open { Done::Open } else { Done::Closed }
    }

    /// The film made on a thread: its music read, its cast placed by playing its scene.
    fn make(&mut self) {
        let Some(plan) = self.plan() else { return };
        let (sender, receiver) = std::sync::mpsc::channel();
        let (data, library, tables) = (self.session.data().to_string(), self.session.library().clone(), self.session.tables());
        std::thread::spawn(move || {
            _ = sender.send(super::made(&plan, &data, &library, tables));
        });
        self.making = Some(receiver);
    }

    /// 1 Place · 2 Cast · 3 Start · 4 Title: where one is, those done (a click goes back to one).
    fn steps_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (at, name) in STEPS.iter().enumerate() {
                let here = at == self.step;
                let reachable = (0..at).all(|before| self.ready(before));
                let chosen = match at {
                    0 => self.place.as_ref().map(|word| place_name(&self.grid.cards, word)),
                    1 if !self.cast.is_empty() => Some(format!("{} Pokémon", self.cast.len())),
                    2 if self.step > 2 => Some(self.start.name().to_string()),
                    _ => None,
                };
                let colour = if here {
                    theme::FILM
                } else if reachable {
                    theme::TEXT
                } else {
                    theme::FAINT
                };
                let mut word = RichText::new(format!("{} {name}", at + 1)).color(colour);
                if here {
                    word = word.strong();
                }
                let response = ui.add_enabled(reachable, egui::Button::new(word).fill(if here { theme::HOVERED } else { Color32::TRANSPARENT }));
                if response.clicked() {
                    self.step = at;
                    self.sought.clear();
                }
                if let Some(chosen) = chosen {
                    ui.label(theme::text(chosen, theme::SMALL, theme::WEAK));
                }
                if at + 1 < STEPS.len() {
                    ui.label(theme::text("›", theme::BODY, theme::FAINT));
                }
            }
        });
    }

    /// Back, Next or Create; why not yet.
    fn buttons(&mut self, ui: &mut egui::Ui) -> bool {
        let mut create = false;
        ui.horizontal(|ui| {
            if self.step > 0 && ui.button("‹ Back").clicked() {
                self.step -= 1;
                self.sought.clear();
            }
            let why = match self.step {
                0 if self.place.is_none() => "choose a place",
                1 if self.cast.len() < FEWEST => "choose two Pokémon or more",
                1 if self.cast.len() > MOST => "six Pokémon at most",
                3 if self.path().is_none() => "a title first",
                3 if !self.ready(3) => "a film of this name already exists",
                _ => "",
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = self.ready(self.step);
                if self.step < 3 {
                    let next = egui::Button::new(RichText::new("Next ›").strong().color(theme::GROUND)).fill(theme::FILM);
                    if ui.add_enabled(ready, next).clicked() {
                        self.step += 1;
                        self.sought.clear();
                        if self.step == 3 && self.title.trim().is_empty() {
                            self.title = self.suggested_title();
                        }
                    }
                } else if self.making.is_some() {
                    ui.spinner();
                    ui.label(theme::weak("placing the cast on the ground…"));
                } else {
                    let button = egui::Button::new(RichText::new("Create the Film").strong().color(theme::GROUND)).fill(theme::FILM);
                    create = ui.add_enabled(ready, button).clicked();
                }
                if !why.is_empty() {
                    ui.label(theme::text(why, theme::SMALL, theme::WEAK));
                }
                if let Some(said) = &self.said {
                    ui.label(theme::text(said, theme::SMALL, theme::WRONG));
                }
            });
        });
        create
    }

    fn search(&mut self, ui: &mut egui::Ui, hint: &str) {
        let field = egui::TextEdit::singleline(&mut self.sought).hint_text(hint).desired_width(320.0);
        let response = ui.add(field);
        if !response.has_focus() && self.sought.is_empty() {
            response.request_focus();
        }
    }

    // --- 1. The place ---

    fn places_step(&mut self, ui: &mut egui::Ui, pictures: &mut workshop_base::example::Examples) {
        ui.horizontal(|ui| {
            ui.label(theme::text("Where does it happen?", theme::TITLE, theme::STRONG));
            ui.add_space(12.0);
            self.search(ui, "Search the places (beach, town, cave…)");
        });
        ui.add_space(4.0);
        let sought = self.sought.clone();
        let chosen = self.grid.show(ui, self.place.as_deref(), &sought, pictures, theme::FILM);
        if let Some((at, twice)) = chosen {
            self.place = Some(self.grid.cards[at].word.clone());
            if twice {
                self.step = 1;
                self.sought.clear();
            }
        }
    }

    // --- 2. The cast ---

    fn cast_step(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(theme::text("Who is in it?", theme::TITLE, theme::STRONG));
            ui.add_space(12.0);
            self.search(ui, "Search a Pokémon (name or number)");
        });
        ui.add_space(4.0);
        // (The cast chosen, in order: the first two talk first.)
        ui.horizontal_wrapped(|ui| {
            ui.label(theme::text(format!("Cast {}/{MOST}:", self.cast.len()), theme::BODY, theme::TEXT));
            if self.cast.is_empty() {
                ui.label(theme::weak("click two to six Pokémon below; the first two lead"));
            }
            let mut gone = None;
            for (at, word) in self.cast.iter().enumerate() {
                let name = self.people.iter().find(|person| person.word == *word).map_or(word.clone(), |person| person.name.clone());
                if ui
                    .add(egui::Button::new(RichText::new(format!("{name}  ×")).color(theme::GROUND)).fill(theme::FILM).corner_radius(10))
                    .on_hover_text("take out of the cast")
                    .clicked()
                {
                    gone = Some(at);
                }
            }
            if let Some(at) = gone {
                self.cast.remove(at);
            }
        });
        ui.add_space(4.0);
        let sought = film::words::plain(self.sought.trim());
        let shown: Vec<usize> = (0..self.people.len())
            .filter(|at| {
                let person = &self.people[*at];
                sought.is_empty()
                    || person.word.contains(&sought)
                    || film::words::plain(&person.name).contains(&sought)
                    || (person.dex > 0 && person.dex.to_string() == sought)
            })
            .collect();
        let mut loads = 8;
        let mut clicked = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for at in &shown {
                    if self.person_card(ui, *at, &mut loads) {
                        clicked = Some(*at);
                    }
                }
            });
            if shown.is_empty() {
                ui.label(theme::weak("no Pokémon of the project by that name"));
            }
            // (Those of SpriteCollab not imported yet: imported from its window first.)
            if !sought.is_empty() {
                if self.collab.is_empty() {
                    self.collab = collab(&self.session, &self.people);
                }
                let found: Vec<&(u32, String)> =
                    self.collab.iter().filter(|(dex, name)| film::words::plain(name).contains(&sought) || dex.to_string() == sought).take(24).collect();
                if !found.is_empty() {
                    ui.add_space(8.0);
                    ui.label(theme::text("ON SPRITECOLLAB, NOT IMPORTED YET", theme::SMALL, theme::WEAK));
                    ui.horizontal_wrapped(|ui| {
                        for (dex, name) in found {
                            if ui
                                .button(format!("{name} · No. {dex}"))
                                .on_hover_text("open SpriteCollab on it, to import it: then it is in this list")
                                .clicked()
                            {
                                self.session.ask(Ask::SpriteCollab(Some(*dex)));
                            }
                        }
                    });
                }
            }
        });
        if let Some(at) = clicked {
            let word = self.people[at].word.clone();
            match self.cast.iter().position(|known| *known == word) {
                Some(there) => {
                    self.cast.remove(there);
                }
                None if self.cast.len() < MOST => self.cast.push(word),
                None => self.said = Some("six Pokémon at most: take one out first".to_string()),
            }
        }
    }

    /// A Pokémon's card: its portrait, its name, its place in the cast; clicked.
    fn person_card(&mut self, ui: &mut egui::Ui, at: usize, loads: &mut usize) -> bool {
        let (rect, response) = ui.allocate_exact_size(vec2(86.0, 96.0), Sense::click());
        if !ui.is_rect_visible(rect) {
            return false;
        }
        let person = &self.people[at];
        let number = person.number;
        let in_cast = self.cast.iter().position(|word| *word == person.word);
        if !self.faces.contains_key(&number) && *loads > 0 {
            *loads -= 1;
            let path = PathBuf::from(assets::data::path(self.session.data(), &format!("portraits/{number}_0.png")));
            self.faces.insert(number, texture_of(ui.ctx(), &path, &format!("new film face {number}")));
        }
        if !self.faces.contains_key(&number) {
            ui.ctx().request_repaint();
        }
        let person = &self.people[at];
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            3.0,
            if in_cast.is_some() {
                theme::CHOSEN
            } else if response.hovered() {
                theme::HOVERED
            } else {
                theme::GROUND
            },
        );
        let room = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 34.0), vec2(56.0, 56.0));
        match self.faces.get(&number).cloned().flatten() {
            Some(texture) => {
                painter.image(texture.id(), room, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
            None => {
                painter.rect_filled(room, 2.0, theme::PANEL);
                let first = person.name.chars().next().map(|letter| letter.to_string()).unwrap_or_default();
                painter.text(room.center(), egui::Align2::CENTER_CENTER, first, FontId::proportional(24.0), theme::FILM);
            }
        }
        if let Some(place) = in_cast {
            let badge = Rect::from_min_size(rect.min + vec2(3.0, 3.0), vec2(18.0, 18.0));
            painter.rect_filled(badge, 9.0, theme::FILM);
            painter.text(badge.center(), egui::Align2::CENTER_CENTER, (place + 1).to_string(), FontId::proportional(theme::SMALL), theme::GROUND);
        }
        let name = painter.layout(person.name.clone(), FontId::proportional(theme::SMALL), theme::STRONG, rect.width() - 4.0);
        let left = rect.center().x - name.size().x / 2.0;
        painter.galley(egui::pos2(left, rect.bottom() - 26.0), name, theme::STRONG);
        let about = if person.variant {
            format!("{} · a variant", person.word)
        } else if person.dex > 0 {
            format!("{} · Pokédex No. {}", person.word, person.dex)
        } else {
            person.word.clone()
        };
        response.on_hover_text(about).clicked()
    }

    // --- 3. The start ---

    fn start_step(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::text("How does it begin?", theme::TITLE, theme::STRONG));
        ui.label(theme::weak("a first scene to change at will; each plays at once, the cast already placed on the ground"));
        ui.add_space(6.0);
        ui.columns(2, |columns| {
            for start in Start::ALL {
                let chosen = self.start == start;
                let text = RichText::new(format!("{}\n", start.name())).strong().color(if chosen { theme::STRONG } else { theme::TEXT });
                let button =
                    egui::Button::new(text).fill(if chosen { theme::CHOSEN } else { theme::GROUND }).min_size(vec2(columns[0].available_width(), 44.0));
                let response = columns[0].add(button);
                let rect = response.rect;
                columns[0].painter().text(
                    rect.left_bottom() + vec2(8.0, -8.0),
                    egui::Align2::LEFT_BOTTOM,
                    start.about(),
                    FontId::proportional(theme::SMALL),
                    if chosen { theme::TEXT } else { theme::WEAK },
                );
                if response.clicked() {
                    self.start = start;
                }
                if response.double_clicked() {
                    self.step = 3;
                    if self.title.trim().is_empty() {
                        self.title = self.suggested_title();
                    }
                }
                columns[0].add_space(4.0);
            }
            // (What it will write, with this cast.)
            columns[1].label(theme::text("What it writes", theme::SMALL, theme::WEAK));
            if let Some(plan) = self.plan() {
                let text = super::film_text(&Plan { title: if plan.title.is_empty() { "…".to_string() } else { plan.title.clone() }, ..plan }, None, &[]);
                let shown: String = text.lines().skip(3).filter(|line| !line.is_empty()).take(18).collect::<Vec<_>>().join("\n");
                egui::Frame::new().fill(theme::FIELD).inner_margin(8).corner_radius(3).show(&mut columns[1], |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(shown).monospace().size(theme::SMALL).color(theme::TEXT));
                });
            }
        });
    }

    // --- 4. The title ---

    fn title_step(&mut self, ui: &mut egui::Ui) -> bool {
        ui.label(theme::text("Its title", theme::TITLE, theme::STRONG));
        ui.add_space(6.0);
        let field = ui.add(egui::TextEdit::singleline(&mut self.title).hint_text("The Big Day").desired_width(420.0).font(FontId::proportional(theme::TITLE)));
        if !field.has_focus() && self.making.is_none() {
            field.request_focus();
        }
        let enter = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        ui.add_space(4.0);
        let file = self.path().map_or("—".to_string(), |path| path.file_name().unwrap_or_default().to_string_lossy().into_owned());
        ui.label(theme::weak(format!("library/films/{file}")));
        ui.add_space(14.0);
        egui::Grid::new("new film summary").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(theme::weak("Place"));
            ui.label(self.place.as_ref().map_or(String::new(), |word| place_name(&self.grid.cards, word)));
            ui.end_row();
            ui.label(theme::weak("Cast"));
            let names: Vec<String> = self
                .cast
                .iter()
                .map(|word| self.people.iter().find(|person| person.word == *word).map_or(word.clone(), |person| person.name.clone()))
                .collect();
            ui.label(names.join(", "));
            ui.end_row();
            ui.label(theme::weak("Start"));
            ui.label(self.start.name());
            ui.end_row();
        });
        ui.add_space(14.0);
        ui.label(theme::weak(
            "Create writes the film and opens it in the Film space: it plays at once. Then write on in the Script, \
             or drag someone on the Scene to move them (their line is written again).",
        ));
        enter
    }
}

/// The places to choose from, in a grid of cards by shelf, their pictures
/// read as they come into view (the New Film and New Decor windows').
pub struct PlaceGrid {
    session: Arc<Session>,
    pub cards: Vec<PlaceCard>,
    thumbs: HashMap<i64, Option<egui::TextureHandle>>,
}

impl PlaceGrid {
    /// Every place (`game_only`: the game's places alone, those with a picture of their own).
    pub fn new(session: &Arc<Session>, game_only: bool) -> PlaceGrid {
        let mut cards = places(session);
        if game_only {
            cards.retain(|card| card.level.is_some() && card.shelf != FAVOURITES);
        }
        PlaceGrid { session: session.clone(), cards, thumbs: HashMap::new() }
    }

    /// The grid of those `sought` finds; the card clicked (twice?).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        chosen: Option<&str>,
        sought: &str,
        pictures: &mut workshop_base::example::Examples,
        accent: Color32,
    ) -> Option<(usize, bool)> {
        let sought = film::words::plain(sought.trim());
        let shown: Vec<usize> = (0..self.cards.len())
            .filter(|at| {
                let card = &self.cards[*at];
                sought.is_empty() || card.word.contains(&sought) || film::words::plain(&card.name).contains(&sought)
            })
            .collect();
        let mut loads = 4;
        let mut clicked = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let mut at = 0;
            while at < shown.len() {
                let shelf = self.cards[shown[at]].shelf.clone();
                let end = shown[at..].iter().position(|one| self.cards[*one].shelf != shelf).map_or(shown.len(), |more| at + more);
                ui.add_space(6.0);
                ui.label(theme::text(shelf.to_uppercase(), theme::SMALL, theme::WEAK));
                ui.horizontal_wrapped(|ui| {
                    for one in &shown[at..end] {
                        if let Some(twice) = self.card(ui, *one, chosen, &mut loads, pictures, accent) {
                            clicked = Some((*one, twice));
                        }
                    }
                });
                at = end;
            }
            if shown.is_empty() {
                ui.label(theme::weak("no place by that name"));
            }
        });
        clicked
    }

    /// A place's picture made small, read if there is room for it this frame.
    fn thumb(&mut self, context: &egui::Context, level: i64, loads: &mut usize) -> Option<egui::TextureHandle> {
        if !self.thumbs.contains_key(&level) && *loads > 0 {
            *loads -= 1;
            self.thumbs.insert(level, workshop_decor::library::place_thumb(context, self.session.data(), level, 200));
        }
        if !self.thumbs.contains_key(&level) {
            context.request_repaint();
        }
        self.thumbs.get(&level).cloned().flatten()
    }

    /// A place's card: its picture, its name; clicked (twice?).
    fn card(
        &mut self,
        ui: &mut egui::Ui,
        at: usize,
        chosen: Option<&str>,
        loads: &mut usize,
        pictures: &mut workshop_base::example::Examples,
        accent: Color32,
    ) -> Option<bool> {
        let (rect, response) = ui.allocate_exact_size(vec2(156.0, 118.0), Sense::click());
        if !ui.is_rect_visible(rect) {
            return None;
        }
        let card = &self.cards[at];
        let chosen = chosen == Some(card.word.as_str());
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            3.0,
            if chosen {
                theme::CHOSEN
            } else if response.hovered() {
                theme::HOVERED
            } else {
                theme::GROUND
            },
        );
        let room = Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(rect.width() - 8.0, 86.0));
        painter.rect_filled(room, 2.0, theme::PANEL);
        let (level, looks) = (card.level, card.looks);
        let own = match level {
            Some(_) => None,
            None => pictures.picture(ui.ctx(), &format!("scene: {}\nwait 1.5s\n", card.word)).ok().flatten(),
        };
        if level.is_none() && own.is_none() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
        let texture = own.or_else(|| self.thumb(ui.ctx(), level.or(looks)?, loads));
        let card = &self.cards[at];
        if let Some(texture) = texture {
            workshop_decor::library::paint_filled(ui.painter(), &texture, room);
        }
        let painter = ui.painter();
        if chosen {
            painter.rect_stroke(room, 2.0, Stroke::new(2.0_f32, accent), StrokeKind::Inside);
        }
        let name = painter.layout(card.name.clone(), FontId::proportional(theme::BODY), theme::STRONG, rect.width() - 8.0);
        painter.galley(egui::pos2(rect.left() + 4.0, room.bottom() + 4.0), name, theme::STRONG);
        let response = response.on_hover_text(format!("{} · scene: {}", card.name, card.word));
        if response.double_clicked() {
            Some(true)
        } else if response.clicked() {
            Some(false)
        } else {
            None
        }
    }
}

/// The shelf of the places films use most.
const FAVOURITES: &str = "Favourite places";

/// A place's name, by its word.
fn place_name(places: &[PlaceCard], word: &str) -> String {
    places.iter().find(|card| card.word == word).map_or(word.to_string(), |card| card.name.clone())
}

/// A word made a name (`treasure_town`: Treasure town).
fn named(word: &str) -> String {
    let said = word.replace('_', " ");
    let mut letters = said.chars();
    letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
}

/// The places offered: the places films use most, the user's decors, then
/// every place of the game with a picture, by kind (`place_kinds.txt`).
fn places(session: &Session) -> Vec<PlaceCard> {
    let tables = session.tables();
    let mut cards: Vec<PlaceCard> = Vec::new();
    for (word, (level, ..)) in film::tables::PLACES {
        if level == film::tables::MAP_LEVEL {
            continue;
        }
        cards.push(PlaceCard { word: word.to_string(), name: named(word), shelf: FAVOURITES.to_string(), level: Some(level as i64), looks: None });
    }
    for word in decor::names(session.library()) {
        let text = std::fs::read_to_string(session.library().decors().join(format!("{word}.decor"))).unwrap_or_default();
        let looks = workshop_decor::own_place(&text);
        cards.push(PlaceCard { name: named(&word), word, shelf: "Your decors".to_string(), level: None, looks });
    }
    let kinds = workshop_base::project::place_kinds(session.library());
    let mut seen = std::collections::HashSet::new();
    let mut game: Vec<PlaceCard> = tables
        .levels
        .iter()
        .filter(|(_, level)| level.level != 0 && level.level != film::tables::MAP_LEVEL && !tables.picture_levels.contains(&level.level))
        .filter(|(_, level)| assets::data::has(session.data(), &format!("backdrops/{}.png", level.level)))
        .filter(|(_, level)| seen.insert(level.level))
        .map(|(word, level)| PlaceCard {
            word: word.clone(),
            name: if level.name.is_empty() { named(word) } else { level.name.clone() },
            shelf: format!("The game's places · {}", kinds.of_level(level.level as i64).shown()),
            level: Some(level.level as i64),
            looks: None,
        })
        .collect();
    game.sort_by(|a, b| a.shelf.cmp(&b.shelf).then(a.name.cmp(&b.name)));
    cards.extend(game);
    cards
}

/// The Pokémon offered: each species with a body (the game's and those imported), then the variants.
fn people(session: &Session) -> Vec<Person> {
    let tables = session.tables();
    let dex: HashMap<i64, i64> = assets::data::text(session.data(), "dex.txt")
        .lines()
        .filter_map(|line| {
            let (number, at) = line.split_once('\t')?;
            Some((number.trim().parse().ok()?, at.trim().parse().ok()?))
        })
        .collect();
    let drawn: std::collections::HashSet<&String> = tables.with_sprites.iter().collect();
    let mut by_name: std::collections::BTreeMap<String, (i64, String)> = std::collections::BTreeMap::new();
    for (word, (number, name)) in tables.monsters.iter().filter(|(word, _)| drawn.contains(word)) {
        let Ok(number) = number.parse::<i64>() else { continue };
        if number == 0 || name.is_empty() || name.starts_with('?') || film::words::plain(name) != *word {
            continue;
        }
        let kept = by_name.entry(name.clone()).or_insert((number, word.clone()));
        if number < kept.0 {
            *kept = (number, word.clone());
        }
    }
    let mut people: Vec<Person> =
        by_name.into_iter().map(|(name, (number, word))| Person { dex: dex.get(&number).copied().unwrap_or(0), word, name, number, variant: false }).collect();
    people.sort_by_key(|person| if person.dex > 0 { person.dex } else { 100_000 + person.number });
    for (of, words) in &tables.variants {
        let number = of.parse::<i64>().unwrap_or(0);
        for word in words {
            let own = tables.monsters.get(word).and_then(|(own, _)| own.parse::<i64>().ok()).unwrap_or(number);
            people.push(Person { word: word.clone(), name: named(word), number: own, dex: dex.get(&number).copied().unwrap_or(0), variant: true });
        }
    }
    people
}

/// SpriteCollab's Pokémon not among those offered (its clone's `tracker.json`).
fn collab(session: &Session, people: &[Person]) -> Vec<(u32, String)> {
    let known: std::collections::HashSet<i64> = people.iter().map(|person| person.dex).collect();
    spritecollab::tracker_names(&session.spritecollab).into_iter().filter(|(dex, _)| !known.contains(&(*dex as i64))).collect()
}

/// A picture of a file, as a texture (none if it cannot be read).
fn texture_of(context: &egui::Context, path: &std::path::Path, key: &str) -> Option<egui::TextureHandle> {
    let picture = image::open(path).ok()?.to_rgba8();
    let image = egui::ColorImage::from_rgba_unmultiplied([picture.width() as usize, picture.height() as usize], picture.as_raw());
    Some(context.load_texture(key, image, egui::TextureOptions::NEAREST))
}
