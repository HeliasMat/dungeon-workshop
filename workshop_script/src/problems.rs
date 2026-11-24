//! Under the text: the problems (their head, all fixed at once, the list
//! with each one's fixes), the line said in its box as the game cuts it, the
//! status (where the cursor is, how long the film is, what is wrong).

use eframe::egui::{self, Color32, FontId, Rect, RichText, Sense, Stroke, pos2, vec2};
use film::service::{Diagnostic, Spot};

use super::body::{Body, Room};
use super::keys::next_spot;
use super::plan::clock;
use super::text::{index_of, spot_of};
use super::{REMARK, STATUS, WRONG, severity_colour};
use workshop_base::theme;

/// The changes of the first fix offered of each problem, those that do not
/// overlap one taken before them.
pub(super) fn fixes_together(told: &[Diagnostic]) -> Vec<film::service::Edit> {
    let mut changes: Vec<film::service::Edit> = Vec::new();
    for remedy in told.iter().filter_map(|told| told.fixes.first()) {
        let overlaps = remedy.edits.iter().any(|change| changes.iter().any(|known| change.span.start < known.span.end && known.span.start < change.span.end));
        if !overlaps {
            changes.extend(remedy.edits.iter().cloned());
        }
    }
    changes
}

/// The status said: where the cursor is, how many lines and scenes, how long the film is.
pub(super) fn status_said(text: &str, here: Spot, end: Option<f64>) -> String {
    let lines = text.split('\n').count();
    let scenes = text.split('\n').filter(|line| line.trim_start().starts_with("scene:")).count();
    let mut said = format!("Line {}, column {}   ·   {lines} lines · {scenes} scene{}", here.line, here.column, if scenes > 1 { "s" } else { "" });
    if let Some(end) = end {
        said.push_str(&format!("   ·   length {}", clock(end)));
    }
    said
}

/// What is wrong, as the status says it, in its colour.
pub(super) fn problems_said(wrong: usize, remarked: usize) -> (Color32, String) {
    match (wrong, remarked) {
        (0, 0) => (Color32::from_rgb(90, 200, 120), "no problems".to_string()),
        (0, remarked) => (REMARK, format!("{remarked} remark{}", if remarked > 1 { "s" } else { "" })),
        (wrong, _) => (WRONG, format!("{wrong} problem{}", if wrong > 1 { "s" } else { "" })),
    }
}

impl Body<'_> {
    /// The problems under the text: their head, and the list when it is open.
    pub(super) fn problems(&mut self, ui: &mut egui::Ui, problems_rect: Rect, wrong: usize, remarked: usize) {
        let mut panel = ui.new_child(egui::UiBuilder::new().max_rect(problems_rect));
        panel.painter().rect_filled(problems_rect, 0.0, theme::PANEL);
        self.problems_head(&mut panel, problems_rect, wrong, remarked);
        if self.scripts.problems_open && !self.scripts.concentration {
            self.problems_list(&mut panel);
        }
    }

    /// The problems' head: how many, what is said, Fix All, the remarks listed
    /// or not; a click elsewhere on it opens or closes the list.
    fn problems_head(&mut self, panel: &mut egui::Ui, problems_rect: Rect, wrong: usize, remarked: usize) {
        let Body { scripts, open, text_id, .. } = self;
        let (head, head_response) = panel.allocate_exact_size(vec2(problems_rect.width(), 24.0), Sense::click());
        let painter = panel.painter().clone();
        painter.rect_filled(head, 0.0, theme::TABS);
        workshop_base::icons::chevron(&painter, pos2(head.min.x + 10.0, head.center().y), scripts.problems_open && !scripts.concentration, theme::TEXT);
        painter.text(pos2(head.min.x + 20.0, head.center().y), egui::Align2::LEFT_CENTER, "PROBLEMS", FontId::proportional(theme::SMALL), theme::STRONG);
        let looking = open.asked != open.told_for || open.changed.is_some();
        let counts = format!("{wrong} error(s) · {remarked} remark(s){}", if looking { " · reading again…" } else { "" });
        painter.text(
            pos2(head.min.x + 104.0, head.center().y),
            egui::Align2::LEFT_CENTER,
            counts,
            FontId::proportional(theme::SMALL),
            if wrong > 0 { WRONG } else { theme::WEAK },
        );
        if let Some(said) = &scripts.said {
            painter.text(head.right_center() - vec2(110.0, 0.0), egui::Align2::RIGHT_CENTER, said, FontId::proportional(theme::SMALL), theme::WEAK);
        }
        // (All fixed at once: the first fix offered of each, those that do not overlap.)
        let fixes = open.told.iter().filter(|told| !told.fixes.is_empty()).count();
        if fixes > 0 {
            let button = Rect::from_min_size(pos2(head.max.x - 210.0, head.min.y + 3.0), vec2(104.0, 18.0));
            let pressed = panel.interact(button, text_id.with("fix all"), Sense::click());
            painter.rect_filled(button, 3.0, if pressed.hovered() { theme::HOVERED } else { theme::FIELD });
            painter.text(button.center(), egui::Align2::CENTER_CENTER, format!("Fix All ({fixes})"), FontId::proportional(theme::SMALL), theme::TEXT);
            if pressed.on_hover_text("the first fix offered of each, if they do not get in each other's way").clicked() {
                let made = film::service::applied(&open.text, &fixes_together(&open.told));
                open.change(made);
            }
        }
        let toggle_at = Rect::from_min_max(pos2(head.max.x - 100.0, head.min.y + 2.0), head.max - vec2(4.0, 2.0));
        let toggle = panel.interact(toggle_at, text_id.with("remarks"), Sense::click());
        let square = Rect::from_center_size(pos2(toggle_at.min.x + 12.0, toggle_at.center().y), vec2(10.0, 10.0));
        painter.rect_stroke(square, 2.0, Stroke::new(1.0_f32, theme::WEAK), egui::StrokeKind::Inside);
        if scripts.remarks_listed {
            painter.rect_filled(square.shrink(2.5), 1.0, REMARK);
        }
        painter.text(pos2(square.max.x + 6.0, toggle_at.center().y), egui::Align2::LEFT_CENTER, "remarks", FontId::proportional(theme::SMALL), theme::WEAK);
        if toggle.clicked() {
            scripts.remarks_listed = !scripts.remarks_listed;
        } else if head_response.clicked() {
            scripts.problems_open = !scripts.problems_open;
        }
    }

    /// The problems listed (the errors first; the remarks if they are listed):
    /// a click goes there, a fix of each made.
    fn problems_list(&mut self, panel: &mut egui::Ui) {
        let Body { scripts, open, path, .. } = self;
        let mut fix: Option<String> = None;
        let mut go: Option<Spot> = None;
        egui::ScrollArea::vertical().id_salt(("problems", *path)).auto_shrink(false).show(panel, |ui| {
            if open.told.is_empty() {
                ui.add_space(4.0);
                ui.label(theme::text("  Nothing to say.", theme::SMALL, theme::WEAK));
            }
            let listed = scripts.remarks_listed;
            let mut sorted: Vec<&Diagnostic> = open.told.iter().filter(|told| listed || told.is_error()).collect();
            sorted.sort_by_key(|told| (!told.is_error(), told.span.start.line));
            for told in sorted {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let (dot, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 3.5, severity_colour(told));
                    let said = format!("{}  ", told.message);
                    if ui
                        .add(egui::Label::new(RichText::new(said).color(theme::TEXT)).sense(Sense::click()).truncate())
                        .on_hover_text(told.notes.join("\n"))
                        .clicked()
                    {
                        go = Some(told.span.start);
                    }
                    ui.label(theme::text(format!("line {}", told.span.start.line), theme::SMALL, theme::FAINT));
                    for remedy in told.fixes.iter().take(3) {
                        if ui.small_button(&remedy.title).on_hover_text("fix it this way").clicked() {
                            fix = Some(film::service::applied(&open.text, &remedy.edits));
                        }
                    }
                });
            }
        });
        if let Some(spot) = go {
            let at = index_of(&open.text, spot);
            open.jump = Some((at, at));
        }
        if let Some(made) = fix {
            open.change(made);
        }
    }

    /// The line said where the cursor is, in its box as the game cuts it: too long, in red; how long it is read.
    pub(super) fn said_box(&self, ui: &egui::Ui, room: Room, lines: &[String]) {
        let (whole, full) = (room.whole, room.full);
        let painter = ui.painter();
        let room = Rect::from_min_max(pos2(whole.min.x + 6.0, full.max.y + 3.0), pos2(whole.max.x - 6.0, whole.max.y - STATUS - 3.0));
        let over = lines.len() > film::screen::BOX_LINES;
        painter.rect_filled(room, 4.0, Color32::from_rgb(24, 32, 64));
        painter.rect_stroke(room, 4.0, Stroke::new(1.0_f32, if over { WRONG } else { Color32::from_gray(140) }), egui::StrokeKind::Inside);
        for (at, line) in lines.iter().enumerate() {
            let colour = if at >= film::screen::BOX_LINES { WRONG } else { Color32::WHITE };
            painter.text(pos2(room.min.x + 8.0, room.min.y + 8.0 + at as f32 * 15.0), egui::Align2::LEFT_TOP, line, FontId::monospace(12.0), colour);
        }
        if let Some((_, length)) = self.seen.timing.get(&spot_of(&self.open.text, self.open.cursor).line).filter(|(_, length)| *length > 0.0) {
            painter.text(
                pos2(room.max.x - 8.0, room.min.y + 8.0),
                egui::Align2::RIGHT_TOP,
                format!("read in {length:.1} s"),
                FontId::proportional(theme::SMALL),
                Color32::from_gray(160),
            );
        }
        if over {
            let said = format!("too long: {} lines, the box holds {} of {} letters; cut it in two", lines.len(), film::screen::BOX_LINES, film::screen::WIDE);
            painter.text(pos2(room.min.x + 8.0, room.max.y - 6.0), egui::Align2::LEFT_BOTTOM, said, FontId::proportional(theme::SMALL), WRONG);
        }
    }

    /// The status: where the cursor is, how long the text and the film are, what is wrong (a click: the next).
    pub(super) fn status(&mut self, ui: &egui::Ui, whole: Rect, wrong: usize, remarked: usize) {
        let status = Rect::from_min_max(pos2(whole.min.x, whole.max.y - STATUS), whole.max);
        ui.painter().rect_filled(status, 0.0, theme::TABS);
        let here = spot_of(&self.open.text, self.open.cursor);
        let said = status_said(&self.open.text, here, self.seen.end);
        let (colour, problems) = problems_said(wrong, remarked);
        let shown = ui.painter().text(
            status.right_center() - vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format!("● {problems}"),
            FontId::proportional(theme::SMALL),
            colour,
        );
        ui.painter().with_clip_rect(status.with_max_x(shown.min.x - 10.0)).text(
            status.left_center() + vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            said,
            FontId::proportional(theme::SMALL),
            theme::WEAK,
        );
        if ui.interact(shown.expand(3.0), self.text_id.with("status"), Sense::click()).on_hover_text("the next one (F8; Shift+F8: the one before)").clicked() {
            let mut spots: Vec<Spot> = self.open.told.iter().map(|told| told.span.start).collect();
            spots.sort_by_key(|spot| (spot.line, spot.column));
            if let Some(spot) = next_spot(&spots, here.line, true) {
                let at = index_of(&self.open.text, spot);
                self.open.jump = Some((at, at));
            }
        }
    }
}
