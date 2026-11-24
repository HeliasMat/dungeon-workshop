//! The Wiki beside the text: every word of the language sought by its
//! name or by what it does, by family, and a page for each (what it does,
//! its forms and their parts, an example to play, insert, copy; the words it
//! goes with; a recipe's source). F1 opens the page of the word at the
//! cursor (or pointed at).

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, FontId, RichText, Sense, Stroke, vec2};

use super::Scripts;
use super::cast::scene_pair;
use super::completion::line_word;
use super::edits::replaced;
use super::highlight::{Lights, painted};
use super::text::{line_at, word_at};
use workshop_base::theme;
use workshop_wiki::catalogue::{FAMILIES, Origin};
use workshop_wiki::{Page, family_name};

/// What is asked on a page: another page, the list again, the example
/// played or not, its line written at the cursor, the example copied, the recipe's source shown or not.
enum Asked {
    Open(String),
    Back,
    Play,
    Insert(String),
    Copy(String),
    Source,
}

/// The word whose page F1 opens, at a place of the text: the word there if
/// it has a page, else the word its line is said with.
pub(super) fn wiki_word_at(session: &workshop_base::session::Session, text: &str, at: usize) -> Option<String> {
    let pages = workshop_wiki::pages(session);
    let has_page = |word: &str| pages.iter().any(|page| page.word == word);
    if let Some(word) = word_at(text, at).map(|word| film::words::plain(&word)).filter(|word| has_page(word)) {
        return Some(word);
    }
    let (_, _, line) = line_at(text, at);
    line_word(text, &line).filter(|word| has_page(word))
}

/// The word pointed at, if it has a page: its page's line, to say in the
/// pointer's tooltip (F1 for the rest).
pub(super) fn wiki_said_of(session: &workshop_base::session::Session, text: &str, at: usize) -> Option<String> {
    let word = word_at(text, at).map(|word| film::words::plain(&word))?;
    let page = workshop_wiki::page_of(session, &word)?;
    Some(format!("{} — {}\nF1: its page in the wiki", page.word, page.one_line()))
}

/// An example's line made of the scene's own: Pikachu and Piplup made the
/// first two of the scene where it is written (whole words).
pub(super) fn line_for_scene(line: &str, a: &str, b: &str) -> String {
    let [first, second] = workshop_wiki::made::CAST;
    line.split(' ')
        .map(|word| {
            let (core, rest) = word.split_at(word.find(|letter: char| !(letter.is_alphanumeric() || letter == '_')).unwrap_or(word.len()));
            match core {
                core if core == first => format!("{a}{rest}"),
                core if core == second => format!("{b}{rest}"),
                _ => word.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl Scripts {
    /// That word's page open in the Wiki, the Wiki shown.
    pub(super) fn wiki_open(&mut self, word: &str) {
        // (Its example played at once, on the right of the page.)
        (self.wiki_shown, self.wiki_page, self.wiki_playing, self.wiki_source) = (true, Some(word.to_string()), true, false);
        self.wiki_sought.clear();
    }
}

/// The Wiki: a window of its own, beside the Workshop's (moved, made larger,
/// put on another screen), to read while writing; what it inserts goes to
/// the film at the cursor. Closed, the Wiki is put away.
pub(super) fn wiki_window(ctx: &egui::Context, scripts: &mut Scripts) {
    // (For a picture, `WORKSHOP_CAPTURE`: over the Workshop's own window, where the picture sees it.)
    if scripts.session.launch.capture.is_some() {
        egui::Window::new("Wiki")
            .default_size([1100.0, 700.0])
            .frame(egui::Frame::window(&ctx.style()).fill(theme::PANEL).inner_margin(10.0))
            .show(ctx, |ui| wiki_panel(ui, scripts));
        return;
    }
    let id = egui::ViewportId::from_hash_of("the wiki's window");
    let builder = egui::ViewportBuilder::default().with_title("Wiki · Dungeon Workshop").with_inner_size([1280.0, 820.0]).with_min_inner_size([640.0, 420.0]);
    ctx.show_viewport_immediate(id, builder, |ctx, _| {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(10.0)).show(ctx, |ui| wiki_panel(ui, scripts));
        if ctx.input(|input| input.viewport().close_requested()) {
            scripts.wiki_shown = false;
        }
    });
}

/// The width of a page's text when the example plays beside it (the rest: the example, large).
fn text_wide(full: f32) -> f32 {
    (full * 0.42).clamp(300.0, 560.0).min(full)
}

/// The Wiki: its search, then a page (its example playing on its right, large), or the words by family (those found, the best first).
pub(super) fn wiki_panel(ui: &mut egui::Ui, scripts: &mut Scripts) {
    ui.horizontal(|ui| {
        ui.label(theme::text("WIKI", theme::SMALL, theme::WEAK));
        ui.label(theme::text("every word: what it does, an example", theme::SMALL - 1.0, theme::FAINT));
    });
    let field = ui.add(
        egui::TextEdit::singleline(&mut scripts.wiki_sought).hint_text("search: a word, or what it does (jump, rain, camera…)").desired_width(f32::INFINITY),
    );
    if field.changed() && !scripts.wiki_sought.trim().is_empty() {
        scripts.wiki_page = None;
    }
    // (DS Mode, the film written `for ds`: each word's DS badge, and the words that export alone if asked.)
    let for_ds = scripts.open.as_ref().is_some_and(|open| film::ds::written_for_ds(&open.text));
    if for_ds {
        ui.checkbox(&mut scripts.wiki_ds_only, "DS only").on_hover_text("only the words that export to the DS (direct, or with a loss)");
    }
    let ds = Ds { shown: for_ds, only: for_ds && scripts.wiki_ds_only };
    ui.add_space(4.0);
    let pages = workshop_wiki::pages(&scripts.session);
    let asked = match scripts.wiki_page.clone().and_then(|word| pages.iter().find(|page| page.word == word)) {
        Some(page) => {
            let related = workshop_wiki::related(&pages, page, 10);
            let source = match (&page.origin, scripts.wiki_source) {
                (Some(Origin::Recipe { from, .. }), true) => workshop_wiki::recipe_source(&scripts.session, from, &page.word),
                _ => None,
            };
            // (The page's text on the left, scrolled; its example on the right, large, where it stays.)
            let full = ui.available_rect_before_wrap();
            let played = !page.retired && full.width() >= 640.0;
            let left = if played { egui::Rect::from_min_size(full.min, vec2(text_wide(full.width()), full.height())) } else { full };
            let mut text_ui = ui.new_child(egui::UiBuilder::new().max_rect(left).layout(egui::Layout::top_down(egui::Align::Min)));
            let asked = egui::ScrollArea::vertical()
                .id_salt(("wiki page", &page.word))
                .auto_shrink(false)
                .show(&mut text_ui, |ui| page_view(ui, scripts, page, &related, source.as_deref(), ds))
                .inner;
            if played {
                let right = egui::Rect::from_min_max(egui::pos2(left.max.x + 14.0, full.min.y), full.max);
                let mut shown = ui.new_child(egui::UiBuilder::new().max_rect(right).layout(egui::Layout::top_down(egui::Align::Min)));
                example_played(&mut shown, scripts, page);
            }
            ui.allocate_rect(full, Sense::hover());
            asked
        }
        None => word_list(ui, &pages, &scripts.wiki_sought, ds),
    };
    match asked {
        Some(Asked::Open(word)) => scripts.wiki_open(&word),
        Some(Asked::Back) => scripts.wiki_page = None,
        Some(Asked::Play) => scripts.wiki_playing = !scripts.wiki_playing,
        Some(Asked::Source) => scripts.wiki_source = !scripts.wiki_source,
        Some(Asked::Copy(example)) => ui.ctx().copy_text(example),
        Some(Asked::Insert(line)) => insert_line(scripts, &line),
        None => {}
    }
}

/// How the Wiki shows the DS: each word's badge (DS Mode), only the words
/// that export.
#[derive(Clone, Copy)]
struct Ds {
    shown: bool,
    only: bool,
}

/// A word's DS badge: `DS direct`, `DS with loss`, `not on DS`, coloured so;
/// what is lost or why, on hover.
fn ds_badge(ui: &mut egui::Ui, word: &str, size: f32) {
    let status = film::ds::status(word);
    let colour = match status {
        film::ds::Export::Direct => theme::DUNGEON,
        film::ds::Export::Close => theme::FILM,
        _ => theme::WRONG,
    };
    let why = match status {
        film::ds::Export::Close => film::ds::lost(word).map(|lost| format!("lost on DS: {lost}")),
        film::ds::Export::Direct => film::ds::how(word).map(|how| format!("on DS: {how}")),
        _ => Some(film::ds::refused(word)),
    };
    let badge = egui::Frame::new().stroke(Stroke::new(1.0_f32, colour)).corner_radius(3.0).inner_margin(egui::Margin::symmetric(4, 0));
    let shown = badge.show(ui, |ui| ui.label(theme::text(status.badge(), size, colour))).response;
    if let Some(why) = why {
        shown.on_hover_text(why);
    }
}

/// The words found, the best first; nothing sought: by family, each family folded.
fn word_list(ui: &mut egui::Ui, pages: &[Page], sought: &str, ds: Ds) -> Option<Asked> {
    let mut asked = None;
    let mut found = workshop_wiki::search::ranked(pages, &FAMILIES, sought);
    if ds.only {
        found.retain(|page| film::ds::status(&page.word).exports());
    }
    ui.label(theme::text(format!("{} word{}", found.len(), if found.len() == 1 { "" } else { "s" }), theme::SMALL, theme::FAINT));
    egui::ScrollArea::vertical().id_salt("wiki words").auto_shrink(false).show(ui, |ui| {
        if !sought.trim().is_empty() {
            for page in found.iter().take(80) {
                if word_row(ui, page, true, ds) {
                    asked = Some(Asked::Open(page.word.clone()));
                }
            }
            return;
        }
        for (family, name) in FAMILIES.iter().enumerate() {
            let inner: Vec<&&Page> = found.iter().filter(|page| page.family == family).collect();
            if inner.is_empty() {
                continue;
            }
            let name = name.to_string();
            egui::CollapsingHeader::new(RichText::new(format!("{name}  ({})", inner.len())).color(theme::STRONG)).id_salt(("wiki family", family)).show(
                ui,
                |ui| {
                    for page in inner {
                        if word_row(ui, page, false, ds) {
                            asked = Some(Asked::Open(page.word.clone()));
                        }
                    }
                },
            );
        }
    });
    asked
}

/// A word in the list: its name, what it does in a line (its family too, when found); whether it was clicked.
fn word_row(ui: &mut egui::Ui, page: &Page, with_family: bool, ds: Ds) -> bool {
    let response = ui
        .vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.horizontal(|ui| {
                ui.label(RichText::new(&page.word).monospace().color(theme::FILM));
                if with_family {
                    ui.label(theme::text(family_name(page.family), theme::SMALL - 1.0, theme::FAINT));
                }
                if ds.shown {
                    ds_badge(ui, &page.word, theme::SMALL - 2.0);
                }
            });
            ui.add(egui::Label::new(theme::text(page.one_line(), theme::SMALL - 1.0, theme::WEAK)).truncate());
            ui.add_space(3.0);
        })
        .response;
    let response = ui.interact(response.rect, ui.id().with(("wiki row", &page.word)), Sense::click());
    if response.hovered() {
        ui.painter().rect_stroke(response.rect.expand(1.0), 2.0, Stroke::new(1.0_f32, theme::GAP), egui::StrokeKind::Outside);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// A word's page: its name and family, what it does, its forms, its example
/// (played, inserted, copied), the words it goes with, a recipe's source.
fn page_view(ui: &mut egui::Ui, scripts: &mut Scripts, page: &Page, related: &[String], source: Option<&str>, ds: Ds) -> Option<Asked> {
    let mut asked = None;
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new(theme::text("‹ All the words", theme::SMALL, theme::WEAK)).frame(false)).clicked() {
            asked = Some(Asked::Back);
        }
        ui.label(theme::text(format!("· {}", family_name(page.family)), theme::SMALL, theme::FAINT));
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new(&page.word).monospace().size(theme::BODY + 4.0).color(theme::STRONG));
        if ds.shown {
            ds_badge(ui, &page.word, theme::SMALL - 1.0);
        }
    });
    ui.label(theme::text(page.origin_said(), theme::SMALL - 1.0, theme::FAINT));
    if ds.shown {
        let said = match film::ds::status(&page.word) {
            film::ds::Export::Direct => film::ds::how(&page.word).map(|how| format!("On DS: {how}.")),
            film::ds::Export::Close => film::ds::lost(&page.word).map(|lost| format!("On DS, with a loss: {lost}.")),
            _ => Some(format!("{}.", film::ds::refused(&page.word))),
        };
        if let Some(said) = said {
            ui.add(egui::Label::new(theme::text(said, theme::SMALL - 1.0, theme::WEAK)).wrap());
        }
    }
    ui.add_space(4.0);
    ui.label(theme::text(&page.explained, theme::BODY, theme::TEXT));
    ui.add_space(8.0);
    if page.retired {
        // (A word a film does not take as written: what the Script says of its line, no example to play.)
        heading(ui, "NOT WRITTEN IN A FILM");
        example_view(ui, page.focus_line(), 1, scripts.size.min(13.0));
        if scripts.wiki_refused.as_ref().is_none_or(|(word, _)| *word != page.word) {
            let told = film::service::diagnostics(&page.example, scripts.session.tables()).into_iter().find(|told| told.is_error());
            scripts.wiki_refused = Some((page.word.clone(), told.map(|told| told.message).unwrap_or_default()));
        }
        if let Some((_, told)) = &scripts.wiki_refused {
            ui.add(egui::Label::new(theme::text(told, theme::SMALL, theme::WEAK)).wrap());
        }
    } else {
        example_buttons(ui, scripts, page, &mut asked);
    }
    ui.add_space(8.0);
    heading(ui, "HOW IT IS WRITTEN");
    forms_view(ui, page);
    if !related.is_empty() {
        ui.add_space(8.0);
        heading(ui, "SEE ALSO");
        ui.horizontal_wrapped(|ui| {
            for word in related {
                if ui.add(egui::Button::new(RichText::new(word).monospace().size(theme::SMALL).color(theme::FILM))).clicked() {
                    asked = Some(Asked::Open(word.clone()));
                }
            }
        });
    }
    if matches!(page.origin, Some(Origin::Recipe { .. })) {
        ui.add_space(8.0);
        let said = if source.is_some() { "How it's made  (hide)" } else { "How it's made  (show its lines)" };
        if ui
            .add(egui::Button::new(theme::text(said, theme::SMALL, theme::STRONG)).frame(false))
            .on_hover_text("the recipe's own lines, simpler words")
            .clicked()
        {
            asked = Some(Asked::Source);
        }
        if let Some(source) = source {
            example_view(ui, source, 0, scripts.size.min(13.0));
        }
    }
    asked
}

/// The example, its line lit, and what to do with it: play it here, write its line at the cursor, copy it.
fn example_buttons(ui: &mut egui::Ui, scripts: &mut Scripts, page: &Page, asked: &mut Option<Asked>) {
    heading(ui, if page.written { "EXAMPLE" } else { "EXAMPLE (MADE FROM ITS FORM)" });
    example_view(ui, &page.example, page.focus, scripts.size.min(13.0));
    ui.horizontal_wrapped(|ui| {
        let play = if scripts.wiki_playing { "Stop the Example" } else { "▶ Play the Example" };
        if ui.button(play).on_hover_text("the example, on the right, played in a loop or stopped").clicked() {
            *asked = Some(Asked::Play);
        }
        if ui.button("Insert at Cursor").on_hover_text(format!("writes, under the cursor's line:\n{}", page.focus_line().trim())).clicked() {
            *asked = Some(Asked::Insert(page.focus_line().trim().to_string()));
        }
        if ui.button("Copy Example").on_hover_text("the whole little film, to paste").clicked() {
            *asked = Some(Asked::Copy(page.example.clone()));
        }
    });
}

/// The example playing, as large as the room allows (the whole scene seen), with what it is under it.
fn example_played(ui: &mut egui::Ui, scripts: &mut Scripts, page: &Page) {
    heading(ui, "THE EXAMPLE, PLAYING");
    ui.add_space(4.0);
    let room = ui.available_size();
    let width = room.x.min((room.y - 40.0).max(120.0) / 0.75).max(120.0);
    if scripts.wiki_playing {
        scripts.examples.show_from(ui, &page.example, page.focus, width);
    } else {
        scripts.examples.still(ui, &page.example, width);
    }
    ui.add_space(4.0);
    ui.label(theme::text(format!("{}  ·  in a loop, from a moment before its line", page.focus_line().trim()), theme::SMALL - 1.0, theme::FAINT));
}

/// The forms shown at once on a page (the others folded).
const FORMS_SHOWN: usize = 2;

/// A heading of a page.
fn heading(ui: &mut egui::Ui, said: &str) {
    ui.label(theme::text(said, theme::SMALL - 1.0, theme::WEAK));
}

/// The forms (the first ones, the others folded), then their parts, each once: the name, what it means, whether it
/// may be left out (and what it is then).
fn forms_view(ui: &mut egui::Ui, page: &Page) {
    let form_line = |ui: &mut egui::Ui, shown: &str| {
        ui.add(egui::Label::new(RichText::new(shown).monospace().color(theme::FILM)).wrap());
    };
    for form in page.forms.iter().take(FORMS_SHOWN) {
        form_line(ui, &form.shown);
    }
    if page.forms.len() > FORMS_SHOWN {
        let more = page.forms.len() - FORMS_SHOWN;
        egui::CollapsingHeader::new(theme::text(format!("{more} more form{}", if more == 1 { "" } else { "s" }), theme::SMALL, theme::WEAK))
            .id_salt(("wiki forms", &page.word))
            .show(ui, |ui| {
                for form in &page.forms[FORMS_SHOWN..] {
                    form_line(ui, &form.shown);
                }
            });
    }
    let parts = workshop_wiki::made::all_parts(&page.forms);
    if parts.is_empty() {
        return;
    }
    ui.add_space(6.0);
    heading(ui, "ITS PARTS");
    for part in parts {
        let mut job = egui::text::LayoutJob::default();
        let format = |size: f32, colour: egui::Color32, mono: bool| egui::TextFormat {
            font_id: if mono { FontId::monospace(size) } else { FontId::proportional(size) },
            color: colour,
            ..Default::default()
        };
        job.append(&part.name, 0.0, format(theme::SMALL, theme::TEXT, true));
        job.append(&format!("  {}", part.means), 0.0, format(theme::SMALL - 1.0, theme::WEAK, false));
        let left_out = match (&part.default, part.optional) {
            (Some(default), _) => format!(" · may be left out: {default}"),
            (None, true) => " · may be left out".to_string(),
            _ => String::new(),
        };
        job.append(&left_out, 0.0, format(theme::SMALL - 1.0, theme::FAINT, false));
        job.wrap.max_width = ui.available_width();
        ui.label(job);
    }
}

/// A film's text as the Script colours it, in a frame, that line lit (0: none).
fn example_view(ui: &mut egui::Ui, text: &str, lit: usize, size: f32) {
    egui::Frame::new().fill(theme::GROUND).inner_margin(6.0).corner_radius(3.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::horizontal().id_salt(("wiki example", text.len(), lit)).show(ui, |ui| {
            let font = FontId::monospace(size);
            let job = painted(text.trim_end(), &font, f32::INFINITY, &BTreeMap::new(), &Lights::new(), &BTreeSet::new());
            let galley = ui.fonts(|fonts| fonts.layout_job(job));
            let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
            if let Some(row) = lit.checked_sub(1).and_then(|row| galley.rows.get(row)) {
                let lit_rect = row.rect().translate(rect.min.to_vec2()).expand2(vec2(4.0, 0.0));
                ui.painter().rect_filled(
                    egui::Rect::from_x_y_ranges(rect.min.x - 4.0..=rect.max.x.max(lit_rect.max.x), lit_rect.y_range()),
                    2.0,
                    theme::HOVERED,
                );
            }
            ui.painter().galley(rect.min, galley, super::PLAIN);
        });
    });
}

/// A line written under the cursor's (as far in), or in place of an empty one; the cursor after it.
fn insert_line(scripts: &mut Scripts, line: &str) {
    let Some(open) = scripts.open.as_mut() else { return };
    let text = open.text.clone();
    let cursor = open.cursor.min(text.chars().count());
    let (_, start, current) = line_at(&text, cursor);
    let (a, b) = scene_pair(&text, cursor);
    let line = line_for_scene(line, &a, &b);
    let lead: String = current.chars().take_while(|letter| letter.is_whitespace()).collect();
    let end = start + current.chars().count();
    let (made, at) =
        if current.trim().is_empty() { replaced(&text, start..end, &format!("{lead}{line}")) } else { replaced(&text, end..end, &format!("\n{lead}{line}")) };
    open.change(made);
    open.jump = Some((at, at));
    scripts.completion = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_example_line_is_made_of_the_scene_and_f1_finds_the_word() {
        assert_eq!(line_for_scene("pikachu walk to piplup", "eevee", "krabby"), "eevee walk to krabby");
        assert_eq!(line_for_scene("piplup: Hi, pikachu!", "eevee", "krabby"), "krabby: Hi, eevee!");
        assert_eq!(line_for_scene("pikachu2 jump", "eevee", "krabby"), "pikachu2 jump", "whole words only");
        let session = workshop_base::session::Session::for_tests();
        let text = "scene: beach pikachu piplup\npikachu walk to right\nwait 1s";
        assert_eq!(wiki_word_at(&session, text, 31).as_deref(), Some("walk"), "the word under the cursor");
        assert_eq!(wiki_word_at(&session, text, 28).as_deref(), Some("walk"), "on someone: the line's word");
        assert_eq!(wiki_word_at(&session, text, 52).as_deref(), Some("wait"));
        assert!(wiki_said_of(&session, text, 37).is_some_and(|said| said.starts_with("walk — ") && said.contains("F1")));
    }
}
