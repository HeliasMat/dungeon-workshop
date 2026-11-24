//! The music maker, as a studio of music has it: a song of tracks (instruments, drums, audio clips, the
//! microphone), their regions laid along the bars (linked as patterns), a piano roll with scales, chords, an
//! arpeggiator, strumming, humanizing and velocities, a step sequencer, each instrument made with the sound maker,
//! a mixer (filter, effects, two send buses, sidechain), any setting drawn along the song, the keyboard played
//! and recorded, a loop, a click, swing; MIDI files read and written; heard as it changes; written out
//! (`library/sounds/<word>.wav`: `music <word>` in a film; the song in `library/music/<word>.music`;
//! its tracks apart; a `.mid`).

mod editor;
mod keys;
mod midi;
mod mixer;
mod notes;
mod project;
#[cfg(test)]
mod tests;
mod tracks;
mod transport;
mod wave;

pub use editor::make_music;
use std::sync::Arc;

use wave::write_wave;
use workshop_base::session::Session;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use eframe::egui::{self};
use shell::sound::Hears;

use crate::sound::{Note, SoundMaking};
use project::{Clip, Kind, NoteCache, Project, RATE, Region, Rendered, Target, Track};

/// The bottom's views: the notes of the region chosen, the track's instrument, the mixer, the lanes drawn.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Notes,
    Instrument,
    Mixer,
    Automation,
}

/// Where the song is heard from and to, since when.
struct Playing {
    since: Instant,
    from: f32,
    to: f32,
}

/// A region taken by the pointer: its track, its place, lengthened or moved, where it was taken.
#[derive(Clone, Copy)]
struct Taken {
    track: usize,
    region: usize,
    edge: bool,
    grab: f32,
}

/// A song worked out aside, and the notes it kept.
type Worked = (Rendered, Vec<NoteCache>);

/// The scales a piano roll may light (half tones from the tonic in them).
const SCALES: [(&str, &[usize]); 9] = [
    ("None", &[]),
    ("Major", &[0, 2, 4, 5, 7, 9, 11]),
    ("Minor", &[0, 2, 3, 5, 7, 8, 10]),
    ("Harmonic Minor", &[0, 2, 3, 5, 7, 8, 11]),
    ("Major Pentatonic", &[0, 2, 4, 7, 9]),
    ("Minor Pentatonic", &[0, 3, 5, 7, 10]),
    ("Blues", &[0, 3, 5, 6, 7, 10]),
    ("Dorian", &[0, 2, 3, 5, 7, 9, 10]),
    ("Mixolydian", &[0, 2, 4, 5, 7, 9, 10]),
];
const TONICS: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
/// The chords a note may become (half tones over it).
const CHORDS: [(&str, &[f32]); 9] = [
    ("Major", &[0.0, 4.0, 7.0]),
    ("Minor", &[0.0, 3.0, 7.0]),
    ("7e", &[0.0, 4.0, 7.0, 10.0]),
    ("Major 7", &[0.0, 4.0, 7.0, 11.0]),
    ("Minor 7", &[0.0, 3.0, 7.0, 10.0]),
    ("Sus4", &[0.0, 5.0, 7.0]),
    ("Sus2", &[0.0, 2.0, 7.0]),
    ("Diminished", &[0.0, 3.0, 6.0]),
    ("Augmented", &[0.0, 4.0, 8.0]),
];
const ARPEGGIOS: [&str; 4] = ["Up", "Down", "Up and Down", "Random"];

/// The song being made, and all the studio around it.
pub struct MusicMaking {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    pub word: String,
    pub(crate) saved: bool,
    project: Project,
    undone: Vec<Project>,
    redone: Vec<Project>,
    taken: Option<Project>,
    caches: Vec<NoteCache>,
    track: usize,
    region: Option<usize>,
    playhead: f32,
    playing: Option<Playing>,
    rendering: Option<(mpsc::Receiver<Worked>, f32, f32)>,
    tab: Tab,
    editor: Option<(usize, usize, SoundMaking)>,
    piece: usize,
    record: bool,
    metronome: bool,
    octave: i32,
    held: HashMap<egui::Key, (f32, f32)>,
    pixels: f32,
    scroll: f32,
    first_track: usize,
    dragged: Option<Taken>,
    said: Option<String>,
    peaks: Vec<f32>,
    /// The microphone heard, since a beat.
    recording: Option<(shell::record::Recording, f32)>,
    takes: u32,
    lane: Target,
    scale: usize,
    tonic: usize,
    chord: usize,
    arpeggio: usize,
    arpeggio_step: f32,
    strum: f32,
}

/// The keys played as a piano (on a QWERTY keyboard: two rows, as in most music software), half tones from C.
const PIANO_KEYS: [(egui::Key, i32); 17] = [
    (egui::Key::A, 0),
    (egui::Key::W, 1),
    (egui::Key::S, 2),
    (egui::Key::E, 3),
    (egui::Key::D, 4),
    (egui::Key::F, 5),
    (egui::Key::T, 6),
    (egui::Key::G, 7),
    (egui::Key::Y, 8),
    (egui::Key::H, 9),
    (egui::Key::U, 10),
    (egui::Key::J, 11),
    (egui::Key::K, 12),
    (egui::Key::O, 13),
    (egui::Key::L, 14),
    (egui::Key::P, 15),
    (egui::Key::Semicolon, 16),
];

const LANE: f32 = 54.0;
const RULER: f32 = 22.0;
const HEADERS: f32 = 236.0;

/// The folder of the songs made here.
fn songs_folder(session: &Session) -> PathBuf {
    session.library().music()
}

/// The file a song of the project is written in.
pub fn song_file(session: &Session, word: &str) -> PathBuf {
    songs_folder(session).join(format!("{word}.music"))
}

/// The files in the songs' folder ending so, by their words.
fn files_ending(session: &Session, end: &str) -> Vec<String> {
    let mut words: Vec<String> = std::fs::read_dir(songs_folder(session))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|file| file.file_name().to_string_lossy().strip_suffix(end).map(str::to_string))
        .collect();
    words.sort();
    words
}

/// The songs made here, by their words.
pub fn songs(session: &Session) -> Vec<String> {
    files_ending(session, ".music")
}

/// The project's own sounds, by their words.
fn own_sounds(session: &Session) -> Vec<String> {
    shell::own_sounds::names_in(&session.library().sounds())
}

impl MusicMaking {
    /// A new song, begun from a beat, a bass, chords and a tune.
    pub fn new(session: &Arc<Session>, word: &str) -> MusicMaking {
        MusicMaking {
            session: session.clone(),
            word: word.to_string(),
            saved: false,
            project: Project::demo(),
            undone: Vec::new(),
            redone: Vec::new(),
            taken: None,
            caches: Vec::new(),
            track: 0,
            region: Some(0),
            playhead: 0.0,
            playing: None,
            rendering: None,
            tab: Tab::Notes,
            editor: None,
            piece: 0,
            record: false,
            metronome: false,
            octave: 0,
            held: HashMap::new(),
            pixels: 26.0,
            scroll: 0.0,
            first_track: 0,
            dragged: None,
            said: None,
            peaks: Vec::new(),
            recording: None,
            takes: 0,
            lane: Target::Volume,
            scale: 0,
            tonic: 0,
            chord: 0,
            arpeggio: 0,
            arpeggio_step: 0.25,
            strum: 0.03,
        }
        .shown_as_asked()
    }

    /// For a picture of it (`WORKSHOP_MUSIC=1:instrument`): that track, that view.
    fn shown_as_asked(mut self) -> MusicMaking {
        if let Some(asked) = self.session.launch.music.clone()
            && let Some((track, tab)) = asked.split_once(':')
        {
            self.track = track.parse().unwrap_or(0).min(self.project.tracks.len().saturating_sub(1));
            self.tab = match tab {
                "instrument" => Tab::Instrument,
                "mixer" => Tab::Mixer,
                "automation" => Tab::Automation,
                _ => Tab::Notes,
            };
            self.scale = if tab == "scale" { 1 } else { 0 };
        }
        self
    }

    /// A song made here before, as it was written.
    pub fn open(session: &Arc<Session>, word: &str) -> MusicMaking {
        let mut made = MusicMaking::new(session, word);
        workshop_base::safe::check(session, &song_file(session, word));
        match std::fs::read_to_string(songs_folder(session).join(format!("{word}.music"))).ok().and_then(|text| Project::read(&text)) {
            Some(song) => {
                made.project = song;
                made.saved = true;
            }
            None => made.said = Some("not read: starting from a new song".to_string()),
        }
        made
    }

    fn beat_now(&self) -> f32 {
        match &self.playing {
            Some(playing) => playing.from + playing.since.elapsed().as_secs_f32() / self.project.beat(),
            None => self.playhead,
        }
    }

    /// Played from a beat (to the loop's end, or the song's): worked out aside, heard once ready.
    fn play_from(&mut self, from: f32) {
        if self.rendering.is_some() {
            return;
        }
        let song = &self.project;
        let to = if song.looping && song.loop_end > song.loop_start { song.loop_end } else { song.end() };
        let from = if from >= to - 0.01 { if song.looping { song.loop_start } else { 0.0 } } else { from };
        let (sender, receiver) = mpsc::channel();
        let (song, mut caches, click) = (self.project.clone(), std::mem::take(&mut self.caches), self.metronome);
        let own = self.session.own_sounds.clone();
        std::thread::spawn(move || {
            let played = song.render(&mut caches, from, to, click, &own);
            _ = sender.send((played, caches));
        });
        self.rendering = Some((receiver, from, to));
    }

    fn stop(&mut self, audio: &mut Option<Box<dyn Hears>>) {
        if let Some(playing) = self.playing.take() {
            self.playhead = (playing.from + playing.since.elapsed().as_secs_f32() / self.project.beat()).min(playing.to);
        }
        if let Some(audio) = audio.as_mut() {
            audio.stop_held();
        }
    }

    fn word(&self) -> Result<String, String> {
        let word = film::words::plain(&self.word.trim().replace([' ', '-'], "_"));
        if word.is_empty() {
            return Err("a word first".to_string());
        }
        if rules::songs::NOISES.iter().any(|noise| noise.word == word) || rules::songs::SONGS.iter().any(|song| song.word == word) {
            return Err(format!("\"{word}\" is already a sound or a song of the game: another word"));
        }
        Ok(word)
    }

    /// Written out: the song heard (a sound of the project's, `music <word>` in a film) and the song itself.
    fn export(&mut self) -> Result<String, String> {
        let word = self.word()?;
        let end = self.project.end();
        let played = self.project.render(&mut self.caches, 0.0, end, false, &self.session.own_sounds);
        write_wave(&self.session.library().sounds().join(format!("{word}.wav")), &played.left, &played.right)?;
        self.write_song(&word)?;
        self.word = word.clone();
        self.saved = true;
        Ok(word)
    }

    fn write_song(&self, word: &str) -> Result<(), String> {
        std::fs::create_dir_all(songs_folder(&self.session)).map_err(|why| why.to_string())?;
        workshop_base::safe::write(&song_file(&self.session, word), self.project.written()).map_err(|why| why.to_string())
    }

    /// Each track heard alone, in its own file (`library/music/<word>_tracks/`).
    fn export_stems(&mut self) -> Result<String, String> {
        let word = self.word()?;
        let folder = songs_folder(&self.session).join(format!("{word}_tracks"));
        std::fs::create_dir_all(&folder).map_err(|why| why.to_string())?;
        let end = self.project.end();
        for at in 0..self.project.tracks.len() {
            let mut alone = self.project.clone();
            for (other, track) in alone.tracks.iter_mut().enumerate() {
                (track.solo, track.muted) = (other == at, other != at);
            }
            let played = alone.render(&mut Vec::new(), 0.0, end, false, &self.session.own_sounds);
            let name = film::words::plain(&alone.tracks[at].name.replace([' ', '-'], "_"));
            write_wave(&folder.join(format!("{:02}_{name}.wav", at + 1)), &played.left, &played.right)?;
        }
        Ok(folder.display().to_string())
    }

    /// Its notes as a MIDI file (`library/music/<word>.mid`), each instrument and drum track.
    fn export_midi(&self) -> Result<String, String> {
        let word = self.word()?;
        let tracks: Vec<midi::MidiTrack> = self
            .project
            .tracks
            .iter()
            .filter(|track| track.kind != Kind::Audio)
            .map(|track| midi::MidiTrack {
                name: track.name.clone(),
                drums: track.kind == Kind::Drums,
                notes: track
                    .regions
                    .iter()
                    .flat_map(|region| {
                        region.notes.iter().filter(|note| note.start < region.length).map(move |note| {
                            let key = match track.kind {
                                Kind::Drums => midi::DRUM_KEYS.get(note.tone.round().max(0.0) as usize).copied().unwrap_or(42),
                                _ => (60.0 + note.tone).round().clamp(0.0, 127.0) as u8,
                            };
                            (
                                region.start + note.start,
                                note.length.min(region.length - note.start),
                                key,
                                (note.velocity * 127.0).round().clamp(1.0, 127.0) as u8,
                            )
                        })
                    })
                    .collect(),
            })
            .collect();
        std::fs::create_dir_all(songs_folder(&self.session)).map_err(|why| why.to_string())?;
        let file = songs_folder(&self.session).join(format!("{word}.mid"));
        std::fs::write(&file, midi::write(self.project.tempo, self.project.beats_per_bar, &tracks)).map_err(|why| why.to_string())?;
        Ok(file.display().to_string())
    }

    /// A MIDI file's tracks laid in the song (its tempo taken if the song has nothing yet).
    fn import_midi(&mut self, bytes: &[u8], name: &str) -> Result<String, String> {
        let (tempo, tracks) = midi::read(bytes)?;
        if tracks.is_empty() {
            return Err("no note in this file".to_string());
        }
        if self.project.tracks.iter().all(|track| track.regions.is_empty()) {
            self.project.tempo = tempo;
        }
        let per_bar = self.project.beats_per_bar.max(1.0);
        for read in &tracks {
            let at = self.project.tracks.len();
            let mut track = if read.drums { Track::drums(at) } else { Track::instrument("Electric Piano", at) };
            track.name = read.name.clone();
            let end = read.notes.iter().map(|(start, length, ..)| start + length).fold(0.0, f32::max);
            let notes = read
                .notes
                .iter()
                .filter_map(|(start, length, key, velocity)| {
                    let tone = if read.drums { midi::DRUM_KEYS.iter().position(|drum| drum == key)? as f32 } else { f32::from(*key) - 60.0 };
                    Some(Note { start: *start, length: length.max(0.05), tone, velocity: f32::from(*velocity) / 127.0 })
                })
                .collect();
            track.regions.push(Region { notes, ..Region::empty(0.0, ((end / per_bar).ceil() * per_bar).max(per_bar)) });
            self.project.tracks.push(track);
        }
        Ok(format!("\"{name}\": {} track(s)", tracks.len()))
    }

    /// A sound of the project's laid on an audio track (the one chosen, else a new one) at a beat.
    fn lay_sound(&mut self, word: &str, at: f32) {
        let seconds = project::sound_of(&self.session.own_sounds, word).map_or(1.0, |(left, _)| left.len() as f32 / RATE);
        let place = match self.project.tracks.get(self.track).filter(|track| track.kind == Kind::Audio) {
            Some(_) => self.track,
            None => {
                let at = self.project.tracks.len();
                self.project.tracks.push(Track::audio(at));
                at
            }
        };
        let length = ((seconds / self.project.beat()) * 4.0).ceil() / 4.0;
        let track = &mut self.project.tracks[place];
        track.regions.push(Region { clip: Some(Clip::of(word)), ..Region::empty(at, length.max(0.25)) });
        (self.track, self.region, self.tab) = (place, Some(track.regions.len() - 1), Tab::Notes);
    }

    /// Files dropped on the window: a MIDI file's tracks, a sound (kept among the project's) laid at the playhead.
    fn take_dropped(&mut self, context: &egui::Context) {
        let dropped: Vec<PathBuf> = context.input(|input| input.raw.dropped_files.iter().filter_map(|file| file.path.clone()).collect());
        self.take_files(dropped);
    }

    /// Files dropped or chosen: a MIDI file's tracks, a sound laid at the playhead.
    fn take_files(&mut self, dropped: Vec<PathBuf>) {
        for path in dropped {
            let extension = path.extension().map(|end| end.to_string_lossy().to_lowercase()).unwrap_or_default();
            let stem = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
            match extension.as_str() {
                "mid" | "midi" => {
                    self.said = Some(match std::fs::read(&path).map_err(|why| why.to_string()).and_then(|bytes| self.import_midi(&bytes, &stem)) {
                        Ok(said) | Err(said) => said,
                    });
                }
                "wav" => {
                    let word = film::words::plain(&stem.replace([' ', '-'], "_"));
                    let to = self.session.library().sounds().join(format!("{word}.wav"));
                    if !to.exists() {
                        _ = std::fs::copy(&path, &to);
                    }
                    let at = self.playhead;
                    self.lay_sound(&word, at);
                    self.said = Some(format!("\"{word}\" put on an audio track"));
                }
                _ => {}
            }
        }
    }

    /// The microphone heard from now (the song played meanwhile), or stopped: what it heard kept among the
    /// project's sounds and laid where it began.
    fn microphone(&mut self, audio: &mut Option<Box<dyn Hears>>) {
        match self.recording.take() {
            None => match shell::record::Recording::start() {
                Ok(recording) => {
                    let at = self.playhead;
                    self.recording = Some((recording, at));
                    if self.playing.is_none() {
                        self.play_from(at);
                    }
                }
                Err(why) => self.said = Some(format!("mic: {why}")),
            },
            Some((recording, at)) => {
                self.stop(audio);
                let heard = recording.stop();
                self.takes += 1;
                let word = format!("{}_take{}", film::words::plain(&self.word.replace([' ', '-'], "_")), self.takes);
                let path = self.session.library().sounds().join(format!("{word}.wav"));
                match write_wave(&path, &heard, &heard) {
                    Ok(()) => {
                        self.lay_sound(&word, at);
                        self.said = Some(format!("take \"{word}\": {:.1} s", heard.len() as f32 / RATE));
                    }
                    Err(why) => self.said = Some(why),
                }
            }
        }
    }
}
