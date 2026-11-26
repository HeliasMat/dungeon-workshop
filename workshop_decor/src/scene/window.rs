//! The small window beside a marker laid (or taken) with a marker tool:
//! its essentials, each written as it is filled (one change each): a
//! resident's Pokémon (sought in the Pokémon catalog) and first line, a
//! spot's or a zone's name, an entrance's name, where an exit leads.

use eframe::egui::{self, Pos2, RichText};

use crate::edit::{self, Kind};
use crate::{Decors, markers};
use workshop_base::theme;

/// The window, beside that point of the screen, for the thing on line `number`.
pub(super) fn window(ui: &egui::Ui, decors: &mut Decors, number: usize, beside: Pos2) {
    let Some(open) = &decors.open else { return };
    let text = open.text.clone();
    let Some(item) = edit::items(&text).into_iter().find(|one| one.line == number) else {
        decors.marker_window = None;
        return;
    };
    let line = text.lines().nth(number - 1).unwrap_or_default().to_string();
    let mut made: Option<String> = None;
    let mut close = false;
    let area = egui::Area::new(egui::Id::new("marker window")).order(egui::Order::Foreground).fixed_pos(beside).constrain(true);
    area.show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_width(250.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(decors.item_name(&item)).strong().color(theme::STRONG));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close |= ui.small_button("Done").on_hover_text("close (Esc); the marker stays as written").clicked();
                });
            });
            ui.add_space(2.0);
            match item.kind {
                Kind::Lives => resident(ui, decors, &text, number, &item.name, &mut made),
                Kind::Exit => exit(ui, decors, &text, number, &line, &mut made),
                Kind::Spot | Kind::Zone | Kind::Entrance => {
                    let hint = match item.kind {
                        Kind::Spot => "a name, e.g. by_the_well",
                        Kind::Zone => "a name, e.g. the_dock",
                        _ => "a name, e.g. north_gate (optional)",
                    };
                    ui.label(theme::weak("Name"));
                    if let Some(name) = crate::panels::fields::typed(ui, egui::Id::new(("marker name", number)), &item.name, 1, hint) {
                        made = edit::renamed(&text, number, &name);
                    }
                    let said = match item.kind {
                        Kind::Spot => format!("a film says `pikachu put {}`", if item.name.is_empty() { "<name>" } else { &item.name }),
                        Kind::Zone => "walking in plays its `on zone` (Inspector)".to_string(),
                        _ => "a film comes in here: `arrives … by:<name>`".to_string(),
                    };
                    ui.label(theme::text(said, theme::SMALL, theme::FAINT));
                }
                Kind::Piece => {}
            }
            ui.add_space(2.0);
            ui.label(theme::text("drag it on the scene to move it · the Inspector has the rest", theme::SMALL, theme::FAINT));
        });
    });
    if close || ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        decors.marker_window = None;
    }
    if let (Some(made), Some(open)) = (made, &mut decors.open) {
        open.change(made);
    }
}

/// A resident's Pokémon, sought in the catalog (the first found taken by
/// Enter), and the first line he says.
fn resident(ui: &mut egui::Ui, decors: &mut Decors, text: &str, number: usize, word: &str, made: &mut Option<String>) {
    ui.label(theme::weak(format!("Pokémon · {word}")));
    let field = ui.add(egui::TextEdit::singleline(&mut decors.species.sought).hint_text("search: a name, a number, a type…").desired_width(f32::INFINITY));
    if field.changed() {
        decors.species.asked = Some(decors.species.sought.clone());
    }
    let mut picked: Option<String> = None;
    if !decors.species.sought.trim().is_empty() {
        if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            picked = decors.species.found.first().map(|(word, _)| word.clone());
        }
        ui.horizontal_wrapped(|ui| {
            for (found, name) in decors.species.found.iter().take(12) {
                if ui.selectable_label(found == word, name).on_hover_text(format!("`{found}`")).clicked() {
                    picked = Some(found.clone());
                }
            }
            if decors.species.found.is_empty() {
                ui.label(theme::text("nothing found", theme::SMALL, theme::FAINT));
            }
        });
    }
    if let Some(picked) = picked {
        *made = edit::renamed(text, number, &picked);
        decors.resident = picked;
        decors.species.sought.clear();
    }
    ui.add_space(2.0);
    ui.label(theme::weak("First Line"));
    let now = edit::speech(text, number).first().cloned().unwrap_or_default();
    if let Some(said) = crate::panels::fields::typed(ui, egui::Id::new(("resident first", number)), &now, 1, "what they say, e.g. Welcome to the beach!") {
        // (A Pokémon picked in the same frame: its line written on the text it makes.)
        let base = made.clone().unwrap_or_else(|| text.to_string());
        *made = Some(markers::with_first_said(&base, number, &said));
    }
}

/// Where an exit leads: a decor of the project (one of its menu), or a
/// game place by its number, typed.
fn exit(ui: &mut egui::Ui, decors: &Decors, text: &str, number: usize, line: &str, made: &mut Option<String>) {
    ui.label(theme::weak("Leads To"));
    let now = markers::leads_to(line);
    if let Some(to) = crate::panels::fields::typed(ui, egui::Id::new(("exit to", number)), &now, 1, "a decor's name, or a game place's number: 224") {
        *made = markers::led_to(text, number, &to);
    }
    let decor_names = decor::names(decors.session.library());
    if !decor_names.is_empty() {
        ui.menu_button("a decor of yours ▾", |ui| {
            for name in decor_names {
                if ui.selectable_label(now.split_whitespace().next() == Some(name.as_str()), &name).clicked() {
                    *made = markers::led_to(text, number, &name);
                    ui.close();
                }
            }
        });
    }
}
