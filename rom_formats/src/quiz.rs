//! The personality test's data, from the overlay that runs it (13):
//! questions, answers and the points they give each of the 16
//! personalities, the hero each personality makes by gender, and the
//! possible partners.
//!
//! The scored questions come first; the last two are "boy or girl?" and
//! "did you play the earlier games?".
//!
//! Table addresses and layouts are pmdsky-debug's (`overlay13.yml`,
//! `struct personality_test`).

use super::nds::{NdsRom, RomError};
use super::text::Strings;
use crate::sym;
use rules::quiz::{Answer, PERSONALITIES, Question, QuizData};

const OVERLAY: u32 = 13;
const QUESTIONS: usize = 66;
/// Answers with points; those after them belong to the two questions
/// with an effect of their own.
const SCORED_ANSWERS: usize = 171;
const PARTNERS: usize = 21;

/// Addresses of a table in the order EU, NA, JP (0: unknown).
struct Table([u32; 3]);

const PLAY_OLD_GAME_BOOST: Table = Table(sym::PLAY_OLD_GAME_BOOST.address);
const STARTERS_PARTNER_IDS: Table = Table(sym::STARTERS_PARTNER_IDS.address);
const STARTERS_HERO_IDS: Table = Table(sym::STARTERS_HERO_IDS.address);
const STARTERS_STRINGS: Table = Table(sym::STARTERS_STRINGS.address);
const QUIZ_QUESTION_STRINGS: Table = Table(sym::QUIZ_QUESTION_STRINGS.address);
const QUIZ_ANSWER_STRINGS: Table = Table(sym::QUIZ_ANSWER_STRINGS.address);
const QUIZ_ANSWER_POINTS: Table = Table(sym::QUIZ_ANSWER_POINTS.address);
const QUIZ_ANSWER_ASSOCIATIONS: Table = Table(sym::QUIZ_ANSWER_ASSOCIATIONS.address);

pub fn load(rom: &NdsRom, strings: &Strings) -> Result<QuizData, RomError> {
    let region = match rom.game_code().as_bytes().get(3) {
        Some(b'P') => 0,
        Some(b'E') => 1,
        Some(b'J') => 2,
        _ => return Err(RomError::Truncated),
    };
    let (base, binary) = rom.overlay(OVERLAY).ok_or(RomError::Truncated)?;
    let bytes = |table: &Table, length: usize| -> Result<&[u8], RomError> {
        let start = table.0[region].checked_sub(base).filter(|_| table.0[region] != 0);
        let start = start.ok_or(RomError::Truncated)? as usize;
        binary.get(start..start + length).ok_or(RomError::Truncated)
    };
    let words = |table: &Table, count: usize| -> Result<Vec<u16>, RomError> {
        Ok(bytes(table, count * 2)?.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect())
    };
    // The game numbers its strings from 1.
    let text = |id: u16| {
        let found = (id as usize).checked_sub(1).and_then(|index| strings.get(index));
        found.unwrap_or_default().to_string()
    };

    let question_texts = words(&QUIZ_QUESTION_STRINGS, QUESTIONS)?;
    let answer_texts = words(&QUIZ_ANSWER_STRINGS, SCORED_ANSWERS + 4)?;
    let points = bytes(&QUIZ_ANSWER_POINTS, SCORED_ANSWERS * PERSONALITIES)?;
    // Per question: how many answers, and the first.
    let answers_of = bytes(&QUIZ_ANSWER_ASSOCIATIONS, QUESTIONS * 2)?;
    let mut questions = Vec::with_capacity(QUESTIONS);
    for (question, entry) in question_texts.iter().zip(answers_of.as_chunks::<2>().0.iter()) {
        let (count, first) = (entry[0] as usize, entry[1] as usize);
        let answer = |index: usize| -> Result<Answer, RomError> {
            let scored = points.get(index * PERSONALITIES..(index + 1) * PERSONALITIES);
            Ok(Answer { text: text(*answer_texts.get(index).ok_or(RomError::Truncated)?), points: scored.and_then(|scored| scored.try_into().ok()) })
        };
        questions.push(Question { text: text(*question), answers: (first..first + count).map(answer).collect::<Result<_, _>>()? });
    }

    let heroes = words(&STARTERS_HERO_IDS, PERSONALITIES * 2)?;
    let explained = words(&STARTERS_STRINGS, PERSONALITIES * 3)?;
    let boost = bytes(&PLAY_OLD_GAME_BOOST, 8)?;
    Ok(QuizData {
        questions,
        heroes: heroes.as_chunks::<2>().0.iter().map(|pair| [pair[0], pair[1]]).collect(),
        partners: words(&STARTERS_PARTNER_IDS, PARTNERS)?,
        explanations: explained.as_chunks::<3>().0.iter().map(|set| [text(set[0]), text(set[1]), text(set[2])]).collect(),
        old_game_boost: [boost[..4].to_vec(), boost[4..].to_vec()],
        // (Strings 1708 to 1732, counted from 0: the game's "Personality
        // Quiz Aura" block and the partner's, up to its confirmation.)
        aura: (1708..=1732).map(|at| text(at + 1)).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_quiz_loads() {
        let rom = crate::nds::test_rom();
        let strings = Strings::parse(rom.file("MESSAGE/text_e.str").unwrap()).unwrap();
        let quiz = load(&rom, &strings).unwrap();
        for index in [0, 1, 63, 64, 65] {
            let question = &quiz.questions[index];
            let answers: Vec<_> = question.answers.iter().map(|a| (&a.text, a.points.is_some())).collect();
            println!("QUIZ {index}: {:?} {answers:?}", question.text);
        }
        println!("QUIZ heroes {:?}", quiz.heroes);
        println!("QUIZ partners {:?}", quiz.partners);
        println!("QUIZ boost {:?}", quiz.old_game_boost);
        println!("QUIZ explained {:?}", quiz.explanations[0]);
        // Every question has text and two to four answers; only the last
        // two questions have answers without points.
        assert_eq!(quiz.questions.len(), QUESTIONS);
        for (index, question) in quiz.questions.iter().enumerate() {
            assert!(!question.text.is_empty() && (2..=4).contains(&question.answers.len()), "{index}");
            let scored = question.answers.iter().all(|answer| answer.points.is_some());
            assert_eq!(scored, index < 64, "{index}");
        }
        assert!(quiz.heroes.iter().flatten().all(|&hero| hero > 0 && hero < 1200));
        assert!(quiz.partners.iter().all(|&partner| partner > 0 && partner < 1200));
        // The two questions with an effect of their own: the gender (boy
        // first), then the earlier games (yes first).
        let (gender, played) = (&quiz.questions[64], &quiz.questions[65]);
        assert!(gender.text.contains("garçon") && gender.answers[0].text.contains("garçon"));
        assert!(played.text.contains("Explorateurs") && played.answers[0].text == "Oui.");
    }
}
