//! A voice and a sound of voices: rendered, written and read back.

use std::collections::HashMap;

use super::mix::{mix_fields, mixed};
use super::render::Context;
use super::{Measure, Mix, Note, Sound, Synth, Voice};
use workshop_base::own_sounds::{OwnSounds, sample_of};

impl Voice {
    pub(super) fn of(name: &str, synth: Synth) -> Voice {
        Voice { name: name.to_string(), synth, offset: 0.0, gain: 1.0, pan: 0.0, muted: false, solo: false, sample: String::new(), notes: Vec::new() }
    }

    /// Its two channels from its start: each note of its score (or once), each copy of its unison,
    /// another voice bending it, its sample read.
    fn stereo(&self, place: usize, sound: &Sound, own: &OwnSounds) -> (Vec<f32>, Vec<f32>) {
        let rate = shell::audio::OUTPUT_RATE as f32;
        let sample = (self.synth.wave == 9 && !self.sample.is_empty()).then(|| sample_of(own, &self.sample)).flatten();
        let from = self.synth.from_voice.round() as usize;
        let modulator = (from >= 1 && from - 1 != place).then(|| sound.voices.get(from - 1)).flatten().map(|other| other.synth.samples());
        let beat = 60.0 / sound.mix.tempo.max(1.0);
        let played: Vec<(f32, Synth, f32)> = if self.notes.is_empty() {
            vec![(0.0, self.synth, 1.0)]
        } else {
            self.notes
                .iter()
                .map(|note| {
                    let mut synth = self.synth;
                    synth.pitch *= 2f32.powf(note.tone / 12.0);
                    synth.sustain = (note.length * beat - synth.attack).max(0.0);
                    (note.start.max(0.0) * beat, synth, note.velocity.max(0.0))
                })
                .collect()
        };
        let copies = self.synth.unison.round().clamp(1.0, 16.0) as usize;
        let (mut left, mut right) = (Vec::new(), Vec::new());
        for (start, synth, velocity) in played {
            for copy in 0..copies {
                let across = if copies > 1 { copy as f32 / (copies - 1) as f32 * 2.0 - 1.0 } else { 0.0 };
                let context = Context {
                    detune: across * self.synth.detune / 100.0,
                    phase: (copy as f32 * 0.618).fract() * f32::from(u8::from(copies > 1)),
                    seed: copy as u32 * 7919,
                    modulator: modulator.as_deref(),
                    sample: sample.as_deref().map(Vec::as_slice),
                };
                let (mono, pans) = synth.samples_with(&context);
                let gain = velocity / (copies as f32).sqrt();
                let first = (start * rate) as usize;
                if left.len() < first + mono.len() {
                    left.resize(first + mono.len(), 0.0);
                    right.resize(first + mono.len(), 0.0);
                }
                for (at, value) in mono.iter().enumerate() {
                    let pan = (across * self.synth.spread + pans.get(at).copied().unwrap_or(0.0)).clamp(-1.0, 1.0);
                    left[first + at] += value * gain * (1.0 - pan).min(1.0);
                    right[first + at] += value * gain * (1.0 + pan).min(1.0);
                }
            }
        }
        (left, right)
    }
}

impl Sound {
    pub(super) fn of(synth: Synth) -> Sound {
        Sound { voices: vec![Voice::of("Layer 1", synth)], mix: Mix::default() }
    }

    /// Its samples, two channels (left, right, one after the other), and how loud it is.
    pub(crate) fn render(&self, own: &OwnSounds) -> (Vec<f32>, Measure) {
        let rate = shell::audio::OUTPUT_RATE as f32;
        let alone = self.voices.iter().any(|voice| voice.solo);
        type Part = (usize, (Vec<f32>, Vec<f32>), f32, f32);
        let parts: Vec<Part> = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| !voice.muted && (!alone || voice.solo))
            .map(|(place, voice)| ((voice.offset.max(0.0) * rate) as usize, voice.stereo(place, self, own), voice.gain, voice.pan))
            .collect();
        let length = parts.iter().map(|(start, (samples, _), ..)| start + samples.len()).max().unwrap_or(0);
        let (mut left, mut right) = (vec![0.0_f32; length], vec![0.0_f32; length]);
        for (start, (own_left, own_right), gain, pan) in &parts {
            let (to_left, to_right) = ((1.0 - pan).min(1.0) * gain, (1.0 + pan).min(1.0) * gain);
            for (at, (l, r)) in own_left.iter().zip(own_right).enumerate() {
                left[start + at] += l * to_left;
                right[start + at] += r * to_right;
            }
        }
        let (left, right) = mixed(&self.mix, left, right);
        let length = left.len();
        let clipped = left.iter().chain(&right).filter(|sample| sample.abs() > 1.0).count();
        let peak = left.iter().chain(&right).fold(0.0_f32, |most, sample| most.max(sample.abs()));
        let mean = if length > 0 { (left.iter().chain(&right).map(|sample| sample * sample).sum::<f32>() / (2 * length) as f32).sqrt() } else { 0.0 };
        let decibels = |level: f32| if level > 1e-6 { 20.0 * level.log10() } else { -96.0 };
        let stereo: Vec<f32> = left.iter().zip(&right).flat_map(|(l, r)| [l.clamp(-1.0, 1.0), r.clamp(-1.0, 1.0)]).collect();
        (stereo, Measure { peak: decibels(peak), mean: decibels(mean), clipped })
    }

    /// Written as `key=value` lines: its mix, then each voice under its number.
    pub fn written(&self) -> String {
        let mut mix = self.mix;
        let mut said = format!("layers={}\n", self.voices.len());
        for (key, value) in mix_fields(&mut mix) {
            said.push_str(&format!("{key}={value}\n"));
        }
        said.push_str(&format!(
            "mix.reverse={}\nmix.normalize={}\nmix.pingpong={}\nmix.limiter={}\n",
            u8::from(mix.reverse),
            u8::from(mix.normalize),
            u8::from(mix.ping_pong),
            u8::from(mix.limit)
        ));
        for (at, voice) in self.voices.iter().enumerate() {
            let prefix = format!("layer{at}.");
            said.push_str(&format!(
                "{prefix}name={}\n{prefix}layer_start={}\n{prefix}gain={}\n{prefix}pan={}\n{prefix}muted={}\n{prefix}solo={}\n",
                voice.name.replace('\n', " "),
                voice.offset,
                voice.gain,
                voice.pan,
                u8::from(voice.muted),
                u8::from(voice.solo)
            ));
            said.push_str(&voice.synth.written(&prefix));
            if !voice.sample.is_empty() {
                said.push_str(&format!("{prefix}sample={}\n", voice.sample));
            }
            if !voice.notes.is_empty() {
                let score: Vec<String> = voice.notes.iter().map(|note| format!("{}:{}:{}:{}", note.start, note.length, note.tone, note.velocity)).collect();
                said.push_str(&format!("{prefix}score={}\n", score.join(";")));
            }
        }
        said
    }

    /// Read back (what a maker of one voice wrote, too).
    pub fn read(text: &str) -> Sound {
        let mut pairs: HashMap<String, String> = HashMap::new();
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                pairs.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        let number = |key: &str| pairs.get(key).and_then(|value| value.parse::<f32>().ok());
        let Some(count) = number("layers") else {
            let flat: HashMap<String, f32> = pairs.iter().filter_map(|(key, value)| Some((key.clone(), value.parse().ok()?))).collect();
            let mut synth = Synth::read(&flat);
            synth.read_lists(pairs.get("notes"), pairs.get("drawing"), pairs.get("curve"));
            return Sound::of(synth);
        };
        let mut mix = Mix::default();
        for (key, value) in mix_fields(&mut mix) {
            if let Some(given) = number(key) {
                *value = given;
            }
        }
        mix.reverse = number("mix.reverse").is_some_and(|value| value > 0.5);
        mix.normalize = number("mix.normalize").is_some_and(|value| value > 0.5);
        mix.ping_pong = number("mix.pingpong").is_some_and(|value| value > 0.5);
        mix.limit = number("mix.limiter").is_some_and(|value| value > 0.5);
        let voices = (0..count as usize)
            .map(|at| {
                let prefix = format!("layer{at}.");
                let own: HashMap<String, f32> =
                    pairs.iter().filter_map(|(key, value)| Some((key.strip_prefix(&prefix)?.to_string(), value.parse().ok()?))).collect();
                let mut synth = Synth::read(&own);
                synth.read_lists(pairs.get(&format!("{prefix}notes")), pairs.get(&format!("{prefix}drawing")), pairs.get(&format!("{prefix}curve")));
                Voice {
                    name: pairs.get(&format!("{prefix}name")).cloned().unwrap_or_else(|| format!("Layer {}", at + 1)),
                    synth,
                    offset: own.get("layer_start").copied().unwrap_or(0.0),
                    gain: own.get("gain").copied().unwrap_or(1.0),
                    pan: own.get("pan").copied().unwrap_or(0.0),
                    muted: own.get("muted").is_some_and(|value| *value > 0.5),
                    solo: own.get("solo").is_some_and(|value| *value > 0.5),
                    sample: pairs.get(&format!("{prefix}sample")).cloned().unwrap_or_default(),
                    notes: pairs
                        .get(&format!("{prefix}score"))
                        .map(|score| {
                            score
                                .split(';')
                                .filter_map(|note| {
                                    let values: Vec<f32> = note.split(':').filter_map(|value| value.trim().parse().ok()).collect();
                                    (values.len() == 4).then(|| Note { start: values[0], length: values[1], tone: values[2], velocity: values[3] })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect::<Vec<_>>();
        if voices.is_empty() { Sound::of(Synth::default()) } else { Sound { voices, mix } }
    }
}
