//! The notes of a region: the piano roll and its tools, chords and
//! arpeggios, velocities, a clip, the step sequencer.

use std::time::Instant;

use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, pos2, vec2};
use shell::sound::Hears;

use super::editor::{new_link, scale_of};
use super::mixer::{automation_view, instrument_view, mixer_view};
use super::project::{Clip, Kind, RATE, Region};
use super::wave::pcm;
use super::{ARPEGGIOS, CHORDS, MusicMaking, Playing, SCALES, TONICS, Tab, own_sounds, project};
use crate::making::{self, Note, open_audio};
use crate::theme;

/// The bottom: the region's notes (a piano roll, a step sequencer, a clip), the track's instrument (the sound
/// maker), the mixer, the lanes drawn. Its rect.
pub(super) fn editor(ui: &mut egui::Ui, making: &mut MusicMaking, rect: Rect, audio: &mut Option<Box<dyn Hears>>, volume: f32) -> Rect {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    let ui = &mut child;
    ui.horizontal(|ui| {
        for (tab, said) in [(Tab::Notes, "Notes"), (Tab::Instrument, "Instrument"), (Tab::Mixer, "Mixer"), (Tab::Automation, "Automation")] {
            if ui.selectable_label(making.tab == tab, RichText::new(said).strong()).clicked() {
                making.tab = tab;
            }
        }
        ui.separator();
        if let Some(track) = making.project.tracks.get(making.track) {
            ui.label(theme::text(format!("track: {}", track.name), theme::SMALL, theme::WEAK));
        }
    });
    let place = making.track;
    match making.tab {
        Tab::Notes => {
            egui::ScrollArea::vertical().id_salt("region notes").auto_shrink(false).show(ui, |ui| notes_view(ui, making, audio, volume));
        }
        Tab::Instrument => instrument_view(ui, making, place, audio, volume),
        Tab::Mixer => mixer_view(ui, making),
        Tab::Automation => automation_view(ui, making),
    }
    rect
}

/// What the notes tools ask for, done after the region is let go.
enum Asked {
    Hear(f32, f32),
    Copy(Region),
}

/// The region chosen: its notes, written on a piano roll (a scale lit, chords, an arpeggio, strummed, humanized,
/// legato; their forces below) or a step sequencer; an audio region's sound; quantized, transposed, emptied,
/// heard, linked or made its own.
fn notes_view(ui: &mut egui::Ui, making: &mut MusicMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    let place = making.track;
    let Some(number) = making.region.filter(|number| making.project.tracks.get(place).is_some_and(|track| *number < track.regions.len())) else {
        ui.label(theme::text(
            "Choose a region in a track (a double-click in a track makes one; Alt + drag makes a linked copy; right-click removes it).",
            theme::SMALL,
            theme::FAINT,
        ));
        return;
    };
    notes_choices(ui, making, making.project.tracks[place].kind);
    let mut asked = None;
    let kind = making.project.tracks[place].kind;
    let per_bar = making.project.beats_per_bar.max(1.0);
    let beat = making.project.beat();
    let scale = scale_of(making);
    let names: Vec<String> = making.project.tracks[place].kit.iter().map(|(name, _)| name.clone()).collect();
    let link_next = new_link(&making.project.tracks[place]);
    let (chord, arpeggio, step, strum) = (making.chord, making.arpeggio, making.arpeggio_step, making.strum);
    let region = &mut making.project.tracks[place].regions[number];
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::text(format!("start: bar {}", (region.start / per_bar).floor() as i32 + 1), theme::SMALL, theme::WEAK));
        ui.label(theme::text("length", theme::SMALL, theme::WEAK));
        ui.add(egui::DragValue::new(&mut region.length).range(0.25..=512.0).speed(0.1).suffix(" beats"));
        ui.separator();
        if kind != Kind::Audio {
            if ui.button("Quantize").on_hover_text("each note on the nearest sixteenth").clicked() {
                for note in &mut region.notes {
                    note.start = (note.start * 4.0).round() / 4.0;
                    note.length = ((note.length * 4.0).round() / 4.0).max(0.25);
                }
            }
            if ui.button("Clear").clicked() {
                region.notes.clear();
            }
        }
        if ui.button("Linked Copy").on_hover_text("the same just after, linked: changing one changes the other (a pattern)").clicked() {
            if region.link == 0 {
                region.link = link_next;
            }
            asked = Some(Asked::Copy(Region { start: region.start + region.length, ..region.clone() }));
        }
        if region.link != 0 && ui.button("Make Unique").on_hover_text("no longer linked to the others: it changes alone").clicked() {
            region.link = 0;
        }
        if ui.button("▶ Listen to the Region").clicked() {
            asked = Some(Asked::Hear(region.start, region.start + region.length));
        }
    });
    let id = egui::Id::new(("region notes", place, number));
    match kind {
        Kind::Instrument => {
            notes_tools(ui, &mut region.notes, chord, arpeggio, step, strum, scale);
            let beats = region.length;
            making::piano_roll(ui, &mut region.notes, id, beats, true, scale);
            velocities(ui, &mut region.notes, beats);
        }
        Kind::Drums => {
            let beats = region.length;
            step_sequencer(ui, &mut region.notes, &names, beats, id);
            velocities(ui, &mut region.notes, beats);
        }
        Kind::Audio => clip_view(ui, &making.session, region, beat),
    }
    match asked {
        Some(Asked::Hear(from, to)) => {
            making.stop(audio);
            let song = making.project.clone();
            let played = song.render(&mut making.caches, from, to, false, &making.session.own_sounds);
            open_audio(&making.session, audio);
            if let Some(audio) = audio.as_mut() {
                audio.play_held(pcm(&played.left, &played.right), volume);
                making.playing = Some(Playing { since: Instant::now(), from, to });
            }
        }
        Some(Asked::Copy(copy)) => {
            making.project.tracks[place].regions.push(copy);
            making.region = Some(making.project.tracks[place].regions.len() - 1);
        }
        None => {}
    }
}

/// The choices of the notes' tools: the scale and its tonic, the chord, the arpeggio and its step, the strum.
fn notes_choices(ui: &mut egui::Ui, making: &mut MusicMaking, kind: Kind) {
    if kind != Kind::Instrument {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::text("Scale", theme::SMALL, theme::WEAK));
        egui::ComboBox::from_id_salt("scale").selected_text(SCALES[making.scale].0).show_ui(ui, |ui| {
            for (at, (name, _)) in SCALES.iter().enumerate() {
                ui.selectable_value(&mut making.scale, at, *name);
            }
        });
        egui::ComboBox::from_id_salt("tonic").selected_text(TONICS[making.tonic]).width(60.0).show_ui(ui, |ui| {
            for (at, name) in TONICS.iter().enumerate() {
                ui.selectable_value(&mut making.tonic, at, *name);
            }
        });
        ui.separator();
        ui.label(theme::text("Chord", theme::SMALL, theme::WEAK));
        egui::ComboBox::from_id_salt("chord").selected_text(CHORDS[making.chord].0).show_ui(ui, |ui| {
            for (at, (name, _)) in CHORDS.iter().enumerate() {
                ui.selectable_value(&mut making.chord, at, *name);
            }
        });
        ui.label(theme::text("Arpeggio", theme::SMALL, theme::WEAK));
        egui::ComboBox::from_id_salt("arpeggio").selected_text(ARPEGGIOS[making.arpeggio]).show_ui(ui, |ui| {
            for (at, name) in ARPEGGIOS.iter().enumerate() {
                ui.selectable_value(&mut making.arpeggio, at, *name);
            }
        });
        for (said, value) in [("1/4", 1.0), ("1/8", 0.5), ("1/16", 0.25), ("1/32", 0.125)] {
            ui.selectable_value(&mut making.arpeggio_step, value, said);
        }
        ui.label(theme::text("Strum", theme::SMALL, theme::WEAK));
        ui.add(egui::DragValue::new(&mut making.strum).range(0.0..=0.5).speed(0.005).suffix(" beats"));
    });
}

/// The piano roll's tools: the notes brought into the scale, made chords, arpeggiated, strummed, humanized,
/// joined (legato), transposed.
#[allow(clippy::too_many_arguments)]
fn notes_tools(ui: &mut egui::Ui, notes: &mut Vec<Note>, chord: usize, arpeggio: usize, step: f32, strum: f32, scale: Option<[bool; 12]>) {
    ui.horizontal_wrapped(|ui| {
        for (said, by) in [("−12", -12.0), ("−1", -1.0), ("+1", 1.0), ("+12", 12.0)] {
            if ui.small_button(said).on_hover_text("transpose, in semitones").clicked() {
                notes.iter_mut().for_each(|note| note.tone += by);
            }
        }
        ui.separator();
        if let Some(scale) = scale
            && ui.button("Into the Scale").on_hover_text("each note out of the scale, onto the nearest one in it").clicked()
        {
            for note in notes.iter_mut() {
                let tone = note.tone.round() as i32;
                let fits = |tone: i32| scale[tone.rem_euclid(12) as usize];
                note.tone = [0, -1, 1, -2, 2].iter().map(|by| tone + by).find(|tone| fits(*tone)).unwrap_or(tone) as f32;
            }
        }
        if ui.button("Into Chords").on_hover_text("each note becomes the chosen chord (the note at the bottom)").clicked() {
            let intervals = CHORDS[chord.min(CHORDS.len() - 1)].1;
            let mut made = Vec::new();
            for note in notes.iter() {
                made.extend(intervals.iter().map(|by| Note { tone: note.tone + by, ..*note }));
            }
            made.dedup_by(|a, b| a.start == b.start && a.tone == b.tone);
            *notes = made;
        }
        if ui.button("Arpeggiate").on_hover_text("the notes played together, one after the other, all along their length").clicked() {
            *notes = arpeggiated(notes, arpeggio, step);
        }
        if ui.button("Strum").on_hover_text("the notes of a chord a little apart, like a guitar").clicked() {
            let mut by_start: Vec<(f32, Vec<usize>)> = Vec::new();
            for (at, note) in notes.iter().enumerate() {
                match by_start.iter_mut().find(|(start, _)| (start - note.start).abs() < 0.001) {
                    Some((_, list)) => list.push(at),
                    None => by_start.push((note.start, vec![at])),
                }
            }
            for (_, mut list) in by_start {
                list.sort_by(|a, b| notes[*a].tone.total_cmp(&notes[*b].tone));
                for (rank, at) in list.into_iter().enumerate() {
                    let late = rank as f32 * strum;
                    notes[at].start += late;
                    notes[at].length = (notes[at].length - late).max(0.05);
                }
            }
        }
        if ui.button("Humanize").on_hover_text("each note a little louder or softer, a little earlier or later").clicked() {
            let mut seed = now_seed();
            let mut chance = || {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                ((seed >> 16) & 0x7FFF) as f32 / 32_767.0 * 2.0 - 1.0
            };
            for note in notes.iter_mut() {
                note.velocity = (note.velocity + chance() * 0.12).clamp(0.1, 1.0);
                note.start = (note.start + chance() * 0.02).max(0.0);
            }
        }
        if ui.button("Legato").on_hover_text("each note held until the next").clicked() {
            let mut starts: Vec<f32> = notes.iter().map(|note| note.start).collect();
            starts.sort_by(f32::total_cmp);
            starts.dedup();
            for note in notes.iter_mut() {
                if let Some(next) = starts.iter().find(|start| **start > note.start + 0.001) {
                    note.length = next - note.start;
                }
            }
        }
    });
}

fn now_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |since| since.subsec_nanos())
}

/// The notes played together, one after the other along their length: up, down, up and down, by chance.
pub(super) fn arpeggiated(notes: &[Note], pattern: usize, step: f32) -> Vec<Note> {
    let mut groups: Vec<(f32, f32, Vec<Note>)> = Vec::new();
    for note in notes {
        match groups.iter_mut().find(|(start, ..)| (start - note.start).abs() < 0.001) {
            Some((_, length, group)) => {
                *length = length.max(note.length);
                group.push(*note);
            }
            None => groups.push((note.start, note.length, vec![*note])),
        }
    }
    let mut made = Vec::new();
    let mut seed = 12_345_u32;
    for (start, length, mut group) in groups {
        group.sort_by(|a, b| a.tone.total_cmp(&b.tone));
        let order: Vec<Note> = match pattern {
            1 => group.iter().rev().copied().collect(),
            2 if group.len() > 2 => group.iter().chain(group[1..group.len() - 1].iter().rev()).copied().collect(),
            _ => group.clone(),
        };
        let count = (length / step.max(0.03)).floor().max(1.0) as usize;
        for at in 0..count {
            let picked = if pattern == 3 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                order[(seed >> 16) as usize % order.len()]
            } else {
                order[at % order.len()]
            };
            made.push(Note { start: start + at as f32 * step, length: step * 0.9, ..picked });
        }
    }
    made
}

/// The notes' forces, as bars under them: dragged up or down.
fn velocities(ui: &mut egui::Ui, notes: &mut [Note], beats: f32) {
    let wide = ui.available_width().min(900.0);
    let (rect, response) = ui.allocate_exact_size(vec2(wide, 54.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    painter.text(rect.left_center() + vec2(3.0, 0.0), egui::Align2::LEFT_CENTER, "velocity", egui::FontId::proportional(theme::SMALL - 2.0), theme::FAINT);
    let grid = Rect::from_min_max(rect.min + vec2(30.0, 3.0), rect.max - vec2(0.0, 3.0));
    let x_of = |beat: f32| grid.min.x + beat / beats.max(1.0) * grid.width();
    for note in notes.iter() {
        let x = x_of(note.start);
        let top = grid.max.y - note.velocity.clamp(0.0, 1.0) * grid.height();
        painter.vline(x + 1.0, top..=grid.max.y, Stroke::new(3.0_f32, theme::ASSETS));
        painter.circle_filled(pos2(x + 1.0, top), 2.5, Color32::WHITE);
    }
    if let Some(at) = response.interact_pointer_pos().filter(|_| response.dragged() || response.clicked()) {
        let force = ((grid.max.y - at.y) / grid.height()).clamp(0.05, 1.0);
        // (Every note under the pointer, within a few pixels.)
        for note in notes.iter_mut().filter(|note| (x_of(note.start) - at.x).abs() < 5.0) {
            note.velocity = force;
        }
    }
}

/// An audio region's sound: which, from where, how loud, how high, faded, backwards; the region as long as it.
fn clip_view(ui: &mut egui::Ui, session: &crate::session::Session, region: &mut Region, beat: f32) {
    let sounds = own_sounds(session);
    let clip = region.clip.get_or_insert_with(|| Clip::of(sounds.first().map_or("", String::as_str)));
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::text("Sound", theme::SMALL, theme::WEAK));
        egui::ComboBox::from_id_salt("clip sound").selected_text(if clip.sound.is_empty() { "(choose)" } else { clip.sound.as_str() }).width(200.0).show_ui(
            ui,
            |ui| {
                for word in &sounds {
                    ui.selectable_value(&mut clip.sound, word.clone(), word);
                }
            },
        );
        ui.checkbox(&mut clip.reverse, "Reverse");
    });
    egui::Grid::new("clip").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
        for (name, value, range, suffix) in [
            ("Start in the Sound", &mut clip.offset, 0.0..=30.0, " s"),
            ("Volume", &mut clip.gain, 0.0..=2.0, ""),
            ("Pitch", &mut clip.tone, -24.0..=24.0, " semitones"),
            ("Fade In", &mut clip.fade_in, 0.0..=4.0, " s"),
            ("Fade Out", &mut clip.fade_out, 0.0..=4.0, " s"),
        ] {
            ui.label(theme::text(name, theme::SMALL, theme::WEAK));
            ui.add(egui::Slider::new(value, range).suffix(suffix).max_decimals(2));
            ui.end_row();
        }
    });
    if ui.button("The Region as Long as the Sound").clicked()
        && let Some((left, _)) = project::sound_of(&session.own_sounds, &clip.sound)
    {
        let seconds = (left.len() as f32 / RATE - clip.offset).max(0.0) / 2f32.powf(clip.tone / 12.0);
        region.length = ((seconds / beat) * 4.0).ceil() / 4.0;
    }
}

/// A drum region as steps: each row a drum, each column a sixteenth; a click puts or takes a hit (Shift: softer),
/// dragging puts them along.
fn step_sequencer(ui: &mut egui::Ui, notes: &mut Vec<Note>, names: &[String], beats: f32, id: egui::Id) {
    let steps = (beats * 4.0).round().max(1.0) as usize;
    let rows = names.len().max(1);
    let wide = ui.available_width().min(1200.0);
    let (rect, response) = ui.allocate_exact_size(vec2(wide, rows as f32 * 22.0 + 4.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    let grid = Rect::from_min_max(rect.min + vec2(110.0, 2.0), rect.max - vec2(2.0, 2.0));
    let (cell_wide, cell_high) = (grid.width() / steps as f32, grid.height() / rows as f32);
    let cell = |row: usize, step: usize| {
        Rect::from_min_size(pos2(grid.min.x + step as f32 * cell_wide, grid.min.y + row as f32 * cell_high), vec2(cell_wide, cell_high)).shrink(1.5)
    };
    for (row, name) in names.iter().enumerate() {
        painter.text(
            pos2(rect.min.x + 6.0, grid.min.y + (row as f32 + 0.5) * cell_high),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(theme::SMALL - 1.0),
            theme::WEAK,
        );
        for step in 0..steps {
            let shade = if (step / 4) % 2 == 0 { theme::PANEL } else { theme::HOVERED };
            painter.rect_filled(cell(row, step), 2.0, shade);
        }
    }
    let at_step = |note: &Note| ((note.start * 4.0).round() as usize, note.tone.round().max(0.0) as usize);
    for note in notes.iter() {
        let (step, row) = at_step(note);
        if step < steps && row < rows {
            painter.rect_filled(cell(row, step), 2.0, theme::FILM.gamma_multiply(0.35 + 0.65 * note.velocity.clamp(0.0, 1.0)));
        }
    }
    let paint_id = id.with("paint");
    if let Some(at) = response.interact_pointer_pos().filter(|at| grid.contains(*at)) {
        let (step, row) = (((at.x - grid.min.x) / cell_wide) as usize, ((at.y - grid.min.y) / cell_high) as usize);
        let found = notes.iter().position(|note| at_step(note) == (step, row));
        let soft = ui.input(|input| input.modifiers.shift);
        if response.clicked() || response.drag_started() {
            let putting = found.is_none();
            ui.data_mut(|data| data.insert_temp(paint_id, putting));
            match found {
                Some(at) => {
                    notes.remove(at);
                }
                None => notes.push(Note { start: step as f32 / 4.0, length: 0.25, tone: row as f32, velocity: if soft { 0.55 } else { 1.0 } }),
            }
        } else if response.dragged() && ui.data(|data| data.get_temp::<bool>(paint_id)).unwrap_or(false) && found.is_none() && step < steps && row < rows {
            notes.push(Note { start: step as f32 / 4.0, length: 0.25, tone: row as f32, velocity: if soft { 0.55 } else { 1.0 } });
        }
    }
}
