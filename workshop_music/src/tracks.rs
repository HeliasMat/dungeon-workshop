//! The tracks: their headers and the arrangement of their regions.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};
use shell::sound::Hears;

use super::editor::{colour, new_link};
use super::project::{Kind, RATE, Region, Track};
use super::wave::pcm;
use super::{LANE, MusicMaking, RULER, Tab, Taken};
use workshop_base::theme;
use workshop_sound::{self as sound, open_audio};

/// How many lanes fit, and the tracks shown (from the first scrolled to).
fn lanes_shown(making: &MusicMaking, high: f32) -> std::ops::Range<usize> {
    let fit = (((high - RULER) / LANE).floor() as usize).max(1);
    let first = making.first_track.min(making.project.tracks.len().saturating_sub(fit));
    first..(first + fit).min(making.project.tracks.len())
}

/// The tracks' heads: chosen by a click, named, muted or alone, how loud, from which side; tracks added.
pub(super) fn track_headers(ui: &mut egui::Ui, making: &mut MusicMaking, rect: Rect, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    ui.painter().rect_filled(rect, 3.0, theme::PANEL);
    let shown = lanes_shown(making, rect.height() - 30.0);
    let mut removed = None;
    for (row, at) in shown.clone().enumerate() {
        let lane = Rect::from_min_size(pos2(rect.min.x, rect.min.y + RULER + row as f32 * LANE), vec2(rect.width(), LANE - 2.0));
        let chosen = at == making.track;
        ui.painter().rect_filled(lane, 3.0, if chosen { theme::HOVERED } else { theme::GROUND });
        let track = &mut making.project.tracks[at];
        ui.painter().rect_filled(Rect::from_min_size(lane.min, vec2(5.0, lane.height())), 2.0, colour(track.colour));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(lane.shrink2(vec2(9.0, 3.0))).layout(egui::Layout::top_down(egui::Align::Min)));
        child.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut track.name).desired_width(96.0).font(egui::TextStyle::Small));
            for (said, on, help) in [("M", &mut track.muted, "mute"), ("S", &mut track.solo, "solo")] {
                let button = egui::Button::new(RichText::new(said).strong().size(theme::SMALL))
                    .fill(if *on { theme::FILM } else { theme::FIELD })
                    .min_size(vec2(20.0, 18.0));
                if ui.add(button).on_hover_text(help).clicked() {
                    *on = !*on;
                }
            }
            if ui.small_button("×").on_hover_text("remove the track").clicked() {
                removed = Some(at);
            }
        });
        child.horizontal(|ui| {
            let kind = match track.kind {
                Kind::Drums => "drums",
                Kind::Audio => "audio",
                Kind::Instrument => "instr.",
            };
            ui.label(theme::text(kind, theme::SMALL - 2.0, theme::FAINT));
            ui.spacing_mut().slider_width = 70.0;
            ui.add(egui::Slider::new(&mut track.volume, 0.0..=1.5).show_value(false)).on_hover_text("volume");
            ui.add(egui::DragValue::new(&mut track.pan).range(-1.0..=1.0).speed(0.01).max_decimals(2)).on_hover_text("pan: -1 left, 1 right");
            let peak = making.peaks.get(at).copied().unwrap_or(0.0);
            let (meter, _) = ui.allocate_exact_size(vec2(6.0, 16.0), Sense::hover());
            ui.painter().rect_filled(meter, 1.0, theme::GAP);
            let lit = Rect::from_min_max(pos2(meter.min.x, meter.max.y - meter.height() * peak.min(1.0)), meter.max);
            ui.painter().rect_filled(lit, 1.0, if peak > 0.98 { theme::WRONG } else { Color32::from_rgb(0x6C, 0xC6, 0x6C) });
        });
        let response = ui.interact(lane, egui::Id::new(("track head", at)), Sense::click());
        if response.clicked() {
            making.track = at;
            making.region = None;
        }
    }
    if let Some(at) = removed.filter(|_| making.project.tracks.len() > 1) {
        making.project.tracks.remove(at);
        for track in &mut making.project.tracks {
            track.sidechain = match track.sidechain {
                source if source == at + 1 => 0,
                source if source > at + 1 => source - 1,
                source => source,
            };
        }
        making.caches.clear();
        making.track = making.track.min(making.project.tracks.len() - 1);
        (making.region, making.editor) = (None, None);
    }
    // (Tracks added, and the ones out of sight reached.)
    let foot = Rect::from_min_max(pos2(rect.min.x + 4.0, rect.max.y - 28.0), rect.max - vec2(4.0, 2.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(foot).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.menu_button(RichText::new("+ Instr.").strong().color(theme::HOME), |ui| {
        for kind in sound::INSTRUMENTS {
            if ui.button(kind).clicked() {
                let at = making.project.tracks.len();
                making.project.tracks.push(Track::instrument(kind, at));
                making.track = at;
                (making.region, making.editor) = (None, None);
                let preview = sound::note_audio(&sound::instrument(kind), 0.0, 0.5, 0.9, &making.session.own_sounds);
                open_audio(&making.session, audio);
                if let Some(audio) = audio.as_mut() {
                    audio.samples(pcm(&preview.0, &preview.1), volume);
                }
                ui.close();
            }
        }
    });
    if child.button(RichText::new("+ Drums").strong().color(theme::HOME)).clicked() {
        let at = making.project.tracks.len();
        making.project.tracks.push(Track::drums(at));
        making.track = at;
        (making.region, making.editor) = (None, None);
    }
    if child
        .button(RichText::new("+ Audio").strong().color(theme::HOME))
        .on_hover_text("a track of sounds as they are (a double-click there makes a region; drop a .wav on the window)")
        .clicked()
    {
        let at = making.project.tracks.len();
        making.project.tracks.push(Track::audio(at));
        making.track = at;
        (making.region, making.editor) = (None, None);
    }
    let count = making.project.tracks.len();
    if shown.len() < count {
        if child.small_button("▲").clicked() {
            making.first_track = making.first_track.saturating_sub(1);
        }
        if child.small_button("▼").clicked() {
            making.first_track = (making.first_track + 1).min(count - shown.len());
        }
    }
}

/// Where the arrangement is on screen: the ruler, the lanes, the tracks shown, the beats along.
#[derive(Clone)]
struct Frame {
    rect: Rect,
    ruler: Rect,
    lanes: Rect,
    /// Pixels a beat; the first beat at the left edge.
    pixels: f32,
    scroll: f32,
    shown: std::ops::Range<usize>,
}

impl Frame {
    fn x_of(&self, beat: f32) -> f32 {
        self.rect.min.x + (beat - self.scroll) * self.pixels
    }

    fn beat_of(&self, x: f32) -> f32 {
        (x - self.rect.min.x) / self.pixels + self.scroll
    }

    fn lane_of(&self, row: usize) -> Rect {
        Rect::from_min_size(pos2(self.rect.min.x, self.lanes.min.y + row as f32 * LANE), vec2(self.rect.width(), LANE - 2.0))
    }

    fn region_rect(&self, row: usize, region: &Region) -> Rect {
        let lane = self.lane_of(row);
        Rect::from_min_max(pos2(self.x_of(region.start), lane.min.y + 2.0), pos2(self.x_of(region.start + region.length), lane.max.y - 2.0))
    }

    /// The track under a point, and the region there: its number, whether at its right edge.
    fn hit(&self, at: Pos2, making: &MusicMaking) -> Option<(usize, Option<(usize, bool)>)> {
        let row = ((at.y - self.lanes.min.y) / LANE).floor();
        if row < 0.0 {
            return None;
        }
        let track = self.shown.start + row as usize;
        let lane = making.project.tracks.get(track).filter(|_| track < self.shown.end)?;
        let found = lane.regions.iter().enumerate().rev().find(|(_, region)| self.region_rect(row as usize, region).contains(at));
        Some((track, found.map(|(number, region)| (number, (at.x - self.region_rect(row as usize, region).max.x).abs() < 7.0))))
    }
}

/// A beat put on the nearest step.
fn snap(beat: f32, step: f32) -> f32 {
    (beat / step).round() * step
}

/// The arrangement: the bars, the loop, each track's regions (moved, lengthened, made, copied linked with Alt,
/// taken off with a right click), the playhead (put with a click on the ruler); the wheel scrolls (Ctrl: zooms).
pub(super) fn arrangement(ui: &mut egui::Ui, making: &mut MusicMaking, rect: Rect) {
    let ruler = Rect::from_min_size(rect.min, vec2(rect.width(), RULER));
    let lanes = Rect::from_min_max(pos2(rect.min.x, ruler.max.y), rect.max);
    let frame = Frame { rect, ruler, lanes, pixels: making.pixels, scroll: making.scroll, shown: lanes_shown(making, rect.height()) };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    draw_bars(&painter, &frame, making);
    for (row, at) in frame.shown.clone().enumerate() {
        let track = &making.project.tracks[at];
        for (number, region) in track.regions.iter().enumerate() {
            let chosen = at == making.track && making.region == Some(number);
            draw_region(&painter, &frame, making, track, region, frame.region_rect(row, region), chosen);
        }
    }
    // (The playhead; the microphone's take growing.)
    let now = making.beat_now();
    painter.vline(frame.x_of(now), rect.y_range(), Stroke::new(2.0_f32, theme::FILM));
    if let Some((_, since)) = &making.recording {
        painter.rect_filled(
            Rect::from_min_max(pos2(frame.x_of(*since), lanes.min.y), pos2(frame.x_of(now), lanes.min.y + 6.0)),
            0.0,
            Color32::from_rgb(0xC0, 0x30, 0x30),
        );
    }
    on_ruler(ui, making, &frame);
    on_lanes(ui, making, &frame);
    wheel(ui, making, rect);
    // (Following the playhead while it plays.)
    if making.playing.is_some() && frame.x_of(now) > rect.max.x - 40.0 {
        making.scroll = now - 4.0;
    }
}

/// The bars and beats, the song's end, the loop.
fn draw_bars(painter: &egui::Painter, frame: &Frame, making: &MusicMaking) {
    let (ruler, lanes) = (frame.ruler, frame.lanes);
    let per_bar = making.project.beats_per_bar.max(1.0);
    let first = frame.scroll.floor().max(0.0) as i32;
    let last = frame.beat_of(frame.rect.max.x).ceil() as i32;
    for beat in first..=last {
        let x = frame.x_of(beat as f32);
        let bar = (beat as f32 % per_bar).abs() < 0.01;
        painter.vline(x, lanes.y_range(), Stroke::new(1.0_f32, if bar { theme::HOVERED } else { theme::PANEL }));
        if bar {
            painter.text(
                pos2(x + 3.0, ruler.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{}", (beat as f32 / per_bar) as i32 + 1),
                egui::FontId::proportional(theme::SMALL),
                theme::WEAK,
            );
        }
    }
    painter.rect_filled(Rect::from_min_max(pos2(frame.x_of(making.project.end()), lanes.min.y), lanes.max), 0.0, theme::GAP.gamma_multiply(0.7));
    let song = &making.project;
    if song.looping {
        let looped = Rect::from_min_max(pos2(frame.x_of(song.loop_start), ruler.min.y), pos2(frame.x_of(song.loop_end), ruler.max.y));
        painter.rect_filled(looped, 2.0, theme::FILM.gamma_multiply(0.45));
        painter.rect_filled(Rect::from_min_max(pos2(looped.min.x, lanes.min.y), pos2(looped.max.x, lanes.max.y)), 0.0, theme::FILM.gamma_multiply(0.05));
    }
}

/// A region, its notes or its sound small inside; a linked one marked.
fn draw_region(painter: &egui::Painter, frame: &Frame, making: &MusicMaking, track: &Track, region: &Region, shape: Rect, chosen: bool) {
    painter.rect_filled(shape, 4.0, colour(track.colour).gamma_multiply(if chosen { 0.7 } else { 0.4 }));
    painter.rect_stroke(
        shape,
        4.0,
        Stroke::new(if chosen { 2.0_f32 } else { 1.0 }, if chosen { Color32::WHITE } else { colour(track.colour) }),
        StrokeKind::Inside,
    );
    if region.link != 0 {
        painter.text(
            shape.left_top() + vec2(4.0, 2.0),
            egui::Align2::LEFT_TOP,
            format!("linked {}", region.link),
            egui::FontId::proportional(theme::SMALL - 2.5),
            Color32::from_white_alpha(170),
        );
    }
    if let Some(clip) = &region.clip {
        // (Its sound's shape: how loud, along the region.)
        if let Some(mono) = workshop_base::own_sounds::sample_of(&making.session.own_sounds, &clip.sound) {
            let speed = 2f32.powf(clip.tone / 12.0);
            let per_pixel = making.project.beat() / frame.pixels * RATE * speed;
            let columns = shape.width().max(1.0) as usize;
            for column in 0..columns {
                let from = clip.offset * RATE + column as f32 * per_pixel;
                let span = &mono[(from as usize).min(mono.len())..((from + per_pixel) as usize).min(mono.len())];
                let loud = span.iter().fold(0.0_f32, |most, sample| most.max(sample.abs())) * clip.gain;
                let x = shape.min.x + column as f32;
                painter.vline(
                    x,
                    (shape.center().y - loud * shape.height() * 0.45)..=(shape.center().y + loud * shape.height() * 0.45),
                    Stroke::new(1.0_f32, Color32::from_white_alpha(170)),
                );
            }
        }
        painter.text(
            shape.left_bottom() + vec2(4.0, -2.0),
            egui::Align2::LEFT_BOTTOM,
            &clip.sound,
            egui::FontId::proportional(theme::SMALL - 2.0),
            Color32::WHITE,
        );
        return;
    }
    let (low, high) = region.notes.iter().fold((f32::MAX, f32::MIN), |(low, high), note| (low.min(note.tone), high.max(note.tone)));
    let span = (high - low).max(6.0);
    for note in region.notes.iter().filter(|note| note.start < region.length) {
        let y = shape.max.y - 5.0 - (note.tone - low) / span * (shape.height() - 10.0);
        let x0 = frame.x_of(region.start + note.start);
        let x1 = frame.x_of(region.start + (note.start + note.length).min(region.length)).max(x0 + 1.5);
        painter.hline(x0..=x1, y, Stroke::new(2.0_f32, Color32::from_white_alpha(200)));
    }
}

/// The ruler: the playhead put, the loop drawn (Shift).
fn on_ruler(ui: &mut egui::Ui, making: &mut MusicMaking, frame: &Frame) {
    let on_ruler = ui.interact(frame.ruler, egui::Id::new("song ruler"), Sense::click_and_drag());
    let Some(at) = on_ruler.interact_pointer_pos() else {
        return;
    };
    let beat = snap(frame.beat_of(at.x), 1.0).max(0.0);
    let shift = ui.input(|input| input.modifiers.shift);
    if shift {
        let start_id = egui::Id::new("start of the dragged loop");
        if on_ruler.drag_started() || on_ruler.clicked() {
            ui.data_mut(|data| data.insert_temp(start_id, beat));
        }
        let start = ui.data(|data| data.get_temp::<f32>(start_id)).unwrap_or(beat);
        if (beat - start).abs() >= 1.0 {
            (making.project.loop_start, making.project.loop_end) = (start.min(beat), start.max(beat));
            making.project.looping = true;
        }
    } else if making.playing.is_none() {
        making.playhead = snap(frame.beat_of(at.x), 0.25).max(0.0);
    }
}

/// The lanes: a region taken off (right click), made (double click), chosen, taken to be moved or lengthened.
fn on_lanes(ui: &mut egui::Ui, making: &mut MusicMaking, frame: &Frame) {
    let response = ui.interact(frame.lanes, egui::Id::new("song tracks"), Sense::click_and_drag());
    if let Some(at) = response.interact_pointer_pos() {
        if response.secondary_clicked() {
            if let Some((track, Some((number, _)))) = frame.hit(at, making) {
                making.project.tracks[track].regions.remove(number);
                making.region = None;
            }
        } else if response.double_clicked() {
            if let Some((track, None)) = frame.hit(at, making) {
                let per_bar = making.project.beats_per_bar.max(1.0);
                let start = (frame.beat_of(at.x) / per_bar).floor().max(0.0) * per_bar;
                making.project.tracks[track].regions.push(Region::empty(start, per_bar));
                (making.track, making.region, making.tab) = (track, Some(making.project.tracks[track].regions.len() - 1), Tab::Notes);
            }
        } else if response.drag_started() || response.clicked() {
            match frame.hit(at, making) {
                Some((track, Some((number, edge)))) => take_region(ui, making, frame, &response, at, (track, number, edge)),
                Some((track, None)) => {
                    if making.track != track {
                        making.editor = None;
                    }
                    (making.track, making.region) = (track, None);
                }
                None => {}
            }
        }
        if response.dragged()
            && let Some(taken) = making.dragged
            && let Some(region) = making.project.tracks.get_mut(taken.track).and_then(|track| track.regions.get_mut(taken.region))
        {
            let beat = frame.beat_of(at.x);
            let step = if ui.input(|input| input.modifiers.shift) { 0.25 } else { 1.0 };
            if taken.edge {
                region.length = (snap(beat, step) - region.start).max(step);
            } else {
                region.start = snap(beat - taken.grab, step).max(0.0);
            }
        }
    }
    if response.drag_stopped() {
        making.dragged = None;
    }
}

/// A region clicked or taken to be dragged: chosen. (Alt: a copy taken, linked to it (one pattern), the region left
/// where it was.)
fn take_region(ui: &egui::Ui, making: &mut MusicMaking, frame: &Frame, response: &egui::Response, at: Pos2, (track, number, edge): (usize, usize, bool)) {
    let mut number = number;
    if ui.input(|input| input.modifiers.alt) && !edge && response.drag_started() {
        let link = match making.project.tracks[track].regions[number].link {
            0 => new_link(&making.project.tracks[track]),
            link => link,
        };
        making.project.tracks[track].regions[number].link = link;
        let copy = making.project.tracks[track].regions[number].clone();
        making.project.tracks[track].regions.push(copy);
        number = making.project.tracks[track].regions.len() - 1;
    }
    let region = &making.project.tracks[track].regions[number];
    making.dragged = response.drag_started().then_some(Taken { track, region: number, edge, grab: frame.beat_of(at.x) - region.start });
    if making.track != track || making.region != Some(number) {
        making.editor = None;
    }
    (making.track, making.region) = (track, Some(number));
}

/// The wheel: through the tracks; Shift: along the song; Ctrl: closer or further.
fn wheel(ui: &egui::Ui, making: &mut MusicMaking, rect: Rect) {
    if !ui.rect_contains_pointer(rect) {
        return;
    }
    let (delta, ctrl, shift) = ui.input(|input| (input.smooth_scroll_delta, input.modifiers.command, input.modifiers.shift));
    if ctrl && delta.y != 0.0 {
        making.pixels = (making.pixels * 1.0015_f32.powf(delta.y)).clamp(4.0, 160.0);
    } else if shift || delta.x != 0.0 {
        let along = if delta.x != 0.0 { delta.x } else { delta.y };
        making.scroll = (making.scroll - along / making.pixels).max(0.0);
    } else if delta.y != 0.0 {
        let count = making.project.tracks.len();
        if delta.y < -20.0 {
            making.first_track = (making.first_track + 1).min(count.saturating_sub(1));
        } else if delta.y > 20.0 {
            making.first_track = making.first_track.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_and_pixels_go_both_ways() {
        let rect = Rect::from_min_size(pos2(10.0, 0.0), vec2(400.0, 200.0));
        let frame = Frame { rect, ruler: rect, lanes: rect, pixels: 20.0, scroll: 2.0, shown: 0..0 };
        assert_eq!(frame.x_of(2.0), 10.0);
        assert_eq!(frame.x_of(4.0), 50.0);
        assert_eq!(frame.beat_of(frame.x_of(7.5)), 7.5);
        assert_eq!(snap(2.6, 1.0), 3.0);
        assert_eq!(snap(2.6, 0.25), 2.5);
    }
}
