//! Around the text: the margin (each line's number and what is said of it,
//! folds, bulbs, notes, marks), what follows the end of the lines (folded
//! lines, sounds heard, hints), the fixes of a line, the lines of the scene
//! kept at the top, the ruler on the right.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, pos2, vec2};

use super::body::{Body, Marks, RULER, Written};
use super::heard::{hear, heard_on};
use super::plan::clock;
use super::text::spot_of;
use super::{FACE, GUTTER, LABEL, REMARK, WRONG, worst};
use workshop_base::theme;

/// What the margin found as it was drawn: each line shown (its top, its
/// bottom, its number), where each line ends, the lines with a fix, the folds
/// by their first line, the pointer over the margin, the cursor's line.
pub(super) struct Margin {
    pub(super) numbered: Vec<(f32, f32, usize)>,
    pub(super) ends: BTreeMap<usize, Rect>,
    pub(super) fixable: BTreeSet<usize>,
    pub(super) fold_starts: BTreeMap<usize, usize>,
    pub(super) in_margin: Option<Pos2>,
    pub(super) cursor_line: usize,
}

/// What the margin draws a line with, known once for all the lines.
struct Known {
    playing: Option<usize>,
    leads: Vec<usize>,
    found_lines: BTreeSet<usize>,
    space_wide: f32,
    who_lines: BTreeSet<usize>,
}

/// A line's first row, where it is drawn: its number, its middle, its top and bottom, its left.
struct RowAt {
    line: usize,
    y: f32,
    top: f32,
    bottom: f32,
    left: f32,
}

/// The lines (from 1) where someone is named, outside the comments.
pub(super) fn lines_naming(text: &str, who: &str) -> BTreeSet<usize> {
    text.split('\n')
        .enumerate()
        .filter(|(_, line)| {
            !line.trim_start().starts_with('#') && line.split(|letter: char| !letter.is_alphanumeric() && letter != '_').any(|word| word == who)
        })
        .map(|(at, _)| at + 1)
        .collect()
}

/// The line (from 1) a height on the ruler stands for.
pub(super) fn line_on_ruler(y: f32, top: f32, height: f32, lines: usize) -> usize {
    (((y - top) / height * lines as f32) as usize + 1).min(lines)
}

/// A triangle pointing right, its tip there: ▶.
fn play_arrow(painter: &egui::Painter, tip: Pos2, colour: Color32) {
    painter.add(egui::Shape::convex_polygon(vec![tip + vec2(-5.0, -5.0), tip + vec2(3.0, 0.0), tip + vec2(-5.0, 5.0)], colour, Stroke::NONE));
}

impl Body<'_> {
    /// The margin drawn row by row (the lines folded away skipped); what it found given back.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn margin(
        &self,
        ui: &egui::Ui,
        written: &Written,
        editor_rect: Rect,
        marks: &Marks,
        found: &[Range<usize>],
        folds: &[(usize, usize)],
        hidden: &BTreeSet<usize>,
    ) -> Margin {
        let (scripts, open) = (&*self.scripts, &*self.open);
        let painter = written.editor.painter_at(editor_rect);
        let mut margin = Margin {
            numbered: Vec::new(),
            ends: BTreeMap::new(),
            fixable: open.told.iter().filter(|told| !told.fixes.is_empty()).map(|told| told.span.start.line).collect(),
            fold_starts: folds.iter().map(|(first, last)| (*first, *last)).collect(),
            in_margin: ui.input(|input| input.pointer.hover_pos()).filter(|at| editor_rect.contains(*at) && at.x < editor_rect.min.x + GUTTER - 2.0),
            cursor_line: spot_of(&open.text, open.cursor).line,
        };
        let lit_who = scripts.lit_who_scene.clone().or(scripts.lit_who_track.clone());
        let known = Known {
            playing: self.seen.stepped.or(self.seen.line),
            leads: open.text.split('\n').map(|line| line.chars().take_while(|letter| *letter == ' ').count()).collect(),
            found_lines: found.iter().map(|range| spot_of(&open.text, range.start).line).collect(),
            space_wide: ui.fonts(|fonts| fonts.glyph_width(&FontId::monospace(scripts.size), ' ')),
            who_lines: lit_who.as_ref().map(|who| lines_naming(&open.text, who)).unwrap_or_default(),
        };
        let origin = written.origin;
        let mut line = 1;
        let mut starts = true;
        for row in &written.galley.rows {
            // (The last row of each line: where its end is.)
            if !hidden.contains(&line) {
                margin.ends.insert(line, row.rect().translate(origin.to_vec2()));
            }
            if starts && hidden.contains(&line) {
                line += 1;
                starts = row.ends_with_newline;
                continue;
            }
            if starts {
                let (top, bottom) = (origin.y + row.pos.y, origin.y + row.pos.y + row.size.y);
                let at = RowAt { line, y: origin.y + row.pos.y + row.size.y / 2.0, top, bottom, left: origin.x + row.pos.x };
                margin.numbered.push((top, bottom, line));
                if at.y > editor_rect.min.y - 20.0 && at.y < editor_rect.max.y + 20.0 {
                    self.line_number(&painter, editor_rect, &at, &margin, &known, marks);
                    self.line_marks(ui, &painter, editor_rect, &at, &margin, &known);
                }
                line += 1;
            }
            starts = row.ends_with_newline;
        }
        margin
    }

    /// A line's number and what is by it: the green arrow of the line played,
    /// its fold's chevron, its bulb (or the dot of what is said), the cursor's band.
    fn line_number(&self, painter: &egui::Painter, editor_rect: Rect, at: &RowAt, margin: &Margin, known: &Known, marks: &Marks) {
        let RowAt { line, y, top, bottom, .. } = *at;
        let x = editor_rect.min.x + GUTTER - 10.0;
        if known.playing == Some(line) {
            play_arrow(painter, pos2(editor_rect.min.x + 20.0, y), Color32::from_rgb(0x4A, 0xDE, 0x80));
        }
        let colour = if line == margin.cursor_line { Color32::from_rgb(0xC6, 0xC6, 0xC6) } else { Color32::from_rgb(0x6E, 0x76, 0x81) };
        painter.text(pos2(x - 12.0, y), egui::Align2::RIGHT_CENTER, line.to_string(), FontId::monospace(12.0), colour);
        // (A fold: its chevron, open or folded.)
        if margin.fold_starts.contains_key(&line) {
            workshop_base::icons::chevron(
                painter,
                pos2(editor_rect.min.x + GUTTER - 12.0, y),
                !self.scripts.folded.contains(&line),
                Color32::from_rgb(0x8A, 0x8A, 0x8A),
            );
        }
        // (A fix offered: a bulb; else what is said of it, a dot.)
        if margin.fixable.contains(&line) {
            let bulb = pos2(editor_rect.min.x + 8.0, y);
            painter.circle_filled(bulb - vec2(0.0, 1.5), 4.0, Color32::from_rgb(0xFF, 0xCC, 0x33));
            painter.rect_filled(Rect::from_center_size(bulb + vec2(0.0, 4.0), vec2(3.5, 2.5)), 0.5, Color32::from_rgb(0xC0, 0xC0, 0xC0));
        } else if let Some(said) = marks.get(&line) {
            painter.circle_filled(pos2(editor_rect.min.x + 8.0, y), 3.5, worst(said));
        }
        if line == margin.cursor_line && self.focused {
            let band = Rect::from_min_max(pos2(editor_rect.min.x + GUTTER - 2.0, top), pos2(editor_rect.max.x, bottom));
            painter.rect_stroke(band, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0x28, 0x28, 0x28)), egui::StrokeKind::Inside);
        }
    }

    /// What is behind a line and beside its number: played, stopped at, found;
    /// the guides of its indent; its note, breakpoint, bookmark; lit from
    /// elsewhere; ▶ where the pointer is on the margin.
    fn line_marks(&self, ui: &egui::Ui, painter: &egui::Painter, editor_rect: Rect, at: &RowAt, margin: &Margin, known: &Known) {
        let scripts = &*self.scripts;
        let RowAt { line, y, top, bottom, left } = *at;
        let across = Rect::from_min_max(pos2(editor_rect.min.x + GUTTER - 2.0, top), pos2(editor_rect.max.x, bottom));
        // (The line the film plays, green behind; stopped at it, stepping by line: outlined in the Film's colour; the lines found, yellow.)
        if known.playing == Some(line) {
            painter.rect_filled(across, 0.0, Color32::from_rgba_unmultiplied(90, 200, 120, 22));
        }
        if self.seen.stepped == Some(line) {
            painter.rect_filled(across, 0.0, theme::FILM.gamma_multiply(0.14));
            painter.rect_stroke(across, 0.0, Stroke::new(1.0_f32, theme::FILM), egui::StrokeKind::Inside);
        }
        if known.found_lines.contains(&line) {
            painter.rect_filled(across, 0.0, Color32::from_rgba_unmultiplied(240, 200, 90, 18));
        }
        // (The guides of the blocks: a thin line down each step of its indent.)
        for step in (0..known.leads.get(line - 1).copied().unwrap_or(0) / 2).map(|step| step * 2) {
            let x = left + step as f32 * known.space_wide + 1.0;
            painter.vline(x, top..=bottom, Stroke::new(1.0_f32, Color32::from_white_alpha(20)));
        }
        // (A note: a yellow square; pointed at, what it says.)
        if let Some(note) = scripts.notes.get(&line) {
            let square = Rect::from_center_size(pos2(editor_rect.min.x + GUTTER - 34.0, (top + bottom) / 2.0), vec2(8.0, 8.0));
            painter.rect_filled(square, 1.5, if note.done { Color32::from_rgb(0x6C, 0xC6, 0x6C) } else { Color32::from_rgb(0xF2, 0xC9, 0x4C) });
            if ui.rect_contains_pointer(square.expand(4.0)) {
                egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), self.text_id.with(("note", line)), egui::PopupAnchor::Pointer).show(|ui| {
                    ui.label(RichText::new(&note.said).size(theme::SMALL + 1.0));
                    ui.label(theme::text("Ctrl+Shift+N: change it", theme::SMALL - 1.0, theme::FAINT));
                });
            }
        }
        // (A breakpoint: a red dot by its number; marked: a blue tab at the edge.)
        if scripts.breakpoints.contains(&line) {
            painter.circle_filled(pos2(editor_rect.min.x + GUTTER - 22.0, (top + bottom) / 2.0), 4.5, Color32::from_rgb(0xE5, 0x14, 0x00));
        }
        if scripts.bookmarks.contains(&line) {
            painter.rect_filled(Rect::from_min_max(pos2(editor_rect.min.x + 1.0, top + 2.0), pos2(editor_rect.min.x + 4.0, bottom - 2.0)), 1.0, theme::DECOR);
        }
        // (Pointed in the Timeline: the line lit; who is pointed elsewhere: their lines marked.)
        if scripts.lit_line == Some(line) {
            painter.rect_filled(across, 0.0, theme::POINTED.gamma_multiply(0.12));
        }
        if known.who_lines.contains(&line) {
            painter.rect_filled(
                Rect::from_min_max(pos2(editor_rect.min.x + GUTTER - 5.0, top + 1.0), pos2(editor_rect.min.x + GUTTER - 3.0, bottom - 1.0)),
                0.0,
                theme::POINTED,
            );
        }
        // (The pointer on the margin: ▶ plays from here.)
        if known.playing != Some(line) && margin.in_margin.is_some_and(|at| top <= at.y && at.y < bottom) {
            play_arrow(painter, pos2(editor_rect.min.x + 20.0, y), Color32::from_rgb(0x8A, 0x8A, 0x8A));
        }
    }

    /// A folded line: how many lines it hides, after its end.
    pub(super) fn folded_chips(&self, written: &Written, editor_rect: Rect, folds: &[(usize, usize)], margin: &Margin) {
        let painter = written.editor.painter_at(editor_rect);
        for (first, last) in folds.iter().filter(|(first, _)| self.scripts.folded.contains(first)) {
            if let Some(end) = margin.ends.get(first) {
                let chip = Rect::from_min_size(pos2(end.max.x + 8.0, end.center().y - 8.0), vec2(70.0, 16.0));
                painter.rect_filled(chip, 3.0, Color32::from_rgb(0x2D, 0x3B, 0x4A));
                painter.text(
                    chip.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("⋯ {} lines", last - first),
                    FontId::proportional(theme::SMALL - 1.0),
                    theme::TEXT,
                );
            }
        }
    }

    /// After a line that makes heard (`sound x`, `music x`): ▶ heard here, ■
    /// stopped. Those lines given back (whether music, its name).
    pub(super) fn heard_buttons(&mut self, ui: &egui::Ui, written: &Written, editor_rect: Rect, margin: &Margin) -> BTreeMap<usize, (bool, String)> {
        let painter = written.editor.painter_at(editor_rect);
        let scripts = &mut *self.scripts;
        let text = &self.open.text;
        let mut hear_asked = None;
        let heard_lines: BTreeMap<usize, (bool, String)> =
            margin.ends.keys().filter_map(|line| Some((*line, heard_on(text.split('\n').nth(line - 1)?)?))).collect();
        for (line, (music, name)) in &heard_lines {
            let Some(end) = margin.ends.get(line).filter(|end| end.max.y >= editor_rect.min.y && end.min.y <= editor_rect.max.y) else { continue };
            let button = Rect::from_center_size(pos2(end.max.x + 18.0, end.center().y), vec2(16.0, 16.0));
            let response = ui.interact(button, self.text_id.with(("listen", *line)), Sense::click()).on_hover_text(if scripts.heard == Some(*line) {
                "stop".to_string()
            } else {
                format!("listen to the {} \"{name}\"", if *music { "music" } else { "sound" })
            });
            let shade = if response.hovered() { Color32::from_rgb(0x4A, 0xDE, 0x80) } else { Color32::from_rgb(0x5A, 0x9A, 0x6A) };
            if scripts.heard == Some(*line) {
                painter.rect_filled(Rect::from_center_size(button.center(), vec2(8.0, 8.0)), 1.0, shade);
            } else {
                let tip = button.center();
                painter.add(egui::Shape::convex_polygon(vec![tip + vec2(-4.0, -5.0), tip + vec2(5.0, 0.0), tip + vec2(-4.0, 5.0)], shade, Stroke::NONE));
            }
            if response.clicked() {
                hear_asked = Some((*line, *music, name.clone()));
            }
        }
        if let Some((line, music, name)) = hear_asked {
            hear(&scripts.session, &mut scripts.heard, &mut scripts.audio, &mut scripts.said, line, music, &name);
        }
        heard_lines
    }

    /// At the end of each line in sight: when it plays, how long, and what is wrong with it.
    pub(super) fn hints(&self, written: &Written, editor_rect: Rect, margin: &Margin, heard_lines: &BTreeMap<usize, (bool, String)>) {
        let painter = written.editor.painter_at(editor_rect);
        for (line, end) in &margin.ends {
            if end.max.y < editor_rect.min.y || end.min.y > editor_rect.max.y {
                continue;
            }
            let mut x =
                end.max.x + 14.0 + if self.scripts.folded.contains(line) { 80.0 } else { 0.0 } + if heard_lines.contains_key(line) { 20.0 } else { 0.0 };
            let y = end.center().y;
            if let Some((start, length)) = self.seen.timing.get(line) {
                let said = if *length >= 0.2 { format!("{} · {length:.1} s", clock(*start)) } else { clock(*start) };
                x = painter
                    .text(pos2(x, y), egui::Align2::LEFT_CENTER, said, FontId::proportional(theme::SMALL - 1.5), Color32::from_rgb(0x5A, 0x60, 0x68))
                    .max
                    .x
                    + 12.0;
            }
            if let Some(told) = self.open.told.iter().find(|told| told.span.start.line == *line)
                && x < editor_rect.max.x - RULER - 40.0
            {
                let said: String = told.message.lines().next().unwrap_or_default().chars().take(90).collect();
                let colour = if told.is_error() { WRONG } else { REMARK };
                painter.with_clip_rect(editor_rect.with_max_x(editor_rect.max.x - RULER)).text(
                    pos2(x, y),
                    egui::Align2::LEFT_CENTER,
                    said,
                    FontId::proportional(theme::SMALL - 1.0),
                    colour.gamma_multiply(0.75),
                );
            }
        }
    }

    /// The margin clicked: on a bulb its fixes, on a chevron the fold, on ▶
    /// played from there, on a number the film goes there (that line given
    /// back); pointed, what is said of the line. Its response given back.
    pub(super) fn margin_clicks(&mut self, ui: &egui::Ui, editor_rect: Rect, margin: &Margin) -> (egui::Response, Option<usize>) {
        let Body { scripts, open, text_id, .. } = self;
        let mut asked_line = None;
        let margin_rect = Rect::from_min_max(editor_rect.min, pos2(editor_rect.min.x + GUTTER - 2.0, editor_rect.max.y));
        let clicked_margin = ui.interact(margin_rect, text_id.with("margin"), Sense::click());
        if clicked_margin.clicked()
            && let Some(at) = clicked_margin.interact_pointer_pos()
            && let Some((_, _, line)) = margin.numbered.iter().find(|(top, bottom, _)| *top <= at.y && at.y < *bottom)
        {
            let x = at.x - editor_rect.min.x;
            if x < 16.0 && margin.fixable.contains(line) {
                scripts.bulb = Some((*line, at));
            } else if x > GUTTER - 20.0 && margin.fold_starts.contains_key(line) {
                if !scripts.folded.remove(line) {
                    scripts.folded.insert(*line);
                }
            } else if (13.0..28.0).contains(&x) {
                scripts.session.ask(workshop_base::ask::Ask::PlayFrom(*line));
            } else {
                asked_line = Some(*line);
                open.go_to_line(*line);
            }
        }
        let pointed_line =
            margin.in_margin.and_then(|at| margin.numbered.iter().find(|(top, bottom, _)| *top <= at.y && at.y < *bottom)).map(|(_, _, line)| *line);
        let said_there: Vec<(bool, String)> = pointed_line
            .map(|line| open.told.iter().filter(|told| told.span.start.line == line).map(|told| (told.is_error(), told.message.clone())).collect())
            .unwrap_or_default();
        let clicked_margin = clicked_margin.on_hover_ui(|ui| {
            ui.set_max_width(420.0);
            for (wrong, said) in &said_there {
                ui.label(RichText::new(said).size(theme::SMALL + 1.0).color(if *wrong { WRONG } else { REMARK }));
            }
            if said_there.is_empty() {
                ui.label(theme::text(
                    "a click on a number: the film goes to that line · on ▶: it plays from here · on ⌄: fold · on the bulb: the fixes",
                    theme::SMALL,
                    theme::WEAK,
                ));
            }
        });
        (clicked_margin, asked_line)
    }

    /// The bulb (opened by a click, or Ctrl+. on the cursor's line): the fixes
    /// of its line, one chosen made; closed by a click elsewhere.
    pub(super) fn bulb(&mut self, ui: &egui::Ui, editor_rect: Rect, margin: &Margin, asked: bool, clicked_margin: &egui::Response) {
        let Body { scripts, open, .. } = self;
        if asked && let Some((top, _, line)) = margin.numbered.iter().find(|(_, _, line)| *line == margin.cursor_line) {
            scripts.bulb = Some((*line, pos2(editor_rect.min.x + GUTTER, *top + scripts.size + 4.0)));
        }
        let Some((line, at)) = scripts.bulb else { return };
        let fixes: Vec<film::service::Fix> = open.told.iter().filter(|told| told.span.start.line == line).flat_map(|told| told.fixes.clone()).collect();
        let mut chosen: Option<String> = None;
        let area = egui::Area::new(egui::Id::new("script bulb")).order(egui::Order::Foreground).fixed_pos(at + vec2(8.0, 4.0)).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(theme::text(format!("Fix Line {line}"), theme::SMALL, theme::WEAK));
                for remedy in &fixes {
                    if ui.button(&remedy.title).clicked() {
                        chosen = Some(film::service::applied(&open.text, &remedy.edits));
                    }
                }
            });
        });
        let elsewhere = ui.input(|input| input.pointer.any_click())
            && !area.response.rect.contains(ui.input(|input| input.pointer.interact_pos().unwrap_or_default()))
            && !clicked_margin.clicked();
        if let Some(made) = chosen {
            open.change(made);
            scripts.bulb = None;
        } else if elsewhere || fixes.is_empty() {
            scripts.bulb = None;
        }
    }

    /// The line of the scene, and of the moment, kept at the top while what
    /// follows them is read (a click: there).
    pub(super) fn lines_held(&mut self, ui: &egui::Ui, written: &Written, editor_rect: Rect, margin: &Margin) {
        let Some((_, _, first_seen)) = margin.numbered.iter().find(|(top, _, _)| *top >= editor_rect.min.y - 1.0) else { return };
        let painter = written.editor.painter_at(editor_rect);
        let outline = film::service::outline(&self.open.text);
        let holding: Vec<&film::service::Symbol> = outline
            .iter()
            .filter(|symbol| matches!(symbol.kind, "scene" | "moment") && symbol.line < *first_seen && *first_seen <= symbol.end_line.max(symbol.line))
            .collect();
        let size = self.scripts.size;
        let high = size + 8.0;
        for (at, symbol) in holding.iter().enumerate() {
            let band = Rect::from_min_size(pos2(editor_rect.min.x, editor_rect.min.y + at as f32 * high), vec2(editor_rect.width() - RULER, high));
            painter.rect_filled(band, 0.0, Color32::from_rgb(0x25, 0x26, 0x29));
            painter.hline(band.x_range(), band.max.y, Stroke::new(1.0_f32, Color32::from_black_alpha(120)));
            painter.text(
                pos2(band.min.x + GUTTER - 24.0, band.center().y),
                egui::Align2::RIGHT_CENTER,
                symbol.line.to_string(),
                FontId::monospace(12.0),
                Color32::from_rgb(0x6E, 0x76, 0x81),
            );
            let said = self.open.text.lines().nth(symbol.line - 1).unwrap_or_default().trim().to_string();
            painter.text(
                pos2(band.min.x + GUTTER, band.center().y),
                egui::Align2::LEFT_CENTER,
                said,
                FontId::monospace(size),
                if symbol.kind == "scene" { LABEL } else { FACE },
            );
            let clicked = ui.interact(band, self.text_id.with(("pinned", at)), Sense::click());
            if clicked.on_hover_text("a click: go there").clicked() {
                self.open.go_to_line(symbol.line);
            }
        }
    }

    /// The ruler on the right: what is wrong, what is found, the scenes and
    /// moments, the line played, the marks, the cursor; a click or a drag goes there.
    pub(super) fn ruler(&mut self, ui: &egui::Ui, editor_rect: Rect, marks: &Marks, found: &[Range<usize>], cursor_line: usize) {
        let text = &self.open.text;
        let lines_count = text.split('\n').count().max(1) as f32;
        let strip = Rect::from_min_max(pos2(editor_rect.max.x - RULER, editor_rect.min.y), editor_rect.max);
        let painter = ui.painter();
        painter.rect_filled(strip, 0.0, Color32::from_rgb(0x1E, 0x1E, 0x1E));
        painter.vline(strip.min.x, strip.y_range(), Stroke::new(1.0_f32, Color32::from_rgb(0x2B, 0x2B, 0x2B)));
        let y_of = |line: usize| strip.min.y + (line as f32 - 0.5) / lines_count * strip.height();
        for range in found {
            let y = y_of(spot_of(text, range.start).line);
            painter.rect_filled(Rect::from_min_size(pos2(strip.min.x + 2.0, y - 1.0), vec2(4.0, 2.0)), 0.0, Color32::from_rgb(0xD1, 0x8A, 0x16));
        }
        for (line, said) in marks {
            painter.rect_filled(Rect::from_min_size(pos2(strip.min.x + 6.0, y_of(*line) - 1.5), vec2(5.0, 3.0)), 0.0, worst(said));
        }
        for (at, line) in text.split('\n').enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("scene:") {
                painter.rect_filled(
                    Rect::from_min_size(pos2(strip.min.x + 1.0, y_of(at + 1) - 1.0), vec2(RULER - 2.0, 2.0)),
                    0.0,
                    Color32::from_rgb(90, 150, 230),
                );
            } else if trimmed.starts_with("moment:") {
                painter.rect_filled(
                    Rect::from_min_size(pos2(strip.min.x + 4.0, y_of(at + 1) - 1.0), vec2(RULER - 5.0, 2.0)),
                    0.0,
                    Color32::from_rgb(90, 150, 230).gamma_multiply(0.5),
                );
            }
        }
        if let Some(line) = self.seen.stepped.or(self.seen.line) {
            painter.rect_filled(Rect::from_min_size(pos2(strip.min.x + 1.0, y_of(line) - 1.5), vec2(RULER - 2.0, 3.0)), 0.0, Color32::from_rgb(90, 200, 120));
        }
        for line in &self.scripts.bookmarks {
            painter.rect_filled(Rect::from_min_size(pos2(strip.min.x + 1.0, y_of(*line) - 1.5), vec2(4.0, 3.0)), 0.0, theme::DECOR);
        }
        for line in &self.scripts.breakpoints {
            painter.circle_filled(pos2(strip.min.x + 3.0, y_of(*line)), 2.5, Color32::from_rgb(0xE5, 0x14, 0x00));
        }
        painter.hline(strip.x_range(), y_of(cursor_line), Stroke::new(2.0_f32, Color32::from_rgb(0xA0, 0xA0, 0xA0)));
        let on_ruler = ui.interact(strip, self.text_id.with("ruler"), Sense::click_and_drag());
        if (on_ruler.clicked() || on_ruler.dragged())
            && let Some(at) = on_ruler.interact_pointer_pos()
        {
            self.open.go_to_line(line_on_ruler(at.y, strip.min.y, strip.height(), lines_count as usize));
        }
    }
}
