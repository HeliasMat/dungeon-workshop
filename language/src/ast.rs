//! The tree of a text: what each line is, its blocks, its calculations, each
//! piece with the span it was written at. Nothing is resolved here: a name is
//! only a name, an argument only what was written.

use crate::span::Span;

/// A file: what comes before its first scene, then its scenes.
#[derive(Clone, Debug, Default)]
pub struct File {
    pub head: Vec<Stmt>,
    pub scenes: Vec<Scene>,
}

/// `scene: place who who…` and its lines, up to the next scene.
#[derive(Clone, Debug)]
pub struct Scene {
    /// The `scene:` line.
    pub span: Span,
    /// Written `example:`: a recipe file's small scene, played as any other.
    pub example: bool,
    pub args: Vec<Arg>,
    pub comment: Option<String>,
    pub body: Vec<Stmt>,
}

/// A name as written, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub text: String,
    pub span: Span,
}

/// A line, or a block from its opening to its `end`.
#[derive(Clone, Debug)]
pub struct Stmt {
    pub span: Span,
    pub kind: Kind,
    /// `&` (and `& as name`) at its end.
    pub beside: Option<Beside>,
    /// A `# …` at the end of the line.
    pub comment: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Beside {
    pub span: Span,
    pub named: Option<Name>,
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// A blank line.
    Blank,
    /// A line that is only a comment (without its `#`).
    Comment(String),
    /// `title: …`
    Title(String),
    /// `moment: …`
    Moment(String),
    /// `uses name`: the recipes of another file.
    Uses(Name),
    /// `include name`: the scenes of another film, here.
    Includes(Name),
    /// `for ds`: what the film is written for, before its first scene
    /// (`ds`: played on the DS, from a new game; `ds scene`: in place of a
    /// scene of a game going on, the hero and partner chosen).
    For(Name),
    /// `name = species`, before the first scene.
    Naming {
        name: Name,
        species: Name,
    },
    Said(Said),
    Call(Call),
    /// `let x = …` (`kept`: `keep x = …`).
    Let {
        kept: bool,
        name: Name,
        value: Expr,
    },
    /// `x = …`
    Set {
        name: Name,
        value: Expr,
    },
    /// `return [value]`
    Return(Option<Expr>),
    /// `break`
    Break,
    /// `next`
    Continue,
    Block(Box<Block>),
    /// A line that does not read, kept as written (its diagnostic says why).
    Unreadable(String),
}

/// `who (manners): text`
#[derive(Clone, Debug)]
pub struct Said {
    pub who: Name,
    pub manners: Vec<Name>,
    /// The text as written (after the `:`, trimmed).
    pub text: String,
    pub text_span: Span,
    /// The pieces of its first text, the English one.
    pub pieces: Vec<Piece>,
    /// The same text in other languages, written after it: `Hello! | fr: Bonjour !`.
    pub languages: Vec<Language>,
    /// Its stable id, written last (`#k3f9`, without its `#`): the line's
    /// own name, whatever its words become (`scene_player ids`).
    pub id: Option<Name>,
}

/// A said text in another language: `| fr: Bonjour !`.
#[derive(Clone, Debug)]
pub struct Language {
    /// Its code (`fr`, `es`...).
    pub code: Name,
    pub pieces: Vec<Piece>,
}

/// A piece of a said text.
#[derive(Clone, Debug)]
pub enum Piece {
    Text(String),
    /// `{expr}`
    Value(Expr),
    /// `[1s]`
    Pause(Duration),
    /// `{if test: text | other}`: the first text when the test holds, else
    /// the other (none: nothing).
    Choose {
        test: Expr,
        then: Vec<Piece>,
        otherwise: Vec<Piece>,
    },
}

/// `word argument…`
#[derive(Clone, Debug)]
pub struct Call {
    pub word: Name,
    pub args: Vec<Arg>,
}

/// An argument as written: the vocabulary reads it later, by its form.
#[derive(Clone, Debug)]
pub struct Arg {
    pub span: Span,
    /// As written.
    pub text: String,
    pub kind: ArgKind,
}

#[derive(Clone, Debug)]
pub enum ArgKind {
    /// A word (or a sign: `!`, `?`, `...`).
    Word,
    /// `3`, `-12`, `+20`, `0.5`
    Number(f64),
    Duration(Duration),
    /// `"between quotes"`
    Text(String),
    /// `{expr}`
    Value(Expr),
}

/// A duration, in the unit it was written in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Duration {
    Seconds(f64),
    /// `6f`: frames, 60 a second.
    Frames(f64),
}

impl Duration {
    pub fn frames(self) -> f64 {
        match self {
            Duration::Seconds(seconds) => seconds * 60.0,
            Duration::Frames(frames) => frames,
        }
    }
}

/// A block: its opening, its lines, and those after an `else` or an `option`.
#[derive(Clone, Debug)]
pub struct Block {
    pub opening: Opening,
    /// The opening line.
    pub opening_span: Span,
    /// `as name` at the end of a block played beside the film.
    pub named: Option<Name>,
    pub body: Vec<Stmt>,
    pub parts: Vec<Part>,
    /// Written on one line: `when a arrives : line` (its one line in `body`).
    pub short: bool,
    /// What was written after a word that takes nothing (said, kept).
    pub extra: Option<String>,
    /// A comment on its opening line.
    pub opening_comment: Option<String>,
    /// A comment on its `end` line.
    pub end_comment: Option<String>,
}

/// `else if …`, `else`, `option …` and the lines after it.
#[derive(Clone, Debug)]
pub struct Part {
    pub kind: PartKind,
    pub span: Span,
    pub comment: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum PartKind {
    ElseIf(Expr),
    Else,
    /// `option text`, or `option "text" [if test] [once]`: `text` as written
    /// (all after `option`), `words` the answer shown, `pieces` its pieces;
    /// offered only when its test holds, only until taken if `once`.
    Option {
        text: String,
        words: String,
        pieces: Vec<Piece>,
        test: Option<Expr>,
        once: bool,
    },
}

#[derive(Clone, Debug)]
pub enum Opening {
    If(Expr),
    Repeat(Expr),
    /// `for each x[, rank] in list`
    ForEach {
        name: Name,
        rank: Option<Name>,
        list: Expr,
    },
    While(Expr),
    Together,
    /// `beside`
    Beside,
    When(Event),
    /// `at mark [± duration]`; `at 12s` (so long after the scene's start),
    /// `at music 12s` (once the music has played so long): then `mark`
    /// is the time, as written.
    At {
        mark: Name,
        offset: Option<(bool, Expr)>,
        clock: Clock,
    },
    Every(Expr),
    After(Expr),
    Choice,
    /// `fight directed …`: its arguments as written.
    Fight(Vec<Arg>),
    Recipe(Recipe),
    /// A word of the vocabulary's that holds the lines up to its `end`
    /// (`memory`, `dimensional_scream a`): its line as written.
    Word(Call),
}

impl Opening {
    /// Its word, as it opens its block.
    pub fn word(&self) -> &'static str {
        match self {
            Opening::If(_) => "if",
            Opening::Repeat(_) => "repeat",
            Opening::ForEach { .. } => "for each",
            Opening::While(_) => "while",
            Opening::Together => "together",
            Opening::Beside => "beside",
            Opening::When(_) => "when",
            Opening::At { .. } => "at",
            Opening::Every(_) => "every",
            Opening::After(_) => "after",
            Opening::Choice => "choice",
            Opening::Fight(_) => "fight directed",
            Opening::Recipe(_) => "define",
            Opening::Word(_) => "word",
        }
    }
}

/// What an `at` line counts its time from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// A mark of the scene (`mark name`).
    Mark,
    /// The scene's start.
    Scene,
    /// The music playing now, from its `music` line.
    Music,
    /// Here, where the film is (`at +2s`): so long from now.
    Here,
}

/// What a `when` waits for.
#[derive(Clone, Debug)]
pub enum Event {
    Arrives(Name),
    Leaves(Name),
    Knocked(Name),
    /// As soon as it is true.
    Becomes(Expr),
}

/// `define form [-> type]`
#[derive(Clone, Debug)]
pub struct Recipe {
    pub form: Form,
    pub returns: Option<Type>,
}

/// The form of a word or a recipe: its word, then what it takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub word: Name,
    pub elements: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    /// `name:type[=default]`, `*name:type` (one or more), `name...[=default]` (the
    /// words left).
    Param { name: Name, ty: Option<Type>, default: Option<String>, many: bool, rest: bool },
    /// A word as it is: `to`, `off`.
    Word(Name),
    /// `[ … ]`
    Optional(Vec<Element>),
    /// `a | b`: one of them, each a run of elements.
    OneOf(Vec<Vec<Element>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type {
    pub name: String,
    pub of: Option<Box<Type>>,
}

/// A calculation.
#[derive(Clone, Debug)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Number(f64),
    Duration(Duration),
    Text(String),
    Bool(bool),
    Nothing,
    Name(String),
    List(Vec<Expr>),
    /// `(x, y)`
    Point(Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `a.x`
    Field(Box<Expr>, Name),
    /// `l[i]`
    Index(Box<Expr>, Box<Expr>),
    /// `f(a, b)`
    Call(Name, Vec<Expr>),
    /// A calculation that does not read, kept as written (its diagnostic
    /// says why).
    Unreadable(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Minus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Less,
    LessEq,
    More,
    MoreEq,
    Equal,
    NotEqual,
    Plus,
    Minus,
    Times,
    Divide,
    Rest,
}

impl BinOp {
    pub fn sign(self) -> &'static str {
        match self {
            BinOp::Or => "or",
            BinOp::And => "and",
            BinOp::Less => "<",
            BinOp::LessEq => "<=",
            BinOp::More => ">",
            BinOp::MoreEq => ">=",
            BinOp::Equal => "=",
            BinOp::NotEqual => "!=",
            BinOp::Plus => "+",
            BinOp::Minus => "-",
            BinOp::Times => "*",
            BinOp::Divide => "/",
            BinOp::Rest => "%",
        }
    }

    /// How tightly it binds: the higher, the tighter.
    pub fn binding(self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Less | BinOp::LessEq | BinOp::More | BinOp::MoreEq | BinOp::Equal | BinOp::NotEqual => 3,
            BinOp::Plus | BinOp::Minus => 4,
            BinOp::Times | BinOp::Divide | BinOp::Rest => 5,
        }
    }
}
