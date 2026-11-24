//! The window: a bar of menus (and the spaces, on its right), a bar of
//! tools for the space looked at, its panels docked under them (dragged
//! by their tabs anywhere), a bar of state at the bottom.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, RichText};
use egui_dock::{DockArea, DockState, NodeIndex, Style};

use workshop_base::project::{Entry, Kind, Project};
use workshop_base::session::Session;
use workshop_base::theme;

/// The spaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Home,
    Game,
    Film,
    Decor,
    Dungeon,
    Assets,
    Emulator,
}

impl Space {
    pub const ALL: [Space; 7] = [Space::Home, Space::Game, Space::Film, Space::Decor, Space::Dungeon, Space::Assets, Space::Emulator];

    pub fn word(self) -> &'static str {
        match self {
            Space::Home => "Home",
            Space::Game => "Game",
            Space::Film => "Film",
            Space::Decor => "Decor",
            Space::Dungeon => "Dungeon",
            Space::Assets => "Resources",
            Space::Emulator => "Emulator",
        }
    }

    pub fn accent(self) -> egui::Color32 {
        match self {
            Space::Home => theme::HOME,
            Space::Game => theme::GAME,
            Space::Film => theme::FILM,
            Space::Decor => theme::DECOR,
            Space::Dungeon => theme::DUNGEON,
            Space::Assets => theme::ASSETS,
            Space::Emulator => theme::EMULATOR,
        }
    }

    fn at(self) -> usize {
        self as usize
    }
}

/// The panels, each of one space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    // Home.
    Project,
    List,
    Preview,
    // Film.
    Script,
    FilmScene,
    FilmInspector,
    FilmState,
    Timeline,
    // Decor.
    Hierarchy,
    History,
    DecorScene,
    DecorInspector,
    Library,
    // Resources.
    AssetTree,
    AssetGrid,
    AssetInspector,
}

impl Panel {
    fn title(self) -> &'static str {
        match self {
            Panel::Project => "Project",
            Panel::List => "Files",
            Panel::Preview => "Preview",
            Panel::Script => "Script",
            Panel::FilmScene | Panel::DecorScene => "Scene",
            Panel::FilmInspector | Panel::DecorInspector | Panel::AssetInspector => "Inspector",
            Panel::FilmState => "State",
            Panel::Timeline => "Timeline",
            Panel::Hierarchy => "Hierarchy",
            Panel::History => "History",
            Panel::Library | Panel::AssetTree => "Library",
            Panel::AssetGrid => "Resources",
        }
    }
}

/// Each space's panels as they are laid out first.
fn layout(space: Space) -> DockState<Panel> {
    match space {
        Space::Home => {
            let mut dock = DockState::new(vec![Panel::List]);
            let surface = dock.main_surface_mut();
            let [list, _] = surface.split_left(NodeIndex::root(), 0.16, vec![Panel::Project]);
            surface.split_right(list, 0.76, vec![Panel::Preview]);
            dock
        }
        Space::Film => {
            let mut dock = DockState::new(vec![Panel::FilmScene]);
            let surface = dock.main_surface_mut();
            let [scene, _] = surface.split_below(NodeIndex::root(), 0.74, vec![Panel::Timeline]);
            let [scene, _] = surface.split_left(scene, 0.42, vec![Panel::Script]);
            let [_, inspector] = surface.split_right(scene, 0.74, vec![Panel::FilmInspector]);
            surface.split_below(inspector, 0.4, vec![Panel::FilmState]);
            dock
        }
        Space::Decor => {
            let mut dock = DockState::new(vec![Panel::DecorScene]);
            let surface = dock.main_surface_mut();
            let [scene, _] = surface.split_below(NodeIndex::root(), 0.62, vec![Panel::Library]);
            let [scene, _] = surface.split_left(scene, 0.17, vec![Panel::Hierarchy, Panel::History]);
            surface.split_right(scene, 0.76, vec![Panel::DecorInspector]);
            dock
        }
        // (Its editors, its columns on the whole window: no panels.)
        Space::Dungeon | Space::Game | Space::Emulator => DockState::new(Vec::new()),
        Space::Assets => {
            let mut dock = DockState::new(vec![Panel::AssetGrid]);
            let surface = dock.main_surface_mut();
            let [grid, _] = surface.split_left(NodeIndex::root(), 0.16, vec![Panel::AssetTree]);
            surface.split_right(grid, 0.78, vec![Panel::AssetInspector]);
            dock
        }
    }
}

/// A file being made: its kind, its title as typed.
pub struct Making {
    pub kind: Kind,
    pub title: String,
}

/// All the panels work with: the project, what is chosen and open, what was
/// just said; what a panel asks of the window.
pub struct Work {
    /// What every space works with (the library, the data, what is read of them).
    pub session: Arc<Session>,
    pub project: Project,
    /// The Home's own; the file chosen there.
    pub home: crate::home::Home,
    pub chosen: Option<PathBuf>,
    /// The film and the decor open.
    pub film: Option<PathBuf>,
    pub decor: Option<PathBuf>,
    pub said: String,
    pub making: Option<Making>,
    /// A space asked to be looked at.
    pub go: Option<Space>,
    /// The Decor space's own.
    pub decors: crate::decors::Decors,
    /// The Film space's Script; the film played.
    pub scripts: crate::script::Scripts,
    pub films: crate::films::Films,
    /// The Resources space's own.
    pub resources: crate::resources::Resources,
    /// The Dungeon space's own.
    pub dungeons: crate::dungeons::Dungeons,
    /// The Game space's own.
    pub games: crate::games::Games,
    /// The Emulator space's own.
    pub emulate: crate::emulate::Emulate,
}

impl Work {
    /// A file opened in its space.
    pub fn open(&mut self, entry: &Entry) {
        match entry.kind {
            Kind::Film => (self.film, self.go) = (Some(entry.path.clone()), Some(Space::Film)),
            Kind::Decor => {
                (self.decor, self.go) = (Some(entry.path.clone()), Some(Space::Decor));
                self.decors.open(&entry.path);
            }
        }
        self.said = format!("opened: {}", entry.title);
    }
}

/// The window.
pub struct App {
    space: Space,
    docks: [DockState<Panel>; 7],
    work: Work,
    capture: Option<crate::capture::Capture>,
    /// Tools › Convert an Image (Tilequant), while open.
    quant: Option<crate::quant_tool::QuantTool>,
    /// Tools › SpriteCollab, while open.
    spritecollab: Option<crate::spritecollab::Browser>,
    /// The "what to try first" panel, shown once the first launch is done.
    welcome: bool,
}

impl App {
    pub fn new(context: &egui::Context, session: Arc<Session>) -> App {
        theme::apply(context);
        let work = Work {
            project: Project::read(&session),
            home: crate::home::Home::new(&session, context),
            chosen: None,
            film: None,
            decor: None,
            said: String::new(),
            making: None,
            go: None,
            decors: crate::decors::Decors::new(&session, context),
            scripts: crate::script::Scripts::new(&session, context),
            films: crate::films::Films::new(&session, context),
            resources: crate::resources::Resources::new(&session, context),
            dungeons: crate::dungeons::Dungeons::new(&session),
            games: crate::games::Games::new(&session),
            emulate: crate::emulate::Emulate::new(&session),
            session,
        };
        // (`WORKSHOP_SPACE=game|film|decor|dungeon|resources`: that space first, for a picture.)
        let launch = work.session.launch.clone();
        let space = match launch.space.as_deref() {
            Some("game") => Space::Game,
            Some("film") => Space::Film,
            Some("decor") => Space::Decor,
            Some("dungeon") => Space::Dungeon,
            Some("resources") => Space::Assets,
            Some("emulator") => Space::Emulator,
            _ => Space::Home,
        };
        let mut app = App {
            space,
            docks: Space::ALL.map(layout),
            work,
            capture: crate::capture::Capture::asked(&launch),
            quant: crate::quant_tool::QuantTool::asked(context, &launch),
            spritecollab: None,
            welcome: launch.welcome,
        };
        if app.welcome {
            app.first_steps();
        }
        // (`WORKSHOP_SPRITECOLLAB=25`: that window open on that number, for a picture.)
        if let Some(dex) = &launch.spritecollab {
            app.spritecollab = Some(crate::spritecollab::Browser::new(&app.work.session, dex.parse().ok()));
        }
        // (A file named on the command line: opened in its space.)
        if let Some(path) = launch.file.clone() {
            let kind = if path.extension().is_some_and(|own| own == "decor") { Kind::Decor } else { Kind::Film };
            let entry = app.work.project.films.iter().chain(&app.work.project.decors).find(|entry| entry.path == path).cloned();
            let entry = entry.unwrap_or(Entry {
                path,
                kind,
                title: String::new(),
                changed: None,
                about: String::new(),
                lines: 0,
                text: String::new(),
                scenes: 0,
                cast: Vec::new(),
                places: Vec::new(),
            });
            app.work.open(&entry);
            app.space = app.work.go.take().unwrap_or(app.space);
        }
        // (`WORKSHOP_FOLDED="Transform,Resident"`: those components of the Inspector folded, for a picture.)
        for part in &launch.folded {
            app.work.decors.folded_parts.insert(part.clone());
        }
        // (`WORKSHOP_CHOSEN=n`: line n chosen, for a picture.)
        if let Some(line) = launch.chosen
            && let Some(open) = &mut app.work.decors.open
        {
            open.choose(vec![(line, 0)]);
        }
        // (`WORKSHOP_TOOL=terrain|scatter|walls|walk`: that brush in hand, for a picture.)
        app.work.decors.brushes.tool = match launch.tool.as_deref() {
            Some("terrain") => crate::terrains::Tool::Terrain,
            Some("scatter") => crate::terrains::Tool::Strew,
            Some("walls") => crate::terrains::Tool::Walls,
            Some("walk") => crate::terrains::Tool::Walk,
            _ => crate::terrains::Tool::Choose,
        };
        app
    }

    /// The bar of menus, and the spaces on its right.
    fn menus(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.label(theme::text("Workshop", theme::BODY, theme::STRONG).strong());
            ui.add_space(6.0);
            ui.menu_button("File", |ui| {
                if ui.button("New Film…").clicked() {
                    self.work.making = Some(Making { kind: Kind::Film, title: String::new() });
                }
                if ui.button("New Decor…").clicked() {
                    self.work.making = Some(Making { kind: Kind::Decor, title: String::new() });
                }
                ui.separator();
                ui.menu_button("Open Recent", |ui| {
                    let recent: Vec<Entry> = self.work.project.recent().into_iter().take(10).cloned().collect();
                    for entry in recent {
                        if ui.button(format!("{} — {}", entry.title, entry.kind.word())).clicked() {
                            self.work.open(&entry);
                        }
                    }
                });
                let open = self.space == Space::Decor && self.work.decors.open.is_some();
                if ui.add_enabled(open, egui::Button::new("Save").shortcut_text("Ctrl+S")).clicked() {
                    self.save();
                }
                if ui.button("Reload the Project").clicked() {
                    self.work.project = Project::read(&self.work.session);
                    self.work.said = "project reloaded".to_string();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                let open = self.space == Space::Decor && self.work.decors.open.is_some();
                if ui.add_enabled(open, egui::Button::new("Undo").shortcut_text("Ctrl+Z")).clicked()
                    && let Some(decor) = &mut self.work.decors.open
                {
                    decor.undo();
                }
                if ui.add_enabled(open, egui::Button::new("Redo").shortcut_text("Ctrl+Y")).clicked()
                    && let Some(decor) = &mut self.work.decors.open
                {
                    decor.redo();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button(format!("Default Layout ({})", self.space.word())).clicked() {
                    self.docks[self.space.at()] = layout(self.space);
                }
                for space in Space::ALL {
                    if ui.button(format!("Go to: {}", space.word())).clicked() {
                        self.space = space;
                    }
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui
                    .button("Convert an Image (Tilequant)…")
                    .on_hover_text("a picture to the console's rules: tiles of 8, 16 colours each; <name>_quant.png beside it")
                    .clicked()
                {
                    self.quant = Some(crate::quant_tool::QuantTool::new());
                }
                if ui.button("SpriteCollab…").on_hover_text("the PMDCollab community's sprites and portraits: search, look, import into data/extra").clicked()
                {
                    self.spritecollab = Some(crate::spritecollab::Browser::new(&self.work.session, None));
                }
            });
            ui.menu_button("Help", |ui| {
                ui.label(theme::weak("Dungeon Workshop: the project's films and decors."));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                for space in Space::ALL.into_iter().rev() {
                    let here = space == self.space;
                    let marked = format!("{}{}", space.word(), if self.unsaved(space) { " •" } else { "" });
                    let word = RichText::new(marked).color(if here { theme::STRONG } else { theme::TEXT });
                    let tab = egui::Button::new(word).fill(if here { theme::HOVERED } else { egui::Color32::TRANSPARENT }).corner_radius(0);
                    let response = ui.add(tab);
                    if here {
                        let line = egui::Rect::from_min_max(response.rect.left_top(), egui::pos2(response.rect.right(), response.rect.top() + 2.0));
                        ui.painter().rect_filled(line, 0.0, space.accent());
                    }
                    if response.clicked() {
                        self.space = space;
                    }
                }
            });
        });
    }

    /// The bar of tools of the space looked at.
    fn tools(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| match self.space {
            Space::Home => {
                let new = |ui: &mut egui::Ui, word: &str, colour: egui::Color32| {
                    ui.add(egui::Button::new(RichText::new(word).color(theme::GROUND).strong()).fill(colour).corner_radius(3))
                };
                if new(ui, "+ New Film", theme::FILM).clicked() {
                    self.work.making = Some(Making { kind: Kind::Film, title: String::new() });
                }
                if new(ui, "+ New Decor", theme::DECOR).clicked() {
                    self.work.making = Some(Making { kind: Kind::Decor, title: String::new() });
                }
                if new(ui, "Games", theme::GAME).on_hover_text("the games of the project: their chapters, played, checked, exported").clicked() {
                    self.space = Space::Game;
                }
            }
            Space::Decor => self.decor_tools(ui),
            Space::Film => self.film_tools(ui),
            Space::Dungeon => crate::dungeons::tabs(ui, &mut self.work.dungeons),
            Space::Game => crate::games::tools(ui, &mut self.work.games),
            Space::Emulator => crate::emulate::tools(ui, &mut self.work.emulate, self.work.film.as_deref()),
            Space::Assets if self.work.resources.creating() => crate::resources::making_tabs(ui, &mut self.work.resources),
            space => {
                let open = match space {
                    Space::Film => self.work.film.as_ref(),
                    Space::Decor => self.work.decor.as_ref(),
                    _ => None,
                };
                let said = open.map_or("no file open".to_string(), |path| path.file_name().unwrap_or_default().to_string_lossy().into_owned());
                ui.label(theme::weak(said));
            }
        });
    }

    /// Whether a space has something open not written yet (a • on its tab).
    fn unsaved(&self, space: Space) -> bool {
        match space {
            Space::Film => self.work.scripts.open.as_ref().is_some_and(|open| !open.saved()),
            Space::Decor => self.work.decors.open.as_ref().is_some_and(|open| !open.saved()),
            Space::Dungeon => self.work.dungeons.unsaved_count() > 0,
            Space::Assets => self.work.resources.unsaved_count() > 0,
            Space::Game => self.work.games.unsaved(),
            Space::Home | Space::Emulator => false,
        }
    }

    /// The Film's tools: the film open (• not written yet), and "Go to" what it names.
    fn film_tools(&mut self, ui: &mut egui::Ui) {
        let Some(path) = self.work.film.clone() else {
            ui.label(theme::weak("no film open"));
            return;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        ui.label(theme::weak(format!("{name}{}", if self.unsaved(Space::Film) { " •" } else { "" })));
        let text = self.work.scripts.open.as_ref().filter(|open| open.path == path).map(|open| open.text.clone()).unwrap_or_default();
        let named = named_in_film(&self.work.session, &path, &text);
        if named.is_empty() {
            return;
        }
        ui.separator();
        ui.menu_button("Go to ▾", |ui| {
            for (said, going) in &named {
                if ui.button(said).clicked() {
                    self.go_to(going.clone());
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text("the dungeons, fights and decors this film names, opened in their editor");
    }

    /// Another editor opened, on what one names.
    pub fn go_to(&mut self, going: Going) {
        match going {
            Going::Dungeon(word, fight) => {
                self.work.dungeons.open(if fight { crate::dungeons::Open::Fight(word) } else { crate::dungeons::Open::Dungeon(word) });
                self.space = Space::Dungeon;
            }
            Going::Decor(path) => {
                self.work.decors.open(&path);
                self.work.decor = Some(path);
                self.space = Space::Decor;
            }
        }
    }

    /// What the Game space asks shown: a film at a line, a decor with a line chosen, a dungeon or a fight.
    fn show_asked(&mut self, asked: crate::games::Asked) {
        match asked {
            crate::games::Asked::Film(path, line) => {
                match &mut self.work.scripts.open {
                    Some(open) if open.path == path && line > 0 => open.go_to_line(line),
                    _ if line > 0 => self.work.scripts.ask_line(line),
                    _ => {}
                }
                self.work.film = Some(path);
                self.space = Space::Film;
            }
            crate::games::Asked::Decor(path, line) => {
                self.go_to(Going::Decor(path));
                if line > 0
                    && let Some(open) = &mut self.work.decors.open
                {
                    open.choose(vec![(line, 0)]);
                }
            }
            crate::games::Asked::DungeonFile(path) => {
                self.work.dungeons.open(crate::dungeons::Open::File(path));
                self.space = Space::Dungeon;
            }
        }
    }

    /// A file restored by `safe` read again where it is open; one to offer to restore, asked.
    fn restore(&mut self, context: &egui::Context) {
        for path in workshop_base::safe::restored(&self.work.session) {
            if self.work.scripts.open.as_ref().is_some_and(|open| open.path == path) {
                self.work.scripts.open = None;
            }
            if self.work.decors.open.as_ref().is_some_and(|open| open.path == path) {
                self.work.decors.open = None;
                self.work.decors.open(&path);
            }
            self.work.dungeons.reload(&path);
            self.work.resources.reload(&path);
        }
        let Some((path, trouble)) = workshop_base::safe::offered(&self.work.session) else {
            return;
        };
        let mut answer: Option<Option<&str>> = None;
        egui::Window::new("Take a File Back?").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(context, |ui| {
            ui.set_width(460.0);
            ui.label(theme::text(path.display().to_string(), theme::SMALL, theme::STRONG));
            ui.add_space(4.0);
            let backup = workshop_base::safe::beside(&path, "bak").is_file();
            match trouble {
                workshop_base::safe::Trouble::Interrupted => {
                    ui.label(theme::weak("Its last save did not finish: what was to be written was left beside it (.tmp)."));
                    ui.add_space(6.0);
                    if ui.button("Take Back the Interrupted Save (.tmp)").clicked() {
                        answer = Some(Some("tmp"));
                    }
                    if backup && ui.button("Go Back to the Version Before (.bak)").clicked() {
                        answer = Some(Some("bak"));
                    }
                }
                workshop_base::safe::Trouble::NewerBackup => {
                    ui.label(theme::weak("Its backup (.bak) is newer than it: it may have been replaced by an older version."));
                    ui.add_space(6.0);
                    if ui.button("Take Back the Backup (.bak)").clicked() {
                        answer = Some(Some("bak"));
                    }
                }
            }
            if ui.button("Keep the File as It Is").clicked() {
                answer = Some(None);
            }
            ui.label(theme::text("what it was before being taken back stays beside it (.before)", theme::SMALL, theme::FAINT));
        });
        if let Some(from) = answer {
            self.work.said = match workshop_base::safe::answer(&self.work.session, &path, from) {
                Ok(said) | Err(said) => said,
            };
        }
    }

    /// The Decor's tools: living or still, how close, written out or not, what is wrong.
    fn decor_tools(&mut self, ui: &mut egui::Ui) {
        let decors = &mut self.work.decors;
        let Some(open) = &decors.open else {
            ui.label(theme::weak("no decor open"));
            return;
        };
        // (What the mouse does: choose, paint terrains, strew.)
        use crate::terrains::Tool;
        for (tool, word, key, said) in [
            (Tool::Choose, "Choose", "V", "take, move the elements"),
            (Tool::Terrain, "Terrain", "B", "paint terrains with the tiles of a place of the game"),
            (Tool::Strew, "Scatter", "N", "scatter a piece under the brush"),
            (Tool::Walls, "Walls", "M", "paint where one walks, tile by tile"),
            (Tool::Walk, "Walk", "P", "walk a Pokémon around to try the walls"),
        ] {
            let chosen = decors.brushes.tool == tool;
            let text = RichText::new(word).color(if chosen { theme::GROUND } else { theme::TEXT });
            if ui.add(egui::Button::new(text).fill(if chosen { theme::DECOR } else { theme::FIELD })).on_hover_text(format!("{said} ({key})")).clicked() {
                decors.brushes.tool = tool;
            }
        }
        ui.separator();
        let living = decors.living;
        let word = if living { "⏸ Animated" } else { "▶ Animate" };
        let fill = if living { theme::DECOR } else { theme::FIELD };
        let colour = if living { theme::GROUND } else { theme::TEXT };
        let said_living =
            ui.add(egui::Button::new(RichText::new(word).color(colour)).fill(fill)).on_hover_text("the decor plays its animations, as in the game");
        ui.label(theme::weak(format!("{:.0} %", open.seen_scale * 100.0)));
        let fit = ui.button("Fit").on_hover_text("the whole decor in the scene").clicked();
        let grid = decors.grid;
        let grid_said = ui
            .add(egui::Button::new(RichText::new("Grid").color(if grid { theme::GROUND } else { theme::TEXT })).fill(if grid {
                theme::DECOR
            } else {
                theme::FIELD
            }))
            .on_hover_text("the decor's grid (G)");
        let toggle = |ui: &mut egui::Ui, word: &str, on: bool| {
            ui.add(egui::Button::new(RichText::new(word).color(if on { theme::GROUND } else { theme::TEXT })).fill(if on {
                theme::DECOR
            } else {
                theme::FIELD
            }))
        };
        let step = crate::scene::grid_step(&open.text);
        let snap_said = toggle(ui, "Snap", decors.snap)
            .on_hover_text(format!("what is laid or carried lands on the grid of {step:.0} pixels (S; Alt held: the other way for a moment)"));
        let walls_shown = !decors.is_hidden("walls");
        let walls_said = toggle(ui, "Walls", walls_shown).on_hover_text("where the characters cannot walk, in red over the decor (C)");
        ui.separator();
        // (The layer what is laid goes on: under the characters, as the game, or in front of them.)
        let mut layer = decors.active_layer;
        egui::ComboBox::from_id_salt("lays on")
            .selected_text(format!("Lays on: {}", crate::placing::layer_name(layer)))
            .show_ui(ui, |ui| {
                for (word, name, meaning) in crate::placing::PIECE_LAYERS {
                    ui.selectable_value(&mut layer, word, name).on_hover_text(meaning);
                }
            })
            .response
            .on_hover_text("the layer a piece laid from the Library goes on (L moves the pieces chosen to the next one)");
        decors.active_layer = layer;
        ui.separator();
        let wrong = open.said.iter().filter(|said| said.wrong).count();
        let remarks = open.said.len() - wrong;
        if wrong > 0 {
            ui.label(theme::text(format!("{wrong} problem(s)"), theme::SMALL, theme::WRONG)).on_hover_text(
                open.said.iter().filter(|said| said.wrong).map(|said| format!("line {}: {}", said.line, said.text)).collect::<Vec<_>>().join("\n"),
            );
        } else if remarks > 0 {
            ui.label(theme::weak(format!("{remarks} remark(s)")))
                .on_hover_text(open.said.iter().map(|said| format!("line {}: {}", said.line, said.text)).collect::<Vec<_>>().join("\n"));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let state = match (open.saved(), open.is_baked()) {
                (false, _) => "saving…",
                (true, false) => "saved · baking for the films…",
                (true, true) => "saved · ready for the films",
            };
            ui.label(theme::weak(state));
        });
        if said_living.clicked() {
            decors.living = !living;
        }
        if fit {
            crate::decors::fit(decors);
        }
        if grid_said.clicked() {
            decors.grid = !grid;
        }
        if snap_said.clicked() {
            decors.snap = !decors.snap;
        }
        if walls_said.clicked() {
            toggle_walls(decors);
        }
    }

    /// The decor open written out now.
    fn save(&mut self) {
        if let Some(open) = &mut self.work.decors.open {
            self.work.said = match open.save() {
                Ok(()) => "saved".to_string(),
                Err(why) => why,
            };
        }
    }

    /// The keys of the whole window: save, undo, make again.
    fn keys(&mut self, context: &egui::Context) {
        if self.space != Space::Decor || context.wants_keyboard_input() {
            return;
        }
        let (save, undo, redo) = context.input(|input| {
            let command = input.modifiers.command;
            let z = command && input.key_pressed(egui::Key::Z);
            (
                command && input.key_pressed(egui::Key::S),
                z && !input.modifiers.shift,
                (z && input.modifiers.shift) || (command && input.key_pressed(egui::Key::Y)),
            )
        });
        // (/: the Library's search, the slash not typed into it.)
        if context.input(|input| input.events.iter().any(|event| matches!(event, egui::Event::Text(said) if said == "/"))) {
            context.input_mut(|input| input.events.retain(|event| !matches!(event, egui::Event::Text(said) if said == "/")));
            self.work.decors.lib.focus = true;
        }
        // (S: snapping; C: the walls shown. Not while walking: S walks.)
        let walking = self.work.decors.brushes.tool == crate::terrains::Tool::Walk;
        if !walking && context.input(|input| !input.modifiers.any() && input.key_pressed(egui::Key::S)) {
            self.work.decors.snap = !self.work.decors.snap;
        }
        if context.input(|input| !input.modifiers.any() && input.key_pressed(egui::Key::C)) {
            toggle_walls(&mut self.work.decors);
        }
        if save {
            self.save();
        }
        // (V, B, N: the mouse chooses, paints terrains, strews.)
        use crate::terrains::Tool;
        let tool = context.input(|input| {
            if input.modifiers.any() {
                return None;
            }
            [(egui::Key::V, Tool::Choose), (egui::Key::B, Tool::Terrain), (egui::Key::N, Tool::Strew), (egui::Key::M, Tool::Walls), (egui::Key::P, Tool::Walk)]
                .into_iter()
                .find(|(key, _)| input.key_pressed(*key))
                .map(|(_, tool)| tool)
        });
        if let Some(tool) = tool {
            self.work.decors.brushes.tool = tool;
        }
        if context.input(|input| !input.modifiers.any() && input.key_pressed(egui::Key::G)) {
            self.work.decors.grid = !self.work.decors.grid;
        }
        if context.input(|input| !input.modifiers.any() && input.key_pressed(egui::Key::R)) {
            self.work.decors.rulers = !self.work.decors.rulers;
        }
        if let Some(open) = &mut self.work.decors.open {
            if undo {
                open.undo();
            }
            if redo {
                open.redo();
            }
        }
    }

    /// The bar of state: the file, the project, what was just said.
    fn state(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            let white = |said: String| RichText::new(said).size(theme::SMALL).color(theme::STRONG);
            let file = match self.space {
                Space::Film => self.work.film.clone(),
                Space::Decor => self.work.decor.clone(),
                _ => self.work.chosen.clone(),
            };
            let mark = if self.unsaved(self.space) && matches!(self.space, Space::Film | Space::Decor) { " •" } else { "" };
            ui.label(white(file.map_or("library".to_string(), |path| format!("{}{mark}", path.display()))));
            ui.label(white(format!("{} films · {} decors", self.work.project.films.len(), self.work.project.decors.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(white(self.work.said.clone()));
            });
        });
    }

    /// The window making a new film or decor: its title, then made.
    fn making(&mut self, context: &egui::Context) {
        let Some(making) = &mut self.work.making else {
            return;
        };
        let mut open = true;
        let mut made = false;
        let word = making.kind.word().to_lowercase();
        egui::Window::new(format!("New {word}")).open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(
            context,
            |ui| {
                ui.set_width(420.0);
                egui::Grid::new("making").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label(theme::weak("Title"));
                    let field = ui.add(egui::TextEdit::singleline(&mut making.title).hint_text("The Big Day").desired_width(f32::INFINITY));
                    field.request_focus();
                    ui.end_row();
                    ui.label(theme::weak("File"));
                    ui.label(theme::weak(file_of(&self.work.session, making).map_or("—".to_string(), |path| path.display().to_string())));
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                    let free = file_of(&self.work.session, making).is_some_and(|path| !path.exists());
                    if ui.add_enabled(free, egui::Button::new(RichText::new("Create").strong())).clicked() || (enter && free) {
                        made = true;
                    }
                    if !free && !making.title.trim().is_empty() {
                        ui.label(theme::text("this name is already taken", theme::SMALL, theme::WRONG));
                    }
                });
            },
        );
        if made {
            let making = self.work.making.take().expect("being made");
            match create(&self.work.session, &making) {
                Ok(path) => {
                    self.work.project = Project::read(&self.work.session);
                    if let Some(entry) = self.work.project.films.iter().chain(&self.work.project.decors).find(|entry| entry.path == path).cloned() {
                        self.work.open(&entry);
                    }
                }
                Err(why) => self.work.said = why,
            }
        } else if !open {
            self.work.making = None;
        }
    }
}

/// Where a "Go to" goes: a dungeon (or a boss fight) in its editor, a decor in the Decor space.
#[derive(Clone, Debug)]
pub enum Going {
    Dungeon(String, bool),
    Decor(PathBuf),
}

/// What a film names that has an editor: its dungeons and fights (`dungeon <word>`), its decors (`scene: <place>`);
/// a game's film, its game's own first, then the shared library's.
fn named_in_film(session: &Session, film: &Path, text: &str) -> Vec<(String, Going)> {
    // (`<project>/films/<film>.film`: its project two folders up.)
    let project = film.parent().and_then(Path::parent).map_or_else(|| session.library().clone(), |folder| session.library().at(folder));
    let dungeons = project.names("dungeons", "dungeon");
    let fights = project.names("fights", "fight");
    let mut decors = decor::names(session.library());
    decors.extend(project.names("decors", "decor"));
    let mut named: Vec<(String, Going)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let (said, going) = match words.as_slice() {
            ["dungeon", word, ..] if dungeons.iter().any(|known| known == word) => (format!("Dungeon “{word}”"), Going::Dungeon(word.to_string(), false)),
            ["dungeon", word, ..] if fights.iter().any(|known| known == word) => (format!("Boss Fight “{word}”"), Going::Dungeon(word.to_string(), true)),
            ["scene:", place, ..] if decors.iter().any(|known| known == place) => (format!("Decor “{place}”"), Going::Decor(project.decor(place))),
            _ => continue,
        };
        if seen.insert(said.clone()) {
            named.push((said, going));
        }
    }
    named
}

/// The file a new one will be: its title's words joined (`the_big_day`).
fn file_of(session: &Session, making: &Making) -> Option<PathBuf> {
    let words: Vec<String> = making
        .title
        .split_whitespace()
        .map(|word| word.to_lowercase().chars().filter(|letter| letter.is_alphanumeric()).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return None;
    }
    let name = words.join("_");
    Some(match making.kind {
        Kind::Film => session.library().films().join(format!("{name}.film")),
        Kind::Decor => session.library().decors().join(format!("{name}.decor")),
    })
}

/// A new film or decor written: where.
fn create(session: &Session, making: &Making) -> Result<PathBuf, String> {
    let path = file_of(session, making).ok_or("a title first")?;
    let title = making.title.trim();
    let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let text = match making.kind {
        Kind::Film => format!("title: {title}\n\nscene: beach partner\npartner: Hello!\n"),
        Kind::Decor => format!("decor: {name} 512 384\nname {title}\n"),
    };
    std::fs::write(&path, text).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(path)
}

/// The panels drawn, each by its space's module.
struct Panels<'a> {
    work: &'a mut Work,
}

impl egui_dock::TabViewer for Panels<'_> {
    type Tab = Panel;

    fn title(&mut self, tab: &mut Panel) -> egui::WidgetText {
        tab.title().into()
    }

    fn is_closeable(&self, _tab: &Panel) -> bool {
        false
    }

    fn clear_background(&self, tab: &Panel) -> bool {
        !matches!(tab, Panel::DecorScene)
    }

    /// The panels scroll themselves where they need to: none in a scroll
    /// of the dock's (a scene would take all the room there is).
    fn scroll_bars(&self, _tab: &Panel) -> [bool; 2] {
        [false, false]
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Panel) {
        match tab {
            Panel::Project => crate::home::project(ui, self.work),
            Panel::List => crate::home::list(ui, self.work),
            Panel::Preview => crate::home::preview(ui, self.work),
            Panel::DecorScene => crate::scene::scene(ui, &mut self.work.decors),
            Panel::Hierarchy => crate::panels::hierarchy(ui, &mut self.work.decors),
            Panel::History => crate::panels::history(ui, &mut self.work.decors),
            Panel::DecorInspector => crate::panels::inspector(ui, &mut self.work.decors),
            Panel::Library => {
                crate::library::library(ui, &mut self.work.decors, &self.work.project.decors);
                // (A decor found by the Library's search, opened.)
                if let Some(path) = self.work.decors.lib.asked_open.take() {
                    self.work.decors.open(&path);
                    self.work.decor = Some(path);
                }
            }
            Panel::Script => {
                let seen = crate::script::Seen {
                    line: self.work.films.playing_line(),
                    stepped: self.work.films.stepped_line(),
                    playing: self.work.films.playing,
                    timing: self.work.films.timing(),
                    end: self.work.films.timeline.as_ref().map(|_| self.work.films.end as f64 / 60.0),
                    numbers: self.work.films.numbers.clone(),
                };
                if let Some(line) = crate::script::script(ui, &mut self.work.scripts, self.work.film.clone().as_ref(), &seen) {
                    self.work.films.go_to_line(line);
                }
                // (F12 on a shared file, a recipe from one: that file open, at its line.)
                if let Some((path, line)) = self.work.scripts.open_asked.take() {
                    self.work.scripts.ask_line(line);
                    self.work.film = Some(path);
                }
                // (Ctrl+Enter, ▶ in the margin, "Play from Here": the film plays from that line.)
                if let Some(line) = self.work.scripts.play_from.take() {
                    self.work.films.go_to_line(line);
                    self.work.films.playing = true;
                }
            }
            Panel::FilmScene => crate::films::scene(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::Timeline => crate::films::timeline(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::FilmInspector => crate::films::inspector(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::FilmState => crate::films::state(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::AssetTree => crate::resources::tree(ui, &mut self.work.resources, &mut self.work.decors),
            Panel::AssetGrid => crate::resources::grid(ui, &mut self.work.resources, &mut self.work.decors),
            Panel::AssetInspector => {
                crate::resources::inspector(ui, &mut self.work.resources, &mut self.work.decors, &mut self.work.scripts);
                // (A film or a decor it is used in, opened in its space.)
                match self.work.resources.asked.take() {
                    Some(crate::resources::Asked::Film(path)) => (self.work.film, self.work.go) = (Some(path), Some(Space::Film)),
                    Some(crate::resources::Asked::Decor(path)) => {
                        self.work.decors.open(&path);
                        (self.work.decor, self.work.go) = (Some(path), Some(Space::Decor));
                    }
                    Some(crate::resources::Asked::Dungeon(what)) => {
                        self.work.dungeons.open(what);
                        self.work.go = Some(Space::Dungeon);
                    }
                    None => {}
                }
            }
        }
    }
}

/// The look of the docked panels: their tabs as a studio's.
fn dock_style(context: &egui::Context) -> Style {
    let mut style = Style::from_egui(context.style().as_ref());
    style.tab_bar.bg_fill = theme::TABS;
    style.tab_bar.height = 24.0;
    style.tab_bar.hline_color = theme::GAP;
    style.tab_bar.corner_radius = egui::CornerRadius::ZERO;
    style.tab.tab_body.bg_fill = theme::PANEL;
    style.tab.tab_body.stroke = egui::Stroke::NONE;
    style.tab.tab_body.corner_radius = egui::CornerRadius::ZERO;
    style.tab.tab_body.inner_margin = egui::Margin::same(6);
    style.tab.hline_below_active_tab_name = false;
    for (state, fill, text) in [
        (&mut style.tab.active, theme::PANEL, theme::STRONG),
        (&mut style.tab.focused, theme::PANEL, theme::STRONG),
        (&mut style.tab.active_with_kb_focus, theme::PANEL, theme::STRONG),
        (&mut style.tab.focused_with_kb_focus, theme::PANEL, theme::STRONG),
        (&mut style.tab.inactive, theme::TABS, theme::WEAK),
        (&mut style.tab.inactive_with_kb_focus, theme::TABS, theme::WEAK),
        (&mut style.tab.hovered, theme::HOVERED, theme::TEXT),
    ] {
        state.bg_fill = fill;
        state.text_color = text;
        state.outline_color = egui::Color32::TRANSPARENT;
        state.corner_radius = egui::CornerRadius::ZERO;
    }
    style.separator.width = 2.0;
    style.separator.color_idle = theme::GAP;
    style.separator.color_hovered = theme::CHOSEN;
    style.separator.color_dragged = theme::DECOR;
    style.main_surface_border_stroke = egui::Stroke::NONE;
    style.main_surface_border_rounding = egui::CornerRadius::ZERO;
    style.overlay.selection_color = theme::CHOSEN.gamma_multiply(0.5);
    style
}

impl App {
    /// Just after the first launch: the Game space, on the first game there is (none in a new
    /// workshop), and what to try first.
    pub fn first_steps(&mut self) {
        if let Some(game) = self.work.session.library().games().into_iter().next() {
            self.work.games.open_game(&game);
        }
        (self.space, self.welcome) = (Space::Game, true);
    }

    /// The "what to try first" panel, until closed.
    fn welcome(&mut self, context: &egui::Context) {
        if !self.welcome {
            return;
        }
        let mut open = true;
        egui::Window::new("What to Try First")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(context, |ui| {
                ui.label(theme::text("Your data is ready, and the workshop is yours: empty, waiting for your first game.", theme::BODY, theme::STRONG));
                ui.add_space(8.0);
                let step = |ui: &mut egui::Ui, number: &str, said: &str| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(theme::text(number, theme::BODY, theme::GAME));
                        ui.label(theme::text(said, theme::BODY, theme::TEXT));
                    });
                };
                step(ui, "1.", "Make a game: type a title under GAMES (on the left) and click + New Game.");
                step(ui, "2.", "Write its first film in the Film space: the scene plays each line as you type it.");
                step(ui, "3.", "Draw a place in the Decor space, and say who lives there in its life.");
                step(ui, "4.", "Play it from the Game space: arrows walk, Enter talks, M opens the menu.");
                ui.add_space(6.0);
                ui.label(theme::weak("Step by step, from an empty folder to a game sent to a friend: docs/TUTORIAL.md (“Your first game in 30 minutes”)."));
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    self.welcome = false;
                }
            });
        self.welcome &= open;
    }
}

impl eframe::App for App {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.work.scripts.tick(context);
        if let Some(path) = self.work.film.clone() {
            self.work.scripts.open(&path);
        }
        self.work.films.tick(context, &mut self.work.scripts);
        if let Some(told) = self.work.decors.tick(context) {
            self.work.said = told;
        }
        self.keys(context);
        // (The game open in the Game space: the Dungeon space lists and opens its own dungeons and fights too.)
        self.work.dungeons.game = self.work.games.shown();
        let bar = |fill| egui::Frame::new().fill(fill).inner_margin(egui::Margin::symmetric(6, 0));
        egui::TopBottomPanel::top("menus").exact_height(26.0).frame(bar(theme::MENU)).show(context, |ui| self.menus(ui));
        egui::TopBottomPanel::top("tools").exact_height(34.0).frame(bar(theme::TOOLS)).show(context, |ui| self.tools(ui));
        egui::TopBottomPanel::bottom("state").exact_height(20.0).frame(bar(theme::STATE)).show(context, |ui| self.state(ui));
        let style = dock_style(context);
        let at = self.space.at();
        // (Something being made: on the whole window, its tabs in the tools bar; the panels when it is put aside.)
        if self.space == Space::Dungeon {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(4, 4))).show(context, |ui| {
                crate::dungeons::page(ui, &mut self.work.dungeons);
            });
        } else if self.space == Space::Game {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL)).show(context, |ui| {
                crate::games::page(ui, &mut self.work.games);
            });
        } else if self.space == Space::Emulator {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(4, 4))).show(context, |ui| {
                crate::emulate::page(ui, &mut self.work.emulate);
            });
            if !self.work.emulate.said.is_empty() {
                self.work.said = std::mem::take(&mut self.work.emulate.said);
            }
        } else if self.space == Space::Assets && self.work.resources.creating() {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(4, 4))).show(context, |ui| {
                crate::resources::studio(ui, &mut self.work.resources);
            });
        } else {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::GAP)).show(context, |ui| {
                DockArea::new(&mut self.docks[at])
                    .id(egui::Id::new(("dock", at)))
                    .style(style)
                    .show_leaf_collapse_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show_close_buttons(false)
                    .show_inside(ui, &mut Panels { work: &mut self.work });
            });
        }
        self.making(context);
        self.welcome(context);
        self.restore(context);
        crate::quant_tool::show(context, &mut self.quant);
        if let Some(dex) = self.work.resources.ask_spritecollab.take() {
            self.spritecollab = Some(crate::spritecollab::Browser::new(&self.work.session, dex));
        }
        crate::spritecollab::show(context, &mut self.spritecollab);
        if let Some(browser) = &mut self.spritecollab
            && std::mem::take(&mut browser.fresh)
        {
            self.work.resources.forget();
        }
        // (A dungeon or a fight asked from the Resources: opened in the Dungeon space.)
        if let Some(crate::resources::Asked::Dungeon(what)) = self.work.resources.asked.take_if(|asked| matches!(asked, crate::resources::Asked::Dungeon(_))) {
            self.work.dungeons.open(what);
            self.space = Space::Dungeon;
        }
        if let Some(said) = self.work.dungeons.said.take() {
            self.work.said = said;
        }
        // (A film, a decor, a dungeon asked from the Game space: opened in its space.)
        if let Some(asked) = self.work.games.asked.take() {
            self.show_asked(asked);
        }
        if let Some(space) = self.work.go.take() {
            self.space = space;
        }
        if let Some(capture) = &mut self.capture {
            capture.take(context);
        }
    }
}

/// The walls shown over the decor, or hidden (C, the tools' Walls).
fn toggle_walls(decors: &mut crate::decors::Decors) {
    if !decors.hidden.remove("walls") {
        decors.hidden.insert("walls");
    }
}
