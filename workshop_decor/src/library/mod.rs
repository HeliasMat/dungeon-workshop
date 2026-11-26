//! The Library, as a studio's browser of assets. At the top, always there,
//! one search (Ctrl+F or /) that looks everywhere at once: the elements,
//! the backdrops, the models, the project's decors, each kind in its own
//! group, the best first. On the left where one looks: this decor's place,
//! the favourites, those laid lately, all, each category with how many, the
//! models, the backdrops; the game's places grouped by kind, each its
//! picture. On the right what is there, in a grid of pictures or a list.
//!
//! An element's card: a click takes it as a stamp (each click on the scene
//! lays it, until Esc), a double-click or Enter lays it in the middle of the
//! scene, a drag lays it where it is let go (a ghost shows where); its star
//! keeps it among the favourites. Pointed at, it shows at its own size,
//! living. A backdrop's card lays the decor on it (`backdrops.rs`). The
//! cards: `cards.rs`; what is shown and found, decided without drawing: `search.rs`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use decor::tags::{CATEGORIES, Category, PlaceKind};
use eframe::egui::{self, Color32, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::edit;
use crate::terrains::Tool;
use crate::{Decors, paint_art};
use workshop_base::project::Entry;
use workshop_base::theme;

mod backdrops;
mod cards;
pub mod search;

pub use backdrops::{Asked as BackdropAsked, asked_text as backdrop_text, place_elements};
pub use search::title_of;
use search::{Filter, Found, Place, many_said, places, search_all, shown_of};

/// Where one looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Favourites,
    Recent,
    All,
    Category(Category),
    Place(i64),
    Models,
    Backdrops,
}

/// The most laid lately kept.
const MOST_RECENT: usize = 40;
/// The most cards shown at once (a search finds the others).
const MOST_SHOWN: usize = 600;
/// The most of each kind a search shows.
const MOST_FOUND: usize = 120;

/// Where the favourites and those laid lately are kept (on this machine, not in the project).
fn kept_file(decors: &Decors) -> PathBuf {
    decors.session.root().join("target").join("workshop").join("library.txt")
}

/// The favourites and those laid lately, read once.
pub fn read_kept(decors: &mut Decors) {
    if decors.lib.read {
        return;
    }
    decors.lib.read = true;
    let text = std::fs::read_to_string(kept_file(decors)).unwrap_or_default();
    for line in text.lines() {
        match line.split_once(' ') {
            Some(("favori", name)) => decors.lib.favourites.push(name.to_string()),
            Some(("recent", name)) => decors.lib.recent.push(name.to_string()),
            _ => {}
        }
    }
}

fn write_kept(decors: &Decors) {
    let mut text = String::new();
    for name in &decors.lib.favourites {
        text.push_str(&format!("favori {name}\n"));
    }
    for name in &decors.lib.recent {
        text.push_str(&format!("recent {name}\n"));
    }
    let path = kept_file(decors);
    if let Some(folder) = path.parent() {
        _ = std::fs::create_dir_all(folder);
    }
    _ = std::fs::write(path, text);
}

/// A piece laid from the Library: first among those laid lately.
pub fn note_laid(decors: &mut Decors, name: &str) {
    read_kept(decors);
    decors.lib.recent.retain(|known| known != name);
    decors.lib.recent.insert(0, name.to_string());
    decors.lib.recent.truncate(MOST_RECENT);
    write_kept(decors);
}

/// The Library's own state.
pub struct Library {
    pub source: Source,
    pub sought: String,
    /// The search asked to take the keys (Ctrl+F, /).
    pub focus: bool,
    /// The sizes shown: small, medium, large (`decor::tags::Size`).
    pub sizes: [bool; 3],
    pub list: bool,
    pub size: f32,
    pub chosen: Option<String>,
    /// The backdrop chosen (its place).
    pub backdrop: Option<i64>,
    pub favourites: Vec<String>,
    pub recent: Vec<String>,
    pub read: bool,
    pub places_sought: String,
    /// The kinds of places folded on the left.
    pub folded: HashSet<PlaceKind>,
    /// Each place's picture made small, read as its row is first seen (none:
    /// it has none); larger, for the backdrops' cards.
    pub thumbs: HashMap<i64, Option<egui::TextureHandle>>,
    pub pictures: HashMap<i64, Option<egui::TextureHandle>>,
    /// The decor it last opened on its place for (a decor opened: on "This Decor's Place", once).
    pub opened_on: Option<PathBuf>,
}

impl Default for Library {
    fn default() -> Library {
        Library {
            source: Source::All,
            sought: String::new(),
            focus: false,
            sizes: [true; 3],
            list: false,
            size: 88.0,
            chosen: None,
            backdrop: None,
            favourites: Vec::new(),
            recent: Vec::new(),
            read: false,
            places_sought: String::new(),
            folded: HashSet::new(),
            thumbs: HashMap::new(),
            pictures: HashMap::new(),
            opened_on: None,
        }
    }
}

impl Library {
    /// As the Workshop was launched (`WORKSHOP_LIBRARY=backdrops|models|search:<words>`, for a picture).
    pub fn asked(asked: Option<&str>) -> Library {
        let mut library = Library::default();
        match asked {
            Some("backdrops") => library.source = Source::Backdrops,
            Some("models") => library.source = Source::Models,
            Some(said) => library.sought = said.strip_prefix("search:").unwrap_or(said).to_string(),
            None => {}
        }
        library
    }
}

/// A category's colour, its mark on the left and on the cards.
fn colour_of(category: Category) -> Color32 {
    match category {
        Category::Nature => Color32::from_rgb(0x5C, 0xC9, 0x7B),
        Category::Water => Color32::from_rgb(0x38, 0xC6, 0xD9),
        Category::Buildings => Color32::from_rgb(0xE0, 0x8E, 0x5A),
        Category::Furniture => Color32::from_rgb(0xDC, 0xB6, 0x7A),
        Category::Animated => Color32::from_rgb(0x8A, 0xA4, 0xFF),
        Category::SceneObjects => theme::ASSETS,
        Category::Other => theme::WEAK,
    }
}

/// A row of the sources, on the left.
fn source_row(ui: &mut egui::Ui, chosen: bool, label: &str, count: Option<usize>, colour: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
    paint_row(ui, rect, &response, chosen);
    ui.painter().rect_filled(Rect::from_center_size(pos2(rect.min.x + 12.0, rect.center().y), vec2(7.0, 7.0)), 2.0, colour);
    row_text(ui, rect, 22.0, label, count, chosen);
    response
}

/// A row's ground: chosen, pointed at or not.
fn paint_row(ui: &egui::Ui, rect: Rect, response: &egui::Response, chosen: bool) {
    let fill = if chosen {
        cards::CHOSEN_FILL
    } else if response.hovered() {
        Color32::from_rgb(0x2A, 0x2D, 0x2E)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
}

/// A row's words from `left`, its count on the right.
fn row_text(ui: &egui::Ui, rect: Rect, left: f32, label: &str, count: Option<usize>, chosen: bool) {
    let clip = ui.painter().with_clip_rect(rect.shrink2(vec2(2.0, 0.0)));
    clip.text(
        pos2(rect.min.x + left, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(theme::BODY),
        if chosen { theme::STRONG } else { theme::TEXT },
    );
    if let Some(count) = count {
        clip.text(
            pos2(rect.max.x - 6.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            count.to_string(),
            egui::FontId::proportional(theme::SMALL),
            theme::FAINT,
        );
    }
}

/// What the cards did: one clicked (its stamp), one to lay in the middle, one's star clicked.
#[derive(Default)]
struct Picked {
    clicked: Option<String>,
    laid: Option<String>,
    star: Option<String>,
}

/// The place of the game the decor open is made from (its backdrop's, else its `level`), if its elements are in the Library.
fn own_place(decors: &Decors, places: &[Place]) -> Option<Place> {
    let open = decors.open.as_ref()?;
    let level = edit::setting(&open.text, "level").and_then(|level| level.trim().parse::<i64>().ok());
    // (Made like a place, laid on its backdrop, or numbered as one.)
    [crate::own_place(&open.text), level].into_iter().flatten().find_map(|level| places.iter().find(|(known, ..)| *known == level).cloned())
}

/// A decor newly open: the Library on "This Decor's Place" (if it has one,
/// and the Library is on All Elements, nothing sought), once for that decor.
fn opened_on_own_place(decors: &mut Decors, own: Option<&Place>) {
    let Some(path) = decors.open.as_ref().map(|open| open.path.clone()) else { return };
    if decors.lib.opened_on.as_ref() == Some(&path) {
        return;
    }
    decors.lib.opened_on = Some(path);
    if let Some((level, ..)) = own
        && decors.lib.source == Source::All
        && decors.lib.sought.is_empty()
    {
        decors.lib.source = Source::Place(*level);
    }
}

/// What the Library's parts asked, done once all is drawn.
#[derive(Default)]
struct Asked {
    picked: Picked,
    backdrops: backdrops::Done,
    model: Option<Vec<String>>,
    decor: Option<PathBuf>,
}

/// The Library: the search over it all; where one looks, on the left; what is there, or what was found, on the right.
pub fn library(ui: &mut egui::Ui, decors: &mut Decors, decor_files: &[Entry]) {
    read_kept(decors);
    if decors.pieces.is_empty() {
        ui.label(theme::weak("reading the pieces…"));
        return;
    }
    let pieces = decors.pieces.clone();
    let places = places(&pieces);
    let library = decors.session.library().clone();
    let models = decors.models.get_or_insert_with(|| edit::models(&library)).clone();
    let full = ui.available_rect_before_wrap();
    if ui.rect_contains_pointer(full) && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::F)) {
        decors.lib.focus = true;
    }
    // --- The search, and how it is shown, across the top ---
    let top = Rect::from_min_size(full.min, vec2(full.width(), 32.0));
    ui.painter().rect_filled(top, 0.0, theme::TOOLS);
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(top.shrink2(vec2(8.0, 4.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    search_bar(&mut bar, &mut decors.lib);
    let below = Rect::from_min_max(pos2(full.min.x, top.max.y), full.max);
    let side = 230.0_f32.min(below.width() * 0.3);
    let left = Rect::from_min_size(below.min, vec2(side, below.height()));
    let own = own_place(decors, &places);
    opened_on_own_place(decors, own.as_ref());
    sources(ui, decors, left, &pieces, &places, models.len(), own.as_ref());
    // --- What is there, on the right ---
    let right = Rect::from_min_max(pos2(left.max.x + 1.0, below.min.y + 4.0), below.max);
    let mut main = ui.new_child(egui::UiBuilder::new().max_rect(right.shrink2(vec2(8.0, 0.0))));
    let sought = decors.lib.sought.trim().to_string();
    let mut asked = Asked::default();
    let bottom = 20.0;
    let (total, count) = if sought.is_empty() {
        main.label(theme::text(where_said(decors.lib.source, &places), theme::BODY, theme::STRONG));
        main.add_space(4.0);
        let room = Rect::from_min_max(main.cursor().min, pos2(main.max_rect().max.x, main.max_rect().max.y - bottom));
        let mut grid = main.new_child(egui::UiBuilder::new().max_rect(room));
        shelf(&mut grid, decors, &pieces, &models, &mut asked)
    } else {
        let files: Vec<(String, String)> =
            decor_files.iter().map(|entry| (entry.title.clone(), entry.path.file_stem().unwrap_or_default().to_string_lossy().into_owned())).collect();
        let found = search_all(&sought, &pieces, decors.lib.sizes, &decors.backdrops, &models, &files);
        main.label(theme::text(format!("Everywhere · \u{201C}{sought}\u{201D}"), theme::BODY, theme::STRONG));
        main.add_space(4.0);
        let room = Rect::from_min_max(main.cursor().min, pos2(main.max_rect().max.x, main.max_rect().max.y - bottom));
        let mut grid = main.new_child(egui::UiBuilder::new().max_rect(room));
        scrolled(&mut grid, "found", |ui| found_groups(ui, decors, &found, &models, decor_files, &mut asked));
        (found.count(), found.count())
    };
    bottom_line(ui, decors, Rect::from_min_max(pos2(right.min.x + 8.0, right.max.y - bottom), right.max), total, count, &sought);
    ui.allocate_rect(full, Sense::hover());
    // (Enter, the pointer over the Library: the one chosen laid, or the backdrop chosen used.)
    if ui.rect_contains_pointer(full) && !ui.ctx().wants_keyboard_input() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
        match (decors.lib.source, decors.lib.backdrop) {
            (Source::Backdrops, Some(level)) => asked.backdrops.asked = Some(BackdropAsked::Use { level, resized: false, bare: false }),
            _ => asked.picked.laid = decors.lib.chosen.clone(),
        }
    }
    done(decors, asked);
}

/// What one source shows (no search): the models, the backdrops, or the
/// elements of the sizes asked; how many there are, how many shown.
fn shelf(
    grid: &mut egui::Ui,
    decors: &mut Decors,
    pieces: &[crate::workshop::PieceInfo],
    models: &[(String, Vec<String>)],
    asked: &mut Asked,
) -> (usize, usize) {
    let source = decors.lib.source;
    match source {
        Source::Models => {
            let all: Vec<usize> = (0..models.len()).collect();
            scrolled(grid, "models", |ui| {
                if models.is_empty() {
                    ui.label(theme::weak("No models: choose elements, then \"Save\" in the Inspector (Group and Model)."));
                }
                cards::model_cards(ui, models, &all, &mut asked.model);
            });
            (models.len(), models.len())
        }
        Source::Backdrops => {
            let shown = search::backdrops_of(&decors.backdrops, "");
            scrolled(grid, "backdrops", |ui| {
                if shown.is_empty() {
                    ui.label(theme::weak("No backdrops yet: the places' rests are made by `decor inventory` at extraction."));
                }
                backdrops::backdrop_cards(ui, decors, &shown, &mut asked.backdrops);
            });
            (shown.len(), shown.len())
        }
        _ => {
            let lib = &decors.lib;
            let mut shown = shown_of(pieces, &Filter { source, sought: "", sizes: lib.sizes, favourites: &lib.favourites, recent: &lib.recent });
            let total = shown.len();
            shown.truncate(MOST_SHOWN);
            scrolled(grid, "pieces", |ui| {
                if shown.is_empty() {
                    ui.add_space(8.0);
                    ui.label(theme::weak(match source {
                        Source::Favourites => "No favourites: a piece's star, when pointed at, puts it here.",
                        Source::Recent => "Nothing laid from the Library yet.",
                        _ => "Nothing here of the sizes shown.",
                    }));
                }
                cards::element_cards(ui, decors, &shown, &mut asked.picked);
            });
            (total, shown.len())
        }
    }
}

/// A part of the Library that scrolls, the whole room it has.
fn scrolled(ui: &mut egui::Ui, salt: &str, inside: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical().id_salt(salt).auto_shrink(false).show(ui, inside);
}

/// What the Library's parts asked, done: a star, a stamp, one laid in the
/// middle; a backdrop; a model pasted; a decor to open.
fn done(decors: &mut Decors, asked: Asked) {
    let picked = asked.picked;
    if let Some(name) = picked.star {
        starred(&mut decors.lib.favourites, name);
        write_kept(decors);
    }
    if let Some(name) = picked.clicked {
        decors.lib.chosen = Some(name.clone());
        if decors.brushes.tool == Tool::Strew {
            decors.brushes.strewn = Some(name);
        } else {
            // (A card clicked: its stamp in hand, the mouse choosing again.)
            decors.brushes.tool = Tool::Choose;
            decors.stamp = crate::placing::stamp_clicked(decors.stamp.as_ref(), &name);
        }
    }
    if let Some(name) = picked.laid {
        decors.stamp = None;
        lay(decors, &name);
    }
    backdrops::apply(decors, asked.backdrops);
    if let (Some(lines), Some(open)) = (asked.model, &mut decors.open) {
        let (made, chosen) = edit::pasted(&open.text, &lines, open.middle);
        open.change(made);
        open.choose(chosen);
    }
    if let Some(path) = asked.decor {
        decors.session.ask(workshop_base::ask::Ask::Decor { path, line: 0 });
    }
}

/// What was found, kind by kind: a head each (its name, how many), its cards.
fn found_groups(ui: &mut egui::Ui, decors: &mut Decors, found: &Found, models: &[(String, Vec<String>)], decor_files: &[Entry], asked: &mut Asked) {
    if found.count() == 0 {
        ui.add_space(8.0);
        ui.label(theme::weak("Nothing found: another word, a tag (tree, rock, roof…), a place's name; or the sizes shown, at the top."));
        return;
    }
    let head = |ui: &mut egui::Ui, word: &str, count: usize| {
        ui.add_space(4.0);
        let more = if count > MOST_FOUND { format!(" · the first {MOST_FOUND}") } else { String::new() };
        ui.label(theme::text(format!("{word} · {count}{more}"), theme::SMALL, theme::WEAK));
        ui.add_space(2.0);
    };
    if !found.elements.is_empty() {
        head(ui, "ELEMENTS", found.elements.len());
        cards::element_cards(ui, decors, &found.elements[..found.elements.len().min(MOST_FOUND)], &mut asked.picked);
    }
    if !found.backdrops.is_empty() {
        head(ui, "BACKDROPS", found.backdrops.len());
        backdrops::backdrop_cards(ui, decors, &found.backdrops[..found.backdrops.len().min(MOST_FOUND)], &mut asked.backdrops);
    }
    if !found.models.is_empty() {
        head(ui, "MODELS", found.models.len());
        cards::model_cards(ui, models, &found.models, &mut asked.model);
    }
    if !found.decors.is_empty() {
        head(ui, "DECORS", found.decors.len());
        for entry in found.decors.iter().filter_map(|at| decor_files.get(*at)) {
            let said = format!("{}   ·   {}", entry.title, entry.path.file_name().unwrap_or_default().to_string_lossy());
            if ui.selectable_label(false, said).on_hover_text("open this decor").clicked() {
                asked.decor = Some(entry.path.clone());
            }
        }
    }
}

/// The search across the top (a search icon, the field, a cross to empty
/// it), then how the cards are shown: their sizes, how large, grid or list.
fn search_bar(ui: &mut egui::Ui, lib: &mut Library) {
    let (icon, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    workshop_base::icons::search(ui.painter(), icon, theme::WEAK);
    let wide = (ui.available_width() - 400.0).clamp(160.0, 560.0);
    let field =
        ui.add(egui::TextEdit::singleline(&mut lib.sought).hint_text("Search elements, backdrops, models, decors…   (Ctrl+F or /)").desired_width(wide));
    if lib.focus {
        field.request_focus();
        lib.focus = false;
    }
    if field.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        lib.sought.clear();
        field.surrender_focus();
    }
    let field = field.on_hover_text("by name, tag (tree, rock, roof…), the place it comes from; every kind at once (Esc: empty)");
    if !lib.sought.is_empty() {
        let (cross, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        if workshop_base::icons::button(ui, cross, "empty the search (Esc)", |painter, at, tint| workshop_base::icons::cross(painter, at.shrink(3.0), tint)) {
            lib.sought.clear();
            field.request_focus();
        }
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| shown_as(ui, lib));
}

/// The bottom line: what is chosen and what it does; how many there are.
fn bottom_line(ui: &egui::Ui, decors: &Decors, line: Rect, total: usize, shown: usize, sought: &str) {
    let said = match (&decors.stamp, decors.lib.source, decors.lib.backdrop, decors.lib.chosen.as_ref()) {
        (Some(stamp), ..) => format!("Stamp: {}  —  each click on the scene lays it · Esc stops", decors.shown_name(&stamp.name)),
        (None, Source::Backdrops, Some(level), _) => format!(
            "{}  —  Enter or double-click: the decor laid on it · right click: its size, its elements",
            decors.backdrops.iter().find(|one| one.level == level).map_or(format!("place {level}"), |one| one.place.clone())
        ),
        (None, Source::Backdrops, None, _) => "double-click a backdrop, or drag it onto the scene: the decor laid on it (Ctrl+Z undoes it)".to_string(),
        (None, _, _, Some(name)) => format!("{} · {}  —  click: a stamp · Enter: in the middle · drag: where dropped", decors.shown_name(name), name),
        _ => "click a card: a stamp, laid by clicks on the scene · double-click: in the middle · or drag it onto the scene".to_string(),
    };
    let colour = if decors.stamp.is_some() { theme::DECOR } else { theme::WEAK };
    let clip = ui.painter().with_clip_rect(line);
    clip.text(line.left_center(), egui::Align2::LEFT_CENTER, said, egui::FontId::proportional(theme::SMALL), colour);
    let counted = if !sought.is_empty() {
        format!("{total} found")
    } else if decors.lib.source == Source::Backdrops {
        format!("{total} backdrop(s)")
    } else {
        many_said(total, shown)
    };
    clip.text(line.right_center() - vec2(8.0, 0.0), egui::Align2::RIGHT_CENTER, counted, egui::FontId::proportional(theme::SMALL), theme::FAINT);
}

/// Where one looks, on the left: this decor's place, the favourites, those
/// laid lately, all, each category, the models, the backdrops; the game's
/// places (filtered), grouped by kind.
fn sources(ui: &mut egui::Ui, decors: &mut Decors, left: Rect, pieces: &[crate::workshop::PieceInfo], places: &[Place], models: usize, own: Option<&Place>) {
    ui.painter().rect_filled(left, 0.0, theme::TABS);
    let mut sources = ui.new_child(egui::UiBuilder::new().max_rect(left.shrink2(vec2(0.0, 4.0))));
    sources.spacing_mut().item_spacing.y = 0.0;
    let counts = decor::tags::counts(pieces.iter().map(|piece| piece.category));
    let data = decors.session.data().to_string();
    let backdrops = decors.backdrops.len();
    egui::ScrollArea::vertical().id_salt("sources").auto_shrink(false).show(&mut sources, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let lib = &mut decors.lib;
        let searching = !lib.sought.trim().is_empty();
        if let Some((level, name, _, count)) = own {
            let response =
                source_row(ui, lib.source == Source::Place(*level) && !searching, &format!("This Decor's Place · {name}"), Some(*count), theme::DECOR);
            if response.on_hover_text("the elements the game lays on the place this decor is made from").clicked() {
                (lib.source, lib.sought) = (Source::Place(*level), String::new());
            }
            ui.add_space(5.0);
        }
        let mut rows = vec![
            (Source::Favourites, "Favourites".to_string(), lib.favourites.len(), Color32::from_rgb(0xF6, 0xD3, 0x3C)),
            (Source::Recent, "Recently Laid".to_string(), lib.recent.len(), theme::WEAK),
            (Source::All, "All Elements".to_string(), pieces.len(), theme::DECOR),
            (Source::Backdrops, "Backdrops".to_string(), backdrops, Color32::from_rgb(0xC8, 0x9B, 0x5A)),
            (Source::Models, "Models".to_string(), models, Color32::from_rgb(0xDC, 0xB6, 0x7A)),
        ];
        rows.extend(
            CATEGORIES.iter().map(|(category, _, shown)| (Source::Category(*category), shown.to_string(), counts[category.index()], colour_of(*category))),
        );
        for (at, (source, label, count, colour)) in rows.into_iter().enumerate() {
            // (Room between the shelves of one's own, the kinds of things, the categories.)
            if at == 3 || at == 5 {
                ui.add_space(5.0);
            }
            let response = source_row(ui, lib.source == source && !searching, &label, Some(count), colour);
            let response = match source {
                Source::Backdrops => response.on_hover_text("places of the game without their elements, to lay the decor on"),
                Source::Models => response.on_hover_text("groups of lines kept to lay again (Inspector › Group and Model)"),
                _ => response,
            };
            if response.clicked() {
                // (A source chosen: what was sought let go, to see it all.)
                (lib.source, lib.sought) = (source, String::new());
                if source == Source::Models {
                    decors.models = None;
                }
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(theme::text("THE GAME'S PLACES", theme::SMALL, theme::WEAK));
        });
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.add(egui::TextEdit::singleline(&mut decors.lib.places_sought).hint_text("filter the places").desired_width(ui.available_width() - 8.0));
        });
        ui.add_space(3.0);
        place_rows(ui, decors, places, &data);
    });
}

/// The game's places on the left, by kind (each kind folds), each its picture, its name, how many elements.
fn place_rows(ui: &mut egui::Ui, decors: &mut Decors, places: &[Place], data: &str) {
    let wanted = decors.lib.places_sought.to_lowercase();
    let matching: Vec<&Place> =
        places.iter().filter(|(level, name, ..)| wanted.is_empty() || name.to_lowercase().contains(&wanted) || level.to_string() == wanted).collect();
    let mut loaded = 0;
    for (kind, group) in decor::tags::grouped(matching, |place| place.2) {
        let folded = decors.lib.folded.contains(&kind) && wanted.is_empty();
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::click());
        paint_row(ui, rect, &response, false);
        let mark = if folded { "▸" } else { "▾" };
        let small = egui::FontId::proportional(theme::SMALL);
        ui.painter().text(rect.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, mark, small.clone(), theme::WEAK);
        ui.painter().text(rect.left_center() + vec2(20.0, 0.0), egui::Align2::LEFT_CENTER, kind.shown().to_uppercase(), small.clone(), theme::WEAK);
        ui.painter().text(rect.right_center() - vec2(6.0, 0.0), egui::Align2::RIGHT_CENTER, group.len().to_string(), small, theme::FAINT);
        if response.on_hover_text(if folded { "unfold" } else { "fold" }).clicked() && !decors.lib.folded.remove(&kind) {
            decors.lib.folded.insert(kind);
        }
        if folded {
            continue;
        }
        for (level, name, _, count) in group {
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
            let chosen = decors.lib.source == Source::Place(*level) && decors.lib.sought.trim().is_empty();
            paint_row(ui, rect, &response, chosen);
            if ui.is_rect_visible(rect) {
                let thumb = Rect::from_min_size(rect.min + vec2(8.0, 2.0), vec2(36.0, 24.0));
                ui.painter().rect_filled(thumb, 2.0, theme::GROUND);
                if !decors.lib.thumbs.contains_key(level) && loaded < 4 {
                    loaded += 1;
                    let made = place_thumb(ui.ctx(), data, *level, 96);
                    decors.lib.thumbs.insert(*level, made);
                }
                if let Some(Some(texture)) = decors.lib.thumbs.get(level) {
                    paint_filled(ui.painter(), texture, thumb);
                } else if !decors.lib.thumbs.contains_key(level) {
                    ui.ctx().request_repaint();
                }
                row_text(ui, rect, 52.0, name, Some(*count), chosen);
            }
            if response.on_hover_text(format!("the elements of {name} (place {level})")).clicked() {
                (decors.lib.source, decors.lib.sought) = (Source::Place(*level), String::new());
            }
        }
    }
}

/// A place of the game's picture made small (`backdrops/<level>.png`), that wide; none if it has none.
pub fn place_thumb(context: &egui::Context, data: &str, level: i64, wide: u32) -> Option<egui::TextureHandle> {
    let picture = image::open(assets::data::path(data, &format!("backdrops/{level}.png"))).ok()?.to_rgba8();
    let small = image::imageops::thumbnail(&picture, wide, (wide * picture.height() / picture.width().max(1)).max(1));
    let image = egui::ColorImage::from_rgba_unmultiplied([small.width() as usize, small.height() as usize], small.as_raw());
    Some(context.load_texture(format!("place thumb {level} {wide}"), image, egui::TextureOptions::LINEAR))
}

/// A picture filling a room, its middle kept.
pub fn paint_filled(painter: &egui::Painter, texture: &egui::TextureHandle, room: Rect) {
    let size = texture.size_vec2();
    let scale = (room.width() / size.x).max(room.height() / size.y);
    let seen = vec2(room.width() / (size.x * scale), room.height() / (size.y * scale));
    painter.image(texture.id(), room, Rect::from_center_size(pos2(0.5, 0.5), seen), Color32::WHITE);
}

/// Where one is, said over the pieces.
fn where_said(source: Source, places: &[Place]) -> String {
    match source {
        Source::Favourites => "Favourites".to_string(),
        Source::Recent => "Recently Laid".to_string(),
        Source::All => "All Elements".to_string(),
        Source::Category(category) => category.shown().to_string(),
        Source::Place(level) => match places.iter().find(|(known, ..)| *known == level) {
            Some((_, name, kind, _)) => format!("The Game's Places › {} › {name}", kind.shown()),
            None => format!("The Game's Places › place {level}"),
        },
        Source::Models => "Models".to_string(),
        Source::Backdrops => "Backdrops".to_string(),
    }
}

/// How they are shown: a list or a grid (how large), the sizes.
fn shown_as(ui: &mut egui::Ui, lib: &mut Library) {
    if ui.selectable_label(lib.list, "List").on_hover_text("the cards as rows: name, place, size").clicked() {
        lib.list = true;
    }
    if ui.selectable_label(!lib.list, "Grid").on_hover_text("the cards as pictures").clicked() {
        lib.list = false;
    }
    if !lib.list {
        ui.add(egui::Slider::new(&mut lib.size, 56.0..=180.0).show_value(false)).on_hover_text("how large the pictures are");
    }
    ui.separator();
    for (at, word, told) in [(2, "Large", "over 96 pixels"), (1, "Medium", "33 to 96 pixels"), (0, "Small", "up to 32 pixels")] {
        if ui.selectable_label(lib.sizes[at], word).on_hover_text(format!("the elements {told}: shown or not")).clicked() {
            lib.sizes[at] = !lib.sizes[at];
        }
    }
}

/// A favourite no longer, or one more.
fn starred(favourites: &mut Vec<String>, name: String) {
    if let Some(at) = favourites.iter().position(|known| *known == name) {
        favourites.remove(at);
    } else {
        favourites.push(name);
    }
}

/// A piece laid in the middle of what the scene shows (on the grid if snapping), chosen there.
fn lay(decors: &mut Decors, name: &str) {
    let Some(open) = &decors.open else { return };
    let step = crate::placing::grid_step(&open.text);
    let corner = crate::placing::landing(open.middle, decors.size_of(name), step, decors.snap);
    let line = decors.pose_line(name, &format!("{} {}", corner[0], corner[1]));
    note_laid(decors, name);
    lay_line(decors, &line);
}

/// A line laid in the decor open (one step), what it lays chosen.
pub fn lay_line(decors: &mut Decors, line: &str) {
    let Some(open) = &mut decors.open else { return };
    let made = open.with_added(line);
    open.change(made);
    let laid = edit::items(&open.text).into_iter().rev().find(|one| open.text.lines().nth(one.line - 1).is_some_and(|known| known.trim() == line.trim()));
    open.choose(laid.map(|one| vec![(one.line, one.at)]).unwrap_or_default());
}

/// A piece seen through where it would land, outlined, its corner said.
pub fn paint_ghost(decors: &mut Decors, painter: &egui::Painter, name: &str, corner: [f32; 2], to_screen: &dyn Fn([f32; 2]) -> egui::Pos2, said: &str) {
    let [w, h] = decors.size_of(name);
    let rect = Rect::from_min_max(to_screen(corner), to_screen([corner[0] + w, corner[1] + h]));
    if let Some(art) = decors.art_now(name) {
        paint_art(painter, &art, rect, (true, true), 0, false, Color32::from_white_alpha(170));
    }
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, theme::DECOR), StrokeKind::Outside);
    crate::tag(painter, rect.left_top() - vec2(0.0, 4.0), &format!("{said}{} {}", corner[0], corner[1]), Color32::WHITE);
}

/// While a card is dragged over the scene: where it would land, seen through
/// (on the grid if snapping); a backdrop: what dropping it does; a model: its name.
pub fn ghost(
    decors: &mut Decors,
    painter: &egui::Painter,
    response: &egui::Response,
    to_screen: &dyn Fn([f32; 2]) -> egui::Pos2,
    place_at: [f32; 2],
    step: f32,
    snap: bool,
) {
    let Some(name) = response.dnd_hover_payload::<String>() else { return };
    let Some(at) = response.hover_pos() else { return };
    if let Some(model) = name.strip_prefix("model:") {
        crate::tag(painter, at + vec2(10.0, 0.0), &format!("model {model}"), theme::DECOR);
        return;
    }
    if let Some(level) = name.strip_prefix("backdrop:").and_then(|level| level.parse::<i64>().ok()) {
        let said = decors.backdrops.iter().find(|one| one.level == level).map_or(format!("place {level}"), |one| one.place.clone());
        crate::tag(painter, at + vec2(12.0, 0.0), &format!("let go: the decor laid on {said}"), theme::DECOR);
        return;
    }
    let corner = crate::placing::landing(place_at, decors.size_of(&name), step, snap);
    paint_ghost(decors, painter, &name, corner, to_screen, "");
}
