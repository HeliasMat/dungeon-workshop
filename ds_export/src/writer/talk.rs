//! What is said: a line in Sky's box with its portrait, the box's side.

use engine::{Beat, SpeechBeat, Who};

use super::Writer;
use crate::sky::{FACES, quoted};

impl Writer<'_> {
    /// A speech beat: a line closed, the box's side.
    pub(super) fn speech(&mut self, number: i32, word: Option<&str>, beat: &SpeechBeat) {
        match beat {
            // Bubbles (a line said in one, a crowd's chatter): Sky has none; the
            // time they take is kept (R-DS-LOSS).
            SpeechBeat::Bubble { .. } if matches!(word, Some("chatter")) || word.is_none() => {}
            SpeechBeat::Said => {
                // (Where a question would be asked, in the same box.)
                let at = self.body.len();
                self.line("message_Close();");
                self.calm();
                self.closed = Some(at);
            }
            SpeechBeat::BoxSide { top } => self.box_top = *top,
            SpeechBeat::BoxAuto => self.box_top = false,
            SpeechBeat::Mood { .. } => self.notes.push(format!("line {number}: a face changed in the middle of a line (Sky changes it between lines)")),
            other => self.unexported(number, word, &Beat::Speech(other.clone())),
        }
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
