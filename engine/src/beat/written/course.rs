//! How a `Course` is written as its line (read by `beat::parse`'s `course`).

use crate::beat::Course;
use crate::vocabulary::Vocabulary;
use crate::who::Who;

impl Course {
    pub(super) fn fields(&self, _words: &Vocabulary) -> Option<Vec<String>> {
        let word = |word: &str| word.to_string();
        Some(match self {
            Course::Say { entity, face, text, name, timed } => {
                // (A spot never speaks; a `\n` written as is, or a `:` in the
                // speaker, would not come back the same.)
                if matches!(entity, Some(Who::Spot(_))) || text.contains("\\n") {
                    return None;
                }
                let code = entity.map_or(word("-"), |who| who.to_string());
                if code.contains(':') {
                    return None;
                }
                let speaker = match name {
                    Some(name) => format!("{code}:{name}"),
                    None => code,
                };
                let mut line = vec![word("say"), speaker, face.map_or(word("-"), |face| face.to_string()), text.replace('\n', "\\n")];
                line.extend(timed.map(|frames| frames.to_string()));
                line
            }
            // (Answers are kept in one field, between `|`: none, or one with a `|`, cannot be.)
            Course::Ask(answers) if answers.is_empty() || answers.iter().any(|answer| answer.contains('|')) => return None,
            Course::Ask(answers) => vec![word("ask"), answers.join("|")],
            Course::Answer(at) => vec![word("answer"), at.to_string()],
            Course::End => vec![word("end")],
            Course::Rejoin => vec![word("endask")],
            Course::Host(line) => line.clone(),
            Course::Game(ask) => std::iter::once(word("game")).chain(ask.words()?).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::beat::{Beat, Course};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    #[test]
    fn every_course_written_is_read_back_the_same() {
        let courses = vec![
            Course::Say { entity: Some(Who::Monster(25)), face: Some(10), text: "Hello, you!\nHow are you?".into(), name: None, timed: None },
            Course::Say { entity: Some(Who::Monster(25)), face: None, text: "...".into(), name: Some("???".into()), timed: Some(crate::Frames(45.0)) },
            Course::Say { entity: None, face: None, text: "The sea is calm.".into(), name: None, timed: None },
            Course::Ask(vec!["Yes".into(), "No thanks".into()]),
            Course::Answer(2),
            Course::End,
            Course::Rejoin,
            Course::Host(vec!["dungeon".into(), "3".into()]),
            Course::Game(crate::beat::GameAsk::Talk("kecleon".into())),
            Course::Game(crate::beat::GameAsk::Menu),
        ];
        for b in courses {
            let line = Beat::Course(b.clone()).fields(&SAMPLE).expect("written");
            assert_eq!(Beat::parse(&line, &SAMPLE).ok(), Some(Beat::Course(b)), "{line:?}");
        }
    }
}
