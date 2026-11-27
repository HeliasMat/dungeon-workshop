//! The beats of the story's course: a question, its answers, a line only
//! the game reads.

use engine::{Beat, Course, Frames, SpeechBeat, Who};
use scenes::vocabulary::POKEMON;

use crate::film::Film;

/// A line said in the box: by someone (none: told), with a face of their
/// portrait, another name shown if one.
/// It goes by itself after so many frames, if timed.
pub fn say(entity: Option<Who>, face: Option<u16>, text: impl Into<String>, name: Option<String>, timed: Option<f32>) -> Beat {
    Beat::Course(Course::Say { entity, face, text: text.into(), name, timed: timed.map(Frames) })
}

/// The line on screen is said: the box closes.
pub fn said() -> Beat {
    Beat::Speech(SpeechBeat::Said)
}

/// A question and its answers.
pub fn ask(options: Vec<String>) -> Beat {
    Beat::Course(Course::Ask(options))
}

/// What follows is the answer of that place (0: the first).
pub fn answer(at: u16) -> Beat {
    Beat::Course(Course::Answer(at))
}

/// The answers of a question end: all go on from here.
pub fn rejoin() -> Beat {
    Beat::Course(Course::Rejoin)
}

/// A line of the game's own (`fight …`), its words as the game reads them.
pub fn host(line: Vec<String>) -> Beat {
    Beat::Course(Course::Host(line))
}

impl Film {
    /// A line only the game reads (`fight …`), the names in it put in by the
    /// film: someone by their code, `pikachu:Name` by the code too (a crowd's
    /// name is the game's own word there). A line whose numbers are found as
    /// it plays: filled then, by the engine.
    pub fn host(&mut self, line: Vec<String>) {
        let mut put = Vec::with_capacity(line.len());
        for (place, field) in line.iter().enumerate() {
            if place == 0 {
                put.push(field.clone());
            } else if let Some(code) = self.actors.get(field) {
                put.push(code.clone());
            } else if let Some((name, shown)) = field.split_once(':')
                && let Some(code) = self.actors.get(name).cloned()
            {
                put.push(format!("{code}:{}", self.names_in(shown)));
            } else {
                put.push(field.clone());
            }
        }
        self.beat(host(put));
    }
}

/// The pause after a line said, before what follows (unpaced).
const AFTER_SAID: u32 = 11;

impl Film {
    /// A line said in the box, as the game says it: who says it (a name the
    /// film gives, a code, `-` for none; `who:Nom` another name shown), the
    /// face of their portrait (`-`: none), the words (names in them put in,
    /// `{name:pikachu}`), timed so many frames, then the box closes and a
    /// short pause.
    pub fn line_said(&mut self, who: &str, face: &str, text: &str, frames: i32) {
        let (speaker, shown) = match who.split_once(':') {
            Some((speaker, shown)) => (speaker, Some(self.names_in(shown))),
            None => (who, None),
        };
        let code = match self.actors.get(speaker) {
            Some(code) => code.clone(),
            None => speaker.to_string(),
        };
        // (Mega evolved: his mega form's portrait, his own name.)
        let shown = shown.or_else(|| self.staging.megas.contains(speaker).then(|| format!("{{name:{code}}}")));
        let face = face.parse::<u16>().ok().or_else(|| POKEMON.face(face));
        let text = self.names_in(text).replace("\\n", "\n");
        self.beat(say(engine::speaker(Some(&code)), face, text, shown, Some(frames as f32)));
        self.beat(said());
        self.beat(super::wait(AFTER_SAID));
    }
}
