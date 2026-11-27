//! What is offered as one writes: a word offered taken, what the line's word
//! wants (over the line), the words offered as a word is typed (the rest of
//! the one chosen in grey), the words by family to browse.

use eframe::egui::{self, Color32, FontId, Rect, RichText, Sense, pos2, vec2};
use film::service::Spot;
use workshop_base::session::Session;

use super::body::{Body, Written};
use super::cast::someone_here;
use super::completion::{after_space, chained, line_word, no_forms_here, offered, taken, word_first};
use super::edits::{replaced, with_uses};
use super::knowledge::{face_portrait, species_of};
use super::text::{letter_of_word, line_at, spot_of};
use super::{Completion, KEYWORD, OFFERED, PLAIN};
use workshop_base::theme;

/// The first forms of the line's word, when a word is being typed after it (to show what it takes, in order).
pub(super) fn forms_of_line(text: &str, completion: &Completion) -> Vec<String> {
    let (_, start, line) = line_at(text, completion.letters.start);
    let Some(first) = line_word(text, &line) else { return Vec::new() };
    if completion.letters.start <= start + line.chars().take_while(|letter| letter.is_whitespace()).count() {
        return Vec::new();
    }
    film::every_word()
        .find(|(_, word, ..)| *word == first)
        .map(|(_, word, forms, _)| forms.iter().take(2).map(|form| format!("{word} {form}")).collect())
        .unwrap_or_default()
}

/// What is written when a word of a family is taken: an example, whole; a
/// word said of someone, after someone of the scene; else the word. Whether
/// what follows is offered.
pub(super) fn family_word_written(session: &Session, text: &str, cursor: usize, word: &str) -> (String, bool) {
    let subject = workshop_wiki::catalogue::catalogue(session).iter().any(|entry| entry.word == word && entry.subject);
    match word.strip_prefix('\u{1}') {
        Some(example) => (example.to_string(), false),
        None if subject => (format!("{} {word} ", someone_here(text, cursor)), true),
        None => (format!("{word} "), true),
    }
}

/// A word offered taken (`taken`); a recipe of the user's library the film
/// does not use yet (said `(your library: uses <file>)`), its `uses` line too.
pub(super) fn taken_with_uses(text: &str, completion: &Completion, word: &str, said: &str) -> (String, (usize, usize)) {
    let (changed, chosen) = taken(text, completion, word);
    let file = said.rsplit_once("(your library: uses ").map(|(_, rest)| rest.trim_end_matches(')'));
    match file.and_then(|file| with_uses(&changed, file)) {
        Some((made, at, long)) if at <= chosen.0 => (made, (chosen.0 + long, chosen.1 + long)),
        Some((made, ..)) => (made, chosen),
        None => (changed, chosen),
    }
}

impl Body<'_> {
    /// A word offered taken (by the keys), in the place of the letters typed;
    /// a word taken from the words by family, written. What follows, offered.
    pub(super) fn take_offered(&mut self, accept: Option<usize>, browse_take: Option<String>) {
        let Body { scripts, open, .. } = self;
        if let Some(chosen) = accept
            && let Some(completion) = scripts.completion.take()
            && let Some((word, said)) = completion.choices.get(chosen)
        {
            let (changed, chosen) = taken_with_uses(&open.text, &completion, word, said);
            open.change(changed);
            open.jump = Some(chosen);
            if chosen.0 == chosen.1 {
                scripts.completion = chained(&scripts.session, &open.text, chosen.0);
            }
        }
        if let Some(word) = browse_take {
            scripts.browsing = None;
            let (changed, at) = replaced(&open.text, open.cursor..open.cursor, &format!("{word} "));
            open.change(changed);
            open.jump = Some((at, at));
            scripts.completion = chained(&scripts.session, &open.text, at);
        }
    }

    /// What the line's word wants, under the line as it is written: its forms, and what goes where the cursor is.
    pub(super) fn forms(&self, ui: &egui::Ui, written: &Written) {
        let open = &*self.open;
        if !self.focused || self.scripts.completion.is_some() || no_forms_here(&open.text, open.cursor) {
            return;
        }
        let (number, start, line) = line_at(&open.text, open.cursor);
        let lead = line.chars().take_while(|letter| letter.is_whitespace()).count();
        let first: String = line_word(&open.text, &line).unwrap_or_default();
        let column = open.cursor - start;
        let word_at_column = line.find(first.as_str()).map_or(lead, |byte| line[..byte].chars().count());
        if first.is_empty() || column <= word_at_column + first.chars().count() || !film::is_word(&first, &open.text) {
            return;
        }
        let read = word_first(&open.text, open.cursor);
        let forms = film::service::hover(&read, Spot { line: number + 1, column: lead + 1 }, self.scripts.session.library());
        let here = film::service::hover(&read, spot_of(&open.text, open.cursor), self.scripts.session.library());
        let Some(forms) = forms else { return };
        let place = pos2(written.at(start).min.x, written.at(open.cursor).max.y + 4.0);
        egui::Area::new(egui::Id::new("script form")).order(egui::Order::Foreground).pivot(egui::Align2::LEFT_TOP).fixed_pos(place).interactable(false).show(
            ui.ctx(),
            |ui| {
                egui::Frame::popup(ui.style()).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
                    ui.set_max_width(520.0);
                    let forms: String = forms.chars().take(220).collect();
                    ui.label(RichText::new(forms).size(theme::SMALL).color(theme::TEXT));
                    if let Some(here) = here.filter(|here| !here.starts_with(&first)) {
                        ui.label(RichText::new(format!("here: {here}")).size(theme::SMALL).color(KEYWORD));
                    }
                });
            },
        );
    }

    /// What may be typed: offered again as a word is typed (or after a space);
    /// the rest of the word chosen in grey; the words by family; the words offered.
    pub(super) fn offers(&mut self, ui: &egui::Ui, written: &Written, editor_rect: Rect) {
        if written.changed {
            self.offered_again(written);
        }
        self.rest_in_grey(written, editor_rect);
        if self.scripts.browsing.is_some() {
            self.family_words(ui, written);
        }
        let line_forms = self.scripts.completion.as_ref().map(|completion| forms_of_line(&self.open.text, completion)).unwrap_or_default();
        if self.scripts.completion.is_some() {
            self.words_offered(ui, written, &line_forms);
        }
    }

    /// The text changed by typing: a letter of a word, the words it may become; a space, what follows.
    fn offered_again(&mut self, written: &Written) {
        let Body { scripts, open, .. } = self;
        let last = if open.cursor > 0 { open.text.chars().nth(open.cursor - 1) } else { None };
        let typed = last.is_some_and(letter_of_word);
        let spaced = last == Some(' ') && open.text.len() == written.before.len() + 1;
        scripts.completion = if typed {
            offered(&scripts.session, &open.text, open.cursor, false)
        } else if spaced {
            offered(&scripts.session, &open.text, open.cursor, false).or_else(|| after_space(&scripts.session, &open.text, open.cursor))
        } else {
            None
        };
        scripts.hover = None;
        scripts.browsing = None;
    }

    /// The rest of the word chosen, in grey after the cursor (Tab writes it), at the end of a line.
    fn rest_in_grey(&self, written: &Written, editor_rect: Rect) {
        let open = &*self.open;
        let Some(completion) = &self.scripts.completion else { return };
        let Some((word, _)) = completion.choices.get(completion.chosen) else { return };
        let typed: String = open.text.chars().skip(completion.letters.start).take(completion.letters.len()).collect();
        let (_, line_start, line) = line_at(&open.text, open.cursor);
        let after_cursor: String = line.chars().skip(open.cursor - line_start).collect();
        if after_cursor.trim().is_empty() && film::words::plain(word).starts_with(&film::words::plain(&typed)) {
            let rest: String = word.chars().skip(typed.chars().count()).collect();
            let at = written.at(open.cursor);
            written.editor.painter_at(editor_rect).text(
                at.left_top(),
                egui::Align2::LEFT_TOP,
                rest,
                FontId::monospace(self.scripts.size),
                Color32::from_rgb(0x5C, 0x63, 0x6B),
            );
        }
    }

    /// The words by family, under the cursor: the families, their words, the
    /// one chosen (its forms, an example played); one taken written. Closed by a click elsewhere.
    fn family_words(&mut self, ui: &egui::Ui, written: &Written) {
        let Body { scripts, open, .. } = self;
        let Some(browse) = &mut scripts.browsing else { return };
        let at = written.at(open.cursor);
        let examples = &mut scripts.examples;
        let words = workshop_wiki::catalogue::of_family(&scripts.session, browse.family);
        let mut take: Option<String> = None;
        let area =
            egui::Area::new(egui::Id::new("script words")).order(egui::Order::Foreground).fixed_pos(at.left_bottom() + vec2(-4.0, 2.0)).show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(150.0);
                            for (index, name) in workshop_wiki::catalogue::FAMILIES.iter().enumerate() {
                                if ui.selectable_label(browse.family == index, *name).clicked() {
                                    (browse.family, browse.chosen) = (index, 0);
                                }
                            }
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(170.0);
                            egui::ScrollArea::vertical().id_salt("family words").max_height(320.0).show(ui, |ui| {
                                for (index, entry) in words.iter().enumerate() {
                                    let said = ui.selectable_label(browse.chosen == index, RichText::new(&entry.word).monospace());
                                    if index == browse.chosen && browse.moved {
                                        said.scroll_to_me(None);
                                    }
                                    if said.double_clicked() {
                                        take = Some(entry.word.clone());
                                    } else if said.clicked() {
                                        browse.chosen = index;
                                    }
                                }
                            });
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(320.0);
                            if let Some(entry) = words.get(browse.chosen)
                                && let Some(asked) = word_explained(ui, examples, entry)
                            {
                                take = Some(asked);
                            }
                            ui.add_space(6.0);
                            ui.label(theme::text("← → a family · ↑ ↓ a word · Enter writes it · Esc", theme::SMALL - 1.0, theme::FAINT));
                        });
                    });
                });
            });
        browse.moved = false;
        scripts.family = browse.family;
        let elsewhere =
            ui.input(|input| input.pointer.any_click()) && !area.response.rect.contains(ui.input(|input| input.pointer.interact_pos().unwrap_or_default()));
        if browse.fresh {
            browse.fresh = false;
        } else if elsewhere {
            scripts.browsing = None;
        }
        if let Some(word) = take {
            scripts.browsing = None;
            let (written, chain) = family_word_written(&scripts.session, &open.text, open.cursor, &word);
            let (changed, at) = replaced(&open.text, open.cursor..open.cursor, &written);
            open.change(changed);
            open.jump = Some((at, at));
            scripts.completion = if chain { chained(&scripts.session, &open.text, at) } else { None };
        }
    }

    /// The words offered, under the word typed: the forms of the line's word,
    /// the faces as portraits (or the words, with what they are), a move
    /// played; one clicked written.
    fn words_offered(&mut self, ui: &egui::Ui, written: &Written, line_forms: &[String]) {
        let Body { scripts, open, .. } = self;
        let Some(completion) = &mut scripts.completion else { return };
        completion.chosen = completion.chosen.min(completion.choices.len().saturating_sub(1));
        let at = written.at(completion.letters.start);
        let mut clicked: Option<usize> = None;
        let (portraits, examples) = (&mut scripts.portraits, &mut scripts.examples);
        egui::Area::new(egui::Id::new("script offers")).order(egui::Order::Foreground).fixed_pos(at.left_bottom() + vec2(-4.0, 2.0)).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).inner_margin(2).show(ui, |ui| {
                ui.set_width(340.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                // (The forms of the line's word: what it takes, in order.)
                for form in line_forms {
                    ui.label(RichText::new(form).monospace().size(theme::SMALL).color(theme::FILM));
                }
                if !line_forms.is_empty() {
                    ui.add_space(2.0);
                    ui.separator();
                }
                // (Faces: the speaker's portraits, a click takes one.)
                let faces =
                    !completion.choices.is_empty() && completion.choices.iter().all(|(word, _)| film::words::FACES.iter().any(|(face, _)| face == word));
                if faces {
                    clicked = faces_offered(ui, &scripts.session, portraits, &open.text, completion);
                }
                if let Some(index) = words_listed(ui, completion, if faces { 0 } else { OFFERED }) {
                    clicked = Some(index);
                }
                if completion.choices.len() > OFFERED {
                    ui.label(theme::text(format!("{} in all: go on writing, or ↑ ↓", completion.choices.len()), theme::SMALL - 1.0, theme::FAINT));
                }
                // (A move chosen: the one of the line uses it on another.)
                if let Some((name, about)) = completion.choices.get(completion.chosen)
                    && about == "move"
                {
                    ui.add_space(4.0);
                    ui.separator();
                    let user = completion.user.clone().unwrap_or_else(|| "pikachu".to_string());
                    examples.show(ui, &workshop_base::example::film_of_move(&user, name), 336.0);
                }
                // (A word of the user's library chosen: its recipe file played, its example.)
                if let Some((word, _)) = completion.choices.get(completion.chosen)
                    && let Some((_, recipe)) = film::own_recipe_file_of(scripts.session.library(), word)
                {
                    ui.add_space(4.0);
                    ui.separator();
                    examples.show(ui, &recipe, 300.0);
                }
                // (What the one chosen is, all of it.)
                if let Some((_, about)) = completion.choices.get(completion.chosen)
                    && about.chars().count() > 48
                {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.label(RichText::new(about).size(theme::SMALL).color(theme::WEAK));
                }
            });
        });
        let Some(index) = clicked else { return };
        completion.chosen = index;
        if let Some((word, said)) = completion.choices.get(index).cloned() {
            let (changed, chosen) = taken_with_uses(&open.text, completion, &word, &said);
            open.change(changed);
            open.jump = Some(chosen);
            scripts.completion = if chosen.0 == chosen.1 { chained(&scripts.session, &open.text, chosen.0) } else { None };
        } else {
            scripts.completion = None;
        }
    }
}

/// A word of a family, all of it: its name, its family, its forms, what it
/// does, an example (played); the word or the example asked to be written
/// (the example after a `\u{1}`).
fn word_explained(ui: &mut egui::Ui, examples: &mut workshop_base::example::Examples, entry: &workshop_wiki::catalogue::Entry) -> Option<String> {
    let mut take = None;
    ui.label(RichText::new(&entry.word).monospace().size(theme::BODY + 1.0).color(theme::STRONG));
    ui.label(theme::text(workshop_wiki::catalogue::family_shown(entry.family), theme::SMALL, theme::FILM));
    ui.add_space(4.0);
    ui.label(RichText::new(entry.shown()).monospace().size(theme::SMALL).color(KEYWORD));
    if !entry.doc.is_empty() {
        ui.label(theme::text(&entry.doc, theme::SMALL, theme::TEXT));
    }
    ui.add_space(6.0);
    ui.label(theme::weak("Example"));
    let example = workshop_wiki::catalogue::example(entry);
    ui.label(RichText::new(&example).monospace().size(theme::SMALL).color(theme::TEXT));
    examples.show(ui, &workshop_base::example::film_of(&example), 300.0);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Write the Word").clicked() {
            take = Some(entry.word.clone());
        }
        if ui.button("Write the Example").clicked() {
            take = Some(format!("\u{1}{example}"));
        }
    });
    take
}

/// The faces offered as the speaker's portraits, the one chosen lit; the one clicked given back.
fn faces_offered(
    ui: &mut egui::Ui,
    session: &Session,
    portraits: &mut std::collections::HashMap<String, Option<egui::TextureHandle>>,
    text: &str,
    completion: &Completion,
) -> Option<usize> {
    let mut clicked = None;
    let (_, _, line) = line_at(text, completion.letters.start);
    let who = line.split_whitespace().next().unwrap_or("pikachu").to_string();
    let species = species_of(session, text, &who);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (index, (word, _)) in completion.choices.iter().enumerate() {
            let (rect, response) = ui.allocate_exact_size(vec2(62.0, 64.0), Sense::click());
            if index == completion.chosen {
                ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(0x04, 0x39, 0x5E));
            } else if response.hovered() {
                ui.painter().rect_filled(rect, 3.0, theme::HOVERED);
            }
            if let Some(face) = face_portrait(session, ui.ctx(), portraits, &species, word) {
                ui.painter().image(
                    face.id(),
                    Rect::from_center_size(pos2(rect.center().x, rect.min.y + 24.0), vec2(40.0, 40.0)),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.painter().text(pos2(rect.center().x, rect.max.y - 7.0), egui::Align2::CENTER_CENTER, word, FontId::proportional(theme::SMALL - 1.0), PLAIN);
            if response.clicked() {
                clicked = Some(index);
            }
        }
    });
    clicked
}

/// The words offered around the one chosen (at most so many), each with what it is; the one clicked given back.
fn words_listed(ui: &mut egui::Ui, completion: &Completion, most: usize) -> Option<usize> {
    let mut clicked = None;
    let first = completion.chosen.saturating_sub(OFFERED - 1);
    for (index, (word, about)) in completion.choices.iter().enumerate().skip(first).take(most) {
        let (rect, response) = ui.allocate_exact_size(vec2(340.0, 20.0), Sense::click());
        if index == completion.chosen {
            ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(0x04, 0x39, 0x5E));
        }
        ui.painter().text(rect.left_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, word, FontId::monospace(12.5), PLAIN);
        let clip = ui.painter().with_clip_rect(rect);
        let about: String = about.chars().take(48).collect();
        clip.text(rect.right_center() - vec2(6.0, 0.0), egui::Align2::RIGHT_CENTER, about, FontId::proportional(theme::SMALL), theme::FAINT);
        if response.clicked() {
            clicked = Some(index);
        }
    }
    clicked
}
