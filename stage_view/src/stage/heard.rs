//! What is heard of a scene, moves and their effects first: listed in
//! `Sim::heard`, for who plays it.

use engine::{Beat, Played, SHOT_FRAMES, Side, SongId, SoundBeat, SoundOf, SpeechBeat, Who, pan_of};
use scenes::commands::{MoveShown, stage_move};
use shell::sound::{Source, Tune, Voice};

use super::sim::{Facts, Heard, ShownEffect, Sim, tune_heard};

/// How many blows are heard in a frame.
const BLOWS_HEARD: usize = 2;
/// What the game shows of a stat that changes, of paralysis, of confusion:
/// heard as moves' effects are.
const HEARD_SIGHTS: [u16; 15] = [2, 13, 14, 324, 395, 396, 397, 399, 400, 401, 403, 404, 423, 425, 0];
/// How loud an effect's sound is: under the blow it goes with.
const EFFECT_VOLUME: f32 = 0.7;
/// The point of a move that leaves it to its effect.
const ANY_POINT: u8 = 255;

/// A sound at that volume, from that side.
fn sound_at(id: u16, volume: f32, pan: f32) -> Heard {
    if pan == 0.0 { Heard::Sound { id, volume, pitch: 1.0 } } else { Heard::Play(Voice { volume, pan, ..Voice::of(Source::Sound(id)) }) }
}

/// The game's song of the music.
fn tune(song: Option<SongId>) -> Option<Tune> {
    song.map(|song| Tune::Game(song.get()))
}

impl Sim {
    /// A sound of the game, from the side of who makes it.
    fn sound(&mut self, id: u16, volume: f32, from: Who) {
        let pan = self.side_of(from);
        self.heard.push(sound_at(id, volume, pan));
    }

    /// Where someone is heard from, left (-1) to right (1) of where the
    /// view looks.
    fn side_of(&self, who: Who) -> f32 {
        self.stage.cast.get(&who).map_or(0.0, |actor| pan_of(actor.position.x - self.focus.unwrap_or_default().x))
    }

    /// A sound as said, as it is heard: what it is, from where.
    fn played(&self, played: Played, facts: Facts) -> Heard {
        let speed = played.speed();
        let source = match played.of {
            SoundOf::Game(id) => Source::Sound(id.get()),
            SoundOf::Move(id) => Source::Sound(MoveShown::of(facts.tables, id.get(), None).sound),
            SoundOf::Song(id) => Source::Song(id.get()),
            SoundOf::Own(name) => Source::Own(name),
        };
        let pan = match played.pan {
            Side::Middle => 0.0,
            Side::At(pan) => pan,
            Side::From { who, .. } => self.side_of(who),
        };
        Heard::Play(Voice { source, volume: played.volume, pan, speed, looped: played.looped, name: played.name, rise: played.rise.get() })
    }

    /// A move, as in dungeons, and what is heard of it.
    pub(super) fn move_used(&mut self, who: Who, id: u16, target: Option<Who>, facts: Facts) {
        self.blows_heard += 1;
        let species = matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. }).then(|| facts.art.kin(self.cast.who(who).0));
        let shown = MoveShown::of(facts.tables, id, species);
        let [before, strike, on_target, flying] = shown.effects;
        let audible = self.blows_heard <= BLOWS_HEARD;
        if audible {
            self.sound(shown.sound, 1.0, who);
        }
        for effect in [before, strike].into_iter().filter(|effect| *effect != 0) {
            self.user_points.insert(who, shown.point);
            self.heard_effects.insert(effect);
        }
        if target.is_some() {
            self.heard_effects.insert(on_target);
        }
        let wait = facts.art.effect(before).map_or(0.0, |art| art.ticks);
        let thrown = facts.art.effect(flying).filter(|_| target.is_some()).map(|art| art.sound);
        // (The blow lands on the step its user's sprite marks, if it does.)
        let way = self.stage.cast.get(&who).map_or(0, |actor| super::pose::way_of(actor.direction));
        let hit = self.shown.get(&who).and_then(|own| own.hit_of(shown.group, way));
        stage_move(&mut self.stage, who, target, &shown, wait, thrown.is_some(), hit);
        if let Some(sound) = thrown.filter(|_| audible) {
            self.sound(sound, 1.0, who);
        }
    }

    /// What a move shows as it lands, on someone, without a blow.
    pub(super) fn effect_of_move(&mut self, who: Who, id: u16, facts: Facts) {
        let [before, strike, on_target, _] = MoveShown::of(facts.tables, id, None).effects;
        if let Some(effect) = [on_target, strike, before].into_iter().find(|effect| *effect != 0) {
            self.heard_effects.insert(effect);
            self.stage.effect(who, effect, None);
        }
    }

    /// What a move throws, from someone to someone else, without a blow.
    pub(super) fn shot_of_move(&mut self, from: Who, to: Who, id: u16, frames: Option<f32>, facts: Facts) {
        let shown = MoveShown::of(facts.tables, id, None);
        let [_, _, on_target, flying] = shown.effects;
        if let Some(sound) = facts.art.effect(flying).map(|art| art.sound) {
            self.sound(sound, 1.0, from);
        }
        let far = self.stage.cast.get(&from).zip(self.stage.cast.get(&to)).map_or(0.0, |(from, to)| from.position.distance(to.position));
        self.heard_effects.insert(on_target);
        self.stage.shoot_as(from, to, flying, on_target, frames.unwrap_or((far / shown.flight).max(SHOT_FRAMES)), shown.lob, false);
    }

    /// What is heard.
    pub(super) fn heard(&mut self, beat: Beat, facts: Facts) {
        let loud = self.loudness;
        let heard = match beat {
            Beat::Sound(SoundBeat::Music(song)) => {
                self.songs = [tune(song), self.songs[0].take()];
                self.song_start = self.clock;
                Heard::Song { channel: 0, song: song.map(SongId::get), level: loud, step: 0.0 }
            }
            Beat::Sound(SoundBeat::MusicFrom { song, seconds }) => {
                self.songs = [tune(song), self.songs[0].take()];
                self.song_start = self.clock - seconds * 60.0;
                Heard::SongFrom { channel: 0, song: song.map(SongId::get), level: loud, seconds }
            }
            Beat::Sound(SoundBeat::MusicIn { song, frames }) => {
                self.songs = [tune(song), self.songs[0].take()];
                self.song_start = self.clock;
                Heard::Song { channel: 0, song: song.map(SongId::get), level: 0.0, step: loud / frames.0.max(1.0) }
            }
            Beat::Sound(SoundBeat::MusicOwn { name, frames, seconds }) => {
                self.songs = [Some(Tune::Own(name.clone())), self.songs[0].take()];
                self.song_start = self.clock - seconds * 60.0;
                let (level, step) = if frames.0 > 0.0 { (0.0, loud / frames.0) } else { (loud, 0.0) };
                Heard::OwnSong { channel: 0, name, level, step, seconds }
            }
            Beat::Sound(SoundBeat::Play(played)) => self.played(played, facts),
            Beat::Sound(SoundBeat::Fade { name, level, frames, stop }) => Heard::FadeVoice { name, level, frames: frames.0, stop },
            Beat::Sound(SoundBeat::Volume(level)) => {
                self.loudness = level;
                Heard::Volume { channel: 0, level }
            }
            Beat::Sound(SoundBeat::MusicBack) => {
                self.songs.swap(0, 1);
                self.song_start = self.clock;
                tune_heard(self.songs[0].clone(), loud, 0.0)
            }
            Beat::Speech(SpeechBeat::Bubble { who, frames, text, voice }) => {
                let text = self.cast.names_in(&text);
                self.stage.bubble(who, frames.0, text, voice);
                return;
            }
            Beat::Sound(SoundBeat::MoveSound(id)) => Heard::Sound { id: MoveShown::of(facts.tables, id.0, None).sound, volume: 1.0, pitch: 1.0 },
            Beat::Sound(SoundBeat::Ambient(song)) => Heard::Song { channel: 1, song: song.map(SongId::get), level: loud, step: 0.0 },
            Beat::Sound(SoundBeat::MusicSpeed(speed)) => Heard::Speed(speed),
            Beat::Sound(SoundBeat::MusicFade { channel, frames }) => {
                for fading in [0, 1].into_iter().filter(|fading| channel == 2 || channel == *fading) {
                    self.heard.push(Heard::Fade { channel: fading as usize, frames: frames.0 });
                }
                return;
            }
            Beat::Sound(SoundBeat::Sound(id)) => Heard::Sound { id: id.0, volume: 1.0, pitch: 1.0 },
            Beat::Sound(SoundBeat::Note { pitch, sound }) => Heard::Sound { id: sound.0, volume: 1.0, pitch: 2.0_f32.powf(pitch as f32 / 12.0) },
            Beat::Sound(SoundBeat::SoundAt { id, volume, pan }) => sound_at(id.0, volume, pan),
            Beat::Sound(SoundBeat::Jingle(id)) => Heard::Jingle(id),
            _ => return,
        };
        self.heard.push(heard);
    }

    /// Effects come on who has them, and go.
    pub(super) fn effects_pass(&mut self, frames: f32, facts: Facts) {
        // (An effect whose time is up, as the game's own has it, leaves the
        // stage too.)
        let emotes = &self.stage.emotes;
        self.effects.retain(|(who, id), _| emotes.effects_of(*who).iter().any(|effect| effect.id == *id));
        let wanted: Vec<(Who, u16, bool)> =
            self.stage.emotes.effects.iter().flat_map(|(who, shown)| shown.iter().map(|effect| (*who, effect.id, effect.left.is_some()))).collect();
        for (who, id, timed) in wanted {
            if !self.effects.contains_key(&(who, id))
                && let Some(art) = facts.art.effect(id)
            {
                // (Where the move has it, on who uses it; a move's effects
                // are heard as they start.)
                let asked = self.user_points.remove(&who).filter(|point| *point != ANY_POINT);
                if self.heard_effects.remove(&id) || HEARD_SIGHTS.contains(&id) {
                    self.blows_heard += 1;
                    if self.blows_heard <= BLOWS_HEARD {
                        self.sound(art.sound, EFFECT_VOLUME, who);
                    }
                }
                self.effects.insert((who, id), ShownEffect { point: asked.unwrap_or(art.point), art, gone: 0.0 });
            }
            if let Some(shown) = self.effects.get_mut(&(who, id)) {
                shown.gone += frames * self.stage.cast.get(&who).map_or(1.0, |actor| actor.own_time());
                if !timed && !shown.art.repeats && shown.gone >= shown.art.ticks {
                    self.effects.remove(&(who, id));
                    self.stage.emotes.effect_over(who, id);
                }
            }
        }
    }
}

/// How long each sound lasts, in frames, for `wait sound` (see
/// `Director::hear_sounds`): the game's from its library, a move's as the
/// game plays it, ours from their files (none: not known).
pub fn sound_lengths(art: &shell::art::Art, tables: &rules::tables::Tables) -> impl Fn(&SoundOf) -> Option<f32> + Send + Sync + 'static {
    let game = art.sound_lengths();
    let moves: Vec<u16> = (0..tables.move_sounds.len().max(tables.move_motions.len())).map(|id| MoveShown::of(tables, id as u16, None).sound).collect();
    let (folder, project) = (art.folder.clone(), art.project.clone());
    move |of| match of {
        SoundOf::Game(id) => game(id.get()),
        // (A move the game plays silent: no wait.)
        SoundOf::Move(id) => match moves.get(usize::from(id.get())) {
            Some(sound) if sound & 0xFF == 0 => Some(0.0),
            sound => sound.and_then(|sound| game(*sound)),
        },
        SoundOf::Own(name) => shell::sound::own_length(&folder, &project, name),
        SoundOf::Song(_) => None,
    }
}
