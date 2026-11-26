//! The film on DS, before it is exported: what `ds_export::report` tells
//! of it, found again on a thread of its own a moment after each change
//! (the export without writing anything, about a second). In DS Mode, a
//! mark in the margin of each line that loses something (its colour and
//! shape by how much: changed, left out, not possible; what, pointed at);
//! the DS Report, a panel listing the notes by kind (a click: the line);
//! the summary in the status ("on DS: 92% as written, 12 changed…").

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use ds_export::{Fate, Report};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, pos2, vec2};
use workshop_base::theme;

/// How long after the last change the report is made again (it takes about a second).
const MADE_AFTER: Duration = Duration::from_millis(700);

/// The colour of a fate: changed, left out, not possible; added.
pub(crate) fn fate_colour(fate: Fate) -> Color32 {
    match fate {
        Fate::Added => Color32::from_rgb(0x6C, 0xC6, 0x6C),
        Fate::Changed => Color32::from_rgb(0xE0, 0xB8, 0x4A),
        Fate::LeftOut => Color32::from_rgb(0xF0, 0x86, 0x3C),
        Fate::Impossible => Color32::from_rgb(0xF1, 0x4C, 0x4C),
    }
}

/// A fate's mark, there, so big (half its side): changed a diamond, left out
/// a triangle pointing down, not possible a cross, added a plus.
pub(crate) fn fate_mark(painter: &egui::Painter, at: Pos2, half: f32, fate: Fate, colour: Color32) {
    match fate {
        Fate::Changed => {
            let points = vec![at + vec2(0.0, -half), at + vec2(half, 0.0), at + vec2(0.0, half), at + vec2(-half, 0.0)];
            painter.add(egui::Shape::convex_polygon(points, colour, Stroke::NONE));
        }
        Fate::LeftOut => {
            let points = vec![at + vec2(-half, -half * 0.7), at + vec2(half, -half * 0.7), at + vec2(0.0, half)];
            painter.add(egui::Shape::convex_polygon(points, colour, Stroke::NONE));
        }
        Fate::Impossible => {
            let stroke = Stroke::new(2.0_f32, colour);
            painter.line_segment([at + vec2(-half, -half), at + vec2(half, half)], stroke);
            painter.line_segment([at + vec2(-half, half), at + vec2(half, -half)], stroke);
        }
        Fate::Added => {
            let stroke = Stroke::new(2.0_f32, colour);
            painter.line_segment([at + vec2(-half, 0.0), at + vec2(half, 0.0)], stroke);
            painter.line_segment([at + vec2(0.0, -half), at + vec2(0.0, half)], stroke);
        }
    }
}

/// The DS report of the film open: the thread making it, the last made
/// (of which film and revision), the last asked.
pub struct Ds {
    asks: Sender<(u64, String)>,
    answers: Receiver<(u64, Report)>,
    pub(crate) report: Option<Report>,
    made_for: (PathBuf, u64),
    asked: u64,
}

impl Ds {
    /// Its thread: of the texts asked meanwhile, the last only; each
    /// compiled and its export told, nothing written.
    pub(crate) fn new(context: &Arc<film::Context>, waker: &egui::Context) -> Ds {
        let (asks, asked) = channel::<(u64, String)>();
        let (answer, answers) = channel::<(u64, Report)>();
        let (context, waker) = (context.clone(), waker.clone());
        std::thread::spawn(move || {
            while let Ok(mut last) = asked.recv() {
                while let Ok(newer) = asked.try_recv() {
                    last = newer;
                }
                let tables = context.tables();
                let written = film::compile_with(&last.1, tables.clone());
                let report = ds_export::report(&ds_export::Film { name: "film", text: &last.1, written: &written, tables: &tables, art: None });
                if answer.send((last.0, report)).is_err() {
                    break;
                }
                waker.request_repaint();
            }
        });
        Ds { asks, answers, report: None, made_for: (PathBuf::new(), 0), asked: 0 }
    }

    /// Each frame: the report made taken; asked again a moment after the
    /// last change, when it is looked at (`wanted`). The film's path, its
    /// text's revision, how long since it changed.
    pub(crate) fn tick(&mut self, wanted: bool, path: &std::path::Path, revision: u64, text: &str, quiet: Duration) {
        while let Ok((made, report)) = self.answers.try_recv() {
            if made >= self.made_for.1 {
                self.report = Some(report);
                self.made_for.1 = made;
            }
        }
        if self.made_for.0 != path {
            // (Another film: nothing of the last one shown on it.)
            (self.report, self.made_for) = (None, (path.to_path_buf(), 0));
            self.asked = 0;
        }
        if wanted && self.asked != revision && (quiet >= MADE_AFTER || self.report.is_none()) {
            _ = self.asks.send((revision, text.to_string()));
            self.asked = revision;
        }
    }

    /// Whether the report shown is of an older text than the one asked for.
    pub(crate) fn stale(&self, revision: u64) -> bool {
        self.made_for.1 != revision
    }
}

/// What a line's notes say, pointed at: each with its fate and what the DS does instead.
pub(crate) fn said_of(report: &Report, line: usize) -> Vec<(Fate, String)> {
    report
        .on_line(line)
        .map(|note| {
            let instead = if note.instead.is_empty() { String::new() } else { format!("\n   on DS: {}", note.instead) };
            (note.fate, format!("{} ({}){instead}", note.message, note.fate.word()))
        })
        .collect()
}

/// A note's line as the Report lists it, the film as a whole for none.
fn line_said(line: Option<usize>) -> String {
    line.map_or("film".to_string(), |line| line.to_string())
}

/// The DS Report as a panel of its own (View › Add Panel): the summary, the
/// notes by kind with how many, a click on one going to its line.
pub fn ds_panel(ui: &mut egui::Ui, scripts: &mut crate::Scripts) {
    let Some(open) = &mut scripts.open else {
        ui.label(theme::weak("No film open."));
        return;
    };
    let Some(report) = scripts.ds.report.clone() else {
        ui.label(theme::weak("Reading the film as the DS would play it…"));
        return;
    };
    ui.horizontal(|ui| {
        let summary = report.summary();
        ui.label(theme::text(summary.said(), theme::SMALL, theme::STRONG)).on_hover_text(format!(
            "of the {} lines that play: {} as written, {} changed to fit, {} left out, {} not possible on DS",
            summary.lines, summary.as_written, summary.changed, summary.left_out, summary.impossible
        ));
        if scripts.ds.stale(open.revision) {
            ui.label(theme::text("reading again…", theme::SMALL, theme::FAINT));
        }
    });
    if !film::ds::written_for_ds(&open.text) {
        ui.label(theme::text("DS Mode is off: the Script's “…” › DS Mode marks these lines in the margin.", theme::SMALL, theme::FAINT));
    }
    ui.separator();
    let mut go = None;
    egui::ScrollArea::vertical().id_salt(("ds report", open.path.clone())).auto_shrink(false).show(ui, |ui| {
        if report.notes.is_empty() {
            ui.label(theme::text("  Nothing lost: the DS plays it as written.", theme::SMALL, theme::WEAK));
        }
        for (kind, notes) in report.by_kind() {
            let worst = notes.iter().map(|note| note.fate).max().unwrap_or(Fate::Changed);
            let head = RichText::new(format!("{} · {}", kind.title(), notes.len())).size(theme::SMALL + 1.0).color(fate_colour(worst));
            egui::CollapsingHeader::new(head).id_salt(("ds kind", kind)).default_open(true).show(ui, |ui| {
                for note in notes {
                    ui.horizontal(|ui| {
                        let (mark, _) = ui.allocate_exact_size(vec2(12.0, 16.0), Sense::hover());
                        fate_mark(ui.painter(), mark.center(), 3.5, note.fate, fate_colour(note.fate));
                        let line =
                            ui.add(egui::Button::new(theme::text(line_said(note.line), theme::SMALL, theme::FAINT)).frame(false).min_size(vec2(26.0, 0.0)));
                        let mut hover = format!("{} on DS", note.fate.word());
                        if !note.instead.is_empty() {
                            hover.push_str(&format!(": {}", note.instead));
                        }
                        let said =
                            ui.add(egui::Label::new(RichText::new(&note.message).color(theme::TEXT)).sense(Sense::click()).truncate()).on_hover_text(hover);
                        if (said.clicked() || line.clicked())
                            && let Some(line) = note.line
                        {
                            go = Some(line);
                        }
                    });
                }
            });
        }
    });
    if let Some(line) = go {
        open.go_to_line(line);
    }
}

/// The margin's DS mark of a line, beside the text: the worst it loses, its
/// notes said when pointed at (faint when DS Mode is off and the Report is open).
pub(crate) fn margin_mark(ui: &egui::Ui, painter: &egui::Painter, id: egui::Id, report: &Report, line: usize, at: Rect, faint: bool) {
    let Some(worst) = report.worst(line) else { return };
    let colour = if faint { fate_colour(worst).gamma_multiply(0.45) } else { fate_colour(worst) };
    let centre = at.center();
    fate_mark(painter, centre, 2.8, worst, colour);
    if ui.rect_contains_pointer(at.expand(2.0)) {
        egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), id.with(("ds", line)), egui::PopupAnchor::Pointer).show(|ui| {
            ui.label(theme::text("On DS", theme::SMALL, theme::WEAK));
            for (fate, said) in said_of(report, line) {
                ui.horizontal(|ui| {
                    let (mark, _) = ui.allocate_exact_size(vec2(10.0, 14.0), Sense::hover());
                    fate_mark(ui.painter(), mark.center(), 3.0, fate, fate_colour(fate));
                    ui.label(RichText::new(said).size(theme::SMALL + 1.0));
                });
            }
        });
    }
}

/// Where the margin's DS mark of a line is: just left of the text.
pub(crate) fn mark_rect(editor_left: f32, top: f32, bottom: f32) -> Rect {
    let middle = (top + bottom) / 2.0;
    Rect::from_center_size(pos2(editor_left + crate::GUTTER - 4.5, middle), vec2(7.0, 10.0))
}

/// The film as the DS shows it, to play on the Scene (DS Preview): each
/// line the DS leaves out (an item, a zoom, a word it lacks) made a comment,
/// each line said in a bubble said in the box; the lines where they are,
/// so the Script and the Timeline still agree. A line that opens a block
/// is kept, not to break the film.
pub fn ds_look(text: &str, report: &Report) -> String {
    let mut out: Vec<String> = Vec::new();
    for (at, line) in text.split('\n').enumerate() {
        let number = at + 1;
        let bare = line.trim();
        let opens = bare.ends_with(':') || bare.starts_with("scene") || bare.starts_with('#') || bare.is_empty();
        let shown = match report.worst(number) {
            Some(Fate::LeftOut | Fate::Impossible) if !opens => {
                let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
                format!("{lead}# (DS Preview: left out) {bare}")
            }
            _ if report.on_line(number).any(|note| note.kind == ds_export::Kind::Bubble) => without_bubble(line),
            _ => line.to_string(),
        };
        out.push(shown);
    }
    out.join("\n")
}

/// A line said in a bubble (`pichu (bubble, joyous): …`) said in the box
/// (`pichu (joyous): …`).
fn without_bubble(line: &str) -> String {
    let Some((head, said)) = line.split_once(':') else { return line.to_string() };
    let (Some(open), Some(close)) = (head.find('('), head.rfind(')')) else { return line.to_string() };
    let kept: Vec<&str> = head[open + 1..close].split(',').map(str::trim).filter(|word| *word != "bubble" && *word != "top").collect();
    let who = head[..open].trim_end();
    let brackets = if kept.is_empty() { String::new() } else { format!(" ({})", kept.join(", ")) };
    format!("{who}{brackets}:{said}")
}

impl crate::Scripts {
    /// The film open as the DS shows it (`ds_look`), and the revision of the
    /// text its report was made of: none until there is one.
    pub fn ds_preview_text(&self) -> Option<(u64, String)> {
        let open = self.open.as_ref()?;
        let report = self.ds.report.as_ref()?;
        Some((self.ds.made_for.1, ds_look(&open.text, report)))
    }
}
