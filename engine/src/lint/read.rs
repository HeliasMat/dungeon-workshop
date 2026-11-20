//! A scene file's lines read once, as the checker goes through them: each
//! the beat it is, or what shapes the story (`event`, `scene`), or why it
//! does not read.

use super::*;

/// A line as the checker reads it: its number, its fields (for what is
/// told of it, and the rules on what is written itself), what it is.
pub(super) struct Read {
    pub number: usize,
    pub fields: Vec<String>,
    pub is: Is,
}

/// What a line is.
pub(super) enum Is {
    /// An event opens (`event <number> <name> …`).
    Event,
    /// A scene opens (`scene <level> <name>`), by its name if it has one.
    Scene(Option<String>),
    /// What the story reads itself, and no beat: who stands in a scene of
    /// the game's (`actor`), the numbers another scene gives (`uses`).
    Story,
    /// The beat it is.
    Beat(Beat),
    /// A line filled as it plays: its typed beat, its numbers by their
    /// names (`put roc {x} {y}`); one whose names are built as it plays
    /// (`set note n{i}`), read with its numbers as 1.
    Filled(Result<Beat, ParseError>),
    /// Why it does not read.
    Wrong(ParseError),
}

/// Each line read.
pub(super) fn read(lines: &[(usize, Vec<String>)], words: &'static Vocabulary) -> Vec<Read> {
    let one = |fields: &Vec<String>| match fields[0].as_str() {
        "event" => Is::Event,
        "scene" => Is::Scene(fields.get(2).cloned()),
        "actor" | "uses" => Is::Story,
        _ if played_filled(fields) => match Beat::played(fields.clone(), words) {
            Some(Beat::Flow(FlowBeat::Filled(_))) | None => {
                let sample: Vec<String> = fields.iter().map(|field| braces_as_one(field)).collect();
                Is::Filled(Beat::parse(&sample, words))
            }
            Some(beat) => Is::Filled(Ok(beat)),
        },
        _ => match Beat::parse(fields, words) {
            Ok(beat) => Is::Beat(beat),
            Err(error) => Is::Wrong(error),
        },
    };
    lines.iter().map(|(number, fields)| Read { number: *number, fields: fields.clone(), is: one(fields) }).collect()
}

/// What a line does to the ways through a scene.
pub(super) enum Flow<'a> {
    Event,
    Scene,
    If,
    ElseIf,
    Else,
    EndIf,
    While,
    EndWhile,
    /// `break`, `continue`.
    OutOfWhile,
    Label(&'a str),
    /// To a label of the scene (`goto`, `gosub`, `parallel`, `skipto`).
    GoesTo(&'a str),
    GotoScene(&'a str),
    Ask,
    Answer,
    /// The answers' end (`end`, `endask`).
    EndAsk,
}

impl Read {
    /// What it does to the ways through its scene, by its beat. A line that
    /// does not read is taken for what its word would do: a block it opens
    /// is not told as left open as well.
    pub fn flow(&self) -> Option<Flow<'_>> {
        Some(match &self.is {
            Is::Event => Flow::Event,
            Is::Scene(_) => Flow::Scene,
            Is::Beat(Beat::Flow(flow)) => match flow {
                FlowBeat::If(_) => Flow::If,
                FlowBeat::ElseIf(_) => Flow::ElseIf,
                FlowBeat::Else => Flow::Else,
                FlowBeat::EndIf => Flow::EndIf,
                FlowBeat::While(_) => Flow::While,
                FlowBeat::EndWhile => Flow::EndWhile,
                FlowBeat::Break | FlowBeat::Continue => Flow::OutOfWhile,
                FlowBeat::Label(label) => Flow::Label(label),
                FlowBeat::Goto(label) | FlowBeat::Parallel(Some(label)) if label != "off" => Flow::GoesTo(label),
                FlowBeat::GotoScene(name) => Flow::GotoScene(name),
                _ => return None,
            },
            Is::Beat(Beat::Course(course)) => match course {
                Course::Ask(_) => Flow::Ask,
                Course::Answer(_) => Flow::Answer,
                Course::End | Course::Rejoin => Flow::EndAsk,
                _ => return None,
            },
            Is::Wrong(_) => {
                let label = self.fields.get(1).map_or("", String::as_str);
                match self.fields[0].as_str() {
                    "if" => Flow::If,
                    "elif" => Flow::ElseIf,
                    "while" => Flow::While,
                    "label" => Flow::Label(label),
                    "goto" | "parallel" if !label.is_empty() && label != "off" => Flow::GoesTo(label),
                    "gotoscene" => Flow::GotoScene(label),
                    "answer" => Flow::Answer,
                    _ => return None,
                }
            }
            _ => return None,
        })
    }
}
