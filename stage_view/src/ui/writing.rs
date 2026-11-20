//! Text on the canvas, and the game's markup in it.

use super::*;

impl Canvas {
    /// Writes one line of text; returns its width.
    pub fn text(&mut self, art: &UiArt, x: i32, y: i32, text: &str, color: [u8; 3]) -> usize {
        let (width, height, rgba) = art.font.render_colored(text, color);
        self.blit(x, y, width, height, &rgba);
        width.saturating_sub(1)
    }

    /// Text without its shadow, for something light under it.
    pub(crate) fn text_plain(&mut self, art: &UiArt, x: i32, y: i32, text: &str, color: [u8; 3]) {
        let (width, height, rgba) = art.font.render_plain(text, color);
        self.blit(x, y, width, height, &rgba);
    }

    /// Text ending at `right`.
    pub fn text_right(&mut self, art: &UiArt, right: i32, y: i32, text: &str, color: [u8; 3]) {
        let width = art.font.width(text) as i32;
        self.text(art, right - width, y, text, color);
    }

    /// An entry of `markfont.dat`, on a line of text.
    pub fn mark(&mut self, art: &UiArt, x: i32, y: i32, index: usize) -> usize {
        match art.marks.get(index) {
            Some((width, height, rgba)) => {
                self.blit(x, y, *width, *height, rgba);
                *width
            }
            None => 0,
        }
    }

    /// Text with the game's markup, from `(x, y)` in a column `width` wide,
    /// lines `TEXT_PITCH` apart. Understood: `\n`, `[CN]` (centre the
    /// line), `[CLUM_SET:n]` (continue at column n), `[M:Bn]` (a button
    /// from `markfont.dat`), `[CS:c]`...`[CR]` (colour). Other tags are
    /// dropped.
    pub fn markup(&mut self, art: &UiArt, x: i32, y: i32, width: usize, markup: &str) {
        self.markup_end(art, x, y, width, markup);
    }

    /// Draws like `markup`, and tells where the text ends: the place
    /// after its last character, on the line (its top) that holds it.
    pub fn markup_end(&mut self, art: &UiArt, x: i32, y: i32, width: usize, markup: &str) -> (i32, i32) {
        let mut end = (x, y);
        for (row, line) in markup.split('\n').enumerate() {
            let top = y + (row * TEXT_PITCH) as i32;
            let centred = line.starts_with("[CN]");
            let plain_width: usize = if centred { art.font.width(&strip_tags(line)) } else { 0 };
            let mut at = match centred {
                true => x + (width as i32 - plain_width as i32) / 2,
                false => x,
            };
            let mut color = color::WHITE;
            let mut rest = line;
            while !rest.is_empty() {
                let Some(open) = rest.find('[') else {
                    at += self.text(art, at, top, rest, color) as i32;
                    break;
                };
                if open > 0 {
                    at += self.text(art, at, top, &rest[..open], color) as i32;
                }
                let Some(close) = rest[open..].find(']') else {
                    break;
                };
                let tag = &rest[open + 1..open + close];
                if let Some(column) = tag.strip_prefix("CLUM_SET:") {
                    at = x + column.parse::<i32>().unwrap_or(0);
                } else if let Some(index) = tag.strip_prefix("M:B") {
                    at += self.mark(art, at, top, index.parse().unwrap_or(usize::MAX)) as i32;
                } else if let Some(code) = tag.strip_prefix("CS:") {
                    color = code_color(art, code);
                } else if tag == "CR" || tag == "CR:" {
                    color = color::WHITE;
                }
                rest = &rest[open + close + 1..];
            }
            // (A line break after the last character is not the end.)
            if !strip_tags(line).is_empty() {
                end = (at, top);
            }
        }
        end
    }
}

/// The first `count` characters of marked-up text, tags kept: what a
/// message being written out shows.
pub fn reveal(markup: &str, count: usize) -> &str {
    let mut shown = 0;
    let mut characters = markup.char_indices();
    while let Some((at, character)) = characters.next() {
        match character {
            '[' => {
                let close = markup[at..].find(']').map_or(markup.len(), |close| at + close);
                // (A pause, `[W:n]`: as long as n characters take.)
                if let Some(units) = pause_of(&markup[at..close]) {
                    if shown + units > count {
                        return &markup[..at];
                    }
                    shown += units;
                }
                // (Past the tag.)
                for (inner, _) in characters.by_ref() {
                    if inner >= close {
                        break;
                    }
                }
            }
            '\n' => {}
            _ if shown == count => return &markup[..at],
            _ => shown += 1,
        }
    }
    markup
}

/// A pause's length, from its tag up to its `]` (`[W:30`): written out
/// as so many characters would be, nothing shown.
fn pause_of(tag: &str) -> Option<usize> {
    tag.strip_prefix("[W:")?.parse().ok()
}

/// Whether, so many characters written out, a pause is being waited
/// through (`[W:n]`): nothing is written meanwhile.
pub fn pausing(markup: &str, count: usize) -> bool {
    let written = reveal(markup, count);
    strip_tags(written).chars().filter(|&character| character != '\n').count() + pause_units(written) < count.min(units(markup))
}

/// How many characters marked-up text takes to write out, its pauses
/// counted (as the films count them).
fn units(markup: &str) -> usize {
    engine::units(markup)
}

/// How long the pauses of marked-up text are (`[W:n]`), as characters.
pub fn pause_units(markup: &str) -> usize {
    markup.match_indices("[W:").filter_map(|(at, _)| pause_of(&markup[at..at + markup[at..].find(']')?])).sum()
}

/// Where marked-up text waits for a key (`[K]`): how many characters are
/// written out before each.
pub fn key_stops(markup: &str) -> Vec<usize> {
    let (mut stops, mut shown, mut rest) = (Vec::new(), 0, markup);
    while let Some(open) = rest.find('[') {
        shown += rest[..open].chars().filter(|&character| character != '\n').count();
        let Some(close) = rest[open..].find(']') else {
            return stops;
        };
        if &rest[open..open + close + 1] == "[K]" {
            stops.push(shown);
        }
        shown += pause_of(&rest[open..open + close]).unwrap_or(0);
        rest = &rest[open + close + 1..];
    }
    stops
}

/// `text` without the game's `[...]` tags.
pub fn strip_tags(text: &str) -> String {
    let mut plain = String::new();
    let mut tag = false;
    for c in text.chars() {
        match c {
            '[' => tag = true,
            ']' if tag => tag = false,
            c if !tag => plain.push(c),
            _ => {}
        }
    }
    plain
}
