//! A voice's synth: tidied, made by kind, by chance, a little changed, its
//! pitch and envelope at a moment, written and read back.

use std::collections::HashMap;

use super::Chance;
use super::render::Context;
use super::{MOD_KEYS, Synth, WAVES};

/// The settings a synth is written with, by their names (its wave, seed, notes and drawing apart).
fn synth_fields(synth: &mut Synth) -> Vec<(&'static str, &mut f32)> {
    let mut fields = vec![
        ("harmonics", &mut synth.harmonics),
        ("duty", &mut synth.duty),
        ("duty_sweep", &mut synth.duty_sweep),
        ("attack", &mut synth.attack),
        ("sustain", &mut synth.sustain),
        ("punch", &mut synth.punch),
        ("decay", &mut synth.decay),
        ("pitch", &mut synth.pitch),
        ("min_pitch", &mut synth.min_pitch),
        ("slide", &mut synth.slide),
        ("delta_slide", &mut synth.delta_slide),
        ("vibrato", &mut synth.vibrato),
        ("vibrato_hz", &mut synth.vibrato_hz),
        ("arp", &mut synth.arp_mult),
        ("arp_time", &mut synth.arp_time),
        ("repeat", &mut synth.repeat),
        ("lowpass", &mut synth.lowpass),
        ("lowpass_sweep", &mut synth.lowpass_sweep),
        ("resonance", &mut synth.resonance),
        ("highpass", &mut synth.highpass),
        ("highpass_sweep", &mut synth.highpass_sweep),
        ("flanger", &mut synth.flanger),
        ("flanger_sweep", &mut synth.flanger_sweep),
        ("crush", &mut synth.crush),
        ("volume", &mut synth.volume),
        ("fm_ratio", &mut synth.fm_ratio),
        ("fm_depth", &mut synth.fm_depth),
        ("am_hz", &mut synth.am_hz),
        ("am_depth", &mut synth.am_depth),
        ("drive", &mut synth.drive),
        ("notes_count", &mut synth.notes_len),
        ("note_time", &mut synth.note_time),
        ("filter_mode", &mut synth.filter_mode),
        ("filter_gain", &mut synth.filter_gain),
        ("vowel", &mut synth.formant),
        ("vowel_mix", &mut synth.formant_mix),
        ("string_damp", &mut synth.string_damp),
        ("string_bright", &mut synth.string_bright),
        ("sample_start", &mut synth.sample_start),
        ("sample_loop", &mut synth.sample_loop),
        ("unison", &mut synth.unison),
        ("detune", &mut synth.detune),
        ("spread", &mut synth.spread),
        ("ring_hz", &mut synth.ring_hz),
        ("ring_mix", &mut synth.ring_mix),
        ("chorus", &mut synth.chorus),
        ("chorus_hz", &mut synth.chorus_hz),
        ("phaser", &mut synth.phaser),
        ("phaser_hz", &mut synth.phaser_hz),
        ("fold", &mut synth.fold),
        ("lfo1_shape", &mut synth.lfo1_shape),
        ("lfo1_hz", &mut synth.lfo1_hz),
        ("lfo2_shape", &mut synth.lfo2_shape),
        ("lfo2_hz", &mut synth.lfo2_hz),
        ("env2_attack", &mut synth.menv_attack),
        ("env2_decay", &mut synth.menv_decay),
        ("env2_level", &mut synth.menv_sustain),
        ("mod_by", &mut synth.from_voice),
        ("mod_fm", &mut synth.from_fm),
        ("mod_ring", &mut synth.from_ring),
    ];
    for (keys, [source, target, amount]) in MOD_KEYS.iter().zip(synth.mods.iter_mut()) {
        fields.extend([(keys[0], source), (keys[1], target), (keys[2], amount)]);
    }
    fields
}

impl Synth {
    /// Its settings to three decimals (as the sliders show them: they would round them else).
    pub(super) fn tidy(mut self) -> Synth {
        for (_, value) in synth_fields(&mut self) {
            *value = (*value * 1000.0).round() / 1000.0;
        }
        self
    }

    pub(super) fn seconds(&self) -> f32 {
        self.attack + self.sustain + self.decay
    }

    /// A new one, its locked groups kept as they are in that one.
    pub(super) fn kept(mut self, old: &Synth, locks: &[bool; 6]) -> Synth {
        if locks[0] {
            (self.attack, self.sustain, self.punch, self.decay) = (old.attack, old.sustain, old.punch, old.decay);
        }
        if locks[1] {
            (self.pitch, self.min_pitch, self.slide, self.delta_slide) = (old.pitch, old.min_pitch, old.slide, old.delta_slide);
        }
        if locks[2] {
            (self.vibrato, self.vibrato_hz, self.arp_mult, self.arp_time) = (old.vibrato, old.vibrato_hz, old.arp_mult, old.arp_time);
        }
        if locks[3] {
            (self.wave, self.harmonics, self.duty, self.duty_sweep, self.repeat) = (old.wave, old.harmonics, old.duty, old.duty_sweep, old.repeat);
        }
        if locks[4] {
            (self.lowpass, self.lowpass_sweep, self.resonance, self.highpass, self.highpass_sweep) =
                (old.lowpass, old.lowpass_sweep, old.resonance, old.highpass, old.highpass_sweep);
        }
        if locks[5] {
            (self.flanger, self.flanger_sweep, self.crush, self.volume) = (old.flanger, old.flanger_sweep, old.crush, old.volume);
        }
        self
    }

    /// A sound of that kind, made by chance.
    pub(super) fn of_kind(kind: &str, seed: u32) -> Synth {
        let mut chance = Chance::new(seed);
        let mut made = Synth { seed, ..Synth::default() };
        match kind {
            "Coin" => {
                made.wave = if chance.next() < 0.5 { 0 } else { 1 };
                made.pitch = chance.between(700.0, 1300.0);
                made.sustain = chance.between(0.02, 0.08);
                made.decay = chance.between(0.15, 0.35);
                made.punch = chance.between(0.3, 0.6);
                if chance.next() < 0.7 {
                    made.arp_mult = chance.between(1.3, 1.6);
                    made.arp_time = chance.between(0.03, 0.08);
                }
            }
            "Laser" => {
                made.wave = (chance.next() * 3.0) as usize;
                made.pitch = chance.between(800.0, 2400.0);
                made.min_pitch = chance.between(60.0, 200.0);
                made.slide = -chance.between(4.0, 12.0);
                made.duty = chance.between(0.2, 0.6);
                made.duty_sweep = chance.between(-0.5, 0.5);
                made.sustain = chance.between(0.05, 0.15);
                made.decay = chance.between(0.05, 0.2);
                made.punch = chance.between(0.0, 0.3);
            }
            "Explosion" => {
                made.wave = if chance.next() < 0.6 { 4 } else { 6 };
                made.pitch = chance.between(60.0, 400.0);
                made.slide = chance.between(-3.0, 0.5);
                made.sustain = chance.between(0.1, 0.3);
                made.decay = chance.between(0.3, 0.7);
                made.punch = chance.between(0.2, 0.8);
                if chance.next() < 0.5 {
                    made.flanger = chance.between(1.0, 6.0);
                    made.flanger_sweep = chance.between(-4.0, 4.0);
                }
                if chance.next() < 0.33 {
                    made.repeat = chance.between(0.15, 0.4);
                }
            }
            "Power-up" => {
                made.wave = if chance.next() < 0.5 { 1 } else { 0 };
                made.pitch = chance.between(200.0, 600.0);
                made.slide = chance.between(1.0, 4.0);
                made.sustain = chance.between(0.15, 0.35);
                made.decay = chance.between(0.15, 0.4);
                if chance.next() < 0.5 {
                    made.vibrato = chance.between(0.2, 0.5);
                    made.vibrato_hz = chance.between(8.0, 20.0);
                } else {
                    made.repeat = chance.between(0.05, 0.12);
                }
            }
            "Hit" => {
                made.wave = if chance.next() < 0.5 { 4 } else { 1 };
                made.pitch = chance.between(150.0, 900.0);
                made.slide = -chance.between(3.0, 8.0);
                made.sustain = chance.between(0.0, 0.05);
                made.decay = chance.between(0.08, 0.2);
                made.highpass = if chance.next() < 0.5 { chance.between(100.0, 600.0) } else { 0.0 };
            }
            "Jump" => {
                made.wave = 0;
                made.duty = chance.between(0.2, 0.5);
                made.pitch = chance.between(250.0, 600.0);
                made.slide = chance.between(2.0, 6.0);
                made.sustain = chance.between(0.05, 0.15);
                made.decay = chance.between(0.08, 0.2);
                made.lowpass = if chance.next() < 0.5 { chance.between(2000.0, 8000.0) } else { 16000.0 };
            }
            "Blip" => {
                made.wave = (chance.next() * 2.0) as usize;
                made.duty = chance.between(0.2, 0.5);
                made.pitch = chance.between(400.0, 1800.0);
                made.sustain = chance.between(0.03, 0.08);
                made.decay = chance.between(0.0, 0.05);
                made.highpass = 100.0;
            }
            "Magic" => {
                made.wave = 2;
                made.harmonics = chance.between(0.0, 3.0).floor();
                made.pitch = chance.between(400.0, 900.0);
                made.slide = chance.between(0.5, 2.5);
                made.attack = chance.between(0.05, 0.2);
                made.sustain = chance.between(0.2, 0.5);
                made.decay = chance.between(0.3, 0.8);
                made.vibrato = chance.between(0.15, 0.4);
                made.vibrato_hz = chance.between(5.0, 12.0);
                made.flanger = chance.between(0.5, 3.0);
                made.flanger_sweep = chance.between(-1.0, 1.0);
            }
            "Step" => {
                made.wave = if chance.next() < 0.5 { 4 } else { 5 };
                made.pitch = chance.between(80.0, 250.0);
                made.sustain = 0.01;
                made.decay = chance.between(0.04, 0.1);
                made.lowpass = chance.between(800.0, 2500.0);
                made.volume = 0.5;
            }
            _ => {}
        }
        made.tidy()
    }

    /// Anything, by chance.
    pub(super) fn random(seed: u32) -> Synth {
        let mut chance = Chance::new(seed);
        Synth {
            wave: (chance.next() * WAVES.len() as f32) as usize % WAVES.len(),
            harmonics: if chance.next() < 0.3 { chance.between(0.0, 4.0).floor() } else { 0.0 },
            duty: chance.between(0.1, 0.9),
            duty_sweep: chance.between(-0.5, 0.5),
            attack: chance.between(0.0, 0.15).powi(2) * 4.0,
            sustain: chance.between(0.0, 0.4),
            punch: chance.between(0.0, 0.6),
            decay: chance.between(0.05, 0.6),
            pitch: chance.between(60.0, 2400.0),
            min_pitch: 0.0,
            slide: chance.between(-6.0, 6.0),
            delta_slide: chance.between(-4.0, 4.0) * chance.next(),
            vibrato: if chance.next() < 0.3 { chance.between(0.0, 0.5) } else { 0.0 },
            vibrato_hz: chance.between(2.0, 20.0),
            arp_mult: if chance.next() < 0.3 { chance.between(0.6, 1.8) } else { 1.0 },
            arp_time: chance.between(0.02, 0.2),
            repeat: if chance.next() < 0.2 { chance.between(0.05, 0.3) } else { 0.0 },
            lowpass: if chance.next() < 0.4 { chance.between(400.0, 12000.0) } else { 16000.0 },
            lowpass_sweep: chance.between(-2.0, 2.0) * chance.next(),
            resonance: chance.between(0.0, 0.7),
            highpass: if chance.next() < 0.3 { chance.between(50.0, 1500.0) } else { 0.0 },
            highpass_sweep: 0.0,
            flanger: if chance.next() < 0.25 { chance.between(0.5, 6.0) } else { 0.0 },
            flanger_sweep: chance.between(-3.0, 3.0),
            crush: 0.0,
            volume: 0.6,
            fm_depth: if chance.next() < 0.15 { chance.between(0.5, 4.0) } else { 0.0 },
            fm_ratio: chance.between(1.0, 5.0).round(),
            seed,
            ..Synth::default()
        }
        .tidy()
    }

    /// Changed a little, here and there.
    pub(super) fn mutated(&self, seed: u32) -> Synth {
        let mut chance = Chance::new(seed);
        let mut nudge = |value: f32, spread: f32| if chance.next() < 0.5 { value + chance.between(-spread, spread) } else { value };
        Synth {
            duty: nudge(self.duty, 0.08).clamp(0.05, 0.95),
            attack: nudge(self.attack, 0.02).max(0.0),
            sustain: nudge(self.sustain, 0.04).max(0.0),
            punch: nudge(self.punch, 0.08).clamp(0.0, 1.0),
            decay: nudge(self.decay, 0.05).max(0.01),
            pitch: (self.pitch * 2f32.powf(nudge(0.0, 0.25))).clamp(30.0, 4000.0),
            slide: nudge(self.slide, 0.6),
            delta_slide: nudge(self.delta_slide, 0.4),
            vibrato: nudge(self.vibrato, 0.05).clamp(0.0, 1.0),
            lowpass: (self.lowpass * 2f32.powf(nudge(0.0, 0.3))).clamp(100.0, 16000.0),
            resonance: nudge(self.resonance, 0.06).clamp(0.0, 0.95),
            seed,
            ..*self
        }
        .tidy()
    }

    /// The pitch at a moment (before its vibrato): for its curve.
    pub(super) fn pitch_at(&self, time: f32) -> Option<f32> {
        let local = if self.repeat > 0.0 { time % self.repeat } else { time };
        let mut pitch = self.pitch.max(1.0) * 2f32.powf(self.slide * local + 0.5 * self.delta_slide * local * local);
        if self.arp_time > 0.0 && local >= self.arp_time {
            pitch *= self.arp_mult;
        }
        let count = self.notes_len.round().clamp(0.0, 8.0) as usize;
        if count > 0 {
            let at = (local / self.note_time.max(0.005)) as usize % count;
            pitch *= 2f32.powf(self.notes[at] / 12.0);
        }
        (self.min_pitch <= 0.0 || pitch >= self.min_pitch).then_some(pitch.clamp(10.0, 16000.0))
    }

    /// Its envelope at a moment: comes in, holds (struck harder at first), goes.
    pub(super) fn envelope_at(&self, time: f32) -> f32 {
        if time < self.attack {
            time / self.attack.max(1e-6)
        } else if time < self.attack + self.sustain {
            1.0 + self.punch * (1.0 - (time - self.attack) / self.sustain.max(1e-6))
        } else {
            (1.0 - (time - self.attack - self.sustain) / self.decay.max(1e-6)).max(0.0)
        }
    }

    /// Its samples, one channel, at the speakers' rate, from -1 to 1.
    pub(super) fn samples(&self) -> Vec<f32> {
        self.samples_with(&Context::default()).0
    }
}

impl Synth {
    /// Written as `key=value` lines, under a prefix (read back by `read`).
    pub(super) fn written(&self, prefix: &str) -> String {
        let mut copy = *self;
        let listed = |values: &[f32]| values.iter().map(|value| format!("{value}")).collect::<Vec<_>>().join(",");
        let mut said =
            format!("{prefix}wave={}\n{prefix}seed={}\n{prefix}notes={}\n{prefix}drawing={}\n", self.wave, self.seed, listed(&self.notes), listed(&self.drawn));
        if self.curve.iter().any(|value| *value != 0.0) {
            said.push_str(&format!("{prefix}curve={}\n", listed(&self.curve)));
        }
        for (key, value) in synth_fields(&mut copy) {
            said.push_str(&format!("{prefix}{key}={value}\n"));
        }
        said
    }

    /// Its notes and drawing, from their lists (`notes=0,4,7`).
    pub(super) fn read_lists(&mut self, notes: Option<&String>, drawn: Option<&String>, curve: Option<&String>) {
        let parse = |said: &String| said.split(',').filter_map(|value| value.trim().parse::<f32>().ok()).collect::<Vec<_>>();
        if let Some(notes) = notes.map(parse) {
            for (at, value) in notes.into_iter().take(8).enumerate() {
                self.notes[at] = value;
            }
        }
        if let Some(drawn) = drawn.map(parse).filter(|drawn| drawn.len() == 32) {
            self.drawn.copy_from_slice(&drawn);
        }
        if let Some(curve) = curve.map(parse).filter(|curve| curve.len() == 32) {
            self.curve.copy_from_slice(&curve);
        }
    }

    /// Read back from `key=value` pairs (the first maker's too: from a pitch to another over a time).
    pub(super) fn read(pairs: &HashMap<String, f32>) -> Synth {
        let mut made = Synth::default();
        if let Some(wave) = pairs.get("wave") {
            made.wave = (*wave as usize).min(WAVES.len() - 1);
        }
        if let Some(seed) = pairs.get("seed") {
            made.seed = *seed as u32;
        }
        for (key, value) in synth_fields(&mut made) {
            if let Some(given) = pairs.get(key) {
                *value = *given;
            }
        }
        if let (Some(start), Some(end), Some(seconds)) = (pairs.get("start"), pairs.get("end"), pairs.get("duration")) {
            made.pitch = *start;
            made.slide = (end / start.max(1.0)).log2() / seconds.max(0.01);
            made.decay = made.decay.min(*seconds);
            made.sustain = (seconds - made.attack - made.decay).max(0.0);
        }
        made
    }
}
