//! The small windows over the text: what to do with the lines chosen (a
//! right click), a recipe named, what a word is (the pointer resting on it),
//! a name renamed everywhere, what is sought and replaced, a line asked for.

use std::ops::Range;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Key, Rect, RichText, Sense, pos2, vec2};

use super::body::{Body, RULER, Written};
use super::cast::scene_cast;
use super::completion::LineMade;
use super::edits::{each_line, library_recipe, lines_of, made_recipe, recipe_template, replaced, wrapped_in};
use super::knowledge::{Pictured, picture_of, portrait};
use super::text::{index_of, spot_of, word_at};
use super::wiki::wiki_said_of;
use super::{Finding, HOVER_AFTER};
use workshop_base::theme;

/// What is said of the place a word is at: what is wrong there, then what the language says of it.
pub(super) fn said_of_word(session: &workshop_base::session::Session, text: &str, told: &[film::service::Diagnostic], index: usize) -> String {
    let spot = spot_of(text, index);
    let wrong: Vec<String> = told
        .iter()
        .filter(|told| {
            told.span.start.line == spot.line
                && told.span.start.column <= spot.column
                && (told.span.end.line > spot.line || spot.column <= told.span.end.column)
        })
        .map(|told| format!("{}: {}", if told.is_error() { "Error" } else { "Remark" }, told.message))
        .collect();
    let what = film::service::hover(text, spot, session.library());
    wrong.into_iter().chain(what).collect::<Vec<_>>().join("\n\n")
}

/// The ways to change the lines chosen (from 0, the first and the last): their name, what they do, the text made.
fn line_changes(text: &str, range: (usize, usize)) -> [(&'static str, &'static str, LineMade<'_>); 5] {
    [
        ("Start Them Together", "together … end", Box::new(move || wrapped_in(text, range, "together"))),
        ("Play Them Beside the Film", "beside … end", Box::new(move || wrapped_in(text, range, "beside"))),
        (
            "Don't Wait for Them",
            "an & at the end of each",
            Box::new(move || {
                each_line(text, range, |line| {
                    if line.trim().is_empty() || line.trim_start().starts_with('#') || line.trim_end().ends_with('&') {
                        line.to_string()
                    } else {
                        format!("{} &", line.trim_end())
                    }
                })
            }),
        ),
        (
            "Comment Out",
            "# before each",
            Box::new(move || {
                each_line(text, range, |line| {
                    if line.trim().is_empty() {
                        line.to_string()
                    } else {
                        let lead = line.len() - line.trim_start().len();
                        format!("{}# {}", &line[..lead], &line[lead..])
                    }
                })
            }),
        ),
        (
            "Duplicate",
            "the same ones below",
            Box::new(move || {
                let mut lines: Vec<&str> = text.split('\n').collect();
                let copied: Vec<&str> = lines[range.0..=range.1].to_vec();
                lines.splice(range.1 + 1..range.1 + 1, copied);
                lines.join("\n")
            }),
        ),
    ]
}

/// What a field asked in a small window says: Esc no, Enter yes (nothing yet otherwise); the field given the keys.
fn field_answer(ui: &egui::Ui, field: &egui::Response) -> Option<bool> {
    if !field.has_focus() && !field.lost_focus() {
        field.request_focus();
    }
    if ui.input(|input| input.key_pressed(Key::Escape)) {
        Some(false)
    } else if field.lost_focus() {
        Some(ui.input(|input| input.key_pressed(Key::Enter)))
    } else {
        None
    }
}

impl Body<'_> {
    /// A right click on the lines chosen: what to do with them (played from
    /// there, in a block, made a recipe, commented, doubled).
    pub(super) fn lines_menu(&mut self, written: &Written) {
        let Body { scripts, open, .. } = self;
        let chosen_now = written.output.cursor_range.map_or((open.cursor, open.cursor), |range| (range.primary.index, range.secondary.index));
        let (first_chosen, last_chosen) = lines_of(&open.text, chosen_now);
        let mut lines_made: Option<String> = None;
        written.output.response.context_menu(|ui| {
            let many = last_chosen > first_chosen;
            ui.label(theme::text(
                if many { format!("Lines {} to {}", first_chosen + 1, last_chosen + 1) } else { format!("Line {}", first_chosen + 1) },
                theme::SMALL,
                theme::WEAK,
            ));
            if ui.button("Play from Here").clicked() {
                scripts.session.ask(workshop_base::ask::Ask::PlayFrom(first_chosen + 1));
                ui.close();
            }
            ui.separator();
            let text = &open.text;
            // (Once someone of the scene arrives; a recipe of them, named.)
            let cast = scene_cast(text, first_chosen + 1);
            if !cast.is_empty() {
                ui.menu_button("When … Arrives", |ui| {
                    for name in &cast {
                        if ui.button(format!("when {name} arrives")).clicked() {
                            lines_made = Some(wrapped_in(text, (first_chosen, last_chosen), &format!("when {name} arrives")));
                            ui.close();
                        }
                    }
                });
            }
            if ui.button("Make a Recipe of Them…").on_hover_text("define … end at the top of the film, or in a recipe file of your library; called in their place").clicked() {
                scripts.extracting = Some((first_chosen, last_chosen, String::new(), false));
                ui.close();
            }
            for (said, about, made) in line_changes(text, (first_chosen, last_chosen)) {
                if ui.button(said).on_hover_text(about).clicked() {
                    lines_made = Some(made());
                    ui.close();
                }
            }
        });
        if let Some(made) = lines_made {
            open.change(made);
        }
    }

    /// A recipe being named: its lines under that name, before the first scene, called where they were.
    pub(super) fn recipe_naming(&mut self, ui: &egui::Ui) {
        let Body { scripts, open, .. } = self;
        let Some((first, last, name, in_library)) = &mut scripts.extracting else { return };
        let (first, last) = (*first, *last);
        let mut done: Option<bool> = None;
        let named = film::words::plain(&name.trim().replace(' ', "_"));
        let file = film::user_recipes(scripts.session.library()).join(format!("{named}.film"));
        let taken = *in_library && !named.is_empty() && file.exists();
        egui::Window::new("Make a Recipe of Them").collapsible(false).resizable(false).show(ui.ctx(), |ui| {
            ui.label(theme::text(format!("lines {} to {}, under this name; called where they were", first + 1, last + 1), theme::SMALL, theme::WEAK));
            let field = ui.add(egui::TextEdit::singleline(name).hint_text("its name: hop_around").desired_width(240.0));
            if !field.has_focus() && !field.lost_focus() {
                field.request_focus();
            }
            ui.horizontal(|ui| {
                ui.radio_value(in_library, false, "in this film").on_hover_text("define … end at the top of the film");
                ui.radio_value(in_library, true, "in my library").on_hover_text("a recipe file of its own, library/recipes/<name>.film, these lines its example; the film uses it");
            });
            if taken {
                ui.label(theme::text("a recipe file of that name is already in your library", theme::SMALL, theme::WEAK));
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(!taken, egui::Button::new("Make the Recipe")).clicked() || (!taken && field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter))) {
                    done = Some(true);
                }
                if ui.button("Cancel").clicked() || ui.input(|input| input.key_pressed(Key::Escape)) {
                    done = Some(false);
                }
            });
        });
        match done {
            Some(true) => {
                if !named.is_empty() && *in_library {
                    let film_name = open.path.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
                    let (made, recipe) = library_recipe(&open.text, (first, last), &named, &film_name);
                    let written = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| workshop_base::safe::write(&file, recipe));
                    match written {
                        Ok(()) => open.change(made),
                        Err(error) => scripts.said = Some(format!("{}: {error}", file.display())),
                    }
                } else if !named.is_empty() {
                    let made = made_recipe(&open.text, (first, last), &named);
                    open.change(made);
                }
                scripts.extracting = None;
            }
            Some(false) => scripts.extracting = None,
            None => {}
        }
    }

    /// A new recipe file being named: `library/recipes/<name>.film` made from
    /// a ready recipe and its example, then opened (one of that name: opened).
    pub(super) fn new_recipe(&mut self, ui: &egui::Ui) {
        let Body { scripts, .. } = self;
        let Some(name) = &mut scripts.new_recipe else { return };
        let mut done: Option<bool> = None;
        egui::Window::new("New Recipe").collapsible(false).resizable(false).show(ui.ctx(), |ui| {
            ui.label(theme::text("a word of your own, in library/recipes/, with a small scene that shows it", theme::SMALL, theme::WEAK));
            let field = ui.add(egui::TextEdit::singleline(name).hint_text("its name: bow_twice").desired_width(240.0));
            if !field.has_focus() && !field.lost_focus() {
                field.request_focus();
            }
            ui.horizontal(|ui| {
                if ui.button("Make It").clicked() || (field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter))) {
                    done = Some(true);
                }
                if ui.button("Cancel").clicked() || ui.input(|input| input.key_pressed(Key::Escape)) {
                    done = Some(false);
                }
            });
        });
        let named = film::words::plain(&name.trim().replace(' ', "_"));
        match done {
            Some(true) if !named.is_empty() => {
                let path = film::user_recipes(scripts.session.library()).join(format!("{named}.film"));
                let written = if path.exists() {
                    Ok(())
                } else {
                    path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| workshop_base::safe::write(&path, recipe_template(&named)))
                };
                match written {
                    Ok(()) => scripts.session.ask(workshop_base::ask::Ask::Film { path, line: 3 }),
                    Err(error) => scripts.said = Some(format!("{}: {error}", path.display())),
                }
                scripts.new_recipe = None;
            }
            Some(_) => scripts.new_recipe = None,
            None => {}
        }
    }

    /// What a word is, the pointer resting on it a moment: what is said of it,
    /// someone's portrait (and sheet), a place seen.
    pub(super) fn word_pointed(&mut self, ui: &egui::Ui, written: &Written) {
        let Some(pointer) = written.output.response.hover_pos().filter(|_| self.scripts.completion.is_none()) else {
            self.scripts.hover = None;
            return;
        };
        let Body { scripts, open, text_id, .. } = self;
        let index = written.index_at(pointer);
        match &mut scripts.hover {
            Some((known, since, said)) if *known == index => {
                if said.is_none() && since.elapsed() >= HOVER_AFTER {
                    let mut told = said_of_word(&scripts.session, &open.text, &open.told, index);
                    // (A word of the language: what it does in a line, and F1 for its page.)
                    if let Some(wiki) = wiki_said_of(&scripts.session, &open.text, index) {
                        told = if told.is_empty() { wiki } else { format!("{wiki}\n\n{told}") };
                    }
                    *said = Some(told);
                }
                let pictured = picture_of(&scripts.session, &open.text, index);
                let told = said.as_ref().filter(|said| !said.is_empty());
                if told.is_some() || (pictured.is_some() && since.elapsed() >= HOVER_AFTER) {
                    egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), text_id.with("said"), egui::PopupAnchor::Pointer).gap(14.0).show(|ui| {
                        ui.set_max_width(420.0);
                        match &pictured {
                            Some(Pictured::Someone(species, name)) => {
                                ui.horizontal(|ui| {
                                    if let Some(face) = portrait(&scripts.session, ui.ctx(), &mut scripts.portraits, species) {
                                        ui.add(egui::Image::new(&face).fit_to_exact_size(vec2(48.0, 48.0)));
                                    }
                                    ui.label(RichText::new(name).strong().color(theme::STRONG));
                                });
                                // (Its sheet, if one was written: Script › Characters.)
                                let sheet = word_at(&open.text, index).map(|word| crate::story::sheet(scripts.session.library(), &word)).unwrap_or_default();
                                if !sheet.trim().is_empty() {
                                    ui.label(RichText::new(sheet.trim()).italics().size(theme::SMALL).color(theme::WEAK));
                                }
                            }
                            Some(Pictured::Place(word, name)) => {
                                ui.label(RichText::new(name).strong().color(theme::STRONG));
                                scripts.examples.still(ui, &format!("scene: {word}\nwait 1.5s\n"), 240.0);
                            }
                            None => {}
                        }
                        if let Some(said) = told {
                            if pictured.is_some() {
                                ui.separator();
                            }
                            ui.label(RichText::new(said.as_str()).size(theme::SMALL + 1.0));
                        }
                    });
                } else {
                    ui.ctx().request_repaint_after(Duration::from_millis(100));
                }
            }
            _ => {
                scripts.hover = Some((index, Instant::now(), None));
                ui.ctx().request_repaint_after(HOVER_AFTER);
            }
        }
    }

    /// A name renamed everywhere (F2): the new name asked under it, Enter renames.
    pub(super) fn renaming(&mut self, ui: &egui::Ui, written: &Written) {
        let Body { scripts, open, .. } = self;
        let Some((spot, word)) = &mut scripts.renaming else { return };
        let at = written.at(index_of(&open.text, *spot));
        let mut done: Option<bool> = None;
        egui::Area::new(egui::Id::new("script rename")).order(egui::Order::Foreground).fixed_pos(at.left_bottom() + vec2(0.0, 2.0)).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(theme::text("Rename Everywhere (Enter)", theme::SMALL, theme::WEAK));
                let field = ui.add(egui::TextEdit::singleline(word).desired_width(200.0));
                done = field_answer(ui, &field);
            });
        });
        if let Some(keep) = done {
            if keep {
                let made = film::service::renamed(&open.text, *spot, word.trim());
                open.change(made);
            }
            scripts.renaming = None;
        }
    }

    /// What is sought, replaced, over the text on the right: the one found
    /// looked at, gone to; replaced, or all of them.
    pub(super) fn find_bar(&mut self, ui: &egui::Ui, editor_rect: Rect, found: &[Range<usize>]) {
        let Body { scripts, open, .. } = self;
        let Some(finding) = &mut scripts.finding else { return };
        let mut asked = Asked::default();
        let count = found.len();
        finding.at = finding.at.min(count.saturating_sub(1));
        egui::Area::new(egui::Id::new("script find"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(editor_rect.right_top() + vec2(-RULER - 8.0, 6.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).inner_margin(6).show(ui, |ui| {
                    ui.horizontal(|ui| find_row(ui, finding, count, &mut asked));
                    if finding.replacing {
                        ui.horizontal(|ui| asked.replaced = replace_row(ui, finding, &open.text, found));
                    }
                });
            });
        if let Some(at) = asked.jump.and_then(|at| found.get(at)) {
            open.jump = Some((at.end, at.start));
            // (The field keeps the keys: the text is only shown there.)
            finding.focus = true;
        }
        if let Some(made) = asked.replaced {
            open.change(made);
        }
        if asked.close {
            scripts.finding = None;
        }
    }

    /// A line asked for (Ctrl+G): its number typed, Enter goes there.
    pub(super) fn going_to(&mut self, ui: &egui::Ui, editor_rect: Rect) {
        let Body { scripts, open, .. } = self;
        let Some(typed) = &mut scripts.going_to else { return };
        let mut done: Option<bool> = None;
        egui::Area::new(egui::Id::new("script line"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_TOP)
            .fixed_pos(pos2(editor_rect.center().x, editor_rect.min.y + 8.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).inner_margin(6).show(ui, |ui| {
                    ui.label(theme::text(format!("Go to Line (1 to {})", open.text.split('\n').count()), theme::SMALL, theme::WEAK));
                    let field = ui.add(egui::TextEdit::singleline(typed).desired_width(160.0));
                    done = field_answer(ui, &field);
                });
            });
        if let Some(keep) = done {
            if keep && let Ok(line) = typed.trim().parse::<usize>() {
                open.go_to_line(line.max(1));
            }
            scripts.going_to = None;
        }
    }
}

/// What the find bar asks: the one found to go to, the text with what was replaced, to be closed.
#[derive(Default)]
struct Asked {
    jump: Option<usize>,
    replaced: Option<String>,
    close: bool,
}

/// The find bar's first row: what is sought (Enter the next, Shift+Enter the
/// one before), how many found, the case, the ways, Replace, close.
fn find_row(ui: &mut egui::Ui, finding: &mut Finding, count: usize, asked: &mut Asked) {
    let field = ui.add(egui::TextEdit::singleline(&mut finding.sought).hint_text("Find").desired_width(180.0));
    if finding.focus {
        field.request_focus();
        finding.focus = false;
    }
    if field.changed() {
        finding.at = 0;
        asked.jump = Some(0);
    }
    let (enter, shift) = ui.input(|input| (input.key_pressed(Key::Enter), input.modifiers.shift));
    if field.lost_focus() && enter && count > 0 {
        finding.at = if shift { (finding.at + count - 1) % count } else { (finding.at + 1) % count };
        asked.jump = Some(finding.at);
        field.request_focus();
    }
    ui.label(theme::text(if count == 0 { "none".to_string() } else { format!("{} of {count}", finding.at + 1) }, theme::SMALL, theme::WEAK));
    if ui.selectable_label(finding.case, "Aa").on_hover_text("match case").clicked() {
        finding.case = !finding.case;
    }
    let icon = |ui: &mut egui::Ui, said: &str, draw: &dyn Fn(&egui::Painter, Rect, Color32)| {
        let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
        workshop_base::icons::button(ui, rect, said, draw)
    };
    if icon(ui, "the one before (Shift+Enter)", &|painter, at, tint| workshop_base::icons::way(painter, at, 4, tint)) && count > 0 {
        finding.at = (finding.at + count - 1) % count;
        asked.jump = Some(finding.at);
    }
    if icon(ui, "the next one (Enter)", &|painter, at, tint| workshop_base::icons::way(painter, at, 0, tint)) && count > 0 {
        finding.at = (finding.at + 1) % count;
        asked.jump = Some(finding.at);
    }
    if ui.selectable_label(finding.replacing, "Replace").on_hover_text("Ctrl+H").clicked() {
        finding.replacing = !finding.replacing;
    }
    if icon(ui, "close (Esc)", &workshop_base::icons::cross) || ui.input(|input| input.key_pressed(Key::Escape)) {
        asked.close = true;
    }
}

/// The find bar's second row: what replaces, the one looked at replaced, or all; the text then.
fn replace_row(ui: &mut egui::Ui, finding: &mut Finding, text: &str, found: &[Range<usize>]) -> Option<String> {
    let mut made = None;
    ui.add(egui::TextEdit::singleline(&mut finding.with).hint_text("Replace with").desired_width(180.0));
    if ui.button("Replace").clicked()
        && let Some(range) = found.get(finding.at)
    {
        made = Some(replaced(text, range.clone(), &finding.with).0);
    }
    if ui.button("All").on_hover_text("each of those found").clicked() && !found.is_empty() {
        let mut all = text.to_string();
        for range in found.iter().rev() {
            all = replaced(&all, range.clone(), &finding.with).0;
        }
        made = Some(all);
    }
    made
}
