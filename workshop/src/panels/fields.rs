//! Fields written in: a text typed, a rule's field, a thing to add.

use eframe::egui;

use super::inspector::prop;
use crate::decors::Decors;
use crate::edit::{self, Kind};
use workshop_base::theme;

/// A field typed in: what it says while it has the keys, written (given
/// back) once it lets them go changed.
pub(crate) fn typed(ui: &mut egui::Ui, id: egui::Id, now: &str, rows: usize, hint: &str) -> Option<String> {
    let mut typed = ui.data(|data| data.get_temp::<String>(id)).unwrap_or_else(|| now.to_string());
    let field = if rows > 1 {
        ui.add(egui::TextEdit::multiline(&mut typed).id(id.with("field")).code_editor().desired_rows(rows).desired_width(f32::INFINITY).hint_text(hint))
    } else {
        ui.add(egui::TextEdit::singleline(&mut typed).id(id.with("field")).desired_width(ui.available_width()).hint_text(hint))
    };
    if field.has_focus() {
        ui.data_mut(|data| data.insert_temp(id, typed));
        return None;
    }
    ui.data_mut(|data| data.remove::<String>(id));
    (field.lost_focus() && typed != now).then_some(typed)
}

/// A rule's lines, typed in: written once the field lets the keys go, if
/// each block it opens is closed.
pub(super) fn rule_field(ui: &mut egui::Ui, head: &str, body: &str) -> Option<String> {
    let id = egui::Id::new(("life rule", edit::rule_key(head)));
    let wrong_id = id.with("wrong");
    let wrong = ui.data(|data| data.get_temp::<bool>(wrong_id)).unwrap_or(false);
    let mut typed = ui.data(|data| data.get_temp::<String>(id)).unwrap_or_else(|| body.to_string());
    let field = ui.add(
        egui::TextEdit::multiline(&mut typed)
            .id(id.with("field"))
            .code_editor()
            .desired_rows(3)
            .desired_width(f32::INFINITY)
            .hint_text("play a_film\nset a_flag"),
    );
    let mut written = None;
    let (mut keep, mut now_wrong) = (field.has_focus() || wrong, wrong);
    if field.lost_focus() {
        match () {
            _ if typed == body => (keep, now_wrong) = (false, false),
            _ if edit::balanced(&typed) => (written, keep, now_wrong) = (Some(typed.clone()), false, false),
            _ => (keep, now_wrong) = (true, true),
        }
    }
    ui.data_mut(|data| {
        if keep {
            data.insert_temp(id, typed);
        } else {
            data.remove::<String>(id);
        }
        if now_wrong {
            data.insert_temp(wrong_id, true);
        } else {
            data.remove::<bool>(wrong_id);
        }
    });
    if now_wrong {
        ui.label(theme::text("not written: an “if” (or “choice”) without its “end”", theme::SMALL, theme::WRONG));
    }
    written
}

/// What may be added, in the middle of what is seen.
pub(super) fn add(ui: &mut egui::Ui, decors: &mut Decors, made: &mut Option<String>) {
    let Some(open) = &decors.open else { return };
    let [x, y] = open.middle.map(f32::round);
    let mut line: Option<String> = None;
    prop(ui, "Marker", |ui| {
        if ui.button("Spot").on_hover_text("a films' spot, with nothing put there (place pikachu name)").clicked() {
            let names: Vec<String> = edit::items(&open.text).into_iter().filter(|one| one.kind == Kind::Spot).map(|one| one.name).collect();
            let name = (1..).map(|at| format!("spot{at}")).find(|name| !names.contains(name)).unwrap_or_default();
            line = Some(format!("spot {name} {x} {y}"));
        }
        if ui.button("Entrance").on_hover_text("where one comes in, in the films").clicked() {
            line = Some(format!("entrance {x} {y}"));
        }
        if ui.button("Exit").on_hover_text("a zone that leads to a game place (or, in the Inspector, to a decor by its name)").clicked() {
            line = Some(format!("exit {} {} 32 16 to 224", x - 16.0, y - 8.0));
        }
        if ui.button("Zone").on_hover_text("a named zone: walking into it plays the place's `on zone <name>`").clicked() {
            let names: Vec<String> = edit::items(&open.text).into_iter().filter(|one| one.kind == Kind::Zone).map(|one| one.name).collect();
            let name = (1..).map(|at| format!("zone{at}")).find(|name| !names.contains(name)).unwrap_or_default();
            line = Some(format!("zone {name} {} {} 32 32", x - 16.0, y - 16.0));
        }
    });
    prop(ui, "Resident", |ui| {
        ui.add(egui::TextEdit::singleline(&mut decors.resident).hint_text("pikachu").desired_width(80.0));
        if ui.button("Add").on_hover_text("someone who lives there, by their Pokémon name without accents").clicked() && !decors.resident.trim().is_empty() {
            line = Some(format!("resident {} {x} {y}", decors.resident.trim()));
        }
    });
    ui.label(theme::text("The pieces: the Library, below (click or drag).", theme::SMALL, theme::FAINT));
    if let Some(line) = line {
        let text = open.with_added(&line);
        let added = edit::items(&text).into_iter().rev().find(|one| text.lines().nth(one.line - 1).is_some_and(|known| known.trim() == line.trim()));
        *made = Some(text);
        if let Some(open) = &mut decors.open {
            open.choose(added.map(|one| vec![(one.line, one.at)]).unwrap_or_default());
        }
    }
}
