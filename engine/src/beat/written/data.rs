//! How a `DataBeat` is written as its line (read by `beat::data`).

use crate::beat::{DataBeat, Value};
use crate::vocabulary::Vocabulary;

/// A value as a field, if it is read back the same (a name that reads as
/// a number or someone cannot be).
pub(super) fn value(value: &Value) -> Option<String> {
    let field = match value {
        Value::Number(number) => number.to_string(),
        Value::Var(name) => name.clone(),
    };
    (Value::parse(&field) == *value).then_some(field)
}

/// Frames as a field, if a whole number of them (`12f`).
pub(super) fn frames(frames: f32) -> Option<String> {
    (frames >= 0.0 && frames.fract() == 0.0 && frames <= u32::MAX as f32).then(|| format!("{}f", frames as u32))
}

impl DataBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let line = |words: &[&str]| words.iter().map(|word| word.to_string()).collect::<Vec<String>>();
        Some(match self {
            DataBeat::Random { var, from, to } if from <= to => line(&["random", var, &from.to_string(), &to.to_string()]),
            // (Read with the smaller first, always.)
            DataBeat::Random { .. } => return None,
            DataBeat::Calc { var, a, op, b } => line(&["calc", var, &value(a)?, op, &value(b)?]),
            DataBeat::SetText { var, text } => line(&["settext", var, text]),
            DataBeat::Timer { var, frames: time } => {
                let time = match time {
                    None => "off".to_string(),
                    Some(time) => frames(time.0)?,
                };
                // (`timer` alone, or `timer_<name>` by its name.)
                match var.strip_prefix(crate::beat::TIMER) {
                    Some("") => line(&["timer", &time]),
                    Some(named) => line(&["timer", named.strip_prefix('_')?, &time]),
                    None => return None,
                }
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Frames;
    use crate::beat::{Beat, DataBeat, Value};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    #[test]
    fn every_data_beat_written_is_read_back_the_same() {
        let var = || "score".to_string();
        for beat in [
            DataBeat::Set { var: var(), value: Value::Number(3.5) },
            DataBeat::Add { var: var(), value: Value::Var("bonus".into()) },
            DataBeat::Random { var: var(), from: 2, to: 9 },
            DataBeat::Calc { var: var(), a: Value::Var("a".into()), op: "max".into(), b: Value::Number(-2.0) },
            DataBeat::Measure { var: var(), a: Who::Monster(25), b: Who::Character(0) },
            DataBeat::Position { who: Who::Monster(7), x: "x".into(), y: "y".into() },
            DataBeat::Sees { var: var(), who: Who::Monster(7), other: Who::Monster(25), distance: 80.0, angle: 45.0 },
            DataBeat::SetText { var: var(), text: "hello you".into() },
            DataBeat::Timer { var: crate::beat::TIMER.to_string(), frames: Some(Frames(1800.0)) },
            DataBeat::Timer { var: format!("{}_bomb", crate::beat::TIMER), frames: None },
        ] {
            let fields = Beat::Data(beat.clone()).fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&fields, &SAMPLE).ok(), Some(Beat::Data(beat)), "{fields:?}");
        }
    }
}
