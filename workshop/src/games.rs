//! The Game space: a game of one's own (`library/<name>.game`), on the
//! whole window in three columns.
//!
//! - **On the left**, the games of the project (one picked, or a new one
//!   made), the card of the one open (its title, author, team, home: written
//!   back into its file at once, its comments and order kept) and its file,
//!   to write its chapters by hand (Save, Ctrl+S).
//! - **In the middle**, its chapters, a box each: what it plays, the places
//!   it goes to, the dungeons and fights it sends the team to (a click opens
//!   one in its space), its rules, the chapter it leads to.
//! - **On the right**, "Play from Chapter n" (`scene_player play <game>
//!   --chapter n`, in a window of its own), "Export the Game…" (a `.dwpack`,
//!   `game::pack`), and what is wrong with the whole game
//!   (`game::project_check`, read again after each save): a click shows
//!   where.

use std::sync::Arc;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use workshop_base::session::Session;

use ::game::game_file::GameFile;
use ::game::project_check::{Report, file_of};
use eframe::egui::{self, Color32, RichText, Sense};
use egui::text::{CCursor, CCursorRange};
use engine::Diagnostic;

use workshop_base::game::{self, ChapterBox, Link};
use workshop_base::theme;

/// What the Game space asks another space to show: a film (at a line), a
/// decor (that line chosen), a dungeon or a fight.
pub enum Asked {
    Film(PathBuf, usize),
    Decor(PathBuf, usize),
    /// A dungeon's or a fight's file (the game's own, or the shared library's).
    DungeonFile(PathBuf),
}

/// A game open: its file, its text (as typed), as written, as read, its chapters.
struct Open {
    path: PathBuf,
    text: String,
    written: String,
    game: GameFile,
    boxes: Vec<ChapterBox>,
}

impl Open {
    fn read(path: &Path) -> Result<Open, String> {
        let text = std::fs::read_to_string(path).map_err(|why| format!("{}: {why}", path.display()))?.replace("\r\n", "\n");
        let mut open = Open { path: path.to_path_buf(), written: text.clone(), text, game: GameFile::default(), boxes: Vec::new() };
        open.reread();
        Ok(open)
    }

    /// Its text read again as a game.
    fn reread(&mut self) {
        let mut game = GameFile::read(&self.text);
        game.name = self.path.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        game.folder = self.path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        self.boxes = game::chapter_boxes(&game);
        self.game = game;
    }

    fn saved(&self) -> bool {
        self.text == self.written
    }

    /// Its text written in its file.
    fn save(&mut self) -> Result<(), String> {
        workshop_base::safe::write(&self.path, &self.text).map_err(|why| format!("{}: {why}", self.path.display()))?;
        self.written = self.text.clone();
        self.reread();
        Ok(())
    }
}

/// The Game space as the window keeps it.
pub struct Games {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    open: Option<Open>,
    new_title: String,
    /// The check of the whole game: running, then what it found.
    checking: Option<Receiver<Result<Report, String>>>,
    report: Option<Result<Report, String>>,
    /// The chapter "Play" starts from; a line of the file to show.
    from_chapter: u16,
    jump: Option<usize>,
    /// The game being packed.
    packing: Option<workshop_base::tool_run::Run>,
    pub asked: Option<Asked>,
    pub said: Option<String>,
}

impl Games {
    pub fn new(session: &Arc<Session>) -> Games {
        let mut games = Games {
            session: session.clone(),
            open: None,
            new_title: String::new(),
            checking: None,
            report: None,
            from_chapter: 1,
            jump: None,
            packing: None,
            asked: None,
            said: None,
        };
        // (`WORKSHOP_GAME=my_game`: that game open, for a picture.)
        if let Some(name) = session.launch.game.clone() {
            let library = session.library();
            games.open_game(&library.find_game(&name).unwrap_or_else(|| game::folder(library).join(format!("{}.game", name.trim_end_matches(".game")))));
        }
        games
    }

    /// The game open: its folder and its title (its file's name if it has none).
    pub fn shown(&self) -> Option<(PathBuf, String)> {
        let open = self.open.as_ref()?;
        let title = if open.game.title.trim().is_empty() { open.game.name.clone() } else { open.game.title.clone() };
        Some((open.game.folder.clone(), title))
    }

    /// Whether the game open has changes not written yet.
    pub fn unsaved(&self) -> bool {
        self.open.as_ref().is_some_and(|open| !open.saved())
    }

    /// The game of that file open, checked.
    pub fn open_game(&mut self, path: &Path) {
        match Open::read(path) {
            Ok(open) => {
                self.from_chapter = open.game.first().map_or(1, |chapter| chapter.number);
                self.open = Some(open);
                self.check();
            }
            Err(why) => self.said = Some(why),
        }
    }

    /// The whole game checked again, on a thread of its own.
    fn check(&mut self) {
        let Some(open) = &self.open else {
            return;
        };
        let (path, context) = (open.path.clone(), self.session.context.clone());
        let (tell, answer) = channel();
        std::thread::spawn(move || _ = tell.send(::game::project_check::check(&context, &path).map_err(String::from)));
        self.checking = Some(answer);
    }

    /// The text open written; checked again.
    fn save(&mut self) {
        let Some(open) = &mut self.open else {
            return;
        };
        self.said = Some(match open.save() {
            Ok(()) => format!("saved: {}", open.path.display()),
            Err(why) => why,
        });
        self.check();
    }

    /// The text open made that (a change of its card): written at once.
    fn change(&mut self, text: String) {
        if let Some(open) = &mut self.open {
            open.text = text;
            self.save();
        }
    }
}

/// The tools bar: the game open (• not written yet), Save.
pub fn tools(ui: &mut egui::Ui, games: &mut Games) {
    let Some(open) = &games.open else {
        ui.label(theme::weak("no game open: pick one on the left, or make one"));
        return;
    };
    let name = open.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    ui.label(theme::weak(format!("{name}{}", if open.saved() { "" } else { " •" })));
    if ui.add_enabled(!open.saved(), egui::Button::new("Save")).on_hover_text("Ctrl+S").clicked() {
        games.save();
    }
}

/// The whole space.
pub fn page(ui: &mut egui::Ui, games: &mut Games) {
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) && games.unsaved() {
        games.save();
    }
    if let Some(answer) = &games.checking
        && let Ok(report) = answer.try_recv()
    {
        (games.report, games.checking) = (Some(report), None);
    }
    if games.checking.is_some() || games.packing.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    egui::SidePanel::left("game left").resizable(true).default_width(380.0).frame(egui::Frame::new().inner_margin(8)).show_inside(ui, |ui| left(ui, games));
    egui::SidePanel::right("game right").resizable(true).default_width(380.0).frame(egui::Frame::new().inner_margin(8)).show_inside(ui, |ui| right(ui, games));
    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::GROUND).inner_margin(8)).show_inside(ui, |ui| chapters(ui, games));
}

/// A title of a part.
fn part(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(theme::text(title, theme::SMALL, theme::WEAK));
    ui.add_space(2.0);
}

/// A label on the left, what changes it on the right.
fn row<R>(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 20.0), Sense::hover());
        ui.painter().text(rect.left_center(), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(theme::BODY), theme::WEAK);
        add(ui)
    })
    .inner
}

/// The games, the card of the one open, its file.
fn left(ui: &mut egui::Ui, games: &mut Games) {
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        part(ui, "GAMES");
        let here = games.open.as_ref().map(|open| open.path.clone());
        let mut picked = None;
        for path in game::games(games.session.library()) {
            let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            let title = std::fs::read_to_string(&path).ok().and_then(|text| game::head(&text, "title")).unwrap_or_default();
            if ui.selectable_label(here.as_ref() == Some(&path), format!("{title}  ·  {name}.game")).clicked() {
                picked = Some(path);
            }
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut games.new_title).hint_text("The Big Quest").desired_width(180.0));
            // (A folder of its own, games/<word>/: its .game, films/, decors/, dungeons/…)
            let file = game::file_of(games.session.library(), &games.new_title);
            let free = file.as_ref().and_then(|path| path.parent()).is_some_and(|folder| !folder.exists());
            let told = file.map_or(String::new(), |path| format!("{} (and its films/, decors/, dungeons/… beside it)", path.display()));
            if ui.add_enabled(free, egui::Button::new("+ New Game")).on_hover_text(told).clicked() {
                match game::create(games.session.library(), &games.new_title) {
                    Ok(path) => (picked, games.new_title) = (Some(path), String::new()),
                    Err(why) => games.said = Some(why),
                }
            }
        });
        if let Some(path) = picked {
            games.open_game(&path);
        }
        let Some(open) = &games.open else {
            return;
        };
        part(ui, "CARD");
        let text = open.text.clone();
        let mut made: Option<String> = None;
        let field = |ui: &mut egui::Ui, key: &str, hint: &str| {
            let now = game::head(&text, key).unwrap_or_default();
            workshop_decor::panels::typed(ui, egui::Id::new(("game card", key)), &now, 1, hint).map(|value| game::with_head(&text, key, &value))
        };
        if let Some(changed) = row(ui, "Title", |ui| field(ui, "title", "its name on the title screen")) {
            made = Some(changed);
        }
        if let Some(changed) = row(ui, "Author", |ui| field(ui, "author", "who made it")) {
            made = Some(changed);
        }
        // (The team: the hero, the partner, by their names.)
        let team = game::head(&text, "team").unwrap_or_default();
        let mut members: Vec<String> = team.split_whitespace().map(str::to_string).collect();
        members.resize(2, String::new());
        row(ui, "Team", |ui| {
            for (at, hint) in [(0, "hero"), (1, "partner")] {
                ui.scope(|ui| {
                    ui.set_width(110.0);
                    if let Some(name) = workshop_decor::panels::typed(ui, egui::Id::new(("game team", at)), &members[at], 1, hint) {
                        members[at] = name.trim().to_lowercase();
                        made = Some(game::with_head(&text, "team", members.join(" ").trim()));
                    }
                });
            }
        });
        // (Home: where the team comes back to, K.O. in a dungeon: a place, an entrance.)
        let home = game::head(&text, "home").unwrap_or_default();
        let (place, entrance) =
            home.split_once(' ').map_or((home.clone(), String::new()), |(place, entrance)| (place.to_string(), entrance.trim().to_string()));
        row(ui, "Home", |ui| {
            let mut asked: Option<String> = None;
            egui::ComboBox::from_id_salt("game home").selected_text(if place.is_empty() { "none" } else { &place }).width(150.0).show_ui(ui, |ui| {
                if ui.selectable_label(place.is_empty(), "none").clicked() {
                    asked = Some(String::new());
                }
                // (The game's own places and the library's, by kind of place, each by its name.)
                let project = games.session.library().at(&open.game.folder);
                let kinds = workshop_base::project::place_kinds(games.session.library());
                let decors: Vec<(String, String, decor::tags::PlaceKind)> = project
                    .names("decors", "decor")
                    .into_iter()
                    .map(|decor| {
                        let text = std::fs::read_to_string(project.decor(&decor)).unwrap_or_default();
                        let kind = workshop_decor::decor_kind(&kinds, &decor, &text);
                        (workshop_decor::decor_title(&decor, &text), decor, kind)
                    })
                    .collect();
                for (kind, group) in decor::tags::grouped(decors, |decor| decor.2) {
                    ui.label(theme::text(kind.shown().to_uppercase(), theme::SMALL, theme::WEAK));
                    for (title, decor, _) in group {
                        if ui.selectable_label(decor == place, format!("{title}  ·  {decor}")).clicked() {
                            asked = Some(decor);
                        }
                    }
                }
            });
            if !place.is_empty() {
                let there = std::fs::read_to_string(games.session.library().at(&open.game.folder).decor(&place)).unwrap_or_default();
                let named: Vec<String> = decor::doc::entrances(&there).into_iter().map(|(_, name)| name).filter(|name| !name.is_empty()).collect();
                egui::ComboBox::from_id_salt("game home entrance").selected_text(if entrance.is_empty() { "its first" } else { &entrance }).show_ui(ui, |ui| {
                    if ui.selectable_label(entrance.is_empty(), "its first").clicked() {
                        asked = Some(place.clone());
                    }
                    for name in &named {
                        if ui.selectable_label(*name == entrance, name).clicked() {
                            asked = Some(format!("{place} {name}"));
                        }
                    }
                });
            }
            if let Some(asked) = asked {
                made = Some(game::with_head(&text, "home", &asked));
            }
        });
        ui.label(theme::text("the rules are each chapter's “on … end” (in the middle); a dungeon's own rules, in its editor", theme::SMALL, theme::FAINT));
        if let Some(made) = made {
            games.change(made);
        }
        part(ui, "FILE");
        file(ui, games);
    });
}

/// The game's file, to write by hand; a line asked shown (the cursor on it).
fn file(ui: &mut egui::Ui, games: &mut Games) {
    let Some(open) = &mut games.open else {
        return;
    };
    let id = egui::Id::new("game file");
    let shown = egui::TextEdit::multiline(&mut open.text).id(id).code_editor().desired_width(f32::INFINITY).desired_rows(18).show(ui);
    if shown.response.changed() {
        open.reread();
    }
    if let Some(line) = games.jump.take() {
        let at: usize = open.text.lines().take(line.saturating_sub(1)).map(|said| said.chars().count() + 1).sum();
        let mut state = shown.state.clone();
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(at))));
        state.store(ui.ctx(), id);
        ui.memory_mut(|memory| memory.request_focus(id));
        let rect = shown.galley.pos_from_cursor(CCursor::new(at)).translate(shown.galley_pos.to_vec2());
        ui.scroll_to_rect(rect.expand(40.0), Some(egui::Align::Center));
    }
    if !open.saved() {
        let save = ui
            .horizontal(|ui| {
                let save = ui.button("Save").on_hover_text("Ctrl+S").clicked();
                ui.label(theme::text("not written yet", theme::SMALL, theme::WEAK));
                save
            })
            .inner;
        if save {
            games.save();
        }
    }
}

/// The colour of what a link is.
fn link_colour(link: &Link) -> Color32 {
    match link {
        Link::Film(_) => theme::FILM,
        Link::Place(_) | Link::Opened(_) => theme::DECOR,
        Link::Dungeon(_) | Link::Fight(_) => theme::DUNGEON,
    }
}

/// Where a link's file is, if it has one; what opening it asks.
fn link_asked(session: &Session, game: &GameFile, link: &Link) -> Option<Asked> {
    // (Each looked for in the game's folder first, then in the shared library: as the game reads it.)
    let project = session.library().at(&game.folder);
    let decor = |name: &str| project.decor(name);
    let dungeon = |name: &str| Some(project.dungeon(name)).filter(|path| path.is_file()).map(Asked::DungeonFile);
    match link {
        Link::Film(name) => Some(project.film(name)).filter(|path| path.is_file()).map(|path| Asked::Film(path, 0)),
        Link::Place(name) => Some(decor(name)).filter(|path| path.is_file()).map(|path| Asked::Decor(path, 0)),
        Link::Dungeon(name) => dungeon(name),
        Link::Fight(name) => Some(project.fight(name)).filter(|path| path.is_file()).map(Asked::DungeonFile),
        Link::Opened(name) if decor(name).is_file() => Some(Asked::Decor(decor(name), 0)),
        Link::Opened(name) => dungeon(name),
    }
}

/// The chapters, a box each, one under the other.
fn chapters(ui: &mut egui::Ui, games: &mut Games) {
    let Some(open) = &games.open else {
        ui.centered_and_justified(|ui| ui.label(theme::weak("Pick a game on the left, or make one.")));
        return;
    };
    let mut asked: Option<Asked> = None;
    let mut chosen: Option<u16> = None;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.label(theme::text(if open.game.title.is_empty() { &open.game.name } else { &open.game.title }, theme::TITLE, theme::STRONG).strong());
        if let Some((place, entrance)) = &open.game.home {
            ui.label(theme::weak(format!("home: {place}{}", entrance.as_ref().map_or(String::new(), |entrance| format!(" by {entrance}")))));
        }
        ui.add_space(8.0);
        if open.boxes.is_empty() {
            ui.label(theme::weak("no chapter yet: write “chapter 1 \"Its Name\"” … “end” in the file"));
        }
        for (at, chapter) in open.boxes.iter().enumerate() {
            let here = chapter.number == games.from_chapter;
            egui::Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(if here { 2.0_f32 } else { 1.0_f32 }, if here { theme::GAME } else { theme::GAP }))
                .corner_radius(4)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let title = format!("Chapter {}{}", chapter.number, if chapter.title.is_empty() { String::new() } else { format!(" · {}", chapter.title) });
                    if ui
                        .add(egui::Label::new(RichText::new(title).size(theme::TITLE).color(theme::GAME).strong()).sense(Sense::click()))
                        .on_hover_text("play from this chapter; its line in the file")
                        .clicked()
                    {
                        chosen = Some(chapter.number);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if chapter.touches.is_empty() {
                            ui.label(theme::weak("touches nothing"));
                        }
                        for link in &chapter.touches {
                            let (word, name) = link.said();
                            let found = link_asked(&games.session, &open.game, link);
                            let colour = if found.is_some() { link_colour(link) } else { theme::WRONG };
                            let chip =
                                egui::Button::new(RichText::new(format!("{word} {name}")).color(colour).size(theme::SMALL)).fill(colour.gamma_multiply(0.12));
                            let said = if found.is_some() { "open it in its space" } else { "not found in the project" };
                            if ui.add(chip).on_hover_text(said).clicked() {
                                asked = found;
                            }
                        }
                    });
                    for rule in &chapter.rules {
                        ui.label(RichText::new(rule).monospace().size(theme::SMALL).color(theme::TEXT));
                    }
                    match (chapter.leads_to.as_slice(), chapter.ends) {
                        ([], false) => _ = ui.label(theme::text("leads nowhere: no “chapter n” nor “finish”", theme::SMALL, theme::WRONG)),
                        (next, ends) => {
                            ui.horizontal_wrapped(|ui| {
                                for number in next {
                                    if ui.small_button(format!("→ chapter {number}")).clicked() {
                                        chosen = Some(*number);
                                    }
                                }
                                if ends {
                                    ui.label(theme::text("■ the end", theme::SMALL, theme::WEAK));
                                }
                            });
                        }
                    }
                });
            if at + 1 < open.boxes.len() {
                ui.vertical_centered(|ui| ui.label(theme::weak("↓")));
            }
        }
    });
    if let Some(number) = chosen {
        games.from_chapter = number;
        games.jump = open.text.lines().position(|line| line.split_whitespace().take(2).eq(["chapter", &number.to_string()])).map(|at| at + 1);
    }
    if asked.is_some() {
        games.asked = asked;
    }
}

/// Play, export; what is wrong with the game.
fn right(ui: &mut egui::Ui, games: &mut Games) {
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        let Some(open) = &games.open else {
            return;
        };
        part(ui, "PLAY");
        let numbers: Vec<(u16, String)> = open.boxes.iter().map(|chapter| (chapter.number, chapter.title.clone())).collect();
        let path = open.path.clone();
        let mut play = false;
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("play chapter").selected_text(format!("chapter {}", games.from_chapter)).show_ui(ui, |ui| {
                for (number, title) in &numbers {
                    ui.selectable_value(&mut games.from_chapter, *number, format!("{number} · {title}"));
                }
            });
            play = ui
                .add_enabled(!numbers.is_empty(), egui::Button::new(RichText::new(format!("▶ Play from Chapter {}", games.from_chapter)).strong()))
                .on_hover_text("a new game from that chapter, at its home, in a window of its own (scene_player play … --chapter n)")
                .clicked();
        });
        let export = ui.button("Export the Game…").on_hover_text("a .dwpack: the game's own files, nothing of the cartridge").clicked();
        let done = games.packing.as_mut().and_then(|run| run.poll().cloned());
        match (done, &games.packing) {
            (Some(done), _) => {
                games.said = Some(done.unwrap_or_else(|why| format!("not exported: {why}")));
                games.packing = None;
            }
            (None, Some(run)) => _ = ui.label(theme::weak(format!("{}…", run.doing))),
            (None, None) => {}
        }
        if let Some(said) = &games.said {
            ui.label(theme::text(said, theme::SMALL, theme::TEXT));
        }
        if play {
            if games.unsaved() {
                games.save();
            }
            games.said = Some(match launch(games.session.root(), &path, games.from_chapter) {
                Ok(()) => format!("playing from chapter {} in its own window", games.from_chapter),
                Err(why) => why,
            });
        }
        if export {
            if games.unsaved() {
                games.save();
            }
            export_game(games, &path);
        }
        part(ui, "CHECK THE GAME");
        if ui.small_button("Check Again").clicked() {
            games.check();
        }
        problems(ui, games);
    });
}

/// `scene_player play <game> --chapter n`, beside the Workshop, in a window of its own.
fn launch(root: &Path, game: &Path, chapter: u16) -> Result<(), String> {
    let folder = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)).unwrap_or_default();
    let player = folder.join(format!("scene_player{}", std::env::consts::EXE_SUFFIX));
    if !player.exists() {
        return Err(format!("{} not found beside the Workshop (cargo build -p scene_player)", player.display()));
    }
    let game = std::path::absolute(game).unwrap_or_else(|_| game.to_path_buf());
    std::process::Command::new(&player)
        .arg("play")
        .arg(&game)
        .args(["--chapter", &chapter.to_string()])
        .current_dir(root)
        .spawn()
        .map(|_| ())
        .map_err(|why| format!("scene_player: {why}"))
}

/// Where the pack goes asked, then made on a thread of its own.
fn export_game(games: &mut Games, game: &Path) {
    let stem = game.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let dist = games.session.root().join("dist");
    _ = std::fs::create_dir_all(&dist);
    let Some(out) = workshop_base::pick::save("Export the Game", &format!("{stem}.dwpack"), &[("Game Pack", &["dwpack"])], Some(&dist)) else {
        return;
    };
    let game = game.to_path_buf();
    let context = games.session.context.clone();
    games.packing = Some(workshop_base::tool_run::Run::spawn("packing the game", move || {
        let packed = ::game::pack::pack_game(&context, &game, &out)?;
        Ok(format!("exported: {} (“{}”, {} files, nothing of the cartridge)", packed.path.display(), packed.title, packed.files.len()))
    }));
}

/// What the check found, each a click from where it is.
fn problems(ui: &mut egui::Ui, games: &mut Games) {
    if games.checking.is_some() {
        ui.label(theme::weak("checking…"));
    }
    let Some(report) = &games.report else {
        return;
    };
    let report = match report {
        Ok(report) => report,
        Err(why) => {
            ui.label(theme::text(why, theme::SMALL, theme::WRONG));
            return;
        }
    };
    for note in &report.notes {
        ui.label(theme::text(format!("note: {note}"), theme::SMALL, theme::FAINT));
    }
    if report.problems.is_empty() {
        ui.label(RichText::new("nothing to review: it plays").color(Color32::from_rgb(0x6C, 0xC6, 0x6C)));
        return;
    }
    ui.label(theme::text(format!("{} problem(s)", report.problems.len()), theme::SMALL, theme::WRONG));
    let mut pointed: Option<Diagnostic> = None;
    for problem in &report.problems {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("●").color(theme::WRONG).size(theme::SMALL));
            let file = file_of(problem);
            let said = format!("{}:{} · {}", file.file_name().unwrap_or_default().to_string_lossy(), problem.line(), problem.message);
            if ui.add(egui::Label::new(RichText::new(said).color(theme::WRONG).size(theme::SMALL)).sense(Sense::click())).on_hover_text("show it").clicked() {
                pointed = Some(problem.clone());
            }
        });
    }
    if let Some(problem) = pointed {
        let (file, line) = (file_of(&problem), problem.line());
        match file.extension().and_then(|end| end.to_str()) {
            Some("game") => games.jump = Some(line.max(1)),
            Some("film") => games.asked = Some(Asked::Film(file, line)),
            Some("decor") => games.asked = Some(Asked::Decor(file, line)),
            Some("dungeon" | "fight") => games.asked = Some(Asked::DungeonFile(file)),
            _ => games.said = Some(format!("{}: nothing to open it in", file.display())),
        }
    }
}
