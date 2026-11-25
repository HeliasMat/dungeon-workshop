//! What is done with a film by its name: `scene_player check example`.
//! Every command reads the same way: its word, what it works on, then what
//! it asks, then options. It says what it wrote or found, and fails aloud.

mod agents;
mod checking;
mod chrono;
mod ds;
mod dungeons;
mod help;
pub mod json;
mod language;
mod looking;
mod objects;
mod pictures;
mod places;
mod reading;
mod renaming;
mod resources;
mod server;
mod sounds;
mod sprites;

use std::path::{Path, PathBuf};

use std::sync::Arc;

use ::film::check::closest;
use ::film::locate;
use ::film::tables::Tables;
use ::film::{Context, Written};

use preview::art::Art;
use preview::film::Film;

pub use help::known;
use help::{COMMANDS, Command, help};

/// What a command was told.
pub struct Asked<'a> {
    pub words: Vec<&'a String>,
    options: Vec<(&'a str, Option<&'a String>)>,
}

impl<'a> Asked<'a> {
    /// A command's words and options, or what is wrong with them.
    fn read(command: &Command, rest: &'a [String]) -> Result<Self, String> {
        let mut asked = Asked { words: Vec::new(), options: Vec::new() };
        let mut rest = rest.iter();
        while let Some(word) = rest.next() {
            match word.strip_prefix("--") {
                Some(_) if command.flags.contains(&word.as_str()) => asked.options.push((word, None)),
                Some(_) if command.values.contains(&word.as_str()) => {
                    let value = rest.next().ok_or(format!("{word}: what value? scene_player help {}", command.word))?;
                    asked.options.push((word, Some(value)));
                }
                Some(_) => return Err(format!("{}: no option {word}. scene_player help {}", command.word, command.word)),
                None => asked.words.push(word),
            }
        }
        if asked.words.len() < command.least {
            return Err(format!("{}: {} is missing. scene_player help {}", command.word, command.takes, command.word));
        }
        Ok(asked)
    }

    fn first(&self) -> &'a str {
        self.words.first().map_or("", |word| word.as_str())
    }

    fn has(&self, flag: &str) -> bool {
        self.options.iter().any(|(word, _)| *word == flag)
    }

    fn value(&self, option: &str) -> Option<&'a String> {
        self.options.iter().find(|(word, _)| *word == option).and_then(|(_, value)| *value)
    }

    /// `--chapter n`: the chapter a game is played from.
    fn chapter(&self) -> Result<Option<u16>, String> {
        self.value("--chapter").map(|value| value.parse().map_err(|_| format!("--chapter: a chapter's number, not “{value}”"))).transpose()
    }

    fn number(&self, option: &str, usual: usize) -> Result<usize, String> {
        match self.value(option) {
            Some(value) => value.parse().map_err(|_| format!("{option}: a number, not “{value}”")),
            None => Ok(usual),
        }
    }
}

/// A film, or a file of scenes, as a command takes it.
pub struct Piece {
    pub path: String,
    pub name: String,
    pub written: Option<Written>,
    /// Its text (none for an event of the game's story).
    pub text: String,
    /// Given as a text, not a file (`server`, `load text`).
    pub given: bool,
    /// What it is compiled and played with: its project's tables (their
    /// project, their data folder).
    pub tables: Arc<Tables>,
}

impl Piece {
    fn find(context: &Context, name: &str) -> Result<Self, String> {
        let tables = context.tables();
        // (An event of the game's story, as the game plays it: `read:84`.)
        if let Some(event) = preview::film::story_event(name) {
            return Ok(Piece { path: name.to_string(), name: format!("read_{event}"), written: None, text: String::new(), given: false, tables });
        }
        let path = locate(&context.library, name).ok_or_else(|| lost(context, name))?;
        let film = path.extension().is_some_and(|kind| kind == "film");
        let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let written = if film { Some(::film::compile_with(&text, tables.clone())) } else { None };
        Ok(Piece { name: stem(&path), path: path.to_string_lossy().into_owned(), written, text, given: false, tables })
    }

    /// A film given as its text, by that name.
    pub fn of_text(context: &Context, text: &str, name: &str) -> Self {
        let path = context.library.films().join(format!("{name}.film")).to_string_lossy().into_owned();
        let tables = context.tables();
        Piece { path, name: name.to_string(), written: Some(::film::compile_with(text, tables.clone())), text: text.to_string(), given: true, tables }
    }

    /// Its project's art, read from its data folder.
    pub fn art(&self) -> Result<Art, String> {
        Art::open_in(&self.tables.data, self.tables.project.clone())
    }

    fn film(&self) -> Result<Film, String> {
        self.film_with(self.art()?, None)
    }

    /// What the debug view says of it: its names, its text (see `preview::debug`).
    pub fn known(&self, film: &mut Film) {
        let names = self.written.as_ref().map_or(&[][..], |written| written.names.as_slice());
        film.debug.know(names, &self.text);
    }

    /// Ready to play with that art, heard by that.
    pub fn film_with(&self, art: Art, audio: Option<Box<dyn shell::sound::Hears>>) -> Result<Film, String> {
        let mut film = self.film_bare(art, audio)?;
        self.known(&mut film);
        Ok(film)
    }

    fn film_bare(&self, art: Art, audio: Option<Box<dyn shell::sound::Hears>>) -> Result<Film, String> {
        match (&self.written, self.given) {
            (Some(written), true) => {
                if let Some(problem) = written.problems.iter().find(|problem| !problem.is_remark()) {
                    return Err(format!("{}: {problem}", self.name));
                }
                let mut film = Film::of_written(written, art).map_err(|failed| format!("{} : {}", self.name, failed.1))?;
                film.audio = audio;
                Ok(film)
            }
            _ => Film::open(&self.path, art, audio, None, self.tables.clone()),
        }
    }

    /// The engine's words, those it names (typed) by the names the film gives.
    fn says(&self, text: &str, mentions: &[engine::Mention]) -> String {
        self.written.as_ref().map_or(text.to_string(), |written| written.by_name(text, mentions))
    }

    /// A problem as the engine tells it, by the names the film gives.
    #[cfg(test)]
    fn told(&self, problem: &engine::Diagnostic) -> String {
        self.written.as_ref().map_or(problem.to_string(), |written| written.told_by_name(problem))
    }
}

/// Where to look for a film that is not there.
fn lost(context: &Context, name: &str) -> String {
    let films: Vec<String> = std::fs::read_dir(context.library.films()).into_iter().flatten().flatten().map(|file| stem(&file.path())).collect();
    let close = closest(name, films.iter().map(String::as_str), 4, 0.5);
    let close = if close.is_empty() { String::new() } else { format!(" Near: {}.", close.join(", ")) };
    format!("film not found: {name}.{close} A path, or a name in library/films/.")
}

fn stem(path: &Path) -> String {
    path.file_stem().map_or(String::new(), |name| name.to_string_lossy().into_owned())
}

/// `target/films/<name>/`, made.
fn out_folder(context: &Context, name: &str) -> Result<PathBuf, String> {
    let folder = context.root().join("target/films").join(name);
    std::fs::create_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    Ok(folder)
}

/// A command by its word; `debug`: the developer's options a film or a
/// game in a window starts with (`DUNGEON_FILM_DEBUG`, read by `main`).
pub fn run(word: &str, rest: &[String], context: &Arc<Context>, debug: &preview::debug::Debug) -> Result<(), String> {
    let command = COMMANDS.iter().find(|command| command.word == word).ok_or_else(|| help::unknown(word))?;
    let asked = Asked::read(command, rest)?;
    match word {
        "help" => help(asked.words.first().map(|word| word.as_str())),
        "search" => places::search(context, &asked.words),
        "place" => places::describe(context, asked.first()),
        "illustrations" => places::show_images(context, &asked.words),
        "songs" => sounds::songs(&asked.words, context),
        "sounds" => sounds::noises(&asked.words, context),
        "listen" => sounds::listen(asked.first(), context),
        "items" => objects::list(asked.words.first().map_or("", |word| word.as_str()), context),
        "dungeons" => dungeons::prepare(context),
        "spritecollab" => sprites::spritecollab(context, asked.first(), asked.has("--replace")),
        "grid" => places::grid(context, asked.first()),
        "ground" => places::ground(context, asked.first()),
        "check" if asked.has("--json") => checking::check_json(&asked.words, context, !asked.has("--no-play")),
        "check" if asked.has("--no-play") => checking::read_only(&asked.words, context),
        "check" => checking::check(&asked.words, context, asked.has("--all")),
        "state" => looking::state(&Piece::find(context, asked.first())?, &owned(&asked.words[1..]), asked.has("--json")),
        "timing" => {
            let why = asked.value("--why").map(|line| line.parse().map_err(|_| format!("--why: a line number, not “{line}”"))).transpose()?;
            let told = chrono::Asked { why, unplayed: asked.has("--no-play") };
            chrono::chrono(&Piece::find(context, asked.first())?, &owned(&asked.words[1..]), asked.has("--json"), told)
        }
        "properties" => agents::properties(asked.words.first().map(|word| word.as_str()), asked.has("--json")),
        "schema" => agents::schema(asked.has("--json")),
        "resources" => resources::resources(asked.words.first().map(|word| word.as_str()), context, asked.has("--json")),
        "server" => server::serve(context),
        "measure" => looking::measure(&Piece::find(context, asked.first())?, asked.number("--size", 4)?),
        "view" => pictures::view(&Piece::find(context, asked.first())?, asked.number("--scene", 0)?, asked.number("--every", 4)?, context),
        "image" => pictures::image(&Piece::find(context, asked.first())?, &owned(&asked.words[1..]), context),
        "try" => pictures::try_out(&Piece::find(context, asked.first())?, &owned(&asked.words[1..]), asked.value("--on"), context),
        "play" if asked.first().ends_with(".dwpack") => {
            let game = game::pack::open_pack(context.root(), Path::new(asked.first()))?;
            let chapter = asked.chapter()?;
            pictures::play_game(&game, asked.has("--headless"), asked.has("--new"), chapter, asked.has("--auto"), asked.number("--size", 4)?, context, debug)
        }
        "check-game" => checking::check_game(asked.first(), context),
        "play" if asked.first().ends_with(".game") => {
            let chapter = asked.chapter()?;
            pictures::play_game(
                Path::new(asked.first()),
                asked.has("--headless"),
                asked.has("--new"),
                chapter,
                asked.has("--auto"),
                asked.number("--size", 4)?,
                context,
                debug,
            )
        }
        // (A game by its name, `games/<name>/<name>.game`, when no film has it.)
        "play"
            if !asked.first().ends_with(".film") && locate(&context.library, asked.first()).is_none() && context.library.find_game(asked.first()).is_some() =>
        {
            let chapter = asked.chapter()?;
            let path = context.library.find_game(asked.first()).unwrap_or_default();
            pictures::play_game(&path, asked.has("--headless"), asked.has("--new"), chapter, asked.has("--auto"), asked.number("--size", 4)?, context, debug)
        }
        "play" if asked.has("--chapter") => Err("--chapter: a game's (a .game or a .dwpack), not a film's".to_string()),
        "play" => pictures::play(&Piece::find(context, asked.first())?, asked.has("--auto"), asked.value("--from"), asked.number("--size", 4)?, context, debug),
        "video" => pictures::video(&Piece::find(context, asked.first())?, asked.number("--size", 4)?, context),
        "format" => language::format(context, &owned(&asked.words)),
        "ids" => language::ids(context, &owned(&asked.words)),
        "analyze" => language::analyze(context, asked.first(), asked.has("--json")),
        "steps" => language::steps(context, asked.first()),
        "cast" => reading::cast(context, asked.first(), asked.has("--json")),
        "usages" => reading::usages(context, asked.first(), asked.number("--max", 40)?, asked.has("--json")),
        "rename" => renaming::rename(context, asked.first(), asked.words[1], asked.words[2], asked.has("--try")),
        "diff" => reading::diff(context, asked.first(), asked.value("--from").map(|rev| rev.as_str()), asked.has("--json")),
        "fingerprint" => language::fingerprint(context, asked.has("--compare"), asked.has("--fields")),
        "pack" => pictures::pack(&Piece::find(context, asked.first())?, context),
        "pack-game" => pictures::pack_game(asked.first(), context),
        "export-ds" => ds::export(context, asked.first(), asked.value("--out"), asked.has("--table"), asked.has("--partial"), asked.has("--tall")),
        other => Err(format!("“{other}” is in the list of commands but nothing does it")),
    }
}

fn owned(words: &[&String]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

/// What the tests run with: the repository's root, a context on a data folder.
#[cfg(test)]
pub(crate) mod testing {
    use std::path::PathBuf;
    use std::sync::Arc;

    /// The repository's root (the tests run from `scene_player/`).
    pub fn root() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// The repository's library on that data folder.
    pub fn tested(data: &str) -> Arc<::film::Context> {
        Arc::new(::film::Context::new(game::Project::library(root()), data))
    }
}
