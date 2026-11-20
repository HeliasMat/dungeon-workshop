//! What is wrong in a film: each problem with its film line, and for a name
//! not known, the names nearest to it.

use engine::Diagnostic;
use language::text;

use crate::film::Film;
use crate::words::plain;

impl Film {
    /// Something the film cannot play as written: about the words it
    /// quotes, if they are words of the line at hand (`“krabbi”`).
    pub fn wrong(&mut self, number: usize, message: String) {
        let span = self.quoted(number, &message);
        self.problems.push(Diagnostic::at(number, message).on(span));
    }

    /// The same, with its stable code (`engine::codes`, `crate::codes`).
    pub fn wrong_coded(&mut self, number: usize, code: &'static str, message: String) {
        self.wrong(number, message);
        if let Some(problem) = self.problems.pop() {
            self.problems.push(problem.coded(code));
        }
    }

    /// Something that plays, but should be looked at.
    /// A remark with its stable code (see `engine::codes`, `crate::codes`).
    pub fn remark_coded(&mut self, number: usize, code: &'static str, message: String) {
        self.remark(number, message);
        if let Some(problem) = self.problems.pop() {
            self.problems.push(problem.coded(code));
        }
    }

    pub fn remark(&mut self, number: usize, message: String) {
        let span = self.quoted(number, &message);
        self.problems.push(Diagnostic::at(number, message).as_remark().on(span));
    }

    /// Where the first words a problem quotes (`“…”`) are in the film's
    /// text: those of the line at hand, one after another, as written.
    fn quoted(&self, number: usize, message: &str) -> Option<(usize, usize)> {
        let (at, words) = &self.here;
        if *at != number || words.is_empty() {
            return None;
        }
        let said = quote_in(message)?;
        let said: Vec<&str> = said.split_whitespace().collect();
        let first = (0..words.len())
            .find(|start| said.len() <= words.len() - start && said.iter().zip(&words[*start..]).all(|(one, (word, _))| plain(one) == plain(word)))?;
        let last = first + said.len().checked_sub(1)?;
        Some((words[first].1.start, words[last].1.end))
    }

    /// The words of a line as it is written in the film's own text, each
    /// where it is there (none for a line a recipe, a word or a file taken
    /// in made).
    pub fn written_words(&self, line: &crate::ir::Instr) -> Vec<(String, language::Span)> {
        let Some(span) = line.span else { return Vec::new() };
        let Some(written) = self.source.get(span.start..span.end) else { return Vec::new() };
        let mut words = Vec::new();
        let mut start = None;
        for (at, letter) in written.char_indices().chain(std::iter::once((written.len(), ' '))) {
            match (letter.is_whitespace(), start) {
                (false, None) => start = Some(at),
                (true, Some(from)) => {
                    words.push((written[from..at].to_string(), language::Span::new(span.start + from, span.start + at)));
                    start = None;
                }
                _ => {}
            }
        }
        words
    }

    /// How a line was read, told only when asked: neither wrong nor to look
    /// at (`check --all`).
    pub fn note(&mut self, number: usize, message: String) {
        self.notes.push((number, message));
    }

    /// A name not known (`what`: `unknown thing`), with the nearest of those
    /// known and where to look.
    pub fn unknown<'a>(&mut self, number: usize, what: &str, word: &str, known: impl Iterator<Item = &'a str>, look: &str) {
        let close = closest(&plain(word), known, NEAREST, CLOSE_ENOUGH);
        self.wrong(number, unknown(what, word, &close, look));
    }
}

/// How many of the nearest names a problem gives, and how near they must be.
const NEAREST: usize = 3;
const CLOSE_ENOUGH: f32 = 0.5;

/// The first words a message quotes, between `“` and `”` (or `"`, `«` `»`).
fn quote_in(message: &str) -> Option<&str> {
    let (open, close) =
        [('“', '”'), ('"', '"'), ('«', '»')].into_iter().filter_map(|(open, close)| Some((message.find(open)?, (open, close)))).min_by_key(|(at, _)| *at)?.1;
    let (_, after) = message.split_once(open)?;
    let (said, _) = after.split_once(close)?;
    Some(said.trim())
}

/// What is said of a name not known, all alike: `unknown thing “pome”.
/// Near: apple, pear (scene_player items)`.
pub fn unknown(what: &str, word: &str, close: &[String], look: &str) -> String {
    let close = if close.is_empty() { String::new() } else { format!(". Near: {}", close.join(", ")) };
    let look = if look.is_empty() { String::new() } else { format!(" ({look})") };
    format!("{what} “{word}”{close}{look}")
}

/// The names nearest to a word, as a problem offers them.
pub fn closest_of<'a>(word: &str, names: impl Iterator<Item = &'a str>) -> Vec<String> {
    closest(&plain(word), names, NEAREST, CLOSE_ENOUGH)
}

/// What a problem of a name not known offers in its place (`unknown` says it
/// so): the word said, and the names near it; none for another problem.
pub fn offered(problem: &str) -> Option<(String, Vec<String>)> {
    let (_, after) = problem.split_once('“')?;
    let (word, rest) = after.split_once('”')?;
    let near = rest.strip_prefix(". Near: ")?;
    let near = near.split(" (").next().unwrap_or(near);
    let near: Vec<String> = near.split(", ").map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect();
    (!near.is_empty()).then(|| (word.to_string(), near))
}

/// The words nearest to one (`language::text::similarity`), the nearest
/// first, the equally near by their names.
pub fn closest<'a>(word: &str, names: impl Iterator<Item = &'a str>, most: usize, least: f32) -> Vec<String> {
    let mut names: Vec<&str> = names.collect();
    names.sort_unstable();
    let near = |name: &str| Some(text::similarity(word, name)).filter(|near| *near >= f64::from(least));
    text::nearest(names, most, near).into_iter().map(str::to_string).collect()
}

/// What a problem adds of the names nearest to a word.
pub fn nearby<'a>(word: &str, names: impl Iterator<Item = &'a str>) -> String {
    let close = closest(&plain(word), names, NEAREST, CLOSE_ENOUGH);
    if close.is_empty() { String::new() } else { format!(". Near: {}", close.join(", ")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_not_known_offers_those_near_it() {
        let said = unknown("unknown thing", "aple", &["apple".to_string(), "ape".to_string()], "scene_player items");
        assert_eq!(offered(&said), Some(("aple".to_string(), vec!["apple".to_string(), "ape".to_string()])));
        assert_eq!(offered(&unknown("unknown word", "zap", &[], "")), None, "nothing near");
        assert_eq!(offered("line 3: piplup cannot go to (412, 302)"), None);
    }
}
