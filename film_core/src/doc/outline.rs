//! How a film's lines hold together, as written: what comes before the
//! scenes, each scene (its place, those it names, how it opens), its moments,
//! their lines, and the lines a block holds up to its `end`. Whatever is
//! written, there is an outline: a block not closed runs to the end of its
//! scene, an `end` with nothing open is a line of its own.

use super::Doc;
use crate::words::plain;
use crate::words::{first_word, head, in_seconds, spoken};

/// What a line is.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Blank,
    Comment,
    /// `title: ...`
    Title,
    /// `transition: wipe`, for the scene that follows.
    Transition,
    /// `pinchy = krabby`
    Name,
    /// `scene: beach a b`
    Scene {
        place: String,
        cast: Vec<String>,
    },
    /// `moment: the dream`
    Moment {
        title: String,
    },
    /// `who (face): words`
    Said {
        who: String,
        face: Option<String>,
    },
    /// A line of a word: `walk a to b`.
    Word {
        word: String,
    },
    /// `> command`: for the engine, as it is.
    Raw,
    /// A line that holds others up to its `end` (`define`, `repeat`,
    /// `beside`, `together`, `fight directed`, `at 3s:`, `if a:`).
    Opens {
        word: String,
    },
    /// `end`
    Fin,
}

/// A line, and those it holds if it opens a block.
#[derive(Clone, Debug, PartialEq)]
pub struct LineNode {
    pub number: usize,
    pub kind: Kind,
    pub inner: Vec<LineNode>,
}

/// A moment of a scene: its title and line (none: the lines before the
/// scene's first moment), its lines.
#[derive(Clone, Debug, PartialEq)]
pub struct MomentNode {
    pub number: Option<usize>,
    pub title: String,
    pub lines: Vec<LineNode>,
}

/// A scene: its line, its place, those it names, the line that says how it
/// opens, its moments.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneNode {
    pub number: usize,
    pub place: String,
    pub cast: Vec<String>,
    pub transition: Option<usize>,
    pub moments: Vec<MomentNode>,
}

/// A film as written: what comes before its scenes, its scenes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outline {
    pub head: Vec<LineNode>,
    pub scenes: Vec<SceneNode>,
}

/// What a line is, by the text's own readers.
pub fn kind_of(line: &str) -> Kind {
    let trimmed = line.trim();
    let lower = trimmed.to_lowercase();
    if trimmed.is_empty() {
        return Kind::Blank;
    }
    if trimmed.starts_with('#') {
        return Kind::Comment;
    }
    if trimmed.starts_with('>') {
        return Kind::Raw;
    }
    if after_label(trimmed, &lower, &["title"]).is_some() {
        return Kind::Title;
    }
    if after_label(trimmed, &lower, &["transition"]).is_some() {
        return Kind::Transition;
    }
    if let Some(said) = after_label(trimmed, &lower, &["scene", "example"]) {
        let mut words = said.split_whitespace().map(|word| word.trim_end_matches(',').to_string());
        let place = words.next().unwrap_or_default();
        return Kind::Scene { place, cast: words.collect() };
    }
    if let Some(title) = after_label(trimmed, &lower, &["moment"]) {
        return Kind::Moment { title: title.trim().to_string() };
    }
    if opens(trimmed) {
        let word = when_of(trimmed).map_or_else(|| first_word(trimmed), |when| when.word);
        return Kind::Opens { word };
    }
    if first_word(trimmed) == "end" && head(trimmed).1.is_empty() {
        return Kind::Fin;
    }
    if name_given(trimmed).is_some() {
        return Kind::Name;
    }
    if let Some((who, face, _)) = spoken(trimmed) {
        return Kind::Said { who: plain(who), face: face.map(str::to_string) };
    }
    Kind::Word { word: first_word(trimmed) }
}

impl Outline {
    pub(super) fn of(doc: &Doc) -> Outline {
        let lines: Vec<(usize, Kind)> = doc.numbered().map(|(number, line)| (number, kind_of(line))).collect();
        let mut outline = Outline::default();
        let mut transition = None;
        let mut at = 0;
        while at < lines.len() {
            let (number, kind) = &lines[at];
            match kind {
                Kind::Scene { place, cast } => {
                    let scene =
                        SceneNode { number: *number, place: place.clone(), cast: cast.clone(), transition: transition.take(), moments: vec![first_moment()] };
                    outline.scenes.push(scene);
                    at += 1;
                }
                // (How the next scene opens: told with it, not in the one before.)
                Kind::Transition => {
                    transition = Some(*number);
                    outline.held_by_last(LineNode { number: *number, kind: kind.clone(), inner: Vec::new() });
                    at += 1;
                }
                Kind::Moment { title } if !outline.scenes.is_empty() => {
                    let moments = &mut outline.scenes.last_mut().expect("a scene").moments;
                    moments.push(MomentNode { number: Some(*number), title: title.clone(), lines: Vec::new() });
                    at += 1;
                }
                _ => {
                    let (node, after) = held(&lines, at);
                    outline.held_by_last(node);
                    at = after;
                }
            }
        }
        outline
    }

    /// A line kept where the text is at: in the last moment of the last
    /// scene, or before the scenes.
    fn held_by_last(&mut self, node: LineNode) {
        match self.scenes.last_mut().and_then(|scene| scene.moments.last_mut()) {
            Some(moment) => moment.lines.push(node),
            None => self.head.push(node),
        }
    }

    /// Every line of it, in order, those blocks hold too.
    pub fn every_line(&self) -> Vec<&LineNode> {
        fn all<'a>(nodes: &'a [LineNode], out: &mut Vec<&'a LineNode>) {
            for node in nodes {
                out.push(node);
                all(&node.inner, out);
            }
        }
        let mut out = Vec::new();
        all(&self.head, &mut out);
        for scene in &self.scenes {
            for moment in &scene.moments {
                all(&moment.lines, &mut out);
            }
        }
        out
    }
}

/// The lines of a scene before its first moment.
fn first_moment() -> MomentNode {
    MomentNode { number: None, title: String::new(), lines: Vec::new() }
}

/// The line there, with those it holds up to its `end` if it opens a block
/// (up to the next scene, if none closes it); where to go on after.
fn held(lines: &[(usize, Kind)], at: usize) -> (LineNode, usize) {
    let (number, kind) = &lines[at];
    let mut node = LineNode { number: *number, kind: kind.clone(), inner: Vec::new() };
    let mut next = at + 1;
    if !matches!(kind, Kind::Opens { .. }) {
        return (node, next);
    }
    while next < lines.len() {
        match &lines[next].1 {
            Kind::Scene { .. } => break,
            Kind::Fin => {
                node.inner.push(LineNode { number: lines[next].0, kind: Kind::Fin, inner: Vec::new() });
                return (node, next + 1);
            }
            _ => {
                let (inner, after) = held(lines, next);
                node.inner.push(inner);
                next = after;
            }
        }
    }
    (node, next)
}

/// What follows a `word:` at the start of a line, if it starts so.
fn after_label<'a>(line: &'a str, lower: &str, labels: &[&str]) -> Option<&'a str> {
    let said = labels.iter().any(|label| lower.strip_prefix(label).is_some_and(|rest| rest.trim_start().starts_with(':')));
    said.then(|| line.split_once(':').map_or("", |(_, rest)| rest))
}

/// `pinchy = krabby`: the two words.
fn name_given(line: &str) -> Option<(&str, &str)> {
    let letters = |word: &str| !word.is_empty() && word.chars().all(|letter| letter.is_alphanumeric() || letter == '_');
    let (name, kind) = line.split_once('=')?;
    let kind = kind.trim_start();
    let end = kind.find(|letter: char| !(letter.is_alphanumeric() || letter == '_')).unwrap_or(kind.len());
    (letters(name.trim_end()) && end > 0).then(|| (name.trim_end(), kind.trim_end()))
}

/// Whether a line opens what an `end` closes: `define`, `repeat`, `if a:`,
/// a `when` or an `at` with nothing after its colon, `beside`, `together`,
/// a fight directed, a line anchored.
fn opens(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    let kept_if = matches!(words.as_slice(), ["if", name] | ["if", "not", name] if name.ends_with(':'));
    let a_fight = |line: &str| {
        let (first, rest) = head(line);
        first.eq_ignore_ascii_case("fight") && head(rest).0.eq_ignore_ascii_case("directed")
    };
    let block = match when_of(line) {
        Some(when) if when.rest.is_empty() => true,
        Some(when) => when.word == "at" && a_fight(when.rest),
        None => a_fight(line) || matches!(first_word(line).as_str(), "beside" | "together") || opens_anchored(line),
    };
    matches!(first_word(line).as_str(), "define" | "repeat") || block || kept_if
}

/// When a line says it happens.
struct When<'a> {
    /// `at`, `during` or `after`: words of before, told how they are written now.
    word: String,
    /// What comes after the `:`.
    rest: &'a str,
}

/// `at 3s:`, `during 4s:`, `after a:` at the start of a line.
fn when_of(line: &str) -> Option<When<'_>> {
    let (word, rest) = head(line);
    let word = word.to_lowercase();
    let colon = rest.find(':')?;
    let value = rest[..colon].trim_end();
    let fits = match word.as_str() {
        "at" | "during" => in_seconds(value),
        "after" => !value.is_empty() && !value.contains(char::is_whitespace),
        _ => false,
    };
    fits.then(|| When { word, rest: rest[colon + 1..].trim_start() })
}

/// Whether a line opens a block of lines anchored to what happens: `when a
/// arrives` or `at name + 2s:` with nothing after.
fn opens_anchored(line: &str) -> bool {
    use crate::ir::{Instr, Shape};
    matches!(Instr::line(0, line).shape, Shape::Arrives { then: None, .. } | Shape::AtMark { then: None, .. })
}
