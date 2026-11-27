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
    let scenes = text.split('\n').filter(|line| film::lines::scene_head(line).is_some()).count();
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

/// Problems said alike, as one line: the first of them, the lines they are on, all of them.
pub(super) struct Group<'a> {
    pub(super) first: &'a Diagnostic,
    pub(super) lines: Vec<usize>,
    pub(super) all: Vec<&'a Diagnostic>,
}

impl Group<'_> {
    /// How the group is said: the problem's own words for one; for several, what they share (`shape`), without
    /// the line it quotes first.
    pub(super) fn said(&self) -> String {
        if self.all.len() == 1 {
            return self.first.message.clone();
        }
        let shape = shape(&self.first.message);
        shape.strip_prefix("“…”: ").or_else(|| shape.strip_prefix("“…” ")).unwrap_or(&shape).to_string()
    }
}

/// What a problem says, without what is its own: what it quotes (`"…"`, `“…”`) and its numbers, so that two
/// alike on two lines say the same.
pub(super) fn shape(message: &str) -> String {
    let mut shape = String::new();
    let mut quoted: Option<char> = None;
    let mut letters = message.chars().peekable();
    while let Some(letter) = letters.next() {
        match (quoted, letter) {
            (Some(end), _) if letter == end => {
                quoted = None;
                shape.push_str("…”");
            }
            (Some(_), _) => {}
            (None, '"') => {
                quoted = Some('"');
                shape.push('“');
            }
            (None, '“') => {
                quoted = Some('”');
                shape.push('“');
            }
            (None, digit) if digit.is_ascii_digit() => {
                while letters.peek().is_some_and(|next| next.is_ascii_digit() || *next == '.') {
                    letters.next();
                }
                shape.push('n');
            }
            (None, other) => shape.push(other),
        }
    }
    shape
}

/// The problems as they are listed: those said alike (by their `shape`, and their code) folded into one (the
/// lines they are on kept), the errors first, then by their first line; the remarks only if they are listed.
pub(super) fn grouped(told: &[Diagnostic], remarks: bool) -> Vec<Group<'_>> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    for one in told.iter().filter(|one| remarks || one.is_error()) {
        let line = one.span.start.line;
        let alike = |group: &&mut Group<'_>| {
            group.first.is_error() == one.is_error() && group.first.code == one.code && shape(&group.first.message) == shape(&one.message)
        };
        match groups.iter_mut().find(alike) {
            Some(group) => {
                if !group.lines.contains(&line) {
                    group.lines.push(line);
                }
                group.all.push(one);
            }
            None => groups.push(Group { first: one, lines: vec![line], all: vec![one] }),
        }
    }
    for group in &mut groups {
        group.lines.sort_unstable();
    }
    groups.sort_by_key(|group| (!group.first.is_error(), group.lines.first().copied().unwrap_or(0)));
    groups
}

/// Whether the problems under the text are open: as asked, else when something is wrong.
pub(super) fn problems_shown(asked: Option<bool>, wrong: usize) -> bool {
    asked.unwrap_or(wrong > 0)
}

/// What a click in the problems asks: a spot gone to, the text made so.
#[derive(Default)]
struct Picked {
    go: Option<Spot>,
    fix: Option<String>,
}

/// The problems listed, grouped (`grouped`): a click on one goes to its first
/// line, on a line's number to that line; a fix of one, all of a group's at once.
fn listed(ui: &mut egui::Ui, open: &crate::Script, remarks: bool, salt: impl std::hash::Hash) -> Picked {
    let mut picked = Picked::default();
    egui::ScrollArea::vertical().id_salt(salt).auto_shrink(false).show(ui, |ui| {
        let groups = grouped(&open.told, remarks);
        if groups.is_empty() {
            ui.add_space(4.0);
            let said = if open.told.is_empty() { "  Nothing to say." } else { "  No errors (the remarks are not listed)." };
            ui.label(theme::text(said, theme::SMALL, theme::WEAK));
        }
        for group in groups {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                let (dot, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 3.5, severity_colour(group.first));
                if group.all.len() > 1 {
                    ui.label(theme::text(format!("{} ×", group.all.len()), theme::SMALL, theme::STRONG));
                }
                let lines = group.lines.iter().map(usize::to_string).collect::<Vec<_>>().join(", ");
                let mut hover = format!("line {lines}");
                if group.all.len() > 1 {
                    hover.push_str(&format!("\nthe first: {}", group.first.message));
                }
                for note in &group.first.notes {
                    hover.push_str(&format!("\n{note}"));
                }
                if ui.add(egui::Label::new(RichText::new(group.said()).color(theme::TEXT)).sense(Sense::click()).truncate()).on_hover_text(hover).clicked() {
                    picked.go = Some(group.first.span.start);
                }
                for line in group.lines.iter().take(6) {
                    if ui
                        .add(egui::Button::new(theme::text(format!("{line}"), theme::SMALL, theme::FAINT)).frame(false))
                        .on_hover_text("go to this line")
                        .clicked()
                    {
                        picked.go = Some(Spot { line: *line, column: 1 });
                    }
                }
                if group.lines.len() > 6 {
                    ui.label(theme::text(format!("+{}", group.lines.len() - 6), theme::SMALL, theme::FAINT));
                }
                if group.all.len() == 1 {
                    for remedy in group.first.fixes.iter().take(3) {
                        if ui.small_button(&remedy.title).on_hover_text("fix it this way").clicked() {
                            picked.fix = Some(film::service::applied(&open.text, &remedy.edits));
                        }
                    }
                } else if let Some(remedy) = group.first.fixes.first()
                    && group.all.iter().all(|one| !one.fixes.is_empty())
                    && ui
                        .small_button(format!("{} (all {})", remedy.title, group.all.len()))
                        .on_hover_text("the first fix of each, where they do not overlap")
                        .clicked()
                {
                    let all: Vec<Diagnostic> = group.all.iter().map(|one| (*one).clone()).collect();
                    picked.fix = Some(film::service::applied(&open.text, &fixes_together(&all)));
                }
            });
        }
    });
    picked
}

/// What was picked in the problems, done: the cursor there, the text fixed.
fn picked_done(open: &mut crate::Script, picked: Picked) {
    if let Some(spot) = picked.go {
        let at = index_of(&open.text, spot);
        open.jump = Some((at, at));
    }
    if let Some(made) = picked.fix {
        open.change(made);
    }
}

/// The Problems as a panel of their own (docked in the Film space): how many,
/// Fix All, the remarks listed or not, the list grouped.
pub fn problems_panel(ui: &mut egui::Ui, scripts: &mut crate::Scripts) {
    let (wrong, remarked) = scripts.counts();
    let Some(open) = &mut scripts.open else {
        ui.label(theme::weak("No film open."));
        return;
    };
    let mut fix_all = false;
    ui.horizontal(|ui| {
        let (colour, said) = problems_said(wrong, remarked);
        ui.label(theme::text(format!("● {said}"), theme::SMALL, colour));
        let looking = open.asked != open.told_for || open.changed.is_some();
        if looking {
            ui.label(theme::text("reading again…", theme::SMALL, theme::FAINT));
        }
        ui.checkbox(&mut scripts.remarks_listed, theme::text("remarks", theme::SMALL, theme::WEAK))
            .on_hover_text("the remarks listed too, not only the errors");
        let fixes = open.told.iter().filter(|told| !told.fixes.is_empty()).count();
        if fixes > 0 {
            fix_all =
                ui.small_button(format!("Fix All ({fixes})")).on_hover_text("the first fix offered of each, if they do not get in each other's way").clicked();
        }
    });
    ui.separator();
    let path = open.path.clone();
    let picked = listed(ui, open, scripts.remarks_listed, ("problems panel", path));
    if fix_all {
        let made = film::service::applied(&open.text, &fixes_together(&open.told));
        open.change(made);
    }
    picked_done(open, picked);
}

impl Body<'_> {
    /// The problems under the text: their head, and the list when it is open
    /// (nothing when they are a panel of their own).
    pub(super) fn problems(&mut self, ui: &mut egui::Ui, problems_rect: Rect, wrong: usize, remarked: usize) {
        if problems_rect.height() <= 0.0 {
            return;
        }
        let mut panel = ui.new_child(egui::UiBuilder::new().max_rect(problems_rect));
        panel.painter().rect_filled(problems_rect, 0.0, theme::PANEL);
        self.problems_head(&mut panel, problems_rect, wrong, remarked);
        if problems_shown(self.scripts.problems_open, wrong) && !self.scripts.concentration {
            let picked = listed(&mut panel, self.open, self.scripts.remarks_listed, ("problems", self.path));
            picked_done(self.open, picked);
        }
    }

    /// The problems' head: how many, what is said, Fix All, the remarks listed
    /// or not; a click elsewhere on it opens or closes the list.
    fn problems_head(&mut self, panel: &mut egui::Ui, problems_rect: Rect, wrong: usize, remarked: usize) {
        let Body { scripts, open, text_id, .. } = self;
        let shown = problems_shown(scripts.problems_open, wrong);
        let (head, head_response) = panel.allocate_exact_size(vec2(problems_rect.width(), 24.0), Sense::click());
        let painter = panel.painter().clone();
        painter.rect_filled(head, 0.0, theme::TABS);
        workshop_base::icons::chevron(&painter, pos2(head.min.x + 10.0, head.center().y), shown && !scripts.concentration, theme::TEXT);
        painter.text(pos2(head.min.x + 20.0, head.center().y), egui::Align2::LEFT_CENTER, "PROBLEMS", FontId::proportional(theme::SMALL), theme::STRONG);
        let looking = open.asked != open.told_for || open.changed.is_some();
        let alike = grouped(&open.told, scripts.remarks_listed).len();
        let folded = if alike < open.told.len() && scripts.remarks_listed { format!(" · {alike} kinds") } else { String::new() };
        let counts = format!("{wrong} error(s) · {remarked} remark(s){folded}{}", if looking { " · reading again…" } else { "" });
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
        } else if head_response.on_hover_text("open or close the list (View › Add Panel › Problems: a panel of their own)").clicked() {
            scripts.problems_open = Some(!shown);
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
        // (DS Mode: the film on DS in one line, left of the problems.)
        let mut left = shown.min.x;
        if film::ds::written_for_ds(&self.open.text)
            && let Some(report) = &self.scripts.ds.report
        {
            let summary = report.summary();
            let worst = report.notes.iter().map(|note| note.fate).filter(|fate| *fate != ds_export::Fate::Added).max();
            let colour = worst.map_or(Color32::from_rgb(90, 200, 120), crate::ds::fate_colour);
            let ds = ui.painter().text(
                shown.left_center() - vec2(14.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                summary.said(),
                FontId::proportional(theme::SMALL),
                colour,
            );
            ui.interact(ds.expand(3.0), self.text_id.with("ds status"), Sense::hover()).on_hover_text(format!(
                "of the {} lines that play: {} as written, {} changed to fit, {} left out, {} not possible (View › Add Panel › DS Report: each of them)",
                summary.lines, summary.as_written, summary.changed, summary.left_out, summary.impossible
            ));
            left = ds.min.x;
        }
        ui.painter().with_clip_rect(status.with_max_x(left - 10.0)).text(
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
