//! What is said: a line in Sky's box with its portrait, the box's side; a
//! line said in a bubble as a box that closes by itself, a crowd's chatter
//! as emotes over it.

use engine::{Beat, EmoteId, SpeechBeat, Voice, Who};

use super::Writer;
use crate::notes::{Fate, Kind};
use crate::sky::{FACES, quoted};

/// A chatter's bubble as the emote Sky shows over the one saying it: `!`,
/// `?`, laughing, joy, or sweat for a sigh (Sky's emote numbers).
fn chatter_emote(text: &str) -> u16 {
    let text = text.trim();
    let lower = text.to_lowercase();
    if lower.contains("ha") && text.ends_with('!') {
        93
    } else if lower.starts_with("yes") {
        90
    } else if text.ends_with('?') {
        89
    } else if text.ends_with('!') {
        88
    } else {
        92
    }
}

impl Writer<'_> {
    /// A speech beat: a line closed, the box's side, a bubble.
    pub(super) fn speech(&mut self, number: i32, word: Option<&str>, beat: &SpeechBeat) {
        match beat {
            // A crowd's chatter: Sky has no bubbles, each one's emote over him instead (R-DS-LOSS).
            SpeechBeat::Bubble { who, text, .. } if matches!(word, Some("chatter")) => self.effect(number, *who, EmoteId(chatter_emote(text))),
            // A line said in a bubble: Sky's box, closing by itself after the bubble's time (R-DS-LOSS).
            SpeechBeat::Bubble { who, frames, text, voice } => self.bubble_line(number, word, *who, frames.0, text, *voice),
            SpeechBeat::Said => {
                // (Where a question would be asked, in the same box.)
                let at = self.body.len();
                self.line("message_Close();");
                self.calm();
                self.closed = Some(at);
            }
            SpeechBeat::BoxSide { top } => self.box_top = *top,
            SpeechBeat::BoxAuto => self.box_top = false,
            SpeechBeat::Mood { .. } => {
                self.note(number, Kind::Speech, Fate::LeftOut, "a face changed in the middle of a line", "Sky changes faces between lines only")
            }
            other => self.unexported(number, word, &Beat::Speech(other.clone())),
        }
    }

    /// A line said in a bubble (`who (bubble): …`, `bubble`, `think`,
    /// `shout`), as Sky can: a line of its box, the face its brackets give,
    /// closed by itself once the bubble's time is over
    /// (`message_SetWaitMode`, as Sky's endings time theirs), then the box
    /// waits for its key again. No two at once: one after the other.
    fn bubble_line(&mut self, number: i32, word: Option<&str>, who: Who, frames: f32, text: &str, voice: Voice) {
        let face = self.bubble_faces.get(&number).copied();
        let message = if voice == Voice::Thought { "a thought in a bubble" } else { "a line in a bubble" };
        self.note(number, Kind::Bubble, Fate::Changed, message, "a line of Sky's box that closes by itself");
        // (Its words cut to the box, not to a bubble.)
        let text = ::film::screen::wrap(&text.replace('\n', " ")).join("\n");
        let frames = frames.round().max(30.0) as i32;
        self.line(&format!("message_SetWaitMode({frames}, 0);"));
        let said = if voice == Voice::Thought { Some("think") } else { word.filter(|word| *word != "bubble" && *word != "shout") };
        self.say(number, said, Some(who), face, &text);
        self.line("message_Close();");
        self.line("message_SetWaitMode(-1, -1);");
        self.calm();
        // (The bubble's time is the box's: the film's wait for it is left out.)
        self.said_over = true;
        self.talking = false;
    }

    /// A line said: its face (or the species' normal one, or its name
    /// alone), its box, as Sky's message operations show it.
    pub(super) fn say(&mut self, number: i32, word: Option<&str>, entity: Option<Who>, face: Option<u16>, text: &str) {
        // (`{value}`: Sky's texts print no variable of a script: the number as last written before.)
        let mut text = text.to_string();
        for (name, value) in &self.values {
            text = text.replace(&format!("{{{name}}}"), &format!("{value}"));
        }
        let text = quoted(text.trim_end());
        let text = if text.starts_with(' ') || text.starts_with('[') { text } else { format!(" {text}") };
        match (entity, word) {
            // Someone joins, as Sky tells it: its jingle, the line centred in the box.
            (None, Some("join")) => {
                self.line("me_Play(5);");
                self.line("message_ResetActor();");
                self.line(&format!("message_Mail(\"[CN]{}\");", text.trim_start()));
                self.line("WaitMe(5);");
            }
            (None, Some("narrate")) if text.trim_start().starts_with("[CN]") => {
                self.line(&format!("message_Explanation(\"{}\");", text.trim_start()));
            }
            (None, _) => {
                self.line("message_ResetActor();");
                self.line(&format!("message_Talk(\"{text}\");"));
            }
            // One drawn as the stand-in: no portrait, no name of the stand-in's; its own name said.
            (Some(who), _) if self.stand_in(who) => {
                if self.who(number, who).is_none() {
                    return;
                }
                let name = self.film_names.get(&who).map_or_else(|| "???".to_string(), |name| capitalised(name));
                self.line("message_ResetActor();");
                self.line(&format!("message_Talk(\" {}:{text}\");", quoted(&name)));
            }
            (Some(who), _) => {
                let Some(actor) = self.who(number, who) else { return };
                let kind = self.actors.get(&self.became.get(&who).copied().unwrap_or(who)).map_or(0, |one| one.kind);
                let face = face.unwrap_or(0);
                let shown = if self.game.has_face(kind, face) {
                    Some(face)
                } else if self.game.has_face(kind, 0) {
                    Some(0)
                } else {
                    None
                };
                match shown {
                    Some(face) => {
                        let face = FACES.get(usize::from(face)).map_or(face.to_string(), |name| format!("FACE_{name}"));
                        let place = if self.box_top { "FACE_POS_TOP_L_FACEINW" } else { "FACE_POS_STANDARD" };
                        self.line(&format!("message_SetFace({actor}, {face}, {place});"));
                    }
                    None => self.line(&format!("message_SetActor({actor});")),
                }
                if word == Some("think") {
                    self.line(&format!("message_Monologue(\"{text}\");"));
                } else {
                    self.line(&format!("message_Talk(\"{text}\");"));
                }
            }
        }
        self.talking = true;
    }
}

/// A film's name as a box shows a speaker's (`pichu_spiky`: “Pichu spiky”).
fn capitalised(name: &str) -> String {
    let name = name.replace('_', " ");
    let mut letters = name.chars();
    letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chatter_s_words_are_sky_s_emotes() {
        assert_eq!(chatter_emote("!"), 88);
        assert_eq!(chatter_emote("Really?"), 89);
        assert_eq!(chatter_emote("Ha ha!"), 93);
        assert_eq!(chatter_emote("Yes!"), 90);
        assert_eq!(chatter_emote("..."), 92);
        assert_eq!(capitalised("pichu_spiky"), "Pichu spiky");
    }
}
