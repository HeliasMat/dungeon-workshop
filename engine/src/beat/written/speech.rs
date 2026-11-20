//! How a `SpeechBeat` is written as its line (read by `beat::speech`).

use crate::beat::{SpeechBeat, Voice};
use crate::vocabulary::Vocabulary;

/// A switch as the reader takes it.
impl SpeechBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let word = |word: &str| word.to_string();
        Some(match self {
            SpeechBeat::Bubble { who, frames, text, voice } => {
                // (A line break is read from `\n`: one written as is could not come back.)
                if text.contains("\\n") {
                    return None;
                }
                let verb = match voice {
                    Voice::Said => "bubble",
                    Voice::Thought => "think",
                    Voice::Shouted => "shout",
                };
                vec![word(verb), who.to_string(), frames.to_string(), text.replace('\n', "\\n")]
            }
            // (Nothing or less is read as no pace: only a pace above it comes back.)
            SpeechBeat::PortraitSide { who, right } => vec![word("portrait"), who.to_string(), word(if *right { "right" } else { "left" })],
            // (A speed is read clamped: only one within it comes back.)
            SpeechBeat::TextSpeed(Some(speed)) if (0.1..=8.0).contains(speed) => vec![word("textspeed"), speed.to_string()],
            SpeechBeat::TextSpeed(Some(_)) => return None,
            SpeechBeat::TextSpeed(None) => vec![word("textspeed"), word("normal")],
            SpeechBeat::BoxSide { top } => vec![word("boxside"), word(if *top { "top" } else { "bottom" })],
            SpeechBeat::BoxAuto => vec![word("boxside"), word("auto")],
            SpeechBeat::Audience(group) if group.is_empty() => vec![word("audience"), word("off")],
            SpeechBeat::Audience(group) => std::iter::once(word("audience")).chain(group.iter().map(ToString::to_string)).collect(),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Frames;
    use crate::SoundId;
    use crate::beat::{Beat, SpeechBeat, Voice};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    #[test]
    fn every_speech_beat_written_is_read_back_the_same() {
        let pika = Who::Monster(25);
        let beats = vec![
            SpeechBeat::Bubble { who: pika, frames: Frames(60.0), text: "Oh my\nmy".into(), voice: Voice::Said },
            SpeechBeat::Bubble { who: pika, frames: Frames(12.5), text: "Hm".into(), voice: Voice::Thought },
            SpeechBeat::Bubble { who: pika, frames: Frames(30.0), text: "Hey!".into(), voice: Voice::Shouted },
            SpeechBeat::Said,
            SpeechBeat::PortraitSide { who: pika, right: true },
            SpeechBeat::PortraitSide { who: pika, right: false },
            SpeechBeat::TextSpeed(Some(1.5)),
            SpeechBeat::TextSpeed(None),
            SpeechBeat::BoxSide { top: true },
            SpeechBeat::BoxSide { top: false },
            SpeechBeat::BoxAuto,
            SpeechBeat::Mood { who: pika, face: Some(10) },
            SpeechBeat::Mood { who: pika, face: None },
            SpeechBeat::Typing { who: pika, sound: Some(SoundId(8964)) },
            SpeechBeat::Typing { who: pika, sound: None },
            SpeechBeat::AutoFace(true),
            SpeechBeat::AutoFace(false),
            SpeechBeat::Audience(vec![pika, Who::Monster(7)]),
            SpeechBeat::Audience(Vec::new()),
            SpeechBeat::HoldGaze { who: pika, held: false },
            SpeechBeat::React(true),
        ];
        for b in beats {
            let line = Beat::Speech(b.clone()).fields(&SAMPLE).expect("written");
            assert_eq!(Beat::parse(&line, &SAMPLE).ok(), Some(Beat::Speech(b)), "{line:?}");
        }
    }
}
