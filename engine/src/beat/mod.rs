//! What a scene does, one beat after another (`Beat`), as the project's
//! story files and this project's own scenes write them (`Beat::parse`),
//! and what the view follows (`Cam`).

use glam::Vec2;

use crate::language::speaker;
use crate::problem::ParseError;
use crate::vocabulary::{GameAsk, Vocabulary};
use crate::who::Who;

mod asking;
mod body;
mod cast;
mod data;
mod declare;
mod flow;
mod forms;
mod kinds;
mod motion;
mod names;
mod node;
mod num;
mod parse;
mod place;
mod play;
mod screen;
mod sound;
mod speech;

mod view;
mod written;

use crate::base::{AnimId, Direction, EmoteId, Frames, MoveId, SongId, SoundId, colour_named};
pub use asking::*;
pub use body::{BodyBeat, Play, SheetAnim, Stacked};
pub use cast::Role;
pub use data::DataBeat;
pub(crate) use data::TIMER;
pub use flow::{FlowBeat, LineEnd};
pub use forms::{Form, form_of, forms};
pub use kinds::*;
pub use motion::MotionBeat;
pub use names::*;
pub use node::{Key, KeyTime, NodeBeat, NodeRef, PathShape, Sort, TrackMode};
pub use num::{Num, Point};
pub use parse::{Arg, Miss, commands, is_command};
pub use place::{CrowdForm, CrowdShape, Exit, Formation, Gait, Pace, PlaceBeat, Spot};
pub use play::PlayBeat;
pub use screen::ScreenBeat;
pub use sound::{Played, Side, SoundBeat, SoundOf, pan_of};
pub use speech::SpeechBeat;
pub use view::ViewBeat;
#[cfg(test)]
mod tests;

/// One thing a scene does (a family, a file: its beats and how a line reads as one), and what makes the
/// story's course ([`Course`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Beat {
    Body(BodyBeat),
    Motion(MotionBeat),
    View(ViewBeat),
    Screen(ScreenBeat),
    Sound(SoundBeat),
    Speech(SpeechBeat),
    Flow(FlowBeat),
    Data(DataBeat),
    Play(PlayBeat),
    Place(PlaceBeat),
    /// A property of any node set, a node made (see `NodeBeat`).
    Node(NodeBeat),
    /// A line only the game can play (see `vocabulary`), at once.
    Host(Vec<String>),
    Course(Course),
}

impl From<BodyBeat> for Beat {
    fn from(beat: BodyBeat) -> Self {
        Beat::Body(beat)
    }
}

impl From<MotionBeat> for Beat {
    fn from(beat: MotionBeat) -> Self {
        Beat::Motion(beat)
    }
}

impl From<ViewBeat> for Beat {
    fn from(beat: ViewBeat) -> Self {
        Beat::View(beat)
    }
}

impl From<ScreenBeat> for Beat {
    fn from(beat: ScreenBeat) -> Self {
        Beat::Screen(beat)
    }
}

impl From<SoundBeat> for Beat {
    fn from(beat: SoundBeat) -> Self {
        Beat::Sound(beat)
    }
}

impl From<SpeechBeat> for Beat {
    fn from(beat: SpeechBeat) -> Self {
        Beat::Speech(beat)
    }
}

impl From<FlowBeat> for Beat {
    fn from(beat: FlowBeat) -> Self {
        Beat::Flow(beat)
    }
}

impl From<DataBeat> for Beat {
    fn from(beat: DataBeat) -> Self {
        Beat::Data(beat)
    }
}

impl From<PlaceBeat> for Beat {
    fn from(beat: PlaceBeat) -> Self {
        Beat::Place(beat)
    }
}

impl From<NodeBeat> for Beat {
    fn from(beat: NodeBeat) -> Self {
        Beat::Node(beat)
    }
}

impl From<PlayBeat> for Beat {
    fn from(beat: PlayBeat) -> Self {
        Beat::Play(beat)
    }
}

/// On or off, as written.
fn on_off(field: &str) -> Option<bool> {
    match field {
        "on" | "yes" | "1" => Some(true),
        "off" | "no" | "0" => Some(false),
        _ => None,
    }
}

/// A wait as written: seconds (`1.5s`), frames (`6f`, or `6`).
fn frames_of(field: &str) -> Option<u32> {
    match (field.strip_suffix('s'), field.strip_suffix('f')) {
        (Some(seconds), _) => Some((seconds.parse::<f32>().ok()? * 60.0).round().max(0.0) as u32),
        (_, Some(frames)) => frames.parse().ok(),
        _ => field.parse().ok(),
    }
}

/// Whether a line is to be read when it is played, the numbers it names
/// put in first: braces in a field other than its words (`put roc {x}
/// {y}`, `set note n{i}`, `halo @ {x} {y}`). Not the lines that shape the
/// scene's ways (ifs, loops, labels, questions, lines said: their words are
/// filled as they are said).
pub fn played_filled(line: &[String]) -> bool {
    !braced(line).is_empty()
}

/// The fields of a line with numbers named in braces, but for its words
/// (see `played_filled`).
fn braced(line: &[String]) -> Vec<&String> {
    let words = match line.first().map(String::as_str) {
        Some(
            "say" | "ask" | "answer" | "end" | "event" | "scene" | "actor" | "label" | "goto" | "return" | "if" | "elif" | "else" | "endif" | "while"
            | "endwhile" | "waituntil" | "break" | "continue" | "parallel" | "gotoscene",
        )
        | None => return Vec::new(),
        Some("bubble" | "think" | "shout" | "popup") => Some(3),
        Some("caption" | "typecaption" | "credits" | "settext") => Some(2),
        Some("banner" | "log") => Some(1),
        Some("battlelog") => Some(2),
        Some("hud") => Some(if line.get(1).is_some_and(|place| hud_place(place).is_some()) { 2 } else { 1 }),
        _ => None,
    };
    let named = |field: &String| field.contains('{') && field.contains('}') && !field.contains("{name:") && !field.contains("{item:");
    line.iter().enumerate().skip(1).filter(|(at, field)| Some(*at) != words && named(field)).map(|(_, field)| field).collect()
}

/// The names in braces of a field (`w:{x}:{y}`: `x`, `y`; `n{i}`: `i`).
fn names_in(field: &str) -> Vec<String> {
    field.split('{').skip(1).filter_map(|rest| rest.split_once('}')).map(|(name, _)| name.to_string()).collect()
}

impl Beat {
    /// A line whose numbers are found as it plays (see `played_filled`):
    /// the typed beat when each of them is a number it holds (`put roc {x}
    /// {y}`, `Num::Var`); else the line itself, read once filled
    /// (`FlowBeat::Filled`), if it reads then (with each number as 1). None
    /// for a line with no number to find, or one that would not read.
    pub fn played(line: Vec<String>, words: &'static Vocabulary) -> Option<Beat> {
        let braced = braced(&line);
        if braced.is_empty() {
            return None;
        }
        // (Each field a number by its name alone, or a spot's: `w:{x}:{y}`.)
        let mut names: Vec<String> = braced.iter().flat_map(|field| names_in(field)).collect();
        if let Ok(beat) = Beat::parse(&line, words) {
            let mut held = beat.vars();
            names.sort();
            held.sort();
            if held == names {
                return Some(beat);
            }
        }
        let sample: Vec<String> = line.iter().map(|field| braces_as_one(field)).collect();
        Beat::parse(&sample, words).ok()?;
        Some(Beat::Flow(FlowBeat::Filled(line)))
    }
}

/// What makes the story's course: what waits for the player or leaves
/// the scene (a line, a question, a menu, a dungeon, a place to walk in),
/// played by the `Director`.
#[derive(Clone, Debug, PartialEq)]
pub enum Course {
    /// A line said (none: a narration), with a portrait's face; the name
    /// shown instead of the speaker's own (`???`, a voice...); it goes by
    /// itself after so many frames if timed (none: it waits for its key).
    Say {
        entity: Option<Who>,
        face: Option<u16>,
        text: String,
        name: Option<String>,
        timed: Option<Frames>,
    },
    /// A question and its answers.
    Ask(Vec<String>),
    /// After a question: where the way on of each answer starts, and where
    /// the last one ends. Each goes to the end of the scene.
    Answer(u16),
    End,
    /// The last answer's end, as `End`, but each way goes on past it, in
    /// the same scene (`endask`): a question in a loop.
    Rejoin,
    /// A line only the game can play (see `vocabulary`), as the story goes
    /// on: once the line on screen is read.
    Host(Vec<String>),
    /// What a scene a game makes of a place asks that game (`game talk
    /// kecleon`), typed: handed over as it is (`Call::Game`), the story
    /// holding if it asks to ([`GameAsk::holds`]).
    Game(GameAsk),
}

impl Beat {
    /// Whether it is of a scene's record: it happens as the scene goes,
    /// without a key (else, of the story's course).
    pub fn staged(&self) -> bool {
        !matches!(self, Beat::Course(_))
    }
}

/// A field with what its braces name as 1 (`{x}`, `n{i}`: `1`, `n1`).
pub(crate) fn braces_as_one(field: &str) -> String {
    let mut field = field.to_string();
    while let Some(close) = field.find('}') {
        let Some(open) = field[..close].rfind('{') else {
            break;
        };
        field.replace_range(open..=close, "1");
    }
    field
}
