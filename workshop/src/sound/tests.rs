//! The tests of the sounds: every kind and wave, a beginner's kinds and
//! words, the deepest settings, voices mixed, written and read back.

use super::presets::{EASY_KINDS, KINDS, eased, preset};
use super::views::spectrum;
use super::*;

#[test]
fn every_kind_and_wave_of_sound_makes_samples() {
    // (Every kind of sound, every wave, makes samples, none too loud.)
    for kind in KINDS {
        let samples = Synth::of_kind(kind, 42).samples();
        assert!(!samples.is_empty() && samples.iter().all(|sample| sample.abs() <= 1.0 && sample.is_finite()), "{kind}");
    }
    for (wave, name) in WAVES.iter().enumerate() {
        let synth = Synth { wave, harmonics: 2.0, ..Synth::default() };
        assert!(synth.samples().iter().all(|sample| sample.is_finite()), "{name}");
    }
    assert!(Synth::random(7).samples().iter().all(|sample| sample.is_finite()));
}

#[test]
fn a_beginner_chooses_and_moves_sounds_an_expert_bends_them() {
    // (Every kind a beginner sees makes a sound, clean.)
    for kind in EASY_KINDS.iter().flat_map(|(_, kinds)| kinds.iter()) {
        let (stereo, _) = preset(kind, 11).render(&crate::own_sounds::OwnSounds::default());
        assert!(!stereo.is_empty() && stereo.iter().all(|sample| sample.is_finite() && sample.abs() <= 1.0), "{kind}");
    }
    // (The words do what they say: higher, longer.)
    let base = preset("Jump", 4);
    let higher = eased(&base, &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    assert!(higher.voices[0].synth.pitch > base.voices[0].synth.pitch * 2.0);
    let longer = eased(&base, &[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    assert!(longer.voices[0].synth.seconds() > base.voices[0].synth.seconds() * 2.0);
    assert_eq!(eased(&base, &[0.0; 7]).voices[0].synth, base.voices[0].synth.tidy());
    // (Notes one after the other: the pitch steps; a drawn wave and FM: clean, and read back.)
    let fanfare = Synth { notes_len: 3.0, note_time: 0.1, notes: [0.0, 12.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0], ..Synth::default() };
    let (first, second) = (fanfare.pitch_at(0.05).unwrap_or(0.0), fanfare.pitch_at(0.15).unwrap_or(0.0));
    assert!((second / first - 2.0).abs() < 0.01, "{first} {second}");
    let bent = Synth {
        wave: 7,
        fm_depth: 3.0,
        fm_ratio: 2.0,
        am_depth: 0.5,
        drive: 0.7,
        drawn: std::array::from_fn(|at| if at % 4 < 2 { 0.9 } else { -0.3 }),
        ..Synth::default()
    };
    assert!(bent.samples().iter().all(|sample| sample.is_finite() && sample.abs() <= 1.0));
    let back = Sound::read(&Sound::of(bent).written());
    assert_eq!(back.voices[0].synth.drawn, bent.drawn);
    assert_eq!(back.voices[0].synth.notes, bent.notes);
    assert!((back.voices[0].synth.fm_depth - 3.0).abs() < 1e-6);
}

#[test]
fn the_deepest_sounds_are_made_clean_and_written_back() {
    let clean = |samples: &[f32], what: &str| assert!(!samples.is_empty() && samples.iter().all(|sample| sample.is_finite() && sample.abs() <= 1.0), "{what}");
    // (Every route, from every source, every filter kind and vowel, the string, folds, rings, chorus, phaser.)
    for target in 1..MOD_TARGETS.len() {
        for source in 1..MOD_SOURCES.len() {
            let mut mods = [[0.0; 3]; 6];
            mods[0] = [source as f32, target as f32, 1.5];
            let synth =
                Synth { mods, curve: std::array::from_fn(|at| (at as f32 / 8.0).sin()), formant: 2.5, lfo1_shape: (target % 5) as f32, ..Synth::default() };
            clean(&synth.samples(), &format!("route {source} → {target}"));
        }
    }
    for (mode, name) in FILTER_MODES.iter().enumerate() {
        let synth = Synth { filter_mode: mode as f32, lowpass: 900.0, resonance: 0.8, wave: 1, ..Synth::default() };
        clean(&synth.samples(), name);
    }
    let string = Synth { wave: 8, pitch: 220.0, sustain: 0.5, ..Synth::default() };
    let plucked = string.samples();
    clean(&plucked, "string");
    let loud = |part: &[f32]| part.iter().map(|sample| sample.abs()).fold(0.0_f32, f32::max);
    assert!(loud(&plucked[..2000]) > loud(&plucked[plucked.len() - 2000..]), "the string dies away");
    let coloured = Synth { fold: 0.7, ring_mix: 0.5, chorus: 0.8, phaser: 0.8, wave: 2, ..Synth::default() };
    clean(&coloured.samples(), "fold, ring, chorus, phaser");
    // (A sample not found is silence, not a fault.)
    clean(&Synth { wave: 9, ..Synth::default() }.samples(), "sample with no data");
    // (A unison wide, a score, another voice bending one, the mix's all.)
    let mut lead =
        Voice::of("lead", Synth { wave: 1, unison: 7.0, detune: 30.0, spread: 1.0, from_voice: 2.0, from_fm: 3.0, from_ring: 0.3, ..Synth::default() });
    lead.notes = vec![Note { start: 0.0, length: 1.0, tone: 0.0, velocity: 1.0 }, Note { start: 2.0, length: 0.5, tone: 7.0, velocity: 0.6 }];
    let sound = Sound {
        voices: vec![lead, Voice::of("modulator", Synth { wave: 2, pitch: 330.0, ..Synth::default() })],
        mix: Mix { tempo: 120.0, low: 6.0, middle: -3.0, high: 4.0, compress: 0.6, width: 1.6, ping_pong: true, echo_mix: 0.3, limit: true, ..Mix::default() },
    };
    let (stereo, _) = sound.render(&crate::own_sounds::OwnSounds::default());
    clean(&stereo, "the whole");
    assert!(stereo.len() / 2 >= (1.25 * shell::audio::OUTPUT_RATE as f32) as usize, "the score lasts its two and a half beats (1.25 s at 120)");
    assert!(stereo.chunks(2).any(|pair| (pair[0] - pair[1]).abs() > 0.01), "the unison spreads from left to right");
    // (Written, read back the same: score, sample, routes, curve, mix.)
    let mut written = sound.clone();
    written.voices[1].sample = "test_beep".to_string();
    written.voices[0].synth.mods[2] = [1.0, 2.0, -0.75];
    written.voices[0].synth.curve[5] = 0.5;
    assert_eq!(Sound::read(&written.written()), written);
}

#[test]
fn a_sound_of_voices_is_mixed_written_and_read_back() {
    let mut sound = Sound::of(Synth::of_kind("Jump", 3));
    let mut second = Voice::of("bass", Synth::of_kind("Hit", 5));
    (second.offset, second.pan, second.gain) = (0.1, -0.5, 0.8);
    sound.voices.push(second);
    sound.mix = Mix { echo_mix: 0.4, room_mix: 0.3, fade_out: 0.05, normalize: true, ..Mix::default() };
    let (stereo, measure) = sound.render(&crate::own_sounds::OwnSounds::default());
    assert!(stereo.len() > 2 && stereo.iter().all(|sample| sample.is_finite() && sample.abs() <= 1.0));
    assert!(measure.peak < 0.0 && measure.peak > -2.0, "normalised to -1 dB: {}", measure.peak);
    let back = Sound::read(&sound.written());
    assert_eq!(back.voices.len(), 2);
    assert_eq!(back.voices[1].name, "bass");
    assert!((back.voices[1].offset - 0.1).abs() < 1e-6 && (back.mix.echo_mix - 0.4).abs() < 1e-6 && back.mix.normalize);
    assert!((back.voices[0].synth.pitch - sound.voices[0].synth.pitch).abs() < 0.01);
    // (What the first maker wrote: one voice, from a pitch to another.)
    let older = Sound::read("wave=2\nstart=440\nend=880\nduration=0.5\ndecay=0.1\n");
    assert_eq!(older.voices.len(), 1);
    assert!((older.voices[0].synth.slide - 2.0).abs() < 0.01, "{}", older.voices[0].synth.slide);
    // (A spectrum: a high note is high.)
    let high = Synth { wave: 2, pitch: 4000.0, sustain: 0.2, decay: 0.0, ..Synth::default() }.samples();
    let levels = spectrum(&high, 4, 16);
    // (Rows from 60 Hz (the last) to 12 kHz (the first): 4 kHz is the 12th from below, the 4th from above.)
    let near_the_note: f32 = levels[3 * 4..4 * 4].iter().sum();
    let lowest: f32 = levels[15 * 4..].iter().sum();
    assert!(near_the_note > lowest + 1.0, "{near_the_note} {lowest}");
}

/// The expert's editor works out: a value dragged (taken, let go), the drawn wave's models and smoothing, a score made of the notes and doubled, a layer's name.
#[test]
fn the_expert_editor_works_out() {
    use super::editor::layer_said;
    use super::settings::{dragged_pair, saw_drawn, score_doubled, score_of_notes, smoothed, square_drawn};
    assert_eq!(dragged_pair(true, false, true, true), (true, false), "a drag begun: taken");
    assert_eq!(dragged_pair(false, true, false, false), (false, true), "a drag stopped: let go");
    assert_eq!(dragged_pair(false, false, true, false), (true, true), "typed: both at once");
    assert_eq!(dragged_pair(false, false, true, true), (false, false), "moving: neither");
    assert_eq!((square_drawn()[0], square_drawn()[31]), (0.8, -0.8));
    assert_eq!((saw_drawn()[0], saw_drawn()[31]), (1.0, -1.0));
    let mut spike = [0.0_f32; 32];
    spike[0] = 4.0;
    let soft = smoothed(spike);
    assert_eq!((soft[31], soft[0], soft[1], soft[2]), (1.0, 2.0, 1.0, 0.0), "toward its neighbours, going round");
    let mut synth = Synth::of_kind("Blip", 42);
    (synth.notes_len, synth.note_time) = (3.0, 0.25);
    let mut notes = score_of_notes(&synth, 120.0);
    assert_eq!(
        notes.iter().map(|note| (note.start, note.length, note.tone)).collect::<Vec<_>>(),
        vec![(0.0, 0.5, synth.notes[0]), (0.5, 0.5, synth.notes[1]), (1.0, 0.5, synth.notes[2])]
    );
    score_doubled(&mut notes);
    assert_eq!(notes.len(), 6);
    assert_eq!(notes[3].start, 2.0, "again from the beat after its end");
    let mut voice = Voice::of("Bell", synth);
    voice.muted = true;
    assert_eq!(layer_said(1, &voice), "2  Bell (muted)");
}

/// A wave read at a part of its period, a pitch bent by its tuning.
#[test]
fn a_wave_is_read_and_its_pitch_bent() {
    let square = Synth { wave: 0, ..Synth::default() };
    assert_eq!((square.tone(0.2, 0.5), square.tone(0.7, 0.5)), (0.5, -0.5));
    let saw = Synth { wave: 1, ..Synth::default() };
    assert_eq!(saw.tone(0.25, 0.5), 0.5);
    let still = Synth { vibrato: 0.0, ..Synth::default() };
    assert!((still.bent_pitch(440.0, 12.0, 0.0, 44100.0) - 880.0).abs() < 1e-3, "an octave up");
    assert_eq!(still.bent_pitch(1.0, 0.0, 0.0, 44100.0), 10.0, "no lower than 10");
}
