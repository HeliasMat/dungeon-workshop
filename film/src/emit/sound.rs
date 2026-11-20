//! The beats heard: sounds, songs.

use engine::{Beat, Frames, MoveId, SongId, SoundBeat, SoundId, Who};
use scenes::vocabulary::POKEMON;

use crate::film::Film;

/// A sound of the game.
pub fn sound(id: SoundId) -> Beat {
    Beat::Sound(SoundBeat::Sound(id))
}

/// The sound a note is played with, untold.
pub const NOTE_SOUND: SoundId = SoundId(0x3F02);

/// A note played (`engine::note_named`), with that sound of the game.
pub fn note(pitch: i32, sound: SoundId) -> Beat {
    Beat::Sound(SoundBeat::Note { pitch, sound })
}

/// A little tune of the game, by its number.
pub fn jingle(id: u16) -> Beat {
    Beat::Sound(SoundBeat::Jingle(id))
}

/// The music on that channel fades away in so many frames.
pub fn music_fade(channel: u8, frames: f32) -> Beat {
    Beat::Sound(SoundBeat::MusicFade { channel, frames: Frames(frames) })
}

impl Film {
    /// A sound of the game, by its number or its name (`click`, `hop`). One
    /// the game has not: told.
    pub fn sound(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();
        match name.parse::<u16>().ok().or_else(|| POKEMON.sound(name)) {
            Some(id) => self.beat(sound(SoundId(id))),
            None => self.not_a(name, "a sound of the game"),
        }
    }
}

/// A song of the game plays (none: the music stops).
pub fn music(song: Option<SongId>) -> Beat {
    Beat::Sound(SoundBeat::Music(song))
}

/// A song plays from that many seconds into it (none: the music stops).
pub fn music_from(song: Option<SongId>, seconds: f32) -> Beat {
    Beat::Sound(SoundBeat::MusicFrom { song, seconds })
}

/// What a move sounds like, by the move's number.
pub fn move_sound(id: MoveId) -> Beat {
    Beat::Sound(SoundBeat::MoveSound(id))
}

/// A sound heard as the next blow lands on someone.
pub fn on_hit(who: Who, sound: SoundBeat) -> Beat {
    Beat::Sound(SoundBeat::OnHit { who, sound: Box::new(sound) })
}
