//! The beats of how lines are said.

use engine::{Beat, Frames, SpeechBeat, Voice, Who};

use crate::film::Film;
use crate::screen::BREAK;

/// Lines said around someone no longer turn it (held), or do again.
pub fn hold_gaze(who: Who, held: bool) -> Beat {
    Beat::Speech(SpeechBeat::HoldGaze { who, held })
}

/// Someone shouts those words, for that many frames.
pub fn shout(who: Who, frames: f32, text: impl Into<String>) -> Beat {
    Beat::Speech(SpeechBeat::Bubble { who, frames: Frames(frames), text: text.into(), voice: Voice::Shouted })
}

impl Film {
    /// `shout <who> - <words>`: shouted long enough to be read, as the
    /// engine times a `-`; `{name:pikachu}` by its code.
    pub fn shout(&mut self, who: &str, text: &str) {
        let text = self.names_in(text);
        let frames = (40.0 + 4.0 * text.chars().count() as f32).min(360.0);
        self.on(who, |who| shout(who, frames, text.replace(BREAK, "\n")));
    }
}

/// Someone says those words in a bubble, for that many frames.
pub fn bubble(who: Who, frames: f32, text: impl Into<String>) -> Beat {
    Beat::Speech(SpeechBeat::Bubble { who, frames: Frames(frames), text: text.into(), voice: Voice::Said })
}

/// The dialogue box kept at the top of the screen, or at its bottom.
pub fn box_side(top: bool) -> Beat {
    Beat::Speech(SpeechBeat::BoxSide { top })
}

/// The dialogue box where it hides no speaker, as the game has it.
pub fn box_auto() -> Beat {
    Beat::Speech(SpeechBeat::BoxAuto)
}

/// At each line, its speaker and the one before turn to each other (or no more).
pub fn auto_face(on: bool) -> Beat {
    Beat::Speech(SpeechBeat::AutoFace(on))
}

/// Those listening, who look at who speaks at each line.
pub fn audience(who: Vec<Who>) -> Beat {
    Beat::Speech(SpeechBeat::Audience(who))
}

/// The lines that follow write themselves out so many times as fast (none: as usual).
pub fn text_speed(times: Option<f32>) -> Beat {
    Beat::Speech(SpeechBeat::TextSpeed(times))
}

impl Film {
    /// `bubble|think|shout <who> - <words>`: in a bubble long enough to be
    /// read, as the engine times a `-`; `{name:pikachu}` by its code. Its
    /// frames.
    pub fn bubble(&mut self, who: &str, text: &str, voice: Voice) -> f32 {
        let text = self.names_in(text);
        let frames = (40.0 + 4.0 * text.chars().count() as f32).min(360.0);
        let text = text.replace(BREAK, "\n");
        self.on(who, |who| Beat::Speech(SpeechBeat::Bubble { who, frames: Frames(frames), text: text.clone(), voice }));
        frames
    }
}
