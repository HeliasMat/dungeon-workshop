//! Reading a command as a [`Beat`], by what it does: from its arguments,
//! typed (`Beat::build`, as a film binds them), or from the fields of a
//! line of fields as `story.txt` writes it (`Beat::parse`, each field a word).

use std::cell::Cell;
use std::sync::LazyLock;

use super::declare::{self, declare};
use super::*;
use diagnostics::text::{distance, nearest};

/// One argument of a command: as a film binds it, typed, or a field of a
/// line as it is written (a word, read as what its place asks for).
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    /// A number (a time: in frames).
    Number(f64),
    /// A number found as the scene plays, by its name (`Num::Var`).
    Found(String),
    /// Someone, or something, of the stage.
    Who(Who),
    /// A word: a name, a kind, `off`, `@`; any field of a written line.
    Word(String),
}

impl From<&str> for Arg {
    fn from(word: &str) -> Self {
        Arg::Word(word.to_string())
    }
}

/// Where a command did not read: the furthest argument there but not as
/// asked, and what was asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Miss {
    pub at: usize,
    pub asked: &'static str,
}

/// The arguments of a command (its word first), read as what they should
/// be; the furthest one that did not read as asked, kept to be told.
#[derive(Clone, Copy)]
pub(super) struct Fields<'a> {
    line: &'a [Arg],
    pub(super) words: &'static Vocabulary,
    /// Where these arguments start in the command (a beat inside another).
    base: usize,
    missed: &'a Cell<Option<Miss>>,
}

impl<'a> Fields<'a> {
    /// The arguments up to that one (not that one).
    pub(super) fn before(&self, end: usize) -> Fields<'a> {
        Fields { line: &self.line[..end.min(self.line.len())], ..*self }
    }

    /// The arguments from that one on, a command of their own.
    pub(super) fn from(&self, at: usize) -> Option<Fields<'a>> {
        Some(Fields { line: self.line.get(at..)?, base: self.base + at, ..*self })
    }

    /// What was asked of one there, if it does not give it: kept if it is
    /// the furthest yet.
    fn asked<T>(&self, at: usize, asked: &'static str, read: Option<T>) -> Option<T> {
        let place = self.base + at;
        if read.is_none() && self.has(at) && self.missed.get().is_none_or(|missed| place >= missed.at) {
            self.missed.set(Some(Miss { at: place, asked }));
        }
        read
    }

    pub(super) fn len(&self) -> usize {
        self.line.len()
    }

    pub(super) fn has(&self, at: usize) -> bool {
        at < self.line.len()
    }

    /// That word there (none for a number, someone).
    pub(super) fn word(&self, at: usize) -> Option<&'a str> {
        match self.line.get(at)? {
            Arg::Word(word) => Some(word),
            _ => None,
        }
    }

    /// The command's own word.
    pub(super) fn verb(&self) -> Option<&'a str> {
        self.word(0)
    }

    /// That word there, as asked for (`a label`).
    pub(super) fn named(&self, at: usize, asked: &'static str) -> Option<&'a str> {
        let word = self.word(at);
        self.asked(at, asked, word)
    }

    /// The words from there on (none if one is a number, someone).
    pub(super) fn words_from(&self, at: usize) -> Option<Vec<String>> {
        self.line.get(at..)?.iter().map(|arg| if let Arg::Word(word) = arg { Some(word.clone()) } else { None }).collect()
    }

    /// Each of them as a word: a number as it shows, one found as it plays
    /// `{name}`, someone by his code. For the commands whose values have a
    /// grammar of their own, in words (a node's: `props::read`).
    pub(super) fn spelled(&self) -> Vec<String> {
        let spelled = |arg: &Arg| match arg {
            Arg::Number(number) => number.to_string(),
            Arg::Found(name) => format!("{{{name}}}"),
            Arg::Who(who) => who.to_string(),
            Arg::Word(word) => word.clone(),
        };
        self.line.iter().map(spelled).collect()
    }

    /// Where that word is, from there on.
    pub(super) fn find(&self, from: usize, word: &str) -> Option<usize> {
        (from..self.line.len()).find(|at| self.is(*at, word))
    }

    /// A word of a table there, by what reads it.
    pub(super) fn lookup<T>(&self, at: usize, asked: &'static str, read: impl FnOnce(&str) -> Option<T>) -> Option<T> {
        let found = self.word(at).and_then(read);
        self.asked(at, asked, found)
    }

    pub(super) fn number(&self, at: usize) -> Option<u16> {
        let read = match self.line.get(at)? {
            Arg::Number(number) if number.fract() == 0.0 && (0.0..=f64::from(u16::MAX)).contains(number) => Some(*number as u16),
            Arg::Word(word) => word.parse().ok(),
            _ => None,
        };
        self.asked(at, "a whole number", read)
    }

    pub(super) fn real(&self, at: usize) -> Option<f32> {
        let read = match self.line.get(at)? {
            Arg::Number(number) => Some(*number as f32),
            Arg::Word(word) => word.parse().ok(),
            _ => None,
        };
        self.asked(at, "a number", read)
    }

    /// A whole number, which may be less than 0.
    pub(super) fn whole(&self, at: usize) -> Option<i32> {
        let read = match self.line.get(at)? {
            Arg::Number(number) if number.fract() == 0.0 => Some(*number as i32),
            Arg::Word(word) => word.parse().ok(),
            _ => None,
        };
        self.asked(at, "a whole number", read)
    }

    /// A time: seconds (`1.5s`), frames (`6f`, or `6`).
    pub(super) fn frames(&self, at: usize) -> Option<u32> {
        let read = match self.line.get(at)? {
            Arg::Number(number) if number.fract() == 0.0 && *number >= 0.0 => Some(*number as u32),
            Arg::Word(word) => frames_of(word),
            _ => None,
        };
        self.asked(at, "a duration", read)
    }

    /// A time, or one drawn between two (`1s~3s`): its least, and how much
    /// more it may be.
    pub(super) fn frames_between(&self, at: usize) -> Option<(u32, u32)> {
        match self.word(at).and_then(|word| word.split_once('~')) {
            Some((from, to)) => {
                let read = frames_of(from).zip(frames_of(to));
                self.asked(at, "a duration", read).map(|(from, to)| (from, to.saturating_sub(from)))
            }
            None => self.frames(at).map(|frames| (frames, 0)),
        }
    }

    /// A number, or one found as the scene plays (`{name}`).
    pub(super) fn num(&self, at: usize) -> Option<Num> {
        let read = match self.line.get(at)? {
            Arg::Number(number) => Some(Num::Lit(*number as f32)),
            Arg::Found(name) => Some(Num::Var(name.clone())),
            Arg::Word(word) => Num::read(word),
            Arg::Who(_) => None,
        };
        self.asked(at, "a number", read)
    }

    /// A point in two fields from there, each a number or one found as the
    /// scene plays.
    pub(super) fn point(&self, at: usize) -> Option<Point> {
        Some(Point { x: self.num(at)?, y: self.num(at + 1)? })
    }

    /// A number or a name of one (see `Value::parse`).
    pub(super) fn value(&self, at: usize) -> Option<Value> {
        Some(match self.line.get(at)? {
            Arg::Number(number) => Value::Number(*number),
            Arg::Found(name) => Value::Var(name.clone()),
            Arg::Who(who) => Value::Var(who.to_string()),
            Arg::Word(word) => Value::parse(word),
        })
    }

    /// That word there, as written.
    pub(super) fn is(&self, at: usize, word: &str) -> bool {
        self.word(at) == Some(word)
    }

    /// `1` there.
    pub(super) fn flag(&self, at: usize) -> bool {
        match self.line.get(at) {
            Some(Arg::Number(number)) => *number == 1.0,
            _ => self.is(at, "1"),
        }
    }

    /// `on` or `off` (`yes`/`no`, `1`/`0`).
    pub(super) fn on_off(&self, at: usize) -> Option<bool> {
        let read = match self.line.get(at)? {
            Arg::Number(number) if *number == 1.0 || *number == 0.0 => Some(*number == 1.0),
            Arg::Word(word) => on_off(word),
            _ => None,
        };
        self.asked(at, "on or off", read)
    }

    pub(super) fn who(&self, at: usize) -> Option<Who> {
        let read = match self.line.get(at)? {
            Arg::Who(who) => Some(*who),
            Arg::Word(word) => Who::parse(word),
            Arg::Number(number) if number.fract() == 0.0 && *number >= 0.0 => Who::parse(&(*number as u64).to_string()),
            _ => None,
        };
        self.asked(at, "someone", read)
    }

    /// Someone, then others, from a field on: at least one.
    pub(super) fn group(&self, from: usize) -> Option<Vec<Who>> {
        (from..self.line.len()).map(|at| self.who(at)).collect::<Option<Vec<Who>>>().filter(|group| !group.is_empty())
    }
}

impl Beat {
    /// A command read from its arguments, typed (its word first); where it
    /// does not read, if one of them is not as asked.
    pub fn build(line: &[Arg], words: &'static Vocabulary) -> Result<Self, Option<Miss>> {
        let missed = Cell::new(None);
        let fields = Fields { line, words, base: 0, missed: &missed };
        Beat::read(&fields).ok_or_else(|| missed.get())
    }

    /// A line of fields read (`story.txt`, a line filled as it plays), each
    /// field a word.
    pub fn parse(line: &[String], words: &'static Vocabulary) -> Result<Self, ParseError> {
        let word = line.first().ok_or(ParseError::Empty)?;
        let args: Vec<Arg> = line.iter().map(|field| Arg::Word(field.clone())).collect();
        Beat::build(&args, words).map_err(|_| ParseError::of(word, words))
    }

    /// A command read, by what it does.
    pub(super) fn read(fields: &Fields) -> Option<Self> {
        body::read(fields)
            .map(Beat::Body)
            .or_else(|| motion::read(fields).map(Beat::Motion))
            .or_else(|| view::read(fields).map(Beat::View))
            .or_else(|| screen::read(fields).map(Beat::Screen))
            .or_else(|| sound::read(fields).map(Beat::Sound))
            .or_else(|| speech::read(fields).map(Beat::Speech))
            .or_else(|| flow::read(fields).map(Beat::Flow))
            .or_else(|| data::read(fields).map(Beat::Data))
            .or_else(|| play::read(fields).map(Beat::Play))
            .or_else(|| place::read(fields).map(Beat::Place))
            .or_else(|| node::read(fields).map(Beat::Node))
            .or_else(|| host(fields))
            .or_else(|| course(fields).map(Beat::Course))
    }
}

fn course(fields: &Fields) -> Option<Course> {
    Some(match fields.verb()? {
        "endask" => Course::Rejoin,
        // (What a scene a game makes of a place asks it: `game talk kecleon`.)
        "game" => Course::Game(GameAsk::of_words(&fields.words_from(1)?)?),
        // (`<who>:<name>`: the name shown instead of its own.)
        "say" => {
            let said = fields.word(1)?;
            let (code, name) = match said.split_once(':') {
                Some((code, name)) => (code.to_string(), Some(name.to_string())),
                None => (said.to_string(), None),
            };
            let face = fields.number(2).or_else(|| fields.words.face(fields.word(2)?));
            let text = fields.word(3).unwrap_or_default().replace("\\n", "\n");
            Course::Say {
                entity: speaker(Some(&code)),
                face,
                text,
                name,
                timed: match fields.has(4) {
                    true => Some(Frames(fields.real(4)?)),
                    false => None,
                },
            }
        }
        "ask" => Course::Ask(fields.word(1)?.split('|').map(str::to_string).collect()),
        "answer" => Course::Answer(fields.number(1)?),
        "end" => Course::End,
        _ => return None,
    })
}

/// A line only the game can play, as it is written (its words).
fn host(fields: &Fields) -> Option<Beat> {
    let word = fields.verb()?;
    match () {
        _ if fields.words.host_now.contains(&word) => Some(Beat::Host(fields.words_from(0)?)),
        _ if fields.words.host_course.contains(&word) => Some(Beat::Course(Course::Host(fields.words_from(0)?))),
        _ => None,
    }
}

impl ParseError {
    /// Why a line starting with that word could not be read.
    fn of(word: &str, words: &Vocabulary) -> Self {
        let known = COMMANDS.contains(&word) || words.host_line(word);
        if known {
            return ParseError::Malformed { word: word.to_string() };
        }
        let mut names: Vec<&'static str> = COMMANDS.to_vec();
        names.sort_unstable();
        let close = nearest(names, 1, |name| Some(distance(word, name)).filter(|far| *far <= 2).map(|far| -(far as f64))).first().copied();
        ParseError::Unknown { word: word.to_string(), close }
    }
}

/// The engine's commands, by their own names.
pub fn commands() -> &'static [&'static str] {
    &COMMANDS
}

/// Every command of the engine, by its own names (each kind's `WORDS`).
pub(super) static COMMANDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut all: Vec<&'static str> = declare::every_kind().flat_map(|command| std::iter::once(command.own).chain(command.also.iter().copied())).collect();
    all.sort_unstable();
    all.dedup();
    all
});

// (The story's course, read here; `timed`, read by the language.)
declare! {
    Course;
    hand: "answer", "ask", "end", "endask", "game", "say", "timed";
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("answer", &["[text...]"], ""),
    Form::new("ask", &["[text...]"], ""),
    Form::new("end", &[""], "end of the scene (and of the answers of a choice)"),
    Form::new(
        "endask",
        &[""],
        "in place of a choice's `end`: each answer carries on **after it, in the same scene** (instead of going to the next scene): a question in a loop",
    ),
    Form::new(
        "game",
        &["talk who:code rest..."],
        "(written by the game in the scenes it makes of its places) asks the game, typed as the line is read (`GameAsk`, handed over by `Call::Game`): a talk and the menu hold the scene until the game is done",
    ),
    Form::new("say", &["who:code face:word text..."], "the line, and further on the wait until it is read (the scene goes on between the two)"),
    Form::new("timed", &["frames:duration"], "the previous line goes by on its own"),
];

/// Whether a word is a command of the engine, or of the game it plays for.
pub fn is_command(word: &str, words: &Vocabulary) -> bool {
    const SHORT: &[&str] = &[
        "bolt",
        "boo",
        "case",
        "cast",
        "chat",
        "cheer",
        "crossto",
        "default",
        "define",
        "do",
        "each",
        "enddef",
        "endonce",
        "endrepeat",
        "endswitch",
        "event",
        "group",
        "include",
        "line",
        "list",
        "narrate",
        "once",
        "oneof",
        "otherwise",
        "question",
        "when",
        "repeat",
        "spot",
        "switch",
    ];
    COMMANDS.contains(&word) || SHORT.contains(&word) || words.host_line(word)
}
