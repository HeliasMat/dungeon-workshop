//! The sound being made: its versions undone and redone, written, heard.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use shell::sound::Hears;

use super::now_seed;
use super::presets::eased;
use super::{Measure, Sound, SoundMaking, Synth};
use workshop_base::done::Done;
use workshop_base::session::Session;

/// Whether sounds are made as an expert (kept from one time to the next).
fn expert_file(session: &Session) -> PathBuf {
    session.root().join("target").join("workshop").join("sounds_expert.txt")
}

impl SoundMaking {
    pub fn new(session: &Arc<Session>, word: &str) -> SoundMaking {
        let sound = Sound::of(Synth::of_kind("Blip", now_seed()));
        SoundMaking {
            session: session.clone(),
            word: word.to_string(),
            versions: vec![("Blip".to_string(), sound.clone())],
            sound,
            voice: 0,
            undone: Vec::new(),
            redone: Vec::new(),
            played: None,
            taken: None,
            handle: None,
            looped: false,
            play_on_change: true,
            locks: [false; 6],
            variants: 4,
            reference: 0,
            rendered: None,
            picture: None,
            saved: false,
            said: None,
            expert: std::fs::read_to_string(expert_file(session)).is_ok_and(|said| said.trim() == "1"),
            base: Sound::of(Synth::default()),
            kind: "Blip".to_string(),
            words: [0.0; 7],
            easy_undone: Vec::new(),
            text: None,
            embedded: false,
        }
        .simply_begun()
    }

    /// Its beginner's state begun from what it is.
    fn simply_begun(mut self) -> SoundMaking {
        self.base = self.sound.clone();
        self.words = [0.0; 7];
        self
    }

    /// Made as an expert or simply (kept).
    pub(super) fn set_expert(&mut self, expert: bool) {
        if expert == self.expert {
            return;
        }
        self.expert = expert;
        if !expert {
            (self.base, self.words) = (self.sound.clone(), [0.0; 7]);
        }
        if let Some(folder) = expert_file(&self.session).parent() {
            _ = std::fs::create_dir_all(folder);
        }
        _ = std::fs::write(expert_file(&self.session), if expert { "1" } else { "0" });
    }

    /// The beginner's sound: chosen (a kind), moved (the words), kept to be undone.
    pub(super) fn ease(&mut self, base: Option<(Sound, String)>, words: [f32; 7]) {
        self.easy_undone.push((self.base.clone(), self.kind.clone(), self.words));
        if let Some((base, kind)) = base {
            (self.base, self.kind) = (base, kind);
        }
        self.words = words;
        let sound = eased(&self.base, &self.words);
        let said = self.kind.clone();
        self.set(sound, &said);
    }

    /// One of the project's own sounds, to be made again: by what made it, if it was made here.
    pub fn open(session: &Arc<Session>, word: &str) -> SoundMaking {
        let mut made = SoundMaking::new(session, word);
        if let Ok(text) = std::fs::read_to_string(params_file(session, word)) {
            made.sound = Sound::read(&text);
            for voice in &mut made.sound.voices {
                voice.synth = voice.synth.tidy();
            }
            made.versions = vec![("saved".to_string(), made.sound.clone())];
            made.base = made.sound.clone();
            made.saved = true;
        } else {
            made.said = Some("not made here: starting from a new sound".to_string());
        }
        made
    }

    pub(super) fn synth(&self) -> Synth {
        self.sound.voices[self.voice].synth
    }

    /// A sound to be changed inside another maker (a song's instrument, a drum).
    pub fn editing(session: &Arc<Session>, sound: Sound) -> SoundMaking {
        let mut made = SoundMaking::new(session, "instrument");
        made.versions = vec![("at the start".to_string(), sound.clone())];
        made.base = sound.clone();
        made.sound = sound;
        made.embedded = true;
        made
    }

    /// The sound as it is now.
    pub fn sound(&self) -> &Sound {
        &self.sound
    }

    /// A new version, the one before kept to be undone.
    pub(super) fn set(&mut self, sound: Sound, said: &str) {
        self.undone.push(std::mem::replace(&mut self.sound, sound));
        self.redone.clear();
        self.versions.insert(0, (said.to_string(), self.sound.clone()));
        self.versions.truncate(30);
        self.voice = self.voice.min(self.sound.voices.len() - 1);
        self.saved = false;
    }

    /// The voice chosen made another (its locked groups kept), as a new version.
    pub(super) fn set_synth(&mut self, synth: Synth, said: &str) {
        let mut sound = self.sound.clone();
        let old = sound.voices[self.voice].synth;
        sound.voices[self.voice].synth = synth.kept(&old, &self.locks).tidy();
        self.set(sound, said);
    }

    /// Its samples as they are now (made again only when it changed).
    pub(super) fn rendered(&mut self) -> (Vec<f32>, Measure) {
        if self.rendered.as_ref().is_none_or(|(sound, ..)| *sound != self.sound) {
            let (samples, measure) = self.sound.render(&self.session.own_sounds);
            self.rendered = Some((self.sound.clone(), samples, measure));
        }
        let (_, samples, measure) = self.rendered.as_ref().expect("made");
        (samples.clone(), *measure)
    }

    /// Written where the films' own sounds are; what made it kept, to make it again.
    fn save_as(&mut self, word: &str, sound: &Sound) -> Result<(), String> {
        if rules::songs::NOISES.iter().any(|noise| noise.word == word) {
            return Err(format!("\"{word}\" is a sound of the game: another word"));
        }
        let folder = self.session.library().sounds();
        std::fs::create_dir_all(&folder).map_err(|why| why.to_string())?;
        let (stereo, _) = sound.render(&self.session.own_sounds);
        std::fs::write(folder.join(format!("{word}.wav")), shell::sound::stereo_wave(&stereo)).map_err(|why| why.to_string())?;
        let file = params_file(&self.session, word);
        if let Some(folder) = file.parent() {
            _ = std::fs::create_dir_all(folder);
        }
        _ = std::fs::write(file, sound.written());
        Ok(())
    }

    fn save(&mut self) -> Result<String, String> {
        let word = film::words::plain(&self.word.trim().replace([' ', '-'], "_"));
        if word.is_empty() {
            return Err("a word first".to_string());
        }
        let sound = self.sound.clone();
        self.save_as(&word, &sound)?;
        self.word = word.clone();
        self.saved = true;
        Ok(word)
    }

    /// So many of it, each a little changed (`step_1`, `step_2`…): for a sound heard often.
    pub(super) fn save_variants(&mut self) -> Result<Vec<String>, String> {
        let word = film::words::plain(&self.word.trim().replace([' ', '-'], "_"));
        if word.is_empty() {
            return Err("a word first".to_string());
        }
        let mut words = Vec::new();
        let seed = now_seed();
        for number in 1..=self.variants {
            let mut sound = self.sound.clone();
            for (at, voice) in sound.voices.iter_mut().enumerate() {
                if number > 1 {
                    voice.synth = voice.synth.mutated(seed.wrapping_add(number * 7919 + at as u32)).kept(&voice.synth, &self.locks);
                }
            }
            let named = format!("{word}_{number}");
            self.save_as(&named, &sound)?;
            words.push(named);
        }
        Ok(words)
    }
}

/// Where what made a sound is kept.
fn params_file(session: &Session, word: &str) -> PathBuf {
    session.root().join("target").join("workshop").join("sounds").join(format!("{word}.txt"))
}

pub fn open_audio(session: &Session, audio: &mut Option<Box<dyn Hears>>) {
    if audio.is_none() {
        *audio = shell::sound::Audio::open_in(session.data(), session.library().clone()).map(|audio| Box::new(audio) as Box<dyn Hears>);
    }
}

/// The sound heard as it is now, the playhead started.
pub(super) fn hear(making: &mut SoundMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    open_audio(&making.session, audio);
    let (stereo, _) = making.rendered();
    let seconds = stereo.len() as f32 / 2.0 / shell::audio::OUTPUT_RATE as f32;
    if let Some(audio) = audio.as_mut() {
        audio.samples(stereo.into_iter().map(|sample| (sample * 32767.0) as i16).collect(), volume);
        making.played = Some((Instant::now(), seconds));
    } else {
        making.said = Some("no sound: the sound card or the game's sounds are missing".to_string());
    }
}

pub(super) fn undo_sound(making: &mut SoundMaking) {
    if let Some(before) = making.undone.pop() {
        making.redone.push(std::mem::replace(&mut making.sound, before));
        making.voice = making.voice.min(making.sound.voices.len() - 1);
        making.saved = false;
    }
}

pub(super) fn redo_sound(making: &mut SoundMaking) {
    if let Some(after) = making.redone.pop() {
        making.undone.push(std::mem::replace(&mut making.sound, after));
        making.voice = making.voice.min(making.sound.voices.len() - 1);
        making.saved = false;
    }
}

pub(super) fn save_sound(making: &mut SoundMaking) -> Option<Done> {
    match making.save() {
        Ok(word) => {
            making.said = Some(format!("written: sound {word}"));
            Some(Done::Sound(word))
        }
        Err(why) => {
            making.said = Some(why);
            None
        }
    }
}
