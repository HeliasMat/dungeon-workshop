//! A landmark, and someone who lives there: where they face, how far they
//! wander.

use assets::project::Project;
use eframe::egui::{self, RichText};

use super::badge_of;
use super::fields::rule_field;
use super::inspector::{component, head, prop, raw, remarks};
use crate::Decors;
use crate::edit::{self, Item, Kind};
use workshop_base::theme;

/// A landmark chosen: its name, where it is, its size and where it leads (an exit).
pub(super) fn landmark(ui: &mut egui::Ui, decors: &mut Decors, item: &Item) {
    let Some(open) = &decors.open else { return };
    let number = item.line;
    let text = open.text.clone();
    let line = text.lines().nth(number - 1).unwrap_or_default().to_string();
    let words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    head(ui, None, badge_of(decors, Some(item), false), &decors.item_name(item), &format!("line {number}"));
    remarks(ui, decors, &[number]);
    let mut made: Option<String> = None;
    let library = decors.session.library().clone();
    component(ui, decors, "Marker", |ui, _| {
        let mut name = item.name.clone();
        let mut point = item.corner;
        let mut size = item.size;
        // (An exit: to a game place by its number, or to a decor by its name, by one of its entrances.)
        let mut to = words.get(6).cloned().unwrap_or_else(|| "224".to_string());
        let mut entrance = words.get(7).cloned().unwrap_or_default();
        if item.kind != Kind::Exit {
            let hint = match item.kind {
                Kind::Spot => "a name, e.g. by_the_well",
                Kind::Zone => "a name, e.g. the_dock",
                _ => "a name, e.g. north_gate (optional)",
            };
            prop(ui, "Name", |ui| ui.add(egui::TextEdit::singleline(&mut name).hint_text(hint).desired_width(ui.available_width())));
        }
        prop(ui, "Position", |ui| {
            ui.add(egui::DragValue::new(&mut point[0]).speed(1.0).prefix("X "));
            ui.add(egui::DragValue::new(&mut point[1]).speed(1.0).prefix("Y "));
        });
        if matches!(item.kind, Kind::Exit | Kind::Zone) {
            prop(ui, "Size", |ui| {
                ui.add(egui::DragValue::new(&mut size[0]).speed(1.0).range(4.0..=4096.0).prefix("L "));
                ui.add(egui::DragValue::new(&mut size[1]).speed(1.0).range(4.0..=4096.0).prefix("H "));
            });
        }
        if item.kind == Kind::Exit {
            leads_to(ui, &library, number, &mut to, &mut entrance);
        }
        let (rewritten, unnamed) = rewritten(item.kind, &name, point, size, &to, &entrance);
        if rewritten.split_whitespace().ne(line.split_whitespace()) && !unnamed {
            made = Some(edit::with_lines(&text, &[(number, rewritten)]));
        }
    });
    if item.kind == Kind::Zone {
        let head = format!("on zone {}", item.name);
        let body = edit::life_rules(&text).into_iter().find(|rule| edit::rule_key(&rule.head) == head).map(|rule| rule.body).unwrap_or_default();
        component(ui, decors, "On Zone", |ui, _| {
            ui.label(theme::text(format!("{head} … end: what the game does when the team walks in"), theme::SMALL, theme::FAINT));
            if let Some(body) = rule_field(ui, &head, &body) {
                made = Some(edit::with_rule(&text, &head, &body));
            }
        });
    }
    component(ui, decors, "Line", |ui, decors| raw(ui, decors, number, &mut made));
    ui.add_space(8.0);
    if ui.button(RichText::new("Remove").color(theme::WRONG)).on_hover_text("Suppr").clicked() {
        made = Some(edit::removed(&text, &[(number, 0)]));
        if let Some(open) = &mut decors.open {
            open.choose(Vec::new());
        }
    }
    if let (Some(made), Some(open)) = (made, &mut decors.open) {
        open.change(made);
    }
}

/// Where an exit leads: a game place by its number, or a decor by its name,
/// by one of its entrances.
fn leads_to(ui: &mut egui::Ui, library: &Project, number: usize, to: &mut String, entrance: &mut String) {
    prop(ui, "Leads To", |ui| {
        let shown = if to.parse::<i64>().is_ok() { format!("game place {to}") } else { to.clone() };
        egui::ComboBox::from_id_salt(("exit to", number)).selected_text(shown).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
            let numbered = to.parse::<i64>().is_ok();
            if ui.selectable_label(numbered, "a game place, by its number").clicked() && !numbered {
                (*to, *entrance) = ("224".to_string(), String::new());
            }
            ui.separator();
            for decor in decor::names(library) {
                if ui.selectable_label(*to == decor, &decor).clicked() {
                    *to = decor;
                }
            }
        });
    });
    if let Ok(mut place) = to.parse::<i64>() {
        prop(ui, "Game Place", |ui| {
            if ui.add(egui::DragValue::new(&mut place).range(0..=999)).changed() {
                *to = place.to_string();
            }
        });
    } else {
        // (The entrances of the decor it leads to.)
        let there = std::fs::read_to_string(library.decors().join(format!("{to}.decor"))).unwrap_or_default();
        let named: Vec<String> = decor::doc::entrances(&there).into_iter().map(|(_, name)| name).filter(|name| !name.is_empty()).collect();
        prop(ui, "By Entrance", |ui| {
            let shown = if entrance.is_empty() { "its first".to_string() } else { entrance.clone() };
            egui::ComboBox::from_id_salt(("exit entrance", number)).selected_text(shown).show_ui(ui, |ui| {
                if ui.selectable_label(entrance.is_empty(), "its first").clicked() {
                    entrance.clear();
                }
                for name in &named {
                    if ui.selectable_label(*entrance == *name, name).clicked() {
                        *entrance = name.clone();
                    }
                }
            });
        });
    }
}

/// A landmark's line as its fields now say it, and whether it is a spot or
/// a zone left with no name.
fn rewritten(kind: Kind, name: &str, point: [f32; 2], size: [f32; 2], to: &str, entrance: &str) -> (String, bool) {
    let clean = |said: &str| said.split_whitespace().collect::<Vec<_>>().join("_");
    let number_of = |value: f32| format!("{}", value.round() as i64);
    let rewritten = match kind {
        Kind::Spot => format!("spot {} {} {}", clean(name), number_of(point[0]), number_of(point[1])),
        Kind::Entrance => format!("entrance {} {} {}", number_of(point[0]), number_of(point[1]), name.trim()).trim_end().to_string(),
        Kind::Zone => format!("zone {} {} {} {} {}", clean(name), number_of(point[0]), number_of(point[1]), number_of(size[0]), number_of(size[1])),
        _ => format!("exit {} {} {} {} to {to} {entrance}", number_of(point[0]), number_of(point[1]), number_of(size[0]), number_of(size[1]))
            .trim_end()
            .to_string(),
    };
    let unnamed = matches!(kind, Kind::Spot | Kind::Zone) && clean(name).is_empty();
    (rewritten, unnamed)
}

/// The way someone faces, of the words after his feet; how far he wanders.
pub(super) fn facing_and_wander(rest: &[String]) -> (usize, i64) {
    let facing = rest.first().and_then(|word| decor::lang::FACING.iter().position(|known| known == word)).unwrap_or(0);
    let wander = rest.iter().position(|word| word == "wanders").and_then(|at| rest.get(at + 1)?.parse().ok()).unwrap_or(0);
    (facing, wander)
}

/// Those words made again with that way and that wandering.
pub(super) fn with_facing_and_wander(rest: &[String], facing: usize, wander: i64) -> Vec<String> {
    let mut others: Vec<String> = Vec::new();
    let mut at = 0;
    while at < rest.len() {
        if at == 0 && decor::lang::FACING.contains(&rest[0].as_str()) {
        } else if rest[at] == "wanders" {
            at += 1;
        } else {
            others.push(rest[at].clone());
        }
        at += 1;
    }
    let mut out = vec![decor::lang::FACING[facing.min(7)].to_string()];
    if wander > 0 {
        out.push("wanders".to_string());
        out.push(wander.to_string());
    }
    out.extend(others);
    out
}
