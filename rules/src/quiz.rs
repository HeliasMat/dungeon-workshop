//! The personality test that opens the story: questions whose answers
//! give points to 16 personalities; the personality with the most points
//! and the gender chosen make the hero; the partner is then picked among
//! the starters that do not share a type with it.
//!
//! The rules are the original's as pmdsky-debug documents them
//! (`GetPersonality`, `struct personality_test`, `PLAY_OLD_GAME_BOOST`,
//! `STARTERS_TYPE_INCOMPATIBILITY_TABLE`). Like the town's services, it is
//! shown with the script engine's message and choice windows.
// The aura is read once the hero is known, as the original's does: its
// colour is the personality's (which colour goes with which is a guess:
// the game's sixteen in its own order). Its touch screen part is a key
// here.
// Where the two questions with an effect of their own come among the
// others is a guess (last, the earlier games first); their effect does not
// depend on it.

use crate::services::{Input, View};

pub const PERSONALITIES: usize = 16;
/// The `message_Menu` that runs the test.
pub const MENU: u16 = 48;
/// Questions asked: those drawn, then the two with an effect of their own
/// (the game keeps ten question numbers).
const DRAWN: usize = 8;
/// Points the earlier games' players get for some personalities.
const OLD_GAME_BOOST: u32 = 4;
/// Marks the end of a list of personalities.
const NO_PERSONALITY: u8 = 16;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Answer {
    pub text: String,
    /// Points for each personality; `None` for the answers of the two
    /// questions with an effect of their own.
    pub points: Option<[u8; PERSONALITIES]>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Question {
    pub text: String,
    pub answers: Vec<Answer>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct QuizData {
    /// The scored questions, then "boy or girl?" and "did you play the
    /// earlier games?".
    pub questions: Vec<Question>,
    /// The hero of each personality: male, female (monster numbers).
    pub heroes: Vec<[u16; 2]>,
    pub partners: Vec<u16>,
    /// For each personality: what it is like, then the line naming the
    /// hero, male and female.
    pub explanations: Vec<[String; 3]>,
    /// Personalities boosted for players of the earlier games, by gender.
    pub old_game_boost: [Vec<u8>; 2],
    /// The aura's texts: five that read it, its sixteen colours, then
    /// those of the partner (the choice asked for and confirmed).
    pub aura: Vec<String>,
}

/// The aura's texts before its colours, and its colours.
const AURA_INTRO: usize = 5;
pub const AURA_COLOURS: usize = 16;
/// The aura's text that has the partner confirmed.
const CONFIRM_TEXT: usize = AURA_INTRO + AURA_COLOURS + 3;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    /// The n-th question asked.
    Question(usize),
    /// A page of what the personality is like.
    Explain(usize),
    Hero,
    /// A page of the aura being read.
    Aura(usize),
    Partner,
    /// That partner, to be confirmed.
    Confirm(usize),
}

/// The test being taken.
#[derive(Clone, Debug)]
pub struct Quiz {
    /// Numbers of the questions asked, in order.
    asked: Vec<usize>,
    step: Step,
    cursor: usize,
    points: [u32; PERSONALITIES],
    played_old_game: bool,
    /// 0: male, 1: female.
    gender: usize,
    personality: usize,
    /// What the personality is like, page by page; then the aura's.
    pages: Vec<String>,
    aura: Vec<String>,
    partners: Vec<u16>,
}

impl Quiz {
    /// `random(n)` draws a number below `n`.
    pub fn new(data: &QuizData, mut random: impl FnMut(usize) -> usize) -> Self {
        // Distinct scored questions, then the two others.
        let scored = data.questions.len().saturating_sub(2);
        let mut pool: Vec<usize> = (0..scored).collect();
        let mut asked: Vec<usize> = (0..DRAWN.min(scored)).map(|_| pool.swap_remove(random(pool.len()))).collect();
        asked.extend((scored..data.questions.len()).rev());
        Self {
            asked,
            step: Step::Question(0),
            cursor: 0,
            points: [0; PERSONALITIES],
            played_old_game: false,
            gender: 0,
            personality: 0,
            pages: Vec::new(),
            aura: Vec::new(),
            partners: Vec::new(),
        }
    }

    /// While the aura is shown: its colour (0 to 15), and whether it is
    /// known yet (the last page).
    pub fn aura(&self) -> Option<(usize, bool)> {
        match self.step {
            Step::Aura(page) => Some((self.personality % AURA_COLOURS, page + 1 >= self.aura.len())),
            _ => None,
        }
    }

    /// The partner on show while one is chosen, for its portrait.
    pub fn partner_shown(&self) -> Option<u16> {
        match self.step {
            Step::Partner => self.partners.get(self.cursor).copied(),
            Step::Confirm(partner) => self.partners.get(partner).copied(),
            _ => None,
        }
    }

    /// The personality with the most points; those level get up to 19
    /// points more each, at random, and the first of the best wins.
    fn winner(&self, data: &QuizData, mut random: impl FnMut(usize) -> usize) -> usize {
        // Those who played the earlier games get a boost for some
        // personalities, by gender.
        let mut totals = self.points;
        let boosted = data.old_game_boost[self.gender].iter();
        for &personality in boosted.take_while(|&&personality| personality < NO_PERSONALITY) {
            if self.played_old_game {
                totals[personality as usize] += OLD_GAME_BOOST;
            }
        }
        let most = totals.iter().copied().max().unwrap_or(0);
        let mut best = (0, 0);
        for (personality, &points) in totals.iter().enumerate() {
            let total = points + if points == most { random(20) as u32 + 1 } else { 0 };
            if total > best.1 {
                best = (personality, total);
            }
        }
        best.0
    }

    /// Takes a key. `types(monster)` gives a monster's two types (0 for
    /// none). Once the partner is chosen: `(hero, partner)`.
    pub fn press(&mut self, input: Input, data: &QuizData, types: &dyn Fn(u16) -> (u8, u8), mut random: impl FnMut(usize) -> usize) -> Option<(u16, u16)> {
        let hero = |quiz: &Self| data.heroes.get(quiz.personality).map_or(0, |pair| pair[quiz.gender]);
        let choices = match self.step {
            Step::Question(at) => data.questions[self.asked[at]].answers.len(),
            Step::Partner => self.partners.len(),
            Step::Confirm(_) => 2,
            _ => 0,
        };
        match (input, &self.step) {
            (Input::Down, _) if choices > 0 => self.cursor = (self.cursor + 1) % choices,
            (Input::Up, _) if choices > 0 => self.cursor = (self.cursor + choices - 1) % choices,
            (Input::Confirm, &Step::Question(at)) => {
                let number = self.asked[at];
                let special = number + 2 >= data.questions.len();
                match (data.questions[number].answers[self.cursor].points, special) {
                    (Some(points), _) => {
                        for (total, points) in self.points.iter_mut().zip(points) {
                            *total += points as u32;
                        }
                    }
                    // "Boy or girl?": the boy comes first.
                    (None, _) if number + 2 == data.questions.len() => self.gender = self.cursor.min(1),
                    // "Did you play the earlier games?": yes comes first.
                    (None, _) => self.played_old_game = self.cursor == 0,
                }
                self.cursor = 0;
                self.step = Step::Question(at + 1);
                if at + 1 == self.asked.len() {
                    self.personality = self.winner(data, &mut random);
                    let explained = data.explanations.get(self.personality);
                    let text = explained.map_or("", |set| set[0].as_str());
                    // The game's text breaks its pages with `[P]`.
                    let pages = text.replace("[P]", "[C]");
                    let pages = pages.split("[C]").map(|page| page.trim_matches('\n').to_string());
                    self.pages = pages.filter(|page| !page.is_empty()).collect();
                    self.step = Step::Explain(0);
                }
            }
            (Input::Confirm, &Step::Explain(page)) => {
                self.step = match page + 1 < self.pages.len() {
                    true => Step::Explain(page + 1),
                    false => Step::Hero,
                };
            }
            (Input::Confirm, Step::Hero) if !data.aura.is_empty() && self.aura.is_empty() => {
                // The aura's pages, the game's own: what reads it, then its
                // colour.
                let colour = data.aura.get(AURA_INTRO + self.personality % AURA_COLOURS).cloned().unwrap_or_default();
                let mut text = data.aura[..AURA_INTRO.min(data.aura.len())].join("[C]");
                let asked = data.aura.get(AURA_INTRO + AURA_COLOURS).cloned().unwrap_or_default();
                text = format!("{}{}[C]{asked}", text.trim_end_matches("[C]"), colour.replace("[R]", " "));
                let cleaned = strip_waits(&text.replace("[P]", "[C]"));
                self.aura = cleaned.split("[C]").map(|page| page.trim_matches('\n').to_string()).filter(|page| !page.is_empty()).collect();
                self.step = Step::Aura(0);
            }
            (Input::Confirm, &Step::Aura(page)) if page + 1 < self.aura.len() => self.step = Step::Aura(page + 1),
            (Input::Confirm, Step::Hero | Step::Aura(_)) => {
                // The starters that share no type with the hero.
                let (first, second) = types(hero(self));
                let shares = |partner: &u16| {
                    let (a, b) = types(*partner);
                    [a, b].iter().any(|&kind| kind != 0 && (kind == first || kind == second))
                };
                self.partners = data.partners.iter().copied().filter(|partner| !shares(partner)).collect();
                (self.step, self.cursor) = (Step::Partner, 0);
            }
            (Input::Confirm, Step::Partner) if self.cursor < self.partners.len() => (self.step, self.cursor) = (Step::Confirm(self.cursor), 0),
            (Input::Confirm, &Step::Confirm(partner)) if self.cursor == 0 => {
                return self.partners.get(partner).map(|&partner| (hero(self), partner));
            }
            (Input::Confirm | Input::Cancel, &Step::Confirm(partner)) => (self.step, self.cursor) = (Step::Partner, partner),
            _ => {}
        }
        None
    }

    /// What to show; `name` names a monster.
    pub fn view(&self, data: &QuizData, name: impl Fn(u16) -> String) -> View {
        let (message, options) = match &self.step {
            Step::Question(at) => {
                let question = &data.questions[self.asked[*at]];
                (question.text.clone(), question.answers.iter().map(|answer| answer.text.clone()).collect())
            }
            Step::Explain(page) => (self.pages.get(*page).cloned().unwrap_or_default(), Vec::new()),
            Step::Hero => {
                let explained = data.explanations.get(self.personality);
                (explained.map_or(String::new(), |set| set[1 + self.gender].clone()), Vec::new())
            }
            Step::Aura(page) => (self.aura.get(*page).cloned().unwrap_or_default(), Vec::new()),
            Step::Partner => (
                match data.aura.len() > AURA_INTRO + AURA_COLOURS + 1 {
                    true => strip_waits(&data.aura[AURA_INTRO + AURA_COLOURS + 1]),
                    false => "Choose the Pokémon who will be your partner.".to_string(),
                },
                self.partners.iter().map(|&partner| name(partner)).collect(),
            ),
            Step::Confirm(partner) => (
                match data.aura.get(CONFIRM_TEXT) {
                    Some(text) => strip_waits(text),
                    None => format!("{}: is this the Pokémon?", self.partners.get(*partner).map_or(String::new(), |&partner| name(partner))),
                },
                vec!["Yes".to_string(), "No".to_string()],
            ),
        };
        View { message, options, cursor: self.cursor }
    }
}

/// A text without its timed waits (`[W:n]`), which a key replaces here.
fn strip_waits(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("[W:") {
        out.push_str(&rest[..at]);
        rest = rest[at..].find(']').map_or("", |end| &rest[at + end + 1..]);
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two scored questions (each answer for one personality), then the two
    /// others; personality 0 makes monster 10 or 11, personality 1 makes 20
    /// or 21.
    fn data() -> QuizData {
        let answer = |text: &str, personality: Option<usize>| Answer {
            text: text.to_string(),
            points: personality.map(|personality| {
                let mut points = [0; PERSONALITIES];
                points[personality] = 3;
                points
            }),
        };
        let question = |text: &str, answers: Vec<Answer>| Question { text: text.to_string(), answers };
        let mut heroes = vec![[1, 2]; PERSONALITIES];
        (heroes[0], heroes[1]) = ([10, 11], [20, 21]);
        QuizData {
            questions: vec![
                question("first", vec![answer("zero", Some(0)), answer("one", Some(1))]),
                question("second", vec![answer("zero", Some(0)), answer("one", Some(1))]),
                question("boy or girl?", vec![answer("boy", None), answer("girl", None)]),
                question("played?", vec![answer("yes", None), answer("no", None)]),
            ],
            heroes,
            partners: vec![30, 31, 32],
            explanations: vec![["what it is like[C]more".to_string(), "a boy".to_string(), "a girl".to_string()]; PERSONALITIES],
            old_game_boost: [vec![1, 16, 16, 16], vec![16; 4]],
            aura: Vec::new(),
        }
    }

    /// Monsters 20 and 31 are of type 5, the others of the type of their
    /// number.
    fn types(monster: u16) -> (u8, u8) {
        match monster {
            20 | 31 => (5, 0),
            other => (other as u8, 0),
        }
    }

    fn take(quiz: &mut Quiz, data: &QuizData, inputs: &[Input]) -> Option<(u16, u16)> {
        inputs.iter().find_map(|&input| quiz.press(input, data, &types, |_| 0))
    }

    #[test]
    fn answers_make_the_hero_and_the_partners_it_can_have() {
        use Input::*;
        let data = data();
        let mut quiz = Quiz::new(&data, |_| 0);
        assert_eq!(quiz.asked.len(), 4);
        assert!(quiz.asked[..2].contains(&0) && quiz.asked[..2].contains(&1));
        let view = quiz.view(&data, |_| String::new());
        assert_eq!(view.options, ["zero", "one"]);
        // Both scored answers for personality 1; did not play; a girl.
        take(&mut quiz, &data, &[Down, Confirm, Down, Confirm, Down, Confirm, Down, Confirm]);
        assert_eq!(quiz.view(&data, |_| String::new()).message, "what it is like");
        take(&mut quiz, &data, &[Confirm]);
        assert_eq!(quiz.view(&data, |_| String::new()).message, "more");
        take(&mut quiz, &data, &[Confirm]);
        assert_eq!(quiz.view(&data, |_| String::new()).message, "a girl");
        // The hero is monster 21: every starter can be its partner.
        take(&mut quiz, &data, &[Confirm]);
        let view = quiz.view(&data, |monster| format!("monster {monster}"));
        assert_eq!(view.options, ["monster 30", "monster 31", "monster 32"]);
        assert_eq!(quiz.partner_shown(), Some(30));
        // Confirmed, or not.
        assert_eq!(take(&mut quiz, &data, &[Down, Down, Confirm]), None);
        assert_eq!(quiz.partner_shown(), Some(32));
        assert_eq!(take(&mut quiz, &data, &[Down, Confirm]), None);
        assert_eq!(quiz.view(&data, |_| String::new()).options.len(), 3);
        assert_eq!(take(&mut quiz, &data, &[Confirm, Confirm]), Some((21, 32)));
    }

    #[test]
    fn the_earlier_games_boost_and_shared_types_count() {
        use Input::*;
        let data = data();
        let mut quiz = Quiz::new(&data, |_| 0);
        // One answer each way: level at 3. Having played, as a boy,
        // personality 1 gets 4 more.
        let first = quiz.asked[0];
        let towards = |personality: usize| if personality == 0 { vec![Confirm] } else { vec![Down, Confirm] };
        take(&mut quiz, &data, &towards(0));
        take(&mut quiz, &data, &towards(1));
        take(&mut quiz, &data, &[Confirm, Confirm]);
        assert_eq!((quiz.points[0], quiz.points[1], first < 2), (3, 3, true));
        assert!(quiz.played_old_game && quiz.personality == 1);
        // Monster 20 shares its type with starter 31.
        take(&mut quiz, &data, &[Confirm, Confirm, Confirm]);
        let view = quiz.view(&data, |monster| format!("monster {monster}"));
        assert_eq!(view.options, ["monster 30", "monster 32"]);
        assert_eq!(take(&mut quiz, &data, &[Confirm, Confirm]), Some((20, 30)));
    }

    #[test]
    fn level_personalities_are_drawn() {
        let data = data();
        let mut quiz = Quiz::new(&data, |_| 0);
        quiz.points[2] = 5;
        quiz.points[7] = 5;
        // The second of the two level ones draws more.
        let mut draws = [3, 9].into_iter();
        assert_eq!(quiz.winner(&data, |_| draws.next().unwrap()), 7);
        assert_eq!(quiz.winner(&data, |_| 4), 2);
    }
}
