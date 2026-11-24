//! What the one chosen is, how a film (or a decor) writes it, where it is used.

use eframe::egui::{self, Color32, RichText, vec2};

use super::super::{Item, Resources, Shelf};
use super::{Shown, plural, section};
use workshop_base::ask::Ask;
use workshop_base::theme;
use workshop_script::Scripts;

/// What a shelf's thing is called, over what it is.
pub(super) fn kind_name(shelf: Shelf) -> &'static str {
    match shelf {
        Shelf::Pokemon | Shelf::Variants => "Character",
        Shelf::Songs => "Music",
        Shelf::Sounds => "Sound",
        Shelf::Portraits => "Portrait",
        Shelf::Items => "Item",
        Shelf::Attacks => "Attack",
        Shelf::Statuses => "Status Effect",
        Shelf::Weather => "Weather",
        Shelf::Effects => "Effect",
        Shelf::Backdrops => "Backdrop",
        Shelf::Tilesets => "Tileset",
        Shelf::Pieces => "Decor Element",
        Shelf::Dungeons => "Dungeon",
        Shelf::Bosses => "Boss Fight",
        _ => "Object",
    }
}

/// What it is: its word, its name, its size, its images, its file.
pub(super) fn what_it_is(ui: &mut egui::Ui, item: &Item, shown: &Shown) {
    section(ui, kind_name(item.shelf));
    egui::Grid::new("what it is").num_columns(2).spacing(vec2(10.0, 6.0)).min_col_width(70.0).show(ui, |ui| {
        let field = |ui: &mut egui::Ui, name: &str, value: String, mono: bool| {
            ui.label(theme::text(name, theme::SMALL, theme::WEAK));
            let text = if mono { RichText::new(value).monospace().size(theme::SMALL) } else { RichText::new(value).size(theme::SMALL) };
            ui.add(egui::Label::new(text.color(theme::TEXT)).selectable(true));
            ui.end_row();
        };
        field(ui, "Word", item.word.clone(), true);
        if item.label != item.word {
            field(ui, "Name", item.label.clone(), false);
        }
        field(ui, "It Is", item.about.clone(), false);
        if let Some(sheet) = &shown.sheet {
            field(ui, "Size", format!("{} × {}", sheet.cell.x, sheet.cell.y), true);
            if sheet.count > 1 {
                let seconds = sheet.ticks.iter().take(sheet.count).map(|ticks| f64::from(*ticks)).sum::<f64>() / 60.0;
                field(ui, "Images", format!("{} · {seconds:.2} s in all", sheet.count), false);
            }
        }
        if let Some(file) = &item.file {
            field(ui, "File", file.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(), true);
        }
    });
}

/// How a film writes it (a body's animation if one plays): copied, written
/// in the open film, watched played.
pub(super) fn in_a_film(ui: &mut egui::Ui, resources: &mut Resources, scripts: &mut Scripts, item: &Item, shown: &Shown) {
    let line = match shown.anims.iter().find(|(file, _)| shown.body && *file == resources.anim) {
        Some((_, word)) => Some(format!("{} anim {word}", item.word)),
        None => item.line(),
    };
    let Some(line) = line else {
        return;
    };
    let (said, copy) = match item.shelf {
        Shelf::Pieces => ("In a Decor", "Copy Decor Line"),
        Shelf::Tilesets => ("In a Dungeon", "Copy Dungeon Line"),
        _ => ("In a Film", "Copy Film Line"),
    };
    section(ui, said);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        egui::Frame::new().fill(theme::GROUND).inner_margin(egui::Margin::symmetric(8, 5)).show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(&line).monospace().size(theme::SMALL).color(Color32::from_rgb(0xDC, 0xDC, 0xAA))).selectable(true));
        });
    });
    let person = matches!(item.shelf, Shelf::Pokemon | Shelf::Variants);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if ui.button(copy).on_hover_text("the line, to paste in a film").clicked() {
            ui.ctx().copy_text(line.clone());
            resources.said = Some(format!("copied: {line}"));
        }
        if !matches!(item.shelf, Shelf::Pieces | Shelf::Tilesets)
            && ui.button("Write in the Film").on_hover_text("a line under the cursor's, in the open film").clicked()
        {
            resources.said = Some(match scripts.open.as_mut() {
                Some(open) => {
                    open.add_line(&line);
                    format!("written in {}: {line}", open.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default())
                }
                None => "no film open: open one in the Film space".to_string(),
            });
        }
        if item.shelf.watched() && ui.button("Watch It Play").on_hover_text("this line played, on the beach").clicked() {
            resources.watching = !resources.watching;
        }
    });
    let film = if person { Some(workshop_base::example::film_of(&line)) } else { item.watched_film() };
    if let Some(film) = film.filter(|_| resources.watching) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            resources.examples.show(ui, &film, (ui.available_width() - 16.0).min(280.0));
        });
    }
}

/// A move's effects, side by side, playing: what each one is, its number.
pub(super) fn move_effects(ui: &mut egui::Ui, resources: &mut Resources, item: &Item) {
    let effects = super::super::game_shelves::move_effects(&resources.session, item.number);
    if effects.iter().all(Option::is_none) {
        return;
    }
    section(ui, "Its Effects");
    let context = ui.ctx().clone();
    let seconds = resources.began.elapsed().as_secs_f64();
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        for (effect, role) in effects.iter().zip(super::super::game_shelves::EFFECT_ROLES) {
            let Some(effect) = effect else { continue };
            let shown = Item {
                shelf: Shelf::Effects,
                word: effect.to_string(),
                label: String::new(),
                about: String::new(),
                number: i64::from(*effect),
                file: None,
                order: 0,
            };
            ui.vertical(|ui| {
                let (room, response) = ui.allocate_exact_size(vec2(72.0, 72.0), egui::Sense::click());
                ui.painter().rect_filled(room, 2.0, theme::GROUND);
                if let Some(sheet) = resources.picture(&context, &shown) {
                    let step = sheet.step_at(seconds);
                    ui.painter().image(sheet.texture.id(), workshop_picture::fitted(sheet.cell, room.shrink(4.0)), sheet.uv(step), Color32::WHITE);
                    context.request_repaint_after(std::time::Duration::from_millis(33));
                }
                ui.label(theme::text(format!("{role} · {effect}"), theme::SMALL, theme::WEAK));
                if response.on_hover_text(format!("effect pikachu {effect}: this one alone")).clicked() {
                    ui.ctx().copy_text(format!("effect pikachu {effect}"));
                    resources.said = Some(format!("copied: effect pikachu {effect}"));
                }
            });
        }
    });
}

/// How many films and decors write it, and which (a click opens one).
pub(super) fn where_used(ui: &mut egui::Ui, resources: &mut Resources, item: &Item) {
    let used = resources.used_in(&item.word);
    let films = used.iter().filter(|path| path.extension().is_some_and(|own| own == "film")).count();
    let decors_count = used.len() - films;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text(format!("Used in {films} film{} and {decors_count} decor{}", plural(films), plural(decors_count)), theme::SMALL, theme::WEAK));
    });
    if used.is_empty() {
        return;
    }
    egui::CollapsingHeader::new(theme::text("Which", theme::SMALL, theme::TEXT)).id_salt(("where", item.key())).show(ui, |ui| {
        for path in used.iter().take(60) {
            let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            if ui.add(egui::Button::new(theme::text(&name, theme::SMALL, theme::TEXT)).frame(false)).on_hover_text("open it").clicked() {
                resources.session.ask(if path.extension().is_some_and(|own| own == "film") {
                    Ask::Film { path: path.clone(), line: 0 }
                } else {
                    Ask::Decor { path: path.clone(), line: 0 }
                });
            }
        }
    });
}
