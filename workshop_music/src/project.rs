//! A song: its tracks (an instrument, a drum kit or audio clips each), their regions, their effects, filter,
//! sends and sidechain, any of their settings drawn along the song, two shared buses, the master; written as
//! text (`library/music/<word>.music`), played into two channels.

use std::collections::HashMap;
use std::sync::Arc;

use workshop_sound::{self as sound, Biquad, Mix, Note, Sound};

/// The speakers' rate.
pub(crate) const RATE: f32 = shell::audio::OUTPUT_RATE as f32;

/// What a track plays: an instrument (each note at its pitch), a drum kit (each row a drum), or sounds as they
/// are (audio clips).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    Instrument,
    Drums,
    Audio,
}

/// A sound laid in an audio region: which (one of the project's own), from where in it (s), how loud, how many
/// half tones higher (faster), faded in and out (s), backwards.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Clip {
    pub(crate) sound: String,
    pub(crate) offset: f32,
    pub(crate) gain: f32,
    pub(crate) tone: f32,
    pub(crate) fade_in: f32,
    pub(crate) fade_out: f32,
    pub(crate) reverse: bool,
}

impl Clip {
    pub(crate) fn of(sound: &str) -> Clip {
        Clip { sound: sound.to_string(), offset: 0.0, gain: 1.0, tone: 0.0, fade_in: 0.0, fade_out: 0.01, reverse: false }
    }
}

/// A region of a track: where it starts and how long it is (beats), its notes (from its start), the regions it is
/// linked to (the same number, not 0: their notes are one), its sound (an audio track's).
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Region {
    pub(crate) start: f32,
    pub(crate) length: f32,
    pub(crate) notes: Vec<Note>,
    pub(crate) link: u32,
    pub(crate) clip: Option<Clip>,
}

impl Region {
    pub(crate) fn empty(start: f32, length: f32) -> Region {
        Region { start, length, notes: Vec::new(), link: 0, clip: None }
    }
}

/// What a track's drawn lane moves.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(crate) enum Target {
    Volume,
    Pan,
    Cutoff,
    SendA,
    SendB,
}

impl Target {
    pub(crate) const ALL: [Target; 5] = [Target::Volume, Target::Pan, Target::Cutoff, Target::SendA, Target::SendB];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Target::Volume => "Volume",
            Target::Pan => "Pan",
            Target::Cutoff => "Filter",
            Target::SendA => "Reverb Send",
            Target::SendB => "Echo Send",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Target::Volume => "volume",
            Target::Pan => "pan",
            Target::Cutoff => "cutoff",
            Target::SendA => "send_a",
            Target::SendB => "send_b",
        }
    }

    /// A drawn value (0 to 1) as what it moves: a factor, a side, Hz, a send.
    pub(crate) fn value(self, drawn: f32) -> f32 {
        let drawn = drawn.clamp(0.0, 1.0);
        match self {
            Target::Volume => drawn * 2.0,
            Target::Pan => drawn * 2.0 - 1.0,
            Target::Cutoff => 20.0 * 1000f32.powf(drawn),
            Target::SendA | Target::SendB => drawn,
        }
    }

    /// What a drawn value is, said.
    pub(crate) fn said(self, drawn: f32) -> String {
        let value = self.value(drawn);
        match self {
            Target::Volume => format!("×{value:.2}"),
            Target::Pan => format!("{value:+.2}"),
            Target::Cutoff => format!("{value:.0} Hz"),
            Target::SendA | Target::SendB => format!("{value:.2}"),
        }
    }
}

/// A track: its name, what it plays, how loud, from which side, muted or alone, its effects, its filter
/// (cutoff Hz, resonance), its sends to the two buses, the track that squeezes it (its number from 1; 0: none)
/// and how much, its regions, its lanes drawn along the song (points from 0 to 1), its colour.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Track {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) instrument: Sound,
    pub(crate) kit: Vec<(String, Sound)>,
    pub(crate) volume: f32,
    pub(crate) pan: f32,
    pub(crate) muted: bool,
    pub(crate) solo: bool,
    pub(crate) fx: Mix,
    pub(crate) cutoff: f32,
    pub(crate) resonance: f32,
    pub(crate) send_a: f32,
    pub(crate) send_b: f32,
    pub(crate) sidechain: usize,
    pub(crate) duck: f32,
    pub(crate) regions: Vec<Region>,
    pub(crate) lanes: Vec<(Target, Vec<(f32, f32)>)>,
    pub(crate) colour: [u8; 3],
}

const COLOURS: [[u8; 3]; 8] = [
    [0xA7, 0x8B, 0xFA],
    [0xE0, 0xA9, 0x3A],
    [0x3B, 0xA7, 0xD9],
    [0x6C, 0xC6, 0x6C],
    [0xF0, 0x71, 0x71],
    [0xF5, 0x9E, 0xC8],
    [0x8A, 0xA4, 0xFF],
    [0xD9, 0xC8, 0x6A],
];

impl Track {
    /// A track of an instrument of that kind (`sound::INSTRUMENTS`).
    pub(crate) fn instrument(kind: &str, at: usize) -> Track {
        Track {
            name: kind.to_string(),
            kind: Kind::Instrument,
            instrument: sound::instrument(kind),
            kit: Vec::new(),
            volume: 0.5,
            pan: 0.0,
            muted: false,
            solo: false,
            fx: Mix::default(),
            cutoff: 20_000.0,
            resonance: 0.0,
            send_a: 0.0,
            send_b: 0.0,
            sidechain: 0,
            duck: 0.6,
            regions: Vec::new(),
            lanes: Vec::new(),
            colour: COLOURS[at % COLOURS.len()],
        }
    }

    /// A track of drums (the kit synthesized).
    pub(crate) fn drums(at: usize) -> Track {
        Track { name: "Drums".to_string(), kind: Kind::Drums, kit: sound::drum_kit(), volume: 0.65, ..Track::instrument("Chiptune", at) }
    }

    /// A track of sounds as they are.
    pub(crate) fn audio(at: usize) -> Track {
        Track { name: "Audio".to_string(), kind: Kind::Audio, volume: 0.8, ..Track::instrument("Chiptune", at) }
    }

    /// Where its last region ends (beats).
    pub(crate) fn end(&self) -> f32 {
        self.regions.iter().map(|region| region.start + region.length).fold(0.0, f32::max)
    }

    /// Its points drawn for a target (none: empty).
    pub(crate) fn lane(&self, target: Target) -> &[(f32, f32)] {
        self.lanes.iter().find(|(own, _)| *own == target).map_or(&[], |(_, points)| points.as_slice())
    }

    pub(crate) fn lane_mut(&mut self, target: Target) -> &mut Vec<(f32, f32)> {
        if let Some(at) = self.lanes.iter().position(|(own, _)| *own == target) {
            return &mut self.lanes[at].1;
        }
        self.lanes.push((target, Vec::new()));
        &mut self.lanes.last_mut().expect("just put").1
    }

    /// What a target is at a beat: as drawn, else as set.
    pub(crate) fn at(&self, target: Target, beat: f32) -> f32 {
        let points = self.lane(target);
        if points.is_empty() {
            return match target {
                Target::Volume => 1.0,
                Target::Pan => self.pan,
                Target::Cutoff => self.cutoff,
                Target::SendA => self.send_a,
                Target::SendB => self.send_b,
            };
        }
        let drawn = match points.iter().position(|(at, _)| *at > beat) {
            Some(0) => points[0].1,
            Some(after) => {
                let ((a, va), (b, vb)) = (points[after - 1], points[after]);
                va + (vb - va) * ((beat - a) / (b - a).max(1e-6))
            }
            None => points[points.len() - 1].1,
        };
        target.value(drawn)
    }
}

/// A song: its tempo (beats a minute), its beats a bar and bars, its loop, how much its off sixteenths swing,
/// its tracks, its two buses (a reverb, an echo), its master and how loud it is.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Project {
    pub(crate) tempo: f32,
    pub(crate) beats_per_bar: f32,
    pub(crate) bars: f32,
    pub(crate) looping: bool,
    pub(crate) loop_start: f32,
    pub(crate) loop_end: f32,
    pub(crate) swing: f32,
    pub(crate) tracks: Vec<Track>,
    pub(crate) bus_a: Mix,
    pub(crate) bus_b: Mix,
    pub(crate) master: Mix,
    pub(crate) master_volume: f32,
}

/// Two channels of samples.
type Channels = (Vec<f32>, Vec<f32>);

/// The notes of an instrument already played, by their tone, length and force (made again when it changes); the
/// sounds of the audio clips read.
#[derive(Default)]
pub(crate) struct NoteCache {
    made_of: Option<(Sound, Vec<(String, Sound)>)>,
    notes: HashMap<(i32, u32, u16), Arc<Channels>>,
    sounds: HashMap<String, Option<Arc<Channels>>>,
}

/// A song played: its two channels, how loud each track went.
pub(crate) struct Rendered {
    pub(crate) left: Vec<f32>,
    pub(crate) right: Vec<f32>,
    pub(crate) peaks: Vec<f32>,
}

/// A note, as a region keeps it.
fn note(start: f32, length: f32, tone: f32, velocity: f32) -> Note {
    Note { start, length, tone, velocity }
}

use workshop_base::own_sounds::OwnSounds;

/// One of the project's own sounds, in two channels (none: not found).
pub(crate) fn sound_of(own: &OwnSounds, word: &str) -> Option<Channels> {
    let stereo = shell::own_sounds::read(&own.data, &own.library, word)?;
    Some((
        stereo.iter().step_by(2).map(|value| f32::from(*value) / 32768.0).collect(),
        stereo.iter().skip(1).step_by(2).map(|value| f32::from(*value) / 32768.0).collect(),
    ))
}

impl Project {
    /// A song with nothing in it but a drum track and a piano.
    pub(crate) fn empty() -> Project {
        Project {
            tempo: 110.0,
            beats_per_bar: 4.0,
            bars: 8.0,
            looping: false,
            loop_start: 0.0,
            loop_end: 16.0,
            swing: 0.0,
            tracks: vec![Track::drums(0), Track::instrument("Electric Piano", 1)],
            bus_a: Mix { room: 0.85, room_mix: 1.0, ..Mix::default() },
            bus_b: Mix { echo: 0.41, feedback: 0.45, echo_mix: 1.0, ping_pong: true, ..Mix::default() },
            master: Mix { limit: true, compress: 0.2, ..Mix::default() },
            master_volume: 1.0,
        }
    }

    /// A song to begin from: a beat, a bass squeezed by the kick, chords in the reverb, a melody in the echo,
    /// four bars looped.
    pub(crate) fn demo() -> Project {
        let mut song = Project { looping: true, loop_end: 16.0, ..Project::empty() };
        let mut beat = Vec::new();
        for bar in 0..4 {
            let at = bar as f32 * 4.0;
            beat.extend([note(at, 0.25, 0.0, 1.0), note(at + 2.0, 0.25, 0.0, 1.0), note(at + 2.5, 0.25, 0.0, 0.7)]);
            beat.extend([note(at + 1.0, 0.25, 1.0, 1.0), note(at + 3.0, 0.25, 1.0, 1.0)]);
            for eighth in 0..8 {
                beat.push(note(at + eighth as f32 * 0.5, 0.25, 3.0, if eighth % 2 == 0 { 0.8 } else { 0.5 }));
            }
        }
        song.tracks[0].regions.push(Region { notes: beat, ..Region::empty(0.0, 16.0) });
        // (Chords: C, A minor, F, G.)
        let chords = [[0.0, 4.0, 7.0], [-3.0, 0.0, 4.0], [-7.0, -3.0, 0.0], [-5.0, -1.0, 2.0]];
        let mut keys = Vec::new();
        for (bar, chord) in chords.iter().enumerate() {
            for tone in chord {
                keys.push(note(bar as f32 * 4.0, 3.8, *tone, 0.7));
            }
        }
        song.tracks[1].regions.push(Region { notes: keys, ..Region::empty(0.0, 16.0) });
        song.tracks[1].send_a = 0.35;
        let mut bass = Track::instrument("Synth Bass", 2);
        let roots = [0.0, -3.0, -7.0, -5.0];
        bass.regions.push(Region {
            notes: roots
                .iter()
                .enumerate()
                .flat_map(|(bar, root)| {
                    [
                        note(bar as f32 * 4.0, 1.5, *root, 1.0),
                        note(bar as f32 * 4.0 + 2.0, 1.0, *root, 0.8),
                        note(bar as f32 * 4.0 + 3.0, 0.75, root + 12.0, 0.7),
                    ]
                })
                .collect(),
            ..Region::empty(0.0, 16.0)
        });
        (bass.sidechain, bass.duck) = (1, 0.5);
        let mut lead = Track::instrument("Square Lead", 3);
        lead.send_b = 0.3;
        let tune = [
            (0.0, 12.0),
            (1.0, 16.0),
            (2.0, 19.0),
            (3.5, 16.0),
            (4.0, 12.0),
            (5.0, 9.0),
            (6.0, 12.0),
            (8.0, 5.0),
            (9.0, 9.0),
            (10.0, 12.0),
            (12.0, 7.0),
            (13.0, 11.0),
            (14.0, 14.0),
        ];
        lead.regions.push(Region { notes: tune.iter().map(|(at, tone)| note(*at, 0.9, *tone, 0.8)).collect(), ..Region::empty(0.0, 16.0) });
        song.tracks.extend([bass, lead]);
        song
    }

    /// How long a beat lasts (s).
    pub(crate) fn beat(&self) -> f32 {
        60.0 / self.tempo.max(1.0)
    }

    /// Where the song ends (beats): its bars, or its last region.
    pub(crate) fn end(&self) -> f32 {
        let regions = self.tracks.iter().map(Track::end).fold(0.0, f32::max);
        (self.bars * self.beats_per_bar).max(regions)
    }

    /// Played from a beat to another: each track by itself (at once, on as many cores), its notes and sounds, its
    /// filter, its effects; then each squeezed by its sidechain, mixed at its volume and side as drawn, sent to the
    /// buses; the buses' effects, the master's; a click at each beat if asked.
    pub(crate) fn render(&self, caches: &mut Vec<NoteCache>, from: f32, to: f32, click: bool, own: &OwnSounds) -> Rendered {
        caches.resize_with(self.tracks.len(), NoteCache::default);
        let alone = self.tracks.iter().any(|track| track.solo);
        let parts: Vec<Option<Channels>> = std::thread::scope(|scope| {
            let handles: Vec<_> = self
                .tracks
                .iter()
                .zip(caches.iter_mut())
                .enumerate()
                .map(|(at, (track, cache))| {
                    // (A track heard, or one that squeezes another.)
                    let wanted = (!track.muted && (!alone || track.solo)) || self.tracks.iter().any(|other| other.sidechain == at + 1);
                    scope.spawn(move || wanted.then(|| self.track_audio(track, cache, from, to, own)))
                })
                .collect();
            handles.into_iter().map(|handle| handle.join().ok().flatten()).collect()
        });
        let beat = self.beat();
        let span = ((to - from).max(0.0) * beat * RATE) as usize;
        let length = parts.iter().flatten().map(|(left, _)| left.len()).max().unwrap_or(0).max(span);
        let (mut left, mut right) = (vec![0.0_f32; length], vec![0.0_f32; length]);
        let (mut a_left, mut a_right, mut b_left, mut b_right) = (vec![0.0_f32; length], vec![0.0_f32; length], vec![0.0_f32; length], vec![0.0_f32; length]);
        let mut peaks = vec![0.0_f32; self.tracks.len()];
        // (How loud each track goes, followed: what squeezes another.)
        let followed: Vec<Option<Vec<f32>>> = parts
            .iter()
            .enumerate()
            .map(|(at, part)| {
                let squeezes = self.tracks.iter().any(|other| other.sidechain == at + 1);
                part.as_ref().filter(|_| squeezes).map(|(l, r)| {
                    let (attack, release) = ((-1.0 / (0.001 * RATE)).exp(), (-1.0 / (0.15 * RATE)).exp());
                    let mut level = 0.0_f32;
                    l.iter()
                        .zip(r)
                        .map(|(l, r)| {
                            let now = l.abs().max(r.abs());
                            level = if now > level { attack * level + (1.0 - attack) * now } else { release * level + (1.0 - release) * now };
                            level
                        })
                        .collect()
                })
            })
            .collect();
        for (at, (track, part)) in self.tracks.iter().zip(&parts).enumerate() {
            let Some((own_left, own_right)) = part else { continue };
            if track.muted || (alone && !track.solo) {
                continue;
            }
            let squeezer = track.sidechain.checked_sub(1).and_then(|source| followed.get(source)).and_then(Option::as_ref);
            let drawn = !track.lane(Target::Volume).is_empty()
                || !track.lane(Target::Pan).is_empty()
                || !track.lane(Target::SendA).is_empty()
                || !track.lane(Target::SendB).is_empty();
            let mut settings = (1.0, track.pan, track.send_a, track.send_b);
            for (sample, (l, r)) in own_left.iter().zip(own_right).enumerate() {
                if drawn && sample % 64 == 0 {
                    let now = from + sample as f32 / RATE / beat;
                    settings = (track.at(Target::Volume, now), track.at(Target::Pan, now), track.at(Target::SendA, now), track.at(Target::SendB, now));
                }
                let (gain, pan, send_a, send_b) = settings;
                let squeezed = squeezer.and_then(|levels| levels.get(sample)).map_or(1.0, |level| 1.0 - track.duck.clamp(0.0, 1.0) * (level * 3.0).min(1.0));
                let gain = gain.max(0.0) * track.volume * squeezed;
                let (l, r) = (l * (1.0 - pan).min(1.0) * gain, r * (1.0 + pan).min(1.0) * gain);
                left[sample] += l;
                right[sample] += r;
                a_left[sample] += l * send_a;
                a_right[sample] += r * send_a;
                b_left[sample] += l * send_b;
                b_right[sample] += r * send_b;
                peaks[at] = peaks[at].max(l.abs()).max(r.abs());
            }
        }
        // (The buses: what was sent to them, through their effects, back in.)
        for (bus, bus_left, bus_right) in [(&self.bus_a, a_left, a_right), (&self.bus_b, b_left, b_right)] {
            if bus_left.iter().chain(&bus_right).all(|sample| *sample == 0.0) {
                continue;
            }
            let (wet_left, wet_right) = sound::mixed(bus, bus_left, bus_right);
            if wet_left.len() > left.len() {
                left.resize(wet_left.len(), 0.0);
                right.resize(wet_left.len(), 0.0);
            }
            for (at, (l, r)) in wet_left.iter().zip(&wet_right).enumerate() {
                left[at] += l;
                right[at] += r;
            }
        }
        // (The click before the master, so its limiter holds it too.)
        if click {
            let mut beat_at = from.ceil();
            while beat_at < to {
                let start = ((beat_at - from) * beat * RATE) as usize;
                let pitch = if (beat_at % self.beats_per_bar.max(1.0)).abs() < 0.01 { 1760.0 } else { 1320.0 };
                for at in 0..(0.03 * RATE) as usize {
                    if let (Some(l), Some(r)) = (left.get_mut(start + at), right.get_mut(start + at)) {
                        let value = (std::f32::consts::TAU * pitch * at as f32 / RATE).sin() * 0.25 * (1.0 - at as f32 / (0.03 * RATE));
                        *l += value;
                        *r += value;
                    }
                }
                beat_at += 1.0;
            }
        }
        for sample in left.iter_mut().chain(right.iter_mut()) {
            *sample *= self.master_volume.max(0.0);
        }
        let (mut left, mut right) = sound::mixed(&self.master, left, right);
        // (What is silent past the end, taken off.)
        let keep = left.iter().zip(&right).rposition(|(l, r)| l.abs() > 1e-4 || r.abs() > 1e-4).map_or(0, |at| at + 1).max(span.min(left.len()));
        left.truncate(keep);
        right.truncate(keep);
        Rendered { left, right, peaks }
    }

    /// A track played from a beat to another: its notes (swung) or sounds, its filter (as drawn), its effects.
    fn track_audio(&self, track: &Track, cache: &mut NoteCache, from: f32, to: f32, own: &OwnSounds) -> Channels {
        let beat = self.beat();
        let length = ((to - from).max(0.0) * beat * RATE) as usize + (4.0 * RATE) as usize;
        let (mut left, mut right) = (vec![0.0_f32; length], vec![0.0_f32; length]);
        let made_of = (track.instrument.clone(), track.kit.clone());
        if cache.made_of.as_ref() != Some(&made_of) {
            cache.notes.clear();
            cache.made_of = Some(made_of);
        }
        let mut lay = |first: f32, audio: &Channels| {
            let first = (first * RATE) as isize;
            for (offset, (l, r)) in audio.0.iter().zip(&audio.1).enumerate() {
                let at = first + offset as isize;
                if at >= 0 && (at as usize) < length {
                    left[at as usize] += l;
                    right[at as usize] += r;
                }
            }
        };
        for region in &track.regions {
            if let (Kind::Audio, Some(clip)) = (track.kind, &region.clip) {
                // (A sound laid: from where it starts in the song, or partly if it began before.)
                let (start, end) = (region.start, region.start + region.length);
                if end <= from || start >= to {
                    continue;
                }
                let read = cache.sounds.entry(clip.sound.clone()).or_insert_with(|| sound_of(own, &clip.sound).map(Arc::new)).clone();
                if let Some(sound) = read {
                    let made = clipped(&sound, clip, region.length * beat);
                    lay((start - from) * beat, &made);
                }
                continue;
            }
            for note in region.notes.iter().filter(|note| note.start >= 0.0 && note.start < region.length) {
                // (Swing: the sixteenths off the beat, a little late.)
                let sixteenth = (note.start * 4.0).round();
                let swung = if (note.start * 4.0 - sixteenth).abs() < 0.01 && sixteenth as i64 % 2 == 1 { self.swing.clamp(0.0, 1.0) * 0.125 } else { 0.0 };
                let at = region.start + note.start + swung;
                if at < from || at >= to {
                    continue;
                }
                let held = note.length.min(region.length - note.start).max(0.05);
                let key = match track.kind {
                    Kind::Drums => (note.tone.round() as i32, 0, (note.velocity * 100.0).round() as u16),
                    _ => ((note.tone * 10.0).round() as i32, (held * beat * 1000.0).round() as u32, (note.velocity * 100.0).round() as u16),
                };
                let audio = cache
                    .notes
                    .entry(key)
                    .or_insert_with(|| {
                        Arc::new(match track.kind {
                            Kind::Drums => track
                                .kit
                                .get(note.tone.round().max(0.0) as usize)
                                .map_or((Vec::new(), Vec::new()), |(_, sound)| sound::one_shot(sound, note.velocity, own)),
                            _ => sound::note_audio(&track.instrument, note.tone, held * beat, note.velocity, own),
                        })
                    })
                    .clone();
                lay((at - from) * beat, &audio);
            }
        }
        // (The track's filter: a resonant low-pass, its cutoff as drawn.)
        let drawn = !track.lane(Target::Cutoff).is_empty();
        if drawn || track.cutoff < 19_900.0 || track.resonance > 0.0 {
            let (mut on_left, mut on_right) = (Biquad::default(), Biquad::default());
            for at in 0..length {
                if at % 32 == 0 {
                    let cutoff = if drawn { track.at(Target::Cutoff, from + at as f32 / RATE / beat) } else { track.cutoff };
                    let q = 0.707 + track.resonance.clamp(0.0, 0.95) * 8.0;
                    on_left.set(0, cutoff.clamp(20.0, 19_000.0), q, 0.0, RATE);
                    on_right.set(0, cutoff.clamp(20.0, 19_000.0), q, 0.0, RATE);
                }
                left[at] = on_left.run(left[at]).clamp(-4.0, 4.0);
                right[at] = on_right.run(right[at]).clamp(-4.0, 4.0);
            }
        }
        sound::mixed(&track.fx, left, right)
    }

    /// Written as `key=value` lines.
    pub(crate) fn written(&self) -> String {
        let mut said = format!(
            "music=1\ntempo={}\nmeter={}\nbars={}\nloop={}:{}:{}\nswing={}\nmaster_volume={}\ntracks={}\n",
            self.tempo,
            self.beats_per_bar,
            self.bars,
            u8::from(self.looping),
            self.loop_start,
            self.loop_end,
            self.swing,
            self.master_volume,
            self.tracks.len()
        );
        said.push_str(&sound::mix_written(&self.master, "master."));
        said.push_str(&sound::mix_written(&self.bus_a, "bus_a."));
        said.push_str(&sound::mix_written(&self.bus_b, "bus_b."));
        for (at, track) in self.tracks.iter().enumerate() {
            let prefix = format!("track{at}.");
            said.push_str(&format!(
                "{prefix}name={}\n{prefix}kind={}\n{prefix}volume={}\n{prefix}pan={}\n{prefix}muted={}\n{prefix}solo={}\n{prefix}colour={},{},{}\n{prefix}cutoff={}\n{prefix}resonance={}\n{prefix}send_a={}\n{prefix}send_b={}\n{prefix}sidechain={}\n{prefix}duck={}\n",
                track.name.replace('\n', " "),
                match track.kind {
                    Kind::Drums => "drums",
                    Kind::Audio => "audio",
                    Kind::Instrument => "instrument",
                },
                track.volume,
                track.pan,
                u8::from(track.muted),
                u8::from(track.solo),
                track.colour[0],
                track.colour[1],
                track.colour[2],
                track.cutoff,
                track.resonance,
                track.send_a,
                track.send_b,
                track.sidechain,
                track.duck
            ));
            said.push_str(&sound::mix_written(&track.fx, &format!("{prefix}fx.")));
            match track.kind {
                Kind::Instrument => {
                    for line in track.instrument.written().lines() {
                        said.push_str(&format!("{prefix}instrument.{line}\n"));
                    }
                }
                Kind::Drums => {
                    said.push_str(&format!("{prefix}kit={}\n", track.kit.len()));
                    for (number, (name, sound)) in track.kit.iter().enumerate() {
                        said.push_str(&format!("{prefix}kit{number}={}\n", name.replace('\n', " ")));
                        for line in sound.written().lines() {
                            said.push_str(&format!("{prefix}kit{number}.{line}\n"));
                        }
                    }
                }
                Kind::Audio => {}
            }
            said.push_str(&format!("{prefix}regions={}\n", track.regions.len()));
            for (number, region) in track.regions.iter().enumerate() {
                let notes: Vec<String> = region.notes.iter().map(|note| format!("{}:{}:{}:{}", note.start, note.length, note.tone, note.velocity)).collect();
                said.push_str(&format!(
                    "{prefix}region{number}={}:{}:{}\n{prefix}region{number}.notes={}\n",
                    region.start,
                    region.length,
                    region.link,
                    notes.join(";")
                ));
                if let Some(clip) = &region.clip {
                    said.push_str(&format!(
                        "{prefix}region{number}.clip={}|{}|{}|{}|{}|{}|{}\n",
                        clip.sound,
                        clip.offset,
                        clip.gain,
                        clip.tone,
                        clip.fade_in,
                        clip.fade_out,
                        u8::from(clip.reverse)
                    ));
                }
            }
            for (target, points) in &track.lanes {
                if !points.is_empty() {
                    let points: Vec<String> = points.iter().map(|(at, value)| format!("{at}:{value}")).collect();
                    said.push_str(&format!("{prefix}auto.{}={}\n", target.key(), points.join(";")));
                }
            }
        }
        said
    }

    /// Read back; none if it is no song.
    pub(crate) fn read(text: &str) -> Option<Project> {
        let pairs: HashMap<String, String> =
            text.lines().filter_map(|line| line.split_once('=')).map(|(key, value)| (key.trim().to_string(), value.trim().to_string())).collect();
        pairs.get("music")?;
        let number = |key: &str, otherwise: f32| pairs.get(key).and_then(|value| value.parse::<f32>().ok()).unwrap_or(otherwise);
        let list = |key: &str| -> Vec<Vec<f32>> {
            pairs.get(key).map_or(Vec::new(), |said| {
                said.split(';')
                    .filter(|part| !part.trim().is_empty())
                    .map(|part| part.split(':').filter_map(|value| value.trim().parse().ok()).collect())
                    .collect()
            })
        };
        let nested = |prefix: &str| -> Sound {
            let lines: Vec<&str> = text.lines().filter_map(|line| line.strip_prefix(prefix)).collect();
            Sound::read(&lines.join("\n"))
        };
        let looped = list("loop").into_iter().next().unwrap_or_default();
        let empty = Project::empty();
        let has = |prefix: &str| pairs.keys().any(|key| key.starts_with(prefix));
        let mut song = Project {
            tempo: number("tempo", 110.0),
            beats_per_bar: number("meter", 4.0),
            bars: number("bars", 8.0),
            looping: looped.first().is_some_and(|on| *on > 0.5),
            loop_start: looped.get(1).copied().unwrap_or(0.0),
            loop_end: looped.get(2).copied().unwrap_or(16.0),
            swing: number("swing", 0.0),
            tracks: Vec::new(),
            bus_a: if has("bus_a.") { sound::mix_read(&pairs, "bus_a.") } else { empty.bus_a },
            bus_b: if has("bus_b.") { sound::mix_read(&pairs, "bus_b.") } else { empty.bus_b },
            master: sound::mix_read(&pairs, "master."),
            master_volume: number("master_volume", 1.0),
        };
        for at in 0..number("tracks", 0.0) as usize {
            let prefix = format!("track{at}.");
            let own = |key: &str| pairs.get(&format!("{prefix}{key}"));
            let own_number = |key: &str, otherwise: f32| number(&format!("{prefix}{key}"), otherwise);
            let kind = match own("kind").map(String::as_str) {
                Some("drums") => Kind::Drums,
                Some("audio") => Kind::Audio,
                _ => Kind::Instrument,
            };
            let colour: Vec<u8> = own("colour").map_or(Vec::new(), |said| said.split(',').filter_map(|value| value.trim().parse().ok()).collect());
            let mut track = Track {
                name: own("name").cloned().unwrap_or_else(|| format!("Track {}", at + 1)),
                kind,
                instrument: if kind == Kind::Instrument { nested(&format!("{prefix}instrument.")) } else { sound::instrument("Chiptune") },
                kit: Vec::new(),
                volume: own_number("volume", 0.8),
                pan: own_number("pan", 0.0),
                muted: own_number("muted", 0.0) > 0.5,
                solo: own_number("solo", 0.0) > 0.5,
                fx: sound::mix_read(&pairs, &format!("{prefix}fx.")),
                cutoff: own_number("cutoff", 20_000.0),
                resonance: own_number("resonance", 0.0),
                send_a: own_number("send_a", 0.0),
                send_b: own_number("send_b", 0.0),
                sidechain: own_number("sidechain", 0.0).max(0.0) as usize,
                duck: own_number("duck", 0.6),
                regions: Vec::new(),
                lanes: Vec::new(),
                colour: if colour.len() == 3 { [colour[0], colour[1], colour[2]] } else { COLOURS[at % COLOURS.len()] },
            };
            if kind == Kind::Drums {
                track.kit = (0..own_number("kit", 0.0) as usize)
                    .map(|piece| (own(&format!("kit{piece}")).cloned().unwrap_or_default(), nested(&format!("{prefix}kit{piece}."))))
                    .collect();
            }
            for number_at in 0..own_number("regions", 0.0) as usize {
                let place = list(&format!("{prefix}region{number_at}")).into_iter().next().unwrap_or_default();
                let notes = list(&format!("{prefix}region{number_at}.notes"))
                    .into_iter()
                    .filter(|values| values.len() == 4)
                    .map(|values| note(values[0], values[1], values[2], values[3]))
                    .collect();
                let clip = own(&format!("region{number_at}.clip")).and_then(|said| {
                    let parts: Vec<&str> = said.split('|').collect();
                    let value = |at: usize, otherwise: f32| parts.get(at).and_then(|value| value.parse().ok()).unwrap_or(otherwise);
                    (!parts.is_empty() && !parts[0].is_empty()).then(|| Clip {
                        sound: parts[0].to_string(),
                        offset: value(1, 0.0),
                        gain: value(2, 1.0),
                        tone: value(3, 0.0),
                        fade_in: value(4, 0.0),
                        fade_out: value(5, 0.01),
                        reverse: value(6, 0.0) > 0.5,
                    })
                });
                track.regions.push(Region {
                    start: place.first().copied().unwrap_or(0.0),
                    length: place.get(1).copied().unwrap_or(4.0),
                    notes,
                    link: place.get(2).copied().unwrap_or(0.0).max(0.0) as u32,
                    clip,
                });
            }
            for target in Target::ALL {
                let points: Vec<(f32, f32)> =
                    list(&format!("{prefix}auto.{}", target.key())).into_iter().filter(|point| point.len() == 2).map(|point| (point[0], point[1])).collect();
                if !points.is_empty() {
                    track.lanes.push((target, points));
                }
            }
            // (The first studio's volume drawn, a factor from 0 to 2.)
            let old: Vec<(f32, f32)> =
                list(&format!("{prefix}volume_drawn")).into_iter().filter(|point| point.len() == 2).map(|point| (point[0], point[1] / 2.0)).collect();
            if !old.is_empty() && track.lane(Target::Volume).is_empty() {
                track.lanes.push((Target::Volume, old));
            }
            song.tracks.push(track);
        }
        Some(song)
    }
}

/// A clip's sound as heard: from its offset, so many half tones higher (faster), backwards, faded, as long as the
/// region (s) at most.
fn clipped(sound: &Channels, clip: &Clip, seconds: f32) -> Channels {
    let speed = 2f32.powf(clip.tone / 12.0);
    let (source_left, source_right) = sound;
    let total = source_left.len();
    if total < 2 {
        return (Vec::new(), Vec::new());
    }
    let start = clip.offset.max(0.0) * RATE;
    let count = (((total as f32 - start) / speed).max(0.0) as usize).min((seconds * RATE) as usize);
    let read = |channel: &[f32], at: usize| {
        let from = start + at as f32 * speed;
        let from = if clip.reverse { (total - 1) as f32 - from } else { from };
        if from < 0.0 || from >= (total - 1) as f32 {
            return 0.0;
        }
        let (below, along) = (from as usize, from.fract());
        channel[below] * (1.0 - along) + channel[below + 1] * along
    };
    let fade = |at: usize| {
        let (time, left) = (at as f32 / RATE, (count - at) as f32 / RATE);
        let mut gain = clip.gain;
        if clip.fade_in > 0.0 {
            gain *= (time / clip.fade_in).min(1.0);
        }
        if clip.fade_out > 0.0 {
            gain *= (left / clip.fade_out).min(1.0);
        }
        gain
    };
    ((0..count).map(|at| read(source_left, at) * fade(at)).collect(), (0..count).map(|at| read(source_right, at) * fade(at)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_song_is_played_clean_and_written_back_the_same() {
        let mut song = Project::demo();
        song.tracks[1].lanes = vec![(Target::Volume, vec![(0.0, 0.1), (8.0, 0.5)]), (Target::Cutoff, vec![(0.0, 0.3), (16.0, 1.0)])];
        song.tracks[0].fx.compress = 0.4;
        song.swing = 0.4;
        let mut audio = Track::audio(4);
        audio.regions.push(Region { clip: Some(Clip { tone: 3.0, reverse: true, ..Clip::of("test_beep") }), link: 2, ..Region::empty(1.0, 2.0) });
        song.tracks.push(audio);
        assert_eq!(Project::read(&song.written()), Some(song.clone()));
        let mut caches = Vec::new();
        let played = song.render(&mut caches, 0.0, 4.0, true, &OwnSounds::default());
        let seconds = played.left.len() as f32 / RATE;
        assert!(seconds >= 4.0 * song.beat() - 0.01, "{seconds}");
        assert!(played.left.iter().chain(&played.right).all(|sample| sample.is_finite() && sample.abs() <= 1.0), "clean");
        assert!(played.peaks.iter().take(4).all(|peak| *peak > 0.0), "every track is heard: {:?}", played.peaks);
        // (Played again from its notes kept: the same, at once.)
        let again = song.render(&mut caches, 0.0, 4.0, true, &OwnSounds::default());
        assert_eq!(again.left, played.left);
        assert!((song.tracks[1].at(Target::Volume, 4.0) - 0.6).abs() < 0.01);
        // (The kick squeezes the bass: quieter right after a kick than before the next.)
        let mut squeezed = Project::demo();
        squeezed.tracks.truncate(3);
        squeezed.tracks[1].muted = true;
        squeezed.tracks[0].volume = 0.0;
        let bass_only =
            |song: &Project| song.render(&mut Vec::new(), 0.0, 4.0, false, &OwnSounds::default()).left.iter().map(|sample| sample * sample).sum::<f32>();
        let mut free = squeezed.clone();
        free.tracks[2].sidechain = 0;
        assert!(bass_only(&squeezed) < bass_only(&free), "the sidechain ducks the bass");
    }

    #[test]
    fn a_clip_is_read_faster_backwards_and_faded() {
        let sound: Channels = ((0..32_000).map(|at| at as f32 / 32_000.0).collect(), vec![0.5; 32_000]);
        let clip = Clip { tone: 12.0, fade_in: 0.1, ..Clip::of("x") };
        let (left, right) = clipped(&sound, &clip, 10.0);
        assert_eq!(left.len(), 16_000, "an octave higher: twice as short");
        assert!(right[0].abs() < 1e-6 && (right[8000] - 0.5).abs() < 1e-3, "the fade in");
        let backwards = clipped(&sound, &Clip { reverse: true, ..Clip::of("x") }, 10.0);
        assert!(backwards.0[100] > 0.9, "backwards: the end first");
    }
}
