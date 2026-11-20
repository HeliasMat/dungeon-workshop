//! What the commands say for a program (`--json`, `server`): one JSON
//! object a line, each with its `command` and the version of its shape
//! (`shape`), as `film/FILMS.md` "For agents" tells them. The pieces
//! every command shares: a problem, a value of the registry, seconds.

use std::sync::{Arc, Mutex};

pub use serde_json::{Map, Value, json};

use engine::props::Value as PropValue;
use engine::{Diagnostic, codes};

/// The version of every shape: raised when one changes in a way that
/// breaks who reads it (a field gone or another meaning); a field added
/// leaves it.
pub const SHAPE: u32 = 1;

/// A command's answer: its name, its shape's version, then its fields.
pub fn answer(command: &str, fields: Value) -> Value {
    let mut made = Map::new();
    made.insert("command".to_string(), json!(command));
    made.insert("shape".to_string(), json!(SHAPE));
    if let Value::Object(fields) = fields {
        made.extend(fields);
    }
    Value::Object(made)
}

/// A file's path as told: from the project's root, with `/`.
pub fn path(root: &std::path::Path, path: &str) -> String {
    let shown = std::path::Path::new(path);
    shown.strip_prefix(root).unwrap_or(shown).display().to_string().replace('\\', "/")
}

/// Written on one line.
pub fn print(value: &Value) {
    println!("{value}");
}

/// Seconds of so many frames, to a hundredth.
pub fn seconds(frames: usize) -> f64 {
    (frames as f64 / 60.0 * 100.0).round() / 100.0
}

/// A number, rounded to a hundredth (what is shown is no finer).
pub fn number(value: f32) -> Value {
    match value.is_finite() {
        true => json!((f64::from(value) * 100.0).round() / 100.0),
        false => Value::Null,
    }
}

/// A point: `[x, y]`.
pub fn point(at: glam::Vec2) -> Value {
    json!([number(at.x), number(at.y)])
}

/// A value of the registry: a number, a boolean, `[x, y]`, a colour as
/// `#rrggbb` (`#rrggbbaa` when it does not wholly show), a word, null.
pub fn prop_value(value: &PropValue) -> Value {
    match value {
        PropValue::Number(number_of) => number(*number_of),
        PropValue::Bool(on) => json!(on),
        PropValue::Point(at) => point(*at),
        PropValue::Colour(_) | PropValue::Choice(_) => json!(value.to_string()),
        PropValue::Nothing => Value::Null,
    }
}

/// The line and column (from 1, in letters) of a byte of a text.
pub fn spot(text: &str, byte: usize) -> (usize, usize) {
    let position = language::Source::new(text).position(byte.min(text.len()));
    (position.line, position.column)
}

/// The words a problem offers instead of one not known, and that word.
fn offered(message: &str) -> (Option<String>, Vec<String>) {
    match ::film::check::offered(message) {
        Some((word, near)) => (Some(word), near),
        None => (None, Vec::new()),
    }
}

/// The first word of a message that its line has whole, past the line's
/// own first word (`bulbasaur is there…` on `walk bulbasaur to right`).
fn named_on(line_text: &str, message: &str) -> Option<String> {
    let piece = |letter: char| letter.is_alphanumeric() || letter == '_';
    let words: Vec<&str> = line_text.split(|letter: char| !piece(letter)).filter(|word| !word.is_empty()).collect();
    let past_first = words.get(1..).unwrap_or_default();
    message.split(|letter: char| !piece(letter)).filter(|word| word.chars().count() >= 2).find(|word| past_first.contains(word)).map(str::to_string)
}

/// What a message advises after its `→`, if it does.
fn advice(message: &str) -> Option<String> {
    message.split_once('→').map(|(_, after)| after.trim().to_string()).filter(|after| !after.is_empty())
}

/// A line put right: the word not known written as the nearest name, or
/// the new form of an old one (`write “…”`).
fn corrected(line_text: &str, message: &str, word: Option<&str>, near: &[String]) -> Option<String> {
    if let (Some(word), Some(near)) = (word, near.first())
        && line_text.contains(word)
    {
        let lead = &line_text[..line_text.len() - line_text.trim_start().len()];
        return Some(format!("{lead}{}", line_text.trim_start().replacen(word, near, 1)));
    }
    let (_, after) = message.split_once("write “")?;
    let (now, _) = after.split_once('”')?;
    let lead = &line_text[..line_text.len() - line_text.trim_start().len()];
    Some(format!("{lead}{now}"))
}

/// Who tells the problem: how it was read (`read`) or as it played
/// (`play`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Read,
    Played,
}

/// A problem, for a program: its stable code, its weight (`error`,
/// `remark`, `detail`), its line and column, its message, the nearest
/// names, the line put right, the advice; when it played, if it did.
pub struct Told<'a> {
    pub code: &'static str,
    pub weight: &'static str,
    pub line: usize,
    /// Its bytes in the text, if the problem says.
    pub span: Option<(usize, usize)>,
    pub message: &'a str,
    /// Those it names, typed.
    pub mentions: &'a [engine::Mention],
    pub frame: Option<usize>,
    pub scene: Option<usize>,
    pub source: Source,
}

impl Told<'_> {
    /// A problem read from the text.
    pub fn read(problem: &Diagnostic) -> Told<'_> {
        Told {
            code: ::film::codes::of(problem),
            weight: if problem.is_remark() { "remark" } else { "error" },
            line: problem.line(),
            span: problem.span.bytes.map(|bytes| (bytes.start, bytes.end)),
            message: &problem.message,
            mentions: &problem.mentions,
            frame: None,
            scene: None,
            source: Source::Read,
        }
    }

    /// What a rehearsal saw.
    pub fn seen(seen: &engine::Seen, wrong: bool, scene: usize) -> Told<'_> {
        Told {
            code: codes::seen(&seen.kind, wrong),
            weight: match (wrong, seen.small) {
                (true, _) => "error",
                (false, true) => "detail",
                (false, false) => "remark",
            },
            line: usize::try_from(seen.line).unwrap_or(0),
            span: None,
            message: &seen.text,
            mentions: &seen.mentions,
            frame: Some(seen.frame as usize),
            scene: Some(scene),
            source: Source::Played,
        }
    }

    /// Its column, and where its words end, on its line of `text`; those words.
    fn columns(&self, text: &str, message: &str) -> (usize, Option<usize>, Option<String>) {
        let line_text = text.lines().nth(self.line.wrapping_sub(1)).unwrap_or("");
        // (The word not known; else the first word of the message found on the line past its first.)
        let word = offered(message).0.or_else(|| named_on(line_text, message));
        match self.span {
            Some((start, end)) => (spot(text, start).1, Some(spot(text, end).1), text.get(start..end).map(|words| words.trim().to_string())),
            None => match word.as_ref().and_then(|word| line_text.find(word.as_str()).map(|byte| (byte, word.len()))) {
                Some((byte, long)) => (line_text[..byte].chars().count() + 1, Some(line_text[..byte + long].chars().count() + 1), word.clone()),
                None => (line_text.chars().take_while(|letter| letter.is_whitespace()).count() + 1, None, None),
            },
        }
    }

    /// As a line of text, where it is first, as a compiler tells it
    /// (`library/films/x.film:12:5: …`, the words it is about after
    /// when its message does not quote them): `path` the film's, `text` its text.
    pub fn located(&self, root: &std::path::Path, path: &str, text: &str, says: &dyn Fn(&str, &[engine::Mention]) -> String) -> String {
        let message = says(self.message, self.mentions);
        if self.line == 0 || text.is_empty() {
            let at = if self.line == 0 { String::new() } else { format!(":{}", self.line) };
            return format!("{}{at}: {message}", self::path(root, path));
        }
        let (column, _, words) = self.columns(text, &message);
        let words = words.filter(|words| !words.is_empty() && words.chars().count() <= 40 && !message.contains(words.as_str()));
        let words = words.map_or(String::new(), |words| format!(" (“{words}”)"));
        let second = self.frame.map_or(String::new(), |frame| format!(" ({} s)", seconds(frame)));
        format!("{}:{}:{column}{second}: {message}{words}", self::path(root, path), self.line)
    }

    /// As JSON, `text` the film's (for the columns and the line put right).
    pub fn json(&self, text: &str, says: &dyn Fn(&str, &[engine::Mention]) -> String) -> Value {
        let message = says(self.message, self.mentions);
        let line_text = text.lines().nth(self.line.wrapping_sub(1)).unwrap_or("");
        let (word, near) = offered(&message);
        let (column, end, words) = self.columns(text, &message);
        json!({
            "word": words,
            "code": self.code,
            "severity": self.weight,
            "source": if self.source == Source::Read { "read" } else { "play" },
            "line": self.line,
            "column": column,
            "end_column": end,
            "message": message,
            "closest": near,
            "correction": corrected(line_text, &message, word.as_deref(), &near).filter(|_| self.line > 0),
            "advice": advice(&message),
            "second": self.frame.map(seconds),
            "scene": self.scene,
        })
    }
}

/// What a film plays to be heard, as it goes, for `state`: a stand-in for
/// the speakers that keeps each song and sound asked, with its frame.
#[derive(Clone, Default)]
pub struct Heard {
    pub frame: usize,
    /// On: what is asked is kept (off once a pass has heard it all, so a
    /// moment played again is not heard twice).
    pub keeping: bool,
    pub events: Vec<(usize, Event)>,
}

/// Something asked to be heard.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Song { channel: usize, song: Option<String>, level: f32 },
    Volume { channel: usize, level: f32 },
    Sound { name: String, volume: f32, pan: f32, looped: bool, named: Option<String> },
    Fade { name: String, level: f32, stop: bool },
    Hush,
}

/// A song of the game by its word, else its number.
fn song_name(song: u16) -> String {
    rules::songs::SONGS.iter().find(|known| known.number == song).map_or_else(|| song.to_string(), |known| known.word.to_string())
}

fn noise_name(id: u16) -> String {
    rules::songs::NOISES.iter().find(|known| known.number == id).map_or_else(|| id.to_string(), |known| known.word.to_string())
}

/// The speakers' stand-in, shared with who reads it.
pub struct Listener(pub Arc<Mutex<Heard>>);

impl Listener {
    fn keep(&self, event: Event) {
        if let Ok(mut heard) = self.0.lock()
            && heard.keeping
        {
            let frame = heard.frame;
            heard.events.push((frame, event));
        }
    }
}

impl shell::sound::Hears for Listener {
    fn song(&mut self, channel: usize, song: Option<u16>, level: f32, _step: f32) {
        self.keep(Event::Song { channel, song: song.map(song_name), level });
    }
    fn song_from(&mut self, channel: usize, song: Option<u16>, level: f32, _seconds: f32) {
        self.keep(Event::Song { channel, song: song.map(song_name), level });
    }
    fn own_song(&mut self, channel: usize, name: &str, level: f32, _step: f32, _seconds: f32) {
        self.keep(Event::Song { channel, song: Some(name.to_string()), level });
    }
    fn volume(&mut self, channel: usize, level: f32) {
        self.keep(Event::Volume { channel, level });
    }
    fn sound(&self, id: u16, volume: f32, _pitch: f32) {
        self.keep(Event::Sound { name: noise_name(id), volume, pan: 0.0, looped: false, named: None });
    }
    fn play(&mut self, voice: shell::sound::Voice) {
        let name = match &voice.source {
            shell::sound::Source::Sound(id) => noise_name(*id),
            shell::sound::Source::Song(song) => song_name(*song),
            shell::sound::Source::Own(name) => name.clone(),
        };
        self.keep(Event::Sound { name, volume: voice.volume, pan: voice.pan, looped: voice.looped, named: voice.name.clone() });
    }
    fn fade_voice(&mut self, name: &str, level: f32, _frames: f32, stop: bool) {
        self.keep(Event::Fade { name: name.to_string(), level, stop });
    }
    fn hush(&mut self) {
        self.keep(Event::Hush);
    }
    fn jingle(&self, id: u16) {
        self.keep(Event::Sound { name: format!("jingle {id}"), volume: 1.0, pan: 0.0, looped: false, named: None });
    }
    fn fade(&mut self, channel: usize, _frames: f32) {
        self.keep(Event::Song { channel, song: None, level: 0.0 });
    }
    fn speed(&mut self, _speed: f32) {}
    fn tick(&mut self) {
        if let Ok(mut heard) = self.0.lock() {
            heard.frame += 1;
        }
    }
}

/// How long a sound heard once is said to be playing.
const RECENT: usize = 30;

impl Heard {
    /// What is heard at a frame: the song of each channel, the sounds kept
    /// (looped or named), those just begun.
    pub fn at(&self, frame: usize) -> Value {
        let mut songs: [(Option<String>, f32); 2] = [(None, 1.0), (None, 1.0)];
        let mut lasting: Vec<(String, Value)> = Vec::new();
        let mut recent: Vec<Value> = Vec::new();
        for (when, event) in self.events.iter().take_while(|(when, _)| *when <= frame) {
            match event {
                Event::Song { channel, song, level } => songs[(*channel).min(1)] = (song.clone(), *level),
                Event::Volume { channel, level } => songs[(*channel).min(1)].1 = *level,
                Event::Sound { name, volume, pan, looped, named } if *looped || named.is_some() => {
                    let key = named.clone().unwrap_or_else(|| name.clone());
                    lasting.retain(|(kept, _)| *kept != key);
                    lasting.push((key, json!({"sound": name, "name": named, "volume": number(*volume), "pan": number(*pan), "loop": looped})));
                }
                Event::Sound { name, volume, pan, .. } if frame - when <= RECENT => {
                    recent.push(json!({"sound": name, "volume": number(*volume), "pan": number(*pan), "since_s": seconds(frame - when)}))
                }
                Event::Fade { name, stop: true, .. } => lasting.retain(|(kept, _)| kept != name),
                Event::Fade { name, level, .. } => {
                    if let Some((_, kept)) = lasting.iter_mut().find(|(kept, _)| kept == name) {
                        kept["volume"] = number(*level);
                    }
                }
                Event::Hush => lasting.clear(),
                Event::Sound { .. } => {}
            }
        }
        let song = |(song, level): &(Option<String>, f32)| song.as_ref().map(|song| json!({"music": song, "volume": number(*level)}));
        json!({
            "music": song(&songs[0]),
            "ambient": song(&songs[1]),
            "sounds": lasting.into_iter().map(|(_, kept)| kept).collect::<Vec<Value>>(),
            "recent": recent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_are_escaped_as_json_has_them() {
        let made = answer("trial", json!({"text": "a \"b\" \\ c\nd\té\u{1}"}));
        let line = made.to_string();
        assert!(line.contains(r#""text":"a \"b\" \\ c\nd\té\u0001""#), "{line}");
        assert!(line.starts_with(r#"{"command":"trial","shape":1"#), "{line}");
        assert_eq!(serde_json::from_str::<Value>(&line).expect("read back"), made);
        assert_eq!(number(f32::NAN), Value::Null);
        assert_eq!(number(1.23456), json!(1.23));
    }

    #[test]
    fn a_registry_value_is_plain_json() {
        assert_eq!(prop_value(&PropValue::Point(glam::Vec2::new(1.0, 2.5))), json!([1.0, 2.5]));
        assert_eq!(prop_value(&PropValue::Colour([255, 136, 0, 255])), json!("#ff8800"));
        assert_eq!(prop_value(&PropValue::Colour([255, 136, 0, 128])), json!("#ff880080"));
        assert_eq!(prop_value(&PropValue::Nothing), Value::Null);
        assert_eq!(prop_value(&PropValue::Bool(true)), json!(true));
    }

    #[test]
    fn a_problem_tells_its_column_its_nearest_names_and_its_line_put_right() {
        let text = "scene: beach pikachu\n  jump pikachuu\n";
        let problem = Diagnostic::at(2, "no one is called “pikachuu”. Near: pikachu".to_string());
        let told = Told::read(&problem).json(text, &|text, _| text.to_string());
        assert_eq!(told["code"], json!(::film::codes::READ_WRONG));
        assert_eq!((told["line"].clone(), told["column"].clone(), told["end_column"].clone()), (json!(2), json!(8), json!(16)));
        assert_eq!(told["closest"], json!(["pikachu"]));
        assert_eq!(told["correction"], json!("  jump pikachu"));
        let old = Diagnostic::at(2, "“size” is an old form: write “prop a scale 2”".to_string()).as_remark();
        let told = Told::read(&old).json("x\n  size a 2\n", &|text, _| text.to_string());
        assert_eq!((told["severity"].clone(), told["correction"].clone()), (json!("remark"), json!("  prop a scale 2")));
    }

    #[test]
    fn what_is_heard_at_a_moment() {
        let heard = Heard {
            frame: 0,
            keeping: false,
            events: vec![
                (1, Event::Song { channel: 0, song: Some("calm".to_string()), level: 1.0 }),
                (2, Event::Sound { name: "wind".to_string(), volume: 0.5, pan: 0.0, looped: true, named: Some("wind".to_string()) }),
                (100, Event::Sound { name: "click".to_string(), volume: 1.0, pan: 0.0, looped: false, named: None }),
                (200, Event::Fade { name: "wind".to_string(), level: 0.0, stop: true }),
            ],
        };
        let at = heard.at(110);
        assert_eq!(at["music"]["music"], json!("calm"));
        assert_eq!((at["sounds"][0]["name"].clone(), at["recent"][0]["sound"].clone()), (json!("wind"), json!("click")));
        let later = heard.at(250);
        assert_eq!((later["sounds"].clone(), later["recent"].clone()), (json!([]), json!([])));
    }
}
