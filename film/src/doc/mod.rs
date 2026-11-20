//! A film's text as it is written, nothing of it lost: its lines, each with
//! its own ending, read back to the same bytes; what each line is and how
//! they hold together (`outline`); and the one way to change it, an `Edit`,
//! which gives back the edit that undoes it. Lines are numbered from 1, as
//! everywhere else (`line 42`), blank lines and comments counted.
//!
//! What a line is, the text's own readers say (`crate::text`): this only
//! keeps the text whole for who edits it. A typed change, "the `x` of line
//! 42 is 312", is an `Edit::Words` at the place the line's form gave that
//! name (`grammar::Value::at`).

mod outline;

pub use outline::{Kind, LineNode, MomentNode, Outline, SceneNode, kind_of};

/// A film's text, as written.
#[derive(Clone, Debug, PartialEq)]
pub struct Doc {
    /// Each line with its ending (`\n`, `\r\n`; the last one may have none).
    lines: Vec<String>,
    /// How many edits it has had: what is worked out from it says which.
    revision: u64,
    /// How its lines end (`\n`, `\r\n`), as it was read: the lines it is
    /// given end so.
    ending: &'static str,
}

/// A change to the text. Line numbers are from 1; words are counted after a
/// line's first one, as a form counts them.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// That line's text, all of it.
    Line { number: usize, text: String },
    /// A new line, put there: the one that was there comes after it.
    Insert { number: usize, text: String },
    /// That line gone.
    Remove { number: usize },
    /// Of that line's words after its first, `long` of them from `at`, made
    /// these (none: they go; `long` 0: these come in there).
    Words { number: usize, at: usize, long: usize, with: Vec<String> },
    /// So many lines from that one made these (a scene moved, in one edit).
    Lines { number: usize, long: usize, with: Vec<String> },
    /// So many lines from that one, endings and all, made this text, cut in
    /// lines as it is read: what typing in the text does (`Doc::edit_to`).
    Text { number: usize, long: usize, with: String },
}

/// A word of a line, and where it is in it, in bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

impl Default for Doc {
    /// An empty text, its lines to end as most do.
    fn default() -> Doc {
        Doc::read("")
    }
}

impl Doc {
    pub fn read(text: &str) -> Doc {
        let ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
        Doc { lines: text.split_inclusive('\n').map(str::to_string).collect(), revision: 0, ending }
    }

    /// The text, to the byte as it was read and edited.
    pub fn text(&self) -> String {
        self.lines.concat()
    }

    /// The text as it is typed: its lines ending in `\n` alone.
    pub fn typed(&self) -> String {
        let text = self.text();
        if self.ending == "\r\n" { text.replace("\r\n", "\n") } else { text }
    }

    /// The edit that makes the text the one typed (its lines ending in `\n`,
    /// written with the text's own endings): the lines that changed, those
    /// before and after them left as they are; none if it is the same.
    pub fn edit_to(&self, typed: &str) -> Option<Edit> {
        let typed = if self.ending == "\r\n" { typed.replace('\n', "\r\n") } else { typed.to_string() };
        let new: Vec<&str> = typed.split_inclusive('\n').collect();
        let old = &self.lines;
        let before = old.iter().zip(&new).take_while(|(old, new)| old == *new).count();
        let room = old.len().min(new.len()) - before;
        let after = old.iter().rev().zip(new.iter().rev()).take(room).take_while(|(old, new)| old == *new).count();
        if before == old.len() && before == new.len() {
            return None;
        }
        Some(Edit::Text { number: before + 1, long: old.len() - before - after, with: new[before..new.len() - after].concat() })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// How many lines it has.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// That line's text, without its ending.
    pub fn line(&self, number: usize) -> Option<&str> {
        let line = self.lines.get(number.checked_sub(1)?)?;
        Some(line.strip_suffix('\n').map_or(line.as_str(), |line| line.strip_suffix('\r').unwrap_or(line)))
    }

    /// Every line, numbered, without its ending.
    pub fn numbered(&self) -> impl Iterator<Item = (usize, &str)> {
        (1..=self.lines.len()).filter_map(|number| Some((number, self.line(number)?)))
    }

    /// The words of a line, and where each one is.
    pub fn tokens(&self, number: usize) -> Vec<Token> {
        let Some(line) = self.line(number) else {
            return Vec::new();
        };
        let mut tokens = Vec::new();
        let mut start = None;
        for (at, letter) in line.char_indices().chain(std::iter::once((line.len(), ' '))) {
            match (letter.is_whitespace(), start) {
                (false, None) => start = Some(at),
                (true, Some(from)) => {
                    tokens.push(Token { text: line[from..at].to_string(), start: from, end: at });
                    start = None;
                }
                _ => {}
            }
        }
        tokens
    }

    /// What it is, and how its lines hold together.
    pub fn outline(&self) -> Outline {
        Outline::of(self)
    }

    /// The edit made, and the one that undoes it; none if it cannot be made
    /// (a line that is not there), told why.
    pub fn apply(&mut self, edit: &Edit) -> Result<Edit, String> {
        let undo = match edit {
            Edit::Line { number, text } => {
                let was = self.line(*number).ok_or_else(|| self.no_line(*number))?.to_string();
                self.set(*number, text);
                Edit::Line { number: *number, text: was }
            }
            Edit::Insert { number, text } => {
                if *number == 0 || *number > self.lines.len() + 1 {
                    return Err(self.no_line(*number));
                }
                let (ending, count) = (self.ending, self.lines.len());
                match self.lines.last_mut() {
                    // (After a last line with no ending: it is given one, and
                    // the new last line has none, as the text had.)
                    Some(last) if *number == count + 1 && !last.ends_with('\n') => {
                        last.push_str(ending);
                        self.lines.push(text.clone());
                    }
                    _ => self.lines.insert(number - 1, format!("{text}{ending}")),
                }
                Edit::Remove { number: *number }
            }
            Edit::Remove { number } => {
                let was = self.line(*number).ok_or_else(|| self.no_line(*number))?.to_string();
                let last_unended = *number == self.lines.len() && !self.lines[number - 1].ends_with('\n');
                self.lines.remove(number - 1);
                // (The last line gone that had no ending: the one before loses its own.)
                if let Some(last) = self.lines.last_mut().filter(|_| last_unended) {
                    let kept = last.trim_end_matches('\n').trim_end_matches('\r').len();
                    last.truncate(kept);
                }
                Edit::Insert { number: *number, text: was }
            }
            Edit::Lines { number, long, with } => {
                let (from, to) = (number.wrapping_sub(1), number.wrapping_sub(1).wrapping_add(*long));
                if *number == 0 || to > self.lines.len() {
                    return Err(format!("no lines {number} to {}: the film has {}", number + long - 1, self.lines.len()));
                }
                let was: Vec<String> = (*number..number + long).filter_map(|at| self.line(at).map(str::to_string)).collect();
                // (Each line with the ending the text has; the last one's kept.)
                let ending = self.ending;
                let last = if to == self.lines.len() { self.lines.last().filter(|last| !last.ends_with('\n')).map(|_| "") } else { None };
                let mut made: Vec<String> = with.iter().map(|text| format!("{text}{ending}")).collect();
                if let (Some(unended), Some(made_last)) = (last, made.last_mut()) {
                    made_last.truncate(made_last.len() - ending.len());
                    made_last.push_str(unended);
                }
                self.lines.splice(from..to, made);
                Edit::Lines { number: *number, long: with.len(), with: was }
            }
            Edit::Text { number, long, with } => {
                let (from, to) = (number.wrapping_sub(1), number.wrapping_sub(1).wrapping_add(*long));
                if *number == 0 || to > self.lines.len() {
                    return Err(format!("no lines {number} to {}: the film has {}", number + long - 1, self.lines.len()));
                }
                let was = self.lines[from..to].concat();
                let made: Vec<String> = with.split_inclusive('\n').map(str::to_string).collect();
                let long = made.len();
                self.lines.splice(from..to, made);
                Edit::Text { number: *number, long, with: was }
            }
            Edit::Words { number, at, long, with } => {
                let tokens = self.tokens(*number);
                let line = self.line(*number).ok_or_else(|| self.no_line(*number))?.to_string();
                // (Words after the first: the first one is the line's word.)
                let (from, to) = (at + 1, at + 1 + long);
                if to > tokens.len() || from > tokens.len() {
                    return Err(format!("line {number}: no word {} to {}", from, to));
                }
                let said = with.join(" ");
                let text = match (long, with.is_empty()) {
                    // (In between two words: after the one before, a space.)
                    (0, _) => {
                        let after = tokens[from - 1].end;
                        format!("{} {said}{}", &line[..after], &line[after..])
                    }
                    // (Gone, with the space before them.)
                    (_, true) => format!("{}{}", &line[..tokens[from - 1].end], &line[tokens[to - 1].end..]),
                    _ => format!("{}{said}{}", &line[..tokens[from].start], &line[tokens[to - 1].end..]),
                };
                self.set(*number, &text);
                // (Undone as the whole line it was: its spaces as they were.)
                Edit::Line { number: *number, text: line }
            }
        };
        self.revision += 1;
        Ok(undo)
    }

    /// A line's text made that, its ending kept.
    fn set(&mut self, number: usize, text: &str) {
        let line = &mut self.lines[number - 1];
        let ending = if line.ends_with('\n') { ending_of(line) } else { "" };
        *line = format!("{text}{ending}");
    }

    fn no_line(&self, number: usize) -> String {
        format!("no line {number}: the film has {}", self.lines.len())
    }
}

/// The ending a line has, or the one the text has: `\r\n` or `\n`.
fn ending_of(line: &str) -> &'static str {
    if line.ends_with("\r\n") { "\r\n" } else { "\n" }
}
