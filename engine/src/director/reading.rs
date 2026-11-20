//! How fast a line is written out and how long it stays to be read, for all
//! who need it: the message box writes at [`LETTERS_A_FRAME`], a film gives
//! its lines [`line_frames`], a film going on by itself reads one
//! [`read_after`] it is written out, and the check finds a line gone too
//! fast under [`least_shown`].

use crate::rehearsal::without_tags;

/// The letters the box writes out a frame at the game's usual speed (the
/// options' “normal”).
pub const LETTERS_A_FRAME: f32 = 1.0;
/// The least a line stays, all told.
pub const SHORTEST_LINE: f32 = 58.0;
/// What reading takes once a line is written out: from the start, and for
/// a letter; a shout is read faster.
const READ_START: f32 = 38.0;
const READ_LETTER: f32 = 2.2;
const SHOUTED: f32 = 0.8;
/// Under this, once written out, a line went too fast to be read: at the
/// least, and for a letter.
const ENOUGH_START: f32 = 18.0;
const ENOUGH_LETTER: f32 = 0.9;

/// The letters of a page as the box writes them: tags and line breaks left out.
pub fn letters(page: &str) -> usize {
    without_tags(page).chars().filter(|&letter| letter != '\n').count()
}

/// The characters a page takes to write out: its letters, and its pauses
/// (`[W:n]`, as long as n letters).
pub fn units(page: &str) -> usize {
    let pauses: usize = page.match_indices("[W:").filter_map(|(at, _)| page[at + 3..at + page[at..].find(']')?].parse::<usize>().ok()).sum();
    letters(page) + pauses
}

/// The frames a page takes to write out, at the usual speed.
pub fn writing_frames(page: &str) -> f32 {
    units(page) as f32 / LETTERS_A_FRAME
}

/// The frames so many letters are read in, once written out.
pub fn read_after(letters: usize, shouted: bool) -> f32 {
    let frames = READ_START + letters as f32 * READ_LETTER;
    if shouted { frames * SHOUTED } else { frames }
}

/// The frames a line stays: written out, its speaker's name before it
/// (none: a narration), then read.
pub fn line_frames(name: &str, text: &str, shouted: bool) -> u32 {
    let page = if name.is_empty() { text.to_string() } else { format!("{name}: {text}") };
    SHORTEST_LINE.max(writing_frames(&page) + read_after(letters(text), shouted)).floor() as u32
}

/// The frames a page must show at the least to be read: written out, then
/// a short read.
pub fn least_shown(page: &str) -> f32 {
    writing_frames(page) + ENOUGH_START + letters(page) as f32 * ENOUGH_LETTER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_stays_longer_than_it_takes_to_write_and_be_read() {
        let page = "[CS:N]Koffing[CR]: Kof! Kof!";
        assert_eq!(units(page), 18);
        assert!(line_frames("Koffing", "Kof! Kof!", false) as f32 >= least_shown(page));
        assert!(line_frames("Koffing", "Kof! Kof!", true) as f32 >= least_shown(page));
        let long = "A very long line indeed, one that takes up both of its rows, and a bit more.";
        assert!(line_frames("Pikachu", long, true) as f32 >= least_shown(&format!("[CS:N]Pikachu[CR]: {long}")));
        assert_eq!(units("Ah[W:30]... yes."), 10 + 30);
    }
}
