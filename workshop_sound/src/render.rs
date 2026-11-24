//! A synth played into samples: its modulation, its filters (`Biquad`), the
//! samples it may play.

use std::f32::consts::{PI, TAU};

use super::Chance;
use super::{MOD_TARGETS, Synth, VOWEL_FORMANTS};

impl Synth {
    /// What its routes move at a moment: the LFOs and the random held at each of their cycles kept along.
    fn modulation(&self, time: f32, seconds: f32, random: f32, lfos: &mut LfoState, chance: &mut Chance) -> Moved {
        let mut lfo = [0.0_f32; 2];
        for (at, (shape, hz)) in [(self.lfo1_shape, self.lfo1_hz), (self.lfo2_shape, self.lfo2_hz)].into_iter().enumerate() {
            let position = hz.max(0.0) * time;
            let cycle = position.floor() as i64;
            if cycle != lfos.cycles[at] {
                lfos.cycles[at] = cycle;
                lfos.held[at] = chance.between(-1.0, 1.0);
            }
            let part = position.fract();
            lfo[at] = match shape.round() as i32 {
                1 => 1.0 - 4.0 * (part - 0.5).abs(),
                2 => {
                    if part < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                3 => 1.0 - 2.0 * part,
                4 => lfos.held[at],
                _ => (std::f32::consts::TAU * part).sin(),
            };
        }
        let envelope = if time < self.menv_attack {
            time / self.menv_attack.max(1e-6)
        } else {
            let after = time - self.menv_attack;
            if after < self.menv_decay { 1.0 - (1.0 - self.menv_sustain) * after / self.menv_decay.max(1e-6) } else { self.menv_sustain }
        };
        let along = (time / seconds.max(1e-6)).clamp(0.0, 1.0) * 31.0;
        let (below, part) = (along as usize, along.fract());
        let curve = self.curve[below.min(31)] * (1.0 - part) + self.curve[(below + 1).min(31)] * part;
        let mut moved = Moved::default();
        for [source, target, amount] in self.mods {
            let value = match source.round() as i32 {
                1 => lfo[0],
                2 => lfo[1],
                3 => envelope,
                4 => curve,
                5 => random,
                _ => continue,
            } * amount;
            match target.round() as i32 {
                1 => moved.pitch += value,
                2 => moved.cutoff += value,
                3 => moved.volume += value,
                4 => moved.pan += value,
                5 => moved.fm += value,
                6 => moved.duty += value,
                7 => moved.resonance += value,
                8 => moved.drive += value,
                9 => moved.vowel += value,
                10 => moved.ring += value,
                11 => moved.fold += value,
                _ => {}
            }
        }
        moved
    }

    /// Its samples, one channel, and where each is heard (empty: in the middle, its routes not moving it),
    /// played so (out of tune, from a phase, another voice bending it, a sample read).
    pub(super) fn samples_with(&self, context: &Context) -> (Vec<f32>, Vec<f32>) {
        let rate = shell::audio::OUTPUT_RATE as f32;
        let seconds = self.seconds().max(0.01);
        let count = (seconds * rate) as usize;
        let mut out = Vec::with_capacity(count);
        let routed = |target: i32| self.mods.iter().any(|[source, aim, amount]| source.round() > 0.5 && aim.round() as i32 == target && *amount != 0.0);
        let modulated = (1..MOD_TARGETS.len() as i32).any(routed);
        let mut pans = Vec::new();
        let panned = routed(4);
        let (mut voice, random) = Voice::new(self, context, rate);
        for at in 0..count {
            let time = at as f32 / rate;
            if modulated && at % 16 == 0 {
                voice.moved = self.modulation(time, seconds, random, &mut voice.lfos, &mut voice.chance);
            }
            let Some(pitch) = self.pitch_at(time) else { break };
            let pitch = self.bent_pitch(pitch, voice.moved.pitch + context.detune, time, rate);
            voice.step(pitch, rate);
            let other = context.modulator.and_then(|other| other.get(at).copied()).unwrap_or(0.0);
            let sample = self.wave_at(&mut voice, context, pitch, time, other, rate);
            let sample = self.shaped(&mut voice, sample, time, other, rate);
            let sample = self.filtered(&mut voice, sample, at, time, rate);
            let sample = self.effects(&mut voice, sample, at, time, rate);
            out.push((sample * self.volume).clamp(-1.0, 1.0));
            if panned {
                pans.push(voice.moved.pan.clamp(-1.0, 1.0));
            }
        }
        (out, pans)
    }

    /// Its pitch at a moment, so many half tones out of tune, with its vibrato.
    pub(super) fn bent_pitch(&self, pitch: f32, half_tones: f32, time: f32, rate: f32) -> f32 {
        let pitch = pitch * 2f32.powf(half_tones / 12.0);
        (pitch * (1.0 + self.vibrato * 0.5 * (TAU * self.vibrato_hz * time).sin())).clamp(10.0, rate * 0.45)
    }

    /// A wave read at a part of its period (`duty`: a square's high part).
    pub(super) fn tone(&self, part: f32, duty: f32) -> f32 {
        match self.wave {
            0 => {
                if part < duty {
                    0.5
                } else {
                    -0.5
                }
            }
            1 => 1.0 - 2.0 * part,
            2 => (TAU * part).sin(),
            7 => {
                // (Drawn: between its 32 steps, smoothly.)
                let at = part * 32.0;
                let (below, along) = (at as usize % 32, at.fract());
                self.drawn[below] * (1.0 - along) + self.drawn[(below + 1) % 32] * along
            }
            _ => 4.0 * (part - 0.5).abs() - 1.0,
        }
    }

    /// The wave's sample at a moment: bent by its FM (or another voice), of
    /// its kind (noises, the plucked string, a sample read, overtones).
    fn wave_at(&self, voice: &mut Voice, context: &Context, pitch: f32, time: f32, other: f32, rate: f32) -> f32 {
        let local = if self.repeat > 0.0 { time % self.repeat } else { time };
        // (FM: a second wave, so much faster, bending where the first is; or another voice bending it.)
        voice.modulator += pitch * self.fm_ratio.max(0.0) / rate;
        let depth = self.fm_depth + voice.moved.fm;
        let mut bent = if depth != 0.0 { voice.phase + depth * (TAU * voice.modulator).sin() / TAU } else { voice.phase };
        if self.from_fm != 0.0 {
            bent += self.from_fm * other / TAU;
        }
        let part = bent.rem_euclid(1.0);
        let duty = (self.duty + self.duty_sweep * local + voice.moved.duty).clamp(0.05, 0.95);
        let harmonics = self.harmonics.round().clamp(0.0, 16.0) as usize;
        match self.wave {
            4 => voice.noise[(part * 32.0) as usize % 32],
            5 => voice.pinked.clamp(-1.0, 1.0),
            6 => voice.bit,
            8 => self.plucked(voice, pitch, rate),
            9 => match context.sample {
                Some(data) if data.len() > 1 => self.sample_read(voice, data, pitch, rate),
                _ => 0.0,
            },
            _ if harmonics > 0 => {
                // (Its overtones over it, each softer.)
                let mut sum = 0.0;
                let mut weight = 0.0;
                for overtone in 0..=harmonics {
                    let strength = 1.0 / (overtone + 1) as f32;
                    sum += self.tone((part * (overtone + 1) as f32).fract(), duty) * strength;
                    weight += strength;
                }
                sum / weight
            }
            _ => self.tone(part, duty),
        }
    }

    /// The plucked string: its line read a period late, smoothed as it goes round.
    fn plucked(&self, voice: &mut Voice, pitch: f32, rate: f32) -> f32 {
        let string_length = voice.string.len();
        let length = (rate / pitch).clamp(2.0, (string_length - 2) as f32);
        let read = (voice.string_at as f32 + string_length as f32 - length) % string_length as f32;
        let (below, along) = (read as usize % string_length, read.fract());
        let value = voice.string[below] * (1.0 - along) + voice.string[(below + 1) % string_length] * along;
        let bright = self.string_bright.clamp(0.0, 1.0);
        let smoothed = value * bright + 0.5 * (value + voice.string_last) * (1.0 - bright);
        voice.string_last = value;
        voice.string[voice.string_at] = smoothed * (1.0 - (0.0005 + 0.03 * self.string_damp.clamp(0.0, 1.0)));
        voice.string_at = (voice.string_at + 1) % string_length;
        value
    }

    /// A sample read at its pitch (440: as it is), from its start, looped or not.
    fn sample_read(&self, voice: &mut Voice, data: &[f32], pitch: f32, rate: f32) -> f32 {
        let length = data.len() as f32;
        let start = (self.sample_start.max(0.0) * rate).min(length - 2.0);
        let position =
            if self.sample_loop > 0.5 { start + (voice.sample_position - start).rem_euclid((length - 1.0 - start).max(1.0)) } else { voice.sample_position };
        voice.sample_position += pitch / 440.0;
        if position < length - 1.0 {
            let (below, along) = (position as usize, position.fract());
            data[below] * (1.0 - along) + data[below + 1] * along
        } else {
            0.0
        }
    }

    /// The sample shaped: folded, at its envelope's loudness, trembling, ringing, driven.
    fn shaped(&self, voice: &mut Voice, mut sample: f32, time: f32, other: f32, rate: f32) -> f32 {
        let moved = voice.moved;
        // (Folded: pushed past its edges, it folds back over itself.)
        let fold = (self.fold + moved.fold).max(0.0);
        if fold > 0.0 {
            sample = (sample * (1.0 + fold * 6.0) * PI / 2.0).sin();
        }
        sample *= self.envelope_at(time) * (1.0 + moved.volume).max(0.0);
        // (AM: its loudness trembling; a ring: multiplied by a wave, or by another voice.)
        if self.am_depth > 0.0 {
            sample *= 1.0 - self.am_depth.min(1.0) * (0.5 + 0.5 * (TAU * self.am_hz * time).sin());
        }
        if self.ring_mix > 0.0 {
            voice.ring_phase += self.ring_hz.max(0.0) * 2f32.powf(moved.ring) / rate;
            let mix = self.ring_mix.min(1.0);
            sample = sample * (1.0 - mix) + sample * (TAU * voice.ring_phase).sin() * mix;
        }
        if self.from_ring > 0.0 {
            let mix = self.from_ring.min(1.0);
            sample = sample * (1.0 - mix) + sample * other * 2.0 * mix;
        }
        let drive = (self.drive + moved.drive).max(0.0);
        if drive > 0.0 {
            let push = 1.0 + drive * 12.0;
            sample = (sample * push).tanh() / push.tanh();
        }
        sample
    }

    /// The sample filtered: its resonant filter, a vowel, the high-pass.
    fn filtered(&self, voice: &mut Voice, mut sample: f32, at: usize, time: f32, rate: f32) -> f32 {
        let moved = voice.moved;
        // (The filter: a resonant biquad of its kind, steady at every cutoff, that sweeps and is moved.)
        let mode = self.filter_mode.round().clamp(0.0, 4.0) as usize;
        if self.lowpass < 15_900.0 || self.resonance > 0.0 || self.lowpass_sweep != 0.0 || mode != 0 || moved.cutoff != 0.0 || moved.resonance != 0.0 {
            if at.is_multiple_of(16) {
                let cutoff = (self.lowpass * 2f32.powf(self.lowpass_sweep * time + moved.cutoff)).clamp(40.0, rate * 0.45);
                let q = 0.707 + (self.resonance + moved.resonance).clamp(0.0, 0.95) * 8.0;
                voice.filter.set([0, 1, 2, 3, 4][mode], cutoff, q, self.filter_gain, rate);
            }
            sample = voice.filter.run(sample).clamp(-2.0, 2.0);
        }
        // (A vowel: three narrow bands where a mouth rings, between two vowels too.)
        let vowel = self.formant + moved.vowel;
        if self.formant_mix > 0.0 && vowel >= 1.0 {
            if at.is_multiple_of(16) {
                let place = (vowel - 1.0).clamp(0.0, 4.0);
                let (below, along) = (place as usize, place.fract());
                let above = (below + 1).min(4);
                for (band, formant) in voice.formants.iter_mut().enumerate() {
                    let frequency = VOWEL_FORMANTS[below][band] * (1.0 - along) + VOWEL_FORMANTS[above][band] * along;
                    formant.set(1, frequency, 8.0, 0.0, rate);
                }
            }
            let spoken: f32 = voice.formants.iter_mut().zip([1.0, 0.7, 0.45]).map(|(formant, weight)| formant.run(sample) * weight).sum::<f32>() * 1.8;
            let mix = self.formant_mix.min(1.0);
            sample = sample * (1.0 - mix) + spoken * mix;
        }
        // (High-pass: what is too low taken off, sweeping too.)
        let high_cut = (self.highpass * 2f32.powf(self.highpass_sweep * time)).clamp(0.0, rate * 0.45);
        if high_cut > 1.0 {
            let rc = 1.0 / (TAU * high_cut);
            let alpha = rc / (rc + 1.0 / rate);
            voice.high_out = alpha * (voice.high_out + sample - voice.high_in);
            voice.high_in = sample;
            sample = voice.high_out;
        }
        sample
    }

    /// The effects over it: the flanger, the chorus, the phaser, a coarser grain.
    fn effects(&self, voice: &mut Voice, mut sample: f32, at: usize, time: f32, rate: f32) -> f32 {
        // (The flanger: itself a little later, over itself.)
        if self.flanger > 0.0 || self.flanger_sweep != 0.0 {
            let delay = (((self.flanger + self.flanger_sweep * time).abs() / 1000.0 * rate) as usize).min(1023);
            voice.flange[at % 1024] = sample;
            sample += voice.flange[(at + 1024 - delay) % 1024];
        }
        if self.chorus > 0.0 {
            sample = self.chorused(voice, sample, at, time, rate);
        }
        if self.phaser > 0.0 {
            sample = self.phased(voice, sample, time, rate);
        }
        // (A coarser grain: held, so many times a second.)
        if self.crush > 0.0 {
            let every = (rate / self.crush.max(100.0)).max(1.0) as usize;
            if at.is_multiple_of(every) {
                voice.held = sample;
            }
            sample = voice.held;
        }
        sample
    }

    /// Chorus: three of it, each a little late, the lateness swaying.
    fn chorused(&self, voice: &mut Voice, sample: f32, at: usize, time: f32, rate: f32) -> f32 {
        voice.chorus_line[at % 2048] = sample;
        let mut wet = 0.0;
        for copy in 0..3 {
            let late = (0.012 + 0.005 * copy as f32 + 0.004 * self.chorus.min(1.0) * (TAU * (self.chorus_hz * time + copy as f32 / 3.0)).sin()) * rate;
            let read = (at as f32 + 2048.0 - late) % 2048.0;
            let (below, along) = (read as usize % 2048, read.fract());
            wet += voice.chorus_line[below] * (1.0 - along) + voice.chorus_line[(below + 1) % 2048] * along;
        }
        let mix = self.chorus.min(1.0);
        sample * (1.0 - 0.5 * mix) + wet / 3.0 * mix
    }

    /// Phaser: four all-pass steps, their place swaying, over itself.
    fn phased(&self, voice: &mut Voice, sample: f32, time: f32, rate: f32) -> f32 {
        let sway = 0.5 + 0.5 * (TAU * self.phaser_hz * time).sin();
        let frequency = 200.0 * 2f32.powf(sway * 4.0);
        let tangent = (PI * frequency / rate).tan();
        let coefficient = (tangent - 1.0) / (tangent + 1.0);
        let mut through = sample;
        for stage in 0..4 {
            let (last_in, last_out) = (voice.phaser_state[stage * 2], voice.phaser_state[stage * 2 + 1]);
            let made = coefficient * through + last_in - coefficient * last_out;
            voice.phaser_state[stage * 2] = through;
            voice.phaser_state[stage * 2 + 1] = made;
            through = made;
        }
        let mix = self.phaser.min(1.0);
        sample * (1.0 - 0.5 * mix) + through * 0.5 * mix
    }
}

/// What a synth keeps from a sample to the next as it is played: its chance,
/// its LFOs, what its routes move; its phase (the noises renewed each period,
/// the coloured ones stepped 32 times a period); its FM's and ring's phases;
/// the plucked string; where a sample is read; its filters' and effects' memories.
struct Voice {
    chance: Chance,
    lfos: LfoState,
    moved: Moved,
    noise: [f32; 32],
    phase: f32,
    periods: i64,
    steps: i64,
    pink: [f32; 3],
    pinked: f32,
    lfsr: u16,
    bit: f32,
    modulator: f32,
    ring_phase: f32,
    string: Vec<f32>,
    string_at: usize,
    string_last: f32,
    sample_position: f32,
    filter: Biquad,
    formants: [Biquad; 3],
    high_out: f32,
    high_in: f32,
    flange: Vec<f32>,
    chorus_line: Vec<f32>,
    phaser_state: [f32; 8],
    held: f32,
}

impl Voice {
    /// A voice ready to play (its noise drawn, its string plucked), and the random its routes hold.
    fn new(synth: &Synth, context: &Context, rate: f32) -> (Voice, f32) {
        let mut chance = Chance::new(synth.seed.wrapping_add(context.seed));
        let mut noise = [0.0_f32; 32];
        for value in &mut noise {
            *value = chance.between(-1.0, 1.0);
        }
        let random = Chance::new(synth.seed ^ 0x5EED_u32.wrapping_add(context.seed)).between(-1.0, 1.0);
        // (The plucked string: a line of noise, read a period late, smoothed as it goes round.)
        let string_length = (rate / 20.0) as usize + 2;
        let string: Vec<f32> = if synth.wave == 8 { (0..string_length).map(|_| chance.between(-1.0, 1.0)).collect() } else { Vec::new() };
        let voice = Voice {
            chance,
            lfos: LfoState { cycles: [i64::MIN; 2], held: [0.0; 2] },
            moved: Moved::default(),
            noise,
            phase: context.phase,
            periods: 0,
            steps: 0,
            pink: [0.0; 3],
            pinked: 0.0,
            lfsr: 0x7FFF,
            bit: 0.5,
            modulator: 0.0,
            ring_phase: 0.0,
            string,
            string_at: 0,
            string_last: 0.0,
            sample_position: synth.sample_start.max(0.0) * rate,
            filter: Biquad::default(),
            formants: [Biquad::default(); 3],
            high_out: 0.0,
            high_in: 0.0,
            flange: vec![0.0; 1024],
            chorus_line: if synth.chorus > 0.0 { vec![0.0; 2048] } else { Vec::new() },
            phaser_state: [0.0; 8],
            held: 0.0,
        };
        (voice, random)
    }

    /// Its phase on at that pitch: a new period draws the noise again; a new
    /// 32nd of it steps the pink noise and the noise of bits.
    fn step(&mut self, pitch: f32, rate: f32) {
        self.phase += pitch / rate;
        if self.phase.floor() as i64 != self.periods {
            self.periods = self.phase.floor() as i64;
            for value in &mut self.noise {
                *value = self.chance.between(-1.0, 1.0);
            }
        }
        // (The noises that are not white: stepped 32 times a period, so they have a pitch.)
        if (self.phase * 32.0).floor() as i64 != self.steps {
            self.steps = (self.phase * 32.0).floor() as i64;
            let white = self.chance.between(-1.0, 1.0);
            self.pink[0] = 0.99765 * self.pink[0] + white * 0.099_046;
            self.pink[1] = 0.963 * self.pink[1] + white * 0.296_516_4;
            self.pink[2] = 0.57 * self.pink[2] + white * 1.052_691_3;
            self.pinked = (self.pink[0] + self.pink[1] + self.pink[2] + white * 0.1848) * 0.25;
            let feedback = (self.lfsr ^ (self.lfsr >> 1)) & 1;
            self.lfsr = (self.lfsr >> 1) | (feedback << 14);
            self.bit = if self.lfsr & 1 == 1 { 0.5 } else { -0.5 };
        }
    }
}

/// How a synth is played: so many half tones out of tune, from a phase (a copy of its unison, its own chance),
/// another voice bending it, a sample read.
#[derive(Default)]
pub(super) struct Context<'a> {
    pub(super) detune: f32,
    pub(super) phase: f32,
    pub(super) seed: u32,
    pub(super) modulator: Option<&'a [f32]>,
    pub(super) sample: Option<&'a [f32]>,
}

/// What a synth's routes move, at a moment.
#[derive(Clone, Copy, Default)]
struct Moved {
    pitch: f32,
    cutoff: f32,
    volume: f32,
    pan: f32,
    fm: f32,
    duty: f32,
    resonance: f32,
    drive: f32,
    vowel: f32,
    ring: f32,
    fold: f32,
}

/// The LFOs' cycles so far, and the random each holds for its cycle.
struct LfoState {
    cycles: [i64; 2],
    held: [f32; 2],
}

/// A filter of two poles and two zeros (the cookbook's), of a kind: low-pass, band-pass, notch, peak,
/// high-pass, low shelf, high shelf.
#[derive(Clone, Copy, Default)]
pub struct Biquad {
    terms: [f32; 5],
    inputs: [f32; 2],
    outputs: [f32; 2],
}

impl Biquad {
    pub fn set(&mut self, kind: usize, frequency: f32, q: f32, gain: f32, rate: f32) {
        let w0 = std::f32::consts::TAU * frequency.clamp(10.0, rate * 0.49) / rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q.max(0.05));
        let a = 10f32.powf(gain / 40.0);
        let root = 2.0 * a.sqrt() * alpha;
        let [b0, b1, b2, a0, a1, a2] = match kind {
            1 => [alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha],
            2 => [1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha],
            3 => [1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a],
            4 => [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha],
            5 => [
                a * ((a + 1.0) - (a - 1.0) * cos + root),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                a * ((a + 1.0) - (a - 1.0) * cos - root),
                (a + 1.0) + (a - 1.0) * cos + root,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                (a + 1.0) + (a - 1.0) * cos - root,
            ],
            6 => [
                a * ((a + 1.0) + (a - 1.0) * cos + root),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                a * ((a + 1.0) + (a - 1.0) * cos - root),
                (a + 1.0) - (a - 1.0) * cos + root,
                2.0 * ((a - 1.0) - (a + 1.0) * cos),
                (a + 1.0) - (a - 1.0) * cos - root,
            ],
            _ => [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        };
        self.terms = [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0];
    }

    pub fn run(&mut self, sample: f32) -> f32 {
        let [b0, b1, b2, a1, a2] = self.terms;
        let made = b0 * sample + b1 * self.inputs[0] + b2 * self.inputs[1] - a1 * self.outputs[0] - a2 * self.outputs[1];
        self.inputs = [sample, self.inputs[0]];
        self.outputs = [made, self.outputs[0]];
        made
    }
}
