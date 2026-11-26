//! The text itself: drawn, coloured, typed in. The Script's body is drawn
//! part after part: the head over the text (`head.rs`), the keys taken first
//! (`keys.rs`), the editor here, its margin and its ruler (`margin.rs`), what
//! is offered as one types (`offers.rs`), the small windows over the text
//! (`popups.rs`), the problems and the status under it (`problems.rs`), an
//! asset of the Film's Assets panel dropped on it (`assets.rs`).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, pos2};
use egui::text::{CCursor, CCursorRange};
use film::service::{Diagnostic, Spot};

use super::edits::shifted;
use super::highlight::{Lights, lights_of, painted};
use super::keys::Keyed;
use super::knowledge::{Defined, defined};
use super::text::{found_in, index_of, letter_of_word, line_at, number_at, spot_of, word_at};
use super::windows::{project_windows, scripts_parts};
use super::{GUTTER, STATUS, Script, Scripts, Scrub, Seen, severity_colour};
use workshop_base::theme;

/// The ruler's width, on the right of the text.
pub(super) const RULER: f32 = 12.0;

/// What is said of each line (from 1): its columns (from 1), in the colour of what is said.
pub(super) type Marks = BTreeMap<usize, Vec<(Range<usize>, Color32)>>;

/// What the parts of the Script share as they are drawn: the Script's state,
/// the film open (taken out of it meanwhile), its path, what the film is
/// doing, the text's id, whether the text has the keys.
pub(super) struct Body<'a> {
    pub(super) scripts: &'a mut Scripts,
    pub(super) open: &'a mut Script,
    pub(super) path: &'a PathBuf,
    pub(super) seen: &'a Seen,
    pub(super) text_id: egui::Id,
    pub(super) focused: bool,
}

/// Where the parts go: the whole room; it without the line's box and the
/// status; the editor; the problems under it.
#[derive(Clone, Copy)]
pub(super) struct Room {
    pub(super) whole: Rect,
    pub(super) full: Rect,
    pub(super) editor: Rect,
    pub(super) problems: Rect,
}

/// What the text editor drew: its own Ui (painted on after), what it gave
/// back, where its text is, whether the text changed, the text before.
pub(super) struct Written {
    pub(super) editor: egui::Ui,
    pub(super) output: egui::text_edit::TextEditOutput,
    pub(super) galley: Arc<egui::Galley>,
    pub(super) origin: Pos2,
    pub(super) changed: bool,
    pub(super) before: String,
}

impl Written {
    /// Where a letter (its index) is drawn on the screen.
    pub(super) fn at(&self, index: usize) -> Rect {
        self.galley.pos_from_cursor(CCursor::new(index)).translate(self.origin.to_vec2())
    }

    /// The letter (its index) under a point of the screen.
    pub(super) fn index_at(&self, at: Pos2) -> usize {
        self.galley.cursor_from_pos(at - self.origin).index
    }
}

/// The Script's body: the film's text and all around it; the line the film
/// is asked to go to (a number clicked in the margin).
pub(super) fn script_body(ui: &mut egui::Ui, scripts: &mut Scripts, path: Option<&PathBuf>, seen: &Seen) -> Option<usize> {
    let Some(path) = path else {
        ui.add_space(8.0);
        ui.label(theme::weak("No film open: the Home, or File › Open."));
        return None;
    };
    scripts.open(path);
    let whole = ui.available_rect_before_wrap();
    let text_id = egui::Id::new(("script", path));
    let focused = ui.memory(|memory| memory.has_focus(text_id));
    let counts = scripts.counts();
    let mut open = scripts.open.take()?;
    let asked = Body { scripts, open: &mut open, path, seen, text_id, focused }.shown(ui, whole, counts);
    scripts.open = Some(open);
    asked
}

/// The line said where the cursor is, cut in lines as the game's box cuts it (nothing if nothing is said).
pub(super) fn said_box_of(text: &str, cursor: usize) -> Option<Vec<String>> {
    film::screen::boxed(&line_at(text, cursor).2).filter(|lines| lines.iter().any(|line| !line.trim().is_empty()))
}

/// How high the box of the line said is drawn (a line more for the warning when it is too long).
pub(super) fn said_box_height(lines: Option<&[String]>) -> f32 {
    lines.map_or(0.0, |lines| lines.len() as f32 * 15.0 + 16.0 + if lines.len() > film::screen::BOX_LINES { 16.0 } else { 0.0 })
}

/// What is said of each line: the columns said wrong or remarked upon, in their colour.
pub(super) fn marks_by_line(told: &[Diagnostic]) -> Marks {
    let mut marks: Marks = BTreeMap::new();
    for told in told {
        let end = if told.span.end.line == told.span.start.line { told.span.end.column.max(told.span.start.column + 1) } else { usize::MAX };
        marks.entry(told.span.start.line).or_default().push((told.span.start.column..end, severity_colour(told)));
    }
    marks
}

/// The places of a word in the text where it is whole (not inside a longer word).
pub(super) fn whole_word_found(text: &str, word: &str) -> Vec<Range<usize>> {
    let letters: Vec<char> = text.chars().collect();
    found_in(text, word, true)
        .into_iter()
        .filter(|range| {
            let before = range.start == 0 || !letter_of_word(letters[range.start - 1]);
            let after = range.end >= letters.len() || !letter_of_word(letters[range.end]);
            before && after
        })
        .collect()
}

/// The lines folded away: those under the first line of each fold folded.
pub(super) fn hidden_lines(folds: &[(usize, usize)], folded: &BTreeSet<usize>) -> BTreeSet<usize> {
    let mut hidden: BTreeSet<usize> = BTreeSet::new();
    for (first, last) in folds.iter().filter(|(first, _)| folded.contains(first)) {
        hidden.extend(first + 1..=*last);
    }
    hidden
}

/// The number dragged with Alt, written as far as it was dragged: a step of
/// its last decimal every 3 points (every 6 with decimals).
pub(super) fn scrubbed(value: f64, decimals: usize, moved: f32) -> String {
    let step = if decimals > 0 { 10_f64.powi(-(decimals as i32)) } else { 1.0 };
    let value = value + f64::from((moved / if decimals > 0 { 6.0 } else { 3.0 }).round()) * step;
    format!("{value:.0$}", decimals)
}

impl Body<'_> {
    /// The Script drawn, part after part, in the order they are drawn and take
    /// the input; the line the film is asked to go to.
    fn shown(mut self, ui: &mut egui::Ui, whole: Rect, (wrong, remarked): (usize, usize)) -> Option<usize> {
        self.follow_marks();
        let said_box = said_box_of(&self.open.text, self.open.cursor);
        let full = Rect::from_min_max(whole.min, pos2(whole.max.x, whole.max.y - STATUS - said_box_height(said_box.as_deref())));
        self.tabs(ui);
        self.head(ui);
        let scripts = &mut *self.scripts;
        project_windows(
            ui,
            scripts_parts(
                &scripts.session,
                &mut scripts.project_search,
                &mut scripts.quick_open,
                &mut scripts.beside,
                &mut scripts.versions_shown,
                &mut scripts.version_chosen,
                &mut scripts.notes,
                &mut scripts.note_editing,
                &mut scripts.said,
            ),
            self.open,
        );
        if self.scripts.view != crate::story::View::Text {
            self.story(ui, whole);
            return None;
        }
        let room = self.room(ui, whole, full);
        let marks = marks_by_line(&self.open.told);
        let selection = self.selection(ui);
        let keyed = if self.focused { self.keys(ui, selection) } else { Keyed::default() };
        self.take_offered(keyed.accept, keyed.browse_take);
        let (found, lights) = self.lights();
        let folds = film::service::folds(&self.open.text);
        let hidden = self.folded_away(&folds);
        let written = self.editor(ui, room.editor, &marks, &lights, &hidden);
        self.scrub(ui, &written, room.editor);
        self.clicked_in_text(ui, &written, room.editor, selection);
        self.lines_menu(&written);
        self.recipe_naming(ui);
        // (Put somewhere by a key, the plan, a problem: brought into sight.)
        if let Some((a, _)) = self.open.jump.take() {
            let at = written.at(a);
            written.editor.scroll_to_rect(at.expand(40.0), Some(egui::Align::Center));
        }
        let margin = self.margin(ui, &written, room.editor, &marks, &found, &folds, &hidden);
        self.folded_chips(&written, room.editor, &folds, &margin);
        let heard_lines = self.heard_buttons(ui, &written, room.editor, &margin);
        if self.scripts.hints {
            self.hints(&written, room.editor, &margin, &heard_lines);
        }
        let (clicked_margin, asked_line) = self.margin_clicks(ui, room.editor, &margin);
        self.bulb(ui, room.editor, &margin, keyed.bulb_asked, &clicked_margin);
        self.lines_held(ui, &written, room.editor, &margin);
        self.asset_dropped(ui, &written, room.editor);
        self.ruler(ui, room.editor, &marks, &found, margin.cursor_line);
        self.forms(ui, &written);
        self.offers(ui, &written, room.editor);
        self.word_pointed(ui, &written);
        self.renaming(ui, &written);
        self.find_bar(ui, room.editor, &found);
        self.going_to(ui, room.editor);
        self.problems(ui, room.problems, wrong, remarked);
        if let Some(lines) = &said_box {
            self.said_box(ui, room, lines);
        }
        self.status(ui, whole, wrong, remarked);
        ui.allocate_rect(whole, Sense::hover());
        asked_line
    }

    /// The folds, bookmarks, breakpoints and notes follow their lines as lines come and go above them.
    fn follow_marks(&mut self) {
        let Body { scripts, open, .. } = self;
        if scripts.marks_text == open.text {
            return;
        }
        if !scripts.marks_text.is_empty() {
            shifted(&scripts.marks_text, &open.text, &mut scripts.folded);
            shifted(&scripts.marks_text, &open.text, &mut scripts.bookmarks);
            shifted(&scripts.marks_text, &open.text, &mut scripts.breakpoints);
            if !scripts.notes.is_empty() {
                let mut moved = BTreeMap::new();
                for (line, note) in std::mem::take(&mut scripts.notes) {
                    let mut alone = BTreeSet::from([line]);
                    shifted(&scripts.marks_text, &open.text, &mut alone);
                    if let Some(now) = alone.first() {
                        moved.insert(*now, note);
                    }
                }
                if moved.keys().ne(crate::workspace::read_notes(scripts.session.library(), &open.path).keys()) {
                    crate::workspace::write_notes(scripts.session.library(), &open.path, &moved);
                }
                scripts.notes = moved;
            }
        }
        scripts.marks_text = open.text.clone();
    }

    /// Where the editor and the problems go, under the head: the problems
    /// open (a share of the height), or their head alone.
    fn room(&self, ui: &egui::Ui, whole: Rect, full: Rect) -> Room {
        let wrong = self.open.told.iter().filter(|told| told.is_error()).count();
        let problems_height = if self.scripts.problems_docked && !self.scripts.concentration {
            0.0
        } else if super::problems::problems_shown(self.scripts.problems_open, wrong) && !self.scripts.concentration {
            150.0_f32.min(full.height() * 0.35)
        } else {
            26.0
        };
        let editor = Rect::from_min_max(ui.cursor().min, pos2(full.max.x, full.max.y - problems_height));
        let problems = Rect::from_min_max(pos2(full.min.x, full.max.y - problems_height), full.max);
        Room { whole, full, editor, problems }
    }

    /// What is chosen in the text, as the editor last had it (its two ends; the cursor first).
    fn selection(&self, ui: &egui::Ui) -> (usize, usize) {
        let state = egui::TextEdit::load_state(ui.ctx(), self.text_id);
        state
            .as_ref()
            .and_then(|state| state.cursor.char_range())
            .map_or((self.open.cursor, self.open.cursor), |range| (range.primary.index, range.secondary.index))
    }

    /// What is lit: what is found (the one looked at brighter); else the word
    /// under the cursor, wherever else it is. What is found, given back too.
    fn lights(&self) -> (Vec<Range<usize>>, Lights) {
        let text = &self.open.text;
        let mut lights: Lights = BTreeMap::new();
        let found: Vec<Range<usize>> = self.scripts.finding.as_ref().map(|finding| found_in(text, &finding.sought, finding.case)).unwrap_or_default();
        if let Some(finding) = &self.scripts.finding {
            lights_of(text, &found, Color32::from_rgb(0x61, 0x3A, 0x14), &mut lights);
            if let Some(one) = found.get(finding.at) {
                lights_of(text, std::slice::from_ref(one), Color32::from_rgb(0x9E, 0x6A, 0x03), &mut lights);
            }
        }
        if self.scripts.finding.is_none()
            && let Some(word) = word_at(text, self.open.cursor).filter(|word| word.chars().count() >= 3)
        {
            let same = whole_word_found(text, &word);
            if same.len() > 1 {
                lights_of(text, &same, Color32::from_rgb(0x34, 0x3A, 0x40), &mut lights);
            }
        }
        (found, lights)
    }

    /// The folds still there kept folded, unfolded if the cursor goes in; the lines they hide.
    fn folded_away(&mut self, folds: &[(usize, usize)]) -> BTreeSet<usize> {
        let folded = &mut self.scripts.folded;
        folded.retain(|start| folds.iter().any(|(first, _)| first == start));
        let cursor_line_now = spot_of(&self.open.text, self.open.cursor).line;
        folded.retain(|start| folds.iter().find(|(first, _)| first == start).is_none_or(|(_, last)| !(start + 1..=*last).contains(&cursor_line_now)));
        hidden_lines(folds, folded)
    }

    /// The text editor itself, in its colours, scrolled (the line written kept in the middle, in Focus).
    fn editor(&mut self, ui: &mut egui::Ui, editor_rect: Rect, marks: &Marks, lights: &Lights, hidden: &BTreeSet<usize>) -> Written {
        let mut editor = ui.new_child(egui::UiBuilder::new().max_rect(editor_rect.with_max_x(editor_rect.max.x - RULER)));
        let Body { scripts, open, path, text_id, .. } = self;
        let text_id = *text_id;
        let font = FontId::monospace(scripts.size);
        let wrap = scripts.wrap;
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
            let job = painted(text.as_str(), &font, if wrap { width } else { f32::INFINITY }, marks, lights, hidden);
            ui.fonts(|fonts| fonts.layout_job(job))
        };
        if let Some((a, b)) = open.jump {
            let mut state = egui::TextEdit::load_state(editor.ctx(), text_id).unwrap_or_default();
            state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(b), CCursor::new(a))));
            state.store(editor.ctx(), text_id);
            editor.memory_mut(|memory| memory.request_focus(text_id));
        }
        let before = open.text.clone();
        let scrolling = if wrap { egui::ScrollArea::vertical() } else { egui::ScrollArea::both() };
        let center_now = scripts.concentration && scripts.centered.is_some_and(|(_, left)| left > 0);
        let caret = open.cursor;
        let jumped = open.jump.map(|(a, _)| a);
        let size = scripts.size;
        let scroll = scrolling.id_salt(("script scrolls", *path)).auto_shrink(false).show(&mut editor, |ui| {
            ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::from_rgb(0x1E, 0x1E, 0x1E));
            let shown = ui
                .horizontal_top(|ui| {
                    ui.add_space(GUTTER);
                    egui::TextEdit::multiline(&mut open.text)
                        .id(text_id)
                        .font(FontId::monospace(size))
                        .code_editor()
                        .frame(false)
                        .lock_focus(true)
                        .desired_width(if wrap { ui.available_width() } else { f32::INFINITY })
                        .desired_rows(30)
                        .layouter(&mut layouter)
                        .show(ui)
                })
                .inner;
            // (Concentration: the line written brought to the middle, here, where the scrolling is.)
            if center_now {
                let at = shown.cursor_range.map_or(CCursor::new(caret), |range| range.primary);
                ui.scroll_to_rect(shown.galley.pos_from_cursor(at).translate(shown.galley_pos.to_vec2()), Some(egui::Align::Center));
            }
            // (Put somewhere by a key, the plan, a problem, a film opened on a line: brought into sight, here, where the
            // scrolling is; asked after it, it would be lost.)
            if let Some(at) = jumped {
                let rect = shown.galley.pos_from_cursor(CCursor::new(at)).translate(shown.galley_pos.to_vec2());
                ui.scroll_to_rect(rect.expand(40.0), Some(egui::Align::Center));
            }
            shown
        });
        let output = scroll.inner;
        let changed = open.text != before;
        if changed {
            open.touched();
        }
        if let Some(range) = output.cursor_range {
            open.cursor = range.primary.index;
        }
        self.keep_centred();
        Written { galley: output.galley.clone(), origin: output.galley_pos, output, editor, changed, before }
    }

    /// Concentration: the line written kept in the middle of the screen (a few frames each time it changes).
    fn keep_centred(&mut self) {
        if !self.scripts.concentration {
            return;
        }
        let line = spot_of(&self.open.text, self.open.cursor).line;
        if self.scripts.centered.is_none_or(|(was, _)| was != line) {
            self.scripts.centered = Some((line, 4));
        }
        if let Some((_, left @ 1..)) = self.scripts.centered.as_mut() {
            *left -= 1;
        }
    }

    /// Alt + a number dragged: more to the right, less to the left, the Scene
    /// following; a click alone with Alt stays a cursor more.
    fn scrub(&mut self, ui: &egui::Ui, written: &Written, editor_rect: Rect) {
        let Body { scripts, open, .. } = self;
        let (pressed, held, pointer, alt) =
            ui.input(|input| (input.pointer.primary_pressed(), input.pointer.primary_down(), input.pointer.interact_pos(), input.modifiers.alt));
        if pressed
            && alt
            && let Some(at) = pointer.filter(|at| editor_rect.contains(*at) && at.x > editor_rect.min.x + GUTTER)
            && let Some((start, len, value, decimals)) = number_at(&open.text, written.index_at(at))
        {
            scripts.scrub = Some(Scrub { start, len, value, decimals, from_x: at.x, moved: false });
        }
        let Some(scrub) = scripts.scrub.as_mut() else { return };
        if !held {
            if scrub.moved {
                scripts.extra.clear();
            }
            scripts.scrub = None;
        } else if let Some(at) = pointer {
            let moved = at.x - scrub.from_x;
            if moved.abs() > 3.0 || scrub.moved {
                scrub.moved = true;
                let written = scrubbed(scrub.value, scrub.decimals, moved);
                let letters: Vec<char> = open.text.chars().collect();
                if scrub.start + scrub.len <= letters.len() {
                    let made: String = letters[..scrub.start]
                        .iter()
                        .chain(written.chars().collect::<Vec<_>>().iter())
                        .chain(letters[scrub.start + scrub.len..].iter())
                        .collect();
                    scrub.len = written.chars().count();
                    open.change(made);
                    let caret = scrub.start + scrub.len;
                    open.jump = Some((caret, caret));
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                scripts.said = Some(format!("{written} · Alt + drag: right more, left less"));
            }
        }
    }

    /// What a click, the pointer and the wheel do in the text: Alt+click a
    /// cursor more (the cursors drawn), Ctrl+click where a name is given,
    /// Ctrl+wheel the letters' size; the line the film plays kept in sight;
    /// the line pointed, told to the other panels.
    fn clicked_in_text(&mut self, ui: &egui::Ui, written: &Written, editor_rect: Rect, selection: (usize, usize)) {
        let playing = self.seen.stepped.or(self.seen.line);
        let Body { scripts, open, seen, focused, .. } = self;
        let response = &written.output.response;
        if response.clicked() && scripts.scrub.as_ref().is_none_or(|scrub| !scrub.moved) {
            if ui.input(|input| input.modifiers.alt) {
                if selection.0 != open.cursor && !scripts.extra.contains(&selection.0) {
                    scripts.extra.push(selection.0);
                }
            } else {
                scripts.extra.clear();
            }
        }
        scripts.extra.retain(|at| *at <= open.text.chars().count());
        for at in &scripts.extra {
            let spot = written.at(*at);
            written.editor.painter_at(editor_rect).vline(spot.min.x, spot.y_range(), Stroke::new(2.0_f32, Color32::from_rgb(0xAE, 0xAF, 0xAD)));
        }
        if !scripts.extra.is_empty() {
            scripts.said = Some(format!("{} cursors · Esc: just one", scripts.extra.len() + 1));
        }
        // (The film playing, the text not being written: the line it plays kept in sight.)
        if (seen.playing && !*focused || seen.stepped.is_some()) && playing != scripts.followed {
            scripts.followed = playing;
            if let Some(line) = playing {
                let at = written.at(index_of(&open.text, Spot { line, column: 1 }));
                written.editor.scroll_to_rect(at.expand(40.0), Some(egui::Align::Center));
            }
        }
        let response = &written.output.response;
        if response.clicked() && ui.input(|input| input.modifiers.command) {
            match defined(&scripts.session, &open.text, open.cursor) {
                Defined::Here(line) => {
                    let at = index_of(&open.text, Spot { line, column: 1 });
                    open.jump = Some((at, at));
                }
                Defined::File(path, line) => scripts.session.ask(workshop_base::ask::Ask::Film { path, line }),
                Defined::Said(said) => scripts.said = Some(said),
            }
        }
        if response.hovered() {
            let zoom = ui.input(|input| input.zoom_delta());
            if zoom != 1.0 {
                scripts.size = (scripts.size * zoom).clamp(9.0, 28.0);
            }
        }
        scripts.hovered_line = response.hover_pos().map(|at| spot_of(&open.text, written.index_at(at)).line);
    }
}
