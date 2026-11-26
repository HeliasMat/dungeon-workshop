//! Music and sounds, as Sky plays them.

use engine::{Beat, SoundBeat};

use super::Writer;
use crate::notes::{Fate, Kind};

impl Writer<'_> {
    /// A sound beat: a song, its volume, a sound, a jingle.
    pub(super) fn sound(&mut self, number: i32, word: Option<&str>, beat: &SoundBeat) {
        match beat {
            // The music lowered, or back: Sky's own volume (0 to 256), at once.
            SoundBeat::Volume(volume) => self.line(&format!("bgm_ChangeVolume(0, {});", (volume * 256.0).round() as i32)),
            SoundBeat::Music(Some(song)) => self.line(&format!("bgm_Play({});", song.0)),
            SoundBeat::Music(None) => self.line("bgm_FadeOut(30);"),
            SoundBeat::MusicIn { song: Some(song), frames } => self.line(&format!("bgm_PlayFadeIn({}, {}, 256);", song.0, frames.0.round() as i32)),
            SoundBeat::MusicIn { song: None, frames } | SoundBeat::MusicFade { frames, .. } => {
                self.line(&format!("bgm_FadeOut({});", frames.0.round() as i32));
            }
            SoundBeat::Sound(sound) | SoundBeat::SoundFrom { id: sound, .. } => self.line(&format!("se_Play({});", sound.0)),
            SoundBeat::OnHit { sound, .. } => match sound.as_ref() {
                SoundBeat::Sound(sound) => self.line(&format!("se_Play({});", sound.0)),
                _ => self.note(number, Kind::Sound, Fate::LeftOut, "a move's own sound on the hit", "nothing: it plays in dungeons only"),
            },
            SoundBeat::Jingle(jingle) => self.line(&format!("me_Play({jingle});")),
            other => self.unexported(number, word, &Beat::Sound(other.clone())),
        }
    }
}
