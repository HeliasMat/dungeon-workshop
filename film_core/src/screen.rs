//! Words as the screen shows them: cut to the box, a bubble or the screen,
//! their marks, and the game's colours on names.

use crate::film::Film;
use crate::words::plain;

/// A line of a box, in letters; the most lines a box holds.
pub const WIDE: usize = 34;
pub const BOX_LINES: usize = 3;
/// Between two lines of a box, as the engine reads it.
pub const BREAK: &str = "\\n";
/// A line of the screen, of a bubble, in letters.
const SCREEN_WIDE: usize = 38;
const BUBBLE_WIDE: usize = 26;

/// Words cut to lines of that many letters.
/// Words joined back as English writes them: no space before `!`, `?`, `,`, `;` (a line's
/// words are read one by one, `!` among them).
pub fn glued(text: &str) -> String {
    let mut text = text.to_string();
    for mark in ["!", "?", ",", ";"] {
        text = text.replace(&format!(" {mark}"), mark);
    }
    text
}

fn cut(text: &str, wide: usize) -> Vec<String> {
    let text = &glued(text);
    let (mut lines, mut line) = (Vec::new(), String::new());
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > wide {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Words cut to the lines the screen holds.
pub fn on_screen(text: &str) -> String {
    cut(text, SCREEN_WIDE).join(BREAK)
}

/// Words in lines that fit a bubble.
pub fn in_a_bubble(text: &str) -> String {
    cut(text, BUBBLE_WIDE).join(BREAK)
}

/// What is left of a line once its marks (`[W:90]`) are out.
pub fn without_marks(text: &str) -> String {
    let (mut out, mut inside) = (String::new(), false);
    for letter in text.chars() {
        match letter {
            '[' => inside = true,
            ']' if inside => inside = false,
            _ if !inside => out.push(letter),
            _ => {}
        }
    }
    out
}

/// A line cut to the box: its marks count for nothing.
pub fn wrap(text: &str) -> Vec<String> {
    let (mut lines, mut line) = (Vec::new(), String::new());
    for word in text.split_whitespace() {
        let with = format!("{line} {word}");
        if !line.is_empty() && without_marks(&with).chars().count() > WIDE {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else if line.is_empty() {
            line = word.to_string();
        } else {
            line = with;
        }
    }
    lines.push(line);
    lines
}

/// A line of the film said in the box, as the box shows it: its lines, its
/// marks and stars out (`[1s]`, `*word*`); none if it is not a line said.
pub fn boxed(line: &str) -> Option<Vec<String>> {
    let (_, _, said) = crate::text::spoken(line.trim())?;
    Some(wrap(&without_marks(said).replace('*', "")))
}

/// A name with its first letter up.
pub fn capitalized(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
}

/// A line with the game's colours: places, Pokémon, and items but at the start
/// of a sentence (`Rock, come!`).
pub fn coloured(film: &Film, line: &str) -> String {
    let (mut out, mut at) = (String::new(), 0);
    while at < line.len() {
        if let Some(found) = place_at(&line[at..], &film.tables.place_names) {
            let name = &line[at..at + found];
            out.push_str("[CS:P]");
            out.push_str(&name.replace(BREAK, &format!("[CR]{BREAK}[CS:P]")));
            out.push_str("[CR]");
            at += found;
            continue;
        }
        let rest = &line[at..];
        let letter = rest.chars().next().unwrap_or(' ');
        if rest.starts_with(BREAK) {
            out.push_str(BREAK);
            at += BREAK.len();
        } else if letter.is_alphabetic() {
            let end = rest
                .find(|letter: char| !(letter.is_alphanumeric() || letter == '_' || letter == '\'' || letter == '-' || letter == '\\'))
                .unwrap_or(rest.len());
            let end = rest[..end].find('\\').unwrap_or(end);
            let word = &rest[..end];
            let upper = letter.is_uppercase();
            let before = line[..at].trim_end().chars().last();
            let begins = at == 0 || matches!(before, Some('.' | '!' | '?'));
            if film.tables.monsters.contains_key(&plain(word)) && upper {
                out.push_str(&format!("[CS:N]{word}[CR]"));
            } else if film.tables.items.contains_key(&plain(word)) && upper && !begins {
                out.push_str(&format!("[CS:I]{word}[CR]"));
            } else {
                out.push_str(word);
            }
            at += end;
        } else {
            out.push(letter);
            at += letter.len_utf8();
        }
    }
    out
}

/// How long the game's place name at the start of a text is, if there is one;
/// a name may be cut between two lines of a box.
fn place_at(text: &str, names: &[String]) -> Option<usize> {
    let first = text.chars().next()?;
    if !first.is_uppercase() {
        return None;
    }
    names.iter().find_map(|name| {
        let mut at = 0;
        for letter in name.chars() {
            let rest = &text[at..];
            if letter == ' ' && rest.starts_with(BREAK) {
                at += BREAK.len();
            } else if rest.starts_with(letter) {
                at += letter.len_utf8();
            } else {
                return None;
            }
        }
        Some(at)
    })
}
