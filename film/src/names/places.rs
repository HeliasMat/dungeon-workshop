//! Places, ways and the game's pictures and moves by their words; a word not
//! known is told with the nearest.

use crate::check::{closest, unknown};
use crate::film::Film;
use crate::tables::{HOME, MAP_PICTURE, NOT_FOR_FILMS, Named, PLACES, Picture, Tables};
use crate::words::{of, plain};

impl Tables {
    /// Every name of a place: ours, those made by the `decor` builder, the game's.
    pub fn place_names_known(&self) -> impl Iterator<Item = &str> {
        PLACES.iter().map(|(name, _)| *name).chain(self.decors.keys().map(String::as_str)).chain(self.levels.keys().map(String::as_str))
    }

    /// A place by its word; what is wrong with it, if anything.
    pub fn place_of(&self, word: &str) -> Result<(u16, i32, i32), String> {
        let key = plain(word);
        if let Some(place) = of(&PLACES, &key) {
            return Ok(place);
        }
        if let Some(place) = self.decors.get(&key) {
            return Ok(*place);
        }
        if let Some(picture) = self.pictures.get(&key) {
            if NOT_FOR_FILMS.contains(&picture.category.as_str()) {
                return Err(format!("“{word}” is not a picture for films ({})", picture.category));
            }
            return Ok((picture.level, picture.x, picture.y));
        }
        if let Some(level) = self.levels.get(&key) {
            return Ok((level.level, level.x, level.y));
        }
        let close = closest(&key, self.place_names_known(), 5, 0.4);
        Err(unknown("unknown place", word, &close, &format!("scene_player search {word}")))
    }
}

/// The sides of every place: a way out that is not one of its own.
pub const SIDES: [&str; 2] = ["left", "right"];

impl Film {
    /// A way in or out a line says: a side, or one of the place's own; told
    /// when it is neither.
    pub fn way_said(&mut self, word: &str, number: usize) -> Option<String> {
        let word = plain(word);
        if SIDES.contains(&word.as_str()) || self.entrances.contains(&word) {
            return Some(word);
        }
        let known: Vec<String> = SIDES.iter().map(|side| side.to_string()).chain(self.entrances.iter().cloned()).collect();
        self.unknown(number, "unknown entrance", &word, known.iter().map(String::as_str), &known.join(", "));
        None
    }

    /// A picture of the game by its name; told when there is none.
    pub fn picture_named(&mut self, word: &str, number: usize) -> Option<Picture> {
        let found = self.tables.pictures.get(&plain(word)).cloned();
        if found.is_none() {
            let tables = self.tables.clone();
            self.unknown(number, "unknown picture", word, tables.pictures.keys().map(String::as_str), "scene_player illustrations");
        }
        found
    }

    /// A picture of the catalogue that must be there; a dark one where it is not.
    pub fn picture(&self, name: &str) -> Picture {
        self.tables.pictures.get(name).cloned().unwrap_or(Picture { level: 0, category: String::new(), x: 128, y: 96 })
    }

    /// An animation of the game by its name; told when there is none.
    pub fn animation_named(&mut self, word: &str, number: usize) -> Option<String> {
        let name = plain(word);
        if self.tables.animations.contains_key(&name) {
            return Some(name);
        }
        let tables = self.tables.clone();
        self.unknown(number, "unknown animation", word, tables.animations.keys().map(String::as_str), "scene_player illustrations");
        None
    }

    /// A move as written: its number and its name; told when there is none.
    pub fn move_named(&mut self, words: &[String], number: usize) -> Option<Named> {
        let tables = self.tables.clone();
        let name = plain(&words.concat());
        if let Some(row) = tables.moves.get(&name) {
            return Some(row.clone());
        }
        // (The nearest by the game's own names.)
        let close: Vec<String> = closest(&name, tables.moves.keys().map(String::as_str), 4, 0.5).iter().map(|name| tables.moves[name].1.clone()).collect();
        self.wrong(number, unknown("unknown move", &words.join(" "), &close, ""));
        None
    }

    /// The number of a move as written; told when there is none.
    pub fn move_said(&mut self, words: &[String], number: usize) -> Option<String> {
        self.move_named(words, number).map(|row| row.0)
    }

    /// A place's point on the map's picture of the clouds as they are now.
    pub fn on_the_map(&mut self, word: &str, number: usize) -> Option<(i32, i32)> {
        let key = plain(word);
        let at = if matches!(key.as_str(), "town" | "guild" | "home" | "treasuretown") { Some(HOME) } else { self.tables.map_spots.get(&key).copied() };
        let Some(at) = at else {
            let tables = self.tables.clone();
            self.unknown(number, "unknown place on the map", word, tables.map_spots.keys().map(String::as_str), "");
            return None;
        };
        Some((at.0 + (self.clouds - 1) % 2 * MAP_PICTURE.0, at.1 + (self.clouds - 1) / 2 * MAP_PICTURE.1))
    }
}
