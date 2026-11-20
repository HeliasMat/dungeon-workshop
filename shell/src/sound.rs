//! What is heard: songs and sounds made from the game's banks, and files of
//! our own (`own_sounds`), played on the machine's speakers, in two
//! channels. Songs take a moment to make: made aside, they start when
//! ready, unless another was asked for meanwhile. Beside the music and the
//! ambiance, as many sounds as asked go on at once (`Voice`): looped, faded,
//! stopped by their name.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use assets::project::Project;

use crate::audio::OUTPUT_RATE;
use crate::music::Library;
use crate::own_sounds;
use rodio::buffer::SamplesBuffer;
use rodio::source::{ChannelVolume, Source as _};
use rodio::{OutputStream, OutputStreamHandle, Sink};

/// The most of a song, and of a sound or short tune, that is played.
pub const MUSIC_SECONDS: f32 = 180.0;
pub const SOUND_SECONDS: f32 = 8.0;

/// How long each of the game's sounds lasts, in frames (60 a second), as
/// the game's library in a folder has it: the library read the first time
/// one is asked, each length kept (none: no library, or no such sound).
/// Kept by whoever holds it (the shell's `Art`), not behind its back.
pub struct SoundLengths {
    folder: String,
    library: std::sync::OnceLock<Option<Arc<Library>>>,
    lengths: std::sync::Mutex<HashMap<u16, Option<f32>>>,
}

impl SoundLengths {
    pub fn new(folder: &str) -> SoundLengths {
        SoundLengths { folder: folder.to_string(), library: std::sync::OnceLock::new(), lengths: std::sync::Mutex::default() }
    }

    /// That sound's length, in frames.
    pub fn of(&self, id: u16) -> Option<f32> {
        let mut lengths = self.lengths.lock().ok()?;
        if let Some(known) = lengths.get(&id) {
            return *known;
        }
        let library = self.library.get_or_init(|| Library::open(&self.folder).map(Arc::new));
        let frames = library.as_ref().and_then(|library| library.sound_seconds(id, SOUND_SECONDS)).map(|seconds| seconds * 60.0);
        lengths.insert(id, frames);
        frames
    }
}

/// How long a sound of our own lasts, in frames (none: no such file).
pub fn own_length(folder: &str, project: &Project, name: &str) -> Option<f32> {
    own_sounds::seconds(folder, project, name).map(|seconds| seconds * 60.0)
}

/// What a voice plays.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// One of the game's sounds.
    Sound(u16),
    /// One of the game's songs (an ambiance).
    Song(u16),
    /// A file of our own, by its name.
    Own(String),
}

/// A sound played as asked: how loud, from which side (-1 left, 1 right),
/// how fast (its pitch with it), looped, named to be faded or stopped,
/// coming in over so many frames.
#[derive(Clone, Debug, PartialEq)]
pub struct Voice {
    pub source: Source,
    pub volume: f32,
    pub pan: f32,
    pub speed: f32,
    pub looped: bool,
    pub name: Option<String>,
    pub rise: f32,
}

impl Voice {
    /// A sound as it is: once, in the middle, unnamed.
    pub fn of(source: Source) -> Voice {
        Voice { source, volume: 1.0, pan: 0.0, speed: 1.0, looped: false, name: None, rise: 0.0 }
    }

    /// Whether it is kept, to be faded or stopped: named, looped or rising.
    fn kept(&self) -> bool {
        self.looped || self.name.is_some() || self.rise > 0.0
    }
}

/// How loud each side is for a sound from there (-1 left, 1 right): as it
/// is in the middle, the other side dying away towards one.
pub fn balance(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0))
}

/// How loud a voice goes now, and towards what.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    pub now: f32,
    step: f32,
    to: f32,
    /// Gone once there.
    stop: bool,
}

impl Level {
    /// At that level; rising to it from silence over so many frames.
    pub fn new(level: f32, rise: f32) -> Level {
        let mut made = Level { now: level, step: 0.0, to: level, stop: false };
        if rise > 0.0 {
            made.now = 0.0;
            made.fade(level, rise, false);
        }
        made
    }

    /// To that level over so many frames; `stop`: gone once there.
    pub fn fade(&mut self, level: f32, frames: f32, stop: bool) {
        (self.to, self.stop) = (level, stop);
        self.step = (level - self.now) / frames.max(1.0);
        if frames <= 0.0 {
            self.now = level;
        }
    }

    /// A frame goes by; whether it is gone.
    pub fn tick(&mut self) -> bool {
        if self.step != 0.0 {
            self.now += self.step;
            if (self.step > 0.0 && self.now >= self.to) || (self.step < 0.0 && self.now <= self.to) {
                (self.now, self.step) = (self.to, 0.0);
            }
        }
        self.stop && self.step == 0.0
    }
}

/// What can be asked to be heard.
pub trait Hears {
    /// A song on a channel (0 the music, 1 what is heard around; none:
    /// silence).
    fn song(&mut self, channel: usize, song: Option<u16>, level: f32, step: f32);
    /// A song from that many seconds into it.
    fn song_from(&mut self, channel: usize, song: Option<u16>, level: f32, seconds: f32);
    /// A file of our own as the song of a channel, from that many seconds
    /// into it.
    fn own_song(&mut self, _channel: usize, _name: &str, _level: f32, _step: f32, _seconds: f32) {}
    /// How loud a channel is now.
    fn volume(&mut self, channel: usize, level: f32);
    fn sound(&self, id: u16, volume: f32, pitch: f32);
    /// A sound as asked (see `Voice`); a voice of the same name stops.
    fn play(&mut self, voice: Voice) {
        if let (Source::Sound(id), false) = (&voice.source, voice.kept()) {
            self.sound(*id, voice.volume, voice.speed);
        }
    }
    /// A voice by its name goes to that level over so many frames; `stop`:
    /// it stops once there.
    fn fade_voice(&mut self, _name: &str, _level: f32, _frames: f32, _stop: bool) {}
    /// Every voice stops (the music and the ambiance go on).
    fn hush(&mut self) {}
    /// All that is heard held where it is (paused), or going on from there.
    fn pause(&mut self, _paused: bool) {}
    /// Samples heard once, as they are (two channels, left and right one
    /// after the other, at the speakers' rate): a sound being made.
    fn samples(&mut self, _stereo: Vec<i16>, _volume: f32) {}
    /// Samples (two channels) played until stopped, a song being made: those played so before stop.
    fn play_held(&mut self, _stereo: Vec<i16>, _volume: f32) {}
    fn stop_held(&mut self) {}
    fn jingle(&self, id: u16);
    /// A channel fades away over so many frames.
    fn fade(&mut self, channel: usize, frames: f32);
    /// How fast the music plays (1: as it is).
    fn speed(&mut self, speed: f32);
    /// A frame goes by.
    fn tick(&mut self);
    /// All that was heard, into a file, for what keeps it.
    fn write_to(&self, _path: &str) -> Result<(), crate::Error> {
        Ok(())
    }
}

/// Where a song made aside goes.
enum Into {
    Channel(usize),
    Voice(String),
}

/// A song made, where it was asked for, and when.
struct Made {
    into: Into,
    asked: u32,
    pcm: Vec<i16>,
    /// The samples it starts past, the first time.
    past: usize,
}

/// A voice playing on the speakers.
struct Playing {
    sink: Sink,
    level: Level,
    asked: u32,
    pan: f32,
    /// Its song is still being made.
    waiting: bool,
}

pub struct Audio {
    /// (Kept: no sound without it.)
    _stream: OutputStream,
    handle: OutputStreamHandle,
    library: Arc<Library>,
    folder: String,
    /// The project whose own sounds it plays.
    project: Project,
    channels: [Sink; 2],
    asked: [Arc<AtomicU32>; 2],
    made: (mpsc::Sender<Made>, mpsc::Receiver<Made>),
    /// How loud each channel goes now, and by how much it changes a frame.
    levels: [(f32, f32); 2],
    speed: f32,
    voices: HashMap<String, Playing>,
    voices_made: u32,
    /// Whether the song of each channel is one of ours (in two channels).
    own_tune: [bool; 2],
    /// Our own sounds read so far, in two channels.
    own: RefCell<HashMap<String, Option<Arc<Vec<i16>>>>>,
    /// What plays until stopped (a song being made).
    held: Option<Sink>,
}

fn sink(handle: &OutputStreamHandle) -> Option<Sink> {
    Sink::try_new(handle).ok()
}

impl Audio {
    /// None without anything to play on; that project's own sounds.
    pub fn open_in(folder: &str, project: Project) -> Option<Self> {
        let library: Library = bincode::deserialize(&crate::data::read(folder, "sound.bin")?).ok()?;
        let (stream, handle) = OutputStream::try_default().ok()?;
        let channels = [sink(&handle)?, sink(&handle)?];
        Some(Audio {
            _stream: stream,
            handle,
            library: Arc::new(library),
            folder: folder.to_string(),
            project,
            channels,
            asked: [Arc::new(AtomicU32::new(0)), Arc::new(AtomicU32::new(0))],
            made: mpsc::channel(),
            levels: [(1.0, 0.0), (1.0, 0.0)],
            speed: 1.0,
            voices: HashMap::new(),
            voices_made: 0,
            own_tune: [false; 2],
            own: RefCell::default(),
            held: None,
        })
    }

    fn play_once(&self, pcm: Option<Vec<i16>>, volume: f32, pitch: f32) {
        if let (Some(pcm), Ok(sink)) = (pcm, Sink::try_new(&self.handle)) {
            sink.set_volume(volume);
            sink.set_speed(pitch);
            sink.append(SamplesBuffer::new(1, OUTPUT_RATE, pcm));
            sink.detach();
        }
    }

    /// A sound of ours, in two channels.
    fn own(&self, name: &str) -> Option<Arc<Vec<i16>>> {
        self.own.borrow_mut().entry(name.to_string()).or_insert_with(|| own_sounds::read(&self.folder, &self.project, name).map(Arc::new)).clone()
    }

    /// Samples into a sink, from a side, looped or not.
    fn append(sink: &Sink, channels: u16, pcm: Vec<i16>, pan: f32, looped: bool) {
        let (left, right) = balance(pan);
        let buffer = SamplesBuffer::new(channels, OUTPUT_RATE, pcm);
        match (channels, looped) {
            // (One channel: heard more on one side than the other.)
            (1, true) => sink.append(ChannelVolume::new(buffer.repeat_infinite(), vec![left, right])),
            (1, false) => sink.append(ChannelVolume::new(buffer, vec![left, right])),
            (_, true) => sink.append(buffer.repeat_infinite()),
            (_, false) => sink.append(buffer),
        }
    }
}

/// Two channels, each side as loud as it is from there.
fn panned(stereo: &[i16], pan: f32) -> Vec<i16> {
    if pan == 0.0 {
        return stereo.to_vec();
    }
    let (left, right) = balance(pan);
    stereo.as_chunks::<2>().0.iter().flat_map(|pair| [(pair[0] as f32 * left) as i16, (pair[1] as f32 * right) as i16]).collect()
}

impl Hears for Audio {
    fn play_held(&mut self, stereo: Vec<i16>, volume: f32) {
        self.stop_held();
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.set_volume(volume);
            sink.append(SamplesBuffer::new(2, OUTPUT_RATE, stereo));
            self.held = Some(sink);
        }
    }

    fn stop_held(&mut self) {
        if let Some(sink) = self.held.take() {
            sink.stop();
        }
    }

    fn samples(&mut self, stereo: Vec<i16>, volume: f32) {
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.set_volume(volume);
            sink.append(SamplesBuffer::new(2, OUTPUT_RATE, stereo));
            sink.detach();
        }
    }

    fn song(&mut self, channel: usize, song: Option<u16>, level: f32, step: f32) {
        self.made_from(channel, song.map(Tune::Game), level, step, 0);
    }

    fn song_from(&mut self, channel: usize, song: Option<u16>, level: f32, seconds: f32) {
        self.made_from(channel, song.map(Tune::Game), level, 0.0, (seconds * OUTPUT_RATE as f32) as usize);
    }

    fn own_song(&mut self, channel: usize, name: &str, level: f32, step: f32, seconds: f32) {
        self.made_from(channel, Some(Tune::Own(name.to_string())), level, step, (seconds * OUTPUT_RATE as f32) as usize * 2);
    }

    fn volume(&mut self, channel: usize, level: f32) {
        self.levels[channel] = (level, 0.0);
        self.channels[channel].set_volume(level);
    }

    fn sound(&self, id: u16, volume: f32, pitch: f32) {
        if id & 0xFF != 0 {
            self.play_once(self.library.sound(id, SOUND_SECONDS), volume, pitch);
        }
    }

    fn play(&mut self, voice: Voice) {
        let Ok(sink) = Sink::try_new(&self.handle) else {
            return;
        };
        let level = Level::new(voice.volume, voice.rise);
        sink.set_volume(level.now);
        sink.set_speed(voice.speed);
        self.voices_made += 1;
        let asked = self.voices_made;
        let name = voice.name.clone().unwrap_or_else(|| format!("_{asked}"));
        let mut waiting = false;
        match &voice.source {
            Source::Sound(id) if id & 0xFF != 0 => match self.library.sound(*id, SOUND_SECONDS) {
                Some(pcm) => Audio::append(&sink, 1, pcm, voice.pan, voice.looped),
                None => return,
            },
            Source::Sound(_) => return,
            Source::Own(name) => match self.own(name) {
                Some(pcm) => Audio::append(&sink, 2, panned(&pcm, voice.pan), 0.0, voice.looped),
                None => return,
            },
            // (Made aside, appended when ready: see `tick`.)
            Source::Song(song) => {
                let (library, send, song, name) = (self.library.clone(), self.made.0.clone(), *song, name.clone());
                waiting = true;
                std::thread::spawn(move || {
                    if let Some(pcm) = library.music(song, MUSIC_SECONDS) {
                        _ = send.send(Made { into: Into::Voice(name), asked, pcm, past: 0 });
                    }
                });
            }
        }
        if !voice.kept() {
            sink.detach();
            return;
        }
        if let Some(before) = self.voices.insert(name, Playing { sink, level, asked, pan: voice.pan, waiting }) {
            before.sink.stop();
        }
    }

    fn fade_voice(&mut self, name: &str, level: f32, frames: f32, stop: bool) {
        if let Some(voice) = self.voices.get_mut(name) {
            voice.level.fade(level, frames, stop);
        }
    }

    fn hush(&mut self) {
        for (_, voice) in self.voices.drain() {
            voice.sink.stop();
        }
    }

    fn pause(&mut self, paused: bool) {
        for sink in self.channels.iter().chain(self.voices.values().map(|voice| &voice.sink)) {
            if paused {
                sink.pause();
            } else {
                sink.play();
            }
        }
    }

    fn jingle(&self, id: u16) {
        self.play_once(self.library.jingle(id, SOUND_SECONDS), 1.0, 1.0);
    }

    fn fade(&mut self, channel: usize, frames: f32) {
        self.levels[channel].1 = -self.levels[channel].0 / frames.max(1.0);
    }

    fn speed(&mut self, speed: f32) {
        self.speed = speed;
        self.channels[0].set_speed(speed);
    }

    /// (The songs made start, those fading fade, the voices played out go.)
    fn tick(&mut self) {
        while let Ok(made) = self.made.1.try_recv() {
            match made.into {
                Into::Channel(channel) if self.asked[channel].load(Ordering::SeqCst) == made.asked => {
                    let channels = if self.own_tune[channel] { 2 } else { 1 };
                    // (From that far into it: a song goes round, so as far into its round.)
                    let past = made.past % made.pcm.len().max(1) / channels as usize * channels as usize;
                    if past > 0 {
                        self.channels[channel].append(SamplesBuffer::new(channels, OUTPUT_RATE, made.pcm[past..].to_vec()));
                    }
                    self.channels[channel].append(SamplesBuffer::new(channels, OUTPUT_RATE, made.pcm).repeat_infinite());
                    self.channels[0].set_speed(self.speed);
                }
                Into::Voice(name) => {
                    if let Some(voice) = self.voices.get_mut(&name).filter(|voice| voice.asked == made.asked) {
                        Audio::append(&voice.sink, 1, made.pcm, voice.pan, true);
                        voice.waiting = false;
                    }
                }
                Into::Channel(_) => {}
            }
        }
        for (channel, (level, step)) in self.levels.iter_mut().enumerate() {
            if *step != 0.0 {
                *level = (*level + *step).clamp(0.0, 1.0);
                self.channels[channel].set_volume(*level);
                if *level == 0.0 || *level == 1.0 {
                    *step = 0.0;
                }
            }
        }
        self.voices.retain(|_, voice| {
            let gone = voice.level.tick();
            voice.sink.set_volume(voice.level.now);
            if gone {
                voice.sink.stop();
            }
            // (One played to its end goes; one made aside is not there yet.)
            !gone && (voice.waiting || !voice.sink.empty())
        });
    }
}

/// A song of a channel: the game's, or a file of our own.
#[derive(Clone, Debug, PartialEq)]
pub enum Tune {
    Game(u16),
    Own(String),
}

impl Audio {
    /// A song made aside, to start past so many samples.
    fn made_from(&mut self, channel: usize, song: Option<Tune>, level: f32, step: f32, past: usize) {
        let asked = self.asked[channel].fetch_add(1, Ordering::SeqCst) + 1;
        self.channels[channel].stop();
        self.levels[channel] = (level, step);
        self.channels[channel].set_volume(level);
        self.own_tune[channel] = matches!(song, Some(Tune::Own(_)));
        let song = match song {
            None => return,
            Some(Tune::Own(name)) => {
                if let Some(pcm) = self.own(&name) {
                    _ = self.made.0.send(Made { into: Into::Channel(channel), asked, pcm: pcm.to_vec(), past });
                }
                return;
            }
            Some(Tune::Game(song)) => song,
        };
        let (library, send, latest) = (self.library.clone(), self.made.0.clone(), self.asked[channel].clone());
        std::thread::spawn(move || {
            if let Some(pcm) = library.music(song, MUSIC_SECONDS).filter(|_| latest.load(Ordering::SeqCst) == asked) {
                _ = send.send(Made { into: Into::Channel(channel), asked, pcm, past });
            }
        });
    }
}

/// Samples going on, for what is kept: in one channel or two, where they
/// are, how loud.
#[derive(Default)]
struct Going {
    pcm: Option<Rc<Vec<i16>>>,
    channels: usize,
    at: f32,
    level: f32,
    step: f32,
}

/// A voice kept, for what is kept.
struct KeptVoice {
    pcm: Rc<Vec<i16>>,
    channels: usize,
    at: f32,
    speed: f32,
    level: Level,
    sides: (f32, f32),
    looped: bool,
}

impl KeptVoice {
    /// Its sample at `at` on each side, or none past its end (looped: never).
    fn sample(pcm: &[i16], channels: usize, at: usize, looped: bool) -> Option<(f32, f32)> {
        let frames = pcm.len() / channels.max(1);
        if frames == 0 || (!looped && at >= frames) {
            return None;
        }
        let at = (at % frames) * channels;
        let left = pcm[at] as f32 / 32768.0;
        Some((left, if channels > 1 { pcm[at + 1] as f32 / 32768.0 } else { left }))
    }
}

/// What was asked kept instead of played, frame by frame: what is made into
/// a video has the sound it would have had, in two channels.
pub struct Recorder {
    library: Library,
    folder: String,
    project: Project,
    songs: HashMap<u16, Rc<Vec<i16>>>,
    own: HashMap<String, Option<Rc<Vec<i16>>>>,
    channels: [Going; 2],
    voices: HashMap<String, KeptVoice>,
    voices_made: u32,
    speed: f32,
    frame: u64,
    /// Left and right, one after the other.
    heard: RefCell<Vec<f32>>,
}

impl Recorder {
    /// That project's own sounds.
    pub fn open_in(folder: &str, project: Project) -> Option<Self> {
        let library = bincode::deserialize(&crate::data::read(folder, "sound.bin")?).ok()?;
        Some(Recorder {
            library,
            folder: folder.to_string(),
            project,
            songs: HashMap::new(),
            own: HashMap::new(),
            channels: Default::default(),
            voices: HashMap::new(),
            voices_made: 0,
            speed: 1.0,
            frame: 0,
            heard: RefCell::default(),
        })
    }

    /// Where this frame starts among the samples of each side.
    fn here(&self) -> usize {
        (self.frame * OUTPUT_RATE as u64 / 60) as usize
    }

    /// The samples heard, room made for them until there on each side.
    fn room(heard: &RefCell<Vec<f32>>, until: usize) -> std::cell::RefMut<'_, Vec<f32>> {
        let mut heard = heard.borrow_mut();
        if heard.len() < until * 2 {
            heard.resize(until * 2, 0.0);
        }
        heard
    }

    /// Samples heard from now on, once.
    fn add(&self, pcm: &[i16], channels: usize, volume: f32, pitch: f32, pan: f32) {
        let frames = pcm.len() / channels.max(1);
        if frames == 0 {
            return;
        }
        let (start, count) = (self.here(), (frames as f32 / pitch.max(0.01)) as usize);
        let (left, right) = balance(pan);
        let mut heard = Recorder::room(&self.heard, start + count);
        for at in 0..count {
            let Some((l, r)) = KeptVoice::sample(pcm, channels, ((at as f32 * pitch) as usize).min(frames - 1), false) else {
                break;
            };
            heard[(start + at) * 2] += l * volume * left;
            heard[(start + at) * 2 + 1] += r * volume * right;
        }
    }

    /// A song of the game, made once.
    fn song_made(&mut self, song: u16) -> Option<Rc<Vec<i16>>> {
        if let Some(pcm) = self.songs.get(&song) {
            return Some(pcm.clone());
        }
        let pcm = Rc::new(self.library.music(song, MUSIC_SECONDS)?);
        self.songs.insert(song, pcm.clone());
        Some(pcm)
    }

    /// A sound of ours, read once, in two channels.
    fn own_read(&mut self, name: &str) -> Option<Rc<Vec<i16>>> {
        let (folder, project) = (&self.folder, &self.project);
        self.own.entry(name.to_string()).or_insert_with(|| own_sounds::read(folder, project, name).map(Rc::new)).clone()
    }
}

impl Hears for Recorder {
    fn song(&mut self, channel: usize, song: Option<u16>, level: f32, step: f32) {
        let pcm = song.and_then(|song| self.song_made(song));
        self.channels[channel] = Going { pcm, channels: 1, at: 0.0, level, step };
    }

    fn song_from(&mut self, channel: usize, song: Option<u16>, level: f32, seconds: f32) {
        self.song(channel, song, level, 0.0);
        self.channels[channel].at = seconds * OUTPUT_RATE as f32;
    }

    fn own_song(&mut self, channel: usize, name: &str, level: f32, step: f32, seconds: f32) {
        let pcm = self.own_read(name);
        self.channels[channel] = Going { pcm, channels: 2, at: seconds * OUTPUT_RATE as f32, level, step };
    }

    fn volume(&mut self, channel: usize, level: f32) {
        (self.channels[channel].level, self.channels[channel].step) = (level, 0.0);
    }

    fn sound(&self, id: u16, volume: f32, pitch: f32) {
        if id & 0xFF != 0
            && let Some(pcm) = self.library.sound(id, SOUND_SECONDS)
        {
            self.add(&pcm, 1, volume, pitch, 0.0);
        }
    }

    fn play(&mut self, voice: Voice) {
        let (pcm, channels) = match &voice.source {
            Source::Sound(id) if id & 0xFF != 0 => (self.library.sound(*id, SOUND_SECONDS).map(Rc::new), 1),
            Source::Sound(_) => (None, 1),
            Source::Song(song) => (self.song_made(*song), 1),
            Source::Own(name) => (self.own_read(name), 2),
        };
        let Some(pcm) = pcm else {
            return;
        };
        if !voice.kept() {
            self.add(&pcm, channels, voice.volume, voice.speed, voice.pan);
            return;
        }
        self.voices_made += 1;
        let name = voice.name.clone().unwrap_or_else(|| format!("_{}", self.voices_made));
        let kept = KeptVoice {
            pcm,
            channels,
            at: 0.0,
            speed: voice.speed,
            level: Level::new(voice.volume, voice.rise),
            sides: balance(voice.pan),
            looped: voice.looped,
        };
        self.voices.insert(name, kept);
    }

    fn fade_voice(&mut self, name: &str, level: f32, frames: f32, stop: bool) {
        if let Some(voice) = self.voices.get_mut(name) {
            voice.level.fade(level, frames, stop);
        }
    }

    fn hush(&mut self) {
        self.voices.clear();
    }

    fn jingle(&self, id: u16) {
        if let Some(pcm) = self.library.jingle(id, SOUND_SECONDS) {
            self.add(&pcm, 1, 1.0, 1.0, 0.0);
        }
    }

    fn fade(&mut self, channel: usize, frames: f32) {
        self.channels[channel].step = -self.channels[channel].level / frames.max(1.0);
    }

    fn speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    fn tick(&mut self) {
        let start = self.here();
        self.frame += 1;
        let end = self.here();
        let mut heard = Recorder::room(&self.heard, end);
        for (number, channel) in self.channels.iter_mut().enumerate() {
            if let Some(pcm) = channel.pcm.as_ref().filter(|pcm| !pcm.is_empty()) {
                let speed = if number == 0 { self.speed } else { 1.0 };
                for at in start..end {
                    if let Some((left, right)) = KeptVoice::sample(pcm, channel.channels, channel.at as usize, true) {
                        heard[at * 2] += left * channel.level;
                        heard[at * 2 + 1] += right * channel.level;
                    }
                    channel.at += speed;
                }
            }
            channel.level = (channel.level + channel.step).clamp(0.0, 1.0);
        }
        self.voices.retain(|_, voice| {
            for at in start..end {
                let Some((left, right)) = KeptVoice::sample(&voice.pcm, voice.channels, voice.at as usize, voice.looped) else {
                    return false;
                };
                heard[at * 2] += left * voice.level.now * voice.sides.0;
                heard[at * 2 + 1] += right * voice.level.now * voice.sides.1;
                voice.at += voice.speed;
            }
            !voice.level.tick()
        });
    }

    /// (A plain WAVE file: two channels of 16 bits.)
    fn write_to(&self, path: &str) -> Result<(), crate::Error> {
        let heard = self.heard.borrow();
        std::fs::write(path, stereo_wave(&heard)).map_err(|why| crate::Error::Written { path: path.to_string(), why })
    }
}

/// A WAVE file of two channels of 16 bits, from samples -1 to 1, left and
/// right one after the other.
pub fn stereo_wave(samples: &[f32]) -> Vec<u8> {
    let size = samples.len() as u32 * 2;
    let mut bytes = Vec::with_capacity(44 + size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&OUTPUT_RATE.to_le_bytes());
    bytes.extend_from_slice(&(OUTPUT_RATE * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sound_is_as_loud_in_the_middle_and_dies_away_on_the_other_side() {
        assert_eq!(balance(0.0), (1.0, 1.0));
        assert_eq!(balance(-1.0), (1.0, 0.0));
        assert_eq!(balance(1.0), (0.0, 1.0));
        let (left, right) = balance(0.5);
        assert!(right == 1.0 && (left - 0.5).abs() < 1e-6);
        assert_eq!(balance(7.0), (0.0, 1.0));
    }

    #[test]
    fn a_voice_rises_fades_and_goes_once_stopped() {
        let mut level = Level::new(0.8, 4.0);
        assert_eq!(level.now, 0.0);
        for _ in 0..4 {
            assert!(!level.tick());
        }
        assert!((level.now - 0.8).abs() < 1e-6);
        level.fade(0.2, 3.0, false);
        for _ in 0..5 {
            assert!(!level.tick());
        }
        assert!((level.now - 0.2).abs() < 1e-6);
        level.fade(0.0, 2.0, true);
        assert!(!level.tick());
        assert!(level.tick());
        // (Stopped at once.)
        let mut level = Level::new(1.0, 0.0);
        level.fade(0.0, 0.0, true);
        assert!(level.tick());
    }

    #[test]
    fn a_voice_kept_is_one_named_looped_or_rising() {
        let voice = Voice::of(Source::Sound(3));
        assert!(!voice.kept());
        assert!(Voice { looped: true, ..voice.clone() }.kept());
        assert!(Voice { name: Some("rain".to_string()), ..voice.clone() }.kept());
        assert!(Voice { rise: 30.0, ..voice }.kept());
    }

    #[test]
    fn samples_past_the_end_are_none_unless_looped() {
        let pcm = [100, -100, 200, -200];
        assert_eq!(KeptVoice::sample(&pcm, 2, 2, false), None);
        assert!(KeptVoice::sample(&pcm, 2, 2, true).is_some_and(|(left, right)| left > 0.0 && right < 0.0));
        assert!(KeptVoice::sample(&pcm, 1, 3, false).is_some_and(|(left, right)| left == right));
    }

    #[test]
    fn what_is_kept_is_written_in_two_channels() {
        let bytes = stereo_wave(&[0.5, -0.5]);
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2);
        let wave = own_sounds::Wave::read(&bytes).expect("read back");
        assert_eq!((wave.channels, wave.frames()), (2, 1));
        assert!(wave.samples[0] > 0.0 && wave.samples[1] < 0.0);
    }
}
