//! The transport bar: play, stop, loop, tempo, written.

use eframe::egui::{self, Color32, RichText, vec2};
use shell::sound::Hears;

use super::editor::{redo, undo};
use super::{MusicMaking, files_ending, own_sounds, songs_folder};
use crate::done::Done;
use crate::theme;

/// The transport: play, stop, record (keys, microphone), the loop, where it is, the tempo, the bars, swing, the
/// click, undo; the word, written out (the song, its tracks apart, MIDI); MIDI read in.
pub(super) fn transport(ui: &mut egui::Ui, making: &mut MusicMaking, audio: &mut Option<Box<dyn Hears>>, done: &mut Option<Done>) {
    ui.horizontal_wrapped(|ui| {
        ui.add_space(6.0);
        play_buttons(ui, making, audio);
        ui.separator();
        song_settings(ui, making);
        ui.separator();
        ui.menu_button("Import", |ui| import_menu(ui, making));
        export_buttons(ui, making, audio, done);
        if let Some(said) = &making.said {
            ui.label(theme::text(said, theme::SMALL, theme::FAINT));
        }
    });
}

/// Play or stop, back to the start, record the keys or the microphone, the loop, the metronome.
fn play_buttons(ui: &mut egui::Ui, making: &mut MusicMaking, audio: &mut Option<Box<dyn Hears>>) {
    let playing = making.playing.is_some() || making.rendering.is_some();
    let said = if making.rendering.is_some() {
        "… working"
    } else if playing {
        "⏸ Stop"
    } else {
        "▶ Play"
    };
    if ui
        .add(egui::Button::new(RichText::new(said).strong().color(theme::GROUND)).fill(theme::HOME).min_size(vec2(96.0, 26.0)))
        .on_hover_text("Space")
        .clicked()
    {
        if playing {
            making.stop(audio);
        } else {
            let from = making.playhead;
            making.play_from(from);
        }
    }
    if ui.button("Start").on_hover_text("the playhead at the start (Home)").clicked() {
        making.stop(audio);
        making.playhead = if making.project.looping { making.project.loop_start } else { 0.0 };
    }
    let red = Color32::from_rgb(0xC0, 0x30, 0x30);
    let record = egui::Button::new(RichText::new("Rec.").strong().color(if making.record { Color32::WHITE } else { theme::TEXT })).fill(if making.record {
        red
    } else {
        theme::FIELD
    });
    if ui
        .add(record)
        .on_hover_text("played on the keyboard while playing: written into the chosen track (A W S E D F T G Y H U J K O L P ;: the notes; Z X: the octave)")
        .clicked()
    {
        making.record = !making.record;
    }
    let microphone = match &making.recording {
        Some((recording, _)) => format!("Mic {:.1} s", recording.seconds()),
        None => "Mic".to_string(),
    };
    let mic = egui::Button::new(RichText::new(microphone).strong().color(if making.recording.is_some() { Color32::WHITE } else { theme::TEXT }))
        .fill(if making.recording.is_some() { red } else { theme::FIELD });
    if ui.add(mic).on_hover_text("records the mic from the playhead, the song played at the same time; again: stops, the take put on an audio track").clicked()
    {
        making.microphone(audio);
    }
    ui.checkbox(&mut making.project.looping, "Loop").on_hover_text("played again from its start to its end (Shift + drag on the ruler: place it)");
    ui.checkbox(&mut making.metronome, "Metronome");
}

/// Where a beat is, as `bar.beat` (both from 1).
fn bar_and_beat(now: f32, per_bar: f32) -> String {
    format!("{:>3}.{}", (now / per_bar).floor() as i32 + 1, (now % per_bar).floor() as i32 + 1)
}

/// Where it is; the tempo, the meter, the length, the swing; the octave; undo, redo.
fn song_settings(ui: &mut egui::Ui, making: &mut MusicMaking) {
    let now = making.beat_now();
    let per_bar = making.project.beats_per_bar.max(1.0);
    ui.label(RichText::new(bar_and_beat(now, per_bar)).monospace().size(theme::TITLE).color(theme::STRONG)).on_hover_text("bar.beat");
    ui.separator();
    ui.label(theme::text("Tempo", theme::SMALL, theme::WEAK));
    ui.add(egui::DragValue::new(&mut making.project.tempo).range(20.0..=400.0).speed(0.5).suffix(" bpm"));
    ui.label(theme::text("Meter", theme::SMALL, theme::WEAK));
    ui.add(egui::DragValue::new(&mut making.project.beats_per_bar).range(1.0..=16.0).speed(0.05).suffix(" beats"));
    ui.label(theme::text("Length", theme::SMALL, theme::WEAK));
    ui.add(egui::DragValue::new(&mut making.project.bars).range(1.0..=512.0).speed(0.1).suffix(" bars"));
    ui.label(theme::text("Swing", theme::SMALL, theme::WEAK)).on_hover_text("the off-beat sixteenths, a little late: it swings");
    ui.add(egui::DragValue::new(&mut making.project.swing).range(0.0..=1.0).speed(0.01).max_decimals(2));
    ui.separator();
    ui.label(theme::text(format!("octave {:+}", making.octave), theme::SMALL, theme::WEAK)).on_hover_text("W: lower, X: higher");
    if ui.add_enabled(!making.undone.is_empty(), egui::Button::new("Undo")).on_hover_text("Ctrl+Z").clicked() {
        undo(making);
    }
    if ui.add_enabled(!making.redone.is_empty(), egui::Button::new("Redo")).on_hover_text("Ctrl+Y").clicked() {
        redo(making);
    }
}

/// A MIDI's tracks or a sound read in: picked, of the songs' folder, of one's own sounds.
fn import_menu(ui: &mut egui::Ui, making: &mut MusicMaking) {
    if ui.button("A File… (.mid, .wav)").on_hover_text("a MIDI's tracks, or a sound laid at the playhead; also: drop it on the window").clicked() {
        let chosen = crate::pick::files("A MIDI or a Sound", &[("MIDI or WAV", &["mid", "midi", "wav"])], None);
        making.take_files(chosen);
        ui.close();
    }
    ui.label(theme::text("drop a .mid or a .wav on the window, or:", theme::SMALL, theme::FAINT));
    for word in files_ending(&making.session, ".mid") {
        if ui.button(format!("{word}.mid")).clicked() {
            let path = songs_folder(&making.session).join(format!("{word}.mid"));
            making.said = Some(match std::fs::read(path).map_err(|why| why.to_string()).and_then(|bytes| making.import_midi(&bytes, &word)) {
                Ok(said) | Err(said) => said,
            });
            ui.close();
        }
    }
    ui.separator();
    ui.label(theme::text("one of your sounds, on an audio track:", theme::SMALL, theme::FAINT));
    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        for word in own_sounds(&making.session) {
            if ui.button(&word).clicked() {
                let at = making.playhead;
                making.lay_sound(&word, at);
                ui.close();
            }
        }
    });
}

/// The word, the song exported (more: its tracks apart, MIDI, the song alone), closed.
fn export_buttons(ui: &mut egui::Ui, making: &mut MusicMaking, audio: &mut Option<Box<dyn Hears>>, done: &mut Option<Done>) {
    ui.label(theme::text("Word", theme::SMALL, theme::WEAK));
    ui.add(egui::TextEdit::singleline(&mut making.word).desired_width(130.0));
    let said = if making.saved { "✔ Exported" } else { "Export" };
    if ui
        .add(egui::Button::new(RichText::new(said).strong().color(theme::GROUND)).fill(theme::ASSETS))
        .on_hover_text("the song as heard, in library/sounds/<word>.wav (in a film: music <word>), and the song itself, in library/music/<word>.music")
        .clicked()
    {
        match making.export() {
            Ok(word) => {
                making.said = Some(format!("in a film: music {word}"));
                *done = Some(Done::Sound(word));
            }
            Err(why) => making.said = Some(why),
        }
    }
    ui.menu_button("More…", |ui| {
        if ui.button("Separate Tracks").on_hover_text("each track alone, in its own file").clicked() {
            making.said = Some(match making.export_stems() {
                Ok(folder) => format!("tracks in {folder}"),
                Err(why) => why,
            });
            ui.close();
        }
        if ui.button("MIDI").on_hover_text("the notes, for other music software").clicked() {
            making.said = Some(match making.export_midi() {
                Ok(file) => format!("written: {file}"),
                Err(why) => why,
            });
            ui.close();
        }
        if ui.button("Save the Song Alone").on_hover_text("without the sound heard: to pick up later").clicked() {
            making.said = Some(match making.word().and_then(|word| making.write_song(&word).map(|()| word)) {
                Ok(word) => format!("kept: {word}.music"),
                Err(why) => why,
            });
            ui.close();
        }
    });
    if ui.button("Close").clicked() {
        making.stop(audio);
        *done = Some(Done::Closed);
    }
}

#[cfg(test)]
mod tests {
    use super::bar_and_beat;

    #[test]
    fn a_beat_is_told_as_its_bar_and_beat() {
        assert_eq!(bar_and_beat(0.0, 4.0), "  1.1");
        assert_eq!(bar_and_beat(5.5, 4.0), "  2.2");
        assert_eq!(bar_and_beat(47.0, 3.0), " 16.3");
    }
}
