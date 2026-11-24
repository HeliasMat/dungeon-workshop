//! The Hierarchy's Layers: each layer's row (its colour, its name, how many
//! things it holds), shown or hidden, alone, locked, how much it shows; the
//! layer things are put on; a line dropped on it moved there.

use eframe::egui::{self, Color32, Sense, pos2, vec2};

use super::hierarchy::{Asked, Dragged, Element, ROW, ROW_FOCUSED, ROW_HOVERED, Read, action_rect, section_head};
use crate::Decors;
use crate::edit;
use workshop_base::theme;

/// The colour of a layer, as its square shows it.
pub(super) fn layer_colour(word: &str) -> Color32 {
    match word {
        "terrains" => Color32::from_rgb(0x5B, 0xBF, 0x5A),
        "ground" => Color32::from_rgb(0xC8, 0x9B, 0x5A),
        "objects" => Color32::from_rgb(0x8A, 0xA4, 0xFF),
        "residents" => Color32::from_rgb(0xF6, 0xD3, 0x3C),
        "over" => Color32::from_rgb(0xC0, 0x84, 0xFC),
        "markers" => theme::DECOR,
        _ => theme::WRONG,
    }
}

/// What a layer holds, as said when it is pointed at.
pub(super) fn layer_meaning(word: &str) -> &'static str {
    match word {
        "terrains" => "the painted terrains, under everything",
        "ground" => "on the ground: under the residents, top and bottom (`layer ground`)",
        "objects" => "as in the game: the bottom under the residents, the top in front of them",
        "residents" => "those who live there",
        "over" => "in front of everyone, top and bottom (`layer over`)",
        "markers" => "the films' spots, entrances, exits",
        _ => "where no one walks, painted by hand",
    }
}

/// Whether things may be put on a layer (and lines dropped on it).
pub(super) fn placeable(word: &str) -> bool {
    matches!(word, "ground" | "objects" | "over")
}

/// The things of a layer (none for the walls, painted, not put).
fn layer_elements(read: &Read, word: &str) -> Vec<Element> {
    match word {
        "terrains" => read.terrains.iter().map(|(line, _)| (*line, 0)).collect(),
        "walls" => Vec::new(),
        _ => read.items.iter().filter(|one| one.layer == word).map(|one| (one.line, one.at)).collect(),
    }
}

/// The Layers section: its head, the layer shown alone, each layer's row.
pub(super) fn layers(ui: &mut egui::Ui, decors: &mut Decors, read: &Read, asked: &mut Asked) {
    let mut layers_open = decors.sections.0;
    section_head(ui, "Layers", &mut layers_open, |_, _| {});
    decors.sections.0 = layers_open;
    if !layers_open {
        return;
    }
    if let Some(alone) = decors.solo {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let name = edit::LAYERS.iter().find(|(word, _)| *word == alone).map_or(alone, |(_, name)| name);
            ui.label(theme::text(format!("shown alone: {name}"), theme::SMALL, theme::DECOR));
            if ui.small_button("show all").clicked() {
                decors.solo = None;
            }
        });
    }
    for (word, name) in edit::LAYERS {
        layer_row(ui, decors, read, asked, word, name);
    }
}

/// A layer's row: its square, its name, how many; its actions; a click makes
/// it the active layer, a double-click chooses all it holds, a line dropped goes there.
fn layer_row(ui: &mut egui::Ui, decors: &mut Decors, read: &Read, asked: &mut Asked, word: &'static str, name: &str) {
    let keys = layer_elements(read, word);
    let count = match word {
        "walls" => {
            let (walled, free) = edit::painted_walls(&read.text);
            walled.len() + free.len()
        }
        _ => keys.len(),
    };
    let placeable = placeable(word);
    let active = decors.active_layer == word;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    let dropping = placeable && response.dnd_hover_payload::<Dragged>().is_some();
    let fill = if dropping {
        theme::DECOR.gamma_multiply(0.35)
    } else if active {
        ROW_FOCUSED
    } else if response.hovered() {
        ROW_HOVERED
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    let shown = !decors.is_hidden(word);
    let colour = if !shown {
        theme::FAINT
    } else if active {
        theme::STRONG
    } else {
        theme::TEXT
    };
    let x = rect.min.x + 20.0;
    ui.painter().rect_filled(
        egui::Rect::from_center_size(pos2(x + 5.0, rect.center().y), vec2(8.0, 8.0)),
        2.0,
        layer_colour(word).gamma_multiply(if shown { 1.0 } else { 0.35 }),
    );
    ui.painter().text(pos2(x + 16.0, rect.center().y), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(theme::BODY), colour);
    let after = x + 22.0 + ui.painter().layout_no_wrap(name.to_string(), egui::FontId::proportional(theme::BODY), colour).size().x;
    let said = if active { format!("active · {count}") } else { count.to_string() };
    ui.painter().text(
        pos2(after, rect.center().y),
        egui::Align2::LEFT_CENTER,
        said,
        egui::FontId::proportional(theme::SMALL),
        if active { theme::DECOR } else { theme::FAINT },
    );
    layer_actions(ui, decors, rect, word, shown);
    let response = response.on_hover_text(format!(
        "{}{}",
        layer_meaning(word),
        if placeable { "\nclick: what is put goes there · double-click: all its elements chosen" } else { "" }
    ));
    if response.hovered() {
        asked.hovered = keys.clone();
    }
    if response.clicked() && placeable {
        decors.active_layer = word;
    }
    if response.double_clicked() && !keys.is_empty() {
        asked.chose = Some(keys.clone());
    }
    if placeable && let Some(dragged) = response.dnd_release_payload::<Dragged>() {
        asked.made = edit::on_layer(&read.text, dragged.0, word);
    }
    response.context_menu(|ui| layer_menu(ui, decors, asked, word, &keys));
}

/// A layer's actions, always shown when not as by default: the eye (Alt: it
/// alone), the lock, how much it shows.
fn layer_actions(ui: &mut egui::Ui, decors: &mut Decors, rect: egui::Rect, word: &'static str, shown: bool) {
    let locked = decors.locked.contains(word);
    let pointed = ui.rect_contains_pointer(rect);
    if pointed || !shown || decors.solo == Some(word) {
        let alt = ui.input(|input| input.modifiers.alt);
        if workshop_base::icons::button(ui, action_rect(rect, 0), "shown or hidden (Alt+click: it alone)", |painter, at, tint| {
            workshop_base::icons::eye(painter, at, shown, tint)
        }) {
            if alt {
                decors.solo = if decors.solo == Some(word) { None } else { Some(word) };
            } else if decors.solo.is_some() {
                decors.solo = None;
                decors.hidden.remove(word);
            } else if shown {
                decors.hidden.insert(word);
            } else {
                decors.hidden.remove(word);
            }
        }
    }
    if (pointed || locked)
        && workshop_base::icons::button(ui, action_rect(rect, 1), "locked: the mouse takes nothing of this layer", |painter, at, tint| {
            workshop_base::icons::lock(painter, at, locked, tint)
        })
    {
        if locked {
            decors.locked.remove(word);
        } else {
            decors.locked.insert(word);
        }
    }
    let seen = decors.opacity.get(word).copied().unwrap_or(1.0);
    if pointed && word != "walls" && word != "markers" {
        let room = egui::Rect::from_center_size(pos2(action_rect(rect, 2).center().x - 10.0, rect.center().y), vec2(44.0, 18.0));
        let mut percent = seen * 100.0;
        if ui
            .put(room, egui::DragValue::new(&mut percent).range(0.0..=100.0).speed(1.0).suffix("%").max_decimals(0))
            .on_hover_text("how much it shows, in the Workshop only")
            .changed()
        {
            decors.opacity.insert(word, percent / 100.0);
        }
    } else if seen < 1.0 {
        ui.painter().text(
            pos2(action_rect(rect, 2).center().x - 10.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}%", seen * 100.0),
            egui::FontId::proportional(theme::SMALL),
            theme::FAINT,
        );
    }
}

/// A layer's menu: all its things chosen, the list filtered on it, it alone shown, made active, its opacity back.
fn layer_menu(ui: &mut egui::Ui, decors: &mut Decors, asked: &mut Asked, word: &'static str, keys: &[Element]) {
    if !keys.is_empty() && ui.button("Choose All Its Elements").clicked() {
        asked.chose = Some(keys.to_vec());
        ui.close();
    }
    if ui.button(if decors.layer_filter == Some(word) { "Stop Filtering the List" } else { "List Only This" }).clicked() {
        decors.layer_filter = if decors.layer_filter == Some(word) { None } else { Some(word) };
        ui.close();
    }
    if ui.button(if decors.solo == Some(word) { "Show All" } else { "Show Only This" }).clicked() {
        decors.solo = if decors.solo == Some(word) { None } else { Some(word) };
        ui.close();
    }
    if placeable(word) && ui.button("Active Layer").clicked() {
        decors.active_layer = word;
        ui.close();
    }
    if ui.button("Opacity 100%").clicked() {
        decors.opacity.remove(word);
        ui.close();
    }
}
