//! A thing by its words: an item of the game (`Oran Berry`), one of its props
//! (`chest`, `o186`, the game's own name), a picture of ours or of the film's;
//! `chest2` is a second one.

use crate::film::{Film, Kind};
use crate::words::{digits, plain};

/// A thing as a film means it: its key, how it is shown and called.
#[derive(Clone, Debug)]
pub struct Thing {
    pub key: String,
    pub kind: Kind,
    pub name: String,
}

/// A thing by its words, if the game or the film has it.
pub fn known_thing(film: &Film, words: &[String]) -> Option<Thing> {
    let name = plain(&words.concat());
    let bare = name.trim_end_matches(|letter: char| letter.is_ascii_digit());
    [name.as_str(), bare]
        .into_iter()
        .filter(|key| !key.is_empty())
        .find_map(|key| kind_of(film, key).map(|(kind, shown)| Thing { key: key.to_string(), kind, name: shown }))
}

/// A thing by one word.
pub fn thing_named(film: &Film, word: &str) -> Option<Thing> {
    known_thing(film, &[word.to_string()])
}

fn kind_of(film: &Film, key: &str) -> Option<(Kind, String)> {
    let tables = film.tables.clone();
    if let Some((file, cells)) = film.own.get(key) {
        return Some((Kind::Drawn(file.clone(), *cells), key.to_string()));
    }
    if let Some(item) = tables.items.get(key) {
        return Some((Kind::Item(item.0.clone()), item.1.clone()));
    }
    if let Some((file, cells)) = tables.drawings.get(key) {
        return Some((Kind::Drawn(file.clone(), *cells), key.to_string()));
    }
    let number = key.strip_prefix('o').filter(|number| digits(number)).and_then(|number| number.parse().ok());
    let prop = number.filter(|object| tables.prop_frames.contains_key(object)).or_else(|| tables.props.get(key).copied());
    prop.map(|object| (Kind::Prop(object), key.to_string()))
}

/// A thing by its words; told when there is none.
pub fn thing_said(film: &mut Film, words: &[String], number: usize) -> Option<Thing> {
    if let Some(thing) = known_thing(film, words) {
        return Some(thing);
    }
    let tables = film.tables.clone();
    let own: Vec<String> = film.own.keys().cloned().collect();
    let names = tables.items.keys().chain(tables.props.keys()).chain(tables.drawings.keys()).chain(own.iter()).map(String::as_str);
    film.unknown(number, "unknown thing", &words.join(" "), names, "scene_player items");
    None
}

/// The thing the first words say, the longest there is: how many words.
pub fn leading_thing(film: &Film, words: &[String]) -> Option<(usize, Thing)> {
    // (A name is never its numbers: `put object bucket 456 186` puts a bucket
    // at that point, not one called “bucket 456 186”.)
    let named = words.iter().take_while(|word| !word.chars().all(|letter| letter.is_ascii_digit() || letter == '-')).count().max(1);
    (1..=words.len().min(4).min(named)).rev().find_map(|size| known_thing(film, &words[..size]).map(|thing| (size, thing)))
}
