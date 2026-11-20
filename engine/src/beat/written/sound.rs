//! How a `SoundBeat` is written as its line (read by `beat::sound`).

use crate::beat::sound::{Played, Side};
use crate::beat::{SongId, SoundBeat};
use crate::vocabulary::Vocabulary;

/// A song as the reader takes it (none: `-`, which names no song).
fn song(song: &Option<SongId>) -> String {
    song.map_or("-".to_string(), |song| song.to_string())
}

/// So many frames as a wait is read (`6f`), if they are whole.
fn frames(frames: f32) -> Option<String> {
    (frames >= 0.0 && frames.fract() == 0.0).then(|| format!("{frames}f"))
}

impl SoundBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let word = |word: &str| word.to_string();
        Some(match self {
            SoundBeat::Music(id) => vec![word("music"), song(id)],
            SoundBeat::MusicIn { song: id, frames: time } => vec![word("music"), song(id), frames(time.0)?],
            SoundBeat::MusicBack => vec![word("music"), word("back")],
            // (Read in hundredths and clamped: only a speed that comes back the same.)
            SoundBeat::MusicSpeed(speed) => {
                let hundredths = speed * 100.0;
                if (hundredths / 100.0).clamp(0.25, 4.0) != *speed {
                    return None;
                }
                vec![word("musicspeed"), hundredths.to_string()]
            }
            SoundBeat::Sound(id) => vec![word("sound"), id.to_string()],
            SoundBeat::SoundFrom { id, from } => vec![word("sound"), id.to_string(), from.to_string()],
            // (No line reads a sound at a volume: the game's own.)
            SoundBeat::SoundAt { .. } => return None,
            SoundBeat::OnHit { who, sound } => [word("onhit"), who.to_string()].into_iter().chain(sound.fields(words)?).collect(),
            // (Read as frames, a sixtieth of a second each: only whole ones come back.)
            SoundBeat::MusicFrom { song: id, seconds } => {
                let time = (seconds * 60.0).round();
                if time / 60.0 != *seconds {
                    return None;
                }
                vec![word("musicfrom"), song(id), frames(time)?]
            }
            SoundBeat::Volume(volume) if (0.0..=1.0).contains(volume) => vec![word("volume"), volume.to_string()],
            SoundBeat::Volume(_) => return None,
            SoundBeat::Note { pitch, sound } => vec![word("note"), pitch.to_string(), sound.to_string()],
            SoundBeat::Play(sound) => played(sound)?,
            SoundBeat::Fade { name, level, frames: time, stop } => {
                let mut line = vec![word("fadesound"), name.clone(), level.to_string(), frames(time.0)?];
                line.extend(stop.then(|| word("stop")));
                line
            }
            SoundBeat::MusicOwn { name, frames: time, seconds } => match (time.0, seconds) {
                (0.0, 0.0) => vec![word("music"), format!("own:{name}")],
                (_, 0.0) => vec![word("music"), format!("own:{name}"), frames(time.0)?],
                (0.0, _) if ((seconds * 60.0).round() / 60.0) == *seconds => vec![word("musicfrom"), format!("own:{name}"), frames((seconds * 60.0).round())?],
                _ => return None,
            },
            _ => return None,
        })
    }
}

/// A sound played as said, as `playsound` reads it: only what differs from
/// a sound as it is.
fn played(sound: &Played) -> Option<Vec<String>> {
    let mut line = vec!["playsound".to_string(), sound.of.to_string()];
    // (A name of one word, that no other field is taken for.)
    if sound.name.as_ref().is_some_and(|name| name.is_empty() || name.contains(char::is_whitespace)) {
        return None;
    }
    let mut field = |name: &str, value: String| line.extend([name.to_string(), value]);
    if sound.volume != 1.0 {
        field("volume", sound.volume.to_string());
    }
    match &sound.pan {
        Side::Middle => {}
        Side::At(pan) => field("pan", pan.to_string()),
        Side::From { who, .. } => field("from", who.to_string()),
    }
    if matches!(sound.pan, Side::From { softer: false, .. }) {
        line.push("near".to_string());
    }
    let mut field = |name: &str, value: String| line.extend([name.to_string(), value]);
    if sound.pitch != 0 {
        field("pitch", sound.pitch.to_string());
    }
    if let Some(name) = &sound.name {
        field("as", name.clone());
    }
    if sound.rise.0 != 0.0 {
        field("rise", frames(sound.rise.0)?);
    }
    if sound.looped {
        line.push("loop".to_string());
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use crate::Frames;
    use crate::MoveId;
    use crate::SongId;
    use crate::SoundId;
    use crate::beat::sound::{Played, Side, SoundOf};
    use crate::beat::{Beat, SoundBeat};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    #[test]
    fn every_sound_beat_written_is_read_back_the_same() {
        let beats = vec![
            SoundBeat::Music(Some(SongId(73))),
            SoundBeat::Music(None),
            SoundBeat::MusicIn { song: Some(SongId(106)), frames: Frames(90.0) },
            SoundBeat::MusicIn { song: None, frames: Frames(30.0) },
            SoundBeat::MusicBack,
            SoundBeat::MusicFade { channel: 2, frames: Frames(45.5) },
            SoundBeat::MusicSpeed(1.2),
            SoundBeat::Ambient(Some(SongId(106))),
            SoundBeat::Ambient(None),
            SoundBeat::Sound(SoundId(8964)),
            SoundBeat::SoundFrom { id: SoundId(8974), from: Who::Monster(25) },
            SoundBeat::Jingle(3),
            SoundBeat::MoveSound(MoveId(33)),
            SoundBeat::OnHit { who: Who::Monster(25), sound: Box::new(SoundBeat::Sound(SoundId(8964))) },
            SoundBeat::OnHit { who: Who::Monster(25), sound: Box::new(SoundBeat::MoveSound(MoveId(33))) },
            SoundBeat::MusicFrom { song: Some(SongId(73)), seconds: 1.5 },
            SoundBeat::Volume(0.5),
            SoundBeat::Note { pitch: -3, sound: SoundId(8964) },
            SoundBeat::Note { pitch: 14, sound: SoundId(0x3F02) },
            SoundBeat::Play(Played::of(SoundOf::Game(SoundId(8964)))),
            SoundBeat::Play(Played {
                of: SoundOf::Own("soft_wind".to_string()),
                volume: 0.5,
                pan: Side::At(-0.75),
                pitch: -3,
                looped: true,
                name: Some("wind".to_string()),
                rise: Frames(120.0),
            }),
            SoundBeat::Play(Played { pan: Side::From { who: Who::Monster(25), softer: false }, ..Played::of(SoundOf::Move(MoveId(33))) }),
            SoundBeat::Play(Played { pan: Side::From { who: Who::Character(0), softer: true }, ..Played::of(SoundOf::Song(SongId(106))) }),
            SoundBeat::Fade { name: "wind".to_string(), level: 0.0, frames: Frames(120.0), stop: true },
            SoundBeat::Fade { name: "rain".to_string(), level: 0.25, frames: Frames(30.0), stop: false },
            SoundBeat::MusicOwn { name: "my_song".to_string(), frames: Frames::ZERO, seconds: 0.0 },
            SoundBeat::MusicOwn { name: "my_song".to_string(), frames: Frames(90.0), seconds: 0.0 },
            SoundBeat::MusicOwn { name: "my_song".to_string(), frames: Frames::ZERO, seconds: 1.5 },
            SoundBeat::OnHit { who: Who::Monster(25), sound: Box::new(SoundBeat::Play(Played::of(SoundOf::Own("clac".to_string())))) },
        ];
        for b in beats {
            let line = Beat::Sound(b.clone()).fields(&SAMPLE).expect("written");
            assert_eq!(Beat::parse(&line, &SAMPLE).ok(), Some(Beat::Sound(b)), "{line:?}");
        }
    }
}
