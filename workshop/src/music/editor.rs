//! The music's editor laid out: its colours, its scale, its links, undone
//! and redone.

use std::time::Instant;

use eframe::egui::{self, Color32, Rect, Sense, pos2, vec2};
use shell::sound::Hears;

use super::keys::played_keys;
use super::notes::editor;
use super::project::Track;
use super::tracks::{arrangement, track_headers};
use super::transport::transport;
use super::wave::pcm;
use super::{HEADERS, MusicMaking, Playing, SCALES};
use workshop_base::done::Done;
use workshop_sound::open_audio;

pub(super) fn colour(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// The scale chosen, as twelve half tones from Do lit or not (none: no scale).
pub(super) fn scale_of(making: &MusicMaking) -> Option<[bool; 12]> {
    let (_, steps) = SCALES[making.scale.min(SCALES.len() - 1)];
    (!steps.is_empty()).then(|| std::array::from_fn(|semitone| steps.contains(&((semitone + 12 - making.tonic % 12) % 12))))
}

/// The music maker.
pub fn make_music(ui: &mut egui::Ui, making: &mut MusicMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) -> Option<Done> {
    let mut done = None;
    let before = making.project.clone();
    making.track = making.track.min(making.project.tracks.len().saturating_sub(1));
    making.take_dropped(ui.ctx());
    // (The song worked out aside: heard now.)
    if let Some((receiver, from, to)) = &making.rendering
        && let Ok((played, caches)) = receiver.try_recv()
    {
        let (from, to) = (*from, *to);
        making.caches = caches;
        making.peaks = played.peaks.clone();
        open_audio(&making.session, audio);
        if let Some(audio) = audio.as_mut() {
            audio.play_held(pcm(&played.left, &played.right), volume);
            making.playing = Some(Playing { since: Instant::now(), from, to });
        } else {
            making.said = Some("no sound: the sound card is missing".to_string());
        }
        making.rendering = None;
    }
    if making.rendering.is_some() || making.playing.is_some() || making.recording.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }
    // (At its end: again from the loop's start, or stopped there.)
    if let Some(playing) = &making.playing {
        let now = making.beat_now();
        if now >= playing.to {
            let looping = making.project.looping;
            making.stop(audio);
            if looping {
                let start = making.project.loop_start;
                making.play_from(start);
            } else {
                making.playhead = 0.0;
            }
        }
    }
    transport(ui, making, audio, &mut done);
    let room = ui.available_rect_before_wrap();
    let high = (room.height() * 0.4).clamp(170.0, 420.0);
    let top = Rect::from_min_size(room.min, vec2(room.width(), high));
    let bottom = Rect::from_min_max(pos2(room.min.x, top.max.y + 6.0), room.max);
    let headers = Rect::from_min_size(top.min, vec2(HEADERS, top.height()));
    let timeline = Rect::from_min_max(pos2(headers.max.x + 4.0, top.min.y), top.max);
    track_headers(ui, making, headers, audio, volume);
    arrangement(ui, making, timeline);
    let editor_rect = editor(ui, making, bottom, audio, volume);
    played_keys(ui, making, audio, volume, editor_rect);
    sync_links(making);
    // (Changed while taken: one step to undo, once let go; and heard again from where it is, if it plays.)
    if making.project != before {
        making.saved = false;
        if making.taken.is_none() {
            making.taken = Some(before);
        }
    }
    if !ui.input(|input| input.pointer.any_down())
        && let Some(was) = making.taken.take()
        && was != making.project
    {
        making.undone.push(was);
        making.redone.clear();
        if making.undone.len() > 100 {
            making.undone.remove(0);
        }
        if making.playing.is_some() && making.recording.is_none() {
            let now = making.beat_now();
            making.stop(audio);
            making.play_from(now);
        }
    }
    ui.allocate_rect(room, Sense::hover());
    done
}

/// The notes of the region chosen, given to the regions linked to it (the same pattern).
fn sync_links(making: &mut MusicMaking) {
    let Some(track) = making.project.tracks.get_mut(making.track) else { return };
    let Some(chosen) = making.region.and_then(|number| track.regions.get(number)).filter(|region| region.link != 0) else { return };
    let (link, notes, number) = (chosen.link, chosen.notes.clone(), making.region.unwrap_or(0));
    for (at, region) in track.regions.iter_mut().enumerate() {
        if at != number && region.link == link && region.notes != notes {
            region.notes = notes.clone();
        }
    }
}

/// A new link's number on a track.
pub(super) fn new_link(track: &Track) -> u32 {
    track.regions.iter().map(|region| region.link).max().unwrap_or(0) + 1
}

pub(super) fn undo(making: &mut MusicMaking) {
    if let Some(was) = making.undone.pop() {
        making.redone.push(std::mem::replace(&mut making.project, was));
        making.editor = None;
    }
}

pub(super) fn redo(making: &mut MusicMaking) {
    if let Some(next) = making.redone.pop() {
        making.undone.push(std::mem::replace(&mut making.project, next));
        making.editor = None;
    }
}
