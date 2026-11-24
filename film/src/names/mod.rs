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

    /// Where someone's head, hands and middle are, from where it stands
    /// (y downwards, facing us): the game's own points.
    pub fn points_of(&self, code: &str) -> Option<[(i32, i32); 4]> {
        self.tables.points.get(species(code)?).copied()
    }

    /// How someone goes about by nature (walking, if its tables do not say).
    pub fn movement_of(&self, code: &str) -> crate::tables::Movement {
        species(code).and_then(|number| self.tables.movements.get(number)).copied().unwrap_or(crate::tables::Movement::Walks)
    }

    /// How heavy someone is, by its tables (none: not known).
    pub fn weight_of(&self, code: &str) -> Option<u16> {
        species(code).and_then(|number| self.tables.weights.get(number)).copied()
    }

    /// What the game's tables say of someone's kind (none: not known).
    pub fn kind_of(&self, code: &str) -> Option<&crate::tables::Kind> {
        species(code).and_then(|number| self.tables.kinds.get(number))
    }

    /// Whether someone's kind ever learns that move, by level or by a
    /// machine (none: its moves are not known).
    pub fn learns(&self, code: &str, id: u16) -> Option<bool> {
        let kind = self.kind_of(code).filter(|kind| !kind.learnset.is_empty())?;
        Some(kind.learnset.iter().any(|(_, known)| *known == id) || kind.machines.contains(&id))
    }

    /// A move someone's kind has learnt by that level: of its own type and
    /// hitting if it has one, else of its type, else hitting, else any; the
    /// last learnt of them.
    pub fn own_move(&self, code: &str, level: i32) -> Option<u16> {
        let kind = self.kind_of(code)?;
        let learnt: Vec<u16> = kind.learnset.iter().filter(|(at, _)| i32::from(*at) <= level.max(1)).map(|(_, id)| *id).collect();
        let move_kind = |id: u16| self.tables.move_kinds.get(usize::from(id)).copied().unwrap_or_default();
        let mine = |id: u16| move_kind(id).0 != 0 && (move_kind(id).0 == kind.types.0 || move_kind(id).0 == kind.types.1);
        let hits = |id: u16| move_kind(id).1 > 0;
        let wanted: [&dyn Fn(u16) -> bool; 4] = [&|id| mine(id) && hits(id), &mine, &hits, &|_| true];
        wanted.iter().find_map(|wanted| learnt.iter().rev().copied().find(|id| wanted(*id)))
    }

    /// The word of a monster by its number: its own name's, else the first.
    pub fn word_of_number(&self, number: &str) -> Option<String> {
        let named: Vec<(&String, &crate::tables::Named)> = self.tables.monsters.iter().filter(|(_, row)| row.0 == number).collect();
        named.iter().find(|(word, row)| **word == plain(&row.1)).or(named.first()).map(|(word, _)| (*word).clone())
    }

    /// The next form of someone's kind, the first that evolves from it
    /// (`evolution`): its word (none: the last of its family).
    pub fn next_form(&self, code: &str) -> Option<String> {
        let number: u16 = species(code)?.parse().ok()?;
        let mut later: Vec<u16> = self.tables.kinds.iter().filter(|(_, kind)| kind.from == number).filter_map(|(at, _)| at.parse().ok()).collect();
        later.sort();
        later.into_iter().find_map(|at| self.word_of_number(&at.to_string()))
    }

    /// Whether someone's sprite has that animation (none: its sheets are
    /// not known, a variant painted by hand, no data).
    pub fn has_animation(&self, code: &str, group: u16) -> Option<bool> {
        self.tables.sprite_groups.get(species(code)?).map(|groups| groups.contains(&group))
    }
}
