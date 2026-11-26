//! A shelf's things in a grid or a list, and a card's menu. Over them, the
//! tools, the head and the filters: `toolbar.rs`.

use eframe::egui::{self, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

use super::dropped::take_dropped;
use super::pictures::paint_item;
use super::toolbar;
use super::{Item, Resources, Shelf};
use workshop_base::theme;
use workshop_decor::Decors;

/// How the things lie: the width for them, a card's size, how many in a row, a row's height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Layout {
    pub wide: f32,
    pub cell: Vec2,
    pub columns: usize,
    pub row_high: f32,
}

impl Layout {
    /// The cards of that size in that width (a list: one in a row).
    pub(super) fn of(wide: f32, size: f32, listed: bool) -> Layout {
        let cell = vec2(size + 8.0, size + 24.0);
        let columns = if listed { 1 } else { ((wide + 10.0) / (cell.x + 10.0)).floor().max(1.0) as usize };
        let row_high = if listed { 30.0 } else { cell.y + 10.0 };
        Layout { wide, cell, columns, row_high }
    }

    /// Where the scroll goes to show the thing at that place.
    pub(super) fn offset_of(&self, at: usize) -> f32 {
        ((at / self.columns) as f32 * self.row_high - 60.0).max(0.0)
    }

    /// The thing the arrows go to, from that place, among so many.
    pub(super) fn after_arrows(&self, now: usize, count: usize, [left, right, up, down]: [bool; 4]) -> usize {
        let now = now as i64;
        let next = now + i64::from(right) - i64::from(left) + self.columns as i64 * (i64::from(down) - i64::from(up));
        next.clamp(0, count as i64 - 1) as usize
    }
}

/// What the grid's cards were done: one clicked (twice?), one's menu asked.
#[derive(Default)]
struct Picked {
    clicked: Option<(usize, bool)>,
    menu: Option<String>,
}

pub fn grid(ui: &mut egui::Ui, resources: &mut Resources, decors: &mut Decors) {
    take_dropped(resources, ui.ctx());
    resources.work_done();
    resources.thumbs.take(ui.ctx());
    let context = ui.ctx().clone();
    toolbar::tools(ui, resources);
    let searching = !resources.sought.trim().is_empty();
    if !searching {
        resources.everywhere = false;
    }
    // (The catalog's cards, its species and their forms; the songs' and the sounds' cards.)
    if !resources.listed && resources.shelf == Shelf::Pokemon && (!searching || resources.catalog_sought()) {
        pokemon_grid(ui, resources);
        return;
    }
    if !resources.listed && resources.shelf.heard() && !searching {
        let items = resources.shown(decors);
        toolbar::head(ui, resources, items.len());
        toolbar::filters(ui, resources);
        let chosen_at = items.iter().position(|item| Some(item.key()) == resources.chosen);
        let columns = ((ui.available_width() - 10.0) / (super::music_cards::CARD.x + 8.0)).floor().max(1.0) as usize;
        let scroll_to =
            resources.scroll_wanted.then(|| chosen_at.map_or(0.0, |at| ((at / columns) as f32 * (super::music_cards::CARD.y + 8.0) - 60.0).max(0.0)));
        resources.scroll_wanted = false;
        let did = super::music_cards::cards(ui, resources, &items, chosen_at, scroll_to);
        if let Some((at, _)) = did.clicked {
            // (Chosen without the Inspector's own listening: the card heard it already.)
            let item = &items[at];
            if resources.chosen.as_deref() != Some(item.key().as_str()) {
                resources.chosen = Some(item.key());
                (resources.step, resources.face, resources.watching, resources.body, resources.renaming, resources.trashing) = (0, 0, false, None, None, false);
            }
        }
        return;
    }
    // (Those open and not written yet: a • after their name, as on their tabs.)
    let unsaved = resources.unsaved_keys();
    let items: Vec<Item> = resources
        .shown(decors)
        .into_iter()
        .map(|mut item| {
            if unsaved.contains(&item.key()) {
                item.label.push_str(" •");
            }
            item
        })
        .collect();
    toolbar::head(ui, resources, items.len());
    toolbar::filters(ui, resources);
    let chosen_at = items.iter().position(|item| Some(item.key()) == resources.chosen);
    let layout = Layout::of(ui.available_width() - 16.0, resources.size, resources.listed);
    let picked = cards(ui, resources, decors, &context, &items, chosen_at, &layout);
    if items.is_empty() {
        let said = match resources.shelf {
            Shelf::Favourites => "no favourite yet: ★ in the Inspector, or right-click a resource",
            Shelf::Pieces => "the decors' elements are being read… (the Decor space loads them)",
            _ => "nothing here",
        };
        ui.label(theme::weak(said));
    }
    if let Some((at, twice)) = picked.clicked {
        let item = &items[at];
        let heard_before = resources.heard.clone();
        resources.choose(item);
        if twice && item.shelf.heard() && heard_before == resources.heard {
            resources.hear(item);
        } else if twice && let Some(line) = item.line() {
            ui.ctx().copy_text(line.clone());
            resources.said = Some(format!("copied: {line}"));
        }
    }
    if let Some(key) = picked.menu {
        resources.chosen = Some(key);
    }
    keys(ui, resources, &items, chosen_at, &layout);
}

/// The Pokémon shelf: the catalog's species (a generation, or what is sought: a name, a number, a type, `gen 8`),
/// their forms opened under them; a click chooses one (its page in the Inspector), a double click copies its word.
fn pokemon_grid(ui: &mut egui::Ui, resources: &mut Resources) {
    let catalog = resources.catalog();
    let sought = resources.sought.trim().to_string();
    let found = (!sought.is_empty()).then(|| catalog.search(&sought));
    let mut shown = super::pokemon_cards::shown(&catalog, found.as_deref(), |species, by_itself| resources.species_open(species, by_itself));
    if let Some(generation) = resources.filter.as_ref().and_then(|filter| filter.parse::<u8>().ok()).filter(|_| sought.is_empty()) {
        shown.retain(|one| catalog.species[one.species].generation == generation);
    }
    let count: usize = shown.iter().map(|one| one.entries.len()).sum();
    toolbar::head(ui, resources, count);
    toolbar::filters(ui, resources);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        let said = if sought.is_empty() {
            format!("{} species, {} Pokémon with their forms · search by name, number, type, generation (“fire gen 3”)", shown.len(), catalog.len())
        } else {
            format!("{} species found", shown.len())
        };
        ui.label(theme::text(said, theme::SMALL, theme::WEAK));
        if !sought.is_empty() && ui.small_button("Search All Shelves").on_hover_text("this search through every shelf: faces, music, things…").clicked() {
            resources.everywhere = true;
        }
    });
    let size = vec2((resources.size * 1.45).round(), resources.size + 46.0);
    let columns = ((ui.available_width() - 12.0) / (size.x + 8.0)).floor().max(1.0) as usize;
    let rows = super::pokemon_cards::rows(&shown, columns);
    let chosen_word = resources.chosen.as_deref().and_then(|key| key.strip_prefix("pokemon:")).map(str::to_string);
    let scroll_to = resources.scroll_wanted.then(|| {
        let row = chosen_word.as_ref().and_then(|word| {
            let (species, entry) = catalog.find(word)?;
            rows.iter().position(|row| row.cards.contains(&(species, entry)) || row.cards.contains(&(species, 0)))
        });
        row.map_or(0.0, |row| (row as f32 * (size.y + 8.0) - 60.0).max(0.0))
    });
    resources.scroll_wanted = false;
    if rows.is_empty() {
        ui.label(theme::weak("  no Pokémon by that name, number, type or generation"));
    }
    let did = super::pokemon_cards::grid(ui, resources, "resources pokemon", &catalog, &rows, size, chosen_word.as_deref(), scroll_to);
    if let Some(species) = did.toggled {
        let open = shown.iter().find(|one| one.species == species).is_some_and(|one| one.open);
        resources.toggle_species(&catalog.species[species], open);
    }
    let item_of = |(species, entry): (usize, usize)| {
        let (species, at) = (&catalog.species[species], entry);
        let entry = &species.entries[at];
        let about = super::catalog::about(species, entry, at);
        Item { shelf: Shelf::Pokemon, word: entry.word.clone(), label: entry.name.clone(), about, number: entry.number, file: None, order: 0 }
    };
    if let Some(at) = did.clicked.or(did.twice) {
        let item = item_of(at);
        resources.choose(&item);
        if did.twice.is_some() {
            if resources.import_if_missing(&item.word) {
                return;
            }
            ui.ctx().copy_text(item.word.clone());
            resources.said = Some(format!("copied: {} (type it in a film, or drag it from the Film's Assets)", item.word));
        }
    }
}

/// The things: a grid of their pictures, or a list; scrolled to the one
/// chosen when the arrows move or it is asked.
fn cards(
    ui: &mut egui::Ui,
    resources: &mut Resources,
    decors: &mut Decors,
    context: &egui::Context,
    items: &[Item],
    chosen_at: Option<usize>,
    layout: &Layout,
) -> Picked {
    let mut picked = Picked::default();
    let rows = items.len().div_ceil(layout.columns);
    let moved = resources.chosen.is_some()
        && ui.ui_contains_pointer()
        && ui.input(|input| input.key_pressed(egui::Key::ArrowDown) || input.key_pressed(egui::Key::ArrowUp));
    let mut area = egui::ScrollArea::vertical().id_salt(("resource grid", resources.shelf, resources.sought.is_empty())).auto_shrink(false);
    if (moved || resources.scroll_wanted)
        && let Some(at) = chosen_at
    {
        area = area.vertical_scroll_offset(layout.offset_of(at));
    } else if resources.scroll_wanted {
        area = area.vertical_scroll_offset(0.0);
    }
    resources.scroll_wanted = false;
    area.show_rows(ui, layout.row_high, rows, |ui, range| {
        for row in range {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 10.0;
                for (at, item) in items.iter().enumerate().take(((row + 1) * layout.columns).min(items.len())).skip(row * layout.columns) {
                    let chosen = Some(at) == chosen_at;
                    let response = if resources.listed {
                        list_row(ui, resources, decors, context, item, chosen, layout)
                    } else {
                        card(ui, resources, decors, context, item, chosen, layout)
                    };
                    if response.clicked() || response.double_clicked() {
                        picked.clicked = Some((at, response.double_clicked()));
                    }
                    if response.secondary_clicked() {
                        picked.menu = Some(item.key());
                    }
                    response.context_menu(|ui| card_menu(ui, resources, item));
                }
            });
            ui.add_space(layout.row_high - if resources.listed { 28.0 } else { layout.cell.y });
        }
    });
    picked
}

/// A thing as a row of the list: its picture small, its name, what it is, its shelf (sought everywhere), a star.
fn list_row(
    ui: &mut egui::Ui,
    resources: &mut Resources,
    decors: &mut Decors,
    context: &egui::Context,
    item: &Item,
    chosen: bool,
    layout: &Layout,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(layout.wide, layout.row_high - 2.0), Sense::click());
    if chosen {
        ui.painter().rect_filled(rect, 2.0, theme::CHOSEN);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 2.0, theme::HOVERED);
    }
    let thumb = Rect::from_min_size(rect.min + vec2(4.0, 2.0), vec2(rect.height() - 4.0, rect.height() - 4.0));
    ui.painter().rect_filled(thumb, 2.0, theme::GROUND);
    paint_item(ui.painter(), resources, decors, context, item, thumb, 0);
    ui.painter().text(
        pos2(thumb.max.x + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &item.label,
        FontId::proportional(theme::BODY),
        if chosen { theme::STRONG } else { theme::TEXT },
    );
    ui.painter().with_clip_rect(rect).text(
        pos2(rect.min.x + 260.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &item.about,
        FontId::proportional(theme::SMALL),
        theme::WEAK,
    );
    if !resources.sought.trim().is_empty() {
        ui.painter().text(
            rect.right_center() - vec2(30.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            item.shelf.path(),
            FontId::proportional(theme::SMALL),
            theme::FAINT,
        );
    }
    if resources.favourites.contains(&item.key()) {
        workshop_base::icons::star(ui.painter(), Rect::from_center_size(rect.right_center() - vec2(14.0, 0.0), vec2(12.0, 12.0)), true, theme::FILM);
    }
    response
}

/// A name cut to the room under a card (its • kept at the end).
pub(super) fn card_label(label: &str, room_for: usize) -> String {
    match label.strip_suffix(" •") {
        Some(name) => name.chars().take(room_for.saturating_sub(2)).chain(" •".chars()).collect(),
        None => label.chars().take(room_for).collect(),
    }
}

/// A thing as a card of the grid: its picture, chosen, a star, how many films use it, its name; in large when pointed.
fn card(
    ui: &mut egui::Ui,
    resources: &mut Resources,
    decors: &mut Decors,
    context: &egui::Context,
    item: &Item,
    chosen: bool,
    layout: &Layout,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(layout.cell, Sense::click());
    let room = Rect::from_min_size(rect.min + vec2(4.0, 0.0), vec2(resources.size, resources.size));
    ui.painter().rect_filled(room, 2.0, if response.hovered() { theme::HOVERED } else { theme::GROUND });
    // (The game's effects play over and over.)
    let sheet = resources.pictures.get(&item.key()).and_then(|sheet| sheet.as_ref());
    let step = match sheet.filter(|sheet| item.shelf.animated() && sheet.count > 1) {
        Some(sheet) => {
            context.request_repaint_after(std::time::Duration::from_millis(33));
            sheet.step_at(resources.began.elapsed().as_secs_f64())
        }
        None => 0,
    };
    paint_item(ui.painter(), resources, decors, context, item, room, step);
    if chosen {
        ui.painter().rect_stroke(room, 2.0, Stroke::new(2.0_f32, theme::ASSETS), StrokeKind::Outside);
    }
    if resources.favourites.contains(&item.key()) {
        workshop_base::icons::star(ui.painter(), Rect::from_min_size(room.min + vec2(4.0, 4.0), vec2(12.0, 12.0)), true, theme::FILM);
    }
    // (How many films use it, in a corner.)
    let using = resources.films_using(&item.word);
    if using > 0 && item.shelf != Shelf::Effects {
        let said = using.to_string();
        let pill = Rect::from_min_size(room.max - vec2(8.0 + 7.0 * said.len() as f32, 16.0), vec2(4.0 + 7.0 * said.len() as f32, 13.0));
        ui.painter().rect_filled(pill, 6.0, theme::ASSETS.gamma_multiply(0.85));
        ui.painter().text(pill.center(), egui::Align2::CENTER_CENTER, said, FontId::proportional(theme::SMALL - 2.0), theme::GROUND);
    }
    let label = card_label(&item.label, (resources.size / 6.5) as usize);
    ui.painter().text(
        pos2(rect.center().x, room.max.y + 4.0),
        egui::Align2::CENTER_TOP,
        label,
        FontId::proportional(theme::SMALL + 0.5),
        if chosen { theme::STRONG } else { theme::TEXT },
    );
    response.on_hover_ui(|ui| {
        // (In large, as it is pointed at.)
        let (look, _) = ui.allocate_exact_size(vec2(160.0, 160.0), Sense::hover());
        ui.painter().rect_filled(look, 2.0, theme::GROUND);
        paint_item(ui.painter(), resources, decors, context, item, look, step);
        ui.label(RichText::new(&item.label).strong().color(theme::STRONG));
        ui.label(theme::text(&item.about, theme::SMALL, theme::WEAK));
        if using > 0 {
            ui.label(theme::text(format!("in {using} film{}", if using > 1 { "s" } else { "" }), theme::SMALL, theme::ASSETS));
        }
        if !resources.sought.trim().is_empty() {
            ui.label(theme::text(item.shelf.path(), theme::SMALL, theme::FAINT));
        }
    })
}

/// The keys, the pointer over it: ← → ↑ ↓ another; Enter: heard, or its line copied.
fn keys(ui: &egui::Ui, resources: &mut Resources, items: &[Item], chosen_at: Option<usize>, layout: &Layout) {
    if !ui.ui_contains_pointer() || ui.ctx().wants_keyboard_input() || items.is_empty() {
        return;
    }
    let (left, right, up, down, enter) = ui.input(|input| {
        use egui::Key;
        (
            input.key_pressed(Key::ArrowLeft),
            input.key_pressed(Key::ArrowRight),
            input.key_pressed(Key::ArrowUp),
            input.key_pressed(Key::ArrowDown),
            input.key_pressed(Key::Enter),
        )
    });
    if left || right || up || down {
        let item = items[layout.after_arrows(chosen_at.unwrap_or(0), items.len(), [left, right, up, down])].clone();
        resources.choose(&item);
    }
    if enter && let Some(item) = chosen_at.map(|at| items[at].clone()) {
        if item.shelf.heard() {
            resources.hear(&item);
        } else if let Some(line) = item.line() {
            ui.ctx().copy_text(line.clone());
            resources.said = Some(format!("copied: {line}"));
        }
    }
}

/// A thing's menu, a right click on it.
fn card_menu(ui: &mut egui::Ui, resources: &mut Resources, item: &Item) {
    ui.label(theme::text(&item.label, theme::SMALL, theme::WEAK));
    if ui.button("Copy the Word").clicked() {
        ui.ctx().copy_text(item.word.clone());
        resources.said = Some(format!("copied: {}", item.word));
        ui.close();
    }
    if let Some(line) = item.line()
        && ui.button(format!("Copy “{line}”")).clicked()
    {
        ui.ctx().copy_text(line.clone());
        resources.said = Some(format!("copied: {line}"));
        ui.close();
    }
    let key = item.key();
    let said = if resources.favourites.contains(&key) { "Remove from Favourites" } else { "★ Add to Favourites" };
    if ui.button(said).clicked() {
        resources.favourite(&key);
        ui.close();
    }
    if item.shelf.heard() && ui.button("Listen").clicked() {
        resources.hear(item);
        ui.close();
    }
    if let Some(file) = &item.file
        && ui.button("Open to Paint").clicked()
    {
        resources.said = Some(opened(file));
        ui.close();
    }
}

/// A file opened in the program the system gives it (to paint a picture).
pub(super) fn opened(file: &std::path::Path) -> String {
    let made = if cfg!(windows) {
        std::process::Command::new("cmd").args(["/C", "start", ""]).arg(file).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(file).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(file).spawn()
    };
    match made {
        Ok(_) => format!("opened: {}", file.display()),
        Err(why) => why.to_string(),
    }
}
