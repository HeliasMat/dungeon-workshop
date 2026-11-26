//! The Inspector panel: the line of the Script's cursor, when it plays, what
//! the language says of it, its fields.

use std::collections::HashMap;

use eframe::egui::{self, Color32, RichText, Sense, vec2};

use super::lines::{said_parts, with_field, with_line};
use super::playing::clock;
use super::timeline::{KINDS, kind_of_line};
use super::{Films, SECOND};
use workshop_base::theme;
use workshop_script::Scripts;

/// The line looked at: the film's text, its number, as written; what the
/// parts ask: the text made again, another line, a moment to go to.
struct Looked {
    text: String,
    line: usize,
    written: String,
    made: Option<String>,
    go_line: Option<usize>,
    go: Option<usize>,
}

pub fn inspector(ui: &mut egui::Ui, films: &mut Films, scripts: &mut Scripts) {
    let Some(open) = &scripts.open else {
        ui.label(theme::weak("No film open."));
        return;
    };
    let fresh = films.fresh(scripts);
    let text = open.text.clone();
    let line = open.cursor_line();
    let written = text.lines().nth(line.saturating_sub(1)).unwrap_or_default().to_string();
    let told = open.told_full(line);
    let mut looked = Looked { text, line, written, made: None, go_line: None, go: None };
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        head(ui, &mut looked);
        if let Some(parts) = said_parts(&looked.written) {
            spoken(ui, films, &mut looked, parts);
        } else if films.fields.get(&line).is_some_and(|read| read.word_at(&looked.written).is_some()) {
            fields(ui, films, &mut looked, fresh);
        }
        // --- What its word is ---
        let first_column = looked.written.chars().take_while(|letter| letter.is_whitespace()).count() + 1;
        if let Some(help) = film::service::hover(&looked.text, film::service::Spot { line, column: first_column }, films.session.library()) {
            egui::CollapsingHeader::new(RichText::new("The Word").strong()).default_open(false).show(ui, |ui| {
                ui.label(RichText::new(help).size(theme::SMALL).color(theme::TEXT));
            });
            ui.add_space(4.0);
        }
        // (A comment or an empty line plays nothing: no "When It Plays".)
        if !matches!(looked.written.trim_start().chars().next(), None | Some('#')) {
            when_it_plays(ui, films, &mut looked);
        }
        what_is_said(ui, films, &told, &mut looked);
    });
    if let Some(frame) = looked.go {
        films.go(frame);
    }
    if let Some(open) = scripts.open.as_mut() {
        if let Some(made) = looked.made {
            open.set_text(made);
        }
        if let Some(line) = looked.go_line {
            open.go_to_line(line);
        }
    }
}

/// The head: the line, what it is, the lines before and after; as it is written.
fn head(ui: &mut egui::Ui, looked: &mut Looked) {
    let lines: Vec<&str> = looked.text.lines().collect();
    let line = looked.line;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let kind = KINDS[kind_of_line(&looked.written)];
        ui.label(theme::text(format!("Line {line}"), theme::TITLE, theme::STRONG).strong());
        let (chip, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
        ui.painter().rect_filled(chip, 2.0, kind.1);
        ui.label(theme::text(kind.0, theme::SMALL, theme::WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let filled = |at: usize| lines.get(at - 1).is_some_and(|said| !said.trim().is_empty() && !said.trim_start().starts_with('#'));
            if ui.small_button("›").on_hover_text("the next line").clicked() {
                looked.go_line = (line + 1..=lines.len()).find(|at| filled(*at));
            }
            if ui.small_button("‹").on_hover_text("the line before").clicked() {
                looked.go_line = (1..line).rev().find(|at| filled(*at));
            }
        });
    });
    // (As written, wrapped in a box of its own: long lines read whole, not cut.)
    let shown = if looked.written.trim().is_empty() { "(empty)".to_string() } else { looked.written.trim().to_string() };
    egui::Frame::new().fill(theme::FIELD).corner_radius(3).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.add(egui::Label::new(RichText::new(shown).size(theme::SMALL).color(theme::TEXT)).wrap());
    });
    ui.add_space(6.0);
}

/// A line said written again: its lead, who, the face and manners (in brackets), what is said, on one line.
pub(super) fn spoken_line(lead: &str, who: &str, face: &str, manners: &str, said: &str) -> String {
    let inside: Vec<String> =
        [face.trim().to_string()].into_iter().chain(manners.split(',').map(|word| word.trim().to_string())).filter(|word| !word.is_empty()).collect();
    let face = if inside.is_empty() { String::new() } else { format!(" ({})", inside.join(", ")) };
    format!("{lead}{who}{face}: {}", said.replace('\n', " ").trim())
}

/// A line said: who, the face, how, what.
fn spoken(ui: &mut egui::Ui, films: &Films, looked: &mut Looked, (who, faces, manners, said): (String, String, String, String)) {
    workshop_decor::section(ui, "Line");
    ui.add_space(4.0);
    let (mut new_who, mut new_face, mut new_manners, mut new_said) = (who.clone(), faces.clone(), manners.clone(), said.clone());
    egui::Grid::new("speech line").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(theme::weak("Who"));
        egui::ComboBox::from_id_salt("who speaks").selected_text(&new_who).width(160.0).show_ui(ui, |ui| {
            for name in films.cast.iter().chain(["narrator".to_string()].iter()) {
                ui.selectable_value(&mut new_who, name.clone(), name);
            }
        });
        ui.end_row();
        ui.label(theme::weak("Face"));
        let shown = if new_face.is_empty() { "none".to_string() } else { new_face.clone() };
        egui::ComboBox::from_id_salt("face").selected_text(shown).width(160.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut new_face, String::new(), "none");
            let mut seen: Vec<&str> = Vec::new();
            for (_, face) in film::words::FACES {
                if !seen.contains(&face) {
                    seen.push(face);
                    ui.selectable_value(&mut new_face, face.to_string(), face);
                }
            }
        });
        ui.end_row();
        ui.label(theme::weak("Manner")).on_hover_text("fast, slow, top… (separated by commas)");
        ui.add(egui::TextEdit::singleline(&mut new_manners).hint_text("fast, slow, top…").desired_width(180.0));
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.label(theme::weak("What they say ([1s]: a pause, *word*: stressed)"));
    let field = ui.add(egui::TextEdit::multiline(&mut new_said).desired_rows(3).desired_width(f32::INFINITY));
    let typed_done = !field.has_focus();
    if (&new_who, &new_face, &new_manners) != (&who, &faces, &manners) || (typed_done && new_said != said) || (field.changed() && new_said.contains('\n')) {
        let lead: String = looked.written.chars().take_while(|letter| letter.is_whitespace()).collect();
        let rewritten = spoken_line(&lead, &new_who, &new_face, &new_manners, &new_said);
        looked.made = Some(with_line(&looked.text, looked.line, &rewritten));
    }
    ui.add_space(8.0);
}

/// How wide the names of the fields are, in their column (longer ones wrap).
const LABEL_WIDE: f32 = 104.0;

/// A field's name as the form gives it, made readable: one word of a few (`lines_or_catmull_or_bezier`) as
/// `lines | catmull | bezier`, the others' `_` as spaces.
pub(super) fn readable(name: &str) -> String {
    match choices_of(name) {
        Some(choices) => choices.join(" | "),
        None => name.replace('_', " ").trim().to_string(),
    }
}

/// The words a field takes, when its name lists them (`walk_or_run`).
pub(super) fn choices_of(name: &str) -> Option<Vec<&str>> {
    let choices: Vec<&str> = name.split("_or_").collect();
    (choices.len() > 1 && choices.iter().all(|choice| !choice.is_empty())).then_some(choices)
}

/// A number as a field writes it: whole without its point.
pub(super) fn number_said(number: f64) -> String {
    if number.fract() == 0.0 { format!("{number:.0}") } else { format!("{number}") }
}

/// A word's line: each of its fields, as its form reads them.
fn fields(ui: &mut egui::Ui, films: &mut Films, looked: &mut Looked, fresh: bool) {
    let line = looked.line;
    let Some(read) = films.fields.get(&line) else {
        return;
    };
    workshop_decor::section(ui, &format!("{} · {}", read.word, read.group));
    ui.add_space(4.0);
    if !fresh {
        ui.label(theme::weak("the film plays again: the fields come back in a moment"));
    }
    egui::Grid::new("fields").num_columns(2).spacing([10.0, 6.0]).min_col_width(LABEL_WIDE).show(ui, |ui| {
        for value in &read.values {
            let (_, said) = value.kind.told();
            // (The name as the form gives it, made readable, wrapped in its column.)
            ui.allocate_ui(vec2(LABEL_WIDE, 0.0), |ui| {
                ui.add(egui::Label::new(theme::weak(readable(&value.name))).wrap()).on_hover_text(format!("{}: {said}", value.name));
            });
            let now = value.words.join(" ");
            if value.at.is_none() {
                ui.label(theme::text(if now.is_empty() { "—".to_string() } else { format!("{now} (default)") }, theme::SMALL, theme::FAINT));
                ui.end_row();
                continue;
            }
            let chosen = ui.horizontal_wrapped(|ui| field(ui, value, &now, line, &films.cast, &mut films.typing, &mut films.picking)).inner;
            if let Some(words) = chosen.filter(|_| fresh) {
                films.typing.remove(&(line, value.name.clone()));
                let words: Vec<String> = words.split_whitespace().map(str::to_string).collect();
                match with_field(&looked.text, line, read, &value.name, &words) {
                    Ok(new) => looked.made = Some(new),
                    Err(why) => films.said = Some(why),
                }
            }
            ui.end_row();
        }
    });
    if let Some((picked_line, name)) = &films.picking
        && *picked_line == line
    {
        ui.label(theme::text(format!("A click on the Scene: \"{name}\""), theme::SMALL, theme::FILM));
    }
    ui.add_space(8.0);
}

/// A field, by its kind: someone of the cast, a number dragged, words typed
/// (a place: chosen on the Scene too); what is chosen for it.
fn field(
    ui: &mut egui::Ui,
    value: &film::grammar::Value,
    now: &str,
    line: usize,
    cast: &[String],
    typing: &mut HashMap<(usize, String), String>,
    picking: &mut Option<(usize, String)>,
) -> Option<String> {
    let mut chosen: Option<String> = None;
    // (One word of a few, as the form names them (`walk_or_run`): each a button, the one written lit.)
    if let Some(choices) = choices_of(&value.name) {
        // (In the cell's own wrapped row: what does not fit goes under, the row as high.)
        for choice in choices {
            if ui.selectable_label(choice == now, choice).clicked() && choice != now {
                chosen = Some(choice.to_string());
            }
        }
        return chosen;
    }
    match value.kind {
        film::grammar::Kind::Someone | film::grammar::Kind::Code => {
            egui::ComboBox::from_id_salt(("someone", &value.name)).selected_text(now).width(150.0).show_ui(ui, |ui| {
                for name in cast {
                    if ui.selectable_label(*name == now, name).clicked() {
                        chosen = Some(name.clone());
                    }
                }
            });
        }
        film::grammar::Kind::Number | film::grammar::Kind::Coordinate if now.parse::<f64>().is_ok() => {
            let mut number: f64 = now.parse().unwrap_or_default();
            let dragged = ui.add(egui::DragValue::new(&mut number).speed(1.0));
            if dragged.changed() {
                chosen = Some(number_said(number));
            }
        }
        _ => {
            let mut typed = typing.get(&(line, value.name.clone())).cloned().unwrap_or_else(|| now.to_string());
            let place = matches!(value.kind, film::grammar::Kind::Place);
            let room = if place { (ui.available_width() - 90.0).max(60.0) } else { ui.available_width().max(80.0) };
            // (A long value, as points one after the other: on lines of its own, wrapped.)
            let long = typed.chars().count() > 28;
            let edit = if long { egui::TextEdit::multiline(&mut typed).desired_rows(2) } else { egui::TextEdit::singleline(&mut typed) };
            let field = ui.add(edit.desired_width(room));
            if field.lost_focus() && typed.trim() != now && !typed.trim().is_empty() {
                chosen = Some(typed.trim().to_string());
            }
            if field.changed() {
                typing.insert((line, value.name.clone()), typed.clone());
            }
            if place && ui.small_button("on the scene").on_hover_text("a click on the Scene's image chooses it").clicked() {
                *picking = Some((line, value.name.clone()));
            }
        }
    }
    chosen
}

/// The times a line plays (from, to, beside the film), from the Timeline.
fn spans_of(films: &Films, line: usize) -> Vec<(usize, usize, bool)> {
    films
        .timeline
        .as_ref()
        .map(|timeline| {
            timeline
                .spans
                .iter()
                .filter(|span| span.number as usize == line)
                .map(|span| (span.start, span.end, span.label.is_some() || span.thread > 0))
                .collect()
        })
        .unwrap_or_default()
}

/// When it plays (a click: there), who is there then.
fn when_it_plays(ui: &mut egui::Ui, films: &Films, looked: &mut Looked) {
    workshop_decor::section(ui, "When It Plays");
    ui.add_space(4.0);
    let spans = spans_of(films, looked.line);
    if spans.is_empty() {
        ui.label(theme::weak("it does not play (or not yet: the film is playing)"));
    }
    for (start, end, beside) in spans.iter().take(8) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} → {}", clock(*start as f64), clock(*end as f64))).monospace());
            ui.label(theme::weak(format!("{:.1} s{}", (*end as f64 - *start as f64) / SECOND, if *beside { " · beside the film" } else { "" })));
            if ui.small_button("go").clicked() {
                looked.go = Some(*start);
            }
        });
    }
    if let (Some((start, ..)), Some(timeline)) = (spans.first(), films.timeline.as_ref()) {
        let mut there: Vec<&str> = timeline
            .tracks
            .values()
            .filter(|track| track.shown.iter().any(|(from, to)| from <= start && start <= to))
            .map(|track| track.name.as_str())
            .collect();
        there.sort_unstable();
        there.dedup();
        if !there.is_empty() {
            ui.label(theme::text(format!("on stage: {}", there.join(", ")), theme::SMALL, theme::WEAK));
        }
    }
    ui.add_space(8.0);
}

/// What is said of it (and how to put it right), what playing it said wrong, what was last said.
fn what_is_said(ui: &mut egui::Ui, films: &Films, told: &[film::service::Diagnostic], looked: &mut Looked) {
    workshop_decor::section(ui, "What Is Said of It");
    ui.add_space(4.0);
    let played: Vec<&String> = films.wrong.iter().filter(|(known, _)| *known as usize == looked.line).map(|(_, why)| why).collect();
    if told.is_empty() && played.is_empty() {
        ui.label(theme::weak("nothing to say"));
    }
    for one in told {
        let colour = if one.is_error() { Color32::from_rgb(0xF1, 0x4C, 0x4C) } else { Color32::from_rgb(0xCC, 0xA7, 0x00) };
        ui.label(theme::text(&one.message, theme::SMALL, colour));
        for remedy in one.fixes.iter().take(3) {
            if ui.small_button(&remedy.title).clicked() {
                looked.made = Some(film::service::applied(&looked.text, &remedy.edits));
            }
        }
    }
    for why in played {
        ui.label(theme::text(why, theme::SMALL, Color32::from_rgb(0xF1, 0x4C, 0x4C)));
    }
    if let Some(said) = &films.said {
        ui.add_space(6.0);
        ui.label(theme::text(said, theme::SMALL, theme::FAINT));
    }
}
