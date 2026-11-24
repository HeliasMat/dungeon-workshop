//! The Timeline's bands under its ruler, in a column that scrolls down: the
//! scenes (named by their place), the moments, the music and the sounds, the
//! lines' bars (put on rows so that none covers another), the cast (who is
//! on the stage, who moves, who speaks); what a click, a drag or a right
//! click on them asks.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use preview::played::{LineSpan, Timeline, Track};

use super::lines::{seconds_between, stretchable};
use super::playing::{clock, line_frame};
use super::timeline::{Axis, KINDS, kind_of_line, speaker_of};
use super::{BarAct, Films, SECOND};
use crate::script::Scripts;
use workshop_base::theme;

/// What the bands ask: the line and who pointed (lit in the other panels),
/// the frame to go to, the line to choose in the Script, a line stretched to
/// so many seconds, a bar's act, a scene to fill the width.
#[derive(Default)]
pub(super) struct Asked {
    pub(super) lit_line: Option<usize>,
    pub(super) lit_who_track: Option<String>,
    pub(super) go: Option<usize>,
    pub(super) line_chosen: Option<usize>,
    pub(super) stretched: Option<(usize, f64)>,
    pub(super) bar_act: Option<BarAct>,
    pub(super) fit_scene: Option<(usize, usize)>,
}

/// The heights of the bands: the scenes, the moments, a row of lines, a member of the cast, the music.
const SCENES_H: f32 = 20.0;
const MOMENTS_H: f32 = 16.0;
const ROW_H: f32 = 19.0;
const TRACK_H: f32 = 16.0;
const MUSIC_H: f32 = 16.0;

/// The lines' spans put on rows, in the order they start, so that none covers
/// another (each as long as 6 frames at least, 2 apart): each with its row; how many rows.
pub(super) fn rows_of(spans: &[LineSpan]) -> (Vec<(&LineSpan, usize)>, usize) {
    let mut rows: Vec<usize> = Vec::new();
    let mut placed: Vec<(&LineSpan, usize)> = Vec::new();
    let mut spans: Vec<&LineSpan> = spans.iter().collect();
    spans.sort_by_key(|span| (span.start, span.number));
    for span in spans {
        let end_at = span.end.max(span.start + 6);
        let row = rows.iter().position(|free| *free <= span.start).unwrap_or_else(|| {
            rows.push(0);
            rows.len() - 1
        });
        rows[row] = end_at + 2;
        placed.push((span, row));
    }
    let count = rows.len();
    (placed, count)
}

/// A song or a sound asked: the frame it is asked at, its name.
pub(super) type Heard = (usize, String);

/// The songs asked (from the frame they are asked at) and the sounds, from the film's `music` and `sound` lines.
pub(super) fn songs_and_sounds(text_lines: &[&str], frame_of_line: impl Fn(usize) -> Option<usize>) -> (Vec<Heard>, Vec<Heard>) {
    let mut songs: Vec<Heard> = Vec::new();
    let mut sounds: Vec<Heard> = Vec::new();
    for (index, line) in text_lines.iter().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if let (Some(first), Some(name)) = (words.first(), words.get(1))
            && let Some(frame) = frame_of_line(index + 1)
        {
            match *first {
                "music" => songs.push((frame, name.to_string())),
                "sound" => sounds.push((frame, name.to_string())),
                _ => {}
            }
        }
    }
    songs.sort();
    (songs, sounds)
}

/// Why a line begins when it does, in a few words.
pub(super) fn cause_said(cause: &film::time::Cause) -> String {
    use film::time::Cause;
    match cause {
        Cause::Start => "at the start of the scene".to_string(),
        Cause::After(line) => format!("after the end of line {line}"),
        Cause::With(Some(line)) => format!("with line {line} (&)"),
        Cause::With(None) => "beside the film (&)".to_string(),
        Cause::Clock { offset, .. } => format!("at a time of the film ({offset} frames after)"),
        Cause::Event { what, .. } => format!("when {what}"),
    }
}

/// Where the bands are drawn: their painter, their rectangle, the time across.
struct Strip {
    painter: egui::Painter,
    rect: Rect,
    axis: Axis,
}

impl Strip {
    /// An area kept to the part where time is drawn.
    fn clip(&self, area: Rect) -> Rect {
        area.intersect(Rect::from_min_max(pos2(self.axis.content.min.x, self.rect.min.y), pos2(self.axis.content.max.x, self.rect.max.y)))
    }

    /// A band's name, on the left.
    fn label(&self, y: f32, said: &str, colour: Color32) {
        self.painter.text(pos2(self.rect.min.x + 6.0, y), egui::Align2::LEFT_CENTER, said, FontId::proportional(theme::SMALL - 1.0), colour);
    }

    /// The span of two frames, between two heights.
    fn span(&self, from: usize, to: usize, top: f32, bottom: f32) -> Rect {
        Rect::from_min_max(pos2(self.axis.x_of(from as f32), top), pos2(self.axis.x_of(to as f32), bottom))
    }
}

/// What the bands found as they were drawn, for the clicks: the scenes'
/// bands (their start, end, line), the moments' marks (x, frame, line), the bar pointed, a bar's end held.
struct Found<'a> {
    scene_bands: Vec<(Rect, usize, usize, Option<usize>)>,
    moment_marks: Vec<(f32, usize, usize)>,
    hovered_bar: Option<&'a LineSpan>,
    near_end: Option<(usize, usize)>,
}

/// The bands in their scrolling, and what they ask.
pub(super) fn bands(ui: &mut egui::Ui, films: &mut Films, scripts: &Scripts, timeline: &Timeline, axis: Axis, body: Rect) -> Asked {
    let text = scripts.open.as_ref().map(|open| open.text.clone()).unwrap_or_default();
    let text_lines: Vec<&str> = text.lines().collect();
    let cursor_line = scripts.open.as_ref().map_or(0, |open| open.cursor_line());
    let mut asked = Asked::default();
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(body));
    egui::ScrollArea::vertical().id_salt("timeline bottom").auto_shrink(false).show(&mut inside, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let (placed, rows) = rows_of(&timeline.spans);
        let tracks: Vec<Track> = timeline.tracks.values().cloned().collect();
        let lines_h = rows.max(1) as f32 * ROW_H;
        let height = SCENES_H + MOMENTS_H + MUSIC_H + lines_h + 22.0 + tracks.len() as f32 * TRACK_H + 12.0;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height.max(body.height())), Sense::click_and_drag());
        let strip = Strip { painter: ui.painter_at(rect.intersect(body)), rect, axis };
        let scenes_top = rect.min.y + 2.0;
        let outline = film::service::outline(&text);
        let scene_bands = scenes_band(&strip, films, timeline, &outline, scenes_top);
        let moments_top = scenes_top + SCENES_H;
        let moment_marks = moments_band(&strip, timeline, &outline, moments_top);
        let music_top = moments_top + MOMENTS_H;
        music_band(&strip, films, timeline, &text_lines, music_top);
        let lines_top = music_top + MUSIC_H + 2.0;
        let pointed = response.hover_pos();
        let (hovered_bar, near_end) = line_bars(&strip, films, scripts.hovered_line, &placed, &text_lines, cursor_line, lines_top, pointed);
        let cast_top = lines_top + lines_h + 8.0;
        let name_clicked = cast(&strip, films, scripts, timeline, &tracks, &text_lines, &response, cast_top, &mut asked);
        // (Where the film stands; where the pointer would take it.)
        let x = axis.x_of(films.frame() as f32);
        if x >= axis.content.min.x {
            strip.painter.vline(x, rect.y_range(), Stroke::new(1.5_f32, Color32::from_rgb(0xF1, 0x4C, 0x4C)));
        }
        if let Some(at) = pointed.filter(|at| at.x >= axis.content.min.x) {
            strip.painter.vline(at.x, rect.y_range(), Stroke::new(1.0_f32, Color32::from_white_alpha(50)));
        }
        if let Some(span) = hovered_bar {
            bar_said(&response, span, &text_lines);
        }
        asked.lit_line = hovered_bar.map(|span| span.number as usize);
        if let Some(who) = name_clicked {
            films.focus_who = if films.focus_who.as_ref() == Some(&who) { None } else { Some(who) };
        }
        let found = Found { scene_bands, moment_marks, hovered_bar, near_end };
        stretched(ui, &strip, films, &response, &found, &mut asked);
        clicked(films, &response, &found, axis, moments_top, &mut asked);
        bar_menu(films, &response, &found, &mut asked);
    });
    asked
}

/// The scenes, named by their place, one band each (alternating shades); their bands given back.
fn scenes_band(strip: &Strip, films: &Films, timeline: &Timeline, outline: &[film::service::Symbol], top: f32) -> Vec<(Rect, usize, usize, Option<usize>)> {
    strip.label(top + SCENES_H / 2.0, "Scenes", theme::FAINT);
    let scene_names: Vec<String> = outline.iter().filter(|symbol| symbol.kind == "scene").map(|symbol| symbol.name.clone()).collect();
    let scene_lines: Vec<usize> = outline.iter().filter(|symbol| symbol.kind == "scene").map(|symbol| symbol.line).collect();
    let mut scene_bands = Vec::new();
    for (at, scene) in timeline.scenes.iter().enumerate() {
        let until = timeline.scenes.get(at + 1).map_or(films.end, |next| next.start);
        let whole = strip.span(scene.start, until, top, top + SCENES_H - 3.0);
        scene_bands.push((whole, scene.start, until, scene_lines.get(at).copied()));
        let band = strip.clip(whole);
        if band.width() <= 0.0 {
            continue;
        }
        strip.painter.rect_filled(band, 2.0, if at % 2 == 0 { Color32::from_rgb(0x3A, 0x33, 0x22) } else { Color32::from_rgb(0x2E, 0x2A, 0x1F) });
        let name = scene_names.get(at).cloned().unwrap_or_else(|| format!("scene {}", at + 1));
        strip.painter.with_clip_rect(band).text(
            band.left_center() + vec2(6.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("{} · {name}", at + 1),
            FontId::proportional(theme::SMALL - 1.0),
            theme::FILM,
        );
    }
    scene_bands
}

/// The moments, a diamond and a name each; their marks given back.
fn moments_band(strip: &Strip, timeline: &Timeline, outline: &[film::service::Symbol], top: f32) -> Vec<(f32, usize, usize)> {
    let content = strip.axis.content;
    strip.label(top + MOMENTS_H / 2.0, "Moments", theme::FAINT);
    let mut moment_marks = Vec::new();
    for symbol in outline.iter().filter(|symbol| symbol.kind == "moment") {
        let Some(frame) = line_frame(timeline, symbol.line) else { continue };
        let x = strip.axis.x_of(frame as f32);
        moment_marks.push((x, frame, symbol.line));
        if x >= content.min.x && x <= content.max.x {
            let blue = Color32::from_rgb(0x9C, 0xDC, 0xFE);
            strip.painter.add(egui::Shape::convex_polygon(
                vec![pos2(x, top + 3.0), pos2(x + 5.0, top + 8.0), pos2(x, top + 13.0), pos2(x - 5.0, top + 8.0)],
                blue,
                Stroke::NONE,
            ));
            strip.painter.with_clip_rect(strip.clip(Rect::from_min_max(pos2(x + 7.0, top), pos2(content.max.x, top + MOMENTS_H)))).text(
                pos2(x + 8.0, top + 8.0),
                egui::Align2::LEFT_CENTER,
                &symbol.name,
                FontId::proportional(theme::SMALL - 1.5),
                blue,
            );
        }
    }
    moment_marks
}

/// The music: each song from where it is asked until the next; the sounds, ticks.
fn music_band(strip: &Strip, films: &Films, timeline: &Timeline, text_lines: &[&str], top: f32) {
    let content = strip.axis.content;
    strip.label(top + MUSIC_H / 2.0, "Music", theme::FAINT);
    let (songs, sounds) = songs_and_sounds(text_lines, |line| line_frame(timeline, line));
    for (at, (from, name)) in songs.iter().enumerate() {
        if matches!(name.as_str(), "off" | "stop" | "none") {
            continue;
        }
        let until = songs.get(at + 1).map_or(films.end, |next| next.0);
        let band = strip.clip(strip.span(*from, until, top + 2.0, top + MUSIC_H - 2.0));
        if band.width() > 0.0 {
            strip.painter.rect_filled(band, 2.0, KINDS[2].1.gamma_multiply(0.25));
            strip.painter.with_clip_rect(band).text(
                band.left_center() + vec2(5.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("♪ {name}"),
                FontId::proportional(theme::SMALL - 1.5),
                KINDS[2].1,
            );
        }
    }
    for (frame, _) in &sounds {
        let x = strip.axis.x_of(*frame as f32);
        if x >= content.min.x && x <= content.max.x {
            strip.painter.vline(x, top + 3.0..=top + MUSIC_H - 3.0, Stroke::new(2.0_f32, KINDS[2].1));
        }
    }
}

/// Each line as it played: a bar of its kind's colour, its words in it
/// (dimmed if not of the one focused on; lit if chosen or pointed in the
/// Script; its end a grip when it says how long). The bar pointed and the end held, given back.
#[allow(clippy::too_many_arguments)]
fn line_bars<'a>(
    strip: &Strip,
    films: &Films,
    hovered_line: Option<usize>,
    placed: &[(&'a LineSpan, usize)],
    text_lines: &[&str],
    cursor_line: usize,
    top: f32,
    pointed: Option<Pos2>,
) -> (Option<&'a LineSpan>, Option<(usize, usize)>) {
    let (painter, content) = (&strip.painter, strip.axis.content);
    strip.label(top + 10.0, "Lines", theme::FAINT);
    let mut hovered_bar = None;
    let mut near_end = None;
    for (span, row) in placed {
        let start = strip.axis.x_of(span.start as f32);
        let stop = strip.axis.x_of(span.end.max(span.start) as f32).max(start + 3.0);
        if stop < content.min.x || start > content.max.x {
            continue;
        }
        let bar = Rect::from_min_max(pos2(start, top + *row as f32 * ROW_H), pos2(stop, top + *row as f32 * ROW_H + ROW_H - 3.0));
        let words = text_lines.get(span.number as usize - 1).copied().unwrap_or_default();
        let kind = kind_of_line(words);
        if films.hidden_kinds[kind] {
            continue;
        }
        let colour = KINDS[kind].1;
        let beside = span.label.is_some() || span.thread > 0;
        let chosen = span.number as usize == cursor_line;
        let shown = strip.clip(bar);
        let dim = films.focus_who.as_ref().is_some_and(|who| !words.split(|letter: char| !letter.is_alphanumeric() && letter != '_').any(|word| word == who));
        let strength = if chosen {
            1.0
        } else if beside {
            0.35
        } else {
            0.55
        };
        painter.rect_filled(shown, 3.0, colour.gamma_multiply(if dim { 0.15 } else { strength }));
        // (Its end, held, makes it longer or shorter: a line that says how long.)
        let end_grip = Rect::from_min_max(pos2(bar.max.x - 4.0, bar.min.y), pos2(bar.max.x + 3.0, bar.max.y));
        if !beside && stretchable(words) && pointed.is_some_and(|at| end_grip.contains(at)) {
            near_end = Some((span.number as usize, span.start));
            painter.rect_filled(
                Rect::from_min_max(pos2(bar.max.x - 2.0, bar.min.y + 2.0), pos2(bar.max.x, bar.max.y - 2.0)).intersect(shown.expand(1.0)),
                0.0,
                Color32::WHITE,
            );
        }
        if beside {
            painter.rect_stroke(shown, 3.0, Stroke::new(1.0_f32, colour.gamma_multiply(0.8)), StrokeKind::Inside);
        }
        if chosen {
            painter.rect_stroke(shown, 3.0, Stroke::new(1.0_f32, Color32::WHITE), StrokeKind::Inside);
        }
        if hovered_line == Some(span.number as usize) {
            painter.rect_stroke(shown, 3.0, Stroke::new(2.0_f32, theme::POINTED), StrokeKind::Outside);
        }
        if shown.width() > 26.0 {
            painter.with_clip_rect(shown.shrink(2.0)).text(
                pos2(shown.min.x + 4.0, shown.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{}  {}", span.number, words.trim()),
                FontId::proportional(theme::SMALL - 1.0),
                Color32::WHITE,
            );
        }
        if pointed.is_some_and(|at| bar.contains(at)) {
            hovered_bar = Some(*span);
        }
    }
    (hovered_bar, near_end)
}

/// The cast: who is on the stage (faint), who moves, who speaks, a row each;
/// lit when pointed on the Scene; its name pointed, told; clicked, given back.
#[allow(clippy::too_many_arguments)]
fn cast(
    strip: &Strip,
    films: &Films,
    scripts: &Scripts,
    timeline: &Timeline,
    tracks: &[Track],
    text_lines: &[&str],
    response: &egui::Response,
    top: f32,
    asked: &mut Asked,
) -> Option<String> {
    let (painter, rect, content) = (&strip.painter, strip.rect, strip.axis.content);
    painter.hline(rect.x_range(), top, Stroke::new(1.0_f32, theme::GAP));
    strip.label(top + 8.0, "CAST", theme::WEAK);
    let speaking: Vec<(String, usize, usize)> =
        timeline.spans.iter().filter_map(|span| Some((speaker_of(text_lines.get(span.number as usize - 1)?)?, span.start, span.end))).collect();
    let pointed = response.hover_pos();
    let mut name_clicked: Option<String> = None;
    for (at, track) in tracks.iter().enumerate() {
        let y = top + 16.0 + at as f32 * TRACK_H;
        let focused = films.focus_who.as_ref() == Some(&track.name);
        let name_at = Rect::from_min_max(pos2(rect.min.x, y), pos2(content.min.x, y + TRACK_H));
        let lit = scripts.lit_who_scene.as_ref() == Some(&track.name);
        if lit {
            painter.rect_filled(Rect::from_min_max(pos2(rect.min.x, y), pos2(rect.max.x, y + TRACK_H)), 0.0, theme::POINTED.gamma_multiply(0.1));
        }
        if pointed.is_some_and(|at| name_at.contains(at)) {
            asked.lit_who_track = Some(track.name.clone());
        }
        strip.label(
            y + TRACK_H / 2.0,
            &track.name,
            if focused {
                theme::FILM
            } else if lit {
                theme::POINTED
            } else {
                theme::TEXT
            },
        );
        if response.clicked() && response.interact_pointer_pos().is_some_and(|at| name_at.contains(at)) {
            name_clicked = Some(track.name.clone());
        }
        let x_of = |frame: usize| strip.axis.x_of(frame as f32);
        for (from, to) in &track.shown {
            painter.rect_filled(
                strip.clip(Rect::from_min_max(pos2(x_of(*from), y + 6.0), pos2(x_of(*to).max(x_of(*from) + 1.0), y + 9.0))),
                1.0,
                Color32::from_white_alpha(38),
            );
        }
        for (from, to) in &track.moving {
            painter.rect_filled(
                strip.clip(Rect::from_min_max(pos2(x_of(*from), y + 4.0), pos2(x_of(*to).max(x_of(*from) + 1.0), y + 11.0))),
                2.0,
                KINDS[4].1.gamma_multiply(0.85),
            );
        }
        for (who, from, to) in &speaking {
            if who == &track.name {
                painter.rect_filled(
                    strip.clip(Rect::from_min_max(pos2(x_of(*from), y + 2.0), pos2(x_of(*to).max(x_of(*from) + 2.0), y + 13.0))),
                    2.0,
                    KINDS[0].1.gamma_multiply(0.9),
                );
            }
        }
    }
    name_clicked
}

/// A bar pointed at: what it is, when, how long, why, beside the film or not.
fn bar_said(response: &egui::Response, span: &LineSpan, text_lines: &[&str]) {
    let words = text_lines.get(span.number as usize - 1).copied().unwrap_or_default().trim().to_string();
    let why = span.cause.as_ref().map(cause_said).unwrap_or_default();
    response.clone().on_hover_text(format!(
        "line {}: {words}\n{} → {} ({:.2} s){}{}",
        span.number,
        clock(span.start as f64),
        clock(span.end as f64),
        (span.end as f64 - span.start as f64) / SECOND,
        if why.is_empty() { String::new() } else { format!("\n{why}") },
        if span.label.is_some() || span.thread > 0 { "\nbeside the film" } else { "" }
    ));
}

/// A bar's end held: how long it would be, shown; let go, the line stretched to it.
fn stretched(ui: &egui::Ui, strip: &Strip, films: &mut Films, response: &egui::Response, found: &Found, asked: &mut Asked) {
    if found.near_end.is_some() || films.stretching.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if response.drag_started() && found.near_end.is_some() {
        films.stretching = found.near_end;
    }
    if let (Some((_, start)), Some(at)) = (films.stretching, response.interact_pointer_pos()) {
        let seconds = seconds_between(start, strip.axis.frame_of(at.x));
        strip.painter.vline(at.x, strip.rect.y_range(), Stroke::new(1.0_f32, Color32::WHITE));
        crate::decors::tag(&strip.painter, pos2(at.x + 4.0, at.y - 18.0), &format!("{seconds} s"), Color32::WHITE);
        if response.drag_stopped()
            && let Some((line, _)) = films.stretching.take()
        {
            asked.stretched = Some((line, seconds));
        }
    } else if response.drag_stopped() {
        films.stretching = None;
    }
}

/// A click: on a bar, there and its line; on a moment or a scene, its start
/// and its line; elsewhere, that moment. A scene double-clicked fills the width.
fn clicked(films: &Films, response: &egui::Response, found: &Found, axis: Axis, moments_top: f32, asked: &mut Asked) {
    let at = response.interact_pointer_pos();
    if films.stretching.is_none() && asked.stretched.is_none() && (response.clicked() || response.dragged()) {
        let scene_hit = at.filter(|_| response.clicked()).and_then(|at| found.scene_bands.iter().find(|(band, ..)| band.contains(at)));
        let moment_hit = at
            .filter(|at| response.clicked() && (moments_top..moments_top + MOMENTS_H).contains(&at.y))
            .and_then(|at| found.moment_marks.iter().find(|(x, ..)| (x - at.x).abs() <= 6.0));
        if let (true, Some(span)) = (response.clicked(), found.hovered_bar) {
            asked.go = Some(span.start);
            asked.line_chosen = Some(span.number as usize);
        } else if let Some((_, frame, line)) = moment_hit {
            asked.go = Some(*frame);
            asked.line_chosen = Some(*line);
        } else if let Some((_, start, _, line)) = scene_hit {
            asked.go = Some(*start);
            asked.line_chosen = *line;
        } else if let Some(at) = at.filter(|at| at.x >= axis.content.min.x) {
            asked.go = Some(axis.frame_of(at.x));
        }
    }
    if response.double_clicked()
        && let Some((_, start, until, _)) = at.and_then(|at| found.scene_bands.iter().find(|(band, ..)| band.contains(at)))
    {
        asked.fit_scene = Some((*start, *until));
    }
}

/// A bar's menu (a right click on it): its line in the Script, looped, a wait after it, doubled, deleted.
fn bar_menu(films: &mut Films, response: &egui::Response, found: &Found, asked: &mut Asked) {
    if response.secondary_clicked() {
        films.menu_bar = found.hovered_bar.map(|span| (span.number as usize, span.start, span.end));
    }
    let menu_bar = films.menu_bar;
    response.context_menu(|ui| {
        let Some((line, start, stop)) = menu_bar else {
            ui.label(theme::weak("a right-click on a bar: its line"));
            return;
        };
        ui.label(theme::text(format!("Line {line}"), theme::SMALL, theme::WEAK));
        let items = [
            ("Go to the Line in the Script", BarAct::Script(line)),
            ("Loop This Line", BarAct::Loop(start, stop)),
            ("Add \"wait 1s\" After", BarAct::Wait(line)),
            ("Duplicate the Line", BarAct::Twice(line)),
            ("Delete the Line", BarAct::Gone(line)),
        ];
        for (said, act) in items {
            if ui.button(said).clicked() {
                asked.bar_act = Some(act);
                ui.close();
            }
        }
    });
}
