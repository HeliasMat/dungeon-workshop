//! One piece of the decor: its look, its place, its pose; moved in the
//! decor's lines. Someone who lives there: `resident.rs`.

use decor::doc::Pose;
use eframe::egui::{self, RichText};

use super::inspector::{component, head, prop, raw, remarks, reset};
use super::{badge_of, cross_button, resident};
use crate::Decors;
use crate::edit::{self, Item};
use workshop_base::theme;

/// A piece being changed in the Inspector: its line, the point chosen, the
/// decor's text, whether someone lives there, its size; its pose as it was
/// and as it is made; the text made, what is chosen anew.
pub(super) struct Changing {
    pub number: usize,
    pub corner: usize,
    pub text: String,
    pub lives: bool,
    pub size: [f32; 2],
    pub pose: Pose,
    pub made_pose: Pose,
    pub made: Option<String>,
    pub chose: Option<Vec<(usize, usize)>>,
}

/// A piece or someone who lives there: where (each point), how drawn, in
/// which order; his way, his wandering, what he says; its line.
pub(super) fn piece(ui: &mut egui::Ui, decors: &mut Decors, item: &Item) {
    let (number, corner) = (item.line, item.at);
    let Some(pose) = decors.open.as_ref().and_then(|open| open.pose(number)) else {
        return;
    };
    let text = decors.open.as_ref().map(|open| open.text.clone()).unwrap_or_default();
    let lives = pose.word == "resident";
    let look = if lives { None } else { decors.art_now(&pose.name) };
    let moves = decors.pieces.iter().find(|piece| piece.name == pose.name).is_some_and(|piece| piece.moves);
    let size = decors.size_of(&pose.name);
    let title = if lives { format!("Resident · {}", pose.name) } else { decors.shown_name(&pose.name) };
    head(ui, look.as_ref(), badge_of(decors, Some(item), false), &title, &format!("{} {} · line {number}", pose.word, pose.name));
    remarks(ui, decors, &[number]);
    let mut changing = Changing { number, corner, text, lives, size, made_pose: pose.clone(), pose, made: None, chose: None };
    transform(ui, decors, &mut changing);
    if lives {
        resident::components(ui, decors, &mut changing);
    } else {
        rendering(ui, decors, &mut changing.made_pose, moves);
        order(ui, decors, &mut changing);
    }
    component(ui, decors, "Line", |ui, decors| raw(ui, decors, number, &mut changing.made));
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| duplicate_or_remove(ui, &mut changing));
    if changing.made.is_none() && changing.made_pose != changing.pose {
        changing.made = Some(edit::with_lines(&changing.text, &[(number, changing.made_pose.line())]));
    }
    if let Some(open) = &mut decors.open {
        if let Some(made) = changing.made {
            open.change(made);
        }
        if let Some(chose) = changing.chose {
            open.choose(chose);
        }
    }
}

/// A point put on the grid of 8.
pub(super) fn on_grid(point: [f32; 2]) -> [f32; 2] {
    [(point[0] / 8.0).round() * 8.0, (point[1] / 8.0).round() * 8.0]
}

/// Its points (each one chosen, taken out; one more, all on the grid), its size, flipped.
fn transform(ui: &mut egui::Ui, decors: &mut Decors, changing: &mut Changing) {
    let (number, corner, lives, [w, h]) = (changing.number, changing.corner, changing.lives, changing.size);
    component(ui, decors, "Transform", |ui, _| {
        let many = changing.made_pose.points.len() > 1;
        let mut gone: Option<usize> = None;
        for at in 0..changing.made_pose.points.len() {
            let label = if !many { if lives { "Feet".to_string() } else { "Position".to_string() } } else { format!("Point {}", at + 1) };
            let this = at == corner;
            prop(ui, &label, |ui| {
                let point = &mut changing.made_pose.points[at];
                ui.add(egui::DragValue::new(&mut point[0]).speed(1.0).prefix("X ").range(-4096.0..=8192.0));
                ui.add(egui::DragValue::new(&mut point[1]).speed(1.0).prefix("Y ").range(-4096.0..=8192.0));
                if many {
                    if this {
                        ui.label(theme::text("chosen", theme::SMALL, theme::DECOR));
                    } else if ui.small_button("view").on_hover_text("choose it").clicked() {
                        changing.chose = Some(vec![(number, at)]);
                    }
                    if cross_button(ui, "remove this point") {
                        gone = Some(at);
                    }
                }
            });
        }
        if let Some(at) = gone {
            changing.made_pose.points.remove(at);
            changing.chose = Some(vec![(number, 0)]);
        }
        let points = &mut changing.made_pose.points;
        prop(ui, "", |ui| {
            if ui.small_button("+ a point").on_hover_text("the same piece put again, beside").clicked() {
                let last = points.last().copied().unwrap_or_default();
                points.push([last[0] + w, last[1]]);
            }
            if ui.small_button("to the grid").on_hover_text("each point to the nearest multiple of 8").clicked() {
                for point in points.iter_mut() {
                    *point = on_grid(*point);
                }
            }
        });
        if !lives {
            prop(ui, "Size", |ui| ui.label(RichText::new(format!("{w} × {h}")).color(theme::TEXT)));
            prop(ui, "Flipped", |ui| {
                ui.checkbox(&mut changing.made_pose.turned, "");
                if reset(ui, changing.made_pose.turned) {
                    changing.made_pose.turned = false;
                }
            });
        }
    });
}

/// The layer a pose is drawn on, by its word.
pub(super) fn layer_now(layer: Option<&str>) -> &'static str {
    match layer {
        Some("ground") => "ground",
        Some("over") => "over",
        _ => "objects",
    }
}

/// A spot's name as typed, its words joined by `_`; none if empty.
pub(super) fn spot_word(typed: &str) -> Option<String> {
    let spot = typed.split_whitespace().collect::<Vec<_>>().join("_");
    (!spot.is_empty()).then_some(spot)
}

/// How a piece is drawn: its layer, animated (if it may be), its shadow,
/// locked, hidden, a spot at its foot.
fn rendering(ui: &mut egui::Ui, decors: &mut Decors, made_pose: &mut Pose, moves: bool) {
    component(ui, decors, "Rendering", |ui, _| {
        prop(ui, "Layer", |ui| {
            let now = layer_now(made_pose.layer.as_deref());
            for (word, name) in [("ground", "Ground"), ("objects", "Objects"), ("over", "Over")] {
                if ui.selectable_label(now == word, name).clicked() {
                    made_pose.layer = (word != "objects").then(|| word.to_string());
                }
            }
            if reset(ui, now != "objects") {
                made_pose.layer = None;
            }
        });
        if moves {
            prop(ui, "Animated (Game)", |ui| {
                let mut living = !made_pose.still;
                if ui.checkbox(&mut living, "").changed() {
                    made_pose.still = !living;
                }
                if reset(ui, made_pose.still) {
                    made_pose.still = false;
                }
            });
        }
        prop(ui, "Shadow", |ui| {
            let mut shadow = !made_pose.shadowless;
            if ui.checkbox(&mut shadow, "").changed() {
                made_pose.shadowless = !shadow;
            }
            if reset(ui, made_pose.shadowless) {
                made_pose.shadowless = false;
            }
        });
        prop(ui, "Locked", |ui| ui.checkbox(&mut made_pose.locked, ""));
        prop(ui, "Hidden", |ui| ui.checkbox(&mut made_pose.hidden, ""));
        prop(ui, "Spot at Its Foot", |ui| {
            let mut spot = made_pose.spot.clone().unwrap_or_default();
            let field = ui.add(egui::TextEdit::singleline(&mut spot).hint_text("a name for the films").desired_width(ui.available_width() - 24.0));
            if field.lost_focus() || (field.changed() && spot.is_empty()) {
                made_pose.spot = spot_word(&spot);
            }
        });
    });
}

/// Its line moved among the decor's: a step, or all the way.
fn order(ui: &mut egui::Ui, decors: &mut Decors, changing: &mut Changing) {
    let (text, number, made) = (&changing.text, changing.number, &mut changing.made);
    component(ui, decors, "Order", |ui, _| {
        prop(ui, "Drawn", |ui| {
            if ui.button("Behind").clicked() {
                *made = decors_moved(text, number, false);
            }
            if ui.button("In Front").clicked() {
                *made = decors_moved(text, number, true);
            }
        });
        prop(ui, "", |ui| {
            if ui.button("All the Way Back").clicked() {
                *made = edit::to_front(text, number, false);
            }
            if ui.button("All the Way Front").clicked() {
                *made = edit::to_front(text, number, true);
            }
        });
    });
}

/// Duplicated beside (its chosen point), or taken out.
fn duplicate_or_remove(ui: &mut egui::Ui, changing: &mut Changing) {
    if ui.button("Duplicate").on_hover_text("Ctrl+D").clicked() {
        let point = changing.pose.points.get(changing.corner).copied().unwrap_or_default();
        let line = Pose { points: vec![[point[0] + changing.size[0], point[1]]], lead: String::new(), ..changing.pose.clone() }.line();
        changing.made = Some(edit::with_line_added(&changing.text, &line, changing.lives));
    }
    if ui.button(RichText::new("Remove").color(theme::WRONG)).on_hover_text("Suppr").clicked() {
        changing.made = Some(edit::removed(&changing.text, &[(changing.number, changing.corner)]));
        changing.chose = Some(Vec::new());
    }
}

/// A line one step later (in front) or earlier.
pub(super) fn decors_moved(text: &str, number: usize, later: bool) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let to = if later { number + 1 } else { number.checked_sub(1)? };
    if to == 0 || to > lines.len() || edit::is_life(lines[to - 1]) || lines[to - 1].starts_with("decor:") {
        return None;
    }
    let mut made: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    made.swap(number - 1, to - 1);
    Some(made.join("\n") + "\n")
}
