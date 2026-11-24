//! What a word of a film is: its forms, what it writes for the engine, who
//! it names, how it plays (`Word`); a group of them (`Group`), the language
//! they make (`Lexicon`), a line as a word reads it (`Line`), and the forms
//! a line fits. The groups themselves are in `crate::groups` and in a crate
//! each (`film_view`...), `film` gathering them in their order; a line is
//! played by the groups whose form fits, the narrower first (`crate::play`).
//!
//! What a form says, word by word (one reader, `crate::grammar`, checks each
//! line against them and says what is wrong):
//!
//! - `who:pokemon`: someone of the scene; `*who`: one or more;
//! - `where:place`: a place (`x y`, someone, `front_of b`, `left_of b`,
//!   `between a b`, `3 tiles from b`, `tile 12 7`, a place's word);
//! - `d:duration` (`1.5s`, `6f`), `n:number`, `word`: one word of that kind;
//! - `thing...`: one word or more (`Oran Berry`); `text...` (or `name:text...`):
//!   words said, all the rest, if any, never someone; `rest...`: the rest of
//!   a word whose end its own group reads (who is in it named, `named_by`);
//! - `a|b|c`: one of those words; `[...]`: what may be left out.
//!
//! Of a word's forms, the first that fits gives it its values: the narrower
//! first (`place_thing Apple near a` before `place_thing Apple x y`).

use crate::film::Film;
use crate::ir::{Instr, Shape};
use crate::words::plain;

mod commands;
pub mod engine;
mod lexicon;

#[cfg(test)]
pub(crate) use lexicon::BARE;
pub(crate) use lexicon::ENGINE_GROUPS;
pub use lexicon::{Defined, Lexicon, Made, Planned, Recipe, TEXT_WORDS};

/// A word of the language: its forms, what it writes for the engine, who it
/// names (those who are on the stage by being named in the scene), and how
/// it plays (none: the text's own words, read before, and the engine's).
#[derive(Clone, Copy)]
pub struct Word {
    pub word: &'static str,
    pub forms: &'static [&'static str],
    pub writes: &'static str,
    pub names: Names,
    pub play: Option<Play>,
    /// How long its line lasts, the one rule of who waits (`time::WordTime`).
    pub time: crate::time::WordTime,
    /// Who its line brings onto the stage, by the values of its form (none:
    /// no one comes by it). `text::cast` reads it.
    pub brings: Option<Brings>,
    /// The family an author finds it in (`vocabulary::FAMILIES`) and how it
    /// stands on the DS: said on one declaration of the word only (`.family`,
    /// `.ds_direct`...); none on the others, which only add their forms.
    pub family: Option<&'static str>,
    pub ds: Option<crate::ds::OnDs>,
    /// Written word first though its form starts with who acts (`fight a vs
    /// b`, `wait pikachu`).
    pub word_first: bool,
}

/// Who a line of a word brings onto the stage, in the middle of a scene: they
/// need no place at its start. Declared on the word (`Word::brings`,
/// `Word::puts`, `Word::arrives`, `Word::brings_with`, `Word::when`), read
/// by `text::cast::placed_by_a_line` from the values of the form that fits.
#[derive(Clone, Copy)]
pub struct Brings {
    pub comes: Comes,
    pub who: Brought,
    /// A word of the form the line must have said for anyone to come
    /// (`jump a water`, not `jump a`).
    pub when: Option<&'static str>,
}

/// How someone comes by a line, and so whether he was there before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comes {
    /// New, unless the head or a line before already named him (what a rod
    /// pulls out of the water, who drops from a Monster House's ceiling).
    New,
    /// Put there: before the scene shows, always by that line; after, new
    /// unless already named (`put`, a formation).
    Put,
    /// Coming in: before the scene shows, by that line; after, not there
    /// before, even named at the head, unless a line had him act (`enter`,
    /// `appear`, `run_in`, `arrival`).
    Arrives,
}

/// Who a line brings: those given some of its form's names, or those a
/// function makes of its values (`gulls wingull 3`: wingull, wingull2,
/// wingull3).
#[derive(Clone, Copy)]
pub enum Brought {
    Given(&'static [&'static str]),
    Made(fn(&crate::grammar::Bound) -> Vec<String>),
}

impl Brings {
    /// Those it brings, of the values a line's form read (who are someone
    /// or not: the caller keeps those of the scene).
    pub fn of(&self, args: &crate::grammar::Bound) -> Vec<String> {
        if self.when.is_some_and(|word| !args.has(word)) {
            return Vec::new();
        }
        match self.who {
            Brought::Given(names) => names.iter().flat_map(|name| args.words(name).iter().map(|word| plain(word))).collect(),
            Brought::Made(made) => made(args),
        }
    }
}

/// How a word plays a line its form fits.
#[derive(Clone, Copy)]
pub enum Play {
    /// It plays it: the form said all.
    Does(fn(&mut Film, &Line)),
    /// It plays it, or tells that it is not its own after all (false: the
    /// next group may), by what only the film at hand knows (`ola people`:
    /// a crowd of that name).
    Maybe(fn(&mut Film, &Line) -> bool),
}

/// Who a line of a word names, when its form does not say it
/// (`named_by`): declared on the word.
#[derive(Clone, Copy)]
pub enum Names {
    /// Those its form's values say (`named_by`).
    ByForm,
    /// No one: a crowd's own, a picture, words on the screen, a fight's own
    /// line.
    NoOne,
    /// Those a function makes of its values (`bubbles a: Yes | b: No`: the
    /// two who speak).
    Made(fn(&crate::grammar::Bound) -> Vec<String>),
}

impl Word {
    pub const fn new(word: &'static str, forms: &'static [&'static str], writes: &'static str) -> Word {
        Word {
            word,
            forms,
            writes,
            names: Names::ByForm,
            play: None,
            time: crate::time::WordTime::ByLine,
            brings: None,
            family: None,
            ds: None,
            word_first: false,
        }
    }

    /// Its line brings those given these names of its form, new unless
    /// already named in the scene (`Comes::New`).
    pub const fn brings(self, who: &'static [&'static str]) -> Word {
        Word { brings: Some(Brings { comes: Comes::New, who: Brought::Given(who), when: None }), ..self }
    }

    /// Its line puts those given these names there (`Comes::Put`).
    pub const fn puts(self, who: &'static [&'static str]) -> Word {
        Word { brings: Some(Brings { comes: Comes::Put, who: Brought::Given(who), when: None }), ..self }
    }

    /// Its line brings those given these names in, not there before
    /// (`Comes::Arrives`).
    pub const fn arrives(self, who: &'static [&'static str]) -> Word {
        Word { brings: Some(Brings { comes: Comes::Arrives, who: Brought::Given(who), when: None }), ..self }
    }

    /// Its line brings those a function makes of its values, new unless
    /// already named (names made from a count: `gulls wingull 3`).
    pub const fn brings_with(self, made: fn(&crate::grammar::Bound) -> Vec<String>) -> Word {
        Word { brings: Some(Brings { comes: Comes::New, who: Brought::Made(made), when: None }), ..self }
    }

    /// What it brings, only when its form read that word (`jump a water`).
    pub const fn when(self, word: &'static str) -> Word {
        match self.brings {
            Some(brings) => Word { brings: Some(Brings { when: Some(word), ..brings }), ..self },
            None => self,
        }
    }

    /// It lasts its `d` (the film waits for it, unless `&`).
    pub const fn lasts(self) -> Word {
        Word { time: crate::time::WordTime::Lasts, ..self }
    }

    /// It lasts until its `who` gets where it goes (waited, unless `&`).
    pub const fn until(self) -> Word {
        Word { time: crate::time::WordTime::Until, ..self }
    }

    /// It takes no time of the film's: the film goes on at once, whatever
    /// it starts (a shake of so long, a face).
    pub const fn instant(self) -> Word {
        Word { time: crate::time::WordTime::Instant, ..self }
    }

    /// Its time, as the registry tells it: a command of the engine said
    /// nothing of is instant.
    pub fn timing(&self) -> crate::time::WordTime {
        match self.time {
            crate::time::WordTime::ByLine if self.play.is_none() => crate::time::WordTime::Instant,
            time => time,
        }
    }

    /// The family an author finds it in (one of `vocabulary::FAMILIES`):
    /// said once, on one declaration of the word.
    pub const fn family(self, family: &'static str) -> Word {
        Word { family: Some(family), ..self }
    }

    /// Written word first, though its form starts with who acts.
    pub const fn word_first(self) -> Word {
        Word { word_first: true, ..self }
    }

    /// On the DS: written as the game's own operations (`how`).
    pub const fn ds_direct(self, how: &'static str) -> Word {
        Word { ds: Some(crate::ds::OnDs::direct(how)), ..self }
    }

    /// On the DS: written, with less than the film shows (`lost`).
    pub const fn ds_close(self, how: &'static str, lost: &'static str) -> Word {
        Word { ds: Some(crate::ds::OnDs::close(how, lost)), ..self }
    }

    /// On the DS: the game can, the export does not write it yet.
    pub const fn ds_work(self, how: &'static str) -> Word {
        Word { ds: Some(crate::ds::OnDs::work(how)), ..self }
    }

    /// On the DS: nothing (`how`: why).
    pub const fn ds_none(self, how: &'static str) -> Word {
        Word { ds: Some(crate::ds::OnDs::none(how)), ..self }
    }

    /// On the DS, how to write it another way (a word not exported).
    pub const fn ds_fix(self, fix: &'static str) -> Word {
        match self.ds {
            Some(ds) => Word { ds: Some(ds.instead(fix)), ..self },
            None => self,
        }
    }

    pub const fn names(self, names: Names) -> Word {
        Word { names, ..self }
    }

    pub const fn does(self, play: fn(&mut Film, &Line)) -> Word {
        Word { play: Some(Play::Does(play)), ..self }
    }

    pub const fn maybe(self, play: fn(&mut Film, &Line) -> bool) -> Word {
        Word { play: Some(Play::Maybe(play)), ..self }
    }
}

/// Those a line of that word brings onto the stage, each with how he comes:
/// by each of its entries that says so (`Word::brings`), the values of the
/// first of its forms that fits (a spot's name said where a form wants
/// numbers, as its point); only those of the scene.
pub fn brought_by(film: &mut Film, verb: &str, words: &[String]) -> Vec<(String, Comes)> {
    let mut out: Vec<(String, Comes)> = Vec::new();
    let entries = film.lexicon.all_groups().flat_map(|group| group.words.iter()).filter(|known| known.word == verb);
    for (word, brings) in entries.filter_map(|word| Some((word, word.brings?))) {
        let args = match first_fitting(film, word, words) {
            Some(args) => Some(args),
            None => {
                let spliced = spots_as_points(film, words);
                first_fitting(film, word, &spliced)
            }
        };
        let Some(args) = args else { continue };
        for one in brings.of(&args) {
            if film.actor(&one).is_some() && !out.iter().any(|(known, _)| *known == one) {
                out.push((one, brings.comes));
            }
        }
    }
    out
}

/// Who a line of that word names, by the values of the first of its forms
/// that fits (a spot's name said where a form wants numbers, as its point):
/// those given someone (`who:pokemon`, `*who`, `who:code`, `who:word`), and
/// those a place is said by (`to b`, `front_of b`); never a thing (`sound
/// click`) nor words said (`text...`), nor what is left out. None if no
/// form of it fits; as its entry says otherwise (`Names`).
pub fn named_by(film: &mut Film, verb: &str, words: &[String]) -> Option<Vec<String>> {
    let entries: Vec<&Word> = film.lexicon.all_groups().flat_map(|group| group.words.iter()).filter(|known| known.word == verb).collect();
    let spliced = spots_as_points(film, words);
    for said in [words, spliced.as_slice()] {
        for word in &entries {
            for form in word.forms {
                let pieces = crate::grammar::read(form);
                let Ok(args) = crate::grammar::bind(film, &pieces, said) else { continue };
                let made = match word.names {
                    Names::NoOne => return Some(Vec::new()),
                    Names::Made(made) => Some(made(&args)),
                    Names::ByForm => None,
                };
                let texts = crate::grammar::texts(form);
                let mut values: Vec<&crate::grammar::Value> = args
                    .values
                    .iter()
                    .filter(|value| value.at.is_some() && value.kind != crate::grammar::Kind::Thing && !texts.contains(&value.name))
                    .collect();
                values.sort_by_key(|value| value.at);
                let said: Vec<String> = made.unwrap_or_else(|| values.into_iter().flat_map(|value| value.words.iter().cloned()).collect());
                let named = said.iter().filter(|word| !film.thing_words.contains(&plain(word)) && film.actor(word).is_some()).map(|word| plain(word));
                return Some(named.collect());
            }
        }
    }
    None
}

/// What the first of a word's forms that fits reads in those words.
fn first_fitting(film: &mut Film, word: &Word, words: &[String]) -> Option<crate::grammar::Bound> {
    word.forms.iter().find_map(|form| crate::grammar::bind(film, &crate::grammar::read(form), words).ok())
}

/// The words, each spot's name that is no one put for its point (`swim
/// lapras from offshore to 420 128`).
fn spots_as_points(film: &mut Film, words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for word in words {
        match spot_named(film, word).filter(|_| film.actor(word).is_none()) {
            Some((x, y)) => out.extend([x.to_string(), y.to_string()]),
            None => out.push(word.clone()),
        }
    }
    out
}

/// A line of a scene at hand: what it is called by, the words after the first
/// (what the forms read, `grammar::bind`: a word reads `args`), the whole line
/// (for a problem to quote), its number, whether the film waits for it, its
/// block.
pub struct Line {
    pub verb: String,
    pub rest: Vec<String>,
    pub text: String,
    pub number: usize,
    pub wait: bool,
    pub block: Vec<Instr>,
    /// What the form that fits gave each of its names (`grammar`): read
    /// there, typed, rather than from `rest` again.
    pub args: crate::grammar::Bound,
    /// What the line is, as read once (`ir`): an anchor, a fight's moment.
    pub shape: Shape,
}

impl Line {
    /// The same line read as another word (`view follows a` as `camera follows a`).
    pub fn as_verb(&self, verb: &str) -> Line {
        Line {
            verb: verb.to_string(),
            rest: self.rest.clone(),
            text: self.text.clone(),
            number: self.number,
            wait: self.wait,
            block: Vec::new(),
            args: self.args.clone(),
            shape: self.shape.clone(),
        }
    }

    /// A line a word makes for another (`eat a`, for the master's apple):
    /// that word and its words, read by the one reader as a film's line is:
    /// the values of the first form of that word's group that fits.
    pub fn word(film: &mut Film, group: &str, verb: &str, rest: Vec<String>, like: &Line) -> Line {
        let made = Instr::word(like.number, verb, &rest, like.wait);
        let text = made.text.trim_end_matches([' ', '&']).to_string();
        let verb = plain(verb);
        let shape = Shape::Word { verb: verb.clone(), words: rest.clone(), wait: like.wait };
        let mut line = Line { verb, text, rest, number: like.number, wait: like.wait, block: Vec::new(), args: Default::default(), shape };
        let (fitting, _) = fitting_forms(film, &line);
        line.args = fitting.into_iter().find(|(from, _)| *from == group).map(|(_, bound)| bound).unwrap_or_default();
        line
    }
}

/// Who a line is about (its `who`), by the name it has in the film.
pub fn who_said(line: &Line) -> String {
    plain(line.args.word("who").unwrap_or_default())
}

/// The words of a theme.
pub struct Group {
    pub name: &'static str,
    pub words: &'static [Word],
    /// The pictures of one's own (`images/`, by name) its words pick by
    /// themselves, no line naming them (`Lexicon::own_pictures`).
    pub pictures: &'static [&'static str],
}

impl Group {
    /// The group of that name, those its words.
    pub const fn new(name: &'static str, words: &'static [Word]) -> Group {
        Group { name, words, pictures: &[] }
    }

    /// Its words pick those pictures of one's own by themselves.
    pub const fn with_pictures(self, pictures: &'static [&'static str]) -> Group {
        Group { pictures, ..self }
    }

    /// A line of its word, its form fitting: played, or not its own (false:
    /// the next group may). What the word keeps for a moment
    /// (`film.temps`) is released once its line is written.
    pub(crate) fn plays(&self, film: &mut Film, line: &Line) -> bool {
        let Some(play) = self.words.iter().find(|word| word.word == line.verb).and_then(|word| word.play) else {
            return false;
        };
        let opened = film.temps.open();
        let played = match play {
            Play::Does(play) => {
                play(film, line);
                true
            }
            Play::Maybe(play) => play(film, line),
        };
        film.temps.close(opened);
        played
    }
}

/// The line that opens a scene in that place, with those of the stage.
pub fn scene_line(place: &str) -> String {
    format!("scene: {place}")
}

/// Whether a word is the library's own (`_bobber`, `_evolution`): its
/// recipes write it, a film never does; no catalogue shows it.
pub fn internal(word: &str) -> bool {
    word.starts_with('_')
}

/// The forms the line fits, by group, each with what it read (the first
/// form of a group that fits); and why the others do not.
pub(crate) fn fitting_forms(film: &mut Film, line: &Line) -> (Vec<(&'static str, crate::grammar::Bound)>, Vec<String>) {
    let mut fitting: Vec<(&'static str, crate::grammar::Bound)> = Vec::new();
    let mut why = Vec::new();
    for group in film.lexicon.all_groups() {
        for word in group.words.iter().filter(|word| word.word == line.verb) {
            for shape in word.forms {
                match crate::grammar::bind(film, &crate::grammar::read(shape), &line.rest) {
                    Ok(mut bound) if !fitting.iter().any(|(known, _)| *known == group.name) => {
                        placed(film, line, &mut bound);
                        fitting.push((group.name, bound));
                    }
                    Ok(_) => {}
                    Err(reason) => why.push(format!("“{} {shape}”: {reason}", word.word)),
                }
            }
        }
    }
    (fitting, why)
}

/// Each value of a line where its words are written, if the line is as
/// written in the film's own text (its words, those of the line at hand).
fn placed(film: &Film, line: &Line, bound: &mut crate::grammar::Bound) {
    let (at, words) = &film.here;
    // (Written who first, `pikachu walk to b`: its words as the word first reads them.)
    let mut words = words.clone();
    if words.len() > 1 && plain(&words[0].0) != line.verb && plain(&words[1].0) == line.verb {
        words.swap(0, 1);
    }
    let as_written =
        *at == line.number && words.len() > line.rest.len() && line.rest.iter().zip(&words[1..]).all(|(word, (written, _))| plain(word) == plain(written));
    if !as_written {
        return;
    }
    for value in &mut bound.values {
        if let Some(first) = value.at.filter(|_| !value.words.is_empty()) {
            let last = first + value.words.len();
            if let (Some((_, from)), Some((_, to))) = (words.get(first + 1), words.get(last)) {
                value.span = Some(language::Span::new(from.start, to.end));
            }
        }
    }
}

/// A spot of the place at hand, or of the film's, by its name: its point.
pub fn spot_named(film: &Film, word: &str) -> Option<(i32, i32)> {
    let word = plain(word);
    let place = film.tables.entrances.get(&film.level).into_iter().flatten();
    place.chain(film.lasting_spots.iter()).find(|(name, ..)| plain(name) == word).map(|(_, x, y)| (*x, *y))
}

/// The words of a word's line its form says are a thing (`sound click`: `click`),
/// by the first of its forms that fits; none if none does.
pub(crate) fn things_in(film: &mut Film, verb: &str, words: &[String]) -> Vec<String> {
    // (A line played every so often, or later: its own word's form, `every
    // 2s sound click`.)
    if matches!(verb, "every" | "after") && words.len() > 2 {
        return things_in(film, &plain(&words[1]), &words[2..]);
    }
    let forms = film.lexicon.all_groups().flat_map(|group| group.words.iter()).filter(|word| word.word == verb).flat_map(|word| word.forms.iter());
    for form in forms {
        if let Ok(bound) = crate::grammar::bind(film, &crate::grammar::read(form), words) {
            return bound.values.into_iter().filter(|value| value.kind == crate::grammar::Kind::Thing).flat_map(|value| value.words).collect();
        }
    }
    Vec::new()
}
