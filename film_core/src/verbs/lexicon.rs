//! The words of the language gathered: the groups of words, the narrower
//! first, as the one who assembles the language lists them (`film`), between
//! the text's own words and the engine's; and what is read once of them, the
//! library's recipes (`film/recipes/`), the engine's commands no word names,
//! the names of every word. A film carries the lexicon it is read with
//! (`Film::lexicon`); what is asked of the language without a film is asked
//! of a lexicon.

use std::collections::HashSet;
use std::sync::OnceLock;

use super::{Group, Word, commands, engine};
use std::sync::Arc;

use crate::ds::OnDs;
use crate::grammar::{self, Piece};
use crate::ir::RecipeText;

/// The text's own words and the engine's: forms, never played here.
const TEXT: Group = Group::new("text", TEXT_WORDS);
const ENGINE: Group = Group::new("engine", engine::WORDS);

/// The groups of the engine's own commands: a line only they fit is the
/// engine's, whatever word of a film shares its name (`fade 30`).
pub(crate) const ENGINE_GROUPS: [&str; 2] = ["engine", "commands"];

/// The words of the language: its groups, the narrower first; what is read
/// of them once, the first time it is asked.
pub struct Lexicon {
    groups: &'static [Group],
    /// The library's recipe files (`film/recipes/`), each by its name and its
    /// text, as the one who assembles the language takes them in.
    files: &'static [(&'static str, &'static str)],
    pub(crate) recipes: OnceLock<Vec<Recipe>>,
    commands: OnceLock<Group>,
    pub(crate) known: OnceLock<HashSet<String>>,
    /// The vocabulary, derived from the words (`vocabulary.rs`).
    pub(crate) vocabulary: OnceLock<Vec<Planned>>,
}

impl Lexicon {
    /// The language of those groups, the narrower first (the text's words
    /// and the engine's around them).
    pub const fn new(groups: &'static [Group]) -> Lexicon {
        Lexicon::with_library(groups, &[])
    }

    /// The language of those groups and the library's recipe files
    /// (`film/recipes/`), each by the name a film gives it and its text, in
    /// their order.
    pub const fn with_library(groups: &'static [Group], files: &'static [(&'static str, &'static str)]) -> Lexicon {
        Lexicon { groups, files, recipes: OnceLock::new(), commands: OnceLock::new(), known: OnceLock::new(), vocabulary: OnceLock::new() }
    }

    /// The groups of the words written for films, the narrower first.
    pub fn groups(&self) -> &'static [Group] {
        self.groups
    }

    /// The groups of the words written for films, the text's first: those
    /// that declare the vocabulary's words (not the engine's commands).
    pub(crate) fn groups_of_words(&'static self) -> impl Iterator<Item = &'static Group> {
        std::iter::once(&TEXT).chain(self.groups.iter())
    }

    /// Every group, the text's and the engine's too: those whose forms a line is read by.
    pub(crate) fn all_groups(&'static self) -> impl Iterator<Item = &'static Group> {
        self.all_groups_but_commands().chain(std::iter::once(self.commands()))
    }

    /// The groups of the words written for films, and of the engine's
    /// commands given a form of their own (`engine.rs`).
    pub(crate) fn all_groups_but_commands(&'static self) -> impl Iterator<Item = &'static Group> {
        std::iter::once(&TEXT).chain(self.groups.iter()).chain(std::iter::once(&ENGINE))
    }

    /// The group of the engine's commands that no word of these groups names.
    pub(crate) fn commands(&'static self) -> &'static Group {
        self.commands.get_or_init(|| commands::group(self))
    }

    /// The engine's own groups: the commands given a form here, then the others.
    pub(crate) fn engine_groups(&'static self) -> [&'static Group; 2] {
        [&ENGINE, self.commands()]
    }

    /// The library's recipe files, in their order: each by the name a film
    /// gives it (`gestures`, `fishing/line`) and its text.
    pub fn recipe_files(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.files.iter().copied()
    }

    /// The pictures of one's own (`images/`, by name) that words pick by
    /// themselves, no line naming them, each with the group of words that
    /// does (fishing's bobber). A film drawing one lists its file in
    /// `Written::pictures`.
    pub fn own_pictures(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.groups.iter().flat_map(|group| group.pictures.iter().map(|picture| (group.name, *picture)))
    }

    /// A line of that word, with what is given each name (`("who", "eevee")`),
    /// written by those of its forms that take all of them and need no more:
    /// the shortest (`a put 3 4`, not `put face to face a 3 4`), who first when
    /// its form starts with who acts; why not, if
    /// none does.
    pub fn line_of(&'static self, word: &str, given: &[(&str, &str)]) -> Result<String, String> {
        let given: Vec<(&str, Vec<String>)> = given.iter().map(|(name, said)| (*name, said.split_whitespace().map(str::to_string).collect())).collect();
        let given: Vec<(&str, &[String])> = given.iter().map(|(name, said)| (*name, said.as_slice())).collect();
        let forms = self.all_groups().flat_map(|group| group.words.iter()).filter(|known| known.word == word).flat_map(|known| known.forms.iter());
        let shortest = forms
            .filter_map(|form| {
                let pieces = crate::grammar::read(form);
                Some((crate::grammar::write(&pieces, &given)?, crate::grammar::performer_first(&pieces)))
            })
            .min_by_key(|(words, _)| words.len());
        match shortest {
            Some((words, true)) if !words.is_empty() => {
                Ok(std::iter::once(words[0].clone()).chain(std::iter::once(word.to_string())).chain(words[1..].iter().cloned()).collect::<Vec<_>>().join(" "))
            }
            Some((words, _)) => Ok(std::iter::once(word.to_string()).chain(words).collect::<Vec<_>>().join(" ")),
            None => {
                let names: Vec<&str> = given.iter().map(|(name, _)| *name).collect();
                Err(format!("“{word}” has no form that takes {}", names.join(", ")))
            }
        }
    }

    /// The engine's commands as words of the language: those given a form here
    /// (`engine.rs`), then the others, their form from the docs: each its word,
    /// its forms, what it writes.
    pub fn command_words(&'static self) -> impl Iterator<Item = (&'static str, &'static [&'static str], &'static str)> {
        ENGINE.words.iter().chain(self.commands().words.iter()).map(|word| (word.word, word.forms, word.writes))
    }

    /// The word of that name in that group (the text's, the engine's, the
    /// commands' too).
    pub(crate) fn word_of_group(&'static self, group: &str, verb: &str) -> Option<&'static Word> {
        self.all_groups().filter(|known| known.name == group).flat_map(|known| known.words.iter()).find(|word| word.word == verb)
    }

    /// The engine's commands a film may write: not those the vocabulary replaced
    /// (`fadein`: `transition fade in`).
    pub fn shown_commands(&'static self) -> impl Iterator<Item = (&'static str, &'static [&'static str], &'static str)> {
        self.command_words().filter(|(word, ..)| self.replaced(word).is_none())
    }

    /// The time of a word of a group (`every_word`'s), as the registry says it;
    /// a recipe's, by its lines.
    pub fn word_time(&'static self, group: &str, word: &str) -> crate::time::WordTime {
        self.all_groups()
            .filter(|known| known.name == group)
            .flat_map(|known| known.words.iter())
            .find(|known| known.word == word)
            .map_or(crate::time::WordTime::ByLine, Word::timing)
    }

    /// The engine's commands whose time is not instant: each its word and its time.
    pub fn engine_times(&'static self) -> impl Iterator<Item = (&'static str, crate::time::WordTime)> {
        ENGINE
            .words
            .iter()
            .chain(self.commands().words.iter())
            .map(|word| (word.word, word.timing()))
            .filter(|(_, time)| *time != crate::time::WordTime::Instant)
    }
}

/// The language with no group of words but the text's and the engine's:
/// what the core's own tests read with.
#[cfg(test)]
pub(crate) static BARE: Lexicon = Lexicon::new(&[]);

/// The words of the text itself, read before any line is played.
pub const TEXT_WORDS: &[Word] = &[
    Word::new("title", &["text..."], "typecaption before the first scene").family("Structure").ds_direct("back_SetTitleBanner(56, title): alone on black before the scene, as Sky titles its story"),
    Word::new("scene", &["place:word text..."], "scene, backdrop, ground, spread").family("Structure").ds_direct("back_SetGround(LEVEL_…), supervision_Acting(0), camera_SetMyself<performer 0>; the top screen's picture as Sky pairs it with the place (back2_SetMode(4), back2_SetGround), else black as most of Sky's scenes; who stands there goes in the .ssa"),
    Word::new(
        "transition",
        &["kind:fade|white|boss|wipe|iris|cut"],
        "how the next scene comes: whitein, iris, wipe, fadein (and their way out); with in or out, now (verbs/pictures.rs)",
    ).family("Structure").ds_close("fade: screen_FadeIn/Out; white: screen_WhiteOut, then screen_FadeIn back (as Sky); boss: CallCommon(CORO_BOSS_WIPE_FUNC), Sky's own; wipe and iris: a fade (Sky has neither)", "a wipe or an iris becomes a fade"),
    Word::new("define", &["name:word text..."], "nothing: its lines in the place of its name").family("Structure").ds_direct("nothing: recipes are written out before the export"),
    Word::new("repeat", &["n:number"], "nothing: its lines so many times").family("Structure").ds_direct("nothing: its turns are written out before the export"),
    Word::new("else", &[""], "else: the other branch of if … endif (engine commands)").family("Structure").ds_direct("} else {"),
    Word::new(
        "end",
        &["text..."],
        "closes a block (define, repeat, fight directed), or the last one open: a picture's shot, dimensional_scream, memory, choice, fight",
    ).family("Structure").ds_direct("nothing (closes a block)"),
];

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
        crate::grammar::performer_first(&self.form)
    }

    /// Whether it is told a team (`*who`).
    pub(crate) fn of_a_team(&self) -> bool {
        self.form.iter().any(|piece| matches!(piece, Piece::Team(_)))
    }
}

/// A recipe the film writes or uses, and the `uses` line that takes it in
/// (none: written in the film, or in a file it takes in).
#[derive(Clone, Debug)]
pub struct Defined {
    pub recipe: Recipe,
    pub uses: Option<usize>,
}

/// What a word of the vocabulary is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Made {
    /// A `Word` of a group (`film_core/src/verbs/`, a family crate).
    Word,
    /// A recipe written in the language, provided (`film/recipes/`).
    Recipe,
    /// A way of writing of the language itself, no word to look up (a
    /// line said, `{value}` in it): it has no page of its own.
    Syntax,
    /// One of the engine's own commands.
    Command,
}

/// A word of the vocabulary: its family, itself, what it is, whether it is
/// written who first, how it stands on the DS.
#[derive(Clone, Copy, Debug)]
pub struct Planned {
    pub family: &'static str,
    pub word: &'static str,
    pub made: Made,
    pub who_first: bool,
    pub ds: OnDs,
}
