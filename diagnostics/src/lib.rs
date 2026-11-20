//! What is wrong, said one way by every block: its code, how much it
//! matters, where it is (a file, a line, columns, the bytes), what is said,
//! notes, and its fixes as edits of the text.
//!
//! - `span`: where a piece of a text was written (bytes, lines, columns);
//! - `text`: words as they are compared (no accents, the nearest names).
//!
//! Each block owns its codes (`language::diag::codes`, `engine::codes`,
//! `film::codes`, …): this one gives none but the shape.

#![forbid(unsafe_code)]

pub mod span;
pub mod text;

use std::fmt;

pub use span::{Position, Source, Span};

/// How much it matters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// It cannot play as written.
    #[default]
    Error,
    /// It plays, but most likely not as meant.
    Remark,
    /// Worth knowing.
    Detail,
}

impl Severity {
    pub fn word(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Remark => "remark",
            Severity::Detail => "detail",
        }
    }
}

/// Where something is: its file, its first letter and past its last (lines
/// and columns from 1), the bytes of the text it is about.
///
/// A line 0 is the file as a whole, a column 0 the whole line; the bytes,
/// when said, are what counts, the line and columns worked out from them
/// once the text is at hand ([`Location::resolved`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Location {
    /// The file (none: the text being read); for what was seen as a scene
    /// played, where it was (the scene, what it did there).
    pub file: Option<String>,
    pub start: Position,
    pub end: Position,
    pub bytes: Option<Span>,
}

impl Location {
    /// A line of the text read (0: the text as a whole).
    pub fn line(line: usize) -> Location {
        let at = Position { line, column: 0 };
        Location { file: None, start: at, end: at, bytes: None }
    }

    /// Those bytes of the text read.
    pub fn bytes(bytes: Span) -> Location {
        Location { bytes: Some(bytes), ..Location::default() }
    }

    /// A line of a file (0: the file as a whole).
    pub fn in_file(file: impl Into<String>, line: usize) -> Location {
        Location { file: Some(file.into()), ..Location::line(line) }
    }

    /// From one letter to past another.
    pub fn between(start: Position, end: Position) -> Location {
        Location { file: None, start, end, bytes: None }
    }

    /// What is wrong there.
    pub fn wrong(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic { span: self.clone(), ..Diagnostic::said(message) }
    }

    /// Its line and columns worked out from its bytes in that text (as it
    /// is when they are not said).
    pub fn resolved(&self, source: &Source) -> Location {
        match self.bytes {
            Some(bytes) => Location { start: source.position(bytes.start), end: source.position(bytes.end), ..self.clone() },
            None => self.clone(),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match (&self.file, self.start.line) {
            (Some(file), 0) => write!(out, "{file}"),
            (Some(file), line) => write!(out, "{file}:{line}"),
            (None, 0) => Ok(()),
            (None, line) => write!(out, "line {line}"),
        }
    }
}

/// Text put in the place of a span (an empty span inserts, an empty text
/// rubs out).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Edit {
    pub span: Location,
    pub text: String,
}

/// Someone a message names: a Pokémon of a scene, by its species and which
/// of them (0, the first; 2, the second other one), and where the message
/// writes it (bytes of the message). Typed, so that whoever reads the
/// message says it by the name it gives, and never reads it back from the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Mention {
    pub species: u16,
    pub copy: u16,
    pub at: Span,
}

/// A message being written, and those it names in it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Said {
    pub text: String,
    pub mentions: Vec<Mention>,
}

impl Said {
    /// Words added.
    pub fn words(mut self, words: &str) -> Self {
        self.text.push_str(words);
        self
    }

    /// Someone added, written so (`as_written`), named typed.
    pub fn naming(mut self, as_written: &str, species: u16, copy: u16) -> Self {
        let start = self.text.len();
        self.text.push_str(as_written);
        self.mentions.push(Mention { species, copy, at: Span::new(start, self.text.len()) });
        self
    }

    /// Its text, those it names said by `name` (none: as written).
    pub fn by(&self, name: &dyn Fn(&Mention) -> Option<String>) -> String {
        let (mut told, mut from) = (String::new(), 0);
        for mention in &self.mentions {
            let (Some(before), Some(written)) = (self.text.get(from..mention.at.start), self.text.get(mention.at.start..mention.at.end)) else { continue };
            told.push_str(before);
            told.push_str(&name(mention).unwrap_or_else(|| written.to_string()));
            from = mention.at.end;
        }
        told.push_str(self.text.get(from..).unwrap_or_default());
        told
    }
}

impl From<String> for Said {
    fn from(text: String) -> Self {
        Said { text, mentions: Vec::new() }
    }
}

impl From<&str> for Said {
    fn from(text: &str) -> Self {
        Said::from(text.to_string())
    }
}

/// A way to put it right: what it does in a few words, and its edits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fix {
    pub title: String,
    pub edits: Vec<Edit>,
}

/// What is wrong: where, why, how to put it right, as data. A fix is a list
/// of edits of the text, never found again by reading the message: an
/// editor or an agent applies it as it is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// Its stable code (the block's `codes`), the same whatever the message
    /// says; none: not given one yet.
    pub code: Option<&'static str>,
    pub severity: Severity,
    pub span: Location,
    pub message: String,
    pub notes: Vec<String>,
    /// Those its message names, typed: whoever reads it says them by its own
    /// names (a film, as its author wrote them).
    pub mentions: Vec<Mention>,
    pub fixes: Vec<Fix>,
}

impl Diagnostic {
    /// Wrong nowhere in particular (the file as a whole, a file missing).
    pub fn said(message: impl Into<String>) -> Self {
        Diagnostic { message: message.into(), ..Diagnostic::default() }
    }

    /// Wrong on that line of the text read.
    pub fn at(line: impl std::borrow::Borrow<usize>, message: impl Into<String>) -> Self {
        Location::line(*line.borrow()).wrong(message)
    }

    /// Wrong where a scene was when it played (its name, what it does there).
    pub fn played(place: impl Into<String>, message: impl Into<String>) -> Self {
        Location::in_file(place, 0).wrong(message)
    }

    /// With its code and weight, about those bytes of the text.
    pub fn new(code: &'static str, severity: Severity, span: Span, message: impl Into<String>) -> Self {
        Diagnostic { code: Some(code), severity, span: Location::bytes(span), ..Diagnostic::said(message) }
    }

    /// Wrong, with its code, about those bytes of the text.
    pub fn error(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Error, span, message)
    }

    /// The same, as something to look at rather than something wrong.
    pub fn as_remark(self) -> Self {
        Diagnostic { severity: Severity::Remark, ..self }
    }

    /// The same, of that weight.
    pub fn weighing(self, severity: Severity) -> Self {
        Diagnostic { severity, ..self }
    }

    /// The same, about those words of its line (bytes of the text; none:
    /// the whole line).
    pub fn on(mut self, bytes: Option<(usize, usize)>) -> Self {
        self.span.bytes = bytes.map(|(start, end)| Span::new(start, end));
        self
    }

    /// The same, with its stable code.
    pub fn coded(self, code: &'static str) -> Self {
        Diagnostic { code: Some(code), ..self }
    }

    /// The same, in that file.
    pub fn in_file(mut self, file: impl Into<String>) -> Self {
        self.span.file = Some(file.into());
        self
    }

    /// The same, its message and those it names as said.
    pub fn naming(self, said: Said) -> Self {
        Diagnostic { message: said.text, mentions: said.mentions, ..self }
    }

    /// Words put before its message, those it names moved along.
    pub fn led_by(mut self, words: &str) -> Self {
        self.message.insert_str(0, words);
        self.mentions.iter_mut().for_each(|mention| mention.at = Span::new(mention.at.start + words.len(), mention.at.end + words.len()));
        self
    }

    /// Its message, those it names said by `name` (none: as written).
    pub fn message_by(&self, name: &dyn Fn(&Mention) -> Option<String>) -> String {
        Said { text: self.message.clone(), mentions: self.mentions.clone() }.by(name)
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// With a fix of one edit, of those bytes.
    pub fn fix(mut self, title: impl Into<String>, span: Span, text: impl Into<String>) -> Self {
        self.fixes.push(Fix { title: title.into(), edits: vec![Edit { span: Location::bytes(span), text: text.into() }] });
        self
    }

    /// Met while reading there: said there, with where it was met if that
    /// is somewhere already (a piece's line in its own file).
    pub fn within(self, at: Location) -> Self {
        match self.span.to_string().is_empty() {
            true => Diagnostic { span: Location { bytes: self.span.bytes, ..at }, ..self },
            false => {
                // (Its message is at the end of what it says whole: those it names moved along.)
                let whole = self.to_string();
                let before = &whole[..whole.len() - self.message.len()];
                let led = Diagnostic { message: String::new(), ..self.clone() }.led_by(before);
                Diagnostic { severity: self.severity, code: self.code, mentions: led.mentions, ..at.wrong(whole) }
            }
        }
    }

    /// Whether its words say that.
    pub fn says(&self, words: &str) -> bool {
        self.message.contains(words)
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    pub fn is_remark(&self) -> bool {
        self.severity == Severity::Remark
    }

    /// Its line (0: none).
    pub fn line(&self) -> usize {
        self.span.start.line
    }

    /// The bytes it is about (none said: nothing, at the start).
    pub fn bytes(&self) -> Span {
        self.span.bytes.unwrap_or_default()
    }

    /// Its code, or that one when it has none of its own.
    pub fn code_or(&self, otherwise: &'static str) -> &'static str {
        self.code.unwrap_or(otherwise)
    }

    /// The same, its lines and columns worked out from its bytes in that
    /// text, its fixes' too.
    pub fn resolved(self, source: &Source) -> Self {
        let fixes = self
            .fixes
            .iter()
            .map(|fix| Fix {
                title: fix.title.clone(),
                edits: fix.edits.iter().map(|edit| Edit { span: edit.span.resolved(source), text: edit.text.clone() }).collect(),
            })
            .collect();
        Diagnostic { span: self.span.resolved(source), fixes, ..self }
    }

    /// As a compiler says it: `file:3:5: error E0005: …`, the line, a
    /// mark under the span, the notes and the fixes.
    pub fn shown(&self, source: &Source, file: &str) -> String {
        let span = self.bytes();
        let at = source.position(span.start);
        let end = source.position(span.end.max(span.start));
        let mut told = format!("{file}:{}:{}: {} {}: {}\n", at.line, at.column, self.severity.word(), self.code.unwrap_or_default(), self.message);
        let line = source.line_text(at.line);
        let number = at.line.to_string();
        let pad = " ".repeat(number.len());
        let width = if end.line == at.line { end.column.saturating_sub(at.column).max(1) } else { line.chars().count().saturating_sub(at.column - 1).max(1) };
        told += &format!("{number} | {line}\n{pad} | {}{}\n", " ".repeat(at.column - 1), "^".repeat(width));
        for note in &self.notes {
            told += &format!("{pad} = {note}\n");
        }
        for fix in &self.fixes {
            told += &format!("{pad} = fix: {}\n", fix.title);
        }
        told
    }
}

/// `remark: ` for a remark, then where (`line 3`, `file:3`, `file`) and
/// what: `remark: line 3: no`.
impl fmt::Display for Diagnostic {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        if self.is_remark() {
            write!(out, "remark: ")?;
        }
        match self.span.to_string() {
            at if at.is_empty() => write!(out, "{}", self.message),
            at => write!(out, "{at}: {}", self.message),
        }
    }
}

impl std::error::Error for Diagnostic {}

/// The text with those edits made (they must not overlap): by their bytes,
/// else by their lines and columns.
pub fn applied(text: &str, edits: &[Edit]) -> String {
    let source = Source::new(text);
    let mut bytes: Vec<(Span, &str)> = edits
        .iter()
        .map(|edit| (edit.span.bytes.unwrap_or_else(|| Span::new(source.byte(edit.span.start), source.byte(edit.span.end))), edit.text.as_str()))
        .collect();
    bytes.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut made = text.to_string();
    for (span, text) in bytes {
        let (start, end) = (span.start.min(made.len()), span.end.min(made.len()));
        made.replace_range(start..end.max(start), text);
    }
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_tell_where_they_are() {
        assert_eq!(Diagnostic::at(3, "no").to_string(), "line 3: no");
        assert_eq!(Diagnostic::at(3, "no").as_remark().to_string(), "remark: line 3: no");
        assert_eq!(Diagnostic::said("no").to_string(), "no");
        assert_eq!(Diagnostic::played("scene 2", "no").to_string(), "scene 2: no");
        assert_eq!(Location::in_file("a.decor", 4).wrong("no").to_string(), "a.decor:4: no");
        assert_eq!(Location::in_file("a.decor", 0).wrong("no").to_string(), "a.decor: no");
    }

    #[test]
    fn a_diagnostic_is_resolved_and_applied() {
        let text = "one\ntwo three\n";
        let told = Diagnostic::error("E1", Span::new(8, 13), "three?").fix("write four", Span::new(8, 13), "four").resolved(&Source::new(text));
        assert_eq!((told.span.start, told.span.end), (Position { line: 2, column: 5 }, Position { line: 2, column: 10 }));
        assert_eq!(applied(text, &told.fixes[0].edits), "one\ntwo four\n");
        let by_columns = Edit { span: Location::between(Position { line: 1, column: 1 }, Position { line: 1, column: 4 }), text: "1".to_string() };
        assert_eq!(applied(text, &[by_columns]), "1\ntwo three\n");
        assert!(told.shown(&Source::new(text), "f.film").starts_with("f.film:2:5: error E1: three?\n2 | two three\n  |     ^^^^^\n"));
    }

    #[test]
    fn those_a_message_names_are_said_by_the_readers_names_wherever_it_goes() {
        let said = Said::default().naming("species m25", 25, 0).words(" hides ").naming("species m25.2", 25, 2).words(".");
        let name = |mention: &Mention| (mention.copy == 0).then(|| "Pika".to_string());
        assert_eq!(said.by(&name), "Pika hides species m25.2.");
        let told = Diagnostic::at(3, "").naming(said).led_by("in a, ");
        assert_eq!(told.message_by(&name), "in a, Pika hides species m25.2.");
        let within = told.within(Location::in_file("f.film", 2));
        assert_eq!(within.message_by(&name), "line 3: in a, Pika hides species m25.2.");
    }
}
