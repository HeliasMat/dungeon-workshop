//! The Timeline's bands under its ruler, in a column that scrolls down: the
//! scenes (named by their place), the moments, then a track for each of the
//! cast (their lines said, gestures, moves: a bar each, those at the same
//! time one under the other), the camera, the sound and the music, the
//! screen, the rest. What a click, a drag, a double click, a right click or
//! an asset let go on them asks.

use std::collections::BTreeMap;

use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use preview::played::{LineSpan, Timeline, Track};
use workshop_base::asset::Asset;

use super::adding::Inserting;
use super::edits::{asset_line, is_beside, stepped, stretchable, time_said};
use super::lanes::{Lane, bar_words, cast_lanes, lane_of};
use super::playing::{clock, line_frame};
use super::timeline::{Axis, KINDS, kind_of_line};
use super::{BarAct, Films, SECOND};
use workshop_base::theme;
use workshop_script::Scripts;

/// A bar moved on the Timeline, let go: its line, its start, by how many
/// frames, the lines after it moved too (Shift), from which track to which.
pub(super) struct Moved {
    pub(super) line: usize,
    pub(super) start: usize,
    pub(super) by: i64,
    pub(super) ripple: bool,
    pub(super) from: Lane,
    pub(super) to: Lane,
}

/// A bar held: its line, its start, its track, where it was taken, its rectangle then.
#[derive(Clone)]
pub(crate) struct BarDrag {
    line: usize,
    start: usize,
    lane: Lane,
    from_x: f32,
    rect: Rect,
}

/// What the bands ask: the line and who pointed (lit in the other panels),
/// the frame to go to, the line to choose in the Script, a line stretched to
/// so many frames, a bar's act, a scene to fill the width, a bar moved, a
/// line to write at a frame (beside the film or not).
#[derive(Default)]
pub(super) struct Asked {
    pub(super) lit_line: Option<usize>,
    pub(super) lit_who_track: Option<String>,
    pub(super) go: Option<usize>,
    pub(super) line_chosen: Option<usize>,
    pub(super) stretched: Option<(usize, usize)>,
    pub(super) bar_act: Option<BarAct>,
    pub(super) fit_scene: Option<(usize, usize)>,
    pub(super) moved: Option<Moved>,
    pub(super) inserted: Option<(usize, String, bool)>,
}

/// The heights of the bands: the scenes, the moments, a row of bars, a track folded, the music in its track.
const SCENES_H: f32 = 20.0;
const MOMENTS_H: f32 = 16.0;
const ROW_H: f32 = 18.0;
const FOLDED_H: f32 = 9.0;
const MUSIC_H: f32 = 13.0;
/// How wide the tracks' names are, on the left.
pub(super) const NAMES_WIDE: f32 = 118.0;

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

/// Words cut to fit so wide (`…` at their end), none if not even a letter fits.
pub(super) fn fitted(painter: &Painter, said: &str, font: &FontId, wide: f32) -> Option<String> {
    let width = |text: &str| painter.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE).size().x;
    if width(said) <= wide {
        return Some(said.to_string());
    }
    let letters: Vec<char> = said.chars().collect();
    let (mut low, mut high) = (0, letters.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        let cut: String = letters[..middle].iter().collect::<String>() + "…";
        if width(&cut) <= wide { low = middle } else { high = middle - 1 }
    }
    (low > 0).then(|| letters[..low].iter().collect::<String>().trim_end().to_string() + "…")
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

/// A track as laid out: what it is, from where to where down, its rows of bars, folded or not, who it is on the stage.
struct Row<'a> {
    lane: Lane,
    top: f32,
    bottom: f32,
    folded: bool,
    track: Option<&'a Track>,
    /// Its bars: each span, its row.
    bars: Vec<(LineSpan, usize)>,
}

/// A bar as drawn: its line's span, its track (by index), its rectangle, whether its end can be pulled.
struct Bar {
    span: LineSpan,
    row: usize,
    rect: Rect,
    end_pulled: bool,
}

/// The bands in their scrolling, and what they ask.
pub(super) fn bands(ui: &mut egui::Ui, films: &mut Films, scripts: &Scripts, timeline: &Timeline, axis: Axis, body: Rect) -> Asked {
    let text = scripts.open.as_ref().map(|open| open.text.clone()).unwrap_or_default();
    let text_lines: Vec<&str> = text.split('\n').collect();
    let cursor_line = scripts.open.as_ref().map_or(0, |open| open.cursor_line());
    let mut asked = Asked::default();
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(body));
    egui::ScrollArea::vertical().id_salt("timeline bottom").auto_shrink(false).show(&mut inside, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let rows = laid_out(films, timeline, &text_lines);
        let height = SCENES_H + MOMENTS_H + rows.iter().map(|row| row.bottom - row.top).sum::<f32>() + 16.0;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height.max(body.height())), Sense::click_and_drag());
        let strip = Strip { painter: ui.painter_at(rect.intersect(body)), rect, axis };
        let scenes_top = rect.min.y + 2.0;
        let outline = film::service::outline(&text);
        let scene_bands = scenes_band(&strip, films, timeline, &outline, scenes_top);
        let moments_top = scenes_top + SCENES_H;
        let moment_marks = moments_band(&strip, timeline, &outline, moments_top);
        let tracks_top = moments_top + MOMENTS_H;
        let rows: Vec<Row> = rows.into_iter().map(|row| Row { top: row.top + tracks_top, bottom: row.bottom + tracks_top, ..row }).collect();
        let pointed = ui.input(|input| input.pointer.latest_pos()).filter(|at| body.contains(*at));
        let bars = tracks(&strip, films, scripts, timeline, &rows, &text_lines, cursor_line, pointed);
        // (Where the film stands; where the pointer would take it.)
        let x = axis.x_of(films.frame() as f32);
        if x >= axis.content.min.x {
            strip.painter.vline(x, rect.y_range(), Stroke::new(1.5_f32, Color32::from_rgb(0xF1, 0x4C, 0x4C)));
        }
        if let Some(at) = pointed.filter(|at| at.x >= axis.content.min.x) {
            strip.painter.vline(at.x, rect.y_range(), Stroke::new(1.0_f32, Color32::from_white_alpha(50)));
        }
        let hovered = pointed.and_then(|at| bars.iter().rev().find(|bar| bar.rect.contains(at)));
        if let (Some(bar), None) = (hovered, films.bar_drag.as_ref()) {
            bar_said(&response, &bar.span, &text_lines, &rows[bar.row].lane);
        }
        asked.lit_line = hovered.map(|bar| bar.span.number as usize);
        let near_end = pointed.and_then(|at| {
            bars.iter().find(|bar| {
                bar.end_pulled && Rect::from_min_max(pos2(bar.rect.max.x - 4.0, bar.rect.min.y), pos2(bar.rect.max.x + 3.0, bar.rect.max.y)).contains(at)
            })
        });
        let near_end = near_end.map(|bar| (bar.span.number as usize, bar.span.start));
        stretched(ui, &strip, films, &response, near_end, &mut asked);
        dragged(ui, &strip, films, &response, &rows, hovered, near_end.is_some(), &mut asked);
        dropped(ui, &strip, films, &rows, body, &mut asked);
        let lane_at = |y: f32| rows.iter().find(|row| (row.top..row.bottom).contains(&y)).map(|row| &row.lane);
        let found = Found { scene_bands, moment_marks, hovered: hovered.map(|bar| bar.span.clone()) };
        clicked(films, &response, &found, axis, moments_top, &rows, &mut asked);
        if response.double_clicked()
            && found.hovered.is_none()
            && let Some(at) = response.interact_pointer_pos().filter(|at| at.x >= axis.content.min.x)
            && let Some(lane) = lane_at(at.y)
        {
            films.inserting = Some(Inserting::new(lane.clone(), stepped(axis.frame_of(at.x) as i64).max(0) as usize, at));
        }
        for row in &rows {
            if let Some(who) = row.lane.who()
                && pointed.is_some_and(|at| at.x < axis.content.min.x && (row.top..row.bottom).contains(&at.y))
            {
                asked.lit_who_track = Some(who.to_string());
            }
        }
        bar_menu(films, &response, &found, &text_lines, &mut asked);
    });
    asked
}

/// The tracks laid out (from 0 down): the cast's, then the others; each line's bar on the track of who does it, on rows.
fn laid_out<'a>(films: &Films, timeline: &'a Timeline, text_lines: &[&str]) -> Vec<Row<'a>> {
    let on_stage: Vec<String> = timeline.tracks.values().map(|track| track.name.clone()).collect();
    let known: Vec<String> = cast_lanes(&films.cast, &on_stage, &[]);
    let mut by_lane: BTreeMap<Lane, Vec<LineSpan>> = BTreeMap::new();
    for span in &timeline.spans {
        let Some(words) = text_lines.get((span.number as usize).wrapping_sub(1)) else { continue };
        if words.trim().is_empty() || films.hidden_kinds[kind_of_line(words)] {
            continue;
        }
        let lane = lane_of(words, films.fields.get(&(span.number as usize)), &known);
        by_lane.entry(lane).or_default().push(span.clone());
    }
    let doing: Vec<String> = by_lane.keys().filter_map(|lane| lane.who().map(str::to_string)).collect();
    // (A track for each of the scenes' cast and whoever does a line; not for the objects on the stage.)
    let in_scenes: Vec<String> = text_lines.iter().flat_map(|line| scene_cast(line)).collect();
    let lanes: Vec<Lane> = cast_lanes(&films.cast, &on_stage, &doing)
        .into_iter()
        .filter(|who| doing.contains(who) || in_scenes.contains(who))
        .map(Lane::Who)
        .chain(Lane::OTHERS)
        .collect();
    let mut top = 0.0;
    let mut rows = Vec::new();
    for lane in lanes {
        let spans = by_lane.remove(&lane).unwrap_or_default();
        let (placed, count) = rows_of(&spans);
        let bars: Vec<(LineSpan, usize)> = placed.into_iter().map(|(span, row)| (span.clone(), row)).collect();
        let folded = films.folded.contains(&lane);
        let music = if lane == Lane::Sound { MUSIC_H } else { 0.0 };
        let high = if folded { FOLDED_H } else { count.max(1) as f32 * ROW_H + 4.0 + music };
        let track = lane.who().and_then(|who| timeline.tracks.values().find(|track| track.name == who));
        rows.push(Row { lane, top, bottom: top + high, folded, track, bars });
        top += high;
    }
    rows
}

/// The cast of a `scene:` line (the words after its place), none for another line.
fn scene_cast(line: &str) -> Vec<String> {
    let said = line.split('#').next().unwrap_or_default();
    said.trim_start().strip_prefix("scene:").map_or_else(Vec::new, |rest| rest.split_whitespace().skip(1).map(str::to_string).collect())
}

/// What the bands found as they were drawn, for the clicks: the scenes'
/// bands (their start, end, line), the moments' marks (x, frame, line), the bar pointed.
struct Found {
    scene_bands: Vec<(Rect, usize, usize, Option<usize>)>,
    moment_marks: Vec<(f32, usize, usize)>,
    hovered: Option<LineSpan>,
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

/// The music in the sound's track: each song from where it is asked until the next; the sounds, ticks.
fn music_band(strip: &Strip, films: &Films, timeline: &Timeline, text_lines: &[&str], top: f32) {
    let content = strip.axis.content;
    let (songs, sounds) = songs_and_sounds(text_lines, |line| line_frame(timeline, line));
    for (at, (from, name)) in songs.iter().enumerate() {
        if matches!(name.as_str(), "off" | "stop" | "none") {
            continue;
        }
        let until = songs.get(at + 1).map_or(films.end, |next| next.0);
        let band = strip.clip(strip.span(*from, until, top + 1.0, top + MUSIC_H - 2.0));
        if band.width() > 0.0 {
            strip.painter.rect_filled(band, 2.0, KINDS[2].1.gamma_multiply(0.22));
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
            strip.painter.vline(x, top + 2.0..=top + MUSIC_H - 2.0, Stroke::new(2.0_f32, KINDS[2].1));
        }
    }
}

/// Each track: its name (▾ folds it), who is on the stage (faint) and moves
/// (a blue line), its lines' bars of their kind's colour with their words
/// (cut with `…` to the room before the next one; a line beside the film
/// paler, outlined; the cursor's lighter; pointed in the Script, ringed;
/// its end a grip when it says how long). The bars as drawn, given back.
#[allow(clippy::too_many_arguments)]
fn tracks(
    strip: &Strip,
    films: &Films,
    scripts: &Scripts,
    timeline: &Timeline,
    rows: &[Row],
    text_lines: &[&str],
    cursor_line: usize,
    pointed: Option<Pos2>,
) -> Vec<Bar> {
    let (painter, rect, content) = (&strip.painter, strip.rect, strip.axis.content);
    let font = FontId::proportional(theme::SMALL - 1.0);
    let mut drawn = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let whole = Rect::from_min_max(pos2(rect.min.x, row.top), pos2(rect.max.x, row.bottom));
        let who = row.lane.who();
        let focused = who.is_some() && films.focus_who.as_deref() == who;
        let lit = who.is_some() && scripts.lit_who_scene.as_deref() == who;
        if index % 2 == 0 {
            painter.rect_filled(whole, 0.0, Color32::from_white_alpha(4));
        }
        if focused || lit {
            painter.rect_filled(whole, 0.0, (if focused { theme::FILM } else { theme::POINTED }).gamma_multiply(0.08));
        }
        if index == 0 || (who.is_none() && rows[index - 1].lane.who().is_some()) {
            painter.hline(rect.x_range(), row.top, Stroke::new(1.0_f32, theme::GAP));
        }
        // (Its name, folded or not.)
        let name_y = row.top + (if row.folded { FOLDED_H } else { ROW_H }) / 2.0 + 1.0;
        let mark = if row.folded {
            vec![pos2(rect.min.x + 5.0, name_y - 4.0), pos2(rect.min.x + 10.0, name_y), pos2(rect.min.x + 5.0, name_y + 4.0)]
        } else {
            vec![pos2(rect.min.x + 4.0, name_y - 2.5), pos2(rect.min.x + 12.0, name_y - 2.5), pos2(rect.min.x + 8.0, name_y + 2.5)]
        };
        painter.add(egui::Shape::convex_polygon(mark, theme::FAINT, Stroke::NONE));
        let colour = if focused {
            theme::FILM
        } else if lit {
            theme::POINTED
        } else if who.is_some() {
            theme::TEXT
        } else {
            theme::WEAK
        };
        if let Some(name) = fitted(painter, &row.lane.name(), &font, NAMES_WIDE - 22.0) {
            painter.text(pos2(rect.min.x + 16.0, name_y), egui::Align2::LEFT_CENTER, name, font.clone(), colour);
        }
        let x_of = |frame: usize| strip.axis.x_of(frame as f32);
        if let Some(track) = row.track {
            for (from, to) in &track.shown {
                painter.rect_filled(
                    strip.clip(Rect::from_min_max(pos2(x_of(*from), row.top + 1.0), pos2(x_of(*to), row.bottom - 1.0))),
                    0.0,
                    Color32::from_white_alpha(9),
                );
            }
            for (from, to) in &track.moving {
                painter.rect_filled(
                    strip.clip(Rect::from_min_max(pos2(x_of(*from), row.bottom - 3.0), pos2(x_of(*to).max(x_of(*from) + 1.0), row.bottom - 1.0))),
                    1.0,
                    KINDS[4].1.gamma_multiply(0.9),
                );
            }
        }
        let bars_top = row.top + 2.0 + if row.lane == Lane::Sound && !row.folded { MUSIC_H } else { 0.0 };
        if row.lane == Lane::Sound && !row.folded {
            music_band(strip, films, timeline, text_lines, row.top + 1.0);
        }
        for (at, (span, line_row)) in row.bars.iter().enumerate() {
            let start = x_of(span.start);
            let stop = x_of(span.end.max(span.start)).max(start + 3.0);
            if stop < content.min.x || start > content.max.x {
                continue;
            }
            let (top, high) = if row.folded { (row.top + 2.0, FOLDED_H - 4.0) } else { (bars_top + *line_row as f32 * ROW_H, ROW_H - 3.0) };
            let bar = Rect::from_min_max(pos2(start, top), pos2(stop, top + high));
            let words = text_lines.get(span.number as usize - 1).copied().unwrap_or_default();
            let colour = KINDS[kind_of_line(words)].1;
            let beside = span.label.is_some() || span.thread > 0 || is_beside(words);
            let chosen = span.number as usize == cursor_line;
            let shown = strip.clip(bar);
            let strength = if chosen {
                1.0
            } else if beside {
                0.38
            } else {
                0.6
            };
            painter.rect_filled(shown, 3.0, colour.gamma_multiply(strength));
            let end_pulled = !row.folded && stretchable(words);
            if end_pulled && pointed.is_some_and(|at| Rect::from_min_max(pos2(bar.max.x - 4.0, bar.min.y), pos2(bar.max.x + 3.0, bar.max.y)).contains(at)) {
                painter.rect_filled(
                    Rect::from_min_max(pos2(bar.max.x - 2.0, bar.min.y + 2.0), pos2(bar.max.x, bar.max.y - 2.0)).intersect(shown.expand(1.0)),
                    0.0,
                    Color32::WHITE,
                );
            }
            if beside {
                painter.rect_stroke(shown, 3.0, Stroke::new(1.0_f32, colour.gamma_multiply(0.85)), StrokeKind::Inside);
            }
            if chosen {
                painter.rect_stroke(shown, 3.0, Stroke::new(1.0_f32, Color32::WHITE), StrokeKind::Inside);
            }
            if scripts.hovered_line == Some(span.number as usize) {
                painter.rect_stroke(shown, 3.0, Stroke::new(2.0_f32, theme::POINTED), StrokeKind::Outside);
            }
            // (Its words, in the room up to the next bar of its row: the verb seen even on a short bar.)
            if !row.folded {
                let next = row
                    .bars
                    .iter()
                    .skip(at + 1)
                    .filter(|(other, other_row)| other_row == line_row && other.start >= span.start)
                    .map(|(other, _)| x_of(other.start))
                    .fold(content.max.x, f32::min);
                let from = shown.min.x.max(start) + 4.0;
                let room = (next - from - 3.0).max(bar.width() - 6.0).min(content.max.x - from);
                if let Some(said) = fitted(painter, &bar_words(words, &row.lane), &font, room) {
                    let clip = Rect::from_min_max(pos2(from - 1.0, bar.min.y), pos2(from + room + 1.0, bar.max.y)).intersect(strip.clip(whole));
                    painter.with_clip_rect(clip).text(pos2(from, bar.center().y), egui::Align2::LEFT_CENTER, said, font.clone(), Color32::WHITE);
                }
            }
            drawn.push(Bar { span: span.clone(), row: index, rect: bar, end_pulled });
        }
    }
    drawn
}

/// A bar pointed at: what it is, when, how long, why, beside the film or
/// not; what can be done with it.
fn bar_said(response: &egui::Response, span: &LineSpan, text_lines: &[&str], lane: &Lane) {
    let words = text_lines.get(span.number as usize - 1).copied().unwrap_or_default().trim().to_string();
    let why = span.cause.as_ref().map(cause_said).unwrap_or_default();
    let mover = if lane.who().is_some() { "; onto another's track: they do it" } else { "" };
    response.clone().on_hover_text(format!(
        "line {}: {words}\n{} → {} ({:.2} s){}{}\n\ndrag: earlier or later (its wait){mover}\nShift+drag: the lines after it move too\nAlt+drag: alongside (&) or back in sequence",
        span.number,
        clock(span.start as f64),
        clock(span.end as f64),
        (span.end as f64 - span.start as f64) / SECOND,
        if why.is_empty() { String::new() } else { format!("\n{why}") },
        if span.label.is_some() || span.thread > 0 || super::edits::is_beside(&words) { "\nbeside the film" } else { "" }
    ));
}

/// A bar's end held: how long it would be, shown; let go, the line stretched to it.
fn stretched(ui: &egui::Ui, strip: &Strip, films: &mut Films, response: &egui::Response, near_end: Option<(usize, usize)>, asked: &mut Asked) {
    if near_end.is_some() || films.stretching.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if response.drag_started() && near_end.is_some() {
        films.stretching = near_end;
    }
    if let (Some((_, start)), Some(at)) = (films.stretching, response.interact_pointer_pos()) {
        let frames = strip.axis.frame_of(at.x).saturating_sub(start).max(1);
        strip.painter.vline(at.x, strip.rect.y_range(), Stroke::new(1.0_f32, Color32::WHITE));
        workshop_decor::tag(&strip.painter, pos2(at.x + 4.0, at.y - 18.0), &time_said(stepped(frames as i64).max(6) as usize), Color32::WHITE);
        if response.drag_stopped()
            && let Some((line, _)) = films.stretching.take()
        {
            asked.stretched = Some((line, frames));
        }
    } else if response.drag_stopped() {
        films.stretching = None;
    }
}

/// A bar's body held: where it would go shown (its time, the track of whom,
/// alongside or in sequence with Alt); let go, the move asked. Empty space
/// held: the time moved across.
#[allow(clippy::too_many_arguments)]
fn dragged(ui: &egui::Ui, strip: &Strip, films: &mut Films, response: &egui::Response, rows: &[Row], hovered: Option<&Bar>, at_end: bool, asked: &mut Asked) {
    let axis = strip.axis;
    let (alt, shift, origin) = ui.input(|input| (input.modifiers.alt, input.modifiers.shift, input.pointer.press_origin()));
    let middle = response.dragged_by(egui::PointerButton::Middle);
    if response.drag_started() && films.stretching.is_none() && !at_end {
        let held = origin.and_then(|at| hovered.filter(|bar| bar.rect.contains(at)));
        match held.filter(|_| !middle) {
            Some(bar) => {
                films.bar_drag = Some(BarDrag {
                    line: bar.span.number as usize,
                    start: bar.span.start,
                    lane: rows[bar.row].lane.clone(),
                    from_x: origin.map_or(bar.rect.min.x, |at| at.x),
                    rect: bar.rect,
                })
            }
            None => films.pan_from = origin.filter(|at| at.x >= axis.content.min.x || middle).map(|at| (films.left, at.x)),
        }
    }
    let at = ui.input(|input| input.pointer.latest_pos());
    if let (Some((left, from)), Some(at)) = (films.pan_from, at) {
        films.left = (left - (at.x - from) / axis.per_frame).clamp(0.0, axis.end as f32);
        films.follow_head = false;
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if let (Some(held), Some(at)) = (films.bar_drag.clone(), at) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let by = stepped(((at.x - held.from_x) / axis.per_frame) as i64).max(-(held.start as i64));
        let to = rows.iter().find(|row| (row.top..row.bottom).contains(&at.y)).map_or(held.lane.clone(), |row| row.lane.clone());
        let to = if to.who().is_some() && held.lane.who().is_some() { to } else { held.lane.clone() };
        let row_top = rows.iter().find(|row| row.lane == to).map_or(held.rect.min.y, |row| row.top + 2.0);
        let ghost = held.rect.translate(vec2(if alt { 0.0 } else { by as f32 * axis.per_frame }, if alt { 0.0 } else { row_top - held.rect.min.y }));
        strip.painter.rect_stroke(strip.clip(ghost), 3.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Inside);
        let said = if alt {
            "Alt: alongside (&) / in sequence".to_string()
        } else {
            let when =
                format!("{} ({}{})", clock((held.start as i64 + by).max(0) as f64), if by >= 0 { "+" } else { "−" }, time_said(by.unsigned_abs() as usize));
            let who = to.who().filter(|_| to != held.lane).map(|who| format!(" → {who}")).unwrap_or_default();
            format!("{when}{who}{}", if shift { " · the rest moves too" } else { "" })
        };
        workshop_decor::tag(&strip.painter, pos2(ghost.min.x.max(axis.content.min.x), ghost.min.y - 2.0), &said, Color32::WHITE);
        if response.drag_stopped() {
            films.bar_drag = None;
            if alt {
                asked.bar_act = Some(BarAct::Beside(held.line));
            } else if by != 0 || to != held.lane {
                asked.moved = Some(Moved { line: held.line, start: held.start, by, ripple: shift, from: held.lane, to });
            }
        }
    }
    if response.drag_stopped() || !ui.input(|input| input.pointer.any_down()) {
        films.pan_from = None;
        if !response.dragged() {
            films.bar_drag = None;
        }
    }
}

/// An asset of the Assets panel let go over a track: written for whose track it is, at that time.
fn dropped(ui: &egui::Ui, strip: &Strip, films: &mut Films, rows: &[Row], body: Rect, asked: &mut Asked) {
    let Some(asset) = egui::DragAndDrop::payload::<Asset>(ui.ctx()) else { return };
    let axis = strip.axis;
    let Some(at) = ui.input(|input| input.pointer.latest_pos()).filter(|at| body.contains(*at) && at.x >= axis.content.min.x) else { return };
    let Some(row) = rows.iter().find(|row| (row.top..row.bottom).contains(&at.y)) else { return };
    let frame = stepped(axis.frame_of(at.x) as i64).max(0) as usize;
    let words = asset_line(&asset, row.lane.who());
    let lane = Rect::from_min_max(pos2(axis.content.min.x, row.top), pos2(axis.content.max.x, row.bottom));
    strip.painter.rect_stroke(lane, 0.0, Stroke::new(1.0_f32, theme::FILM), StrokeKind::Inside);
    strip.painter.vline(axis.x_of(frame as f32), row.top..=row.bottom, Stroke::new(2.0_f32, theme::FILM));
    let said = match &words {
        Some(words) => format!("{words} at {}", clock(frame as f64)),
        None => "on someone's track".to_string(),
    };
    workshop_decor::tag(&strip.painter, pos2(at.x + 10.0, row.top), &said, theme::FILM);
    ui.ctx().request_repaint();
    if ui.input(|input| input.pointer.any_released()) && egui::DragAndDrop::take_payload::<Asset>(ui.ctx()).is_some() {
        match words {
            Some(words) => asked.inserted = Some((frame, words, false)),
            None => films.said = Some(format!("{}: let go on someone's track", asset.said())),
        }
    }
}

/// A click: on a track's name, it folded (▸) or the one chosen; on a bar,
/// there and its line; on a moment or a scene, its start and its line;
/// elsewhere, that moment. A scene double-clicked fills the width.
fn clicked(films: &mut Films, response: &egui::Response, found: &Found, axis: Axis, moments_top: f32, rows: &[Row], asked: &mut Asked) {
    let at = response.interact_pointer_pos();
    if films.stretching.is_none() && asked.stretched.is_none() && response.clicked() {
        let scene_hit = at.and_then(|at| found.scene_bands.iter().find(|(band, ..)| band.contains(at)));
        let moment_hit = at
            .filter(|at| (moments_top..moments_top + MOMENTS_H).contains(&at.y))
            .and_then(|at| found.moment_marks.iter().find(|(x, ..)| (x - at.x).abs() <= 6.0));
        let name_hit =
            at.filter(|at| at.x < axis.content.min.x).and_then(|at| rows.iter().find(|row| (row.top..row.bottom).contains(&at.y)).map(|row| (at, row)));
        if let Some((at, row)) = name_hit {
            match row.lane.who().filter(|_| at.x > axis.content.min.x - NAMES_WIDE + 14.0) {
                Some(who) => films.choose(who.to_string()),
                None if !films.folded.remove(&row.lane) => _ = films.folded.insert(row.lane.clone()),
                None => {}
            }
        } else if let Some(span) = &found.hovered {
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

/// A bar's menu (a right click on it): its line in the Script, looped, a
/// wait after it, alongside or in sequence, doubled, deleted.
fn bar_menu(films: &mut Films, response: &egui::Response, found: &Found, text_lines: &[&str], asked: &mut Asked) {
    if response.secondary_clicked() {
        films.menu_bar = found.hovered.as_ref().map(|span| (span.number as usize, span.start, span.end));
    }
    let menu_bar = films.menu_bar;
    response.context_menu(|ui| {
        let Some((line, start, stop)) = menu_bar else {
            ui.label(theme::weak("a right-click on a bar: its line; a double-click on a track: a line written there"));
            return;
        };
        ui.label(theme::text(format!("Line {line}"), theme::SMALL, theme::WEAK));
        let beside = text_lines.get(line - 1).is_some_and(|words| is_beside(words));
        let items = [
            ("Go to the Line in the Script", BarAct::Script(line)),
            ("Loop This Line", BarAct::Loop(start, stop)),
            ("Add \"wait 1s\" After", BarAct::Wait(line)),
            (if beside { "Back in Sequence (no &)" } else { "Run Alongside (&)" }, BarAct::Beside(line)),
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
