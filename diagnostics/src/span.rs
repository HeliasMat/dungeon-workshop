//! Where a piece of text was written: its bytes, and its line and column for
//! people.

/// A piece of the text, from its first byte to the one after its last.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// From the start of the one to the end of the other.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }

    /// The text it covers.
    pub fn of(self, text: &str) -> &str {
        text.get(self.start..self.end).unwrap_or("")
    }

    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

/// A line and a column, both from 1; the column counts letters, not bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A text, and where each of its lines starts.
pub struct Source<'a> {
    pub text: &'a str,
    starts: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.bytes().enumerate().filter(|(_, byte)| *byte == b'\n').map(|(at, _)| at + 1));
        Self { text, starts }
    }

    /// The line and column of that byte.
    pub fn position(&self, byte: usize) -> Position {
        let byte = byte.min(self.text.len());
        let line = self.starts.partition_point(|start| *start <= byte) - 1;
        let start = self.starts[line];
        let column = self.text.get(start..byte).map_or(1, |before| before.chars().count() + 1);
        Position { line: line + 1, column }
    }

    /// The byte of that line and column (the end of the line if it is shorter).
    pub fn byte(&self, at: Position) -> usize {
        let Some(start) = self.starts.get(at.line.saturating_sub(1)).copied() else {
            return self.text.len();
        };
        let line = self.line_text(at.line);
        start + line.char_indices().nth(at.column.saturating_sub(1)).map_or(line.len(), |(byte, _)| byte)
    }

    /// The text of that line (from 1), without its end.
    pub fn line_text(&self, line: usize) -> &'a str {
        let Some(start) = self.starts.get(line.saturating_sub(1)).copied() else {
            return "";
        };
        let end = self.starts.get(line).map_or(self.text.len(), |next| next - 1);
        self.text[start..end.max(start)].trim_end_matches('\r')
    }

    pub fn lines(&self) -> usize {
        self.starts.len()
    }
}
