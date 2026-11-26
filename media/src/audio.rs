//! A small software synthesizer for the game's sequenced music and sounds:
//! it plays the notes of a song (`music::Song`) with the samples of its
//! sound banks (`music::Bank`) and returns PCM audio.
//!
//! It is simpler than the original sound driver: one channel (no panning),
//! no LFO, and the pitches of the console's own waves are worked out from
//! the notes the game's sounds play, not from the driver.

use std::collections::HashMap;

use crate::music::{Bank, Envelope, Event, SampleFormat, Song, Track};

pub const OUTPUT_RATE: u32 = 32_000;
const DEFAULT_TEMPO: f64 = 120.0;
/// The shortest a note takes to go, in seconds: no click.
const RELEASE: f32 = 0.005;
/// How long the last notes may ring after the end of the song.
pub const TAIL: f32 = 1.5;
/// Headroom left for several notes sounding at once.
const NOTE_GAIN: f32 = 0.22;
/// How much louder sounds are played than songs, whose many notes at once
/// would otherwise cover them.
pub const SOUND_GAIN: f32 = 3.0;
/// How loud the console's own waves are, a sample's loudest being 1.
const WAVE_LEVEL: f32 = 0.4;
/// The semitones a whole bend is worth until a track says otherwise.
const BEND_RANGE: f32 = 2.0;
/// The samples between two readings of what changes as a note goes.
const CONTROL: usize = 16;

/// The banks a song plays with: its own (its programs), the one holding
/// the sample data of banks that have none, and those its tracks may name
/// (by the two bytes at 0x0E of their files).
pub struct Banks<'a> {
    pub own: &'a Bank,
    pub main: &'a Bank,
    pub named: &'a HashMap<[u8; 2], Bank>,
    /// How much louder than written everything is played.
    pub gain: f32,
}

/// What a note is made of.
enum Source {
    Sample {
        pcm: Vec<i16>,
        loop_start: usize,
        looped: bool,
    },
    /// The console's square wave, up for so many eighths of its time.
    Square(f32),
    /// The console's noise.
    Noise,
}

struct Wave {
    source: Source,
    /// Samples a second on the root key (of the console's waves: an
    /// eighth of a square's period, a step of the noise).
    rate: f64,
}

/// Something of a track that changes with time: `(seconds, value)`, in a
/// straight line from one to the next.
struct Curve {
    points: Vec<(f64, f32)>,
}

impl Curve {
    fn new(value: f32) -> Self {
        Self { points: vec![(0.0, value)] }
    }

    fn at(&self, time: f64) -> f32 {
        let next = self.points.partition_point(|point| point.0 <= time);
        let Some(&(from, value)) = self.points.get(next.wrapping_sub(1)) else {
            return self.points[0].1;
        };
        match self.points.get(next) {
            Some(&(to, then)) if to > from => value + (then - value) * ((time - from) / (to - from)) as f32,
            _ => value,
        }
    }

    /// `value` is reached `over` seconds after `time` (0: at once).
    fn reach(&mut self, time: f64, over: f64, value: f32) {
        let now = self.at(time);
        self.points.retain(|point| point.0 < time);
        self.points.push((time, now));
        self.points.push((time + over.max(0.0), value));
    }
}

/// A note's loudness as it goes (see [`Envelope`]), in seconds and parts
/// of the whole.
struct Shape {
    first: f32,
    attack: f32,
    hold: f32,
    decay: Option<f32>,
    sustain: f32,
    fade: Option<f32>,
    release: f32,
}

impl Shape {
    /// The envelope of a split, with what the track changed of it.
    fn new(envelope: &Envelope, changed: &[Option<u8>; 8]) -> Self {
        let part = |part: usize, own: u8| changed[part].unwrap_or(own);
        if !envelope.on || changed[0].is_some() {
            return Self { first: 1.0, attack: 0.0, hold: 0.0, decay: None, sustain: 1.0, fade: None, release: RELEASE };
        }
        let seconds = |time: u8| envelope.seconds(time);
        Self {
            first: part(1, envelope.first).min(127) as f32 / 127.0,
            attack: seconds(part(2, envelope.attack)).unwrap_or(0.0),
            hold: seconds(part(3, envelope.hold)).unwrap_or(0.0),
            decay: seconds(part(4, envelope.decay)),
            sustain: part(7, envelope.sustain).min(127) as f32 / 127.0,
            fade: seconds(part(5, envelope.fade)),
            release: seconds(part(6, envelope.release)).unwrap_or(TAIL).max(RELEASE),
        }
    }

    /// The loudness `time` seconds into a note held for `held`.
    fn level(&self, time: f32, held: f32) -> f32 {
        let held_level = |time: f32| {
            if time < self.attack {
                return self.first + (1.0 - self.first) * time / self.attack;
            }
            let time = time - self.attack - self.hold;
            let Some(decay) = self.decay.filter(|_| time > 0.0) else {
                return 1.0;
            };
            if time < decay {
                return 1.0 + (self.sustain - 1.0) * time / decay;
            }
            match self.fade {
                Some(fade) if fade > 0.0 => self.sustain * (1.0 - (time - decay) / fade).max(0.0),
                Some(_) => 0.0,
                None => self.sustain,
            }
        };
        match time < held {
            true => held_level(time),
            false => held_level(held) * (1.0 - (time - held) / self.release).max(0.0),
        }
    }
}

/// Converts ticks to seconds, following the song's tempo changes.
struct TempoMap {
    /// `(tick, seconds at that tick, seconds per tick from there)`.
    segments: Vec<(u32, f64, f64)>,
}

impl TempoMap {
    fn new(song: &Song) -> Self {
        let mut changes: Vec<(u32, f64)> = song
            .tracks
            .iter()
            .flat_map(|track| &track.events)
            .filter_map(|&(tick, event)| match event {
                Event::Tempo(bpm) if bpm > 0 => Some((tick, bpm as f64)),
                _ => None,
            })
            .collect();
        changes.sort_by_key(|&(tick, _)| tick);
        let per_tick = |bpm: f64| 60.0 / (bpm * song.ticks_per_quarter.max(1) as f64);
        let mut segments = vec![(0, 0.0, per_tick(DEFAULT_TEMPO))];
        for (tick, bpm) in changes {
            let &(start, seconds, rate) = segments.last().unwrap();
            segments.push((tick, seconds + (tick - start) as f64 * rate, per_tick(bpm)));
        }
        Self { segments }
    }

    fn seconds(&self, tick: u32) -> f64 {
        let index = self.segments.partition_point(|&(start, ..)| start <= tick) - 1;
        let (start, seconds, rate) = self.segments[index];
        seconds + (tick - start) as f64 * rate
    }
}

/// What changes along a track while its notes sound.
struct Controls {
    volume: Curve,
    expression: Curve,
    /// Semitones, of the bends and of the tuning.
    bend: Curve,
    tune: Curve,
}

impl Controls {
    fn new(events: &[(u32, Event)], tempo: &TempoMap) -> Self {
        let mut controls = Self { volume: Curve::new(127.0), expression: Curve::new(127.0), bend: Curve::new(0.0), tune: Curve::new(0.0) };
        let (mut range, mut bent) = (BEND_RANGE, 0.0);
        for &(tick, event) in events {
            let at = tempo.seconds(tick);
            let over = |ticks: u16| tempo.seconds(tick + ticks as u32) - at;
            match event {
                Event::Volume(volume) => controls.volume.reach(at, 0.0, volume as f32),
                Event::VolumeSweep { volume, ticks } => controls.volume.reach(at, over(ticks), volume as f32),
                Event::Expression(volume) => controls.expression.reach(at, 0.0, volume as f32),
                Event::Tune { semitones, ticks } => controls.tune.reach(at, over(ticks), semitones as f32),
                Event::Bend(bend) => {
                    bent = bend as f32 / 8192.0;
                    controls.bend.reach(at, 0.0, bent * range);
                }
                Event::BendRange(semitones) => {
                    range = semitones as f32;
                    controls.bend.reach(at, 0.0, bent * range);
                }
                _ => {}
            }
        }
        controls
    }

    /// How much faster a note plays, and how loud, at `time`.
    fn at(&self, time: f64) -> (f64, f32) {
        let pitch = 2f64.powf((self.bend.at(time) + self.tune.at(time)) as f64 / 12.0);
        (pitch, self.volume.at(time).clamp(0.0, 127.0) / 127.0 * self.expression.at(time).clamp(0.0, 127.0) / 127.0)
    }
}

/// The song as it plays for `max_seconds`: each track that has a loop
/// point (`Event::LoopPoint`) goes back to it at its end, again and again,
/// as the original's sequencer does; its events are laid out one after the
/// other. A song without one is as it is.
fn unrolled(song: &Song, max_seconds: f32) -> Song {
    let tempo = TempoMap::new(song);
    let tracks = song
        .tracks
        .iter()
        .map(|track| {
            let looped = track.events.iter().find(|(_, event)| *event == Event::LoopPoint).map(|&(tick, _)| tick);
            let Some(start) = looped.filter(|&start| start < track.length) else {
                return Track { channel: track.channel, events: track.events.clone(), length: track.length };
            };
            let period = track.length - start;
            // (As many times again as fills the time, by the tempo of the
            // first time through.)
            let once = (tempo.seconds(track.length) - tempo.seconds(start)).max(0.01);
            let left = (max_seconds as f64 - tempo.seconds(track.length)).max(0.0);
            let again = ((left / once).ceil() as u32).min(200);
            let body: Vec<(u32, Event)> = track.events.iter().copied().filter(|&(tick, event)| tick >= start && event != Event::LoopPoint).collect();
            let mut events = track.events.clone();
            for time in 1..=again {
                events.extend(body.iter().map(|&(tick, event)| (tick + time * period, event)));
            }
            Track { channel: track.channel, events, length: track.length + again * period }
        })
        .collect();
    Song { ticks_per_quarter: song.ticks_per_quarter, tracks }
}

/// How long `song` plays once through, in seconds; whether it then goes back
/// to a loop point (and plays on, again and again).
pub fn once_length(song: &Song) -> (f32, bool) {
    let tempo = TempoMap::new(song);
    let length = song.tracks.iter().map(|t| t.length).max().unwrap_or(0);
    let loops = song.tracks.iter().any(|track| track.events.iter().any(|(_, event)| *event == Event::LoopPoint));
    (tempo.seconds(length) as f32, loops)
}

/// How long `song` plays, in seconds, `max_seconds` at most (a song that
/// loops plays them all): as long as [`render`] has its notes play.
pub fn length(song: &Song, max_seconds: f32) -> f32 {
    let song = &unrolled(song, max_seconds);
    let tempo = TempoMap::new(song);
    let length = song.tracks.iter().map(|t| t.length).max().unwrap_or(0);
    (tempo.seconds(length) as f32).min(max_seconds)
}

/// Renders up to `max_seconds` of `song` as mono 16-bit PCM at
/// [`OUTPUT_RATE`], going back to its loop point as often as it takes.
pub fn render(song: &Song, banks: &Banks, max_seconds: f32) -> Vec<i16> {
    let song = &unrolled(song, max_seconds);
    let tempo = TempoMap::new(song);
    let length = song.tracks.iter().map(|t| t.length).max().unwrap_or(0);
    let played = (tempo.seconds(length) as f32).min(max_seconds);
    let mut mix = vec![0f32; ((played + TAIL).min(max_seconds) * OUTPUT_RATE as f32) as usize];
    let mut waves: HashMap<(usize, u16), Option<Wave>> = HashMap::new();

    for track in &song.tracks {
        let controls = Controls::new(&track.events, &tempo);
        let (mut bank, mut high, mut low) = (banks.own, None, None);
        let (mut program, mut changed) = (None, [None; 8]);
        for &(tick, event) in &track.events {
            let (key, velocity, duration) = match event {
                Event::Bank { high: h, low: l } => {
                    (high, low) = (h.or(high), l.or(low));
                    if let (Some(high), Some(low)) = (high, low) {
                        bank = banks.named.get(&[low, high]).unwrap_or(banks.own);
                    }
                    continue;
                }
                Event::Program(id) => {
                    (program, changed) = (Some(id as u16), [None; 8]);
                    continue;
                }
                Event::Envelope { part, value } => {
                    changed[(part & 7) as usize] = Some(value);
                    continue;
                }
                Event::Note { key, velocity, duration } => (key, velocity, duration),
                _ => continue,
            };
            let Some(program) = program.and_then(|id| bank.programs.get(&id)) else { continue };
            let Some(split) = program.split(key) else {
                continue;
            };
            let wave = waves.entry((bank as *const Bank as usize, split.sample)).or_insert_with(|| {
                // The console's own waves have no data; a sample's is in
                // the bank itself or, when it has none, in the main one.
                if let Some(info) = bank.samples.get(&split.sample) {
                    let rate = info.rate as f64 * 2f64.powf(info.fine_tune as f64 / 255.0 / 12.0) / 16.0;
                    match info.format {
                        SampleFormat::Psg => return Some(Wave { source: Source::Square((info.duty.min(7) + 1) as f32 / 8.0), rate }),
                        SampleFormat::Noise => return Some(Wave { source: Source::Noise, rate }),
                        _ => {}
                    }
                }
                let holder = if bank.pcm.is_empty() { banks.main } else { bank };
                let info = holder.samples.get(&split.sample)?;
                let (pcm, loop_start) = holder.decode(split.sample, &holder.pcm)?;
                (pcm.len() > 1).then_some(Wave { source: Source::Sample { pcm, loop_start, looped: info.looped }, rate: info.rate as f64 })
            });
            let Some(wave) = wave else { continue };

            let start = tempo.seconds(tick);
            let held = (tempo.seconds(tick + duration) - start) as f32;
            let scale = |v: i32| (v.clamp(0, 127) as f32) / 127.0;
            let gain = banks.gain * NOTE_GAIN * scale(velocity as i32) * scale(split.volume as i32) * scale(program.volume as i32);
            // The sample plays at its own rate on its root key.
            let step = 2f64.powf((key as f64 - split.root_key as f64) / 12.0) * wave.rate / OUTPUT_RATE as f64;
            let shape = Shape::new(&split.envelope, &changed);
            play_note(&mut mix, wave, &controls, &shape, start, held, step, gain);
        }
    }
    // What rings after the end is kept as long as it is heard.
    let end = (played * OUTPUT_RATE as f32) as usize;
    let heard = mix.iter().rposition(|sample| sample.abs() > 0.0005).map_or(0, |last| last + 1);
    mix.truncate(heard.max(end.min(mix.len())));
    // Soft limit, then 16 bits.
    mix.iter().map(|&s| (s.tanh() * 32767.0) as i16).collect()
}

/// Adds one note to `mix`: the wave played `step` samples at a time from
/// `start` seconds, looping if it loops, held for `held` seconds and then
/// let go, bent and made louder or softer as its track says.
fn play_note(mix: &mut [f32], wave: &Wave, controls: &Controls, shape: &Shape, start: f64, held: f32, step: f64, gain: f32) {
    let rate = OUTPUT_RATE as f64;
    let first = (start * rate) as usize;
    let total = ((held + shape.release) as f64 * rate) as usize;
    let level_at = |n: usize| {
        let (pitch, volume) = controls.at(start + n as f64 / rate);
        (pitch, gain * volume * shape.level(n as f32 / rate as f32, held))
    };
    let (mut pitch, mut level) = level_at(0);
    let mut slope = 0.0;
    let (mut position, mut clock, mut noise, mut up) = (0f64, 0f64, 0x7FFFu16, true);
    for n in 0..total {
        let Some(out) = mix.get_mut(first + n) else {
            return;
        };
        if n % CONTROL == 0 {
            let (next_pitch, next) = level_at(n + CONTROL);
            (pitch, slope) = (next_pitch, (next - level) / CONTROL as f32);
        }
        let value = match &wave.source {
            Source::Sample { pcm, loop_start, looped } => {
                let index = position as usize;
                let Some(&a) = pcm.get(index) else { return };
                let b = match pcm.get(index + 1) {
                    Some(&b) => b,
                    None if *looped => pcm[(*loop_start).min(pcm.len() - 1)],
                    None => a,
                };
                let fraction = (position - index as f64) as f32;
                (a as f32 + (b as f32 - a as f32) * fraction) / 32768.0
            }
            Source::Square(duty) => match ((position / 8.0).fract() as f32) < *duty {
                true => WAVE_LEVEL,
                false => -WAVE_LEVEL,
            },
            Source::Noise => match up {
                true => WAVE_LEVEL,
                false => -WAVE_LEVEL,
            },
        };
        *out += value * level;
        level += slope;
        position += step * pitch;
        match &wave.source {
            Source::Sample { pcm, loop_start, looped } if position >= pcm.len() as f64 => {
                let loop_length = pcm.len().saturating_sub(*loop_start);
                if !looped || loop_length < 1 {
                    return;
                }
                position = *loop_start as f64 + (position - pcm.len() as f64) % loop_length as f64;
            }
            // The console's noise: a register shifted at each step.
            Source::Noise => {
                clock += step * pitch;
                let mut steps = 0;
                while clock >= 1.0 && steps < 64 {
                    (clock, steps) = (clock - 1.0, steps + 1);
                    up = noise & 1 == 0;
                    noise >>= 1;
                    if !up {
                        noise ^= 0x6000;
                    }
                }
                clock = clock.fract();
            }
            _ => {}
        }
    }
}

/// Wraps mono 16-bit PCM into a WAV file held in memory, for the audio
/// backend.
pub fn wav_bytes(pcm: &[i16]) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + pcm.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    // PCM, one channel.
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&OUTPUT_RATE.to_le_bytes());
    wav.extend_from_slice(&(OUTPUT_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in pcm {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::{Program, SampleInfo, Split, Track};

    /// A one-sample bank: a looping 8-bit square wave.
    fn square_bank() -> Bank {
        let mut bank = Bank { pcm: [[100u8; 16], [156u8; 16]].concat(), ..Bank::default() };
        bank.samples.insert(
            7,
            SampleInfo {
                id: 7,
                format: SampleFormat::Pcm8,
                looped: true,
                rate: OUTPUT_RATE,
                position: 0,
                loop_start: 0,
                loop_length: 32,
                coarse_tune: 0,
                fine_tune: 0,
                root_key: 60,
                volume: 127,
                duty: 0,
            },
        );
        let split = Split {
            low_key: 0,
            high_key: 127,
            sample: 7,
            coarse_tune: 0,
            fine_tune: 0,
            root_key: 60,
            volume: 127,
            pan: 64,
            envelope: Envelope { on: false, multiplier: 1, first: 0, attack: 0, hold: 0, decay: 0, sustain: 127, fade: 127, release: 0 },
        };
        bank.programs.insert(1, Program { volume: 127, splits: vec![split] });
        bank
    }

    #[test]
    fn a_note_sounds_only_while_it_plays() {
        // 120 bpm, 48 ticks per beat: 96 ticks = 1 second.
        let events = vec![(0, Event::Tempo(120)), (0, Event::Program(1)), (96, Event::Note { key: 60, velocity: 127, duration: 96 })];
        let song = Song { ticks_per_quarter: 48, tracks: vec![Track { channel: 0, events, length: 288 }] };
        let bank = square_bank();
        let pcm = render(&song, &Banks { own: &bank, main: &bank, named: &HashMap::new(), gain: 1.0 }, 10.0);
        let second = OUTPUT_RATE as usize;
        assert_eq!(pcm.len(), 3 * second);
        let loudest = |range: std::ops::Range<usize>| pcm[range].iter().map(|s| s.abs()).max().unwrap();
        assert_eq!(loudest(0..second), 0);
        assert!(loudest(second + 1000..2 * second) > 3000);
        assert_eq!(loudest(2 * second + second / 2..3 * second), 0);

        let wav = wav_bytes(&pcm);
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + pcm.len() * 2);
    }
}
