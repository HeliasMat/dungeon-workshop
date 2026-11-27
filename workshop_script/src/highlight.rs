//! Each piece of a line in its colour: who speaks, the face, what is said,
//! the words of the language, the names, the numbers; what is lit over them.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ops::Range;

use eframe::egui::{self, Color32, FontId, Stroke};
use egui::text::{LayoutJob, TextFormat};

use super::text::{letter_of_word, spot_of};
use super::{COMMENT, FACE, KEYWORD, LABEL, LINK, NAME, NUMBER, PLAIN, SAID, SYMBOL};

/// What a piece of a line is.
#[derive(Clone, Copy, PartialEq)]
enum Piece {
    Plain,
    Comment,
    Label,
    Keyword,
    Link,
    Name,
    Face,
    Said,
    Number,
    Symbol,
}

fn colour(piece: Piece) -> (Color32, bool) {
    match piece {
        Piece::Plain => (PLAIN, false),
        Piece::Comment => (COMMENT, true),
        Piece::Label => (LABEL, false),
        Piece::Keyword => (KEYWORD, false),
        Piece::Link => (LINK, false),
        Piece::Name => (NAME, false),
        Piece::Face => (FACE, true),
        Piece::Said => (SAID, false),
        Piece::Number => (NUMBER, false),
        Piece::Symbol => (SYMBOL, false),
    }
}

/// The words followed by words of one's own, not of the language (`title_card The tournament…`).
const FREE: [&str; 6] = ["title_card", "log", "message", "say", "name", "title"];

/// The small words that link the others (`to`, `from`, `via`…).
const LINKS: [&str; 22] = [
    "to", "from", "via", "on", "at", "a", "as", "in", "times", "with", "against", "into", "front_of", "behind", "between", "left", "right", "up", "down",
    "far", "near", "end",
];

/// Whether a word is a number, a time or a point (`12`, `1.5s`, `6f`, `-30`, `18h30`, `+20`).
fn numeric(word: &str) -> bool {
    let body = word.trim_start_matches(['-', '+']);
    let body = body.strip_suffix(['s', 'f', '%']).unwrap_or(body);
    !body.is_empty()
        && (body.chars().all(|letter| letter.is_ascii_digit() || letter == '.')
            || body.split_once('h').is_some_and(|(h, m)| h.chars().all(|c| c.is_ascii_digit()) && m.chars().all(|c| c.is_ascii_digit())))
}

/// The names a film gives (who speaks, who is in its scenes, its own names).
pub(super) fn names_of(text: &str) -> HashSet<String> {
    let mut names: HashSet<String> = ["hero", "partner", "narrator", "all", "there", "team"].iter().map(|word| word.to_string()).collect();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if let Some(rest) = film::lines::scene_head(line) {
            names.extend(rest.split_whitespace().skip(1).map(str::to_string));
        } else if let Some((left, _)) = line.split_once(" = ") {
            names.insert(left.trim().to_string());
        } else if let Some((who, _)) = speaker(line) {
            names.insert(who.to_string());
        }
    }
    names
}

/// A line said: who says it, and where what is said begins (its byte).
pub(crate) fn speaker(line: &str) -> Option<(&str, usize)> {
    let lead = line.len() - line.trim_start().len();
    let rest = &line[lead..];
    let word: String = rest.chars().take_while(|letter| letter_of_word(*letter)).collect();
    if word.is_empty() {
        return None;
    }
    let mut at = lead + word.len();
    let after = line[at..].trim_start();
    at = line.len() - after.len();
    let after = if after.starts_with('(') {
        let close = after.find(')')?;
        at += close + 1;
        line[at..].trim_start()
    } else {
        after
    };
    at = line.len() - after.len();
    (after.starts_with(':') && !["title", "scene", "example", "moment", "transition"].contains(&word.as_str()))
        .then(|| (&line[lead..lead + word.len()], at + 1))
}

/// A line cut into its pieces (byte ranges), each what it is.
fn pieces(line: &str, text: &str, names: &HashSet<String>) -> Vec<(Range<usize>, Piece)> {
    let mut out: Vec<(Range<usize>, Piece)> = Vec::new();
    let code_end = line.find('#').unwrap_or(line.len());
    let code = &line[..code_end];
    let lead = code.len() - code.trim_start().len();
    if lead > 0 {
        out.push((0..lead, Piece::Plain));
    }
    // (Each word of a piece of the line, from that byte.)
    let words = |out: &mut Vec<(Range<usize>, Piece)>, from: usize, first_is_verb: bool| {
        let mut at = from;
        let mut first = first_is_verb;
        for part in code[from..].split_inclusive(char::is_whitespace) {
            let word = part.trim_end();
            let piece = if word.is_empty() {
                Piece::Plain
            } else if first && film::is_word(word, text) {
                Piece::Keyword
            } else if numeric(word.trim_end_matches(',')) {
                Piece::Number
            } else if ["&", ",", "=", "|", ":"].contains(&word) {
                Piece::Symbol
            } else if names.contains(word.trim_end_matches([',', ':'])) {
                Piece::Name
            } else if LINKS.contains(&word) {
                Piece::Link
            } else if film::is_word(word, text) {
                Piece::Keyword
            } else {
                Piece::Plain
            };
            if !word.is_empty() {
                first = false;
            }
            out.push((at..at + part.len(), piece));
            at += part.len();
        }
    };
    let trimmed = code.trim_start();
    if let Some(label) = ["title:", "scene:", "example:", "moment:", "transition:"].iter().find(|label| trimmed.starts_with(*label)) {
        out.push((lead..lead + label.len(), Piece::Label));
        if label == &"title:" || label == &"moment:" {
            out.push((lead + label.len()..code_end, Piece::Said));
        } else {
            words(&mut out, lead + label.len(), false);
        }
    } else if let Some((who, said_at)) = speaker(code) {
        let who_at = lead..lead + who.len();
        out.push((who_at.clone(), Piece::Name));
        out.push((who_at.end..said_at - 1, Piece::Face));
        out.push((said_at - 1..said_at, Piece::Symbol));
        // (What is said; `[1s]` and `*word*` as how it is said.)
        let said = &code[said_at..];
        let mut at = said_at;
        let mut inside: Option<char> = None;
        let mut start = at;
        for letter in said.chars() {
            let opens = inside.is_none() && (letter == '[' || letter == '*');
            let closes = inside.is_some_and(|open| (open == '[' && letter == ']') || (open == '*' && letter == '*' && at > start));
            if opens {
                if at > start {
                    out.push((start..at, Piece::Said));
                }
                (inside, start) = (Some(letter), at);
            } else if closes {
                out.push((start..at + letter.len_utf8(), Piece::Face));
                (inside, start) = (None, at + letter.len_utf8());
            }
            at += letter.len_utf8();
        }
        if code_end > start {
            out.push((start..code_end, if inside.is_some() { Piece::Face } else { Piece::Said }));
        }
    } else if let Some(word) = FREE.iter().find(|word| trimmed.split_whitespace().next() == Some(**word)) {
        out.push((lead..lead + word.len(), Piece::Keyword));
        out.push((lead + word.len()..code_end, Piece::Said));
    } else {
        words(&mut out, lead, true);
    }
    if code_end < line.len() {
        out.push((code_end..line.len(), Piece::Comment));
    }
    out
}

/// Lit pieces of the text, by line (from 1): their letters' columns (from 1), their colour.
pub(super) type Lights = BTreeMap<usize, Vec<(Range<usize>, Color32)>>;

/// The text in its colours, what is wrong underlined (by line: its letters'
/// columns, wrong or remarked), what is lit behind (what is found, the word
/// under the cursor).
pub(super) fn painted(
    text: &str,
    font: &FontId,
    wrap: f32,
    marks: &BTreeMap<usize, Vec<(Range<usize>, Color32)>>,
    lights: &Lights,
    hidden: &BTreeSet<usize>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let names = names_of(text);
    // (A line folded away: so small and clear that its row takes no room.)
    let tiny = TextFormat { font_id: FontId::monospace(0.5), color: Color32::TRANSPARENT, ..TextFormat::default() };
    for (at, line) in text.split('\n').enumerate() {
        if at > 0 {
            job.append("\n", 0.0, if hidden.contains(&at) { tiny.clone() } else { TextFormat::simple(font.clone(), PLAIN) });
        }
        if hidden.contains(&(at + 1)) {
            job.append(line, 0.0, tiny.clone());
            continue;
        }
        let marked = marks.get(&(at + 1));
        let lit = lights.get(&(at + 1));
        for (range, piece) in pieces(line, text, &names) {
            let (colour, italics) = colour(piece);
            let mut format = TextFormat { font_id: font.clone(), color: colour, italics, ..TextFormat::default() };
            // (Its columns, from 1: underlined where something is said of them.)
            let first = line[..range.start].chars().count() + 1;
            let last = first + line[range.clone()].chars().count();
            if let Some((_, shade)) = marked.into_iter().flatten().find(|(columns, _)| columns.start < last && first < columns.end.max(columns.start + 1)) {
                format.underline = Stroke::new(1.5_f32, *shade);
            }
            // (Cut where a light begins or ends: each part lit or not.)
            let mut cuts: Vec<usize> = vec![first, last];
            for (columns, _) in lit.into_iter().flatten() {
                cuts.extend([columns.start, columns.end].into_iter().filter(|cut| first < *cut && *cut < last));
            }
            cuts.sort_unstable();
            cuts.dedup();
            let piece_text: Vec<char> = line[range].chars().collect();
            for pair in cuts.windows(2) {
                let part: String = piece_text[pair[0] - first..pair[1] - first].iter().collect();
                let mut format = format.clone();
                if let Some((_, light)) = lit.into_iter().flatten().find(|(columns, _)| columns.start <= pair[0] && pair[1] <= columns.end) {
                    format.background = *light;
                }
                job.append(&part, 0.0, format);
            }
        }
    }
    job
}

/// Ranges of letters lit, by line and column.
pub(super) fn lights_of(text: &str, ranges: &[Range<usize>], colour: Color32, lights: &mut Lights) {
    for range in ranges {
        let (start, end) = (spot_of(text, range.start), spot_of(text, range.end));
        if start.line == end.line {
            lights.entry(start.line).or_default().push((start.column..end.column, colour));
        }
    }
}
