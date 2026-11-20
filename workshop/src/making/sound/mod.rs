//! Sounds made as the best sound makers of games make theirs, then mixed:
//! the synth's settings and names, a voice, a sound of voices, its mix, its
//! measures, and the sound being made.

use std::time::Instant;

use eframe::egui;

mod controls;
mod editor;
mod instruments;
mod mix;
mod presets;
mod render;
mod settings;
mod simple;
mod synth;
#[cfg(test)]
mod tests;
mod versions;
mod views;
mod voices;

pub(crate) use controls::piano_roll;
pub use editor::make_sound;
pub(crate) use instruments::{INSTRUMENTS, drum_kit, instrument};
pub(crate) use mix::{mix_editor, mix_read, mix_written, mixed, note_audio, one_shot};
pub(crate) use render::{Biquad, OwnSounds, sample_of};
pub(crate) use versions::open_audio;

/// The waves a sound is made of.
const WAVES: [&str; 10] = ["Square", "Saw", "Sine", "Triangle", "Noise", "Pink noise", "8-bit noise", "Drawn", "String", "Sample"];

/// The filter's kinds; the vowels it may speak; what moves a setting, what it moves; the LFOs' shapes.
const FILTER_MODES: [&str; 5] = ["Low-pass", "Band-pass", "Notch", "Peak", "High-pass"];
const VOWELS: [&str; 6] = ["—", "a", "e", "i", "o", "oo"];
const VOWEL_FORMANTS: [[f32; 3]; 5] =
    [[730.0, 1090.0, 2440.0], [530.0, 1840.0, 2480.0], [270.0, 2290.0, 3010.0], [570.0, 840.0, 2410.0], [300.0, 870.0, 2240.0]];
const MOD_SOURCES: [&str; 6] = ["—", "LFO 1", "LFO 2", "Envelope 2", "Curve", "Random"];
const MOD_TARGETS: [(&str, &str); 12] = [
    ("—", ""),
    ("Pitch", "in semitones"),
    ("Cutoff", "in octaves"),
    ("Volume", "1: twice as loud"),
    ("Pan", "-1 left, 1 right"),
    ("FM", "the depth added"),
    ("Duty", "added to the square's duty cycle"),
    ("Resonance", "added"),
    ("Drive", "added"),
    ("Vowel", "1: one vowel further"),
    ("Ring", "in octaves"),
    ("Fold", "added"),
];
const LFO_SHAPES: [&str; 5] = ["Sine", "Triangle", "Square", "Saw", "Random"];
const MOD_KEYS: [[&str; 3]; 6] = [
    ["mod1_source", "mod1_target", "mod1_amount"],
    ["mod2_source", "mod2_target", "mod2_amount"],
    ["mod3_source", "mod3_target", "mod3_amount"],
    ["mod4_source", "mod4_target", "mod4_amount"],
    ["mod5_source", "mod5_target", "mod5_amount"],
    ["mod6_source", "mod6_target", "mod6_amount"],
];

/// The groups of settings, each locked or not (chance and mutation leave a locked one as it is).
const GROUPS: [&str; 6] = ["Envelope", "Pitch", "Vibrato and Arpeggio", "Wave", "Filters", "Layer Effects"];

/// What a voice is made of, as the old games' sound makers had it (and Bfxr's harmonics).
#[derive(Clone, Copy, PartialEq, Debug)]
struct Synth {
    wave: usize,
    harmonics: f32,
    duty: f32,
    duty_sweep: f32,
    attack: f32,
    sustain: f32,
    punch: f32,
    decay: f32,
    pitch: f32,
    min_pitch: f32,
    slide: f32,
    delta_slide: f32,
    vibrato: f32,
    vibrato_hz: f32,
    arp_mult: f32,
    arp_time: f32,
    repeat: f32,
    lowpass: f32,
    lowpass_sweep: f32,
    resonance: f32,
    highpass: f32,
    highpass_sweep: f32,
    flanger: f32,
    flanger_sweep: f32,
    crush: f32,
    volume: f32,
    /// FM: a second wave bending the first (how much faster it goes, how deep); AM: its loudness trembling.
    fm_ratio: f32,
    fm_depth: f32,
    am_hz: f32,
    am_depth: f32,
    /// How hard it is pushed (a warm distortion).
    drive: f32,
    /// Notes played one after the other (in semitones over its pitch): how many, each how long.
    notes_len: f32,
    note_time: f32,
    notes: [f32; 8],
    /// A wave drawn by hand: one of its periods, in 32 steps.
    drawn: [f32; 32],
    /// The filter's kind (`FILTER_MODES`), its gain for a peak (dB); a vowel spoken (1 to 5, between them too) and how much.
    filter_mode: f32,
    filter_gain: f32,
    formant: f32,
    formant_mix: f32,
    /// The plucked string's (the "String" wave): how fast it dies, how bright.
    string_damp: f32,
    string_bright: f32,
    /// A sample (the "Sample" wave): where it starts (s), looped or not.
    sample_start: f32,
    sample_loop: f32,
    /// So many of it at once, out of tune by so many cents, spread so wide.
    unison: f32,
    detune: f32,
    spread: f32,
    /// Ring modulation (Hz, how much); chorus, phaser (how much, how fast); the wave folded over itself.
    ring_hz: f32,
    ring_mix: f32,
    chorus: f32,
    chorus_hz: f32,
    phaser: f32,
    phaser_hz: f32,
    fold: f32,
    /// What moves the settings: two LFOs (shape, Hz), a second envelope (attack, decay, level held),
    /// a curve drawn along the sound, six routes (source, target, amount).
    lfo1_shape: f32,
    lfo1_hz: f32,
    lfo2_shape: f32,
    lfo2_hz: f32,
    menv_attack: f32,
    menv_decay: f32,
    menv_sustain: f32,
    mods: [[f32; 3]; 6],
    curve: [f32; 32],
    /// Another voice bending this one (its number from 1; 0: none): by FM, by a ring.
    from_voice: f32,
    from_fm: f32,
    from_ring: f32,
    seed: u32,
}

/// A drawn wave to begin with: a sine.
fn sine_drawn() -> [f32; 32] {
    std::array::from_fn(|at| (std::f32::consts::TAU * at as f32 / 32.0).sin())
}

impl Default for Synth {
    fn default() -> Synth {
        Synth {
            wave: 0,
            harmonics: 0.0,
            duty: 0.5,
            duty_sweep: 0.0,
            attack: 0.0,
            sustain: 0.08,
            punch: 0.0,
            decay: 0.08,
            pitch: 880.0,
            min_pitch: 0.0,
            slide: 0.0,
            delta_slide: 0.0,
            vibrato: 0.0,
            vibrato_hz: 6.0,
            arp_mult: 1.0,
            arp_time: 0.0,
            repeat: 0.0,
            lowpass: 16000.0,
            lowpass_sweep: 0.0,
            resonance: 0.0,
            highpass: 0.0,
            highpass_sweep: 0.0,
            flanger: 0.0,
            flanger_sweep: 0.0,
            crush: 0.0,
            volume: 0.6,
            fm_ratio: 2.0,
            fm_depth: 0.0,
            am_hz: 8.0,
            am_depth: 0.0,
            drive: 0.0,
            notes_len: 0.0,
            note_time: 0.1,
            notes: [0.0, 4.0, 7.0, 12.0, 7.0, 4.0, 0.0, -5.0],
            drawn: sine_drawn(),
            filter_mode: 0.0,
            filter_gain: 6.0,
            formant: 0.0,
            formant_mix: 1.0,
            string_damp: 0.2,
            string_bright: 0.5,
            sample_start: 0.0,
            sample_loop: 0.0,
            unison: 1.0,
            detune: 15.0,
            spread: 0.5,
            ring_hz: 30.0,
            ring_mix: 0.0,
            chorus: 0.0,
            chorus_hz: 0.8,
            phaser: 0.0,
            phaser_hz: 0.5,
            fold: 0.0,
            lfo1_shape: 0.0,
            lfo1_hz: 4.0,
            lfo2_shape: 1.0,
            lfo2_hz: 0.5,
            menv_attack: 0.0,
            menv_decay: 0.3,
            menv_sustain: 0.0,
            mods: [[0.0; 3]; 6],
            curve: [0.0; 32],
            from_voice: 0.0,
            from_fm: 0.0,
            from_ring: 0.0,
            seed: 1,
        }
    }
}

/// A note of a voice's score: when (in beats), how long (beats), how many half tones over its pitch, how hard.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Note {
    pub(crate) start: f32,
    pub(crate) length: f32,
    pub(crate) tone: f32,
    pub(crate) velocity: f32,
}

/// A voice of a sound: its synth, when it comes in, how loud, from which side, muted or alone.
#[derive(Clone, PartialEq, Debug)]
struct Voice {
    name: String,
    synth: Synth,
    offset: f32,
    gain: f32,
    pan: f32,
    muted: bool,
    solo: bool,
    /// The own sound it reads (the "Sample" wave); its score (none: played once, as it is).
    sample: String,
    notes: Vec<Note>,
}

/// What is done to the voices mixed: an echo, a room, faster or slower, backwards, coming in and going out, made as loud as it may.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Mix {
    pub(crate) echo: f32,
    pub(crate) feedback: f32,
    pub(crate) echo_mix: f32,
    pub(crate) room: f32,
    pub(crate) room_mix: f32,
    pub(crate) speed: f32,
    pub(crate) fade_in: f32,
    pub(crate) fade_out: f32,
    pub(crate) reverse: bool,
    pub(crate) normalize: bool,
    /// The scores' beats a minute; low, middle, high (dB); compressed (0 to 1); how wide (1: as it is);
    /// the echo from one side to the other; nothing ever past -1 dB.
    pub(crate) tempo: f32,
    pub(crate) low: f32,
    pub(crate) middle: f32,
    pub(crate) high: f32,
    pub(crate) compress: f32,
    pub(crate) width: f32,
    pub(crate) ping_pong: bool,
    pub(crate) limit: bool,
}

impl Default for Mix {
    fn default() -> Mix {
        Mix {
            echo: 0.15,
            feedback: 0.35,
            echo_mix: 0.0,
            room: 0.5,
            room_mix: 0.0,
            speed: 1.0,
            fade_in: 0.0,
            fade_out: 0.0,
            reverse: false,
            normalize: false,
            tempo: 120.0,
            low: 0.0,
            middle: 0.0,
            high: 0.0,
            compress: 0.0,
            width: 1.0,
            ping_pong: false,
            limit: false,
        }
    }
}

/// How loud a sound is: its peak and its mean (in dB under the most), how many samples were clipped.
#[derive(Clone, Copy, Default)]
pub(crate) struct Measure {
    peak: f32,
    mean: f32,
    clipped: usize,
}

/// A sound: its voices, mixed so.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Sound {
    voices: Vec<Voice>,
    mix: Mix,
}

/// A sound being made: what it is made of, its versions, how it is heard and seen.
pub struct SoundMaking {
    /// What the Workshop works with (`session`).
    pub session: std::sync::Arc<crate::session::Session>,
    pub word: String,
    sound: Sound,
    voice: usize,
    undone: Vec<Sound>,
    redone: Vec<Sound>,
    versions: Vec<(String, Sound)>,
    played: Option<(Instant, f32)>,
    /// What it was when a setting (or a handle) was taken: to know, when let go, whether it moved.
    taken: Option<Sound>,
    handle: Option<usize>,
    looped: bool,
    play_on_change: bool,
    locks: [bool; 6],
    variants: u32,
    reference: usize,
    rendered: Option<(Sound, Vec<f32>, Measure)>,
    picture: Option<(Sound, egui::TextureHandle)>,
    pub(super) saved: bool,
    said: Option<String>,
    /// Made as an expert, or simply; simply: the sound chosen, and the words moving it (and theirs to undo).
    expert: bool,
    base: Sound,
    kind: String,
    words: [f32; 7],
    easy_undone: Vec<(Sound, String, [f32; 7])>,
    /// The sound written as text, to be changed so (an expert's).
    text: Option<String>,
    /// Inside another maker (a song's instrument): nothing written, nothing closed.
    pub(crate) embedded: bool,
}
