//! Recipes: `define name a b` ... `end`, called by its name, its words in place
//! of the names given; `repeat 3` ... `end`. The film's own, and those of the
//! library (`film/recipes/`), which make many of the words of the language
//! out of simpler ones.
//!
//! What a recipe is told, in its first line:
//! - `a`: one word; `a:pokemon`, `a:thing`, `a:duration`, `a:number`, `a:place`
//!   (`x y`, `there`, someone, `front_of b`, a place's word): one of that kind,
//!   told if it is not;
//! - `*a`: those named, one or more (`for each x in *a` plays its lines
//!   for each, `for each x, rank in *a` with his rank; `a.1` the first);
//! - `turns=4`, `turns:number=4`: a word that may be left out, or given by its
//!   name in any place (`turns=6`; `_` for a space); `words...`: all the words
//!   that those after it leave (`words...=Rare_Fossil`);
//! - `off`, `to`, `against`, `via`, `on`, `with`, `in`: that very word; one
//!   that may be left out, or of a choice (`[miss]`, `right|left`), is a name
//!   too, `if miss` telling whether the call said it.
//!
//! Several recipes may have one name: the first that fits is played.
//!
//! A recipe is kept as the language wrote it, and written for each call by
//! `front::called`, with what it was given: a word given stands for itself;
//! `{a}` is the name as the game writes it (`Eevee`); `{where.x}`, `{where.y}` a
//! place's point (found as the film plays, if it is someone); `{y+34}`,
//! `{where.x-20}` a reckoning, each sign as tight as it binds; `let px = …` a
//! number of this call only; `if a` … `end` lines kept only if `a` was given
//! (`if not a`, if not). Called with `&`, a recipe is played beside
//! the film (`beside` ... `end`), and `& as wind` names it, for `wait
//! wind`.
//!
//! Everywhere, `there` is the last point a line was given, and `team` the last
//! team a line named: one line goes on where the one before left off.
//!
//! Each recipe a film writes or uses is checked once, alone, where it is
//! written (`checked`); a line it writes that still fails at a call says
//! which recipe line it comes from.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use assets::project::Project;

use super::front::{CallOf, CallValue, RecipeText};
use crate::film::Film;
use crate::grammar::{self, Kind, Piece, Value as Given};
use crate::ir::{Instr, Shape};
use crate::words::{frames_of, plain};

/// How deep a recipe may call others.
const DEEPEST: usize = 8;

/// The library's recipes, a file a theme.
const LIBRARY: [(&str, &str); 3] = [
    ("gestures", include_str!("../../recipes/gestures.film")),
    ("fights", include_str!("../../recipes/fights.film")),
    ("fishing/line", include_str!("../../recipes/fishing/line.film")),
];

/// The words that are words said, or that carry them: no `there` nor `team`
/// is read in them.
const SAID: [&str; 23] = [
    "message",
    "engraving",
    "popup",
    "log",
    "battlelog",
    "narrate",
    "title_card",
    "sign",
    "bubble",
    "thought",
    "shout",
    "whisper",
    "chapter",
    "place_title",
    "story",
    "notice",
    "mission_posted",
    "chorus",
    "time_skip",
    "the_end",
    "credits",
    "fail",
    "bottom_reached",
];

/// The words of the language, not recipes, that name a team, for `team`.
const TEAMS: [&str; 5] = ["arrival", "trek", "take_stairs", "fireflies", "darkness"];

/// A recipe: what it is told, its lines as the language wrote them
/// (written at each call, `front::called`); where it is from, what it does.
#[derive(Clone, Debug)]
pub struct Recipe {
    pub(crate) form: Vec<Piece>,
    pub(crate) text: Arc<RecipeText>,
    pub name: String,
    pub from: String,
    /// Its file, as told (`gestures.film`; none: the film's own).
    pub file: String,
    pub doc: String,
    /// How it is called, and what it does: for the table of every word.
    pub told_and_doc: String,
}

impl Recipe {
    /// How it is called, to say so.
    pub fn shape(&self) -> String {
        format!("{} {}", self.name, grammar::shown(&self.form))
    }

    /// Whether it is said of someone first (`who`, `who:pokemon`, `*who`).
    pub fn said_of_someone(&self) -> bool {
        crate::order::performer_first(&self.form)
    }

    /// Whether it is told a team (`*who`).
    fn of_a_team(&self) -> bool {
        self.form.iter().any(|piece| matches!(piece, Piece::Team(_)))
    }
}

type Recipes = HashMap<String, Vec<Recipe>>;

/// The library's recipes, read once.
pub fn library() -> &'static [Recipe] {
    static READ: OnceLock<Vec<Recipe>> = OnceLock::new();
    READ.get_or_init(|| LIBRARY.iter().flat_map(|(from, text)| recipes_of(text, from)).collect())
}

/// The recipes of a text (its `define` … `end`), each with the comment
/// above it as what it does.
pub fn recipes_of(text: &str, from: &str) -> Vec<Recipe> {
    let lines = super::front::internal(text, 0, crate::tables::ENGLISH).lines;
    lines
        .iter()
        .filter_map(|item| match &item.shape {
            Shape::Recipe(written) => Some(recipe(written, from)),
            _ => None,
        })
        .collect()
}

/// How far apart the numbers of the lines of each file taken in are.
pub const INCLUDED: usize = 1_000_000;

/// A line's number as the film tells it: a line of a file taken in, the
/// line that takes it in (and the file, and its own line).
pub fn told_line(included: &[(usize, String)], number: usize) -> (usize, Option<(String, usize)>) {
    match included.get((number / INCLUDED).wrapping_sub(1)).filter(|_| number >= INCLUDED) {
        Some((line, name)) => (*line, Some((name.clone(), number % INCLUDED))),
        None => (number, None),
    }
}

/// Where a film's `uses` and `include` look for a file, in turn: the
/// library's, then the project's (its `films/recipes`, its `films`), then
/// the shared library's (`library/films/recipes`, `library/films`)
/// if the project is a game's.
fn shared_folders(project: &Project) -> Vec<std::path::PathBuf> {
    let mut folders = vec![project.root().join("film/recipes")];
    for films in project.folders("films") {
        folders.extend([films.join("recipes"), films]);
    }
    folders
}

/// The files a film may use or take in (`uses`, `include`), by the names
/// it gives them: the library's recipes, then the project's.
pub fn shared_files(project: &Project) -> Vec<String> {
    let mut known: Vec<String> = LIBRARY.iter().map(|(from, _)| from.to_string()).collect();
    for folder in shared_folders(project) {
        let Ok(files) = std::fs::read_dir(folder) else { continue };
        let mut found: Vec<String> = files.flatten().filter_map(|file| file.file_name().to_str()?.strip_suffix(".film").map(str::to_string)).collect();
        found.sort();
        known.extend(found);
    }
    let mut seen = std::collections::HashSet::new();
    known.retain(|name| seen.insert(name.clone()));
    known
}

/// Where a file a film uses is on disk (the library's recipes too).
pub fn shared_path(project: &Project, name: &str) -> Option<std::path::PathBuf> {
    let name = name.trim_end_matches(".film");
    shared_folders(project).into_iter().map(|folder| folder.join(format!("{name}.film"))).find(|path| path.is_file())
}

/// The text of a file a film uses (`uses fishing`, `include intro`): a
/// recipe of the library by its name, or a `.film` of the project's; what
/// to say if there is none.
pub fn shared_text(project: &Project, name: &str) -> Result<String, String> {
    let name = name.trim_end_matches(".film");
    if let Some((_, text)) = LIBRARY.iter().find(|(from, _)| *from == name) {
        return Ok(text.to_string());
    }
    if let Some(text) = shared_path(project, name).and_then(|path| std::fs::read_to_string(path).ok()) {
        return Ok(text);
    }
    // (An optional family's library, `uses dungeon`: all its files of film/recipes/dungeon/.)
    let parts: Vec<&str> =
        LIBRARY.iter().filter(|(from, _)| from.strip_prefix(name).is_some_and(|rest| rest.starts_with('/'))).map(|(_, text)| *text).collect();
    if !parts.is_empty() {
        return Ok(parts.join("\n"));
    }
    // (An optional family whose words are all the language's, `uses guild`: nothing more to take in.)
    if crate::plan::OPTIONAL.iter().any(|(_, library)| *library == name) {
        return Ok(String::new());
    }
    let known = shared_files(project);
    let near = crate::check::closest(name, known.iter().map(String::as_str), 3, 0.4);
    let near = if near.is_empty() { String::new() } else { format!("; near: {}", near.join(", ")) };
    Err(format!("no file “{name}” (neither in film/recipes, nor in library/films){near}"))
}

fn recipe(written: &Arc<RecipeText>, from: &str) -> Recipe {
    let form = grammar::read_recipe(&language::format::elements_text(&written.form.elements));
    let (name, doc) = (plain(&written.form.word.text), written.doc.clone());
    let file = match from {
        "film" => String::new(),
        library if LIBRARY.iter().any(|(known, _)| *known == library) => format!("film/recipes/{library}.film"),
        shared => format!("{shared}.film"),
    };
    let mut recipe = Recipe { form, text: written.clone(), name, from: from.to_string(), file, doc, told_and_doc: String::new() };
    recipe.told_and_doc = format!("`{}`: {}", recipe.shape().trim_end(), recipe.doc);
    recipe
}

/// Where a recipe's line is, to say so: `gestures.film:12`, or `line 12`
/// of the film's own.
pub fn told_at(file: &str, line: usize) -> String {
    if file.is_empty() { format!("line {line}") } else { format!("{file}:{line}") }
}

/// A recipe a film can call, as told: its name, how it is called, what it
/// does, where it is (its file, its line in it).
#[derive(Clone, Debug, PartialEq)]
pub struct Told {
    pub name: String,
    pub shape: String,
    pub doc: String,
    pub from: String,
    pub line: usize,
}

/// The recipes a film has to call: its own (`define`, from its text),
/// those of the files it uses (`uses`), then the library's.
pub fn recipes_for(project: &Project, text: &str) -> Vec<Told> {
    let told = |recipe: &Recipe, from: &str| Told {
        name: recipe.name.clone(),
        shape: recipe.shape().trim_end().to_string(),
        doc: recipe.doc.clone(),
        from: from.to_string(),
        line: recipe.text.number,
    };
    let mut all: Vec<Told> = recipes_of(text, "film").iter().map(|recipe| told(recipe, "")).collect();
    for line in text.lines() {
        if let ["uses" | "include", name] = line.split_whitespace().collect::<Vec<_>>().as_slice()
            && let Ok(shared) = shared_text(project, name)
        {
            all.extend(recipes_of(&shared, name).iter().map(|recipe| told(recipe, name)));
        }
    }
    all.extend(library().iter().filter(|recipe| !crate::verbs::internal(&recipe.name)).map(|recipe| told(recipe, &recipe.from)));
    all
}

/// A line with `there` and `team` put for what they stand for, and noted
/// (`check --all` tells it: a line moved may change what they stand for).
fn shortened(film: &mut Film, number: usize, words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let before = at.checked_sub(1).map(|at| plain(&words[at]));
        match (plain(word).as_str(), film.last_spot) {
            // (`out of there` is not a point.)
            ("there", Some((x, y))) if at > 0 && !matches!(before.as_deref(), Some("of" | "from")) => {
                out.extend([x.to_string(), y.to_string()]);
                film.note(number, format!("“there”: {x} {y}, the last point said"));
            }
            ("team", _) if at > 0 && !film.last_team.is_empty() => {
                out.extend(film.last_team.iter().cloned());
                film.note(number, format!("“team”: {}, the last team named", film.last_team.join(" ")));
            }
            _ => out.push(word.to_string()),
        }
    }
    out
}

/// What a line leaves for the next: its last point.
fn remembered(film: &mut Film, words: &[String]) {
    let numbers: Vec<Option<i32>> = words.iter().map(|word| word.trim_end_matches(',').parse().ok()).collect();
    if let Some(pair) = numbers.windows(2).rev().find_map(|pair| pair[0].zip(pair[1])) {
        film.last_spot = Some(pair);
    }
}

/// The team a line of a word of the language names, if it is one of those
/// that name a team.
fn named_team(film: &mut Film, words: &[String]) {
    if !TEAMS.contains(&plain(&words[0]).as_str()) {
        return;
    }
    let first = words[1..].iter().take_while(|word| word.parse::<i32>().is_ok()).count();
    let team: Vec<String> = words[1 + first..].iter().map(|word| plain(word)).take_while(|word| film.actor(word).is_some()).collect();
    if team.len() > 1 {
        film.last_team = team;
    }
}

/// What a recipe is given, if the words fit it (by the one reader,
/// `grammar`): each name, its words, a time as frames (`90f`); else why not.
fn given(film: &mut Film, recipe: &Recipe, said: &[String]) -> Result<Vec<Given>, String> {
    let mut bound = grammar::bind(film, &recipe.form, said)?.values;
    for value in bound.iter_mut().filter(|value| value.kind == Kind::Time) {
        if let Some(frames) = value.words.first().and_then(|word| frames_of(word)) {
            value.words = vec![format!("{frames}f")];
        }
    }
    Ok(bound)
}

/// A recipe's lines, written for this call with what it was given (by
/// `front`, from what the language wrote); what is wrong in them told on
/// the line that calls it.
fn cooked(film: &mut Film, recipe: &Recipe, given: &[Given], number: usize) -> Vec<Instr> {
    film.recipe_calls += 1;
    // (As the game writes them, only those the recipe says so: `{who}`.)
    let written = &recipe.text.text[recipe.text.span.start..recipe.text.span.end];
    let values = given
        .iter()
        .map(|one| {
            let said = written.contains(&format!("{{{}}}", one.name));
            CallValue { name: one.name.clone(), words: one.words.clone(), shown: if said { shown(film, &one.words) } else { one.words.join(" ") } }
        })
        .collect();
    let call = CallOf { id: film.recipe_calls, number, values, file: recipe.file.clone(), apart: false };
    let read = super::front::called(&recipe.text, call, &film.tables.language);
    let source = language::Source::new(&recipe.text.text);
    for (_, wrong) in read.diagnostics.into_iter().filter(|(_, wrong)| wrong.severity == language::Severity::Error) {
        let at = told_at(&recipe.file, source.position(wrong.bytes().start).line);
        film.wrong(number, format!("in the recipe “{}” ({at}): {}", recipe.name, wrong.message));
    }
    read.lines
}

/// Words a recipe is given, as the game writes them in words said: someone
/// by his name (`Eevee`), a team as one says it (`Eevee, Pikachu and
/// Piplup`), other words as they are.
fn shown(film: &mut Film, words: &[String]) -> String {
    let named: Vec<bool> = words.iter().map(|word| film.actor(word).is_some()).collect();
    let names: Vec<String> =
        words.iter().zip(&named).map(|(word, named)| if *named { crate::verbs::dungeon::called_by(film, word) } else { word.clone() }).collect();
    match names.as_slice() {
        [before @ .., last] if named.iter().all(|named| *named) && !before.is_empty() => format!("{} and {last}", before.join(", ")),
        _ => names.join(" "),
    }
}

/// `... & as wind` at the end of a line: the name, the line without it.
fn named_beside(words: &mut Vec<String>) -> Option<String> {
    let long = words.len();
    if long >= 3 && words[long - 3] == "&" && plain(&words[long - 2]) == "as" {
        let name = words.pop();
        words.pop();
        words.pop();
        return name;
    }
    None
}

/// A line of the film as the next one reads it: `there` and `team` put for
/// what they stand for in a word's words, an engine's fields and the lines
/// an anchor or a fight's moment holds (never in words said); what it
/// leaves for the next (its last point, the team it names). The line as it
/// was where nothing is put in it.
fn read_on(film: &mut Film, instr: &Instr) -> Instr {
    match &instr.shape {
        Shape::Word { verb, words, wait } if !said_word(verb, &word_line(instr, verb, words, *wait)) => {
            let line = word_line(instr, verb, words, *wait);
            let written = shortened(film, instr.number, &line);
            named_team(film, &written);
            remembered(film, &written);
            if written == line { instr.clone() } else { as_word(instr, written) }
        }
        Shape::Engine(fields) => {
            let written = shortened(film, instr.number, fields);
            remembered(film, &written);
            if written == *fields {
                instr.clone()
            } else {
                Instr { shape: Shape::Engine(written.clone()), text: format!("> {}", written.join(" ")), span: None, ..instr.clone() }
            }
        }
        Shape::Arrives { who, then } => {
            let who = shortened(film, instr.number, &[vec!["when".to_string()], who.clone()].concat())[1..].to_vec();
            let then = then.as_ref().map(|then| Box::new(read_on(film, then)));
            let shape = Shape::Arrives { who, then };
            if shape == instr.shape { instr.clone() } else { Instr { shape, span: None, ..instr.clone() } }
        }
        Shape::AtMark { name, after, then } => {
            let then = then.as_ref().map(|then| Box::new(read_on(film, then)));
            let shape = Shape::AtMark { name: name.clone(), after: *after, then };
            if shape == instr.shape { instr.clone() } else { Instr { shape, span: None, ..instr.clone() } }
        }
        Shape::WhenLife { who, left, lines } => {
            let lines: Vec<Instr> = lines.iter().map(|line| read_on(film, line)).collect();
            let shape = Shape::WhenLife { who: who.clone(), left: *left, lines };
            if shape == instr.shape { instr.clone() } else { Instr { shape, span: None, ..instr.clone() } }
        }
        _ => instr.clone(),
    }
}

/// Whether a word's line is words said (`???: La la la...`), or a word
/// that carries them: no `there` nor `team` is read in it.
fn said_word(verb: &str, line: &[String]) -> bool {
    SAID.contains(&verb) || line.iter().take(3).any(|word| word.ends_with(':'))
}

/// A word's line as its words: the word as written, its words, `&` if the
/// film goes on meanwhile.
fn word_line(instr: &Instr, verb: &str, words: &[String], wait: bool) -> Vec<String> {
    let first = instr.text.split_whitespace().next().filter(|first| plain(first) == verb).unwrap_or(verb).to_string();
    let mut line: Vec<String> = std::iter::once(first).chain(words.iter().cloned()).collect();
    if !wait {
        line.push("&".to_string());
    }
    line
}

/// A word's line made again from its words (`&` at the end: beside the film).
fn as_word(instr: &Instr, mut written: Vec<String>) -> Instr {
    let wait = written.last().is_none_or(|last| last != "&");
    if !wait {
        written.pop();
    }
    let mut made = Instr::word(instr.number, &written[0], &written[1..], wait);
    // (Still the line as written, if it reads the same.)
    made.span = instr.span.filter(|_| made.text == instr.text);
    made
}

/// The most times `repeat` plays its lines, and the most lines a film
/// becomes once its recipes and repeats are played out: past them, a
/// mistake (`repeat 2147483647`) is told, not attempted.
const MOST_REPEATS: usize = 1000;
const MOST_LINES: usize = 200_000;

fn play(lines: &[Instr], depth: usize, recipes: &mut Recipes, film: &mut Film, out: &mut Vec<Instr>) {
    for instr in lines.iter().filter(|instr| instr.shape != Shape::Blank) {
        let number = instr.number;
        // (`define …`: the film's own recipe, from here on, checked once
        // with the others; one of a file taken in, told of that file.)
        if let Shape::Recipe(written) = &instr.shape {
            let mut recipe = recipe(written, "film");
            if let (_, Some((name, _))) = told_line(&film.included, written.number) {
                recipe.file = format!("{name}.film");
            }
            if !film.defined.iter().any(|known| Arc::ptr_eq(&known.recipe.text, written)) {
                film.defined.push(super::checked::Defined { recipe: recipe.clone(), uses: None });
            }
            recipes.entry(recipe.name.clone()).or_default().insert(0, recipe);
            continue;
        }
        if let Some(inner) = &instr.inner {
            match instr.verb() {
                Some("repeat") => {
                    let count: usize = instr.words().first().and_then(|count| count.parse().ok()).unwrap_or(1);
                    if count > MOST_REPEATS {
                        film.wrong(number, format!("“{}”: {count} times, at most {MOST_REPEATS}", instr.text));
                        continue;
                    }
                    for _ in 0..count {
                        play(inner, depth + 1, recipes, film, out);
                        if out.len() > MOST_LINES {
                            film.wrong(number, format!("“{}”: the film would be more than {MOST_LINES} lines", instr.text));
                            break;
                        }
                    }
                }
                // (A track's keys: lines of their own, no recipe's.)
                Some("keys") => {
                    let mut block = read_on(film, instr);
                    block.inner = Some(inner.clone());
                    out.push(block);
                }
                // (Another block: its opening read on (`there`, `team`), its
                // lines played in it.)
                _ => {
                    let mut block = read_on(film, instr);
                    let mut held = Vec::new();
                    play(inner, depth, recipes, film, &mut held);
                    block.inner = Some(held);
                    out.push(block);
                }
            }
            continue;
        }
        let Shape::Word { verb, words, wait } = &instr.shape else {
            out.push(read_on(film, instr));
            continue;
        };
        // (Who first, a recipe of a file taken in, `krabby reunion piplup`:
        // read as its word first, as the language's own are.)
        let swapped: (String, Vec<String>);
        let (verb, words) = match words.first() {
            Some(second)
                if !recipes.contains_key(verb.as_str())
                    && film.actor(verb).is_some()
                    && recipes.get(&plain(second)).is_some_and(|known| known.iter().any(Recipe::said_of_someone)) =>
            {
                swapped = (plain(second), std::iter::once(verb.clone()).chain(words[1..].iter().cloned()).collect());
                (&swapped.0, &swapped.1)
            }
            _ => (verb, words),
        };
        // (`for ds`: what the film is written for, told once it is read.)
        if verb == "for" && words.first().is_some_and(|word| word == "ds") {
            film.for_ds = Some(crate::ds::Mode { boot: words.len() == 1, line: number });
            continue;
        }
        // (`uses fishing`: the recipes of a file; `include intro`: all its
        // lines, here, as if written on this one.)
        if let ("uses" | "include", [name]) = (verb.as_str(), words.as_slice()) {
            match shared_text(&film.tables.project.clone(), name) {
                Err(told) => film.wrong(number, told),
                Ok(_) if depth > 8 => film.wrong(number, format!("“{}”: files that include each other without end", instr.text)),
                Ok(text) if verb == "uses" => {
                    film.uses.push(plain(name));
                    for recipe in recipes_of(&text, name) {
                        if !film.defined.iter().any(|known| known.recipe.file == recipe.file && known.recipe.text.number == recipe.text.number) {
                            film.defined.push(super::checked::Defined { recipe: recipe.clone(), uses: Some(number) });
                        }
                        recipes.entry(recipe.name.clone()).or_default().insert(0, recipe);
                    }
                }
                Ok(text) => {
                    // (Its lines by their own numbers, in a range of their own: what
                    // is wrong in them is told of the file and its line.)
                    film.included.push((number, name.to_string()));
                    let base = INCLUDED * film.included.len();
                    let lines = read(film, &text, base);
                    play(&lines, depth + 1, recipes, film, out);
                }
            }
            continue;
        }
        // (Words said, kept as they are; a recipe of that word still plays them.)
        let line = word_line(instr, verb, words, *wait);
        let said = said_word(verb, &line);
        let known = recipes.get(verb).filter(|_| depth < DEEPEST).cloned();
        if said && !recipes.contains_key(verb) {
            out.push(instr.clone());
            continue;
        }
        let mut written = if said { line.clone() } else { shortened(film, number, &line) };
        let name = named_beside(&mut written);
        let meanwhile = name.is_some() || written.last().is_some_and(|last| last == "&");
        match known {
            Some(known) => {
                if written.last().is_some_and(|last| last == "&") {
                    written.pop();
                }
                let mut why = Vec::new();
                let mut fits = None;
                for recipe in &known {
                    match given(film, recipe, &written[1..]) {
                        Ok(given) => {
                            fits = Some((recipe.clone(), given));
                            break;
                        }
                        Err(reason) => why.push(format!("“{}”: {reason}", recipe.shape())),
                    }
                }
                match fits {
                    Some((recipe, given)) => {
                        // (A recipe called by a line of the text, as written:
                        // its fields are what the recipe is given.)
                        let as_in_the_text = instr.span.is_some() && written.iter().eq(line.iter().filter(|word| *word != "&"));
                        // (Written the word first, said of someone: who first now, `pikachu hug piplup`.)
                        let one_first = match recipe.form.first() {
                            Some(Piece::Team(name)) => given.iter().find(|one| one.name == *name).is_some_and(|one| one.words.len() == 1),
                            _ => recipe.said_of_someone(),
                        };
                        if as_in_the_text && instr.from.0.is_none() {
                            crate::plan::asks_its_library(film, number, verb);
                        }
                        if as_in_the_text && instr.from.0.is_none() && one_first {
                            let words = film.written_words(instr);
                            crate::order::told_who_first(film, number, &words, verb);
                        }
                        if let Some(bound) = as_in_the_text.then(|| grammar::bind(film, &recipe.form, &written[1..]).ok()).flatten() {
                            let read = crate::compile::ReadAs { word: verb.clone(), family: "recipe", values: bound.values };
                            film.read_as.entry(number).or_insert(read);
                        }
                        // (Its places given in two numbers, to be told if they have a name.)
                        for one in given.iter().filter(|one| one.kind == Kind::Place) {
                            if let [x, y] = one.words.as_slice()
                                && let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>())
                            {
                                film.points_said.entry(number).or_default().push((x as i32, y as i32));
                            }
                        }
                        // (A recipe of a team: those it is called with, all.)
                        if recipe.of_a_team() {
                            let named: Vec<String> = written[1..].iter().map(|word| plain(word)).take_while(|word| film.actor(word).is_some()).collect();
                            if named.len() > 1 {
                                film.last_team = named;
                            }
                        }
                        let lines = cooked(film, &recipe, &given, number);
                        // (With `&`: all of it beside the film, a block of its own.)
                        if meanwhile {
                            let mut held = Vec::new();
                            play(&lines, depth + 1, recipes, film, &mut held);
                            out.push(beside_block(number, &name, held));
                        } else {
                            play(&lines, depth + 1, recipes, film, out);
                        }
                    }
                    // (No recipe of that name fits, but a word of the language does: the word's.)
                    None if crate::verbs::is_word(verb, "") && crate::verbs::wrong_form(film, verb, verb, &written[1..], &instr.text).is_none() => {
                        out.push(if written == line { instr.clone() } else { as_word(instr, written) });
                    }
                    None => {
                        let from = instr.from.0.as_ref().map_or(String::new(), |from| format!(" ({})", from.told()));
                        film.wrong(number, format!("“{}”: {}{from}", instr.text, why.join("; ")));
                    }
                }
            }
            None => {
                named_team(film, &written);
                remembered(film, &written);
                let made = if written == line { instr.clone() } else { as_word(instr, written) };
                if name.is_some() {
                    out.push(beside_block(number, &name, vec![made]));
                } else {
                    out.push(made);
                }
            }
        }
    }
}

/// A text read by the language (`front`), its numbers from `base`: its
/// tree of lines, what is wrong told.
fn read(film: &mut Film, text: &str, base: usize) -> Vec<Instr> {
    let read = super::front::internal(text, base, &film.tables.language);
    for (number, wrong) in read.diagnostics {
        if wrong.severity == language::Severity::Error {
            let fix = wrong.fixes.first().map_or(String::new(), |fix| format!(" ({})", fix.title));
            film.wrong(number, format!("{}{fix}", wrong.message));
        }
    }
    let mut lines = read.lines;
    // (The film's own text, not a file taken in: each line as written there
    // is told where it stands.)
    if base == 0 {
        own_spans(&mut lines, text);
    }
    lines
}

/// Each line of the tree written in the text as it stands, its range there
/// (a `&` at its end, said beside, the same line; written who first,
/// `pikachu walk to b`, the same line as the word first reads it).
fn own_spans(lines: &mut [Instr], text: &str) {
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(at, _)| at + 1));
    let bare = |line: &str| line.trim().trim_end_matches([' ', '&']).split_whitespace().collect::<Vec<&str>>().join(" ");
    let swapped = |line: &str| {
        let mut words: Vec<&str> = line.split_whitespace().collect();
        if words.len() > 1 {
            words.swap(0, 1);
        }
        words.join(" ")
    };
    for line in lines {
        if let Some(&start) = line.number.checked_sub(1).and_then(|at| starts.get(at)) {
            let end = text[start..].find('\n').map_or(text.len(), |length| start + length);
            let (written, read) = (bare(&text[start..end]), bare(&line.text));
            if written == read || swapped(&written) == read {
                line.span = Some(language::Span::new(start, end));
            }
        }
        if let Some(inner) = &mut line.inner {
            own_spans(inner, text);
        }
    }
}

/// One line a word makes for the language to play, the library's recipes
/// played out in it (`evolution a m133`, of `evolve`): its lines.
pub fn unfold_word(film: &mut Film, line: Instr) -> Vec<Instr> {
    let mut recipes = Recipes::new();
    for recipe in library() {
        recipes.entry(recipe.name.clone()).or_default().push(recipe.clone());
    }
    let mut out = Vec::new();
    play(&[line], 0, &mut recipes, film, &mut out);
    out
}

/// The film's lines with its recipes played out, and the library's: its
/// tree of lines, each with its number; the empty and the comments left out.
pub fn unfold(text: &str, film: &mut Film) -> Vec<Instr> {
    let numbered = read(film, text, 0);
    let mut recipes = Recipes::new();
    for recipe in library() {
        recipes.entry(recipe.name.clone()).or_default().push(recipe.clone());
    }
    let mut out = Vec::new();
    play(&numbered, 0, &mut recipes, film, &mut out);
    out
}

/// Lines played beside the film (`beside [name]` … `end`), named if they are.
fn beside_block(number: usize, name: &Option<String>, lines: Vec<Instr>) -> Instr {
    let named: Vec<String> = name.iter().cloned().collect();
    Instr { inner: Some(lines), ..Instr::word(number, "beside", &named, true) }
}
