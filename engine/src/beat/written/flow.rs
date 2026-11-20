//! How a `FlowBeat` is written as its line (read by `beat::flow`).

use crate::Frames;
use crate::beat::asking::{Compare, Test, Value};
use crate::beat::{Beat, FlowBeat};
use crate::vocabulary::Vocabulary;

impl FlowBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let named = |verb: &str, name: &str| vec![verb.to_string(), name.to_string()];
        Some(match self {
            FlowBeat::Wait(frames) => vec!["t".to_string(), frames.to_string()],
            FlowBeat::WaitBetween { from, to } => vec!["t".to_string(), format!("{from}~{to}")],
            FlowBeat::WaitFound(frames) => vec!["t".to_string(), frames.to_string()],
            FlowBeat::WaitTrack(node) => named("waittrack", &node.to_string()),
            FlowBeat::WaitSound { of, pitch: 0 } => named("waitsound", &of.to_string()),
            FlowBeat::WaitSound { of, pitch } => vec!["waitsound".to_string(), of.to_string(), "pitch".to_string(), pitch.to_string()],
            FlowBeat::WaitUntil(test) => asked("waituntil", test)?,
            FlowBeat::If(test) => asked("if", test)?,
            FlowBeat::ElseIf(test) => asked("elif", test)?,
            FlowBeat::While(test) => asked("while", test)?,
            // (A label called `off` would be read as none.)
            FlowBeat::Parallel(Some(label)) if label != "off" => named("parallel", label),
            FlowBeat::Parallel(None) => named("parallel", "off"),
            FlowBeat::After { frames, more, beat } => later("after", frames.0, more.0, beat, words)?,
            // (Every is read at least a frame apart.)
            FlowBeat::Every { frames, more, beat } if *frames >= Frames(1.0) => later("every", frames.0, more.0, beat, words)?,
            FlowBeat::EveryOff => named("every", "off"),
            FlowBeat::Log(text) => named("log", text),
            FlowBeat::Line { number, end } => {
                let mut line = vec!["filmline".to_string(), number.to_string()];
                match end {
                    crate::beat::LineEnd::Next => {}
                    crate::beat::LineEnd::Again => line.push("again".to_string()),
                    crate::beat::LineEnd::Who(who) => line.extend(["until".to_string(), who.to_string()]),
                    crate::beat::LineEnd::Frames(frames) => line.extend(["for".to_string(), frames.to_string()]),
                    crate::beat::LineEnd::Track(node) => line.extend(["track".to_string(), node.to_string()]),
                }
                line
            }
            // (A line filled as it plays: that line, read once filled.)
            FlowBeat::Filled(line) => line.clone(),
            // (A label called `off`, every less than a frame apart: no line reads them so.)
            _ => return None,
        })
    }
}

/// A beat later, or again and again: so many frames (or drawn between two),
/// then the beat's own line (none: whole frames only, as they are read).
fn later(verb: &str, frames: f32, more: f32, beat: &Beat, words: &Vocabulary) -> Option<Vec<String>> {
    let whole = |n: f32| (n >= 0.0 && n.fract() == 0.0).then_some(n as u32);
    let (from, more) = (whole(frames)?, whole(more)?);
    let time = if more == 0 { format!("{from}") } else { format!("{from}~{}", from + more) };
    Some([verb.to_string(), time].into_iter().chain(beat.fields(words)?).collect())
}

/// A verb and its test, as `test` reads it: each condition in full, joined
/// by `and`, those joined by `or` (none: a name `Value::parse` reads otherwise).
fn asked(verb: &str, test: &Test) -> Option<Vec<String>> {
    let mut line = vec![verb.to_string()];
    for (at, all) in test.any.iter().enumerate() {
        if all.is_empty() {
            return None;
        }
        if at > 0 {
            line.push("or".to_string());
        }
        for (at, condition) in all.iter().enumerate() {
            if at > 0 {
                line.push("and".to_string());
            }
            let value = match &condition.value {
                Value::Number(number) => number.to_string(),
                Value::Var(name) => name.clone(),
            };
            if Value::parse(&value) != condition.value || ["and", "or"].contains(&condition.var.as_str()) {
                return None;
            }
            let compare = match condition.compare {
                Compare::Equal => "==",
                Compare::Unequal => "!=",
                Compare::Less => "<",
                Compare::LessOrEqual => "<=",
                Compare::Greater => ">",
                Compare::GreaterOrEqual => ">=",
            };
            line.extend([condition.var.clone(), compare.to_string(), value]);
        }
    }
    (!test.any.is_empty()).then_some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SoundId;
    use crate::SoundOf;
    use crate::beat::SoundBeat;
    use crate::beat::asking::Condition;
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;
    use glam::Vec2;

    #[test]
    fn every_flow_beat_written_is_read_back_the_same() {
        let test = Test {
            any: vec![
                vec![
                    Condition { var: "a".into(), compare: Compare::Equal, value: Value::Number(1.0) },
                    Condition { var: "b".into(), compare: Compare::GreaterOrEqual, value: Value::Var("c".into()) },
                ],
                vec![Condition { var: "d".into(), compare: Compare::Unequal, value: Value::Number(0.0) }],
            ],
        };
        let sound = Box::new(Beat::Sound(SoundBeat::Sound(SoundId(8964))));
        for beat in [
            FlowBeat::Wait(Frames(90.0)),
            FlowBeat::WaitBetween { from: Frames(60.0), to: Frames(180.0) },
            FlowBeat::WaitFor(Who::Monster(25)),
            FlowBeat::WaitMoves(Who::Character(0)),
            FlowBeat::WaitTrack(crate::beat::NodeRef::Named("robot.arm".to_string())),
            FlowBeat::WaitTrack(crate::beat::NodeRef::Who(Who::Monster(25))),
            FlowBeat::WaitAll,
            FlowBeat::WaitSound { of: SoundOf::Game(SoundId(0x3F02)), pitch: 0 },
            FlowBeat::WaitSound { of: SoundOf::Own("clack".to_string()), pitch: -2 },
            FlowBeat::WaitSound { of: SoundOf::Move(crate::MoveId(33)), pitch: 0 },
            FlowBeat::WaitCam,
            FlowBeat::WaitFade,
            FlowBeat::WaitBubbles,
            FlowBeat::WaitUntil(test.clone()),
            FlowBeat::WaitNear { who: Who::Monster(7), at: Vec2::new(12.5, -3.0).into(), radius: 20.0 },
            FlowBeat::If(test.clone()),
            FlowBeat::ElseIf(test.clone()),
            FlowBeat::Else,
            FlowBeat::EndIf,
            FlowBeat::While(test),
            FlowBeat::EndWhile,
            FlowBeat::Break,
            FlowBeat::Continue,
            FlowBeat::Label("start".into()),
            FlowBeat::Goto("start".into()),
            FlowBeat::Return,
            FlowBeat::GotoScene("next".into()),
            FlowBeat::Line { number: 12, end: crate::beat::LineEnd::Next },
            FlowBeat::Line { number: 12, end: crate::beat::LineEnd::Again },
            FlowBeat::Line { number: 3, end: crate::beat::LineEnd::Who(Who::Monster(25)) },
            FlowBeat::Line { number: 3, end: crate::beat::LineEnd::Frames(Frames(90.0)) },
            FlowBeat::Line { number: 3, end: crate::beat::LineEnd::Track(crate::beat::NodeRef::Named("moon".to_string())) },
            FlowBeat::Parallel(Some("start".into())),
            FlowBeat::Parallel(None),
            FlowBeat::After { frames: Frames(30.0), more: Frames(0.0), beat: sound.clone() },
            FlowBeat::After { frames: Frames(30.0), more: Frames(60.0), beat: sound.clone() },
            FlowBeat::Every { frames: Frames(10.0), more: Frames(5.0), beat: sound },
            FlowBeat::EveryOff,
            FlowBeat::Checkpoint("middle".into()),
            FlowBeat::Log("hello there".into()),
        ] {
            let fields = Beat::Flow(beat.clone()).fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&fields, &SAMPLE).ok(), Some(Beat::Flow(beat)), "{fields:?}");
        }
    }
}
