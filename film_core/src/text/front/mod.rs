//! The one reader of a film's text: the language (`language`, `LANGUAGE.md`)
//! read once into its tree, then written as the lines the words play, each
//! with the number of the line of the text it comes from.
//!
//! What the words know is written as they read it. What the language adds
//! is made as the engine's beats, typed (`Shape::Beat`: its tests a
//! `Test`, its numbers `Value`s), its text kept for the tools (`>`); only
//! a line naming someone of the film is left as fields (`position`,
//! `measure`, `waitmoves`), his code put in by the film:
//! - `let`, `keep`, `x = …`, the calculations: `set`, `calc`, `random`,
//!   `position`, `measure`, into numbers of the event (`_x`), kept ones
//!   (`keep`) or a recipe call's own (`$x`);
//! - `if`, `while`, `repeat n` (n played), `break`, `next`: `if`,
//!   `while`, `break`, `continue`;
//! - `when <test>`, `every`, `after`: a block beside the film that waits;
//! - `choice` / `option`, `when health(c) <= 50` in a fight, `for each`:
//!   the words that play them.
//!
//! A recipe stays as the language wrote it (`define`: `RecipeText`), and
//! is written at each call with what that call gives it (`called`): its
//! names put in, its reckonings worked out, `if name` and `for each x
//! in *who` played out, its own numbers that call's (`_x_<call>`), what
//! it gives back `_return_<name>`.

use std::collections::HashMap;
use std::sync::Arc;

use language::ast::{Arg, ArgKind, BinOp, Block, Element, Event, Expr, ExprKind, File, Kind, Opening, PartKind, Piece, Stmt, UnOp};
use language::diag::codes;
use language::format::{elements_text, expression};
use language::line::number_text;
use language::{Diagnostic, Source, Span};

use engine::{Beat, Compare, Condition, DataBeat, FlowBeat, Frames, Num, Test, Value};

use crate::ir::{From, Instr, RecipeLine, Shape};
use crate::verbs::Lexicon;

mod blocks;
mod helpers;
mod said;
mod values;

pub use helpers::*;

/// A text read: its lines as the words play them (each block holding its
/// own), what is wrong.
pub struct Internal {
    pub lines: Vec<Instr>,
    pub diagnostics: Vec<(usize, Diagnostic)>,
}

/// A recipe as the language wrote it (`define`): its form, its lines, the
/// text they are written in and where; what it does (the comment above
/// it), its line.
#[derive(Debug)]
pub struct RecipeText {
    pub form: language::ast::Form,
    pub body: Vec<Stmt>,
    pub text: Arc<str>,
    pub span: Span,
    pub doc: String,
    pub number: usize,
}

impl PartialEq for RecipeText {
    fn eq(&self, other: &Self) -> bool {
        self.form == other.form && self.span == other.span && Arc::ptr_eq(&self.text, &other.text)
    }
}

/// One call of a recipe: its number for the names of the event it makes,
/// the line that calls it, what each of its names was given, where the
/// recipe is written.
pub struct CallOf {
    pub id: usize,
    pub number: usize,
    pub values: Vec<CallValue>,
    /// The recipe's file, as told (none: the film's own).
    pub file: String,
    /// Written apart, to be checked (`recipes::checked`): each line numbered
    /// by its own line in the recipe's text, not by the line that calls it.
    pub apart: bool,
}

/// What a name of a recipe was given: its words, and as the game writes
/// them (someone by his name, `Eevee`; a team, `Eevee and Pikachu`).
pub struct CallValue {
    pub name: String,
    pub words: Vec<String>,
    pub shown: String,
}

/// A number a recipe's reckoning reads: one worked out now (a time as
/// frames, `true`), or the engine's name for one worked out as it plays.
#[derive(Clone)]
enum Operand {
    Number(f64, bool),
    Var(String),
}

impl Operand {
    fn engine(&self) -> String {
        match self {
            Operand::Number(number, _) => (number.round() as i64).to_string(),
            Operand::Var(var) => var.clone(),
        }
    }
}

/// A number as the language writes it for the engine: a figure, or the
/// name of one the engine keeps.
fn operand(field: &str) -> Value {
    field.parse().map_or_else(|_| Value::Var(field.to_string()), Value::Number)
}

/// A test of one comparison: `left compare right`.
fn compared(left: &str, compare: Compare, right: &str) -> Test {
    Test { any: vec![vec![Condition { var: left.to_string(), compare, value: operand(right) }]] }
}

/// What the language writes, in order: a line, a block opened (its line),
/// the block closed; each line with what it is, its text kept for the tools.
enum Out {
    Line(At, String, Shape),
    Open(At, String, Shape),
    Close,
}

/// Where a line written is told: its number, and its own line in the text
/// it was written from (a recipe's line, for a call).
#[derive(Clone, Copy)]
struct At {
    number: usize,
    own: usize,
}

/// The lines written, each block holding its lines (`ir::Instr`); a
/// comment, an empty line: lines of nothing, kept (a recipe's doc).
/// A recipe's lines (`recipe`: its name and its file as told) each know the
/// recipe line they were written from.
fn tree(out: Vec<Out>, recipe: Option<(&str, &str)>) -> Vec<Instr> {
    let from = |at: At| From(recipe.map(|(name, file)| Arc::new(RecipeLine { recipe: name.to_string(), file: file.to_string(), line: at.own })));
    let mut open: Vec<(At, String, Shape, Vec<Instr>)> = vec![(At { number: 0, own: 0 }, String::new(), Shape::Blank, Vec::new())];
    let closed = |open: &mut Vec<(At, String, Shape, Vec<Instr>)>| {
        if let Some((at, text, shape, inner)) = open.pop()
            && let Some(parent) = open.last_mut()
        {
            parent.3.push(Instr { number: at.number, text, shape, inner: Some(inner), span: None, from: from(at) });
        }
    };
    for piece in out {
        match piece {
            Out::Line(at, text, shape) => {
                if let Some(parent) = open.last_mut() {
                    parent.3.push(Instr { number: at.number, text, shape, inner: None, span: None, from: from(at) });
                }
            }
            Out::Open(at, head, made) => open.push((at, head, made, Vec::new())),
            Out::Close if open.len() > 1 => closed(&mut open),
            Out::Close => {}
        }
    }
    while open.len() > 1 {
        closed(&mut open);
    }
    open.pop().map(|(_, _, _, lines)| lines).unwrap_or_default()
}

/// The lines of what was written that play (not empty, not a comment), and
/// the blocks opened: what a short block counts.
fn played(body: &[Out]) -> Vec<(usize, &str)> {
    body.iter()
        .filter_map(|piece| match piece {
            Out::Line(at, line, _) if !line.is_empty() && !line.starts_with('#') => Some((at.number, line.as_str())),
            Out::Open(at, head, _) => Some((at.number, head.as_str())),
            _ => None,
        })
        .collect()
}

/// What a line is anchored to, as the language says it.
enum Anchor {
    /// `when a b arrives`.
    Arrives(Vec<String>),
    /// `at name + 2s`: so many frames after the mark.
    Mark(String, i32),
}

impl Anchor {
    fn shape(self, then: Option<Box<Instr>>) -> Shape {
        match self {
            Anchor::Arrives(who) => Shape::Arrives { who, then },
            Anchor::Mark(name, after) => Shape::AtMark { name, after, then },
        }
    }
}

/// The numbers the engine fills each frame, read as they are.
const THE_GAMES: [&str; 9] = ["clock", "music_clock", "answer", "timer", "controlled", "target", "stopwatch", "hp", "turns"];

/// A text, its numbers from `base` (a file taken in has its own range).
pub fn internal(lexicon: &'static Lexicon, text: &str, base: usize, language: &str) -> Internal {
    let parsed = language::parse_with(text, &holds);
    let source = Source::new(text);
    let mut lower = Lower {
        source: &source,
        base,
        out: Vec::new(),
        wrong: Vec::new(),
        made: 0,
        scopes: vec![HashMap::new()],
        recipe: None,
        put: HashMap::new(),
        pending: Vec::new(),
        text: Arc::from(text),
        comments: Vec::new(),
        call: None,
        rank: 0,
        reckoned: 0,
        fighting: 0,
        own: 0,
        language: language.to_string(),
        lexicon,
        story: crate::plot::names_of(&parsed.file),
    };
    for diagnostic in parsed.diagnostics {
        let number = lower.number(diagnostic.bytes());
        lower.wrong.push((number, diagnostic));
    }
    lower.file(&parsed.file);
    Internal { lines: tree(lower.out, None), diagnostics: lower.wrong }
}

/// A recipe written for one call (`define` … `end`, as the language
/// wrote it): its lines with what the call gives each name, every line the
/// call's number; what is wrong in them.
pub fn called(lexicon: &'static Lexicon, recipe: &RecipeText, call: CallOf, language: &str) -> Internal {
    let source = Source::new(&recipe.text);
    let mut params = HashMap::new();
    gather(&recipe.form.elements, false, &mut params);
    let last = recipe.body.iter().rfind(|stmt| !matches!(stmt.kind, Kind::Blank | Kind::Comment(_))).map(|stmt| stmt.span.start);
    let put: HashMap<String, String> = call.values.iter().map(|value| (value.name.clone(), value.words.join(" "))).collect();
    let (number, id) = (call.number, call.id);
    // (A place given by someone, whose point the recipe reads: found as the
    // film plays, first.)
    let read = &recipe.text[recipe.span.start..recipe.span.end];
    let found: Vec<(String, Vec<String>)> = call
        .values
        .iter()
        .filter(|value| params.get(&value.name).is_some_and(|(_, _, ty)| ty == "place"))
        .filter(|value| !value.words.is_empty() && !(value.words.len() == 2 && value.words.iter().all(|word| word.parse::<f64>().is_ok())))
        .filter(|value| read.contains(&format!("{}.", value.name)))
        .map(|value| (value.name.clone(), value.words.clone()))
        .collect();
    let mut lower = Lower {
        source: &source,
        base: 0,
        out: Vec::new(),
        wrong: Vec::new(),
        made: 0,
        scopes: vec![HashMap::new()],
        recipe: Some(InRecipe { name: recipe.form.word.text.clone(), params, last, early: false }),
        put,
        pending: Vec::new(),
        text: recipe.text.clone(),
        comments: Vec::new(),
        call: Some(call),
        rank: 0,
        reckoned: 0,
        fighting: 0,
        own: 0,
        language: language.to_string(),
        lexicon,
        story: Vec::new(),
    };
    for (name, words) in found {
        lower.word(number, "find_place", std::iter::once(format!("_p{id}_{name}")).chain(words).collect());
    }
    lower.stmts(&recipe.body);
    if lower.recipe.as_ref().is_some_and(|within| within.early) {
        lower.engine(number, format!("label _returned_{id}"));
    }
    let file = lower.call.as_ref().map(|call| call.file.clone()).unwrap_or_default();
    Internal { lines: tree(lower.out, Some((&recipe.form.word.text, &file))), diagnostics: lower.wrong }
}

/// The recipe whose lines are being written: its names, which may be left
/// out, which are teams.
struct InRecipe {
    name: String,
    /// Each name: whether it may be left out, whether it is a team, its type.
    params: HashMap<String, (bool, bool, String)>,
    /// Where its last line starts; whether a `return` before it was written.
    last: Option<usize>,
    early: bool,
}

struct Lower<'a> {
    source: &'a Source<'a>,
    base: usize,
    out: Vec<Out>,
    wrong: Vec<(usize, Diagnostic)>,
    /// Numbers made for reckonings so far.
    made: usize,
    /// The names a film's lines gave a value, innermost last: each its
    /// engine's name.
    scopes: Vec<HashMap<String, String>>,
    recipe: Option<InRecipe>,
    /// What a name stands for in a block played out (`for each`).
    put: HashMap<String, String>,
    /// Lines of the film's own `let`, played as its first scene opens.
    pending: Vec<Out>,
    /// The text being read, for the recipes it holds; the comments just
    /// before the line at hand (a recipe's doc).
    text: Arc<str>,
    comments: Vec<String>,
    /// The recipe's call being written, its rank in a team played out,
    /// how many numbers its reckonings made.
    call: Option<CallOf>,
    rank: usize,
    reckoned: usize,
    /// Within a directed fight: its turns are written who first already
    /// (`pikachu Thunderbolt krabby`), never read as a word said of someone.
    fighting: usize,
    /// The line of the text the statement at hand is written on.
    own: usize,
    /// The language the lines said are written in (`Tables::language`).
    language: String,
    /// The words it is read with (who first: `pikachu walk to b`).
    lexicon: &'static Lexicon,
    /// The film's moments and scenes by name, for `seen(name)` to be checked
    /// (none in a recipe: its calls name them).
    story: Vec<String>,
}

impl Lower<'_> {
    fn number(&self, span: Span) -> usize {
        match &self.call {
            Some(call) if !call.apart => call.number,
            _ => self.base + self.source.position(span.start).line,
        }
    }

    fn end_number(&self, span: Span) -> usize {
        match &self.call {
            Some(call) if !call.apart => call.number,
            _ => self.base + self.source.position(span.end.saturating_sub(1).max(span.start)).line,
        }
    }

    /// Where a line written now is told: its number, and the line of the
    /// text it is written from.
    fn at(&self, number: usize) -> At {
        At { number, own: self.own }
    }

    /// A word's line, as what it is.
    fn word(&mut self, number: usize, verb: &str, words: Vec<String>) {
        let text = std::iter::once(verb.to_string()).chain(words.iter().cloned()).collect::<Vec<_>>().join(" ");
        self.made(number, text, Shape::Word { verb: crate::words::plain(verb), words, wait: true });
    }

    /// A line of nothing: an empty one, a comment (a recipe's doc).
    fn nothing(&mut self, number: usize, line: impl Into<String>) {
        self.out.push(Out::Line(self.at(number), line.into(), Shape::Blank));
    }

    /// A word's line from the words put together for it (`jump pikachu &`):
    /// its word, its words, whether the film waits for it.
    fn line_word(&mut self, number: usize, line: String) {
        let shape = word_of(&line);
        self.made(number, line, shape);
    }

    /// A line written as what it is, its text kept for the tools.
    fn made(&mut self, number: usize, line: String, shape: Shape) {
        self.out.push(Out::Line(self.at(number), line, shape));
    }

    /// Fields for the engine, as they are: a line naming someone of the
    /// film (`position a _x _y`), his code put in by the film.
    fn engine(&mut self, number: usize, line: impl AsRef<str>) {
        let fields = line.as_ref().split_whitespace().map(str::to_string).collect();
        self.made(number, format!(">{}", line.as_ref()), Shape::Engine(fields));
    }

    /// A beat the language makes itself, typed (a test, a number, a loop,
    /// a wait); its text as the engine writes it, kept for the tools.
    fn flow(&mut self, number: usize, beat: impl Into<Beat>) {
        let made = Instr::beat(number, beat.into());
        self.made(number, made.text, made.shape);
    }

    /// A wait of so many frames, or of a number found as it plays (`t {wait}`).
    fn wait_of(&mut self, number: usize, wait: &str) {
        match wait.parse::<f32>() {
            Ok(frames) => self.flow(number, FlowBeat::Wait(Frames(frames))),
            Err(_) => self.flow(number, FlowBeat::WaitFound(Num::Var(wait.to_string()))),
        }
    }

    /// `when health(who) <= N`: the lines played when he has that much of his
    /// life left, each as it was made.
    fn when_life(&mut self, span: Span, number: usize, who: &str, left: &str, lines: Vec<(String, Shape)>) {
        let Ok(life) = left.parse::<i32>() else {
            self.error(span, format!("“when health({who})”: his health as a whole number (“<= 50”), not “{left}”"));
            return;
        };
        let said: Vec<&str> = lines.iter().map(|(line, _)| line.trim()).filter(|line| !line.is_empty()).collect();
        let text = format!("when {who} {left} | {}", said.join(" | "));
        let lines: Vec<Instr> = lines
            .into_iter()
            .filter(|(_, shape)| *shape != Shape::Blank)
            .map(|(line, shape)| (line.trim().to_string(), shape))
            .map(|(line, shape)| Instr { number, text: line, shape, inner: None, span: None, from: Default::default() })
            .collect();
        self.made(number, text, Shape::WhenLife { who: crate::words::plain(who), left: life, lines });
    }

    /// A block opened by that line, closed by `close`.
    fn open(&mut self, number: usize, head: impl Into<String>) {
        let head = head.into();
        let shape = word_of(&head);
        self.out.push(Out::Open(self.at(number), head, shape));
    }

    fn close(&mut self) {
        self.out.push(Out::Close);
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        let number = self.number(span);
        self.wrong.push((number, Diagnostic::error(codes::UNREADABLE, span, message)));
    }

    /// A new number of the event (of the recipe's call, in a recipe).
    fn temp(&mut self) -> String {
        self.made += 1;
        match &self.call {
            Some(call) => format!("_t{}_{}", self.made, call.id),
            None => format!("_t{}", self.made),
        }
    }
}

/// A word's line as front puts its words together (`jump pikachu &`): its
/// word, its words, whether the film waits for it (no `&` at its end).
fn word_of(line: &str) -> Shape {
    let wait = !line.ends_with(" &");
    let words: Vec<String> = line.trim_end_matches([' ', '&']).split_whitespace().map(str::to_string).collect();
    match words.split_first() {
        Some((verb, words)) => Shape::Word { verb: crate::words::plain(verb), words: words.to_vec(), wait },
        None => Shape::Blank,
    }
}
