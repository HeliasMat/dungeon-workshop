//! The Inspector of the decor: a component's frame and its properties, the
//! remarks, the raw line; several things chosen; the decor's settings.

use decor::doc::Pose;
use eframe::egui::{self, Color32, RichText, Sense, pos2, vec2};

use super::fields::{add, rule_field};
use super::landmark::landmark;
use super::paint_badge;
use super::piece::piece;
use crate::decors::{Decors, paint_thumb};
use crate::edit::{self, Item, Kind};
use crate::terrains::Tool;
use crate::theme;

/// How wide the labels are, left of the values.
const LABEL: f32 = 96.0;

/// A component, as Unity has them: its title on a bar (a click folds it),
/// its properties under it.
pub(super) fn component(ui: &mut egui::Ui, decors: &mut Decors, title: &str, body: impl FnOnce(&mut egui::Ui, &mut Decors)) {
    let open = !decors.folded_parts.contains(title);
    ui.add_space(2.0);
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
    ui.painter().rect_filled(rect, 0.0, if response.hovered() { theme::HOVERED } else { theme::TOOLS });
    crate::icons::chevron(ui.painter(), pos2(rect.min.x + 10.0, rect.center().y), open, theme::TEXT);
    ui.painter().text(pos2(rect.min.x + 22.0, rect.center().y), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(theme::BODY), theme::STRONG);
    if response.clicked() && !decors.folded_parts.remove(title) {
        decors.folded_parts.insert(title.to_string());
    }
    if open {
        egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 6, top: 6, bottom: 8 }).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            body(ui, decors);
        });
    }
}

/// A property: its label on the left, what changes it on the right.
pub(super) fn prop<R>(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(LABEL, 20.0), Sense::hover());
        ui.painter().text(rect.left_center(), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(theme::BODY), theme::WEAK);
        add(ui)
    })
    .inner
}

/// The small arrow giving a property back its value by default (shown only when it has another).
pub(super) fn reset(ui: &mut egui::Ui, differs: bool) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    differs && crate::icons::button(ui, rect, "default value", crate::icons::reset)
}

/// The head: a picture (or a badge), a name, what it is and where.
pub(super) fn head(ui: &mut egui::Ui, art: Option<&crate::decors::PieceArt>, badge: (&str, Color32, &str), title: &str, under: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let (square, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
        ui.painter().rect_filled(square, 3.0, theme::FIELD);
        match art {
            Some(art) => paint_thumb(ui.painter(), art, square.shrink(4.0)),
            None => paint_badge(ui.painter(), egui::Rect::from_center_size(square.center(), vec2(30.0, 30.0)), badge),
        }
        ui.vertical(|ui| {
            ui.add_space(6.0);
            ui.label(theme::text(title, theme::TITLE, theme::STRONG).strong());
            ui.label(RichText::new(under).monospace().size(theme::SMALL - 1.0).color(theme::FAINT));
        });
    });
    ui.add_space(4.0);
}

/// What baking said of that line (its remarks, what is wrong).
pub(super) fn remarks(ui: &mut egui::Ui, decors: &Decors, numbers: &[usize]) {
    let Some(open) = &decors.open else { return };
    for said in open.said.iter().filter(|said| numbers.contains(&said.line)) {
        let colour = if said.wrong { theme::WRONG } else { theme::FILM };
        egui::Frame::new()
            .fill(colour.gamma_multiply(0.12))
            .stroke(egui::Stroke::new(1.0_f32, colour.gamma_multiply(0.6)))
            .inner_margin(6)
            .corner_radius(3)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(theme::text(&said.text, theme::SMALL, colour));
            });
        ui.add_space(4.0);
    }
}

/// The line itself, written by hand: kept only if the decor still reads.
pub(super) fn raw(ui: &mut egui::Ui, decors: &mut Decors, number: usize, made: &mut Option<String>) {
    let Some(open) = &decors.open else { return };
    let now = open.text.lines().nth(number - 1).unwrap_or_default().to_string();
    let text = open.text.clone();
    if decors.raw_line.as_ref().is_none_or(|(line, _, was)| *line != number || *was != now) {
        decors.raw_line = Some((number, now.trim().to_string(), now.clone()));
        decors.raw_said = None;
    }
    let Some((_, typed, _)) = decors.raw_line.as_mut() else { return };
    let field = ui.add(egui::TextEdit::multiline(typed).code_editor().desired_rows(2).desired_width(f32::INFINITY));
    let asked = field.has_focus() && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
    let typed = typed.clone();
    ui.horizontal(|ui| {
        let changed = typed.trim() != now.trim();
        if ui.add_enabled(changed, egui::Button::new("Apply")).on_hover_text("Ctrl+Enter").clicked() || (asked && changed) {
            let lead: String = now.chars().take_while(|letter| letter.is_whitespace()).collect();
            let candidate = edit::with_lines(&text, &[(number, format!("{lead}{}", typed.trim()))]);
            match decor::read("decor", &candidate) {
                Ok(_) => {
                    *made = Some(candidate);
                    decors.raw_said = None;
                }
                Err(why) => decors.raw_said = Some(why.message.clone()),
            }
        }
        if changed && ui.button("Cancel").clicked() {
            decors.raw_line = None;
        }
    });
    if let Some(said) = &decors.raw_said {
        ui.label(theme::text(said, theme::SMALL, theme::WRONG));
    }
}

pub fn inspector(ui: &mut egui::Ui, decors: &mut Decors) {
    let Some(open) = &decors.open else {
        ui.label(theme::weak("No decor open."));
        return;
    };
    match decors.brushes.tool {
        Tool::Terrain => return crate::terrains::terrain_panel(ui, decors),
        Tool::Strew => return crate::terrains::strew_panel(ui, decors),
        Tool::Walls => return crate::terrains::walls_panel(ui, decors),
        Tool::Walk => return crate::walk::walk_panel(ui, decors),
        Tool::Choose => {}
    }
    let selection = open.selection();
    let text = open.text.clone();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if selection.len() > 1 {
            return many(ui, decors, &selection);
        }
        let Some((number, corner)) = decors.open.as_ref().and_then(|open| open.chosen) else {
            return settings(ui, decors);
        };
        let line = text.lines().nth(number.wrapping_sub(1)).unwrap_or_default();
        if line.trim_start().starts_with("terrain ") {
            let mut made = None;
            remarks(ui, decors, &[number]);
            crate::terrains::terrain_inspector(ui, decors, number);
            component(ui, decors, "Line", |ui, decors| raw(ui, decors, number, &mut made));
            if let (Some(made), Some(open)) = (made, &mut decors.open) {
                open.change(made);
            }
            return;
        }
        let item = edit::items(&text).into_iter().find(|one| one.line == number && one.at == corner);
        match item {
            Some(item) if matches!(item.kind, Kind::Spot | Kind::Entrance | Kind::Exit | Kind::Zone) => landmark(ui, decors, &item),
            Some(item) => piece(ui, decors, &item),
            None => settings(ui, decors),
        }
    });
}

/// Several chosen: what they have alike, changed for all; lined up, spread;
/// grouped, kept as a model; duplicated, taken out.
pub(super) fn many(ui: &mut egui::Ui, decors: &mut Decors, selection: &[(usize, usize)]) {
    let text = decors.open.as_ref().map(|open| open.text.clone()).unwrap_or_default();
    let items = edit::items(&text);
    let chosen: Vec<&Item> = selection.iter().filter_map(|key| items.iter().find(|one| (one.line, one.at) == *key)).collect();
    let boxes: Vec<((usize, usize), egui::Rect, [f32; 2])> = chosen.iter().map(|one| ((one.line, one.at), decors.item_box(one), one.corner)).collect();
    let numbers: Vec<usize> = {
        let mut numbers: Vec<usize> = selection.iter().map(|(line, _)| *line).collect();
        numbers.sort_unstable();
        numbers.dedup();
        numbers
    };
    head(ui, None, ("×", theme::DECOR, ""), &format!("{} elements", selection.len()), &format!("{} lines · the first chosen is the reference", numbers.len()));
    remarks(ui, decors, &numbers);
    let mut made: Option<String> = None;
    let mut chose: Option<Vec<(usize, usize)>> = None;
    // (What they have alike: a box half ticked when some have it.)
    let poses: Vec<(usize, Pose)> =
        numbers.iter().filter_map(|number| Some((*number, Pose::read(text.lines().nth(number - 1)?)?))).filter(|(_, pose)| pose.word != "resident").collect();
    if !poses.is_empty() {
        component(ui, decors, "Shared Properties", |ui, _| {
            let all = |test: &dyn Fn(&Pose) -> bool| (poses.iter().all(|(_, pose)| test(pose)), poses.iter().any(|(_, pose)| test(pose)));
            let set_all = |change: &dyn Fn(&mut Pose)| -> String {
                let lines: Vec<(usize, String)> = poses
                    .iter()
                    .map(|(number, pose)| {
                        let mut pose = pose.clone();
                        change(&mut pose);
                        (*number, pose.line())
                    })
                    .collect();
                edit::with_lines(&text, &lines)
            };
            prop(ui, "Layer", |ui| {
                for (word, name) in [("ground", "Ground"), ("objects", "Objects"), ("over", "Over")] {
                    let layer_of = |pose: &Pose| match pose.layer.as_deref() {
                        Some("ground") => "ground",
                        Some("over") => "over",
                        _ => "objects",
                    };
                    let (every, some) = all(&|pose| layer_of(pose) == word);
                    let label = if some && !every { format!("{name} ·") } else { name.to_string() };
                    if ui.selectable_label(every, label).clicked() {
                        made = Some(set_all(&|pose| pose.layer = (word != "objects").then(|| word.to_string())));
                    }
                }
            });
            for (label, test, change) in [
                ("Flipped", (|pose: &Pose| pose.turned) as fn(&Pose) -> bool, (|pose: &mut Pose, on: bool| pose.turned = on) as fn(&mut Pose, bool)),
                ("Shadow", |pose| !pose.shadowless, |pose, on| pose.shadowless = !on),
                ("Locked", |pose| pose.locked, |pose, on| pose.locked = on),
                ("Hidden", |pose| pose.hidden, |pose, on| pose.hidden = on),
            ] {
                let (every, some) = all(&test);
                let mut ticked = every;
                if prop(ui, label, |ui| ui.add(egui::Checkbox::without_text(&mut ticked).indeterminate(some && !every)).changed()) {
                    made = Some(set_all(&|pose| change(pose, !every)));
                }
            }
        });
    }
    component(ui, decors, "Align and Distribute", |ui, _| {
        prop(ui, "Align", |ui| {
            for (how, word) in [(edit::Line::Left, "Left"), (edit::Line::Centre, "Centre"), (edit::Line::Right, "Right")] {
                if ui.button(word).clicked() {
                    made = Some(edit::lined_up(&text, &boxes, how));
                }
            }
        });
        prop(ui, "", |ui| {
            for (how, word) in [(edit::Line::Top, "Top"), (edit::Line::Middle, "Middle"), (edit::Line::Bottom, "Bottom")] {
                if ui.button(word).clicked() {
                    made = Some(edit::lined_up(&text, &boxes, how));
                }
            }
        });
        prop(ui, "Distribute", |ui| {
            let enough = boxes.len() >= 3;
            if ui.add_enabled(enough, egui::Button::new("Horizontally")).on_hover_text("equal gaps from left to right (three at least)").clicked() {
                made = Some(edit::spread(&text, &boxes, true));
            }
            if ui.add_enabled(enough, egui::Button::new("Vertically")).clicked() {
                made = Some(edit::spread(&text, &boxes, false));
            }
        });
    });
    component(ui, decors, "Group and Model", |ui, decors| {
        prop(ui, "Group", |ui| {
            ui.add(egui::TextEdit::singleline(&mut decors.group_name).hint_text("name").desired_width(110.0));
            if ui.button("Group").on_hover_text("their lines together, in a folder of the Hierarchy").clicked() {
                match edit::grouped(&text, &numbers, &decors.group_name) {
                    Ok(grouped) => {
                        made = Some(grouped);
                        chose = Some(Vec::new());
                    }
                    Err(why) => decors.model_said = Some(why),
                }
            }
        });
        prop(ui, "Model", |ui| {
            ui.add(egui::TextEdit::singleline(&mut decors.model_name).hint_text("name").desired_width(110.0));
            if ui.button("Save").on_hover_text("put again elsewhere from the Library › Models").clicked() {
                decors.model_said = Some(match edit::save_model(decors.session.library(), &decors.model_name, &edit::copied(&text, selection)) {
                    Ok(path) => format!("model written: {}", path.display()),
                    Err(why) => why,
                });
                decors.models = None;
            }
        });
        if let Some(said) = &decors.model_said {
            ui.label(theme::text(said, theme::SMALL, theme::WEAK));
        }
    });
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("Duplicate").on_hover_text("Ctrl+D").clicked() {
            let area = boxes.iter().map(|(_, rect, _)| *rect).reduce(|a, b| a.union(b));
            if let Some(area) = area {
                let (pasted, now) = edit::pasted(&text, &edit::copied(&text, selection), [area.min.x + area.width(), area.min.y]);
                made = Some(pasted);
                chose = Some(now);
            }
        }
        if ui.button(RichText::new("Remove").color(theme::WRONG)).on_hover_text("Suppr").clicked() {
            made = Some(edit::removed(&text, selection));
            chose = Some(Vec::new());
        }
    });
    if let Some(open) = &mut decors.open {
        if let Some(made) = made {
            open.change(made);
        }
        if let Some(chose) = chose {
            open.choose(chose);
        }
    }
}

/// Nothing chosen: the decor itself (its name, size, grid, number, music,
/// chance), what it holds, what may be added to it.
fn settings(ui: &mut egui::Ui, decors: &mut Decors) {
    let Some(open) = &decors.open else { return };
    let text = open.text.clone();
    let [w, h] = open.size();
    let id = text.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or("decor").to_string();
    head(ui, None, ("D", theme::DECOR, ""), &edit::setting(&text, "name").unwrap_or_else(|| id.clone()), &format!("{id}.decor · {w} × {h}"));
    remarks(ui, decors, &[0]);
    let mut made: Option<String> = None;
    component(ui, decors, "Decor", |ui, decors| {
        let mut name = edit::setting(&text, "name").unwrap_or_default();
        prop(ui, "Name", |ui| {
            let field = ui.add(egui::TextEdit::singleline(&mut name).hint_text("its name in the game").desired_width(ui.available_width()));
            if field.lost_focus() && edit::setting(&text, "name").unwrap_or_default() != name {
                made = Some(edit::with_setting(&text, "name", (!name.trim().is_empty()).then_some(name.trim())));
            }
        });
        let size = decors.size_edit.get_or_insert([w as i64, h as i64]);
        let mut done = false;
        prop(ui, "Size", |ui| {
            for (at, word) in [(0, "L "), (1, "H ")] {
                let response = ui.add(egui::DragValue::new(&mut size[at]).range(decor::lang::SIDES.0..=decor::lang::SIDES.1).speed(8.0).prefix(word));
                done |= response.drag_stopped() || response.lost_focus();
            }
        });
        if done {
            if [size[0], size[1]] != [w as i64, h as i64] {
                made = Some(edit::with_size(&text, size[0], size[1]));
            }
            decors.size_edit = None;
        }
        for (word, label, low, high, said) in [
            ("grid", "Grid", 1, 256, "the grid's step, in pixels"),
            ("level", "Level", 0, 9999, "its number in the game (else the first free one)"),
            ("music", "Music", 0, 999, "the number of the place's music"),
            ("seed", "Seed", 0, 99999, "the chance of `scatter`"),
        ] {
            let now: Option<i64> = edit::setting(&text, word).and_then(|value| value.parse().ok());
            prop(ui, label, |ui| {
                let mut on = now.is_some();
                if ui.checkbox(&mut on, "").on_hover_text(said).changed() {
                    let value = if on { Some(if word == "grid" { "8".to_string() } else { low.max(1).to_string() }) } else { None };
                    made = Some(edit::with_setting(&text, word, value.as_deref()));
                }
                if let Some(mut value) = now
                    && ui.add(egui::DragValue::new(&mut value).range(low..=high)).changed()
                {
                    made = Some(edit::with_setting(&text, word, Some(&value.to_string())));
                }
            });
        }
    });
    component(ui, decors, "Contents", |ui, _| {
        let items = edit::items(&text);
        let count = |kind: Kind| items.iter().filter(|one| one.kind == kind).count();
        for (label, value) in [
            ("Pieces", count(Kind::Piece)),
            ("Residents", count(Kind::Lives)),
            ("Markers", count(Kind::Spot) + count(Kind::Entrance) + count(Kind::Exit)),
            ("Zones", count(Kind::Zone)),
            ("Life Rules", edit::life_rules(&text).len()),
            ("Terrains", crate::terrains::terrain_lines(&text).len()),
            ("Groups", edit::groups(&text).len()),
            ("Lines", text.lines().count()),
        ] {
            prop(ui, label, |ui| ui.label(RichText::new(value.to_string()).color(theme::TEXT)));
        }
    });
    component(ui, decors, "Life Rules", |ui, _| {
        ui.label(theme::text("What the game does here (the actions of a .game: play, set, chapter, go, dungeon…)", theme::SMALL, theme::FAINT));
        let rules = edit::life_rules(&text);
        for rule in &rules {
            ui.label(RichText::new(&rule.head).monospace().size(theme::SMALL).color(theme::GAME));
            if let Some(body) = rule_field(ui, &rule.head, &rule.body) {
                made = Some(edit::with_rule(&text, &rule.head, &body));
            }
        }
        if !rules.iter().any(|rule| edit::rule_key(&rule.head) == "on enter")
            && ui.small_button("+ on enter").on_hover_text("what the game does as the team comes in").clicked()
        {
            made = Some(edit::with_rule(&text, "on enter", "# what the game does as the team comes in"));
        }
    });
    component(ui, decors, "Add", |ui, decors| add(ui, decors, &mut made));
    if let (Some(made), Some(open)) = (made, &mut decors.open) {
        open.change(made);
    }
}
