//! The Inspector of the Resources: the one chosen in large, what it is, how
//! a film writes it, where it is used; a Pokémon's actions.
//!
//! Here the order of its parts; `large.rs` the one chosen in large (its
//! steps, its faces, its animations, how loud), `about.rs` what it is, how a
//! film writes it, where it is used, `actions.rs` what to do with it (a thing
//! drawn renamed, put in the bin), `pokemon.rs` a Pokémon's own actions.

use eframe::egui::{self, FontId, Sense, pos2, vec2};

use super::{Resources, Sheet, Shelf};
use crate::decors::Decors;
use workshop_base::theme;
use workshop_script::Scripts;

mod about;
mod actions;
mod large;
mod pokemon;
#[cfg(test)]
mod tests;

/// A text with a word made another, wherever it is whole.
pub(super) fn word_replaced(text: &str, old: &str, new: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        out.push_str(if !word.is_empty() && film::words::plain(word) == old { new } else { word });
        word.clear();
    };
    for letter in text.chars() {
        if letter.is_alphanumeric() || letter == '_' {
            word.push(letter);
        } else {
            flush(&mut word, &mut out);
            out.push(letter);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// What the parts share of the one chosen: a person's (its faces, its body's
/// animations, its body shown or its face), its sheet, the step shown.
struct Shown {
    person: bool,
    has_faces: bool,
    anims: Vec<(u8, String)>,
    body: bool,
    sheet: Option<Sheet>,
    steps: usize,
    step: usize,
}

pub fn inspector(ui: &mut egui::Ui, resources: &mut Resources, decors: &mut Decors, scripts: &mut Scripts) {
    let Some(item) = resources.chosen_item(decors) else {
        ui.add_space(8.0);
        ui.label(theme::weak("Choose a resource: its preview, what it is, how a film writes it, where it is used."));
        return;
    };
    let context = ui.ctx().clone();
    egui::ScrollArea::vertical().id_salt("resource inspector").auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let shown = large::shown(resources, &context, &item);
        let scale_said = large::in_large(ui, resources, decors, &context, &item, &shown);
        large::steps_row(ui, resources, &item, &shown, &scale_said);
        if shown.person && shown.has_faces {
            large::faces(ui, resources, &item, &shown);
        }
        if shown.person && !shown.anims.is_empty() {
            large::animations(ui, resources, &shown);
        }
        if item.shelf.heard() {
            large::loudness(ui, resources);
        }
        about::what_it_is(ui, &item, &shown);
        about::in_a_film(ui, resources, scripts, &item, &shown);
        if item.shelf == Shelf::Attacks {
            about::move_effects(ui, resources, &item);
        }
        if !matches!(item.shelf, Shelf::Effects | Shelf::Tilesets) {
            about::where_used(ui, resources, &item);
        }
        actions::buttons(ui, resources, &item);
        if shown.person {
            pokemon::actions(ui, resources, &item);
        }
        if item.shelf == Shelf::Drawn
            && let Some(file) = item.file.clone()
        {
            actions::renamed_or_put_away(ui, resources, decors, &item, file);
        }
        if let Some(said) = &resources.said {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(theme::text(said, theme::SMALL, theme::FAINT));
            });
        }
    });
}

/// `s` after a count that is more than one.
fn plural(count: usize) -> &'static str {
    if count > 1 { "s" } else { "" }
}

/// A part of the Inspector's, by its name.
fn section(ui: &mut egui::Ui, name: &str) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::TOOLS);
    workshop_base::icons::chevron(ui.painter(), pos2(rect.min.x + 10.0, rect.center().y), true, theme::WEAK);
    ui.painter().text(pos2(rect.min.x + 20.0, rect.center().y), egui::Align2::LEFT_CENTER, name, FontId::proportional(theme::BODY), theme::STRONG);
}
