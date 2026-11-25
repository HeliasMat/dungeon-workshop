//! The one reader of the language: the forms of its words (`verbs`, each word's entry)
//! and the first lines of its recipes (`define`, `film/recipes/`), read in
//! one notation, and a line read against them: what it gives each name, or
//! what is wrong. And its one writer: a line made by a form from what is
//! given each name (`write`).
//!
//! - `who:pokemon` (someone of the scene), `where:place` (a place: `x y`,
//!   someone, `front_of b`, `left_of b`, `between a b`, `3 tiles from b`,
//!   `tile 12 7`, `@ x y`, a place's word), `d:duration` (`1.5s`, `6f`),
//!   `n:number`, `word`: one word (a place, as many as it takes);
//! - `*who`: one or more of the scene;
//! - `thing...`: one word or more; `text...`, `rest...`: all the rest, if any;
//! - `a|b`: one of those words; `[...]`: what may be left out;
//! - `turns=4`, `object...=Rare_Fossil`: left out, that (`_` for a space); and
//!   any of them may be given by its name, anywhere: `turns=6`.
//!
//! In a form a bare word is that very word; in a recipe's first line it is a
//! name (`who`), but for `off`, `to`, `against`, `via`, `on`, `with`, `in`.

use crate::film::Film;
use crate::words::{frames_of, plain};

/// What a word must be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Word,
    /// A thing, a sound, an object by its name (`thing`): words, never
    /// someone, though one be a Pokémon's name (`sound click`).
    Thing,
    Someone,
    Place,
    Time,
    Number,
    /// Someone of the scene, or as the engine names him: `m25`, `0`, `target`,
    /// a group.
    Code,
    /// One of a point's numbers: a number, `@spot`, `{x}` found as the
    /// film plays.
    Coordinate,
    /// A property of the registry by its name (or one near it: told by the
    /// word, with the names it may be).
    Property,
}

impl Kind {
    pub const ALL: [Kind; 9] = [Kind::Word, Kind::Thing, Kind::Someone, Kind::Place, Kind::Time, Kind::Number, Kind::Code, Kind::Coordinate, Kind::Property];

    /// Its name in a form, and what it is (`scene_player schema`).
    pub fn told(self) -> (&'static str, &'static str) {
        (self.word(), self.said())
    }

    fn read(word: &str) -> Kind {
        match word {
            "pokemon" | "who" => Kind::Someone,
            "place" | "where" => Kind::Place,
            "duration" | "time" => Kind::Time,
            "number" => Kind::Number,
            "code" => Kind::Code,
            "coordinate" => Kind::Coordinate,
            "property" => Kind::Property,
            "thing" => Kind::Thing,
            _ => Kind::Word,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Kind::Word => "word",
            Kind::Thing => "thing",
            Kind::Someone => "pokemon",
            Kind::Place => "place",
            Kind::Time => "duration",
            Kind::Number => "number",
            Kind::Code => "code",
            Kind::Coordinate => "coordinate",
            Kind::Property => "property",
        }
    }

    pub fn said(self) -> &'static str {
        match self {
            Kind::Word => "a word",
            Kind::Thing => "a thing, a sound, an object (its name)",
            Kind::Someone => "someone of the scene",
            Kind::Place => "a place (x y, someone, front_of b, a spot)",
            Kind::Time => "a duration (1.5s, 6f)",
            Kind::Number => "a number",
            Kind::Code => "someone (of the scene, or m25, 0, target, a group)",
            Kind::Coordinate => "a coordinate (a number, @spot, {x})",
            Kind::Property => "a property of the registry (angle, scale, opacity…)",
        }
    }
}

/// A piece of a form.
#[derive(Clone, Debug)]
pub enum Piece {
    /// One word of a kind (a place: as many as it takes); with a default, it
    /// may be left out.
    One { name: String, kind: Kind, default: Option<Vec<String>> },
    /// Those named, one or more.
    Team(String),
    /// Words: at least so many, as few as the rest leaves; with a default;
    /// of a kind (`thing...`: a thing's name, `text...`: words).
    Words { name: String, least: usize, kind: Kind, default: Option<Vec<String>> },
    /// One of these very words.
    Choice(Vec<String>),
    /// What may be left out.
    Maybe(Vec<Piece>),
}

impl Piece {
    fn name(&self) -> Option<&str> {
        match self {
            Piece::One { name, .. } | Piece::Team(name) | Piece::Words { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// The words a form writes as they are (`to`, `with`, `water`), those that
/// may be left out among them.
pub fn literal_words(form: &[Piece], out: &mut Vec<String>) {
    for piece in form {
        match piece {
            Piece::Choice(words) => out.extend(words.iter().map(|word| plain(word))),
            Piece::Maybe(inner) => literal_words(inner, out),
            _ => {}
        }
    }
}

/// A form of a word: `who:pokemon to where:place [in d:duration]`.
pub fn read(form: &str) -> Vec<Piece> {
    read_as(form, false)
}

/// A recipe's first line, after its name: `who:pokemon where:place turns:number=4`.
pub fn read_recipe(form: &str) -> Vec<Piece> {
    read_as(form, true)
}

fn read_as(form: &str, recipe: bool) -> Vec<Piece> {
    let words: Vec<String> = form.replace('[', " [ ").replace(']', " ] ").split_whitespace().map(str::to_string).collect();
    let mut at = 0;
    pieces(&words, &mut at, recipe)
}

fn pieces(words: &[String], at: &mut usize, recipe: bool) -> Vec<Piece> {
    let mut out = Vec::new();
    while *at < words.len() {
        let word = &words[*at];
        *at += 1;
        match word.as_str() {
            "[" => out.push(Piece::Maybe(pieces(words, at, recipe))),
            "]" => return out,
            _ => out.push(piece(word, recipe)),
        }
    }
    out
}

fn piece(word: &str, _recipe: bool) -> Piece {
    let (word, default) = match word.split_once('=') {
        Some((word, given)) => (word, Some(given.replace('_', " ").split_whitespace().map(str::to_string).collect::<Vec<_>>())),
        None => (word, None),
    };
    if let Some(team) = word.strip_prefix('*') {
        return Piece::Team(plain(team.split(':').next().unwrap_or(team)));
    }
    if let Some(words) = word.strip_suffix("...") {
        let (name, kind) = words.split_once(':').unwrap_or((words, words));
        // (Things, or those of the film and its crowds: one at least.)
        let kind = match kind {
            "thing" => Kind::Thing,
            "code" => Kind::Code,
            _ => Kind::Word,
        };
        let least = usize::from(kind != Kind::Word && default.is_none());
        return Piece::Words { name: plain(name), least, kind, default };
    }
    match word.split_once(':') {
        Some((name, kind)) => Piece::One { name: plain(name), kind: Kind::read(&plain(kind)), default },
        None if default.is_some() => Piece::One { name: plain(word), kind: Kind::Word, default },
        None => Piece::Choice(word.split('|').map(str::to_string).collect()),
    }
}

/// The words a form writes with what is given its names (each name's words,
/// once): its very words and those given, what may be left out left out
/// unless given; none if it needs a name not given, has a name given twice
/// over, or a choice of words it cannot make.
pub fn write(form: &[Piece], given: &[(&str, &[String])]) -> Option<Vec<String>> {
    let mut used = Vec::new();
    let words = written(form, given, &mut used)?;
    given.iter().all(|(name, _)| used.contains(name)).then_some(words)
}

fn written<'a>(form: &[Piece], given: &[(&'a str, &[String])], used: &mut Vec<&'a str>) -> Option<Vec<String>> {
    let mut words = Vec::new();
    for piece in form {
        let said = piece.name().and_then(|name| given.iter().find(|(known, _)| *known == name));
        match (piece, said) {
            (Piece::One { .. } | Piece::Team(_) | Piece::Words { .. }, Some((name, said))) if !said.is_empty() => {
                used.push(name);
                words.extend(said.iter().cloned());
            }
            (Piece::One { default: Some(_), .. } | Piece::Words { default: Some(_), .. }, None) => {}
            (Piece::Words { least: 0, .. }, None) => {}
            (Piece::Choice(choice), _) if choice.len() == 1 => words.push(choice[0].clone()),
            (Piece::Maybe(inner), _) => {
                let mut names = Vec::new();
                named(inner, &mut names);
                // (Left out, unless something in it is given.)
                if names.iter().any(|(name, _)| given.iter().any(|(known, _)| known == name)) {
                    words.extend(written(inner, given, used)?);
                }
            }
            _ => return None,
        }
    }
    Some(words)
}

/// How a form is written back, to say so.
pub fn shown(pieces: &[Piece]) -> String {
    let words: Vec<String> = pieces
        .iter()
        .map(|piece| match piece {
            Piece::One { name, kind, default: Some(default) } => format!("[{name}:{}={}]", kind.word(), default.join("_")),
            Piece::One { name, kind, default: None } => format!("{name}:{}", kind.word()),
            Piece::Team(name) => format!("*{name}"),
            Piece::Words { name, default: Some(default), .. } => format!("[{name}...={}]", default.join("_")),
            Piece::Words { name, .. } => format!("{name}..."),
            Piece::Choice(words) => words.join("|"),
            Piece::Maybe(inner) => format!("[{}]", shown(inner)),
        })
        .collect();
    words.join(" ")
}

/// How many words a place takes, from there: `300 120`, `front_of eevee`,
/// `left_of eevee`, `between a b`, `3 tiles from b`, `tile 12 7`, `@ x y`,
/// someone, a place's word.
pub fn place_words(words: &[String], at: usize) -> usize {
    let number = |at: usize| words.get(at).is_some_and(|word| word.parse::<f32>().is_ok() || found_later(word));
    let word = |at: usize| words.get(at).map(|word| plain(word)).unwrap_or_default();
    let long = if number(at) && word(at + 1) == "tiles" && word(at + 2) == "from" {
        4
    } else if (number(at) && number(at + 1)) || matches!(word(at).as_str(), "front_of" | "behind" | "left_of" | "right_of") {
        2
    } else if word(at) == "between" || (matches!(word(at).as_str(), "tile" | "@") && number(at + 1) && number(at + 2)) {
        3
    } else {
        1
    };
    long.min(words.len().saturating_sub(at)).max(1)
}

/// A word that stands for a recipe's untyped name when its lines are
/// checked alone (`text::checked`): `⟨who⟩` could be anything, of any
/// kind, any one of the words a form chooses from. No film writes it.
pub fn any(name: &str) -> String {
    format!("\u{27e8}{name}\u{27e9}")
}

/// Whether a word stands for anything (`any`).
pub fn stands_for_any(word: &str) -> bool {
    word.starts_with('\u{27e8}') && word.ends_with('\u{27e9}')
}

/// A number found as the film plays: `{_px}`.
fn found_later(word: &str) -> bool {
    word.starts_with('{') && word.ends_with('}')
}

/// Whether a word is of its kind.
fn of_kind(film: &mut Film, kind: Kind, word: &str) -> bool {
    if stands_for_any(word) {
        return true;
    }
    match kind {
        Kind::Word | Kind::Thing => true,
        // (A number alone is not a place: `x y` is.)
        Kind::Place => true,
        Kind::Someone => film.actor(word).is_some(),
        Kind::Time => frames_of(word).is_some() || found_later(word),
        Kind::Number => number_in(word).is_some() || found_later(word),
        Kind::Code => film.actor(word).is_some() || engine_code(word) || film.groups.contains_key(&plain(word)),
        Kind::Coordinate => number_in(word).is_some() || word.starts_with('@') || found_later(word),
        Kind::Property => engine::props::find(&plain(word)).is_some(),
    }
}

/// Whether a word names someone as the engine does: `m25`, `m25.1`, `i109`,
/// `o66`, `p2`, a character's number, `target`, `-` (no one).
fn engine_code(word: &str) -> bool {
    let coded = word.len() > 1
        && word.starts_with(['m', 'i', 'o', 'p'])
        && word[1..].split('.').all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    coded || (!word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit())) || matches!(word, "target" | "-")
}

/// A value as what it is, read from its words by its kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Given {
    /// Words as they are.
    Words(Vec<String>),
    /// Someone, or several, by their names.
    Someone(Vec<String>),
    /// A point given in two numbers.
    Point(i32, i32),
    /// A place said otherwise (`front_of b`, someone, a spot's name).
    Place(Vec<String>),
    /// A time, in frames.
    Time(i32),
    Number(f32),
    /// A number the engine finds as the film plays: its name (`_px` for
    /// `{_px}`).
    Later(String),
}

impl Value {
    /// What it is, by its kind.
    pub fn given(&self) -> Given {
        let words = &self.words;
        match (self.kind, words.as_slice()) {
            (_, [one]) if found_later(one) => Given::Later(one.trim_start_matches('{').trim_end_matches('}').to_string()),
            (Kind::Someone | Kind::Code, _) => Given::Someone(words.clone()),
            (Kind::Place, [x, y]) => match (coordinate_in(x), coordinate_in(y)) {
                (Some(x), Some(y)) => Given::Point(x, y),
                _ => Given::Place(words.clone()),
            },
            (Kind::Place, _) => Given::Place(words.clone()),
            (Kind::Time, [one]) => frames_of(one).map_or_else(|| Given::Words(words.clone()), Given::Time),
            (Kind::Number | Kind::Coordinate, [one]) => number_in(one).map_or_else(|| Given::Words(words.clone()), Given::Number),
            _ => Given::Words(words.clone()),
        }
    }
}

/// What a line gives a name of the form.
#[derive(Clone, Debug)]
pub struct Value {
    pub name: String,
    pub words: Vec<String>,
    pub kind: Kind,
    /// Where it starts among the line's words; None when it was not said
    /// there (a default, or given by its name).
    pub at: Option<usize>,
    /// Where its words are in the film's text, when the line is as written
    /// there (`Film::here`).
    pub span: Option<language::Span>,
}

/// A line read against a form: each name, what it was given.
#[derive(Clone, Debug, Default)]
pub struct Bound {
    pub values: Vec<Value>,
}

impl Bound {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.iter().find(|value| value.name == name)
    }

    /// The words given to that name, if any.
    pub fn words(&self, name: &str) -> &[String] {
        self.get(name).map_or(&[], |value| value.words.as_slice())
    }

    /// The one word given to that name.
    pub fn word(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|value| value.words.first()).map(String::as_str)
    }

    /// A number given to that name (a word of a number, whatever its kind).
    pub fn number(&self, name: &str) -> Option<f32> {
        match self.get(name)?.given() {
            Given::Number(number) => Some(number),
            _ => self.word(name).and_then(number_in),
        }
    }

    /// A time given to that name, in frames.
    pub fn frames(&self, name: &str) -> Option<i32> {
        match self.get(name)?.given() {
            Given::Time(frames) => Some(frames),
            _ => self.word(name).and_then(frames_of),
        }
    }

    /// The groups of words given to that name, between its commas
    /// (`walk a via 100 200, throne, to b`; `step_by_step a right 3, b left 2`).
    pub fn list(&self, name: &str) -> Vec<Vec<String>> {
        self.parts(name, ',')
    }

    /// What was given between `|`, read once: `Explosion | Iron Defense`.
    pub fn choices(&self, name: &str) -> Vec<String> {
        self.parts(name, '|').into_iter().map(|part| part.join(" ")).collect()
    }

    /// The points given between commas, those that are two numbers: `300 200, 320 180`.
    pub fn points(&self, name: &str) -> Vec<(i32, i32)> {
        self.list(name)
            .into_iter()
            .filter_map(|pair| {
                let mut numbers = pair.iter().map(|word| coordinate_in(word));
                Some((numbers.next()??, numbers.next()??))
            })
            .collect()
    }

    /// The words given, cut where that sign stands.
    pub fn parts(&self, name: &str, between: char) -> Vec<Vec<String>> {
        self.words(name).join(" ").split(between).map(|part| part.split_whitespace().map(str::to_string).collect()).collect()
    }

    /// Whether that name was given anything.
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some_and(|value| !value.words.is_empty())
    }

    /// A place given as two numbers, if it is: `300 160`.
    pub fn point(&self, name: &str) -> Option<(i32, i32)> {
        if let Some(Given::Point(x, y)) = self.get(name).map(Value::given) {
            return Some((x, y));
        }
        match self.words(name) {
            [x, y] => Some((coordinate_in(x)?, coordinate_in(y)?)),
            _ => None,
        }
    }

    /// The places given, where they are among the line's words.
    pub fn places(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.values.iter().filter(|value| value.kind == Kind::Place).filter_map(|value| Some((value.at?, value.words.len())))
    }

    /// The words the line gave, in the order it said them (a word of a
    /// choice too, `follows`), none left out or given by its name: the same
    /// words, for another word's forms to read (`view on a` as `camera on a`).
    pub fn said(&self) -> Vec<String> {
        let mut said: Vec<&Value> = self.values.iter().filter(|value| value.at.is_some()).collect();
        said.sort_by_key(|value| value.at);
        said.into_iter().flat_map(|value| value.words.iter().cloned()).collect()
    }
}

/// Why a line does not fit, as far as it went.
#[derive(Default)]
struct Missed {
    at: usize,
    why: String,
}

impl Missed {
    fn tell(&mut self, at: usize, why: String) {
        if at >= self.at {
            (self.at, self.why) = (at, why);
        }
    }
}

/// The line read against the form: what each name is given; else, why not.
pub fn bind(film: &mut Film, form: &[Piece], words: &[String]) -> Result<Bound, String> {
    // (Given by name, anywhere: `turns=6`.)
    let mut names = Vec::new();
    named(form, &mut names);
    let mut given = Vec::new();
    let mut said = Vec::new();
    for word in words {
        match word.split_once('=') {
            Some((name, value)) if names.iter().any(|(known, _)| *known == plain(name)) => {
                let kind = names.iter().find(|(known, _)| *known == plain(name)).map_or(Kind::Word, |(_, kind)| *kind);
                given.push(Value {
                    name: plain(name),
                    words: value.replace('_', " ").split_whitespace().map(str::to_string).collect(),
                    kind,
                    at: None,
                    span: None,
                });
            }
            _ => said.push(word.clone()),
        }
    }
    let mut missed = Missed::default();
    let mut bound = given.clone();
    if !(Fit { words: &said, given: &given, missed: &mut missed, bound: &mut bound }).go(film, form, 0) {
        return Err(if missed.why.is_empty() { format!("too many: “{}”", said.join(" ")) } else { missed.why });
    }
    // (What was left out: its default, or nothing.)
    let mut out = Bound { values: bound };
    defaults(form, &mut out);
    Ok(out)
}

/// The names of a form given words said, never someone though one be a
/// Pokémon's name: `text...`, `title:text...`.
pub fn texts(form: &str) -> Vec<String> {
    let words = form.split(|letter: char| letter.is_whitespace() || letter == '[' || letter == ']');
    let said = words.filter_map(|word| word.split('=').next()?.strip_suffix("..."));
    said.filter_map(|said| match said.split_once(':') {
        Some((name, "text")) => Some(plain(name)),
        None if said == "text" => Some(said.to_string()),
        _ => None,
    })
    .collect()
}

/// The names of a form, and their kinds.
fn named(form: &[Piece], out: &mut Vec<(String, Kind)>) {
    for piece in form {
        match piece {
            Piece::One { name, kind, .. } => out.push((name.clone(), *kind)),
            Piece::Team(name) => out.push((name.clone(), Kind::Someone)),
            Piece::Words { name, kind, .. } => out.push((name.clone(), *kind)),
            Piece::Maybe(inner) => named(inner, out),
            Piece::Choice(_) => {}
        }
    }
}

fn defaults(form: &[Piece], out: &mut Bound) {
    for piece in form {
        let missing = piece.name().is_some_and(|name| out.get(name).is_none());
        match piece {
            Piece::One { name, kind, default } if missing => {
                out.values.push(Value { name: name.clone(), words: default.clone().unwrap_or_default(), kind: *kind, at: None, span: None });
            }
            Piece::Words { name, kind, default, .. } if missing => {
                out.values.push(Value { name: name.clone(), words: default.clone().unwrap_or_default(), kind: *kind, at: None, span: None });
            }
            Piece::Team(name) if missing => out.values.push(Value { name: name.clone(), words: Vec::new(), kind: Kind::Someone, at: None, span: None }),
            Piece::Maybe(inner) => defaults(inner, out),
            _ => {}
        }
    }
}

/// A line's words fitted to a form, piece by piece: what is bound so far,
/// what was given by name, and why it fits not, for the furthest it went.
struct Fit<'a> {
    words: &'a [String],
    given: &'a [Value],
    missed: &'a mut Missed,
    bound: &'a mut Vec<Value>,
}

impl Fit<'_> {
    /// The pieces from the first, the words from `at`: whether all fit to the end.
    fn go(&mut self, film: &mut Film, form: &[Piece], at: usize) -> bool {
        let words = self.words;
        let Some((piece, rest)) = form.split_first() else {
            if at < words.len() {
                self.missed.tell(at, format!("too many: “{}”", words[at..].join(" ")));
            }
            return at == words.len();
        };
        // (Given by its name: nothing of the line for it.)
        if piece.name().is_some_and(|name| self.given.iter().any(|value| value.name == name)) {
            return self.go(film, rest, at);
        }
        match piece {
            Piece::One { name, kind, default } => self.one(film, rest, at, name, *kind, default.is_some()),
            Piece::Team(name) => self.team(film, rest, at, name),
            Piece::Words { name, least, kind, default } => {
                let left = words.len().saturating_sub(at);
                let least = if default.is_some() { 0 } else { *least };
                if left < least {
                    self.missed.tell(at, format!("{name}... is missing"));
                    return false;
                }
                // (As few as leave the rest to fit: `Apple 100 180 rattata`.)
                (least..=left).any(|long| {
                    let value = Value { name: name.clone(), words: words[at..at + long].to_vec(), kind: *kind, at: (long > 0).then_some(at), span: None };
                    if long == 0 && default.is_some() {
                        return self.go(film, rest, at);
                    }
                    self.take(film, rest, value, at + long)
                })
            }
            Piece::Choice(choices) => match words.get(at).map(|word| plain(word)) {
                // (The word said is kept under its own name: `args.has("off")`.)
                Some(word) if stands_for_any(&word) || choices.iter().any(|choice| plain(choice) == word) => {
                    let value = Value { name: word.clone(), words: vec![word.clone()], kind: Kind::Word, at: Some(at), span: None };
                    self.take(film, rest, value, at + 1)
                }
                Some(word) => {
                    self.missed.tell(at, format!("“{word}”: {} expected", choices.join(" or ")));
                    false
                }
                None => {
                    self.missed.tell(at, format!("{} is missing", choices.join(" or ")));
                    false
                }
            },
            Piece::Maybe(inner) => {
                let mut with: Vec<Piece> = inner.clone();
                with.extend(rest.iter().cloned());
                self.go(film, &with, at) || self.go(film, rest, at)
            }
        }
    }

    /// A value bound, then the rest fitted from `next`: unbound if it does not fit.
    fn take(&mut self, film: &mut Film, rest: &[Piece], value: Value, next: usize) -> bool {
        self.bound.push(value);
        let fits = self.go(film, rest, next);
        if !fits {
            self.bound.pop();
        }
        fits
    }

    /// One value of its kind (a place, two numbers or a name); left out, with
    /// its default.
    fn one(&mut self, film: &mut Film, rest: &[Piece], at: usize, name: &str, kind: Kind, has_default: bool) -> bool {
        let words = self.words;
        let word = words.get(at);
        let long = if kind == Kind::Place { place_words(words, at) } else { 1 };
        let fits_here = match word {
            // (A number alone is not a place.)
            Some(word) if kind == Kind::Place && long == 1 && word.parse::<f32>().is_ok() => false,
            Some(word) => of_kind(film, kind, word),
            None => false,
        };
        if fits_here {
            let value = Value { name: name.to_string(), words: words[at..at + long].to_vec(), kind, at: Some(at), span: None };
            if self.take(film, rest, value, at + long) {
                return true;
            }
        } else {
            match word {
                Some(word) => self.missed.tell(at, format!("“{word}” is not {} ({name})", kind.said())),
                None => self.missed.tell(at, format!("{name} is missing ({})", kind.said())),
            }
        }
        // (Left out, with its default.)
        has_default && self.go(film, rest, at)
    }

    /// Those of the scene, as many as leave the rest to fit, the most first.
    fn team(&mut self, film: &mut Film, rest: &[Piece], at: usize, name: &str) -> bool {
        let words = self.words;
        let those = words[at.min(words.len())..]
            .iter()
            .take_while(|word| stands_for_any(word) || film.actor(word).is_some() || film.groups.contains_key(&plain(word)))
            .count();
        if those == 0 {
            self.missed.tell(at, format!("{name} is missing (someone of the scene)"));
            return false;
        }
        (1..=those).rev().any(|long| {
            let value = Value {
                name: name.to_string(),
                words: words[at..at + long].iter().map(|word| plain(word)).collect(),
                kind: Kind::Someone,
                at: Some(at),
                span: None,
            };
            self.take(film, rest, value, at + long)
        })
    }
}

/// A number as a film writes it: digits, never `nan` nor `inf`, nor one too
/// big for the engine's numbers (`1e40`).
pub fn number_in(word: &str) -> Option<f32> {
    word.parse::<f32>().ok().filter(|number| number.is_finite())
}

/// The farthest a point of a film can be from the place's corner, in
/// pixels: past it, no place is that big (and the sums made with it would
/// overflow).
pub const FARTHEST: f32 = 100_000.0;

/// A coordinate as a film writes it (`300`, `-12.5`): a number, no farther
/// than `FARTHEST`.
pub fn coordinate_in(word: &str) -> Option<i32> {
    number_in(word).filter(|number| number.abs() <= FARTHEST).map(|number| number as i32)
}

/// Whether a form's first part is one who acts: someone (`who:pokemon`,
/// `who:code`, a `who:word` by its name) or a team (`*who`, said of one
/// only when given one).
pub fn performer_first(pieces: &[Piece]) -> bool {
    match pieces.first() {
        Some(Piece::Team(_)) | Some(Piece::One { kind: Kind::Someone | Kind::Code, .. }) => true,
        Some(Piece::One { kind: Kind::Word, name, .. }) => name == "who",
        _ => false,
    }
}

/// A form as an author writes it: who first if its first part is who acts
/// (`walk who:pokemon to where:place` shows `who:pokemon walk to where:place`),
/// else the word first (`music name:word`), as always when `who_may_lead` is
/// false (a word written word first, `fight a vs b`: `Lexicon::shown_form`).
pub fn shown_form(word: &str, form: &str, who_may_lead: bool) -> String {
    let form = form.trim();
    let who_first = who_may_lead && performer_first(&read(form));
    match (who_first, form.split_once(' ')) {
        (true, Some((who, rest))) => format!("{who} {word} {rest}"),
        (true, None) => format!("{form} {word}"),
        (false, _) => format!("{word} {form}").trim_end().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_is_shown_who_first_when_who_acts() {
        assert_eq!(shown_form("walk", "who:pokemon to where:place", true), "who:pokemon walk to where:place");
        assert_eq!(shown_form("exit", "*who [via side:word]", true), "*who exit [via side:word]");
        assert_eq!(shown_form("jump", "who:pokemon", true), "who:pokemon jump");
        assert_eq!(shown_form("music", "name:word", true), "music name:word");
        assert_eq!(shown_form("frame", "members... [follows]", true), "frame members... [follows]");
        assert_eq!(shown_form("train", "[gap:number] *who", true), "train [gap:number] *who");
    }
}
