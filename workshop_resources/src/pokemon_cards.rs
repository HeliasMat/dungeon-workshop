//! The catalog's cards, as the Resources' Pokémon shelf and the Film's Assets
//! panel show them: a species' card (its portrait, its body standing, its
//! name, its word, its Pokédex number, its forms folded or open, a badge
//! when it is the community's), its forms after it on a band of their own;
//! laid out in rows of so many, only the rows in sight drawn.

use std::collections::HashSet;
use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use super::catalog::{Catalog, Entry, Found, Source, Species};
use super::{Resources, Sheet};
use workshop_base::theme;
use workshop_picture::fitted;

/// What is shown of a species: which, its entries shown (the species itself first), whether its forms are open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub species: usize,
    pub entries: Vec<usize>,
    pub open: bool,
}

/// A row of cards: each its species and its entry; whether it is a band of open forms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub cards: Vec<(usize, usize)>,
    pub band: bool,
}

/// The species shown, as sought: all of them (their forms open if asked), or
/// those found (the forms found shown; all of a species found, open when few
/// species are, or asked). `open(species, by_itself)`: whether its forms are open.
pub fn shown(catalog: &Catalog, found: Option<&[Found]>, open: impl Fn(&Species, bool) -> bool) -> Vec<Shown> {
    match found {
        None => catalog
            .species
            .iter()
            .enumerate()
            .map(|(at, species)| {
                let open = species.forms() > 0 && open(species, false);
                let entries = if open { (0..species.entries.len()).collect() } else { vec![0] };
                Shown { species: at, entries, open }
            })
            .collect(),
        Some(found) => {
            let few = found.len() <= 4;
            found
                .iter()
                .map(|found| {
                    let species = &catalog.species[found.species];
                    let open = species.forms() > 0 && open(species, few || !found.whole);
                    let entries = match (open, found.whole) {
                        (false, _) => vec![0],
                        (true, true) => (0..species.entries.len()).collect(),
                        (true, false) => std::iter::once(0).chain(found.entries.iter().copied().filter(|entry| *entry != 0)).collect(),
                    };
                    Shown { species: found.species, entries, open }
                })
                .collect()
        }
    }
}

/// The rows of cards: the species folded side by side; one open starts a
/// row of its own, its forms after it, and the next one starts the next row.
pub fn rows(shown: &[Shown], columns: usize) -> Vec<Row> {
    let columns = columns.max(1);
    let mut rows: Vec<Row> = Vec::new();
    let mut packed: Vec<(usize, usize)> = Vec::new();
    for one in shown {
        if one.entries.len() > 1 {
            if !packed.is_empty() {
                rows.push(Row { cards: std::mem::take(&mut packed), band: false });
            }
            for chunk in one.entries.chunks(columns) {
                rows.push(Row { cards: chunk.iter().map(|entry| (one.species, *entry)).collect(), band: true });
            }
            continue;
        }
        packed.push((one.species, one.entries.first().copied().unwrap_or(0)));
        if packed.len() == columns {
            rows.push(Row { cards: std::mem::take(&mut packed), band: false });
        }
    }
    if !packed.is_empty() {
        rows.push(Row { cards: packed, band: false });
    }
    rows
}

/// What a card's pointer did: chosen, written, dragged, its forms opened or shut, imported.
#[derive(Default)]
pub struct Did {
    pub clicked: Option<(usize, usize)>,
    pub twice: Option<(usize, usize)>,
    pub dragged: Option<(usize, usize)>,
    pub toggled: Option<usize>,
    pub hovered: bool,
}

/// The cards in rows, only those in sight drawn; `chosen`: the word chosen.
pub fn grid(
    ui: &mut egui::Ui,
    resources: &mut Resources,
    id: &str,
    catalog: &Arc<Catalog>,
    rows: &[Row],
    size: egui::Vec2,
    chosen: Option<&str>,
    scroll_to: Option<f32>,
) -> Did {
    let mut did = Did::default();
    let row_high = size.y + 8.0;
    let mut area = egui::ScrollArea::vertical().id_salt(id).auto_shrink(false);
    if let Some(offset) = scroll_to {
        area = area.vertical_scroll_offset(offset);
    }
    // (Rows as high as said: no room between them but their own.)
    ui.spacing_mut().item_spacing.y = 0.0;
    area.show_rows(ui, row_high, rows.len(), |ui, range| {
        for row in &rows[range] {
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_high), Sense::hover());
            if row.band {
                ui.painter().rect_filled(line.shrink2(vec2(2.0, 0.0)), 0.0, theme::TOOLS);
            }
            for (at, (species, entry)) in row.cards.iter().enumerate() {
                let rect = Rect::from_min_size(line.min + vec2(6.0 + at as f32 * (size.x + 8.0), 4.0), size);
                let species_said = &catalog.species[*species];
                let one = &species_said.entries[*entry];
                let response = ui.interact(rect, ui.id().with((id, species, entry)), Sense::click_and_drag());
                let is_chosen = chosen == Some(one.word.as_str());
                let toggle = card(ui, resources, rect, species_said, one, *entry, &response, is_chosen, row.band);
                did.hovered |= response.hovered();
                if toggle.is_some_and(|chip| response.clicked() && response.interact_pointer_pos().is_some_and(|at| chip.contains(at))) {
                    did.toggled = Some(*species);
                } else if response.double_clicked() {
                    did.twice = Some((*species, *entry));
                } else if response.clicked() {
                    did.clicked = Some((*species, *entry));
                }
                if response.drag_started() {
                    did.dragged = Some((*species, *entry));
                }
                let hint = hover_said(species_said, one);
                let _ = response.on_hover_text(hint);
            }
        }
    });
    did
}

/// What a card's hover says.
fn hover_said(species: &Species, entry: &Entry) -> String {
    let mut said = format!("{}\nin a film: {}", entry.name, entry.word);
    if species.dex > 0 {
        said.push_str(&format!("\nPokédex No. {} · Gen. {}", species.dex, species.generation));
    }
    if !species.types.is_empty() {
        said.push_str(&format!(" · {}", species.types_said()));
    }
    if entry.source == Source::Missing {
        said.push_str("\non SpriteCollab, not imported yet: imported (in the background) when first used");
    } else if let Some(badge) = entry.source.badge() {
        said.push_str(&format!("\n{badge}"));
    }
    said
}

/// A text on one line at most, cut with … to that width.
fn one_line(painter: &egui::Painter, said: &str, font: FontId, colour: Color32, wide: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(said.to_string(), font, colour);
    job.wrap = egui::text::TextWrapping { max_width: wide, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    painter.layout_job(job)
}

/// A card: its pictures, its name, its word, its number, its badge, its forms' chip (where it is, if it has one).
#[allow(clippy::too_many_arguments)]
pub fn card(
    ui: &egui::Ui,
    resources: &mut Resources,
    rect: Rect,
    species: &Species,
    entry: &Entry,
    at: usize,
    response: &egui::Response,
    chosen: bool,
    open: bool,
) -> Option<Rect> {
    if !ui.is_rect_visible(rect) {
        return None;
    }
    let painter = ui.painter_at(rect.expand(1.0));
    let hovered = response.hovered() || response.dragged();
    let fill = if chosen {
        theme::CHOSEN
    } else if hovered {
        theme::HOVERED
    } else {
        theme::GROUND
    };
    painter.rect_filled(rect, 4.0, fill);
    // (The pictures: the portrait on the left, the body standing on the right, on the floor.)
    let pictures = Rect::from_min_max(rect.min + vec2(6.0, 18.0), pos2(rect.max.x - 6.0, rect.max.y - 34.0));
    let face_side = pictures.height().min(pictures.width() * 0.58);
    let face_room = Rect::from_min_size(pos2(pictures.min.x, pictures.center().y - face_side / 2.0), vec2(face_side, face_side));
    let body_room = Rect::from_min_max(pos2(face_room.max.x + 2.0, pictures.min.y), pictures.max);
    paint_pictures(ui, resources, entry, face_room, body_room, hovered);
    // (Its number, its forms, its badge.)
    if species.dex > 0 && at == 0 {
        painter.text(rect.min + vec2(6.0, 3.0), egui::Align2::LEFT_TOP, format!("#{:04}", species.dex), FontId::monospace(theme::SMALL - 1.5), theme::FAINT);
    }
    let chip = (at == 0 && species.forms() > 0).then(|| {
        let said = format!("{} form{}", species.forms(), if species.forms() > 1 { "s" } else { "" });
        let galley = painter.layout_no_wrap(said, FontId::proportional(theme::SMALL - 1.0), theme::STRONG);
        let chip = Rect::from_min_size(pos2(rect.max.x - galley.size().x - 22.0, rect.min.y + 3.0), galley.size() + vec2(18.0, 2.0));
        let pointed = response.hover_pos().is_some_and(|at| chip.contains(at));
        painter.rect_filled(chip, 6.0, if pointed { theme::ASSETS } else { theme::ASSETS.gamma_multiply(0.55) });
        workshop_base::icons::chevron(&painter, pos2(chip.min.x + 8.0, chip.center().y), open, theme::STRONG);
        painter.galley(chip.min + vec2(14.0, 1.0), galley, theme::STRONG);
        chip
    });
    if let Some(badge) = entry.source.badge().filter(|_| at != 0 || entry.source == Source::Missing || species.forms() == 0) {
        let colour = if entry.source == Source::Missing { theme::FILM } else { theme::DECOR };
        let galley = one_line(&painter, badge, FontId::proportional(theme::SMALL - 2.0), theme::GROUND, rect.width() - 16.0);
        let at = if chip.is_some() || at != 0 { pos2(rect.min.x + 6.0, rect.min.y + 3.0) } else { pos2(rect.max.x - galley.size().x - 12.0, rect.min.y + 3.0) };
        let pill = Rect::from_min_size(at, galley.size() + vec2(8.0, 2.0));
        painter.rect_filled(pill, 6.0, colour.gamma_multiply(0.85));
        painter.galley(pill.min + vec2(4.0, 1.0), galley, theme::GROUND);
    }
    // (Its name, what a film writes.)
    let name = if at == 0 { species.name.clone() } else { entry.form.clone() };
    let galley = one_line(&painter, &name, FontId::proportional(theme::BODY), if chosen { theme::STRONG } else { theme::TEXT }, rect.width() - 8.0);
    painter.galley(pos2(rect.center().x - galley.size().x / 2.0, rect.max.y - 31.0), galley, theme::STRONG);
    let word = one_line(&painter, &entry.word, FontId::monospace(theme::SMALL - 1.0), Color32::from_rgb(0xDC, 0xDC, 0xAA), rect.width() - 8.0);
    painter.galley(pos2(rect.center().x - word.size().x / 2.0, rect.max.y - 15.0), word, theme::WEAK);
    if chosen {
        painter.rect_stroke(rect, 4.0, Stroke::new(2.0_f32, theme::ASSETS), StrokeKind::Inside);
    }
    chip
}

/// A card's portrait and body: as the thread made them; the body playing its idle while pointed.
fn paint_pictures(ui: &egui::Ui, resources: &mut Resources, entry: &Entry, face_room: Rect, body_room: Rect, hovered: bool) {
    let painter = ui.painter();
    if entry.number < 0 {
        let first = entry.name.chars().next().map(|letter| letter.to_string()).unwrap_or_default();
        painter.text(face_room.center(), egui::Align2::CENTER_CENTER, first, FontId::proportional(face_room.height() * 0.5), theme::FAINT);
        painter.text(
            body_room.center(),
            egui::Align2::CENTER_CENTER,
            "imported\nwhen used",
            FontId::proportional(theme::SMALL - 1.5),
            theme::FILM.gamma_multiply(0.8),
        );
        return;
    }
    let session = resources.session.clone();
    let thumb = resources.thumbs.of(&session, entry.number).cloned();
    let Some(thumb) = thumb else {
        painter.text(face_room.center(), egui::Align2::CENTER_CENTER, "…", FontId::proportional(theme::SMALL), theme::FAINT);
        return;
    };
    let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    if let Some(face) = &thumb.face {
        painter.image(face.id(), fitted(face.size_vec2(), face_room), whole, Color32::WHITE);
    }
    let Some(body) = &thumb.body else { return };
    let size = body.size_vec2();
    // (Grown by whole times, standing on the floor of its room.)
    let times = (body_room.width() / size.x).min(body_room.height() / size.y).floor().clamp(1.0, 4.0);
    let feet = pos2(body_room.center().x, body_room.max.y);
    let still = Rect::from_min_size(pos2(feet.x - size.x * times / 2.0, feet.y - size.y * times), size * times);
    match hovered.then(|| playing(ui, resources, entry.number)).flatten() {
        Some((sheet, step)) => {
            // (The whole cell, grown as much, its middle where the still one's is.)
            let cell = sheet.cell * times;
            let shown = Rect::from_center_size(still.center(), cell);
            ui.painter().with_clip_rect(body_room.expand2(vec2(2.0, 8.0))).image(sheet.texture.id(), shown, sheet.uv_at(step, 0), Color32::WHITE);
        }
        None => {
            painter.image(body.id(), still, whole, Color32::WHITE);
        }
    }
}

/// A body's idle playing (its sheet read once), its step now.
fn playing(ui: &egui::Ui, resources: &mut Resources, number: i64) -> Option<(Sheet, usize)> {
    let sprites = super::pictures::sprite_sheets(&resources.session);
    let file = [1_u8, 0].into_iter().find(|file| sprites.contains_key(&(u16::try_from(number).unwrap_or(0), *file)))?;
    let key = format!("body:{number}:{file}");
    if !resources.pictures.contains_key(&key) {
        let made = super::pictures::body_sheet(&resources.session, ui.ctx(), number, file);
        resources.pictures.insert(key.clone(), made);
    }
    let sheet = resources.pictures.get(&key).cloned().flatten()?;
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    let step = sheet.step_at(resources.began.elapsed().as_secs_f64());
    Some((sheet, step))
}

/// The words of the species open (their forms shown): `open` asked, `shut` asked; else as the search would have it.
pub fn is_open(open: &HashSet<String>, shut: &HashSet<String>, species: &Species, by_itself: bool) -> bool {
    let word = species.entries.first().map(|entry| entry.word.as_str()).unwrap_or_default();
    if shut.contains(word) {
        return false;
    }
    open.contains(word) || by_itself
}
