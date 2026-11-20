//! The numbers a word keeps for a moment as the scene plays (a point found,
//! a sum being made): each given by the film, a name of its own for each use
//! (`_t12_rocks_x`), never one another word or another use of the same word
//! has. Two uses at once, one inside the other or one alongside the other
//! (`&`), cannot take each other's numbers.
//!
//! A word asks for them (`film.temps.number("sum")`, `film.temps.point("rocks")`)
//! and they are its own until the word's line is over (`Temps::open`,
//! `Temps::close`, around each word and each recipe called): then they are
//! released, and no one is given their names again.

use std::fmt;

use engine::{Num, Point};

/// A number kept for a moment, by its own name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Temp(String);

impl Temp {
    /// Its name, as the engine keeps it.
    pub fn name(&self) -> &str {
        &self.0
    }

    /// As a word reads it in a line (`{_t3_sum}`), its value as it plays.
    pub fn read(&self) -> String {
        format!("{{{}}}", self.0)
    }

    /// As a beat reads it.
    pub fn num(&self) -> Num {
        Num::Var(self.0.clone())
    }
}

impl From<&Temp> for String {
    fn from(temp: &Temp) -> String {
        temp.0.clone()
    }
}

impl fmt::Display for Temp {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(&self.0)
    }
}

/// A point kept for a moment: its two numbers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TempPoint {
    pub x: Temp,
    pub y: Temp,
}

impl TempPoint {
    /// As a beat reads it: its value as it plays.
    pub fn point(&self) -> Point {
        Point { x: self.x.num(), y: self.y.num() }
    }
}

/// What the film has given out, and what is still held.
#[derive(Clone, Debug, Default)]
pub struct Temps {
    /// How many were given: the next one's number.
    made: usize,
    /// Those held, the latest last.
    held: Vec<String>,
}

impl Temps {
    /// A number of its own (`what`: what it is for, in its name).
    pub fn number(&mut self, what: &str) -> Temp {
        self.made += 1;
        let name = format!("_t{}_{what}", self.made);
        self.held.push(name.clone());
        Temp(name)
    }

    /// A point of its own: two numbers, `<name>_x` and `<name>_y`.
    pub fn point(&mut self, what: &str) -> TempPoint {
        self.made += 1;
        let (x, y) = (format!("_t{}_{what}_x", self.made), format!("_t{}_{what}_y", self.made));
        self.held.extend([x.clone(), y.clone()]);
        TempPoint { x: Temp(x), y: Temp(y) }
    }

    /// A word begins: where what it is given starts.
    pub fn open(&self) -> usize {
        self.held.len()
    }

    /// The word is over: what it was given (since `open`) is released.
    pub fn close(&mut self, opened: usize) {
        self.held.truncate(opened.min(self.held.len()));
    }

    /// The names held now.
    pub fn held(&self) -> &[String] {
        &self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_one_inside_another_or_alongside_never_share_a_name() {
        let mut temps = Temps::default();
        let outer = temps.open();
        let rocks = temps.point("rocks");
        // (A word inside the first: its own numbers.)
        let inner = temps.open();
        let again = temps.point("rocks");
        let sum = temps.number("sum");
        assert_ne!(rocks, again);
        assert!(![&rocks.x, &rocks.y].contains(&&sum) && ![&again.x, &again.y].contains(&&sum));
        temps.close(inner);
        assert_eq!(temps.held(), [rocks.x.name(), rocks.y.name()]);
        temps.close(outer);
        assert!(temps.held().is_empty());
        // (Released: the next use, maybe played alongside, has new names.)
        let next = temps.open();
        let later = temps.point("rocks");
        assert!(later != rocks && later != again);
        temps.close(next);
    }
}
