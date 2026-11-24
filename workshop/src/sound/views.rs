//! The sound seen: its wave, its pitch, its spectrum, its measures.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use shell::sound::Hears;

use super::versions::{hear, open_audio};
use super::{Sound, SoundMaking, WAVES};
use crate::done::Done;
use crate::theme;

/// A sound's spectrum: how strong each pitch is (log from 60 Hz to 12 kHz, low at the bottom) along its time.
pub(super) fn spectrum(mono: &[f32], columns: usize, rows: usize) -> Vec<f32> {
    let rate = shell::audio::OUTPUT_RATE as f32;
    let window = 512.min(mono.len().max(1));
    let mut out = vec![0.0_f32; columns * rows];
    if mono.len() < 64 {
        return out;
    }
    let hann: Vec<f32> = (0..window).map(|at| 0.5 - 0.5 * (std::f32::consts::TAU * at as f32 / window as f32).cos()).collect();
    let frequencies: Vec<f32> = (0..rows).map(|row| 60.0 * (12_000.0_f32 / 60.0).powf(row as f32 / (rows - 1).max(1) as f32)).collect();
    for column in 0..columns {
        let start = ((mono.len() - window) as f32 * column as f32 / (columns - 1).max(1) as f32) as usize;
        let piece = &mono[start..start + window];
        for (row, frequency) in frequencies.iter().enumerate() {
            // (Goertzel: one frequency's strength in the piece.)
            let coefficient = 2.0 * (std::f32::consts::TAU * frequency / rate).cos();
            let (mut before, mut last) = (0.0_f32, 0.0_f32);
            for (sample, weight) in piece.iter().zip(&hann) {
                let now = sample * weight + coefficient * last - before;
                before = last;
                last = now;
            }
            let power = (before * before + last * last - coefficient * before * last).max(0.0);
            let decibels = 10.0 * (power.max(1e-9)).log10();
            out[(rows - 1 - row) * columns + column] = ((decibels + 30.0) / 70.0).clamp(0.0, 1.0);
        }
    }
    out
}

/// A strength as a colour: dark, violet, orange, yellow.
fn heat(level: f32) -> Color32 {
    let stops = [(0.0, [0x14, 0x12, 0x1E]), (0.35, [0x55, 0x2A, 0x8C]), (0.65, [0xD8, 0x5A, 0x3A]), (1.0, [0xFA, 0xE6, 0x7A])];
    for pair in stops.windows(2) {
        let ((from, a), (to, b)) = (pair[0], pair[1]);
        if level <= to {
            let part = ((level - from) / (to - from)).clamp(0.0, 1.0);
            let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * part) as u8;
            return Color32::from_rgb(mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2]));
        }
    }
    Color32::from_rgb(0xFA, 0xE6, 0x7A)
}

/// Beside the sound: its measures, a sound of the game to compare, variants written at once, its versions.
pub(super) fn side_sound(ui: &mut egui::Ui, making: &mut SoundMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32, done: &mut Option<Done>) {
    let (stereo, measure) = making.rendered();
    let seconds = stereo.len() as f32 / 2.0 / shell::audio::OUTPUT_RATE as f32;
    ui.label(theme::text("MEASURES", theme::SMALL, theme::WEAK));
    egui::Grid::new("measures").num_columns(2).spacing([8.0, 3.0]).show(ui, |ui| {
        ui.label(theme::text("Duration", theme::SMALL, theme::WEAK));
        ui.label(theme::text(format!("{seconds:.2} s"), theme::SMALL, theme::TEXT));
        ui.end_row();
        ui.label(theme::text("Peak", theme::SMALL, theme::WEAK));
        ui.label(theme::text(format!("{:.1} dB", measure.peak), theme::SMALL, if measure.peak > -0.1 { theme::WRONG } else { theme::TEXT }));
        ui.end_row();
        ui.label(theme::text("Average", theme::SMALL, theme::WEAK));
        ui.label(theme::text(format!("{:.1} dB", measure.mean), theme::SMALL, theme::TEXT));
        ui.end_row();
    });
    // (The level, as a meter.)
    let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
    ui.painter().rect_filled(bar, 2.0, theme::GROUND);
    let fill = |level: f32| ((level + 48.0) / 48.0).clamp(0.0, 1.0);
    ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * fill(measure.mean), bar.height())), 2.0, theme::ASSETS);
    ui.painter().vline(
        bar.min.x + bar.width() * fill(measure.peak),
        bar.y_range(),
        Stroke::new(2.0_f32, if measure.peak > -0.1 { theme::WRONG } else { Color32::WHITE }),
    );
    if measure.clipped > 0 {
        ui.label(theme::text(format!("clips ({} samples): lower a volume or normalize", measure.clipped), theme::SMALL - 1.0, theme::WRONG));
    }
    ui.add_space(8.0);
    ui.separator();
    // --- A sound of the game, to compare ---
    ui.label(theme::text("COMPARE", theme::SMALL, theme::WEAK));
    let noises = rules::songs::NOISES;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("game sound").selected_text(noises[making.reference.min(noises.len() - 1)].word).width(110.0).show_ui(ui, |ui| {
            for (at, noise) in noises.iter().enumerate() {
                ui.selectable_value(&mut making.reference, at, noise.word).on_hover_text(noise.when);
            }
        });
        if ui.button("▶").on_hover_text("the game's sound, to compare").clicked() {
            open_audio(&making.session, audio);
            if let Some(audio) = audio.as_mut() {
                audio.sound(noises[making.reference.min(noises.len() - 1)].number, volume, 1.0);
            }
        }
    });
    ui.add_space(8.0);
    ui.separator();
    // --- So many variants, written at once ---
    ui.label(theme::text("VARIANTS", theme::SMALL, theme::WEAK));
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut making.variants).range(2..=8));
        if ui.button("Save").on_hover_text("the sound and its variants, each changed a little: word_1, word_2… (steps, hits: never twice the same)").clicked()
        {
            match making.save_variants() {
                Ok(words) => {
                    making.said =
                        Some(format!("written: sound {} … sound {}", words.first().cloned().unwrap_or_default(), words.last().cloned().unwrap_or_default()));
                    if let Some(first) = words.first() {
                        *done = Some(Done::Sound(first.clone()));
                    }
                }
                Err(why) => making.said = Some(why),
            }
        }
    });
    ui.add_space(8.0);
    ui.separator();
    // --- Its versions: a click brings one back ---
    ui.label(theme::text("VERSIONS", theme::SMALL, theme::WEAK));
    let mut back: Option<Sound> = None;
    egui::ScrollArea::vertical().id_salt("sound versions").show(ui, |ui| {
        let count = making.versions.len();
        for (at, (said, sound)) in making.versions.iter().enumerate() {
            let current = *sound == making.sound;
            let length: f32 = sound.voices.iter().map(|voice| voice.offset + voice.synth.seconds()).fold(0.0, f32::max);
            let label = format!("{} · {said} · {length:.2} s", count - at);
            if ui.selectable_label(current, RichText::new(label).size(theme::SMALL)).clicked() {
                back = Some(sound.clone());
            }
        }
    });
    if let Some(sound) = back {
        making.undone.push(std::mem::replace(&mut making.sound, sound));
        making.voice = making.voice.min(making.sound.voices.len() - 1);
        making.saved = false;
        hear(making, audio, volume);
    }
    ui.add_space(6.0);
    let word = film::words::plain(&making.word);
    ui.label(theme::text("IN A FILM", theme::SMALL, theme::WEAK));
    ui.add(egui::Label::new(RichText::new(format!("sound {word}")).monospace().size(theme::SMALL).color(Color32::from_rgb(0xDC, 0xDC, 0xAA))).selectable(true));
}

/// The mix's wave drawn (left over, right under), the chosen voice's envelope and its handles on it
/// (dragged: its attack, sustain, punch, decay), where it is heard now. Whether a handle was taken, let go.
pub(super) fn draw_wave(ui: &egui::Ui, making: &mut SoundMaking, rect: Rect, response: &egui::Response) -> (bool, bool) {
    let (stereo, _) = making.rendered();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    let rate = shell::audio::OUTPUT_RATE as f32;
    let frames = stereo.len() / 2;
    let total = (frames as f32 / rate).max(0.01);
    let lanes = [Rect::from_min_max(rect.min, pos2(rect.max.x, rect.center().y - 1.0)), Rect::from_min_max(pos2(rect.min.x, rect.center().y + 1.0), rect.max)];
    for (side, lane) in lanes.iter().enumerate() {
        painter.hline(lane.x_range(), lane.center().y, Stroke::new(1.0_f32, theme::GAP));
        if frames == 0 {
            continue;
        }
        let columns = rect.width().max(1.0) as usize;
        let per = (frames as f32 / columns as f32).max(1.0);
        for column in 0..columns {
            let from = (column as f32 * per) as usize;
            if from >= frames {
                break;
            }
            let to = (((column + 1) as f32 * per) as usize).clamp(from + 1, frames);
            let (low, high) = (from..to).map(|at| stereo[at * 2 + side]).fold((0.0_f32, 0.0_f32), |(low, high), sample| (low.min(sample), high.max(sample)));
            painter.vline(
                rect.min.x + column as f32,
                (lane.center().y - high * lane.height() * 0.46)..=(lane.center().y - low * lane.height() * 0.46),
                Stroke::new(1.0_f32, theme::ASSETS),
            );
        }
    }
    painter.text(rect.left_top() + vec2(4.0, 2.0), egui::Align2::LEFT_TOP, "G", FontId::proportional(theme::SMALL - 2.0), theme::FAINT);
    painter.text(pos2(rect.min.x + 4.0, rect.center().y + 2.0), egui::Align2::LEFT_TOP, "D", FontId::proportional(theme::SMALL - 2.0), theme::FAINT);
    // (The chosen voice's envelope and its handles, where they fall on the mix's time.)
    let voice = making.sound.voices[making.voice].clone();
    let synth = voice.synth;
    let editable = !making.sound.mix.reverse;
    let speed = making.sound.mix.speed.max(0.1);
    let x_of = |time: f32| rect.min.x + rect.width() * ((voice.offset + time) / speed / total);
    let y_of = |level: f32| rect.max.y - 6.0 - (level / 2.0).clamp(0.0, 1.0) * (rect.height() - 12.0);
    let points: Vec<Pos2> = (0..=80).map(|step| synth.seconds() * step as f32 / 80.0).map(|time| pos2(x_of(time), y_of(synth.envelope_at(time)))).collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.5_f32, theme::FILM)));
    let handles =
        [pos2(x_of(synth.attack), y_of(1.0 + synth.punch)), pos2(x_of(synth.attack + synth.sustain), y_of(1.0)), pos2(x_of(synth.seconds()), y_of(0.0))];
    let mut taken = false;
    let mut let_go = false;
    if editable {
        for (at, handle) in handles.iter().enumerate() {
            let chosen = making.handle == Some(at);
            painter.circle_filled(*handle, if chosen { 6.0 } else { 4.5 }, theme::FILM);
            painter.circle_stroke(*handle, if chosen { 6.0 } else { 4.5 }, Stroke::new(1.0_f32, Color32::BLACK));
        }
        if response.drag_started()
            && let Some(at) = response.interact_pointer_pos()
        {
            making.handle = handles
                .iter()
                .enumerate()
                .map(|(index, handle)| (index, handle.distance(at)))
                .filter(|(_, distance)| *distance < 12.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(index, _)| index);
            taken = making.handle.is_some();
        }
        if let Some(handle) = making.handle {
            let delta = response.drag_delta();
            let seconds = delta.x / rect.width() * total * speed;
            let synth = &mut making.sound.voices[making.voice].synth;
            match handle {
                0 => {
                    synth.attack = (synth.attack + seconds).clamp(0.0, 2.0);
                    synth.punch = (synth.punch - delta.y / (rect.height() - 12.0) * 2.0).clamp(0.0, 1.0);
                }
                1 => synth.sustain = (synth.sustain + seconds).clamp(0.0, 2.0),
                _ => synth.decay = (synth.decay + seconds).clamp(0.0, 3.0),
            }
            if response.drag_stopped() {
                *synth = synth.tidy();
                making.handle = None;
                let_go = true;
            }
        }
        if response.hovered() && handles.iter().any(|handle| response.hover_pos().is_some_and(|at| handle.distance(at) < 12.0)) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }
    if let Some((since, length)) = making.played {
        let along = (since.elapsed().as_secs_f32() / length.max(0.01)).min(1.0);
        painter.vline(rect.min.x + rect.width() * along, rect.y_range(), Stroke::new(1.5_f32, Color32::WHITE));
    }
    painter.text(
        rect.right_top() + vec2(-6.0, 4.0),
        egui::Align2::RIGHT_TOP,
        format!("{total:.2} s · layer {}: {}", making.voice + 1, WAVES[synth.wave.min(WAVES.len() - 1)]),
        FontId::proportional(theme::SMALL - 1.0),
        theme::FAINT,
    );
    (taken, let_go)
}

/// The chosen voice's pitch along its time (from 30 Hz to 8 kHz, as the ear hears them).
pub(super) fn draw_pitch(ui: &egui::Ui, making: &SoundMaking, rect: Rect) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    let synth = making.synth();
    let seconds = synth.seconds().max(0.01);
    let y_of = |pitch: f32| rect.max.y - 4.0 - ((pitch.max(30.0) / 30.0).log2() / (8000.0_f32 / 30.0).log2()).clamp(0.0, 1.0) * (rect.height() - 8.0);
    for octave in [110.0_f32, 440.0, 1760.0] {
        painter.hline(rect.x_range(), y_of(octave), Stroke::new(1.0_f32, theme::GAP));
        painter.text(
            pos2(rect.max.x - 4.0, y_of(octave)),
            egui::Align2::RIGHT_BOTTOM,
            format!("{octave:.0}"),
            FontId::proportional(theme::SMALL - 2.5),
            theme::FAINT,
        );
    }
    let mut points = Vec::new();
    for step in 0..=120 {
        let time = seconds * step as f32 / 120.0;
        match synth.pitch_at(time) {
            Some(pitch) => points.push(pos2(rect.min.x + rect.width() * step as f32 / 120.0, y_of(pitch))),
            None => break,
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, Stroke::new(1.5_f32, Color32::from_rgb(0x6C, 0xB6, 0xFF))));
    }
    painter.text(rect.left_top() + vec2(4.0, 2.0), egui::Align2::LEFT_TOP, "pitch (layer)", FontId::proportional(theme::SMALL - 2.0), theme::FAINT);
}

/// The mix's spectrum, low pitches at the bottom (made again only when it changed).
pub(super) fn draw_spectrum(ui: &egui::Ui, making: &mut SoundMaking, rect: Rect) {
    if making.picture.as_ref().is_none_or(|(sound, _)| *sound != making.sound) {
        let (stereo, _) = making.rendered();
        let mono: Vec<f32> = stereo.chunks(2).map(|pair| (pair[0] + pair.get(1).copied().unwrap_or(0.0)) / 2.0).collect();
        let (columns, rows) = (160, 48);
        let levels = spectrum(&mono, columns, rows);
        let pixels: Vec<Color32> = levels.into_iter().map(heat).collect();
        let image = egui::ColorImage { size: [columns, rows], pixels, source_size: Vec2::new(columns as f32, rows as f32) };
        let texture = ui.ctx().load_texture("spectrum", image, egui::TextureOptions::LINEAR);
        making.picture = Some((making.sound.clone(), texture));
    }
    let painter = ui.painter_at(rect);
    if let Some((_, texture)) = &making.picture {
        painter.image(texture.id(), rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    }
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, theme::GAP), StrokeKind::Inside);
    painter.text(
        rect.left_top() + vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        "spectrum (mix)",
        FontId::proportional(theme::SMALL - 2.0),
        Color32::from_white_alpha(160),
    );
}
