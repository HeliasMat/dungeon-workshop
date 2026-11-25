//! What goes wrong opening, reading, building or packing a game, as the
//! programs that use this block meet it: typed, and said as it always was
//! (`Display`). A program turns it into its own words with `to_string()`, or
//! `?` into a `String`.

use std::fmt;
use std::path::PathBuf;

use language::Diagnostic;

/// What went wrong.
#[derive(Debug)]
pub enum Error {
    /// A file not read or not written: its path, why.
    File { path: String, why: std::io::Error },
    /// Something not written, nowhere in particular.
    Io(std::io::Error),
    /// A file of the game's that does not read (a place: its line said).
    Read(Diagnostic),
    /// A place that cannot be built: its file, why.
    NotBuilt { file: String, why: Diagnostic },
    /// A file of the data not found (`dungeons.bin`): its path.
    NotFound(String),
    /// A file of the data found and not read: which, why.
    Unreadable { what: &'static str, why: String },
    /// The rules' book, `dungeons.bin`, missing from the data.
    NoBook,
    /// The rules' book there, and not read.
    BookUnread,
    /// A file a pack refuses, as it is named (`from`, `word`): it is the
    /// game's data, `there`.
    FromTheData { from: String, word: String, there: PathBuf },
    /// A picture or a sound named by a path outside the game's folder.
    Outside { from: String, word: String },
    /// A game's file with no name.
    Nameless,
    /// A pack, and what is wrong with it.
    Pack { pack: PathBuf, wrong: PackWrong },
    /// No game at that path (as it is shown).
    NoGame(String),
    /// A game's file that does not read: the file, its first problem.
    Wrong { file: String, first: Diagnostic },
    /// A chapter the game does not have: the game, the one asked, those it has.
    NoChapter { game: String, number: u16, known: Vec<String> },
    /// What cannot be done now, and why.
    NotNow(&'static str),
    /// What another block said (a film, a game played), in its words.
    Said(String),
}

/// What is wrong with a pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackWrong {
    /// Neither tar nor zip could zip it.
    NotZipped,
    /// No file there.
    Missing,
    /// Neither tar nor unzip could unpack it.
    NotUnpacked,
    /// No manifest in it.
    NoManifest,
    /// Made by a later version: the version of its format.
    Later(u32),
    /// Its manifest names no game: what it names.
    NoGame(String),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::File { path, why } => write!(out, "{path}: {why}"),
            Error::Io(why) => write!(out, "{why}"),
            Error::Read(fault) => write!(out, "{fault}"),
            Error::NotBuilt { file, why } => write!(out, "{file}: cannot be built ({why})"),
            Error::NotFound(path) => write!(out, "{path}: not found"),
            Error::Unreadable { what, why } => write!(out, "{what} unreadable: {why}"),
            Error::NoBook => write!(out, "dungeons.bin introuvable"),
            Error::BookUnread => write!(out, "dungeons.bin illisible"),
            Error::FromTheData { from, word, there } => write!(
                out,
                "{from}: “{word}” comes from the game's data ({}): a pack only holds the game's own files. Draw or record one of your own (images/, sounds/) or name the game's by its name",
                there.display()
            ),
            Error::Outside { from, word } => {
                write!(out, "{from}: “{word}” is a picture or a sound outside the game's folder: put it in its images/ (or sounds/) and name it from there")
            }
            Error::Nameless => write!(out, "a game's file has a name"),
            Error::Pack { pack, wrong } => {
                let pack = pack.display();
                match wrong {
                    PackWrong::NotZipped => write!(out, "{pack}: could not be zipped (tar with zip, or zip, is needed)"),
                    PackWrong::Missing => write!(out, "{pack}: no such pack"),
                    PackWrong::NotUnpacked => write!(out, "{pack}: could not be unpacked (tar or unzip is needed)"),
                    PackWrong::NoManifest => write!(out, "{pack}: not a game's pack (no {})", crate::manifest::MANIFEST),
                    PackWrong::Later(format) => {
                        write!(out, "{pack}: a pack of a later version ({format}; this one reads {}): a newer scene_player is needed", crate::manifest::FORMAT)
                    }
                    PackWrong::NoGame(game) => write!(out, "{pack}: its manifest names no game (“game: {game}”)"),
                }
            }
            Error::NoGame(path) => write!(out, "{path}: no such game (a .game file: its title, team, chapters)"),
            Error::Wrong { file, first } => write!(out, "{file}: {first}"),
            Error::NoChapter { game, number, known } => write!(out, "{game}: no chapter {number} (its chapters: {})", known.join(", ")),
            Error::NotNow(why) => write!(out, "{why}"),
            Error::Said(said) => write!(out, "{said}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::File { why, .. } | Error::Io(why) => Some(why),
            Error::Read(why) | Error::NotBuilt { why, .. } => Some(why),
            _ => None,
        }
    }
}

impl Error {
    /// A file not read or not written.
    pub fn file(path: &std::path::Path, why: std::io::Error) -> Error {
        Error::File { path: path.display().to_string(), why }
    }
}

impl From<String> for Error {
    fn from(said: String) -> Error {
        Error::Said(said)
    }
}

impl From<std::io::Error> for Error {
    fn from(why: std::io::Error) -> Error {
        Error::Io(why)
    }
}

/// As it is said, for a program that speaks in `String`s.
impl From<Error> for String {
    fn from(error: Error) -> String {
        error.to_string()
    }
}
