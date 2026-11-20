//! Sentry duty at the guild: visitors come one after the other, and whose
//! footprint it is has to be told among four names, the faster the more
//! points; a duty with no mistake earns more (`SentryStateFinalizePoints`:
//! 2000 points). The visitors are the game's (`SENTRY_DUTY_MONSTER_IDS`).
// The footprints themselves are not the game's pictures: the visitor's
// portrait, in shadow, stands for them.

use crate::services::{Input, View};

/// The `message_Menu` that starts the game.
pub const MENU: u16 = 57;
/// Visitors in a duty, and the time each is given (seconds).
pub const ROUNDS: usize = 10;
const SECONDS: f32 = 10.0;
/// Points of a right answer, at once and at the last moment.
const MOST: i32 = 1000;
const LEAST: i32 = 100;
const PERFECT_BONUS: i32 = 2000;
const CHOICES: usize = 4;

#[derive(Clone, Debug)]
pub struct Sentry {
    /// The visitors, in order, then the four names of each round (the
    /// visitor among them).
    visitors: Vec<u16>,
    choices: Vec<[u16; CHOICES]>,
    round: usize,
    cursor: usize,
    /// What is left of the round's time.
    left: f32,
    pub points: i32,
    pub right: usize,
    /// The last answer: whether it was right, shown until a key.
    told: Option<bool>,
}

impl Sentry {
    /// A duty with visitors drawn from `pool`; `random(n)` draws a number
    /// below `n`.
    pub fn new(pool: &[u16], mut random: impl FnMut(usize) -> usize) -> Self {
        let pool: Vec<u16> = if pool.len() >= CHOICES { pool.to_vec() } else { (1..=CHOICES as u16 * 2).collect() };
        let visitors: Vec<u16> = (0..ROUNDS).map(|_| pool[random(pool.len())]).collect();
        let choices = visitors
            .iter()
            .map(|&visitor| {
                let mut four = [visitor; CHOICES];
                let mut others: Vec<u16> = pool.iter().copied().filter(|&other| other != visitor).collect();
                for slot in four.iter_mut().skip(1) {
                    *slot = others.swap_remove(random(others.len()));
                }
                let at = random(CHOICES);
                four.swap(0, at);
                four
            })
            .collect();
        Self { visitors, choices, round: 0, cursor: 0, left: SECONDS, points: 0, right: 0, told: None }
    }

    /// The visitor of the round, while one is guessed.
    pub fn visitor(&self) -> Option<u16> {
        self.visitors.get(self.round).copied()
    }

    /// Whether the duty is over.
    pub fn over(&self) -> bool {
        self.round >= ROUNDS && self.told.is_none()
    }

    /// How it went (0: badly, to 3: perfectly), as the guild judges it.
    pub fn rating(&self) -> u8 {
        match (self.right, self.points) {
            (ROUNDS, _) => 3,
            (_, points) if points >= 6000 => 2,
            (_, points) if points >= 3000 => 1,
            _ => 0,
        }
    }

    /// Time passes: a round not answered in time is missed.
    pub fn tick(&mut self, seconds: f32) {
        if self.told.is_some() || self.round >= ROUNDS {
            return;
        }
        self.left -= seconds;
        if self.left <= 0.0 {
            self.answer(None);
        }
    }

    fn answer(&mut self, chosen: Option<u16>) {
        let right = chosen.is_some() && chosen == self.visitor();
        if right {
            self.right += 1;
            self.points += LEAST + ((MOST - LEAST) as f32 * self.left.max(0.0) / SECONDS) as i32;
        }
        self.told = Some(right);
        self.round += 1;
        if self.round == ROUNDS && self.right == ROUNDS {
            self.points += PERFECT_BONUS;
        }
    }

    pub fn press(&mut self, input: Input) {
        if self.told.is_some() {
            if matches!(input, Input::Confirm | Input::Cancel) {
                (self.told, self.left, self.cursor) = (None, SECONDS, 0);
            }
            return;
        }
        let Some(choices) = self.choices.get(self.round) else {
            return;
        };
        match input {
            Input::Down => self.cursor = (self.cursor + 1) % CHOICES,
            Input::Up => self.cursor = (self.cursor + CHOICES - 1) % CHOICES,
            Input::Confirm => self.answer(Some(choices[self.cursor])),
            _ => {}
        }
    }

    /// What to show; `name` names a monster.
    pub fn view(&self, name: impl Fn(u16) -> String) -> View {
        if let Some(right) = self.told {
            let visitor = self.visitors.get(self.round - 1).map_or(String::new(), |&visitor| name(visitor));
            let message = match right {
                true => format!("Right! It was {visitor}.\nPoints: {}", self.points),
                false => format!("Wrong... It was {visitor}.\nPoints: {}", self.points),
            };
            return View { message, options: Vec::new(), cursor: 0 };
        }
        let options = self.choices.get(self.round).map_or(Vec::new(), |four| four.iter().map(|&monster| name(monster)).collect());
        let message = format!(
            "Visitor {} of {ROUNDS}: whose footprints are these?\nTime: {}   Points: {}",
            self.round + 1,
            self.left.ceil().max(0.0) as i32,
            self.points
        );
        View { message, options, cursor: self.cursor }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_answers_score_by_speed() {
        let pool: Vec<u16> = (1..=20).collect();
        let mut seed = 7_usize;
        let mut sentry = Sentry::new(&pool, |below| {
            seed = seed.wrapping_mul(31).wrapping_add(11);
            seed % below
        });
        for _ in 0..ROUNDS {
            let visitor = sentry.visitor().unwrap();
            let at = sentry.choices[sentry.round].iter().position(|&monster| monster == visitor).unwrap();
            assert_eq!(sentry.choices[sentry.round].iter().filter(|&&monster| monster == visitor).count(), 1);
            for _ in 0..at {
                sentry.press(Input::Down);
            }
            sentry.press(Input::Confirm);
            sentry.press(Input::Confirm);
        }
        assert!(sentry.over());
        assert_eq!((sentry.right, sentry.points, sentry.rating()), (ROUNDS, ROUNDS as i32 * MOST + PERFECT_BONUS, 3));
        // Too slow: missed.
        let mut late = Sentry::new(&pool, |_| 0);
        late.tick(SECONDS + 1.0);
        late.press(Input::Confirm);
        assert_eq!((late.round, late.right, late.points), (1, 0, 0));
    }
}
