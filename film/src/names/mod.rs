//! What a film calls those it shows: someone by a name given or a Pokémon's
//! (`krabby2`, a second one; a variant's own word), a crowd by its name; and
//! things, places, ways (`things`, `places`).

pub mod places;
pub mod things;

use crate::film::Film;
use crate::words::plain;

/// The species number of someone's code (`m25.1`: `25`).
pub fn species(code: &str) -> Option<&str> {
    code.strip_prefix('m')?.split('.').next()
}

impl Film {
    /// The code of someone by the word used, if it is someone: a name given,
    /// a Pokémon's, or one with a number for another of its kind.
    pub fn actor(&mut self, word: &str) -> Option<String> {
        let key = plain(word);
        if !key.bytes().any(|letter| letter.is_ascii_lowercase()) {
            return None;
        }
        if let Some(code) = self.actors.get(&key) {
            return Some(code.clone());
        }
        let base = key.trim_end_matches(|letter: char| letter.is_ascii_digit());
        let copy = key[base.len()..].parse::<u32>().map_or(0, |copy| copy.saturating_sub(1));
        let row = self.tables.monsters.get(if base.is_empty() { key.as_str() } else { base })?;
        let code = if copy > 0 { format!("m{}.{copy}", row.0) } else { format!("m{}", row.0) };
        self.give_name(key, code.clone());
        Some(code)
    }

    pub fn give_name(&mut self, name: String, code: String) {
        if !self.actors.contains_key(&name) {
            self.names.push(name.clone());
        }
        self.actors.insert(name, code);
    }

    /// The code of someone named, for what is found on the stage.
    pub fn code(&mut self, word: &str) -> String {
        self.actor(word).unwrap_or_else(|| plain(word))
    }

    /// Who a line is about; told when it is no one.
    pub fn who(&mut self, line: &str, rest: &[String], number: usize) -> Option<String> {
        let who = rest.first().map_or(String::new(), |word| plain(word));
        if self.actor(&who).is_none() {
            self.unknown_someone(number, rest.first().map_or(line, String::as_str));
            return None;
        }
        Some(who)
    }

    /// Someone the film does not know, the nearest names given.
    pub fn unknown_someone(&mut self, number: usize, word: &str) {
        let tables = self.tables.clone();
        let given: Vec<String> = self.actors.keys().cloned().collect();
        let known = given.iter().map(String::as_str).chain(tables.monsters.keys().map(String::as_str));
        let look = format!("scene_player search {}", plain(word));
        self.unknown(number, "unknown Pokémon", word, known, &look);
    }

    /// Those words name, each someone or a crowd's name (`onlookers`, a `gang`):
    /// none if one of them is neither.
    pub fn those(&mut self, words: &[String]) -> Option<Vec<String>> {
        let mut found = Vec::new();
        for word in words {
            let key = plain(word);
            match self.groups.get(&key).cloned() {
                Some(members) => found.extend(members),
                None if self.actor(&key).is_some() => found.push(key),
                None => return None,
            }
        }
        (!found.is_empty()).then_some(found)
    }

    /// Whether the words name a crowd among those they name.
    pub fn names_a_crowd(&self, words: &[String]) -> bool {
        words.iter().any(|word| self.groups.contains_key(&plain(word)))
    }

    /// Who a word names, all of a crowd for a crowd's name.
    pub fn members(&self, word: &str) -> Vec<String> {
        self.groups.get(word).cloned().unwrap_or_else(|| vec![word.to_string()])
    }

    /// How big someone shows: wide, high.
    pub fn body_of(&self, code: &str) -> Option<(i32, i32)> {
        self.tables.bodies.get(species(code)?).copied()
    }
}
