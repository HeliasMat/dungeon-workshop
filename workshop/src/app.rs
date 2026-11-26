//! The window: a bar of menus (and the spaces, on its right), a bar of
//! tools for the space looked at, its panels docked under them (dragged
//! by their tabs anywhere), a bar of state at the bottom.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, RichText};
use egui_dock::{DockArea, DockState, NodeIndex, Style};

pub use workshop_base::ask::Space;
use workshop_base::ask::{Ask, OpenDungeon};
use workshop_base::project::{Entry, Kind, Project};
use workshop_base::session::Session;
use workshop_base::theme;

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
    /// The Pokémon, faces, gestures, music, sounds, things, places, effects, to drag onto the Scene or the Script.
    FilmAssets,
    /// The Wiki, docked (else a window of its own).
    Wiki,
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
            Panel::FilmAssets => "Assets",
            Panel::Wiki => "Wiki",
        }
    }

    /// A space's panels, in the order View › Add Panel lists them.
    fn of(space: Space) -> &'static [Panel] {
        match space {
            Space::Home => &[Panel::Project, Panel::List, Panel::Preview],
            Space::Film => &[Panel::Script, Panel::FilmScene, Panel::FilmAssets, Panel::FilmInspector, Panel::FilmState, Panel::Timeline, Panel::Wiki],
            Space::Decor => &[Panel::Hierarchy, Panel::History, Panel::DecorScene, Panel::DecorInspector, Panel::Library],
            Space::Assets => &[Panel::AssetTree, Panel::AssetGrid, Panel::AssetInspector],
            Space::Dungeon | Space::Game | Space::Emulator => &[],
        }
    }

    /// Whether it can be closed: all but the heart of its space (the Script and the Scene, the list, the grid).
    fn closeable(self) -> bool {
        !matches!(self, Panel::Script | Panel::FilmScene | Panel::DecorScene | Panel::List | Panel::AssetGrid)
    }

    /// The panels it goes beside when it is opened again, the first there.
    fn beside(self) -> &'static [Panel] {
        match self {
            Panel::FilmAssets => &[Panel::FilmInspector, Panel::FilmState],
            Panel::FilmInspector | Panel::FilmState => &[Panel::FilmAssets, Panel::FilmState, Panel::FilmInspector],
            Panel::Wiki => &[Panel::FilmAssets, Panel::FilmInspector, Panel::Timeline],
            Panel::Timeline => &[Panel::FilmState],
            Panel::Hierarchy | Panel::History => &[Panel::History, Panel::Hierarchy],
            Panel::DecorInspector | Panel::Library => &[Panel::DecorInspector, Panel::Library],
            Panel::Project | Panel::Preview => &[Panel::Preview, Panel::Project],
            Panel::AssetTree | Panel::AssetInspector => &[Panel::AssetInspector, Panel::AssetTree],
            _ => &[],
        }
    }
}

/// A panel opened in a space's dock: beside one it goes with, else on the right of the whole.
fn opened(dock: &mut DockState<Panel>, panel: Panel) {
    if dock.find_tab(&panel).is_some() {
        return;
    }
    let near = panel.beside().iter().find_map(|other| dock.find_main_surface_tab(other));
    let surface = dock.main_surface_mut();
    match near {
        Some((node, _)) => {
            surface.set_focused_node(node);
            surface.push_to_focused_leaf(panel);
        }
        None if surface.is_empty() => surface.push_to_first_leaf(panel),
        None => {
            surface.split_right(NodeIndex::root(), 0.75, vec![panel]);
        }
    }
}

/// A panel closed in a space's dock.
fn closed(dock: &mut DockState<Panel>, panel: Panel) {
    if let Some(at) = dock.find_tab(&panel) {
        dock.remove_tab(at);
    }
}

/// The panels a dock shows, by their titles (as the settings keep them).
fn shown_panels(dock: &DockState<Panel>) -> Vec<&'static str> {
    dock.iter_all_tabs().map(|(_, panel)| panel.title()).collect()
}

/// A space's panels as they were left (the settings), laid out as its own are:
/// those closed taken out, those opened put in.
fn arranged(space: Space, settings: &workshop_base::settings::Settings) -> DockState<Panel> {
    let mut dock = layout(space);
    let Some(shown) = settings.panels(space_key(space)) else {
        return dock;
    };
    for panel in Panel::of(space) {
        let wanted = shown.iter().any(|title| title == panel.title());
        if !wanted && panel.closeable() {
            closed(&mut dock, *panel);
        } else if wanted {
            opened(&mut dock, *panel);
        }
    }
    dock
}

/// A space's word in the settings (`panels_film`).
fn space_key(space: Space) -> &'static str {
    match space {
        Space::Home => "home",
        Space::Game => "game",
        Space::Film => "film",
        Space::Decor => "decor",
        Space::Dungeon => "dungeon",
        Space::Assets => "resources",
        Space::Emulator => "emulator",
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
            let [scene, _] = surface.split_left(scene, 0.4, vec![Panel::Script]);
            // (On the right: the assets to drag onto the Scene or the Script; under them, the line's Inspector and the State.)
            let [_, assets] = surface.split_right(scene, 0.68, vec![Panel::FilmAssets]);
            surface.split_below(assets, 0.64, vec![Panel::FilmInspector, Panel::FilmState]);
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

/// All the panels work with: the project, what is chosen and open, what was
/// just said; what a panel asks of the window.
pub struct Work {
    /// What every space works with (the library, the data, what is read of them).
    pub session: Arc<Session>,
    pub project: Project,
    /// The Home's own (the file chosen there with it).
    pub home: workshop_home::Home,
    /// The film and the decor open.
    pub film: Option<PathBuf>,
    pub decor: Option<PathBuf>,
    pub said: String,
    /// The New Film and New Decor windows, while open.
    pub new_film: Option<workshop_home::new_film::ui::NewFilm>,
    pub new_decor: Option<workshop_home::new_decor::NewDecor>,
    /// The Decor space's own.
    pub decors: workshop_decor::Decors,
    /// The Film space's Script; the film played.
    pub scripts: workshop_script::Scripts,
    pub films: workshop_film::Films,
    /// The Resources space's own.
    pub resources: workshop_resources::Resources,
    /// The Dungeon space's own.
    pub dungeons: workshop_dungeon::dungeons::Dungeons,
    /// The Game space's own.
    pub games: workshop_game::Games,
    /// The Emulator space's own.
    pub emulate: workshop_emulator::Emulate,
}

/// The window.
pub struct App {
    space: Space,
    docks: [DockState<Panel>; 7],
    work: Work,
    capture: Option<crate::capture::Capture>,
    /// Tools › Convert an Image (Tilequant), while open.
    quant: Option<workshop_tools::tilequant::QuantTool>,
    /// Tools › SpriteCollab, while open.
    spritecollab: Option<workshop_tools::sprites::Browser>,
    /// The "what to try first" panel, shown once the first launch is done.
    welcome: bool,
    /// The user's settings; whether the game-making spaces (Game, Dungeon, Emulator) show.
    settings: workshop_base::settings::Settings,
    game_spaces: bool,
    /// The panels each space shows, as the settings last had them (written again when they change).
    remembered: [Vec<&'static str>; 7],
}

impl App {
    pub fn new(context: &egui::Context, session: Arc<Session>) -> App {
        theme::apply(context);
        let work = Work {
            project: Project::read(&session),
            home: workshop_home::Home::new(&session, context),
            film: None,
            decor: None,
            said: String::new(),
            new_film: None,
            new_decor: None,
            decors: workshop_decor::Decors::new(&session, context),
            scripts: workshop_script::Scripts::new(&session, context),
            films: workshop_film::Films::new(&session, context),
            resources: workshop_resources::Resources::new(&session, context),
            dungeons: workshop_dungeon::dungeons::Dungeons::new(&session),
            games: workshop_game::Games::new(&session),
            emulate: workshop_emulator::Emulate::new(&session),
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
        let settings = workshop_base::settings::Settings::read(launch.settings.clone());
        let game_spaces = settings.game_spaces(!work.session.library().games().is_empty());
        let docks = Space::ALL.map(|space| arranged(space, &settings));
        let remembered = std::array::from_fn(|at| shown_panels(&docks[at]));
        let mut app = App {
            remembered,
            space,
            settings,
            game_spaces,
            docks,
            work,
            capture: crate::capture::Capture::asked(&launch),
            quant: workshop_tools::tilequant::QuantTool::asked(context, &launch),
            spritecollab: None,
            welcome: launch.welcome,
        };
        if app.welcome {
            app.first_steps();
        }
        // (`WORKSHOP_NEW=film:3:beach:pikachu,piplup`, `decor:beach`: that window open, for a picture.)
        match launch.new.as_deref() {
            Some(said) if said.starts_with("film") => app.work.new_film = Some(workshop_home::new_film::ui::NewFilm::asked(&app.work.session, said)),
            Some(said) if said.starts_with("decor") => app.work.new_decor = Some(workshop_home::new_decor::NewDecor::asked(&app.work.session, said)),
            _ => {}
        }
        // (`WORKSHOP_SPRITECOLLAB=25`: that window open on that number, for a picture.)
        if let Some(dex) = &launch.spritecollab {
            app.spritecollab = Some(workshop_tools::sprites::Browser::new(&app.work.session, dex.parse().ok()));
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
            workshop_home::open(&app.work.session, &entry);
            app.answer();
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
            Some("terrain") => workshop_decor::terrains::Tool::Terrain,
            Some("scatter") => workshop_decor::terrains::Tool::Strew,
            Some("walls") => workshop_decor::terrains::Tool::Walls,
            Some("walk") => workshop_decor::terrains::Tool::Walk,
            _ => workshop_decor::terrains::Tool::Choose,
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
                    self.new_film();
                }
                if ui.button("New Decor…").clicked() {
                    self.new_decor();
                }
                ui.separator();
                ui.menu_button("Open Recent", |ui| {
                    let recent: Vec<Entry> = self.work.project.recent().into_iter().take(10).cloned().collect();
                    for entry in recent {
                        if ui.button(format!("{} — {}", entry.title, entry.kind.word())).clicked() {
                            workshop_home::open(&self.work.session, &entry);
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
                let panels = Panel::of(self.space);
                if !panels.is_empty() {
                    ui.menu_button("Add Panel", |ui| {
                        for panel in panels {
                            let dock = &mut self.docks[self.space.at()];
                            let mut shown = dock.find_tab(panel).is_some();
                            let said = if panel.closeable() { "" } else { " (always there)" };
                            if ui.add_enabled(panel.closeable(), egui::Checkbox::new(&mut shown, format!("{}{said}", panel.title()))).changed() {
                                if shown {
                                    opened(dock, *panel);
                                } else {
                                    closed(dock, *panel);
                                }
                            }
                        }
                    });
                }
                if ui.button(format!("Default Layout ({})", self.space.word())).clicked() {
                    self.docks[self.space.at()] = layout(self.space);
                }
                for space in self.spaces() {
                    if ui.button(format!("Go to: {}", space.word())).clicked() {
                        self.space = space;
                    }
                }
            });
            ui.menu_button("Settings", |ui| {
                let mut shown = self.game_spaces;
                if ui
                    .checkbox(&mut shown, "Show the Game-Making Spaces")
                    .on_hover_text("Game, Dungeon and Emulator: to make a game of your films (chapters, dungeons, the DS). Off: films and decors only.")
                    .changed()
                {
                    self.show_game_spaces(shown);
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui
                    .button("Convert an Image (Tilequant)…")
                    .on_hover_text("a picture to the console's rules: tiles of 8, 16 colours each; <name>_quant.png beside it")
                    .clicked()
                {
                    self.quant = Some(workshop_tools::tilequant::QuantTool::new());
                }
                if ui.button("SpriteCollab…").on_hover_text("the PMDCollab community's sprites and portraits: search, look, import into data/extra").clicked()
                {
                    self.spritecollab = Some(workshop_tools::sprites::Browser::new(&self.work.session, None));
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Your First Film in 5 Minutes…").clicked() {
                    self.space = Space::Home;
                    self.welcome = true;
                }
                ui.label(theme::weak("Step by step: docs/FIRST_FILM.md; a game: docs/TUTORIAL.md."));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                for space in self.spaces().into_iter().rev() {
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
                if new(ui, "+ New Film", theme::FILM).on_hover_text("a place, a cast, a start: it plays at once").clicked() {
                    self.new_film();
                }
                if new(ui, "+ New Decor", theme::DECOR).on_hover_text("from a place of the game, or an empty page").clicked() {
                    self.new_decor();
                }
                ui.separator();
                if ui.button("Your First Film in 5 Minutes").on_hover_text("the steps, from nothing to a film played and exported").clicked() {
                    self.welcome = true;
                }
                if self.game_spaces
                    && new(ui, "Games", theme::GAME).on_hover_text("the games of the project: their chapters, played, checked, exported").clicked()
                {
                    self.space = Space::Game;
                }
            }
            Space::Decor => self.decor_tools(ui),
            Space::Film => self.film_tools(ui),
            Space::Dungeon => workshop_dungeon::dungeons::tabs(ui, &mut self.work.dungeons),
            Space::Game => workshop_game::tools(ui, &mut self.work.games),
            Space::Emulator => workshop_emulator::tools(ui, &mut self.work.emulate, self.work.film.as_deref()),
            Space::Assets if self.work.resources.creating() => workshop_resources::making_tabs(ui, &mut self.work.resources),
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
                self.work.dungeons.open(if fight { OpenDungeon::Fight(word) } else { OpenDungeon::Dungeon(word) });
                self.space = Space::Dungeon;
            }
            Going::Decor(path) => {
                self.work.decors.open(&path);
                self.work.decor = Some(path);
                self.space = Space::Decor;
            }
        }
    }

    /// What the parts asked this frame (`workshop_base::ask`), done in the order asked: the one
    /// place where a part's request reaches another part or the window.
    fn answer(&mut self) {
        for ask in self.work.session.asks.take() {
            match ask {
                Ask::Go(space) => self.space = space,
                Ask::Film { path, line } => {
                    self.welcome = false;
                    match &mut self.work.scripts.open {
                        Some(open) if open.path == path && line > 0 => open.go_to_line(line),
                        _ if line > 0 => self.work.scripts.ask_line(line),
                        _ => {}
                    }
                    self.work.film = Some(path);
                    self.space = Space::Film;
                }
                Ask::Decor { path, line } => {
                    self.welcome = false;
                    self.go_to(Going::Decor(path));
                    if line > 0
                        && let Some(open) = &mut self.work.decors.open
                    {
                        open.choose(vec![(line, 0)]);
                    }
                }
                Ask::Dungeon(what) => {
                    self.work.dungeons.open(what);
                    self.space = Space::Dungeon;
                }
                Ask::PlayFrom(line) => {
                    self.work.films.go_to_line(line);
                    self.work.films.playing = true;
                }
                Ask::Play => {
                    self.work.films.go(0);
                    self.work.films.playing = true;
                }
                Ask::Moved { from, to } => {
                    if self.work.film.as_ref() == Some(&from) {
                        self.work.film.clone_from(&to);
                    }
                    if self.work.decor.as_ref() == Some(&from) {
                        if let Some(to) = &to {
                            self.work.decors.open(to);
                        }
                        self.work.decor = to;
                    }
                }
                Ask::Drawn => {
                    self.work.resources.show_drawn();
                    self.space = Space::Assets;
                }
                Ask::SpriteCollab(dex) => self.spritecollab = Some(workshop_tools::sprites::Browser::new(&self.work.session, dex)),
                Ask::Say(said) => self.work.said = said,
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
        use workshop_decor::terrains::Tool;
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
        let step = workshop_decor::placing::grid_step(&open.text);
        let snap_said = toggle(ui, "Snap", decors.snap)
            .on_hover_text(format!("what is laid or carried lands on the grid of {step:.0} pixels (S; Alt held: the other way for a moment)"));
        let walls_shown = !decors.is_hidden("walls");
        let walls_said = toggle(ui, "Walls", walls_shown).on_hover_text("where the characters cannot walk, in red over the decor (C)");
        ui.separator();
        // (The layer what is laid goes on: under the characters, as the game, or in front of them.)
        let mut layer = decors.active_layer;
        egui::ComboBox::from_id_salt("lays on")
            .selected_text(format!("Lays on: {}", workshop_decor::placing::layer_name(layer)))
            .show_ui(ui, |ui| {
                for (word, name, meaning) in workshop_decor::placing::PIECE_LAYERS {
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
            workshop_decor::fit(decors);
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
        let walking = self.work.decors.brushes.tool == workshop_decor::terrains::Tool::Walk;
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
        use workshop_decor::terrains::Tool;
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
                _ => self.work.home.chosen.clone(),
            };
            let mark = if self.unsaved(self.space) && matches!(self.space, Space::Film | Space::Decor) { " •" } else { "" };
            ui.label(white(file.map_or("library".to_string(), |path| format!("{}{mark}", path.display()))));
            ui.label(white(format!("{} films · {} decors", self.work.project.films.len(), self.work.project.decors.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(white(self.work.said.clone()));
            });
        });
    }

    /// The panels of the space looked at, written in the settings when one was
    /// opened or closed (they open so the next time); the Wiki docked, shown there.
    fn remember_panels(&mut self) {
        let at = self.space.at();
        let shown = shown_panels(&self.docks[at]);
        if shown != self.remembered[at] {
            if let Err(why) = self.settings.set_panels(space_key(self.space), &shown) {
                self.work.said = why;
            }
            self.remembered[at] = shown;
        }
    }

    /// The Wiki docked in the Film space: the Script opens it there (F1, its button), not in a window.
    fn wiki_docked(&mut self) {
        let dock = &mut self.docks[Space::Film.at()];
        let docked = dock.find_tab(&Panel::Wiki);
        self.work.scripts.wiki_docked = docked.is_some();
        if let Some(at) = docked
            && self.work.scripts.wiki_asked()
        {
            dock.set_active_tab(at);
        }
    }

    /// The New Film window opened (on its first step).
    fn new_film(&mut self) {
        self.work.new_decor = None;
        self.welcome = false;
        self.work.new_film = Some(workshop_home::new_film::ui::NewFilm::new(&self.work.session));
    }

    /// The New Decor window opened.
    fn new_decor(&mut self) {
        self.work.new_film = None;
        self.welcome = false;
        self.work.new_decor = Some(workshop_home::new_decor::NewDecor::new(&self.work.session));
    }

    /// The spaces shown in the bar: the game-making ones only if the settings
    /// say so (the one looked at always).
    fn spaces(&self) -> Vec<Space> {
        Space::ALL
            .into_iter()
            .filter(|space| self.game_spaces || *space == self.space || !matches!(space, Space::Game | Space::Dungeon | Space::Emulator))
            .collect()
    }

    /// The game-making spaces shown or hidden, the settings written.
    fn show_game_spaces(&mut self, on: bool) {
        self.game_spaces = on;
        if let Err(why) = self.settings.set_game_spaces(on) {
            self.work.said = why;
        }
        if !on && matches!(self.space, Space::Game | Space::Dungeon | Space::Emulator) {
            self.space = Space::Home;
        }
    }

    /// The New Film and New Decor windows, while open; what they made, opened in its space.
    fn making(&mut self, context: &egui::Context) {
        let pictures = self.work.home.pictures();
        let done = match (&mut self.work.new_film, &mut self.work.new_decor) {
            (Some(new), _) => new.ui(context, pictures),
            (None, Some(new)) => new.ui(context, pictures),
            (None, None) => return,
        };
        use workshop_home::new_film::ui::Done;
        match done {
            Done::Open => {}
            Done::Closed => (self.work.new_film, self.work.new_decor) = (None, None),
            Done::Made(path) => {
                (self.work.new_film, self.work.new_decor) = (None, None);
                self.work.project = Project::read(&self.work.session);
                if let Some(entry) = self.work.project.films.iter().chain(&self.work.project.decors).find(|entry| entry.path == path).cloned() {
                    workshop_home::open(&self.work.session, &entry);
                }
            }
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

/// The panels drawn, each by its space's module.
struct Panels<'a> {
    work: &'a mut Work,
}

impl egui_dock::TabViewer for Panels<'_> {
    type Tab = Panel;

    fn title(&mut self, tab: &mut Panel) -> egui::WidgetText {
        tab.title().into()
    }

    fn is_closeable(&self, tab: &Panel) -> bool {
        tab.closeable()
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
            Panel::Project => workshop_home::project(ui, &mut self.work.home, &mut self.work.project),
            Panel::List => workshop_home::list(ui, &mut self.work.home, &mut self.work.project),
            Panel::Preview => workshop_home::preview(ui, &mut self.work.home, &mut self.work.project),
            Panel::DecorScene => workshop_decor::scene::scene(ui, &mut self.work.decors),
            Panel::Hierarchy => workshop_decor::panels::hierarchy(ui, &mut self.work.decors),
            Panel::History => workshop_decor::panels::history(ui, &mut self.work.decors),
            Panel::DecorInspector => workshop_decor::panels::inspector(ui, &mut self.work.decors),
            Panel::Library => {
                workshop_decor::library::library(ui, &mut self.work.decors, &self.work.project.decors);
            }
            Panel::Script => {
                let seen = workshop_script::Seen {
                    line: self.work.films.playing_line(),
                    stepped: self.work.films.stepped_line(),
                    playing: self.work.films.playing,
                    timing: self.work.films.timing(),
                    end: self.work.films.timeline.as_ref().map(|_| self.work.films.end as f64 / 60.0),
                    numbers: self.work.films.numbers.clone(),
                    chosen: self.work.films.chosen().map(str::to_string),
                };
                if let Some(line) = workshop_script::script(ui, &mut self.work.scripts, self.work.film.clone().as_ref(), &seen) {
                    self.work.films.go_to_line(line);
                }
            }
            Panel::FilmScene => workshop_film::scene(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::Timeline => workshop_film::timeline(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::FilmInspector => workshop_film::inspector(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::FilmState => workshop_film::state(ui, &mut self.work.films, &mut self.work.scripts),
            Panel::FilmAssets => {
                let chosen = self.work.films.chosen().map(str::to_string);
                let cast = self.work.films.cast.clone();
                workshop_resources::picker(ui, &mut self.work.resources, &self.work.decors, &mut self.work.scripts, chosen.as_deref(), &cast);
            }
            Panel::Wiki => workshop_script::wiki(ui, &mut self.work.scripts),
            Panel::AssetTree => workshop_resources::tree(ui, &mut self.work.resources, &mut self.work.decors),
            Panel::AssetGrid => workshop_resources::grid(ui, &mut self.work.resources, &mut self.work.decors),
            Panel::AssetInspector => {
                workshop_resources::inspector(ui, &mut self.work.resources, &mut self.work.decors, &mut self.work.scripts);
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
    /// Just after the first launch: the Home, and what to try first.
    pub fn first_steps(&mut self) {
        (self.space, self.welcome) = (Space::Home, true);
    }

    /// The welcome panel, until closed: make a film or a decor, open one of
    /// the last, the first film's steps.
    fn welcome(&mut self, context: &egui::Context) {
        if !self.welcome || self.work.new_film.is_some() || self.work.new_decor.is_some() {
            return;
        }
        let mut open = true;
        let (mut film, mut decor) = (false, false);
        egui::Window::new(RichText::new("Welcome to the Workshop").strong())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(600.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(context, |ui| {
                ui.set_width(600.0);
                ui.label(theme::text("Make films with Pokémon: a place, a cast, a few lines. Each line plays as you write it.", theme::BODY, theme::STRONG));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let big = |ui: &mut egui::Ui, word: &str, colour: egui::Color32| {
                        ui.add(
                            egui::Button::new(RichText::new(word).size(theme::TITLE).strong().color(theme::GROUND))
                                .fill(colour)
                                .min_size(egui::vec2(180.0, 40.0)),
                        )
                    };
                    film = big(ui, "+ New Film", theme::FILM).on_hover_text("a place, a cast, a start: it plays at once").clicked();
                    decor = big(ui, "+ New Decor", theme::DECOR).on_hover_text("a place of your own, from one of the game's or an empty page").clicked();
                });
                ui.add_space(12.0);
                ui.label(theme::text("YOUR FIRST FILM IN 5 MINUTES", theme::SMALL, theme::FILM));
                let step = |ui: &mut egui::Ui, number: &str, said: &str| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(theme::text(number, theme::BODY, theme::FILM));
                        ui.label(theme::text(said, theme::BODY, theme::TEXT));
                    });
                };
                step(ui, "1.", "New Film: choose a place, two Pokémon or more, a start, a title. It opens playing.");
                step(ui, "2.", "Write two lines in the Script: `pikachu (happy): Hello!` — Ctrl+Space offers what comes next.");
                step(ui, "3.", "Ctrl+Enter plays from the cursor's line; Space plays or pauses.");
                step(ui, "4.", "Drag someone on the Scene to move them: their line is written again.");
                step(ui, "5.", "A video of it: `scene_player video <film>` in a terminal (ffmpeg needed).");
                ui.label(theme::weak("All of it, with pictures: docs/FIRST_FILM.md."));
                ui.add_space(10.0);
                let room = ui.available_width();
                ui.allocate_ui(egui::vec2(room, 0.0), |ui| {
                    ui.label(theme::text("OPEN AGAIN", theme::SMALL, theme::WEAK));
                    if workshop_home::recent_row(ui, &mut self.work.home, &self.work.project, 4) == 0 {
                        ui.label(theme::weak("your films and decors will be here"));
                    }
                });
                ui.add_space(6.0);
                if !self.game_spaces {
                    ui.label(theme::text(
                        "Making a whole game (chapters, dungeons, the DS)? Settings › Show the Game-Making Spaces.",
                        theme::SMALL,
                        theme::FAINT,
                    ));
                }
            });
        self.welcome &= open;
        if film {
            self.new_film();
        }
        if decor {
            self.new_decor();
        }
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
        self.wiki_docked();
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
                workshop_dungeon::dungeons::page(ui, &mut self.work.dungeons);
            });
        } else if self.space == Space::Game {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL)).show(context, |ui| {
                workshop_game::page(ui, &mut self.work.games);
            });
        } else if self.space == Space::Emulator {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(4, 4))).show(context, |ui| {
                workshop_emulator::page(ui, &mut self.work.emulate);
            });
            if !self.work.emulate.said.is_empty() {
                self.work.said = std::mem::take(&mut self.work.emulate.said);
            }
        } else if self.space == Space::Assets && self.work.resources.creating() {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(4, 4))).show(context, |ui| {
                workshop_resources::studio(ui, &mut self.work.resources);
            });
        } else {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::GAP)).show(context, |ui| {
                DockArea::new(&mut self.docks[at])
                    .id(egui::Id::new(("dock", at)))
                    .style(style)
                    .show_leaf_collapse_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show_close_buttons(true)
                    .show_inside(ui, &mut Panels { work: &mut self.work });
            });
        }
        self.remember_panels();
        self.making(context);
        self.welcome(context);
        self.restore(context);
        workshop_tools::tilequant::show(context, &mut self.quant);
        self.answer();
        workshop_tools::sprites::show(context, &mut self.spritecollab);
        if let Some(browser) = &mut self.spritecollab
            && std::mem::take(&mut browser.fresh)
        {
            self.work.resources.forget();
        }
        if let Some(capture) = &mut self.capture {
            capture.take(context);
        }
    }
}

/// The walls shown over the decor, or hidden (C, the tools' Walls).
fn toggle_walls(decors: &mut workshop_decor::Decors) {
    if !decors.hidden.remove("walls") {
        decors.hidden.insert("walls");
    }
}
