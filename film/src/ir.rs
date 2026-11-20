//! The lines a scene plays, each read once into what it is: a line said,
//! a word and its words, fields for the engine, a line anchored to what
//! happens, what a fight plays when someone has so much life left. The
//! words read what a line is from here, never its text again; a word that
//! has another line played makes it here too (`Instr::word`,
//! `Instr::engine`).

use crate::lines::{anchor_parts, arrivals, at_clock, at_mark};
use crate::text::spoken;
use crate::words::plain;

/// A line of a scene, and the lines up to its `end` when it holds a block.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Instr {
    pub number: usize,
    /// As written, for what is told of it.
    pub text: String,
    pub shape: Shape,
    pub inner: Option<Vec<Instr>>,
    /// Where it stands in the film's own text, a line as written there;
    /// None for one a recipe, a word or a file taken in made (what the tools
    /// may edit by its fields is only the former).
    pub span: Option<language::Span>,
    /// The recipe line it was written from, for a line a recipe wrote.
    pub from: From,
}

/// Where a line a recipe wrote stands in that recipe: its name, its file
/// as told (none: the film's own), its own line there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeLine {
    pub recipe: String,
    pub file: String,
    pub line: usize,
}

impl RecipeLine {
    /// Where it is, to say so: `in recipe `bow` (gestures.film:12)`.
    pub fn told(&self) -> String {
        format!("in recipe `{}` ({})", self.recipe, crate::text::recipes::told_at(&self.file, self.line))
    }
}

/// The recipe line a line comes from, if a recipe wrote it. No part of what
/// the line is: two lines alike are alike wherever they were written from.
#[derive(Clone, Debug, Default)]
pub struct From(pub Option<std::sync::Arc<RecipeLine>>);

impl PartialEq for From {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// The marks `at 12s` and `at music 12s` count from (see `Shape::AtMark`):
/// the scene's start, the music playing now. No mark is so named
/// (`mark name` takes a word).
pub const SCENE_CLOCK: &str = "@scene";
pub const MUSIC_CLOCK: &str = "@music";
/// What `at +2s` counts from: here, where the film is.
pub const HERE_CLOCK: &str = "@here";

/// What a line is.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Shape {
    #[default]
    Blank,
    /// `who (manners): words`; `beside`: ended by `&`, said beside the film.
    Said { who: String, brackets: Option<String>, words: String, beside: bool },
    /// Fields for the engine, as they are (`>`, or a line naming someone
    /// of the film, his code put in by the film).
    Engine(Vec<String>),
    /// A beat the language makes itself, typed: its tests, its numbers,
    /// its loops and waits (`while`, `if`, `set`, `calc`, `t`…).
    Beat(engine::Beat),
    /// `when a b arrives [: line]`: who, and the line after its colon.
    Arrives { who: Vec<String>, then: Option<Box<Instr>> },
    /// `at name [+ 2s] : [line]`: the mark, so many frames after it, the line;
    /// `at 12s : …`, `at music 12s : …`: the mark is `SCENE_CLOCK`,
    /// `MUSIC_CLOCK`.
    AtMark { name: String, after: i32, then: Option<Box<Instr>> },
    /// `when mewtwo 50 | line | line`, before or in a fight: who, how much
    /// of his life, the lines played then.
    WhenLife { who: String, left: i32, lines: Vec<Instr> },
    /// `titre: …`: the film's title.
    Title(String),
    /// `transition: volet`: how the scene that follows opens.
    Transition(String),
    /// `pinchy = krabby`: a name the film gives, before its first scene.
    Naming { name: String, species: String },
    /// `scene: beach piplup pinchy`: a scene opens, there, with them.
    Opens { place: String, listed: Vec<String> },
    /// `definis …`: a recipe, as the language wrote it, written again at
    /// each call with what it is given (`front::called`).
    Recipe(std::sync::Arc<crate::text::front::RecipeText>),
    /// `choice A | B`: the player's choice, its answers.
    Choice { options: Vec<String> },
    /// A word and the words after it; `wait`: the film waits for it (not
    /// ended by `&`).
    Word { verb: String, words: Vec<String>, wait: bool },
}

impl Instr {
    /// A line of text read alone into what it is: for the outline of a text
    /// as written (`doc::outline`) and the tests. A film's lines are made
    /// as what they are by the language (`text::front`), never read so.
    pub fn line(number: usize, text: &str) -> Instr {
        Instr { number, text: text.to_string(), shape: Shape::of(number, text), inner: None, span: None, from: From(None) }
    }

    /// A word played with these words (`wait`: not beside the film).
    pub fn word(number: usize, verb: &str, words: &[String], wait: bool) -> Instr {
        let mut text = std::iter::once(verb.to_string()).chain(words.iter().cloned()).collect::<Vec<_>>().join(" ");
        if !wait {
            text.push_str(" &");
        }
        Instr { number, text, shape: Shape::Word { verb: plain(verb), words: words.to_vec(), wait }, inner: None, span: None, from: From(None) }
    }

    /// A beat the language makes itself, its text as the engine writes it
    /// (`>while _round1 < 3`).
    pub fn beat(number: usize, beat: engine::Beat) -> Instr {
        let text = format!(">{}", beat.fields(&scenes::vocabulary::POKEMON).unwrap_or_default().join(" "));
        Instr { number, text, shape: Shape::Beat(beat), inner: None, span: None, from: From(None) }
    }

    /// Fields for the engine as they are.
    pub fn engine(number: usize, fields: Vec<String>) -> Instr {
        Instr { number, text: format!("> {}", fields.join(" ")), shape: Shape::Engine(fields), inner: None, span: None, from: From(None) }
    }

    /// Its word, if it is one (plain).
    pub fn verb(&self) -> Option<&str> {
        match &self.shape {
            Shape::Word { verb, .. } => Some(verb),
            _ => None,
        }
    }

    /// The words after its word, if it is one.
    pub fn words(&self) -> &[String] {
        match &self.shape {
            Shape::Word { words, .. } => words,
            _ => &[],
        }
    }
}

impl Shape {
    fn of(number: usize, text: &str) -> Shape {
        let text = text.trim();
        if text.is_empty() {
            return Shape::Blank;
        }
        if let Some(raw) = text.strip_prefix('>') {
            return Shape::Engine(raw.split_whitespace().map(str::to_string).collect());
        }
        let (head, tail) = anchor_parts(text);
        let then = || (!tail.is_empty()).then(|| Box::new(Instr::line(number, tail)));
        if let Some(who) = arrivals(head) {
            return Shape::Arrives { who: who.into_iter().map(str::to_string).collect(), then: then() };
        }
        if let Some((name, after)) = at_mark(head).or_else(|| at_clock(head)).filter(|_| text.contains(':')) {
            return Shape::AtMark { name, after, then: then() };
        }
        let words: Vec<&str> = text.split_whitespace().collect();
        if let ["when", who, left, ..] = words.as_slice()
            && text.contains('|')
            && let Ok(left) = left.parse::<i32>()
        {
            let lines = text.split('|').skip(1).map(|line| line.trim()).filter(|line| !line.is_empty()).map(|line| Instr::line(number, line)).collect();
            return Shape::WhenLife { who: plain(who), left, lines };
        }
        match words.as_slice() {
            ["choice", ..] if text.contains('|') => {
                return Shape::Choice { options: text["choice".len()..].split('|').map(|option| option.trim().to_string()).collect() };
            }
            _ => {}
        }
        if let Some((who, brackets, said)) = spoken(text) {
            let (said, beside) = match said.strip_suffix(" &") {
                Some(said) => (said.trim_end(), true),
                None => (said, false),
            };
            return Shape::Said { who: who.to_string(), brackets: brackets.map(str::to_string), words: said.to_string(), beside };
        }
        let wait = !text.ends_with(" &");
        let words: Vec<String> = text.trim_end_matches([' ', '&']).split_whitespace().map(str::to_string).collect();
        Shape::Word { verb: plain(&words[0]), words: words[1..].to_vec(), wait }
    }
}
