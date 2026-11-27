//! What is said: a line in Sky's box with its portrait, the box's side; a
//! line said in a bubble as a box that closes by itself, a crowd's chatter
//! as emotes over it. With the mega patch (`film::ds::patches`): a face
//! changing in a line (`[FACE:n]`), bold words (`[BS]…[BR]`), a bubble's
//! box see-through.

use engine::{Beat, EmoteId, SpeechBeat, Voice, Who};

use ::film::ds::patches::{BOLD, FACE_MID_TEXT, PORTRAIT_ALONE, SEE_THROUGH};

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

/// `*words*` as BoldText writes them, `[BS]words[BR]`; a lone star is left.
fn bold(text: &str) -> String {
    let parts: Vec<&str> = text.split('*').collect();
    let pairs = (parts.len() - 1) / 2;
    let mut out = String::new();
    for (at, part) in parts.iter().enumerate() {
        match at {
            at if at % 2 == 1 && at < pairs * 2 => out.push_str(&format!("[BS]{part}[BR]")),
            at if at > pairs * 2 => out.push_str(&format!("*{part}")),
            _ => out.push_str(part),
        }
    }
    out
}

/// A line's words cut where its face changes (`[FACE:n]`): the first
/// part's face none (the line's own), each other's its number.
fn face_parts(text: &str) -> Vec<(Option<u16>, String)> {
    let mut parts = vec![(None, String::new())];
    let mut rest = text;
    while let Some(at) = rest.find("[FACE:") {
        let Some(close) = rest[at..].find(']').map(|close| at + close) else { break };
        let Ok(face) = rest[at + 6..close].parse::<u16>() else { break };
        parts.last_mut().expect("a part").1.push_str(&rest[..at]);
        parts.push((Some(face), String::new()));
        rest = &rest[close + 1..];
    }
    parts.last_mut().expect("a part").1.push_str(rest);
    parts
}

/// A line's words without its faces' changes.
fn without_faces(text: &str) -> String {
    face_parts(text).into_iter().map(|(_, words)| words).collect()
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
    /// waits for its key again. No two at once: one after the other. With
    /// set_textbox_transparency the box is see-through, lighter than a
    /// line said (it acts as the box opens; solid again after).
    fn bubble_line(&mut self, number: i32, word: Option<&str>, who: Who, frames: f32, text: &str, voice: Voice) {
        let face = self.bubble_faces.get(&number).copied();
        let message = if voice == Voice::Thought { "a thought in a bubble" } else { "a line in a bubble" };
        let see_through = self.patches.process(SEE_THROUGH);
        let instead = match see_through {
            Some(_) => "a see-through line of Sky's box that closes by itself",
            None => "a line of Sky's box that closes by itself",
        };
        self.note(number, Kind::Bubble, Fate::Changed, message, instead);
        // (Its words cut to the box, not to a bubble.)
        let text = ::film::screen::wrap(&text.replace('\n', " ")).join("\n");
        let frames = frames.round().max(30.0) as i32;
        if let Some(id) = see_through {
            self.line(&format!("ProcessSpecial({id}, 1, 0);"));
        }
        self.line(&format!("message_SetWaitMode({frames}, 0);"));
        let said = if voice == Voice::Thought { Some("think") } else { word.filter(|word| *word != "bubble" && *word != "shout") };
        self.say(number, said, Some(who), face, &text);
        self.line("message_Close();");
        self.line("message_SetWaitMode(-1, -1);");
        if let Some(id) = see_through {
            self.line(&format!("ProcessSpecial({id}, 0, 0);"));
        }
        self.calm();
        // (The bubble's time is the box's: the film's wait for it is left out.)
        self.said_over = true;
        self.talking = false;
    }

    /// A line said: its face (or the species' normal one, or its name
    /// alone), its box, as Sky's message operations show it.
    pub(super) fn say(&mut self, number: i32, word: Option<&str>, entity: Option<Who>, face: Option<u16>, text: &str) {
        self.opening = None;
        // (`{value}`: Sky's texts print no variable of a script: the number as last written before.)
        let mut text = text.to_string();
        for (name, value) in &self.values {
            text = text.replace(&format!("{{{name}}}"), &format!("{value}"));
        }
        let text = quoted(text.trim_end());
        // (`*words*`: bold with BoldText; else as written.)
        let text = if self.patches.has(BOLD) { bold(&text) } else { text };
        let text = if text.starts_with(' ') || text.starts_with('[') { text } else { format!(" {text}") };
        match entity {
            Some(who) if !self.stand_in(who) => self.said_with_face(number, word, who, face, &text),
            _ => self.said_plain(number, word, entity, &without_faces(&text)),
        }
    }

    /// A line said with no portrait: the narrator's, one joining, one
    /// drawn as the stand-in (its name said).
    fn said_plain(&mut self, number: i32, word: Option<&str>, entity: Option<Who>, text: &str) {
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
                self.line("message_EmptyActor();");
                self.line(&format!("message_Talk(\"{}\");", text.trim_start()));
            }
            // One drawn as the stand-in: no portrait, no name of the stand-in's; its own name said.
            (Some(who), _) => {
                if self.who(number, who).is_none() {
                    return;
                }
                let name = self.film_names.get(&who).map_or_else(|| "???".to_string(), |name| capitalised(name));
                self.line("message_EmptyActor();");
                self.line(&format!("message_Talk(\"{}:{text}\");", quoted(&name)));
            }
        }
        self.talking = true;
    }

    /// A line said with its speaker's portrait: its face (or the species'
    /// normal one, or its name alone). A face changing in it (`[FACE:n]`)
    /// changes in the box with ChangePortraitMidText; else each face's
    /// words are a box of their own, one after the other.
    fn said_with_face(&mut self, number: i32, word: Option<&str>, who: Who, face: Option<u16>, text: &str) {
        let Some(actor) = self.who(number, who) else { return };
        let kind = self.actors.get(&self.became.get(&who).copied().unwrap_or(who)).map_or(0, |one| one.kind);
        // (Transformed, dynamic_actor: Sky would name him and show him as the
        // other; his own name and face instead, as the film has them.)
        if self.mimicking.contains(&who) {
            let face = face.filter(|face| self.game.has_face(kind, *face)).unwrap_or(0);
            let name = self.film_names.get(&who).map_or_else(|| self.game.name(kind), |name| capitalised(name));
            self.line("message_EmptyActor();");
            if let Some(id) = self.patches.process(PORTRAIT_ALONE).filter(|_| self.game.has_face(kind, face)) {
                self.line(&format!("ProcessSpecial({id}, {kind}, {face});"));
            }
            let talk = if word == Some("think") { "message_Monologue" } else { "message_Talk" };
            self.line(&format!("{talk}(\"[CS:N]{}[CR]:{}\");", quoted(&name), without_faces(text)));
            self.talking = true;
            return;
        }
        // (A face the species lacks: its normal one, else none.)
        let shown = |writer: &Self, face: u16| {
            if writer.game.has_face(kind, face) {
                Some(face)
            } else if writer.game.has_face(kind, 0) {
                Some(0)
            } else {
                None
            }
        };
        let first = shown(self, face.unwrap_or(0));
        let talk = if word == Some("think") { "message_Monologue" } else { "message_Talk" };
        let place = if self.box_top { "FACE_POS_TOP_L_FACEINW" } else { "FACE_POS_STANDARD" };
        let set_face = |face: u16| {
            let face = FACES.get(usize::from(face)).map_or(face.to_string(), |name| format!("FACE_{name}"));
            format!("message_SetFace({actor}, {face}, {place});")
        };
        let parts = face_parts(text);
        match first {
            None => {
                self.line(&format!("message_SetActor({actor});"));
                self.line(&format!("{talk}(\"{}\");", without_faces(text)));
            }
            Some(face) if parts.len() == 1 || self.patches.has(FACE_MID_TEXT) => {
                self.line(&set_face(face));
                let mut said = String::new();
                for (changed, words) in parts {
                    if let Some(changed) = changed {
                        said.push_str(&format!("[FACE:{}]", shown(self, changed).unwrap_or(0)));
                    }
                    said.push_str(&words);
                }
                self.line(&format!("{talk}(\"{said}\");"));
            }
            Some(face) => {
                self.note(number, Kind::Speech, Fate::Changed, "a face changed in the middle of a line", "a box of its own for each face");
                for (at, (changed, words)) in parts.into_iter().enumerate() {
                    let mut words = words.as_str();
                    while let Some(rest) = words.strip_prefix("\\n").or_else(|| words.strip_prefix(' ')) {
                        words = rest;
                    }
                    while let Some(rest) = words.strip_suffix("\\n").or_else(|| words.strip_suffix(' ')) {
                        words = rest;
                    }
                    if words.is_empty() {
                        continue;
                    }
                    self.line(&set_face(changed.and_then(|changed| shown(self, changed)).unwrap_or(face)));
                    let lead = if at == 0 && words.starts_with('[') { "" } else { " " };
                    self.line(&format!("{talk}(\"{lead}{words}\");"));
                }
            }
        }
        self.talking = true;
    }
}

/// A film's name as a box shows a speaker's (`pichu_spiky`: “Pichu spiky”).
pub(super) fn capitalised(name: &str) -> String {
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

    #[test]
    fn stars_are_bold_and_faces_cut_a_line() {
        assert_eq!(bold(" The *Oran Berry*! A * alone"), " The [BS]Oran Berry[BR]! A * alone");
        assert_eq!(face_parts(" Won! [FACE:5]But..."), vec![(None, " Won! ".to_string()), (Some(5), "But...".to_string())]);
        assert_eq!(without_faces(" Won! [FACE:5]But..."), " Won! But...");
    }
}
