//! The plan of the film over the text: its scenes, its moments, its recipes,
//! how long each plays.

use std::collections::BTreeMap;

use eframe::egui::{self, Color32, RichText};
use film::service::Spot;

use super::knowledge::place_said;
use super::text::{index_of, spot_of};
use super::{NAME, Script, Scripts, WRONG};
use workshop_base::session::Session;
use workshop_base::theme;

/// The Plan: the film's scenes (their places as the game names them, how
/// many lines are wrong in each), their moments; a click goes there.
pub(super) fn plan(ui: &mut egui::Ui, scripts: &mut Scripts, timing: &BTreeMap<usize, (f64, f64)>, numbers: &BTreeMap<String, f64>) {
    let Scripts { open, folded, notes, values, session, override_text, .. } = scripts;
    let Some(open) = open.as_mut() else { return };
    ui.horizontal(|ui| {
        ui.label(theme::text("LAYOUT", theme::SMALL, theme::WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Unfold").on_hover_text("unfold all").clicked() {
                folded.clear();
            }
            if ui.small_button("Fold").on_hover_text("fold all").clicked() {
                folded.extend(film::service::folds(&open.text).into_iter().map(|(first, _)| first));
            }
        });
    });
    let here = spot_of(&open.text, open.cursor).line;
    let has_numbers = open.text.lines().any(|line| line.trim_start().starts_with("let ") || line.contains("bag_"));
    let plan_high = if has_numbers { (ui.available_height() * 0.5).max(120.0) } else { ui.available_height() };
    let mut gone = egui::ScrollArea::vertical()
        .id_salt("the layout")
        .auto_shrink([false, true])
        .max_height(plan_high)
        .show(ui, |ui| outline(ui, session, open, here, timing))
        .inner;
    // (The notes of the margin and the comments that say what is still to do: a click goes there.)
    gone = notes_and_todos(ui, session, open, notes).or(gone);
    // (The film's numbers: where each is declared, given, read; what each is at the moment shown; other values
    // for those declared, played from the cursor.)
    let variables = crate::debug::variables(&open.text);
    if !variables.is_empty() {
        ui.separator();
        ui.label(theme::text("VARIABLES", theme::SMALL, theme::WEAK))
            .on_hover_text("the value at the moment the Scene shows; F9: a breakpoint, F10: the next line");
        let shown = egui::ScrollArea::vertical().id_salt("the variables").max_height(260.0).show(ui, |ui| {
            let gone = variables_listed(ui, &variables, numbers, values);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button("▶ Play from Here with These Values")
                    .on_hover_text("the film played from the cursor, each \"let\" at the value chosen (the text is not changed)")
                    .clicked()
                {
                    let tried: BTreeMap<String, f64> = values
                        .iter()
                        .filter(|(name, _)| variables.get(*name).is_some_and(|variable| variable.declared.is_some()))
                        .map(|(name, value)| (name.clone(), *value))
                        .collect();
                    *override_text = Some(crate::debug::with_values(&open.text, &tried));
                    session.ask(workshop_base::ask::Ask::PlayFrom(here));
                }
                if ui.small_button("The Text's Values").on_hover_text("the film played again as it is written").clicked() {
                    values.clear();
                    *override_text = Some(open.text.clone());
                }
            });
            gone
        });
        gone = shown.inner.or(gone);
    }
    if let Some(line) = gone {
        let at = index_of(&open.text, Spot { line, column: 1 });
        open.jump = Some((at, at));
    }
}

/// How long lines play, from the first begun to the last ended (`timing`: each line's start and length).
fn lasting(timing: &BTreeMap<usize, (f64, f64)>, lines: std::ops::RangeInclusive<usize>) -> Option<f64> {
    let played: Vec<(f64, f64)> = timing.range(lines).map(|(_, played)| *played).collect();
    let begins = played.iter().map(|(start, _)| *start).reduce(f64::min)?;
    let ends = played.iter().map(|(start, length)| start + length).reduce(f64::max)?;
    Some(ends - begins)
}

/// The film's scenes, their moments, its recipes: the line clicked.
fn outline(ui: &mut egui::Ui, session: &Session, open: &Script, here: usize, timing: &BTreeMap<usize, (f64, f64)>) -> Option<usize> {
    let outline = film::service::outline(&open.text);
    let wrong: Vec<usize> = open.told.iter().filter(|told| told.is_error()).map(|told| told.span.start.line).collect();
    let mut gone = None;
    if outline.is_empty() {
        ui.label(theme::weak("no scene yet: \"scene: place who…\""));
    }
    let mut scene_number = 0;
    for symbol in &outline {
        let lines = symbol.line..=symbol.end_line.max(symbol.line);
        let inside = lines.contains(&here);
        match symbol.kind {
            "scene" => {
                scene_number += 1;
                let wrong_here = wrong.iter().filter(|line| lines.contains(*line)).count();
                ui.horizontal(|ui| {
                    let said = RichText::new(format!("{scene_number}. {}", place_said(session, &symbol.name))).strong();
                    if ui.selectable_label(inside, said).on_hover_text(format!("line {} · {}", symbol.line, symbol.detail)).clicked() {
                        gone = Some(symbol.line);
                    }
                    if wrong_here > 0 {
                        ui.label(theme::text(format!("⚠ {wrong_here}"), theme::SMALL, WRONG));
                    }
                    // (How long it lasts, as it plays.)
                    if let Some(length) = lasting(timing, lines.clone()) {
                        ui.label(theme::text(clock(length), theme::SMALL - 1.0, theme::FAINT)).on_hover_text("how long it lasts, when the film plays it");
                    }
                });
            }
            "moment" => {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if ui.selectable_label(inside, theme::text(format!("· {}", symbol.name), theme::SMALL, theme::TEXT)).clicked() {
                        gone = Some(symbol.line);
                    }
                });
            }
            _ => {
                if ui.selectable_label(false, theme::text(format!("{}  {}", symbol.name, symbol.kind), theme::SMALL, theme::WEAK)).clicked() {
                    gone = Some(symbol.line);
                }
            }
        }
    }
    gone
}

/// The notes of the margin (each ticked done or not) and the text's `# TODO`s: the line clicked.
fn notes_and_todos(ui: &mut egui::Ui, session: &Session, open: &Script, notes: &mut BTreeMap<usize, crate::workspace::Note>) -> Option<usize> {
    let todos = crate::workspace::todos(&open.text);
    if notes.is_empty() && todos.is_empty() {
        return None;
    }
    let mut gone = None;
    ui.separator();
    ui.label(theme::text("NOTES AND TO DO", theme::SMALL, theme::WEAK)).on_hover_text("Ctrl+Shift+N: a note on the line; \"# TODO\" in the text");
    let mut changed = false;
    for (line, note) in notes.iter_mut() {
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut note.done, "").on_hover_text("done").changed();
            let said: String = note.said.lines().next().unwrap_or_default().chars().take(28).collect();
            if ui
                .add(egui::Button::new(theme::text(format!("{line} · {said}"), theme::SMALL, if note.done { theme::FAINT } else { theme::TEXT })).frame(false))
                .on_hover_text(&note.said)
                .clicked()
            {
                gone = Some(*line);
            }
        });
    }
    if changed {
        crate::workspace::write_notes(session.library(), &open.path, notes);
    }
    for (line, said) in todos {
        let said: String = said.chars().take(30).collect();
        if ui.add(egui::Button::new(theme::text(format!("{line} · {said}"), theme::SMALL, Color32::from_rgb(0x6A, 0x99, 0x55))).frame(false)).clicked() {
            gone = Some(line);
        }
    }
    gone
}

/// Each number of the film: what it is now, where it is declared, given, read (a click goes there), the value to
/// try for one declared.
fn variables_listed(
    ui: &mut egui::Ui,
    variables: &BTreeMap<String, crate::debug::Variable>,
    numbers: &BTreeMap<String, f64>,
    values: &mut BTreeMap<String, f64>,
) -> Option<usize> {
    let mut gone = None;
    for (name, variable) in variables {
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).monospace().size(theme::SMALL).color(NAME));
            let now = crate::debug::value_now(numbers, name);
            ui.label(theme::text(now.map_or("—".to_string(), |value| format!("= {value}")), theme::SMALL, theme::STRONG));
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let mut places: Vec<(usize, &str)> = variable.declared.iter().map(|(line, _)| (*line, "let")).collect();
            places.extend(variable.given.iter().map(|line| (*line, "=")));
            places.extend(variable.read.iter().map(|line| (*line, "read")));
            for (line, how) in places.into_iter().take(8) {
                if ui.small_button(format!("{how} {line}")).on_hover_text("go there").clicked() {
                    gone = Some(line);
                }
            }
            if let Some((_, written)) = &variable.declared {
                let value = values.entry(name.clone()).or_insert_with(|| written.parse().unwrap_or(0.0));
                ui.add(egui::DragValue::new(value).speed(0.1)).on_hover_text("the value to try (\"Play with These Values\")");
            }
        });
    }
    gone
}

/// A time in seconds as the Timeline says it (`0:12.3`).
pub(super) fn clock(seconds: f64) -> String {
    format!("{}:{:04.1}", (seconds / 60.0).floor() as u64, seconds % 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_lasts_from_its_first_line_begun_to_its_last_ended() {
        let timing: BTreeMap<usize, (f64, f64)> = [(2, (1.0, 2.0)), (3, (0.5, 1.0)), (4, (2.0, 4.0)), (9, (20.0, 1.0))].into_iter().collect();
        assert_eq!(lasting(&timing, 2..=4), Some(5.5));
        assert_eq!(lasting(&timing, 5..=8), None);
    }
}
