//! The computer's keys played as a piano.

use eframe::egui::{self, Rect};
use shell::sound::Hears;

use super::editor::{new_link, redo, undo};
use super::project::{Kind, Region};
use super::wave::pcm;
use super::{MusicMaking, PIANO_KEYS, Tab};
use crate::sound::{self, Note, open_audio};

/// The keyboard played as a piano (or as drums), heard at once; recorded while it plays; the shortcuts.
pub(super) fn played_keys(ui: &mut egui::Ui, making: &mut MusicMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32, editor: Rect) {
    if ui.ctx().wants_keyboard_input() {
        return;
    }
    let in_editor = ui.rect_contains_pointer(editor) && making.tab == Tab::Instrument;
    let (space, home, undo_asked, redo_asked, delete, duplicate, lower, higher) = ui.input_mut(|input| {
        (
            input.key_pressed(egui::Key::Space),
            input.key_pressed(egui::Key::Home),
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z),
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y),
            input.key_pressed(egui::Key::Delete),
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::D),
            input.key_pressed(egui::Key::Z),
            input.key_pressed(egui::Key::X),
        )
    });
    if space && !in_editor {
        if making.playing.is_some() || making.rendering.is_some() {
            making.stop(audio);
        } else {
            let from = making.playhead;
            making.play_from(from);
        }
    }
    if home {
        making.stop(audio);
        making.playhead = 0.0;
    }
    if !in_editor && undo_asked {
        undo(making);
    }
    if !in_editor && redo_asked {
        redo(making);
    }
    let place = making.track;
    let chosen = making.region.filter(|number| making.project.tracks.get(place).is_some_and(|track| *number < track.regions.len()));
    if delete
        && !in_editor
        && let Some(number) = chosen
    {
        making.project.tracks[place].regions.remove(number);
        making.region = None;
    }
    if duplicate && let Some(number) = chosen {
        let link = match making.project.tracks[place].regions[number].link {
            0 => new_link(&making.project.tracks[place]),
            link => link,
        };
        making.project.tracks[place].regions[number].link = link;
        let region = &making.project.tracks[place].regions[number];
        let copy = Region { start: region.start + region.length, ..region.clone() };
        making.project.tracks[place].regions.push(copy);
        making.region = Some(making.project.tracks[place].regions.len() - 1);
    }
    if lower {
        making.octave = (making.octave - 1).max(-3);
    }
    if higher {
        making.octave = (making.octave + 1).min(3);
    }
    if in_editor {
        return;
    }
    let Some(track) = making.project.tracks.get(place) else { return };
    if track.kind == Kind::Audio {
        return;
    }
    let (kind, octave) = (track.kind, making.octave as f32 * 12.0);
    for (key, tone) in PIANO_KEYS {
        let (pressed, released) = ui.input(|input| (input.key_pressed(key) && !input.modifiers.command, input.key_released(key)));
        if pressed && !making.held.contains_key(&key) {
            let tone = match kind {
                Kind::Drums => tone as f32,
                _ => tone as f32 + octave,
            };
            let track = &making.project.tracks[place];
            let heard = match kind {
                Kind::Drums => track.kit.get(tone as usize).map(|(_, sound)| sound::one_shot(sound, 1.0, &making.session.own_sounds)),
                _ => Some(sound::note_audio(&track.instrument, tone, 0.45, 0.9, &making.session.own_sounds)),
            };
            if let Some((left, right)) = heard {
                open_audio(&making.session, audio);
                if let Some(audio) = audio.as_mut() {
                    audio.samples(pcm(&left, &right), volume);
                }
            }
            making.held.insert(key, (tone, making.beat_now()));
        }
        if released && let Some((tone, start)) = making.held.remove(&key) {
            // (Recording while it plays: the note written where it was played, in the region under it, or a new one.)
            if making.record && making.playing.is_some() {
                let end = making.beat_now();
                let per_bar = making.project.beats_per_bar.max(1.0);
                let track = &mut making.project.tracks[place];
                let found = track.regions.iter().position(|region| start >= region.start && start < region.start + region.length);
                let number = match found {
                    Some(found) => found,
                    None => {
                        track.regions.push(Region::empty((start / per_bar).floor() * per_bar, per_bar * 4.0));
                        track.regions.len() - 1
                    }
                };
                let region = &mut track.regions[number];
                let at = ((start - region.start) * 4.0).round() / 4.0;
                let length = if kind == Kind::Drums { 0.25 } else { (((end - start) * 4.0).round() / 4.0).max(0.25) };
                region.notes.push(Note { start: at, length, tone, velocity: 0.9 });
                making.region = Some(number);
            }
        }
    }
    if !making.held.is_empty() {
        ui.ctx().request_repaint();
    }
}
