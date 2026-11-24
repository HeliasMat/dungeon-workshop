//! The project's windows over the Script: the films, the search in all of
//! them, what may be browsed.

use std::collections::BTreeMap;
use std::path::PathBuf;

use eframe::egui::{self, Color32, RichText, pos2, vec2};

use super::{ProjectSearch, Script};
use workshop_base::theme;

/// The Script's fields the project's windows use.
pub(super) struct Parts<'a> {
    session: &'a workshop_base::session::Session,
    search: &'a mut Option<ProjectSearch>,
    quick: &'a mut Option<(String, bool)>,
    beside: &'a mut Option<PathBuf>,
    versions_shown: &'a mut bool,
    version_chosen: &'a mut Option<String>,
    notes: &'a mut BTreeMap<usize, crate::workspace::Note>,
    note_editing: &'a mut Option<(usize, String)>,
    said: &'a mut Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn scripts_parts<'a>(
    session: &'a workshop_base::session::Session,
    search: &'a mut Option<ProjectSearch>,
    quick: &'a mut Option<(String, bool)>,
    beside: &'a mut Option<PathBuf>,
    versions_shown: &'a mut bool,
    version_chosen: &'a mut Option<String>,
    notes: &'a mut BTreeMap<usize, crate::workspace::Note>,
    note_editing: &'a mut Option<(usize, String)>,
    said: &'a mut Option<String>,
) -> Parts<'a> {
    Parts { session, search, quick, beside, versions_shown, version_chosen, notes, note_editing, said }
}

/// The project's windows: words sought and replaced in all the films; a film opened by its name (or put beside);
/// the versions compared and taken back; a note written.
pub(super) fn project_windows(ui: &mut egui::Ui, mut parts: Parts, open: &mut Script) {
    let context = ui.ctx().clone();
    // --- Sought in all the films ---
    let mut closed = false;
    if let Some(search) = parts.search.as_mut() {
        let mut shown = true;
        egui::Window::new("Search the Project").open(&mut shown).default_width(560.0).default_pos(pos2(160.0, 140.0)).show(&context, |ui| {
            searched(ui, parts.session, search, open, parts.said);
        });
        closed = !shown;
    }
    if closed || context.input(|input| input.key_pressed(egui::Key::Escape)) && parts.search.is_some() && parts.quick.is_none() && parts.note_editing.is_none()
    {
        *parts.search = None;
    }
    if parts.quick.is_some() {
        quick(&context, &mut parts);
    }
    if *parts.versions_shown {
        versions(&context, &mut parts, open);
    }
    note(&context, &mut parts, open);
}

/// The search in all the films: what is sought (a whole word, its case),
/// what replaces it (asked twice), where it is (a click opens it there).
fn searched(ui: &mut egui::Ui, session: &workshop_base::session::Session, search: &mut ProjectSearch, open: &mut Script, said: &mut Option<String>) {
    ui.horizontal(|ui| {
        ui.label(theme::text("Search", theme::SMALL, theme::WEAK));
        let field = ui.add(egui::TextEdit::singleline(&mut search.sought).desired_width(220.0));
        if search.searched.0.is_empty() && search.sought.is_empty() {
            field.request_focus();
        }
        ui.checkbox(&mut search.whole, "whole word");
        ui.checkbox(&mut search.case, "Aa");
    });
    ui.horizontal(|ui| {
        ui.label(theme::text("Replace with", theme::SMALL, theme::WEAK));
        ui.add(egui::TextEdit::singleline(&mut search.with).desired_width(220.0));
    });
    let wanted = (search.sought.clone(), search.whole, search.case);
    if wanted != search.searched {
        search.hits = if wanted.0.chars().count() >= 2 {
            crate::workspace::search(session.library(), &wanted.0, wanted.1, wanted.2, Some((&open.path, &open.text)))
        } else {
            Vec::new()
        };
        (search.searched, search.asked) = (wanted, false);
    }
    let files: Vec<PathBuf> = search.hits.iter().map(|hit| &hit.path).collect::<std::collections::BTreeSet<&PathBuf>>().into_iter().cloned().collect();
    ui.horizontal(|ui| {
        ui.label(theme::text(format!("{} time(s), in {} file(s)", search.hits.len(), files.len()), theme::SMALL, theme::WEAK));
        let asked = if search.asked { format!("Again: replace {} time(s) for good", search.hits.len()) } else { "Replace Everywhere".to_string() };
        if ui
            .add_enabled(!search.hits.is_empty(), egui::Button::new(asked))
            .on_hover_text("in each film where it is (the open one too); to rename someone everywhere: \"whole word\"")
            .clicked()
        {
            if search.asked {
                let count = replaced_everywhere(search, &files, open);
                *said = Some(format!("{count} replacement(s) in {} file(s)", files.len()));
                search.searched = (String::new(), false, false);
            }
            search.asked = !search.asked;
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
        let mut last: Option<&PathBuf> = None;
        for hit in &search.hits {
            if last != Some(&hit.path) {
                ui.label(RichText::new(hit.path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned())).strong().color(theme::FILM));
                last = Some(&hit.path);
            }
            let said: String = hit.text.chars().take(80).collect();
            if ui.add(egui::Button::new(theme::text(format!("  {:>4}   {said}", hit.line), theme::SMALL, theme::TEXT)).frame(false)).clicked() {
                session.ask(workshop_base::ask::Ask::Film { path: hit.path.clone(), line: hit.line });
            }
        }
    });
}

/// What is sought replaced in each of these files (the open film in its
/// text, the others on the disk); how many times.
fn replaced_everywhere(search: &ProjectSearch, files: &[PathBuf], open: &mut Script) -> usize {
    let mut count = 0;
    for path in files {
        if *path == open.path {
            let (made, here) = crate::workspace::replaced_in(&open.text, &search.sought, &search.with, search.whole, search.case);
            count += here;
            open.change(made);
        } else if let Ok(text) = std::fs::read_to_string(path) {
            let (made, here) = crate::workspace::replaced_in(&text.replace("\r\n", "\n"), &search.sought, &search.with, search.whole, search.case);
            if std::fs::write(path, made).is_ok() {
                count += here;
            }
        }
    }
    count
}

/// A film or a recipe opened by its name (or put beside).
fn quick(context: &egui::Context, parts: &mut Parts) {
    let Some((typed, beside)) = parts.quick.as_mut() else {
        return;
    };
    let files = crate::workspace::project_files(parts.session.library());
    let found = crate::workspace::matching(&files, typed);
    let mut chosen: Option<PathBuf> = None;
    let mut done = false;
    egui::Window::new(if *beside { "A Film Beside" } else { "Open" })
        .title_bar(false)
        .fixed_pos(pos2(context.screen_rect().center().x - 220.0, 80.0))
        .fixed_size(vec2(440.0, 320.0))
        .show(context, |ui| {
            let field = ui.add(egui::TextEdit::singleline(typed).hint_text("the name of a film or a recipe…").desired_width(f32::INFINITY));
            field.request_focus();
            if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                chosen = found.first().cloned();
            }
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                done = true;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for path in found.iter().take(40) {
                    let name = path.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
                    let folder = path.parent().and_then(|parent| parent.file_name()).map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                    if ui.add(egui::Button::new(RichText::new(format!("{name}   ")).strong()).frame(false)).on_hover_text(path.display().to_string()).clicked()
                    {
                        chosen = Some(path.clone());
                    }
                    ui.label(theme::text(folder, theme::SMALL - 1.0, theme::FAINT));
                }
            });
        });
    if let Some(path) = chosen {
        if *beside {
            *parts.beside = Some(path);
        } else {
            parts.session.ask(workshop_base::ask::Ask::Film { path, line: 1 });
        }
        done = true;
    }
    if done {
        *parts.quick = None;
    }
}

/// The versions: kept as it was written, the last commit; compared, taken back.
fn versions(context: &egui::Context, parts: &mut Parts, open: &mut Script) {
    let mut shown = true;
    let kept = crate::workspace::versions(parts.session.root(), &open.path);
    let mut taken: Option<String> = None;
    let chosen = &mut *parts.version_chosen;
    egui::Window::new("Versions").open(&mut shown).default_size(vec2(720.0, 480.0)).default_pos(pos2(200.0, 120.0)).show(context, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(170.0);
                ui.label(theme::text("kept while writing (one every two minutes at most)", theme::SMALL - 1.0, theme::FAINT));
                if ui.selectable_label(chosen.as_deref() == Some("commit"), "Last Commit").clicked() {
                    *chosen = Some("commit".to_string());
                }
                egui::ScrollArea::vertical().id_salt("kept versions").max_height(400.0).show(ui, |ui| {
                    for (when, path) in &kept {
                        let key = path.display().to_string();
                        if ui.selectable_label(chosen.as_deref() == Some(key.as_str()), crate::workspace::ago(*when)).clicked() {
                            *chosen = Some(key);
                        }
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| compared(ui, parts.session, chosen.as_deref(), open, &mut taken));
        });
    });
    if let Some(old) = taken {
        open.change(old);
    }
    *parts.versions_shown = shown;
}

/// How many lines went and came, from a version to now.
pub(super) fn counted(compared: &[crate::workspace::Diff]) -> (usize, usize) {
    compared.iter().fold((0, 0), |(gone, came), line| match line {
        crate::workspace::Diff::Gone(_) => (gone + 1, came),
        crate::workspace::Diff::Came(_) => (gone, came + 1),
        crate::workspace::Diff::Same(_) => (gone, came),
    })
}

/// What changed since the version chosen (in red: removed; in green: added); taken back if asked.
fn compared(ui: &mut egui::Ui, session: &workshop_base::session::Session, chosen: Option<&str>, open: &Script, taken: &mut Option<String>) {
    let old = match chosen {
        Some("commit") => crate::workspace::committed(session, &open.path),
        Some(path) => std::fs::read_to_string(path).ok(),
        None => None,
    };
    let Some(old) = old else {
        ui.label(theme::text("Choose a version: what changed since then shows here (in red: removed; in green: added).", theme::SMALL, theme::FAINT));
        return;
    };
    let compared = crate::workspace::diff(&old, &open.text);
    let (gone, came) = counted(&compared);
    ui.horizontal(|ui| {
        ui.label(theme::text(format!("since: −{gone} +{came} line(s)"), theme::SMALL, theme::WEAK));
        if ui.button("Go Back to This Version").on_hover_text("the text put back as it was (Ctrl+Z undoes it)").clicked() {
            *taken = Some(old.clone());
        }
    });
    egui::ScrollArea::both().id_salt("comparison").auto_shrink(false).show(ui, |ui| {
        for line in &compared {
            let (sign, said, shade) = match line {
                crate::workspace::Diff::Same(said) => (" ", said, theme::WEAK),
                crate::workspace::Diff::Gone(said) => ("−", said, Color32::from_rgb(0xF0, 0x71, 0x71)),
                crate::workspace::Diff::Came(said) => ("+", said, Color32::from_rgb(0x6C, 0xC6, 0x6C)),
            };
            ui.label(RichText::new(format!("{sign} {said}")).monospace().size(theme::SMALL).color(shade));
        }
    });
}

/// A note on a line, written (empty: removed), kept beside the film.
fn note(context: &egui::Context, parts: &mut Parts, open: &Script) {
    let mut finished: Option<bool> = None;
    if let Some((line, said)) = parts.note_editing.as_mut() {
        egui::Window::new(format!("Note · line {line}")).collapsible(false).resizable(false).default_pos(pos2(260.0, 220.0)).show(context, |ui| {
            ui.label(theme::text("in the margin, not in the film: to do, to look at again, an idea… (empty: removed)", theme::SMALL, theme::FAINT));
            ui.add(egui::TextEdit::multiline(said).desired_rows(4).desired_width(360.0)).request_focus();
            ui.horizontal(|ui| {
                if ui.button("Keep").clicked() {
                    finished = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    finished = Some(false);
                }
            });
        });
        if finished == Some(true) {
            if said.trim().is_empty() {
                parts.notes.remove(line);
            } else {
                let done = parts.notes.get(line).is_some_and(|note| note.done);
                parts.notes.insert(*line, crate::workspace::Note { done, said: said.trim().to_string() });
            }
            crate::workspace::write_notes(parts.session.library(), &open.path, parts.notes);
        }
    }
    if finished.is_some() {
        *parts.note_editing = None;
    }
}
