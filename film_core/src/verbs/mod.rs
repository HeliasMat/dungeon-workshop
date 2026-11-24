//! The words of a film, by what they do: a group a theme, each with its
//! own words (`WORDS`, its `GROUP`): a word, its forms, what it writes for
//! the engine, who it names, how it plays. The groups here are those the
//! core leans on; the others are a crate each (`film_fishing`...), and
//! `film` gathers them all in their order (`Lexicon`). A line goes to the
//! groups whose form of its word fits, the narrower first; what none plays
//! is the engine's own command (`engine`), or a recipe of the film's.
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
use crate::words::{engine_word, plain};

pub mod aura;
mod commands;
pub mod crowds;
pub mod effects;
pub mod engine;
pub mod game;
pub mod heard;
pub mod items;
mod lexicon;
pub mod nodes;
pub mod pictures;
pub mod rituals;
pub mod said;
pub mod sky;
pub mod things;
pub mod timing;
pub mod view;

pub use lexicon::{Lexicon, TEXT_WORDS};

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
    fn plays(&self, film: &mut Film, line: &Line) -> bool {
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

/// A line of a scene other than a line said, as the engine's commands.
/// Ending with `&`: the film goes on without waiting for it.
impl Film {
    /// A word played for another, by the tree: that word and its words, the
    /// library's recipes played out in it (a gesture of
    /// `film/recipes/gestures.film`: `evolution a m133` for `evolve`).
    pub fn call(&mut self, number: usize, verb: &str, words: &[&str]) {
        let words: Vec<String> = words.iter().map(|word| word.to_string()).collect();
        for instr in crate::text::recipes::unfold_word(self, Instr::word(number, verb, &words, true)) {
            command(self, &instr);
        }
    }
}

pub fn command(film: &mut Film, item: &Instr) {
    // (Its words where they are written, for a problem to say which.)
    let here = (item.number, film.written_words(item));
    let kept = std::mem::replace(&mut film.here, here);
    let before = film.problems.len();
    command_here(film, item);
    film.here = kept;
    // (A line a recipe wrote: what is wrong with it says which recipe line.)
    if let Some(from) = &item.from.0 {
        for problem in film.problems[before..].iter_mut().filter(|problem| !problem.is_remark()) {
            problem.message = format!("{} ({})", problem.message, from.told());
        }
    }
}

fn command_here(film: &mut Film, item: &Instr) {
    // (A beat the language made itself, typed: a test, a number, a loop, a wait.)
    if let Shape::Beat(beat) = &item.shape {
        film.language_beat(beat.clone());
        return;
    }
    // (Fields for the engine: as they are.)
    if let Shape::Engine(fields) = &item.shape {
        if !fields.is_empty() {
            // (Written for the engine as it is: each word typed by the film's names.)
            film.engine_command(&fields[0], &fields[1..], None);
        }
        return;
    }
    let text = item.text.trim_end_matches([' ', '&']).to_string();
    // (A word: its word and its words as read once; a line anchored or a
    // fight's moment: by its first word, what it is kept with it.)
    let (verb, rest, wait) = match &item.shape {
        Shape::Word { verb, words, wait } => (verb.clone(), words.clone(), *wait),
        Shape::Blank => return,
        _ => {
            let words: Vec<String> = text.split_whitespace().map(str::to_string).collect();
            (plain(&words[0]), words[1..].to_vec(), !item.text.ends_with(" &"))
        }
    };
    // (Its first word as written: the engine's command, by the name the line gives it.)
    let first = text.split_whitespace().next().unwrap_or_default().to_string();
    let line = Line {
        verb,
        rest,
        text,
        number: item.number,
        wait,
        block: item.inner.clone().unwrap_or_default(),
        args: Default::default(),
        shape: item.shape.clone(),
    };
    // (`end`: what was opened last is closed, as it was told to be.)
    if line.verb == "end" && line.rest.is_empty() {
        match film.timing.opened.pop() {
            Some(close) => close(film, &line),
            None => film.wrong(line.number, "“end” closes nothing here: no block, no image shot, dimensional_scream, memory, choice or fight open".to_string()),
        }
        return;
    }
    // (A word of the old vocabulary, or the library's own, in the film's own
    // text: refused, the form now its fix.)
    if item.from.0.is_none() && item.span.is_some() {
        let written = film.here.1.clone();
        // (Who first, `pikachu nap`: the old word second.)
        let second = written.get(1).map(|(said, _)| plain(said)).filter(|_| !film.lexicon.is_word(&line.verb, &film.source));
        let old = std::iter::once(line.verb.clone()).chain(second).find(|word| film.lexicon.replaced(word).is_some()).unwrap_or_default();
        if crate::vocabulary::told_replaced(film, line.number, &written, &old) {
            return;
        }
        if internal(&line.verb) && written.first().is_some_and(|(said, _)| plain(said) == line.verb) {
            film.unknown(line.number, "unknown word", &first, film.lexicon.every_word().map(|(_, word, ..)| word), "film/FILMS.md");
            return;
        }
    }
    // (The line must fit one of its word's forms; the groups
    // whose form fits play it first; one that only the engine's form fits
    // goes to the engine.)
    let (fitting, why) = fitting_forms(film, &line);
    // (A word of an optional family: its library taken in, `uses fishing`.)
    if item.from.0.is_none() && item.span.is_some() && !fitting.is_empty() {
        crate::vocabulary::asks_its_library(film, line.number, &line.verb);
    }
    // (Written the word first, said of someone: who first now, `pikachu walk to b`.)
    if item.from.0.is_none()
        && let Some((group, _)) = fitting.first()
        && crate::order::said_of_someone(film, group, &line.verb, &line.rest)
    {
        let written = film.here.1.clone();
        crate::order::told_who_first(film, line.number, &written, &line.verb);
    }
    // (What a line of the text was read as, by the form that fits first, a
    // spot still by its name: for who edits it by its fields. Not those a
    // recipe, a scene or a word writes with its number.)
    let as_in_the_text = item.span.is_some();
    if let Some((group, bound)) = fitting.first().filter(|_| as_in_the_text) {
        let read = crate::compile::ReadAs { word: line.verb.clone(), group, values: bound.values.clone() };
        film.read_as.entry(line.number).or_insert(read);
    }
    // (A place said in numbers where the place has a name for it: its name
    // reads better, and holds if the place changes. A place said by its name,
    // a spot of the place's or of the film's: its point, for the words that
    // reckon with numbers, `crystal a b`.)
    let mut line = line;
    let places: Vec<(usize, usize)> = fitting.first().map(|(_, bound)| bound.places().collect()).unwrap_or_default();
    let mut said: Vec<(i32, i32)> = places
        .iter()
        .filter(|(_, long)| *long == 2)
        .filter_map(|(at, _)| Some((line.rest[*at].parse::<f32>().ok()? as i32, line.rest[*at + 1].parse::<f32>().ok()? as i32)))
        .collect();
    said.extend(film.points_said.remove(&line.number).unwrap_or_default());
    let mut spliced = false;
    for (at, _) in places.iter().filter(|(_, long)| *long == 1).rev() {
        if let Some((x, y)) = spot_named(film, &line.rest[*at]) {
            line.rest.splice(*at..*at + 1, [x.to_string(), y.to_string()]);
            spliced = true;
        }
    }
    let (mut fitting, mut why) = if spliced { fitting_forms(film, &line) } else { (fitting, why) };
    // (No form fits: a named spot where a form wants numbers, `release a b
    // offshore`, is its point too.)
    if fitting.is_empty() {
        let mut named = line.as_verb(&line.verb);
        let mut any = false;
        for at in (0..named.rest.len()).rev() {
            if film.actor(&named.rest[at]).is_none()
                && let Some((x, y)) = spot_named(film, &named.rest[at])
            {
                named.rest.splice(at..at + 1, [x.to_string(), y.to_string()]);
                any = true;
            }
        }
        if any {
            let (found, _) = fitting_forms(film, &named);
            if !found.is_empty() {
                (line.rest, fitting, why) = (named.rest, found, Vec::new());
            }
        }
    }
    let fitting_groups: Vec<&str> = fitting.iter().map(|(group, _)| *group).collect();
    for (x, y) in said {
        if let Some((name, at_x, at_y)) = named_near(film, x, y) {
            film.remark(line.number, format!("{x} {y} is the place's spot “{name}” ({at_x}, {at_y}): write {name}"));
        }
    }
    if fitting_groups.is_empty() && !why.is_empty() {
        film.wrong(line.number, format!("“{}”: {}", line.text, why.join("; ")));
        return;
    }
    let has = |group: &Group| group.words.iter().any(|word| word.word == line.verb);
    let by_the_engine_only = !fitting_groups.is_empty() && fitting_groups.iter().all(|group| lexicon::ENGINE_GROUPS.contains(group));
    // (Each group is given what its own form read in the line. When the
    // engine's form fits too, the groups whose form does not are not
    // asked: what the fitting ones let go is the engine's, `anim krabby
    // DeepBreath` a game animation once no character has it.)
    let engine_fits = fitting_groups.iter().any(|group| lexicon::ENGINE_GROUPS.contains(group));
    let ordered = film
        .lexicon
        .groups()
        .iter()
        .filter(|group| fitting_groups.contains(&group.name))
        .chain(film.lexicon.groups().iter().filter(|group| !engine_fits && !fitting_groups.contains(&group.name)));
    for group in ordered.into_iter().filter(|group| !by_the_engine_only && has(group)) {
        line.args = fitting.iter().find(|(from, _)| *from == group.name).map(|(_, bound)| bound.clone()).unwrap_or_default();
        if group.plays(film, &line) {
            return;
        }
    }
    if !engine_word(&first) && !engine_word(&line.verb) {
        film.unknown(line.number, "unknown word", &first, film.lexicon.every_word().map(|(_, word, ..)| word), "film/FILMS.md");
    } else if line.verb == "else" && line.rest.is_empty() {
        film.beat(crate::emit::otherwise());
    } else if fitting_groups.is_empty() {
        // (Only the words the language has a form for: `engine.rs`.)
        film.wrong(
            line.number,
            format!("“{}”: a command of the engine the language does not know (its form in film_core/src/verbs/engine.rs, or `>` to debug)", line.text),
        );
    } else {
        // (How long it lasts, as the registry says of the word: its `d`, or
        // until its `who` gets there; the film waits for it unless `&`.)
        let (time, bound) = engine_time(film.lexicon, &line.verb, &fitting).unwrap_or((crate::time::WordTime::Instant, Default::default()));
        // (A time without its unit: frames, as written before.)
        let said_time = bound.values.iter().filter(|value| value.kind == crate::grammar::Kind::Time).filter_map(|value| value.words.first());
        if let Some(said) = said_time.clone().find(|said| crate::words::digits(said) && *said != "0") {
            film.remark(line.number, format!("“{}”: a duration without its unit, in frames; write {}", line.text, crate::words::in_seconds_said(said)));
        }
        let lasting = bound.word("d").map(|said| in_frames(film, said)).and_then(|frames| frames.parse::<i32>().ok()).filter(|frames| *frames > 0);
        let lasts = lasting.filter(|_| time == crate::time::WordTime::Lasts);
        // (Instant, but lasting so long beside the film: its line's end.)
        if let Some(frames) = lasting.filter(|_| time == crate::time::WordTime::Instant) {
            film.plan.handle(crate::time::Length::Known(frames));
        }
        let walker = (time == crate::time::WordTime::Until).then(|| bound.word("who").or(line.rest.first().map(String::as_str)).map(plain)).flatten();
        // (Its words typed by what its form read them as, made into its beat.)
        film.engine_command(&first, &line.rest, Some(&bound));
        // (A move waits for its end, unless the line ends with `&`; a sound,
        // sparks, a shake, a face go on by themselves.)
        if let Some(frames) = lasts {
            film.wait_frames(frames, line.wait);
        }
        if let Some(who) = walker {
            film.wait_for(&who, line.wait);
        }
    }
}

/// What is wrong with a word's line read alone, as a recipe writes it
/// (`text::checked`): its word not known, or none of its forms fitting, and
/// why each does not. None when a form fits; what only the scene knows (who
/// stands where, the things put down) is not asked.
pub(crate) fn wrong_form(film: &mut Film, verb: &str, first: &str, rest: &[String], text: &str) -> Option<String> {
    let line = Line {
        verb: verb.to_string(),
        rest: rest.to_vec(),
        text: text.to_string(),
        number: 0,
        wait: true,
        block: Vec::new(),
        args: Default::default(),
        shape: Shape::Blank,
    };
    let (fitting, why) = fitting_forms(film, &line);
    if !fitting.is_empty() {
        return None;
    }
    if !why.is_empty() {
        return Some(format!("“{text}”: {}", why.join("; ")));
    }
    if !engine_word(first) && !engine_word(verb) {
        let close = crate::check::closest_of(first, film.lexicon.every_word().map(|(_, word, ..)| word));
        return Some(crate::check::unknown("unknown word", first, &close, "film/FILMS.md"));
    }
    (verb != "else" || !rest.is_empty())
        .then(|| format!("“{text}”: a command of the engine the language does not know (its form in film_core/src/verbs/engine.rs, or `>` to debug)"))
}

/// How near a point said in numbers must be to a place's named spot to be
/// told to say its name.
const NAMED_NEAR: i32 = 12;

/// The forms the line fits, by group, each with what it read (the first
/// form of a group that fits); and why the others do not.
fn fitting_forms(film: &mut Film, line: &Line) -> (Vec<(&'static str, crate::grammar::Bound)>, Vec<String>) {
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

/// The named spot of the place at hand by that point, if one: the place's
/// own (the letters of a room's plan, its ways in), the film's (`spot`).
fn named_near(film: &Film, x: i32, y: i32) -> Option<(String, i32, i32)> {
    let place = film.tables.entrances.get(&film.level).into_iter().flatten().cloned();
    let film_own = film.lasting_spots.iter().cloned();
    place
        .chain(film_own)
        .map(|(name, at_x, at_y)| (name, at_x, at_y, (at_x - x).pow(2) + (at_y - y).pow(2)))
        .filter(|(.., far)| *far <= NAMED_NEAR * NAMED_NEAR)
        .min_by_key(|(.., far)| *far)
        .map(|(name, at_x, at_y, _)| (name, at_x, at_y))
}

/// The time of the engine's word a line goes to, as the registry has it
/// (`engine.rs`, the commands' own table), and what its form read there.
fn engine_time(
    lexicon: &'static Lexicon,
    verb: &str,
    fitting: &[(&'static str, crate::grammar::Bound)],
) -> Option<(crate::time::WordTime, crate::grammar::Bound)> {
    lexicon.engine_groups().into_iter().find_map(|group| {
        let word = group.words.iter().find(|word| word.word == verb)?;
        let bound = fitting.iter().find(|(from, _)| *from == group.name)?.1.clone();
        Some((word.timing(), bound))
    })
}

/// A time in seconds (`1.5s`) or in frames (`6f`), as frames (the film's
/// pace, `pace`, is only for `wait`); any other word as it is.
fn in_frames(_film: &Film, word: &str) -> String {
    match crate::words::frames_of(word) {
        Some(frames) if word.ends_with('s') || word.ends_with('f') => frames.to_string(),
        _ => word.to_string(),
    }
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
