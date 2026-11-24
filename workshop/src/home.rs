//! The Home: the project at a glance.
//! - the Project (left): the recent files, the favourites, the films, the
//!   decors, the things drawn (in the Resources), the recipes the films
//!   share, the Workshop's bin; how many of each;
//! - the files (middle): new film, new decor, open, sought in all the
//!   project (their titles, places, those in them, their words); a table or
//!   a grid, each with a picture of its place, sorted by a click on a column;
//! - the Preview (right): its picture (▶ plays it), what it is (scenes,
//!   places as the game names them, who is in it, its problems), and what to
//!   do with it: open, duplicate, rename, put in the bin, a favourite.

use std::sync::Arc;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use workshop_base::session::Session;

use decor::tags::PlaceKind;
use eframe::egui::{self, Color32, FontId, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::app::{Space, Work};
use workshop_base::project::{Entry, Kind, Project};
use workshop_base::theme;

/// What the files show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    Recent,
    Favourites,
    Films,
    Decors,
    Shared,
    Bin,
}

impl Shelf {
    fn name(self) -> &'static str {
        match self {
            Shelf::Recent => "Recent",
            Shelf::Favourites => "Favourites",
            Shelf::Films => "Films",
            Shelf::Decors => "Decors",
            Shelf::Shared => "Shared Recipes",
            Shelf::Bin => "Bin",
        }
    }
}

/// A column of the table, to sort by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    Name,
    Kind,
    Places,
    Scenes,
    Changed,
}

/// The Home as the window keeps it.
pub struct Home {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    shelf: Shelf,
    sought: String,
    sort: (Column, bool),
    grid: bool,
    favourites: BTreeSet<PathBuf>,
    pictures: workshop_base::example::Examples,
    /// What the language says of a film (when it was read: its errors, its remarks).
    problems: HashMap<PathBuf, (Option<SystemTime>, usize, usize)>,
    renaming: Option<(String, bool)>,
    trashing: bool,
    erasing: bool,
    pub said: Option<String>,
}

impl Home {
    pub fn new(session: &Arc<Session>, context: &egui::Context) -> Home {
        let favourites = std::fs::read_to_string(favourites_file(session))
            .map(|text| text.lines().filter(|line| !line.is_empty()).map(PathBuf::from).collect())
            .unwrap_or_default();
        Home {
            session: session.clone(),
            shelf: Shelf::Recent,
            sought: String::new(),
            sort: (Column::Changed, true),
            grid: false,
            favourites,
            pictures: workshop_base::example::Examples::with_room(session, context, 400),
            problems: HashMap::new(),
            renaming: None,
            trashing: false,
            erasing: false,
            said: None,
        }
    }

    fn favourite(&mut self, path: &Path) {
        if !self.favourites.remove(path) {
            self.favourites.insert(path.to_path_buf());
        }
        if let Some(folder) = favourites_file(&self.session).parent() {
            _ = std::fs::create_dir_all(folder);
        }
        let said: Vec<String> = self.favourites.iter().map(|path| path.display().to_string()).collect();
        _ = std::fs::write(favourites_file(&self.session), said.join("\n"));
    }
}

fn favourites_file(session: &Session) -> PathBuf {
    session.root().join("target").join("workshop").join("home_favourites.txt")
}

/// A kind's colour.
fn colour_of(kind: Kind) -> Color32 {
    match kind {
        Kind::Film => theme::FILM,
        Kind::Decor => theme::DECOR,
    }
}

/// The little film that shows a file's place: its first scene's, a decor's own.
fn place_film(entry: &Entry) -> Option<String> {
    let place = match entry.kind {
        Kind::Film => entry.places.first()?.clone(),
        Kind::Decor => entry.path.file_stem()?.to_string_lossy().into_owned(),
    };
    Some(format!("scene: {place}\nwait 1.5s\n"))
}

/// A place as the game names it.
fn place_said(session: &Session, word: &str) -> String {
    session.tables().levels.get(&film::words::plain(word)).map(|level| level.name.clone()).filter(|name| !name.is_empty()).unwrap_or_else(|| {
        let said = word.replace('_', " ");
        let mut letters = said.chars();
        letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
    })
}

/// A file's picture painted in a room (its place, as the game shows it); its kind's colour while it is made.
fn paint_picture(painter: &egui::Painter, home: &mut Home, context: &egui::Context, entry: &Entry, room: Rect) {
    painter.rect_filled(room, 2.0, theme::GROUND);
    let picture = place_film(entry).map(|film| home.pictures.picture(context, &film));
    match picture {
        Some(Ok(Some(texture))) => {
            // (Its middle, filling the room.)
            let size = texture.size_vec2();
            let scale = (room.width() / size.x).max(room.height() / size.y);
            let seen = vec2(room.width() / (size.x * scale), room.height() / (size.y * scale));
            let uv = Rect::from_center_size(pos2(0.5, 0.5), seen);
            painter.image(texture.id(), room, uv, Color32::WHITE);
        }
        Some(Ok(None)) => {
            painter.rect_filled(room.shrink(1.0), 2.0, colour_of(entry.kind).gamma_multiply(0.18));
            context.request_repaint_after(std::time::Duration::from_millis(150));
        }
        _ => {
            painter.rect_filled(room.shrink(1.0), 2.0, colour_of(entry.kind).gamma_multiply(0.25));
            let first = entry.title.chars().next().map(|letter| letter.to_uppercase().to_string()).unwrap_or_default();
            painter.text(room.center(), egui::Align2::CENTER_CENTER, first, FontId::proportional(room.height() * 0.5), colour_of(entry.kind));
        }
    }
}

// --- The Projet ---

pub fn project(ui: &mut egui::Ui, work: &mut Work) {
    ui.add_space(4.0);
    let counts = [
        (Shelf::Recent, None, work.project.films.len() + work.project.decors.len(), theme::HOME),
        (Shelf::Favourites, None, work.home.favourites.len(), theme::FILM),
        (Shelf::Films, Some(theme::FILM), work.project.films.len(), theme::TEXT),
        (Shelf::Decors, Some(theme::DECOR), work.project.decors.len(), theme::TEXT),
    ];
    let row = |ui: &mut egui::Ui, said: &str, dot: Option<Color32>, count: usize, chosen: bool, colour: Color32| -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
        ui.painter().rect_filled(
            rect,
            0.0,
            if chosen {
                theme::CHOSEN
            } else if response.hovered() {
                theme::HOVERED
            } else {
                Color32::TRANSPARENT
            },
        );
        let mut x = rect.min.x + 10.0;
        if let Some(dot) = dot {
            ui.painter().rect_filled(Rect::from_center_size(pos2(x + 4.0, rect.center().y), vec2(8.0, 8.0)), 2.0, dot);
            x += 16.0;
        }
        ui.painter().text(
            pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            said,
            FontId::proportional(theme::BODY),
            if chosen { theme::STRONG } else { colour },
        );
        ui.painter().text(
            pos2(rect.max.x - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(theme::SMALL),
            if chosen { theme::STRONG } else { theme::FAINT },
        );
        response
    };
    ui.spacing_mut().item_spacing.y = 0.0;
    for (at, (shelf, dot, count, colour)) in counts.into_iter().enumerate() {
        if at == 2 {
            ui.add_space(8.0);
        }
        let said = if shelf == Shelf::Favourites { "★ Favourites".to_string() } else { shelf.name().to_string() };
        if row(ui, &said, dot, count, work.home.shelf == shelf && work.home.sought.is_empty(), colour).clicked() {
            (work.home.shelf, work.home.sought) = (shelf, String::new());
        }
    }
    // (The things drawn: in the Ressources.)
    let drawn = std::fs::read_dir(work.session.library().images())
        .map_or(0, |files| files.flatten().filter(|file| file.path().extension().is_some_and(|own| own == "png")).count());
    if row(ui, "Drawn Objects", Some(theme::ASSETS), drawn, false, theme::TEXT).on_hover_text("in the Resources space").clicked() {
        work.resources.show_drawn();
        work.session.ask(workshop_base::ask::Ask::Go(Space::Assets));
    }
    ui.add_space(8.0);
    for (shelf, count, colour) in [(Shelf::Shared, work.project.shared.len(), theme::TEXT), (Shelf::Bin, work.project.binned.len(), theme::WEAK)] {
        let said = shelf.name();
        if row(ui, said, None, count, work.home.shelf == shelf && work.home.sought.is_empty(), colour).clicked() {
            (work.home.shelf, work.home.sought) = (shelf, String::new());
        }
    }
}

// --- The files ---

/// The files the shelf shows (or those that answer what is sought), sorted.
fn shown(work: &Work) -> Vec<Entry> {
    let home = &work.home;
    let sought = film::words::plain(home.sought.trim());
    let mut entries: Vec<Entry> = if !sought.is_empty() {
        work.project
            .films
            .iter()
            .chain(&work.project.decors)
            .chain(&work.project.shared)
            .filter(|entry| {
                let file = entry.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
                [entry.title.as_str(), file.as_str(), entry.about.as_str()]
                    .iter()
                    .chain(entry.cast.iter().map(String::as_str).collect::<Vec<_>>().iter())
                    .any(|said| film::words::plain(said).contains(&sought))
                    || film::words::plain(&entry.text).contains(&sought)
            })
            .cloned()
            .collect()
    } else {
        match home.shelf {
            Shelf::Recent => work.project.recent().into_iter().take(60).cloned().collect(),
            Shelf::Favourites => work
                .project
                .films
                .iter()
                .chain(&work.project.decors)
                .chain(&work.project.shared)
                .filter(|entry| home.favourites.contains(&entry.path))
                .cloned()
                .collect(),
            Shelf::Films => work.project.films.clone(),
            Shelf::Decors => work.project.decors.clone(),
            Shelf::Shared => work.project.shared.clone(),
            Shelf::Bin => work.project.binned.clone(),
        }
    };
    let (column, back) = home.sort;
    if !(home.shelf == Shelf::Recent && sought.is_empty() && column == Column::Changed) {
        entries.sort_by(|a, b| {
            let order = match column {
                Column::Name => film::words::plain(&a.title).cmp(&film::words::plain(&b.title)),
                Column::Kind => a.kind.word().cmp(b.kind.word()),
                Column::Places => a.about.cmp(&b.about),
                Column::Scenes => a.scenes.cmp(&b.scenes),
                Column::Changed => a.changed.cmp(&b.changed),
            };
            if back { order.reverse() } else { order }
        });
    }
    entries
}

/// The columns of the table: their names, widths (the name takes the rest), what they sort by.
const COLUMNS: [(&str, f32, Column); 4] =
    [("Kind", 70.0, Column::Kind), ("Places · Size", 220.0, Column::Places), ("Scenes", 60.0, Column::Scenes), ("Modified", 100.0, Column::Changed)];

pub fn list(ui: &mut egui::Ui, work: &mut Work) {
    let context = ui.ctx().clone();
    tools(ui, work);
    // --- Where one is, how it is seen ---
    let entries = shown(work);
    if work.chosen.is_none()
        && let Some(asked) = work.session.launch.home_file.clone()
    {
        work.chosen = work
            .project
            .films
            .iter()
            .chain(&work.project.decors)
            .find(|entry| entry.path.file_stem().is_some_and(|stem| stem.to_string_lossy() == asked))
            .map(|entry| entry.path.clone());
    }
    let (entries, sections) = grouped_decors(work, entries);
    head(ui, work, entries.len());
    let chosen_at = entries.iter().position(|entry| work.chosen.as_ref() == Some(&entry.path));
    let clicked = match &sections {
        Some(sections) => grouped(ui, work, &context, &entries, sections, chosen_at),
        None if work.home.grid => grid(ui, work, &context, &entries, chosen_at),
        None => table(ui, work, &context, &entries, chosen_at),
    };
    if entries.is_empty() {
        let said = match work.home.shelf {
            _ if !work.home.sought.trim().is_empty() => "nothing answers this search",
            Shelf::Favourites => "no favourite yet: ★ in the Preview",
            Shelf::Bin => "the bin is empty",
            _ => "nothing here",
        };
        ui.label(theme::weak(said));
    }
    if let Some((at, twice)) = clicked {
        choose(work, &entries[at]);
        if twice && work.home.shelf != Shelf::Bin {
            let entry = entries[at].clone();
            work.open(&entry);
        }
    }
    keys(ui, work, &entries, chosen_at);
}

/// The tools: open (any file of the project); sought in all the project.
fn tools(ui: &mut egui::Ui, work: &mut Work) {
    let tools = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 34.0));
    ui.painter().rect_filled(tools, 0.0, theme::TOOLS);
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(tools.shrink2(vec2(6.0, 5.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    bar.set_clip_rect(tools.intersect(ui.clip_rect()));
    let mut all: Vec<Entry> = work.project.films.iter().chain(&work.project.decors).cloned().collect();
    all.sort_by_key(|entry| film::words::plain(&entry.title));
    bar.menu_button("Open…", |ui| {
        ui.set_max_height(420.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in &all {
                let said = RichText::new(format!("{}  ·  {}", entry.title, entry.kind.word())).color(colour_of(entry.kind));
                if ui.button(said).clicked() {
                    work.open(entry);
                    ui.close();
                }
            }
        });
    });
    bar.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let field = ui.add(egui::TextEdit::singleline(&mut work.home.sought).hint_text("Search the project  (Ctrl K)").desired_width(280.0));
        if ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::K)) {
            field.request_focus();
        }
        if !work.home.sought.is_empty() && ui.small_button("✕").clicked() {
            work.home.sought.clear();
        }
    });
    ui.advance_cursor_after_rect(tools);
}

/// Where one is, how many files, the list or the grid, what was said.
fn head(ui: &mut egui::Ui, work: &mut Work, count: usize) {
    let head = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 26.0));
    ui.painter().rect_filled(head, 0.0, theme::TABS);
    let title = if work.home.sought.trim().is_empty() { work.home.shelf.name().to_string() } else { format!("\"{}\" in the project", work.home.sought.trim()) };
    ui.painter().text(head.left_center() + vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, title, FontId::proportional(theme::BODY), theme::STRONG);
    let mut head_ui = ui.new_child(egui::UiBuilder::new().max_rect(head.shrink2(vec2(8.0, 2.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
    if head_ui.selectable_label(!work.home.grid, "List").clicked() {
        work.home.grid = false;
    }
    if head_ui.selectable_label(work.home.grid, "Grid").clicked() {
        work.home.grid = true;
    }
    head_ui.label(theme::text(format!("{count} file{}", if count > 1 { "s" } else { "" }), theme::SMALL, theme::WEAK));
    if let Some(said) = &work.home.said {
        head_ui.label(theme::text(said, theme::SMALL, theme::FAINT));
    }
    ui.advance_cursor_after_rect(head);
}

/// A card's size in the grid.
const CARD: egui::Vec2 = vec2(180.0, 150.0);

/// How many cards a row of that width holds (one at least).
fn grid_columns(width: f32) -> usize {
    ((width - 12.0) / (CARD.x + 10.0)).floor().max(1.0) as usize
}

/// A grid of their pictures; the one clicked (twice?).
fn grid(ui: &mut egui::Ui, work: &mut Work, context: &egui::Context, entries: &[Entry], chosen_at: Option<usize>) -> Option<(usize, bool)> {
    let mut clicked = None;
    let columns = grid_columns(ui.available_width());
    let rows = entries.len().div_ceil(columns);
    egui::ScrollArea::vertical().id_salt("home grid").auto_shrink(false).show_rows(ui, CARD.y + 10.0, rows, |ui, range| {
        for row in range {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 10.0;
                for (at, entry) in entries.iter().enumerate().take(((row + 1) * columns).min(entries.len())).skip(row * columns) {
                    let response = grid_card(ui, work, context, entry, Some(at) == chosen_at);
                    if response.clicked() || response.double_clicked() {
                        clicked = Some((at, response.double_clicked()));
                    }
                }
            });
            ui.add_space(10.0);
        }
    });
    clicked
}

/// The Decors shelf (nothing sought): its decors by the kind of place they
/// are (`library/tables/place_kinds.txt`), in that order, and the sections; none else.
fn grouped_decors(work: &Work, entries: Vec<Entry>) -> (Vec<Entry>, Option<Vec<(PlaceKind, usize)>>) {
    if work.home.shelf != Shelf::Decors || !work.home.sought.trim().is_empty() {
        return (entries, None);
    }
    let kinds = workshop_base::project::place_kinds(work.session.library());
    let kind_of = |entry: &Entry| {
        let name = entry.path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        workshop_decor::decor_kind(&kinds, &name, &entry.text)
    };
    let groups = decor::tags::grouped(entries, kind_of);
    let sections = groups.iter().map(|(kind, group)| (*kind, group.len())).collect();
    (groups.into_iter().flat_map(|(_, group)| group).collect(), Some(sections))
}

/// The decors in sections, a kind of place each (its name, how many), in the
/// grid or the list; the one clicked (twice?).
fn grouped(
    ui: &mut egui::Ui,
    work: &mut Work,
    context: &egui::Context,
    entries: &[Entry],
    sections: &[(PlaceKind, usize)],
    chosen_at: Option<usize>,
) -> Option<(usize, bool)> {
    let mut clicked = None;
    let columns = if work.home.grid { grid_columns(ui.available_width()) } else { 1 };
    let name_width = if work.home.grid { 0.0 } else { table_header(ui, work) };
    egui::ScrollArea::vertical().id_salt("home sections").auto_shrink(false).show(ui, |ui| {
        let mut first = 0;
        for (kind, count) in sections {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(theme::text(kind.shown().to_uppercase(), theme::SMALL, theme::WEAK));
                ui.label(theme::text(count.to_string(), theme::SMALL, theme::FAINT));
            });
            ui.add_space(4.0);
            let range: Vec<usize> = (first..first + count).collect();
            first += count;
            if !work.home.grid {
                ui.spacing_mut().item_spacing.y = 0.0;
                for at in range {
                    let response = table_row(ui, work, context, &entries[at], Some(at) == chosen_at, name_width);
                    if response.clicked() || response.double_clicked() {
                        clicked = Some((at, response.double_clicked()));
                    }
                }
                continue;
            }
            for row in range.chunks(columns) {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for at in row {
                        let response = grid_card(ui, work, context, &entries[*at], Some(*at) == chosen_at);
                        if response.clicked() || response.double_clicked() {
                            clicked = Some((*at, response.double_clicked()));
                        }
                    }
                });
                ui.add_space(10.0);
            }
        }
    });
    clicked
}

/// A file's card: its picture (a star if a favourite), its title, its kind and age.
fn grid_card(ui: &mut egui::Ui, work: &mut Work, context: &egui::Context, entry: &Entry, chosen: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(CARD, Sense::click());
    let fill = if chosen {
        theme::CHOSEN
    } else if response.hovered() {
        theme::HOVERED
    } else {
        theme::TOOLS
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    let picture = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(CARD.x - 12.0, 96.0));
    paint_picture(ui.painter(), &mut work.home, context, entry, picture);
    if work.home.favourites.contains(&entry.path) {
        workshop_base::icons::star(ui.painter(), Rect::from_min_size(picture.min + vec2(4.0, 4.0), vec2(12.0, 12.0)), true, theme::FILM);
    }
    let painter = ui.painter_at(rect);
    painter.text(pos2(rect.min.x + 8.0, picture.max.y + 12.0), egui::Align2::LEFT_CENTER, &entry.title, FontId::proportional(theme::BODY), theme::STRONG);
    painter.rect_filled(Rect::from_center_size(pos2(rect.min.x + 12.0, picture.max.y + 32.0), vec2(8.0, 8.0)), 2.0, colour_of(entry.kind));
    painter.text(
        pos2(rect.min.x + 22.0, picture.max.y + 32.0),
        egui::Align2::LEFT_CENTER,
        format!("{} · {}", entry.kind.word(), entry.age()),
        FontId::proportional(theme::SMALL),
        theme::WEAK,
    );
    response.on_hover_text(entry.path.display().to_string())
}

/// The sort after a click on a column's head: the same column reversed, or
/// another (the newest and the most scenes first).
fn sorted_by(sort: (Column, bool), column: Column) -> (Column, bool) {
    if sort.0 == column { (column, !sort.1) } else { (column, matches!(column, Column::Changed | Column::Scenes)) }
}

/// A table: the header sorts; the one clicked (twice?).
fn table(ui: &mut egui::Ui, work: &mut Work, context: &egui::Context, entries: &[Entry], chosen_at: Option<usize>) -> Option<(usize, bool)> {
    let mut clicked = None;
    let name_width = table_header(ui, work);
    egui::ScrollArea::vertical().id_salt("home list").auto_shrink(false).show_rows(ui, 44.0, entries.len(), |ui, range| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for at in range {
            let response = table_row(ui, work, context, &entries[at], Some(at) == chosen_at, name_width);
            if response.clicked() || response.double_clicked() {
                clicked = Some((at, response.double_clicked()));
            }
        }
    });
    clicked
}

/// The table's header, each column sorting; the name's width.
fn table_header(ui: &mut egui::Ui, work: &mut Work) -> f32 {
    let fixed: f32 = COLUMNS.iter().map(|(_, width, _)| width).sum::<f32>() + 76.0;
    let (header, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    ui.painter().rect_filled(header, 0.0, theme::TOOLS);
    let name_width = (header.width() - fixed - 16.0).max(140.0);
    let mut x = header.left() + 8.0 + 76.0;
    for (word, width, column) in std::iter::once(("Name", name_width, Column::Name)).chain(COLUMNS) {
        let room = Rect::from_min_max(pos2(x, header.top()), pos2(x + width - 6.0, header.bottom()));
        let pressed = ui.interact(room, ui.id().with(("colonne", word)), Sense::click());
        let sorted = work.home.sort.0 == column;
        let said = if sorted { format!("{word} {}", if work.home.sort.1 { "▼" } else { "▲" }) } else { word.to_string() };
        ui.painter().text(
            pos2(x, header.center().y),
            egui::Align2::LEFT_CENTER,
            said,
            FontId::proportional(theme::SMALL),
            if sorted || pressed.hovered() { theme::STRONG } else { theme::WEAK },
        );
        if pressed.on_hover_text("sort by this (again: reversed)").clicked() {
            work.home.sort = sorted_by(work.home.sort, column);
        }
        x += width;
    }
    name_width
}

/// A row of the table: its picture, its title (a star if a favourite) and file, its kind, places, scenes, age.
fn table_row(ui: &mut egui::Ui, work: &mut Work, context: &egui::Context, entry: &Entry, chosen: bool, name_width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let painter = ui.painter_at(rect);
    let fill = if chosen {
        theme::CHOSEN
    } else if response.hovered() {
        theme::HOVERED
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(rect, 0.0, fill);
    painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0_f32, theme::TOOLS));
    let picture = Rect::from_min_size(rect.min + vec2(8.0, 4.0), vec2(64.0, 36.0));
    paint_picture(&painter, &mut work.home, context, entry, picture);
    let font = FontId::proportional(theme::BODY);
    let mut x = rect.left() + 8.0 + 76.0;
    let star = if work.home.favourites.contains(&entry.path) { "★ " } else { "" };
    painter.text(pos2(x, rect.top() + 14.0), egui::Align2::LEFT_CENTER, format!("{star}{}", entry.title), font.clone(), theme::STRONG);
    let file = entry.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    painter.text(
        pos2(x, rect.top() + 30.0),
        egui::Align2::LEFT_CENTER,
        file,
        FontId::monospace(theme::SMALL - 1.0),
        if chosen { theme::TEXT } else { theme::FAINT },
    );
    x += name_width;
    let places = if entry.kind == Kind::Film {
        entry.places.iter().map(|place| place_said(&work.session, place)).collect::<Vec<_>>().join(", ")
    } else {
        entry.about.clone()
    };
    let scenes = if entry.kind == Kind::Film { entry.scenes.to_string() } else { "—".to_string() };
    let cells = [(entry.kind.word().to_string(), colour_of(entry.kind)), (places, theme::TEXT), (scenes, theme::TEXT), (entry.age(), theme::TEXT)];
    for ((said, colour), (_, width, _)) in cells.into_iter().zip(COLUMNS) {
        let room = Rect::from_min_max(pos2(x, rect.top()), pos2(x + width - 10.0, rect.bottom()));
        painter.with_clip_rect(room).text(pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, said, font.clone(), colour);
        x += width;
    }
    response.on_hover_text(entry.path.display().to_string())
}

/// The file the arrows go to, from that place, among so many: ↑ ↓ a row
/// (`step` files), ← → one (in the grid only).
fn after_arrows(now: usize, count: usize, step: i64, grid: bool, [up, down, left, right]: [bool; 4]) -> usize {
    let mut next = now as i64 + step * (i64::from(down) - i64::from(up));
    if grid {
        next += i64::from(right) - i64::from(left);
    }
    next.clamp(0, count as i64 - 1) as usize
}

/// The keys, the pointer over it: ↑ ↓ (and ← → in the grid) another; Enter opens; Delete: the bin.
fn keys(ui: &egui::Ui, work: &mut Work, entries: &[Entry], chosen_at: Option<usize>) {
    if !ui.ui_contains_pointer() || ui.ctx().wants_keyboard_input() || entries.is_empty() {
        return;
    }
    let (up, down, left, right, enter, delete) = ui.input(|input| {
        use egui::Key;
        (
            input.key_pressed(Key::ArrowUp),
            input.key_pressed(Key::ArrowDown),
            input.key_pressed(Key::ArrowLeft),
            input.key_pressed(Key::ArrowRight),
            input.key_pressed(Key::Enter),
            input.key_pressed(Key::Delete),
        )
    });
    let grid = work.home.grid;
    let step = if grid { ((ui.available_width() - 12.0) / 190.0).floor().max(1.0) as i64 } else { 1 };
    if up || down || (grid && (left || right)) {
        let entry = entries[after_arrows(chosen_at.unwrap_or(0), entries.len(), step, grid, [up, down, left, right])].clone();
        choose(work, &entry);
    }
    if enter && let Some(entry) = chosen_at.map(|at| entries[at].clone()).filter(|_| work.home.shelf != Shelf::Bin) {
        work.open(&entry);
    }
    if delete && chosen_at.is_some() {
        work.home.trashing = true;
    }
}

/// A file chosen: what is being done to the one before let go.
fn choose(work: &mut Work, entry: &Entry) {
    if work.chosen.as_ref() != Some(&entry.path) {
        (work.home.renaming, work.home.trashing, work.home.erasing) = (None, false, false);
    }
    work.chosen = Some(entry.path.clone());
}

// --- The Preview ---

pub fn preview(ui: &mut egui::Ui, work: &mut Work) {
    let Some(entry) = work.chosen.clone().and_then(|path| work.project.entry(&path).cloned()) else {
        ui.add_space(8.0);
        ui.label(theme::weak("Choose a file to see it here; a double-click opens it."));
        return;
    };
    let context = ui.ctx().clone();
    let binned = work.project.binned.iter().any(|known| known.path == entry.path);
    egui::ScrollArea::vertical().id_salt("home preview").auto_shrink(false).show(ui, |ui| {
        preview_picture(ui, work, &context, &entry, binned);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(theme::text(&entry.title, theme::TITLE, theme::STRONG));
        });
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.add(egui::Label::new(RichText::new(entry.path.display().to_string()).monospace().size(theme::SMALL - 1.0).color(theme::FAINT)).wrap());
        });
        // --- Details ---
        section(ui, "Details");
        let problems = (entry.kind == Kind::Film && !binned).then(|| problems_of(work, &entry));
        let used = films_set_in(work, &entry);
        details(ui, work, &entry, problems, &used);
        // --- What to do with it ---
        ui.add_space(10.0);
        if binned {
            bin_actions(ui, work, &entry);
        } else {
            actions(ui, work, &entry, &used);
        }
        if let Some(said) = &work.home.said {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.add(egui::Label::new(theme::text(said, theme::SMALL, theme::FAINT)).wrap());
            });
        }
    });
}

/// Its picture: ▶ plays a film.
fn preview_picture(ui: &mut egui::Ui, work: &mut Work, context: &egui::Context, entry: &Entry, binned: bool) {
    let room = Rect::from_min_size(ui.cursor().min + vec2(8.0, 8.0), vec2(ui.available_width() - 16.0, 180.0));
    let picture = ui.allocate_rect(room.expand2(vec2(0.0, 8.0)), Sense::click());
    paint_picture(ui.painter(), &mut work.home, context, entry, room);
    ui.painter().rect_stroke(room, 0.0, Stroke::new(1.0_f32, theme::GAP), StrokeKind::Outside);
    if entry.kind == Kind::Film && !binned {
        let button = Rect::from_center_size(room.center(), vec2(44.0, 44.0));
        ui.painter().circle_filled(button.center(), 22.0, Color32::from_black_alpha(if picture.hovered() { 200 } else { 150 }));
        let tip = button.center();
        ui.painter().add(egui::Shape::convex_polygon(
            vec![tip + vec2(-6.0, -10.0), tip + vec2(11.0, 0.0), tip + vec2(-6.0, 10.0)],
            Color32::WHITE,
            Stroke::NONE,
        ));
        if picture.on_hover_text("open it and play it").clicked() {
            work.open(entry);
            work.films.go(0);
            work.films.playing = true;
        }
    }
}

/// A film's errors and remarks, kept while its file does not change.
fn problems_of(work: &mut Work, entry: &Entry) -> (usize, usize) {
    let known = work.home.problems.get(&entry.path).filter(|(when, ..)| *when == entry.changed).map(|(_, wrong, remarked)| (*wrong, *remarked));
    known.unwrap_or_else(|| {
        let told = film::service::diagnostics(&entry.text, work.session.tables());
        let wrong = told.iter().filter(|told| told.is_error()).count();
        let made = (wrong, told.len() - wrong);
        work.home.problems.insert(entry.path.clone(), (entry.changed, made.0, made.1));
        made
    })
}

/// The films set in a decor (none for another file).
fn films_set_in(work: &Work, entry: &Entry) -> Vec<String> {
    if entry.kind != Kind::Decor {
        return Vec::new();
    }
    let word = film::words::plain(&entry.path.file_stem().unwrap_or_default().to_string_lossy());
    work.project.films.iter().filter(|film| film.places.iter().any(|place| film::words::plain(place) == word)).map(|film| film.title.clone()).collect()
}

/// A film's state, as its errors and remarks say it, and its colour.
fn state_said(wrong: usize, remarked: usize) -> (String, Color32) {
    match (wrong, remarked) {
        (0, 0) => ("no problems".to_string(), Color32::from_rgb(0x4A, 0xDE, 0x80)),
        (0, remarked) => (format!("{remarked} remark{}", if remarked > 1 { "s" } else { "" }), Color32::from_rgb(0xE6, 0xB4, 0x50)),
        (wrong, _) => (format!("{wrong} error{}", if wrong > 1 { "s" } else { "" }), theme::WRONG),
    }
}

/// How many lines of a film someone says (`who: …`, `who (face): …`).
fn lines_said(text: &str) -> usize {
    text.lines().filter(|line| line.split_once(':').is_some_and(|(who, _)| !who.trim().is_empty() && !who.contains(' ') || who.contains('('))).count()
}

/// The details: its kind; a film's scenes, places, characters, lines said; another's size and the films set there;
/// its lines, its state, when it changed.
fn details(ui: &mut egui::Ui, work: &Work, entry: &Entry, problems: Option<(usize, usize)>, used: &[String]) {
    egui::Grid::new("home details").num_columns(2).spacing([12.0, 6.0]).min_col_width(84.0).show(ui, |ui| {
        let pair = |ui: &mut egui::Ui, key: &str, value: RichText| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(theme::text(key, theme::SMALL, theme::WEAK));
            });
            ui.add(egui::Label::new(value.size(theme::SMALL)).wrap());
            ui.end_row();
        };
        pair(ui, "Kind", RichText::new(entry.kind.word()).color(colour_of(entry.kind)));
        if entry.kind == Kind::Film {
            pair(ui, "Scenes", RichText::new(entry.scenes.to_string()));
            pair(ui, "Places", RichText::new(entry.places.iter().map(|place| place_said(&work.session, place)).collect::<Vec<_>>().join(", ")));
            let cast: Vec<String> = entry
                .cast
                .iter()
                .take(10)
                .map(|name| {
                    let plain = film::words::plain(name);
                    work.session.tables().monsters.get(&plain).map_or_else(|| name.clone(), |(_, game)| game.clone())
                })
                .collect();
            let more = entry.cast.len().saturating_sub(10);
            pair(ui, "Characters", RichText::new(format!("{}{}", cast.join(", "), if more > 0 { format!(" and {more} others") } else { String::new() })));
            pair(ui, "Lines", RichText::new(lines_said(&entry.text).to_string()));
        } else {
            pair(ui, "Size", RichText::new(entry.about.clone()));
            let films =
                if used.is_empty() { "none".to_string() } else { format!("{} : {}", used.len(), used.iter().take(6).cloned().collect::<Vec<_>>().join(", ")) };
            pair(ui, "In the Films", RichText::new(films));
        }
        pair(ui, "Lines", RichText::new(entry.lines.to_string()));
        if let Some((wrong, remarked)) = problems {
            let (said, colour) = state_said(wrong, remarked);
            pair(ui, "State", RichText::new(said).color(colour));
        }
        pair(ui, "Modified", RichText::new(entry.age()));
    });
}

/// A part of the Preview's, by its name.
fn section(ui: &mut egui::Ui, name: &str) {
    ui.add_space(8.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::TOOLS);
    workshop_base::icons::chevron(ui.painter(), pos2(rect.min.x + 10.0, rect.center().y), true, theme::WEAK);
    ui.painter().text(pos2(rect.min.x + 20.0, rect.center().y), egui::Align2::LEFT_CENTER, name, FontId::proportional(theme::BODY), theme::STRONG);
    ui.add_space(4.0);
}

/// A file of the project: open, duplicate, rename, the bin, a favourite, where it is.
fn actions(ui: &mut egui::Ui, work: &mut Work, entry: &Entry, used: &[String]) {
    let half = ((ui.available_width() - 24.0) / 2.0).max(60.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if ui.add_sized([half, 26.0], egui::Button::new(RichText::new("Open").strong().color(theme::STRONG)).fill(theme::DECOR)).clicked() {
            work.open(entry);
        }
        if ui.add_sized([half, 26.0], egui::Button::new("Duplicate")).on_hover_text("a copy, under another file name").clicked() {
            work.home.said = Some(match duplicated(entry) {
                Ok(path) => {
                    work.project = Project::read(&work.session);
                    work.chosen = Some(path.clone());
                    format!("copied: {}", path.file_name().unwrap_or_default().to_string_lossy())
                }
                Err(why) => why,
            });
        }
    });
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if ui.add_sized([half, 26.0], egui::Button::new("Rename")).clicked() {
            let stem = entry.path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            work.home.renaming = Some((stem, true));
        }
        if ui.add_sized([half, 26.0], egui::Button::new(RichText::new("Bin").color(theme::WRONG))).clicked() {
            work.home.trashing = true;
        }
    });
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let favourite = work.home.favourites.contains(&entry.path);
        if ui.selectable_label(favourite, if favourite { "★ Favourite" } else { "☆ Favourite" }).clicked() {
            work.home.favourite(&entry.path);
        }
        if ui.button("Show in Explorer").clicked() {
            _ = std::process::Command::new("explorer").arg(format!("/select,{}", entry.path.display())).spawn();
        }
    });
    renaming(ui, work, entry, used);
    if work.home.trashing {
        trashing(ui, work, entry);
    }
}

/// Named anew: its file; a decor, in the films that are set there too, if asked.
fn renaming(ui: &mut egui::Ui, work: &mut Work, entry: &Entry, used: &[String]) {
    let mut done: Option<bool> = None;
    if let Some((name, in_films)) = &mut work.home.renaming {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let field = ui.add(egui::TextEdit::singleline(name).desired_width(150.0));
            if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                done = Some(true);
            }
            if ui.button("Rename").clicked() {
                done = Some(true);
            }
            if ui.button("Cancel").clicked() {
                done = Some(false);
            }
        });
        if entry.kind == Kind::Decor && !used.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.checkbox(
                    in_films,
                    theme::text(format!("also in the {} film{} set there", used.len(), if used.len() > 1 { "s" } else { "" }), theme::SMALL, theme::TEXT),
                );
            });
        }
    }
    match done {
        Some(true) => {
            let (name, in_films) = work.home.renaming.take().unwrap_or_default();
            work.home.said = Some(match renamed(work, entry, &name, in_films) {
                Ok(said) => said,
                Err(why) => why,
            });
        }
        Some(false) => work.home.renaming = None,
        None => {}
    }
}

/// Put in the bin: asked once more.
fn trashing(ui: &mut egui::Ui, work: &mut Work, entry: &Entry) {
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text("Put this file in the Workshop's bin?", theme::SMALL, theme::WRONG));
        if ui.button("Yes").clicked() {
            work.home.said = Some(match binned(&work.session, entry) {
                Ok(to) => {
                    if work.film.as_ref() == Some(&entry.path) {
                        work.film = None;
                    }
                    if work.decor.as_ref() == Some(&entry.path) {
                        work.decor = None;
                    }
                    work.project = Project::read(&work.session);
                    work.chosen = None;
                    format!("in the bin: {}", to.file_name().unwrap_or_default().to_string_lossy())
                }
                Err(why) => why,
            });
            work.home.trashing = false;
        }
        if ui.button("No").clicked() {
            work.home.trashing = false;
        }
    });
}

/// A file of the bin: taken back where it was, or rubbed out for good (asked once more).
fn bin_actions(ui: &mut egui::Ui, work: &mut Work, entry: &Entry) {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if ui
            .add(egui::Button::new(RichText::new("Put Back").strong().color(theme::STRONG)).fill(theme::DECOR))
            .on_hover_text("in its folder, as before")
            .clicked()
        {
            work.home.said = Some(match restored(&work.session, &entry.path) {
                Ok(to) => {
                    work.project = Project::read(&work.session);
                    work.chosen = Some(to.clone());
                    format!("put back: {}", to.display())
                }
                Err(why) => why,
            });
        }
        if ui.button(RichText::new("Delete Permanently…").color(theme::WRONG)).clicked() {
            work.home.erasing = true;
        }
    });
    if work.home.erasing {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_space(8.0);
            ui.label(theme::text("Erase this file forever? It cannot be taken back.", theme::SMALL, theme::WRONG));
            if ui.button("Erase").clicked() {
                work.home.said = Some(match std::fs::remove_file(&entry.path) {
                    Ok(()) => {
                        work.project = Project::read(&work.session);
                        work.chosen = None;
                        "erased".to_string()
                    }
                    Err(why) => why.to_string(),
                });
                work.home.erasing = false;
            }
            if ui.button("No").clicked() {
                work.home.erasing = false;
            }
        });
    }
}

/// A file put in the bin (another of the same name there: this one's time before it).
fn binned(session: &Session, entry: &Entry) -> Result<PathBuf, String> {
    let bin = workshop_base::project::bin(session.root());
    std::fs::create_dir_all(&bin).map_err(|why| why.to_string())?;
    let name = entry.path.file_name().ok_or("no name")?.to_string_lossy().into_owned();
    let mut to = bin.join(&name);
    if to.exists() {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_secs());
        to = bin.join(format!("{now}_{name}"));
    }
    std::fs::rename(&entry.path, &to).map_err(|why| why.to_string())?;
    Ok(to)
}

/// A file of the bin back in the folder of its kind (a film's, a decor's, an image's), under its own name.
fn restored(session: &Session, path: &Path) -> Result<PathBuf, String> {
    let name = path.file_name().ok_or("no name")?.to_string_lossy().into_owned();
    // (The time put before it in the bin, taken off.)
    let name = match name.split_once('_') {
        Some((time, rest)) if time.len() >= 9 && time.chars().all(|letter| letter.is_ascii_digit()) => rest.to_string(),
        _ => name,
    };
    let folder = match path.extension().map(|own| own.to_string_lossy().into_owned()).as_deref() {
        Some("film") => session.library().films(),
        Some("decor") => session.library().decors(),
        Some("png") => session.library().images(),
        _ => return Err("an unknown kind of file".to_string()),
    };
    let to = folder.join(&name);
    if to.exists() {
        return Err(format!("\"{name}\" is already there"));
    }
    std::fs::rename(path, &to).map_err(|why| why.to_string())?;
    Ok(to)
}

/// A copy of a file, under a name of its own (a film's title said a copy).
fn duplicated(entry: &Entry) -> Result<PathBuf, String> {
    let stem = entry.path.file_stem().ok_or("no name")?.to_string_lossy().into_owned();
    let extension = entry.path.extension().map(|own| own.to_string_lossy().into_owned()).unwrap_or_default();
    let mut to = entry.path.with_file_name(format!("{stem}_copie.{extension}"));
    let mut number = 2;
    while to.exists() {
        to = entry.path.with_file_name(format!("{stem}_copy{number}.{extension}"));
        number += 1;
    }
    let text = match entry.kind {
        Kind::Film => {
            entry
                .text
                .lines()
                .map(|line| if line.trim_start().starts_with("title:") { format!("{} (copy)", line.trim_end()) } else { line.to_string() })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        }
        Kind::Decor => entry.text.clone(),
    };
    std::fs::write(&to, text).map_err(|why| why.to_string())?;
    Ok(to)
}

/// A file named anew; a decor, in the films set there too if asked; the open ones follow.
fn renamed(work: &mut Work, entry: &Entry, name: &str, in_films: bool) -> Result<String, String> {
    let stem = film::words::plain(&name.trim().replace([' ', '-'], "_"));
    let old = entry.path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    if stem.is_empty() || stem == old {
        return Err("the same name".to_string());
    }
    let extension = entry.path.extension().map(|own| own.to_string_lossy().into_owned()).unwrap_or_default();
    let to = entry.path.with_file_name(format!("{stem}.{extension}"));
    if to.exists() {
        return Err(format!("\"{stem}\" is already taken"));
    }
    std::fs::rename(&entry.path, &to).map_err(|why| why.to_string())?;
    let mut changed = 0;
    if entry.kind == Kind::Decor && in_films {
        for film in work.project.films.iter().filter(|film| film.places.iter().any(|place| film::words::plain(place) == film::words::plain(&old))) {
            let made: String = film
                .text
                .split_inclusive('\n')
                .map(|line| match line.trim_start().strip_prefix("scene:") {
                    Some(rest) if rest.split_whitespace().next().is_some_and(|place| film::words::plain(place) == film::words::plain(&old)) => {
                        line.replacen(&format!(" {}", rest.split_whitespace().next().unwrap_or_default()), &format!(" {stem}"), 1)
                    }
                    _ => line.to_string(),
                })
                .collect();
            if made != film.text && std::fs::write(&film.path, made).is_ok() {
                changed += 1;
            }
        }
    }
    if work.film.as_ref() == Some(&entry.path) {
        work.film = Some(to.clone());
    }
    if work.decor.as_ref() == Some(&entry.path) {
        work.decor = Some(to.clone());
        work.decors.open(&to);
    }
    work.project = Project::read(&work.session);
    work.chosen = Some(to);
    Ok(format!("renamed \"{stem}\"{}", if changed > 0 { format!(", and in {changed} film{}", if changed > 1 { "s" } else { "" }) } else { String::new() }))
}

#[cfg(test)]
mod tests {
    use super::{Column, after_arrows, grid_columns, lines_said, sorted_by, state_said};

    /// The files' sums: how many cards in a row, where the arrows go, the sort after a click.
    #[test]
    fn the_files_lie_and_sort() {
        assert_eq!((grid_columns(392.0), grid_columns(100.0)), (2, 1));
        assert_eq!(after_arrows(1, 10, 3, true, [false, true, false, false]), 4, "a row down");
        assert_eq!(after_arrows(1, 10, 3, true, [false, false, true, false]), 0, "one left");
        assert_eq!(after_arrows(1, 10, 1, false, [true, false, false, false]), 0);
        assert_eq!(after_arrows(9, 10, 3, true, [false, true, false, false]), 9, "no farther than the last");
        assert_eq!(sorted_by((Column::Name, false), Column::Name), (Column::Name, true));
        assert_eq!(sorted_by((Column::Name, false), Column::Changed), (Column::Changed, true), "the newest first");
        assert_eq!(sorted_by((Column::Changed, true), Column::Kind), (Column::Kind, false));
    }

    /// A film's state and its lines said, as the Preview tells them.
    #[test]
    fn a_film_is_told_by_its_problems_and_its_lines() {
        assert_eq!(state_said(0, 0).0, "no problems");
        assert_eq!(state_said(0, 2).0, "2 remarks");
        assert_eq!(state_said(1, 3).0, "1 error");
        assert_eq!(lines_said("pikachu: Hello!\npikachu (happy): Hi.\nwalk to left\n"), 2);
    }
}
