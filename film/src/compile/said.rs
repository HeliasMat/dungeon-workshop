//! A line said: `who (face, fast): words`, timed by itself, acted first as
//! the game's scripts act it; `???: words` and `umbreon?: words` for one the
//! team does not know yet.

use engine::{LETTERS_A_FRAME, SHORTEST_LINE, Voice, line_frames};

use crate::emit;
use crate::film::Film;
use crate::screen::{BOX_LINES, BREAK, WIDE, coloured, in_a_bubble, wrap};
use crate::text::told;
use crate::words::{ACTING, FACES, of, plain};

/// How much faster than usual a line says it is written out and read.
const SPEEDS: [(&str, f32); 2] = [("fast", 1.6), ("slow", 0.6)];
/// The word of a line said over another.
const OVER: &str = "top";
/// The word of a line said in a bubble, waited for as a line: the box free
/// for another skit beside.
const BUBBLE: &str = "bubble";

/// What someone does before saying a line, as the game's scripts have it: it
/// takes on a new face first, not each time it speaks.
fn act(film: &mut Film, who: &str, face: &str, text: &str) {
    match of(&ACTING, face).filter(|_| film.speaking.faces.get(who).map(String::as_str) != Some(face)) {
        Some((hop, emote, sound, frames)) => {
            if let Some((height, length)) = hop {
                film.jump(who, height as f32, length as f32);
            }
            if let Some(emote) = emote {
                film.emote(who, emote, Some(50.0));
            }
            if let Some(sound) = sound {
                film.sound(sound);
            }
            if face == "shouting" {
                film.beat(emit::shake(0, 2.0, Some(14.0)));
            }
            film.wait(i64::from(frames));
        }
        None if text.ends_with("?!") || text.ends_with("!?") => {
            film.jump(who, 6.0, 6.0);
            film.wait(10);
        }
        None => {}
    }
    // (`No...`: as often as not, a shake of the head.)
    let no = text.strip_prefix("No").is_some_and(|rest| !rest.starts_with(|letter: char| letter.is_alphanumeric() || letter == '_'));
    if no {
        for way in [engine::Direction::Right, engine::Direction::Left, engine::Direction::Right] {
            film.on(who, |who| emit::turn(who, way));
            film.wait(6);
        }
        if let Some(last) = film.speaking.last_speaker.clone().filter(|last| last != who) {
            film.face(who, &last);
        }
    }
}

/// How a line is said, by its brackets.
struct Manner {
    face: String,
    speed: f32,
    /// (Over the line on screen, in a bubble.)
    over: bool,
    /// (In a bubble, waited for.)
    bubble: bool,
}

/// What the brackets of a line say: its face, how fast, over another line.
fn manner_said(film: &mut Film, words: Option<&str>, number: usize) -> Manner {
    let mut manner = Manner { face: "normal".to_string(), speed: 1.0, over: false, bubble: false };
    for word in words.unwrap_or("").split(',').map(plain) {
        if let Some(fast) = of(&SPEEDS, &word) {
            manner.speed = fast;
        } else if word == OVER {
            manner.over = true;
        } else if word == BUBBLE {
            manner.bubble = true;
        } else if let Some(found) = of(&FACES, &word) {
            manner.face = found.to_string();
        } else if !word.is_empty() {
            let known: Vec<&str> = FACES.iter().map(|(name, _)| *name).chain(SPEEDS.iter().map(|(name, _)| *name)).chain([OVER, BUBBLE]).collect();
            film.unknown(number, "unknown face", &word, known.iter().copied(), &known.join(", "));
        }
    }
    manner
}

/// A line's pauses (`[1s]`) as the game writes them, and their frames; then
/// the words alone.
fn paused(text: &str, speed: f32) -> (String, i32, String) {
    let (mut out, mut bare, mut frames, mut rest) = (String::new(), String::new(), 0.0, text);
    while let Some(open) = rest.find('[') {
        let mark = rest[open + 1..].find(']').map(|close| &rest[open + 1..open + 1 + close]);
        let seconds = mark.and_then(|mark| mark.strip_suffix('s')).and_then(|seconds| seconds.parse::<f32>().ok());
        let Some(seconds) = seconds else {
            out.push_str(&rest[..=open]);
            bare.push_str(&rest[..=open]);
            rest = &rest[open + 1..];
            continue;
        };
        out.push_str(&rest[..open]);
        bare.push_str(&rest[..open]);
        out.push_str(&format!("[W:{}]", (seconds * 60.0 * LETTERS_A_FRAME * speed).round() as i32));
        frames += seconds * 60.0;
        rest = &rest[open + mark.map_or(0, str::len) + 2..];
    }
    out.push_str(rest);
    bare.push_str(rest);
    (out, frames as i32, bare)
}

/// `???: words`, `umbreon?: words`: whether who says it is one not known yet
/// (`???`, or someone written with a `?`).
pub(super) fn unknown_speaker(film: &mut Film, who: &str) -> bool {
    let bare = who.trim_end_matches('?');
    let hidden = bare.len() < who.len() && bare.chars().all(|letter| letter.is_alphanumeric() || letter == '_' || letter == '-');
    hidden && (bare.is_empty() || film.actor(bare).is_some())
}

impl Film {
    /// One the team does not know yet speaks: no portrait, no name.
    pub(super) fn said_unknown(&mut self, name: &str, text: &str) {
        let name = plain(name);
        let text = text.trim();
        let said = coloured(self, &wrap(text).join(BREAK));
        self.line_said(&format!("{}:???", if name.is_empty() { "-" } else { &name }), "-", &format!(" {said}"), line_frames("???", text, false) as i32);
    }

    /// A line said, its brackets read.
    pub(crate) fn said(&mut self, name: &str, brackets: Option<&str>, text: &str, number: usize) {
        let Manner { face, speed, over, bubble } = manner_said(self, brackets, number);
        let told = told(name);
        let text = text.trim();
        let (written, rests, bare) = paused(text, speed);
        // (Over the line on screen: a bubble, the box being taken.)
        if over && !told {
            self.bubble(&plain(name), &in_a_bubble(&bare), if face == "shouting" { Voice::Shouted } else { Voice::Said });
            return;
        }
        // (In a bubble, as long as it is read, and waited for: no box.)
        if bubble && !told {
            let who = plain(name);
            let who = self.staging.evolved.get(&who).cloned().unwrap_or(who);
            if self.acting {
                act(self, &who, &face, &bare);
            }
            self.speaking.faces.insert(who.clone(), face.clone());
            self.speaking.last_speaker = Some(who.clone());
            let frames = self.bubble(&who, &in_a_bubble(&bare), if face == "shouting" { Voice::Shouted } else { Voice::Said });
            self.wait((frames / speed).round() as i64 + rests as i64);
            return;
        }
        let lines = wrap(&written);
        if lines.len() > BOX_LINES {
            self.wrong(number, format!("line too long for the box ({BOX_LINES} lines of {WIDE} letters at most): cut it in two"));
        }
        let who = if told { "-".to_string() } else { plain(name) };
        let who = self.staging.evolved.get(&who).cloned().unwrap_or(who);
        let face = if told { face } else { face_had(self, &who, face, number) };
        if self.acting && !told {
            act(self, &who, &face, &bare);
        }
        self.speaking.faces.insert(who.clone(), face.clone());
        if !told {
            self.speaking.last_speaker = Some(who.clone());
        }
        if speed != 1.0 {
            self.beat(emit::text_speed(Some(speed)));
        }
        let shown_name = if told { "" } else { who.as_str() };
        let frames =
            self.timing.line_time.unwrap_or(((line_frames(shown_name, &bare, face == "shouting") as f32 / speed).floor().max(SHORTEST_LINE)) as i32 + rests);
        let said = coloured(self, &lines.join(BREAK));
        self.line_said(&who, if told { "-" } else { &face }, &format!(" {said}"), frames);
        if speed != 1.0 {
            self.beat(emit::text_speed(None));
        }
    }
}

/// For a face a species' portraits may lack, the nearest ones, in turn
/// (none had: its normal face).
const NEAREST_FACES: [(&str, &[&str]); 15] = [
    ("happy", &["joyous", "inspired"]),
    ("joyous", &["happy", "inspired"]),
    ("inspired", &["happy", "joyous", "determined"]),
    ("crying", &["teary_eyed", "sad"]),
    ("teary_eyed", &["crying", "sad"]),
    ("sad", &["teary_eyed", "worried"]),
    ("shouting", &["angry", "determined"]),
    ("angry", &["shouting", "determined"]),
    ("determined", &["angry", "inspired"]),
    ("surprised", &["stunned", "worried"]),
    ("stunned", &["surprised", "worried"]),
    ("dizzy", &["pain", "worried"]),
    ("pain", &["worried", "sad"]),
    ("worried", &["sad", "sigh"]),
    ("sigh", &["worried", "sad"]),
];

/// The face someone shows for that one: itself if his portraits have it
/// (or they are not known), else the nearest they have, else his normal
/// one; told when it changes.
fn face_had(film: &mut Film, who: &str, face: String, number: usize) -> String {
    let faces = film.actor(who).and_then(|code| film.tables.portraits.get(crate::names::species(&code)?).cloned());
    let Some(faces) = faces.filter(|faces| !faces.is_empty()) else {
        return face;
    };
    let has = |name: &str| scenes::vocabulary::POKEMON.face(name).is_some_and(|number| faces.contains(&number));
    if face == "normal" || has(&face) {
        return face;
    }
    let nearest = NEAREST_FACES.iter().find(|(said, _)| *said == face).map_or(&[][..], |(_, nearest)| *nearest);
    let instead = nearest.iter().find(|name| has(name)).map_or("normal", |name| *name);
    film.remark(number, format!("{who} has no {face} portrait: {instead} instead"));
    instead.to_string()
}

/// What a line said keeps in the scene at hand, nothing at the next one.
#[derive(Default)]
pub struct Speaking {
    pub faces: std::collections::HashMap<String, String>,
    pub last_speaker: Option<String>,
}
