//! Where things are in the text: letters, bytes and the language's spots,
//! a line at a place, a word, a number to drag, what is found.

use std::ops::Range;

use film::service::Spot;

/// The number at (or just before) a letter of the text, not part of a word (`krabby2` is no number): where, how
/// long, its value, its decimals.
pub(super) fn number_at(text: &str, at: usize) -> Option<(usize, usize, f64, usize)> {
    let letters: Vec<char> = text.chars().collect();
    let digit = |at: usize| letters.get(at).is_some_and(|letter| letter.is_ascii_digit() || *letter == '.');
    let at = if digit(at) {
        at
    } else if at > 0 && digit(at - 1) {
        at - 1
    } else {
        return None;
    };
    let (mut start, mut end) = (at, at + 1);
    while start > 0 && digit(start - 1) {
        start -= 1;
    }
    while digit(end) {
        end += 1;
    }
    if start > 0 && letters[start - 1] == '-' && (start < 2 || !letters[start - 2].is_alphanumeric()) {
        start -= 1;
    }
    if start > 0 && (letters[start - 1].is_alphabetic() || letters[start - 1] == '_') {
        return None;
    }
    let written: String = letters[start..end].iter().collect();
    let value: f64 = written.parse().ok()?;
    let decimals = written.split_once('.').map_or(0, |(_, after)| after.len());
    Some((start, end - start, value, decimals))
}

/// The place (line, column, from 1) of a letter's index.
pub(super) fn spot_of(text: &str, at: usize) -> Spot {
    let (mut line, mut column) = (1, 1);
    for letter in text.chars().take(at) {
        if letter == '\n' {
            (line, column) = (line + 1, 1);
        } else {
            column += 1;
        }
    }
    Spot { line, column }
}

/// The letter's index of a place.
pub(super) fn index_of(text: &str, spot: Spot) -> usize {
    let mut at = 0;
    for (number, line) in text.split('\n').enumerate() {
        if number + 1 == spot.line {
            return at + spot.column.saturating_sub(1).min(line.chars().count());
        }
        at += line.chars().count() + 1;
    }
    text.chars().count()
}

/// The byte of a letter's index.
pub(super) fn byte_of(text: &str, at: usize) -> usize {
    text.char_indices().nth(at).map_or(text.len(), |(byte, _)| byte)
}

/// The line (from 0) a letter's index is on, its first letter's index, its text.
pub(super) fn line_at(text: &str, at: usize) -> (usize, usize, String) {
    let mut start = 0;
    for (number, line) in text.split('\n').enumerate() {
        let long = line.chars().count();
        if at <= start + long {
            return (number, start, line.to_string());
        }
        start += long + 1;
    }
    (0, 0, String::new())
}

pub(super) fn letter_of_word(letter: char) -> bool {
    letter.is_alphanumeric() || letter == '_' || letter == '-'
}

/// Where a word or words are found (letters' indices), its case counting or not.
pub(super) fn found_in(text: &str, sought: &str, case: bool) -> Vec<Range<usize>> {
    if sought.is_empty() {
        return Vec::new();
    }
    let fold = |said: &str| if case { said.to_string() } else { said.to_lowercase() };
    let (hay, needle): (Vec<char>, Vec<char>) = (fold(text).chars().collect(), fold(sought).chars().collect());
    if hay.len() != text.chars().count() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at + needle.len() <= hay.len() {
        if hay[at..at + needle.len()] == needle[..] {
            out.push(at..at + needle.len());
            at += needle.len();
        } else {
            at += 1;
        }
        if out.len() >= 2000 {
            break;
        }
    }
    out
}

/// The word at a place of the text (letters of a word around it), if there is one.
pub(super) fn word_at(text: &str, at: usize) -> Option<String> {
    let letters: Vec<char> = text.chars().collect();
    let mut start = at.min(letters.len());
    while start > 0 && letter_of_word(letters[start - 1]) {
        start -= 1;
    }
    let mut end = at.min(letters.len());
    while end < letters.len() && letter_of_word(letters[end]) {
        end += 1;
    }
    (end > start).then(|| letters[start..end].iter().collect())
}
