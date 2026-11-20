//! A beat written back as the engine's line, for as long as a film hands
//! the engine lines of fields (`film/src/emit.rs`): a word makes its beat,
//! typed, and its line is that beat's. Each kind of beat writes its own
//! (one file each, as `beat/` reads them); a beat with none is written by
//! the word as fields still.

use super::Beat;
use crate::vocabulary::Vocabulary;

mod body;
mod course;
mod data;
mod flow;
mod motion;
mod node;
mod place;
mod play;
mod screen;
mod sound;
mod speech;
mod view;

impl Beat {
    /// Its line, as `Beat::parse` reads it back the same with those words
    /// of the game; none for a beat no word makes yet.
    pub fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        match self {
            Beat::Body(beat) => beat.fields(words),
            Beat::Motion(beat) => beat.fields(words),
            Beat::View(beat) => beat.fields(words),
            Beat::Screen(beat) => beat.fields(words),
            Beat::Sound(beat) => beat.fields(words),
            Beat::Speech(beat) => beat.fields(words),
            Beat::Flow(beat) => beat.fields(words),
            Beat::Data(beat) => beat.fields(words),
            Beat::Play(beat) => beat.fields(words),
            Beat::Place(beat) => beat.fields(words),
            Beat::Node(beat) => beat.fields(words),
            Beat::Host(line) => Some(line.clone()),
            Beat::Course(course) => course.fields(words),
        }
    }

    /// Its line, only if it is read back the very same beat: what a film
    /// may hand the engine in its place.
    pub fn written(&self, words: &'static Vocabulary) -> Option<Vec<String>> {
        self.fields(words).filter(|line| Beat::parse(line, words).ok().as_ref() == Some(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EmoteId;
    use crate::Frames;
    use crate::SoundId;
    use crate::beat::{BodyBeat, Direction, FlowBeat, SoundBeat};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    /// A beat written as its line is read back the same.
    #[test]
    fn a_beat_written_is_read_back_the_same() {
        let emote = SAMPLE.emotes.first().map_or(0, |(_, id)| *id);
        for beat in [
            Beat::Flow(FlowBeat::Wait(Frames(0.0))),
            Beat::Flow(FlowBeat::Wait(Frames(90.0))),
            Beat::Sound(SoundBeat::Sound(SoundId(8964))),
            Beat::Body(BodyBeat::Jump { who: Who::Monster(25), height: 14.0, frames: Frames(12.5) }),
            Beat::Body(BodyBeat::Effect { who: Who::Another { species: 25, copy: 1 }, id: EmoteId(emote), frames: Some(Frames(30.0)), add: false }),
            Beat::Body(BodyBeat::Effect { who: Who::Character(0), id: EmoteId(emote), frames: None, add: false }),
            Beat::Body(BodyBeat::Effect { who: Who::Character(0), id: EmoteId(emote), frames: None, add: true }),
            Beat::Body(BodyBeat::Rise { who: Who::Monster(7), height: 20.0, frames: Frames(30.0) }),
            Beat::Body(BodyBeat::Tremble { who: Who::Monster(7), frames: Frames(12.0), strength: 1.5 }),
            Beat::Body(BodyBeat::Gone(Who::Character(10))),
            Beat::Body(BodyBeat::Face { who: Who::Monster(7), other: Who::Target }),
            Beat::Body(BodyBeat::Turn { who: Who::Monster(7), direction: Direction::Right }),
            Beat::Body(BodyBeat::Put { who: Who::Monster(7), at: glam::Vec2::new(300.0, -12.5).into(), direction: Direction::DownLeft }),
        ] {
            let fields = beat.fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&fields, &SAMPLE).ok(), Some(beat));
        }
    }
}
