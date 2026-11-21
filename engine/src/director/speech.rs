//! Lines said, and the words in them.

use super::*;

/// Words between stars (`*Pomme*`) shown in colour, as the game writes
/// what matters; a lone star is left as it is.
pub fn emphasis(text: &str) -> String {
    let parts: Vec<&str> = text.split('*').collect();
    // (Stars by pairs: with an odd count, the last one stays a star.)
    let pairs = (parts.len() - 1) / 2;
    let mut shown = String::new();
    for (at, part) in parts.iter().enumerate() {
        match at {
            at if at % 2 == 1 && at < pairs * 2 => shown.push_str(&format!("[CS:K]{part}[CR]")),
            at if at > pairs * 2 => shown.push_str(&format!("*{part}")),
            _ => shown.push_str(part),
        }
    }
    shown
}

/// Words as they are compared (see `Director::holds`): in small letters,
/// without accents, without the spaces around.
pub(super) fn plain(words: &str) -> String {
    let plain = |c: char| match c {
        'à' | 'â' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' => 'i',
        'ô' | 'ö' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    };
    words.trim().to_lowercase().chars().map(plain).collect()
}

/// A text with the names it asks for put in: of those it names
/// (`{name:m25}`) and of items (`{item:109}`, `{item:i109}`), as whoever
/// shows it knows them.
pub fn names_in(text: &str, words: &Vocabulary, who: impl Fn(Who) -> String, item: impl Fn(u16) -> String) -> String {
    /// The text with each `{word...}` replaced by what is made of `...`.
    fn each(mut text: String, word: &str, name: impl Fn(&str) -> String) -> String {
        while let Some(at) = text.find(word) {
            let Some(length) = text[at..].find('}') else {
                break;
            };
            let named = name(&text[at + word.len()..at + length]);
            text.replace_range(at..at + length + 1, &named);
        }
        text
    }
    let text = each(text.to_string(), "{name:", |name| Who::parse(name).map_or_else(String::new, &who));
    each(text, words.item_name.0, |name| name.trim_start_matches('i').split('.').next().and_then(|item| item.parse().ok()).map_or_else(String::new, &item))
}

impl Director {
    /// What a line does by itself on the stage, as asked (see
    /// `SpeechBeat::AutoFace`): its speaker and the one before face each other;
    /// the view glides to it when it is out of sight (`keepspeaker`).
    pub(super) fn stage_line(&mut self, stage: &mut Stage, who: Who, face: Option<u16>, focus: Vec2) {
        if self.manner.auto_face
            && let Some(other) = self.manner.last_speaker.filter(|other| *other != who && stage.cast.contains_key(other))
        {
            if !self.manner.held_gaze.contains(&who) && stage.painted_ways(who).is_none() {
                stage.direct(Beat::Body(BodyBeat::Face { who, other }), focus);
            }
            if !self.manner.held_gaze.contains(&other) && stage.painted_ways(other).is_none() {
                stage.direct(Beat::Body(BodyBeat::Face { who: other, other: who }), focus);
            }
        }
        self.listen(stage, who, face, focus);
        // (What the view shows: the screen, less its edges and the box of the
        // line at its foot; a head may stand up to the top.)
        let close = stage.view.zoom.now.max(0.1);
        let half = crate::stage::SCREEN / 2.0 / close;
        let off = stage.cast.get(&who).map(|actor| actor.position - focus);
        let in_screen = off.is_some_and(|off| off.x.abs() <= half.x - 10.0 && off.y >= -half.y + 30.0 && off.y <= half.y - 8.0 / close);
        let under_the_box = off.is_some_and(|off| off.y > half.y - (crate::stage::BOX_HIGH + 4.0) / close);
        // (Low on the screen but on it: the box goes up for this line rather
        // than the view moving, as the game does; it comes back down after.
        // A box the film put somewhere stays where it is.)
        let raise = in_screen && under_the_box && !self.manner.box_kept;
        if raise != self.manner.box_raised && (raise || self.manner.box_raised) {
            stage.words.box_top = raise;
            self.manner.box_raised = raise;
        }
        let unseen = off.is_some() && (!in_screen || (under_the_box && !raise));
        if self.manner.keep_speaker && unseen {
            stage.direct(Beat::View(ViewBeat::PanStays { to: Cam::Who(who), frames: Frames(30.0) }), focus);
        }
        self.manner.last_speaker = Some(who);
    }

    /// Those listening look at who speaks; with `SpeechBeat::React`, one in two
    /// answers its face, not the same ones from one line to the next.
    pub(super) fn listen(&mut self, stage: &mut Stage, who: Who, face: Option<u16>, focus: Vec2) {
        self.manner.lines_heard += 1;
        let listening: Vec<Who> = self.manner.audience.iter().copied().filter(|one| *one != who && stage.cast.contains_key(one)).collect();
        let held = self.manner.held_gaze.clone();
        for (place, one) in listening.into_iter().enumerate() {
            if !held.contains(&one) && stage.painted_ways(one).is_none() {
                stage.direct(Beat::Body(BodyBeat::Face { who: one, other: who }), focus);
            }
            if !self.manner.react || (place + self.manner.lines_heard) % 2 == 1 {
                continue;
            }
            if let Some(emote) = face.and_then(|face| self.words.emote_of_face(face)) {
                stage.effect(one, emote, Some(40.0));
            }
            if face.is_some_and(|face| self.words.leaping_faces.contains(&face)) {
                stage.direct(Beat::Body(BodyBeat::Jump { who: one, height: 6.0, frames: Frames(6.0) }), focus);
            }
        }
    }

    /// A line said: on screen, its speaker's face when it gives none, its
    /// voice, what it does on the stage by itself.
    pub(super) fn say(
        &mut self,
        entity: Option<Who>,
        face: Option<u16>,
        text: &str,
        name: Option<String>,
        stage: &mut Stage,
        focus: Vec2,
        calls: &mut Vec<Call>,
    ) {
        // (`target`: whoever Enter found last.)
        let entity = entity.map(|who| if who == Who::Target { self.target().unwrap_or(who) } else { who });
        self.reading = true;
        let face = face.or_else(|| self.manner.faces.get(&entity?).copied());
        let voice = entity.and_then(|who| self.manner.voices.get(&who).copied());
        if let Some(who) = entity.filter(|who| stage.cast.contains_key(who)) {
            self.stage_line(stage, who, face, focus);
        }
        let right = entity.is_some_and(|who| self.manner.right.contains(&who));
        // (Shown as another: its name and its face.)
        let entity = entity.map(|who| stage.cast.get(&who).and_then(|actor| actor.looks.looks_like).unwrap_or(who));
        calls.push(Call::Say { entity, face, text: emphasis(&self.fill(text)), name, voice, right, speed: self.manner.text_speed });
    }

    /// A text with the numbers it names put in (`{name}`), and the words
    /// kept (see `DataBeat::SetText`); names made of numbers (`{n{i}}`: the
    /// inner first).
    pub fn fill(&self, text: &str) -> String {
        let mut text = self.fill_once(text);
        for _ in 0..3 {
            let again = self.fill_once(&text);
            if again == text {
                break;
            }
            text = again;
        }
        text
    }

    pub(super) fn fill_once(&self, text: &str) -> String {
        let mut text = text.to_string();
        // (`{name:target}`: the name of whoever Enter found last.)
        if let Some(target) = self.target().filter(|_| text.contains("{name:target}")) {
            text = text.replace("{name:target}", &format!("{{name:{target}}}"));
        }
        for (var, value) in &self.vars {
            let mark = format!("{{{var}}}");
            if text.contains(&mark) {
                text = text.replace(&mark, &numbers::number_text(*value));
            }
        }
        for (var, words) in &self.texts {
            let mark = format!("{{{var}}}");
            if text.contains(&mark) {
                text = text.replace(&mark, words);
            }
        }
        text
    }

    /// A beat's words with the numbers they name put in.
    pub(super) fn filled(&self, beat: Beat) -> Beat {
        match beat {
            Beat::Speech(SpeechBeat::Bubble { who, frames, text, voice }) => Beat::Speech(SpeechBeat::Bubble { who, frames, text: self.fill(&text), voice }),
            Beat::Body(BodyBeat::Popup { who, text, colour }) => Beat::Body(BodyBeat::Popup { who, text: self.fill(&text), colour }),
            Beat::Flow(FlowBeat::Log(text)) => Beat::Flow(FlowBeat::Log(self.fill(&text))),
            Beat::Screen(ScreenBeat::Caption { frames, text, typed }) => Beat::Screen(ScreenBeat::Caption { frames, text: self.fill(&text), typed }),
            Beat::Screen(ScreenBeat::Banner(text)) => Beat::Screen(ScreenBeat::Banner(self.fill(&text))),
            Beat::Screen(ScreenBeat::BattleLog { frames, text }) => Beat::Screen(ScreenBeat::BattleLog { frames, text: self.fill(&text) }),
            beat => beat,
        }
    }
}
