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
use crate::text::recipes::Recipe;

/// The text's own words and the engine's: forms, never played here.
const TEXT: Group = Group::new("text", crate::text::WORDS);
const ENGINE: Group = Group::new("engine", engine::WORDS);

/// The groups of the engine's own commands: a line only they fit is the
/// engine's, whatever word of a film shares its name (`fade 30`).
pub(super) const ENGINE_GROUPS: [&str; 2] = ["engine", "commands"];

/// The words of the language: its groups, the narrower first; what is read
/// of them once, the first time it is asked.
pub struct Lexicon {
    groups: &'static [Group],
    recipes: OnceLock<Vec<Recipe>>,
    commands: OnceLock<Group>,
    known: OnceLock<HashSet<String>>,
}

impl Lexicon {
    /// The language of those groups, the narrower first (the text's words
    /// and the engine's around them).
    pub const fn new(groups: &'static [Group]) -> Lexicon {
        Lexicon { groups, recipes: OnceLock::new(), commands: OnceLock::new(), known: OnceLock::new() }
    }

    /// The groups of the words written for films, the narrower first.
    pub fn groups(&self) -> &'static [Group] {
        self.groups
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
    pub(super) fn engine_groups(&'static self) -> [&'static Group; 2] {
        [&ENGINE, self.commands()]
    }

    /// The library's recipes, read once.
    pub fn library(&'static self) -> &'static [Recipe] {
        self.recipes.get_or_init(|| crate::text::recipes::read_library(self))
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
                Some((crate::grammar::write(&pieces, &given)?, crate::order::performer_first(&pieces)))
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

    /// The names of the language's words, its engine's commands and the
    /// library's recipes.
    fn known_words(&'static self) -> &'static HashSet<String> {
        self.known
            .get_or_init(|| self.all_words().map(|(_, word, ..)| word.to_string()).chain(self.command_words().map(|(word, ..)| word.to_string())).collect())
    }

    /// Whether that is a word of the language (or a recipe the film writes).
    pub fn is_word(&'static self, word: &str, text: &str) -> bool {
        let word = crate::words::plain(word);
        self.known_words().contains(&word) || text.contains(&format!("define {word} "))
    }

    /// Whether that word is said of someone first (its form starts with who:
    /// `walk who:pokemon to where:place`, a recipe's `who` or `*who`).
    pub fn takes_someone_first(&'static self, word: &str, text: &str) -> bool {
        let word = crate::words::plain(word);
        let someone = |pieces: Vec<crate::grammar::Piece>| crate::order::performer_first(&pieces);
        let forms = self.all_words().filter(|(_, known, ..)| *known == word).flat_map(|(_, _, forms, _)| forms.iter());
        let commands = self.command_words().filter(|(known, ..)| *known == word).flat_map(|(_, forms, _)| forms.iter());
        forms.chain(commands).any(|form| someone(crate::grammar::read(form)))
            || self.library().iter().any(|recipe| recipe.name == word && recipe.said_of_someone())
            || own_recipe_of_someone(&word, text)
    }

    /// Whether a line is written who first (`pikachu walk to right`): its
    /// first word is no word of the language, its second is one said of someone.
    pub fn subject_first(&'static self, first: &str, second: Option<&str>, text: &str) -> bool {
        second.is_some_and(|second| !self.is_word(first, text) && self.is_word(second, text) && self.takes_someone_first(second, text))
    }

    /// The word of that name in that group (the text's, the engine's, the
    /// commands' too).
    pub(crate) fn word_of_group(&'static self, group: &str, verb: &str) -> Option<&'static Word> {
        self.all_groups().filter(|known| known.name == group).flat_map(|known| known.words.iter()).find(|word| word.word == verb)
    }

    /// Every word of the films' language: where it is read, its forms, what it
    /// writes (a recipe: its file in `film/recipes/`, no form, what it does).
    /// Not the library's own (`internal`).
    pub fn every_word(&'static self) -> impl Iterator<Item = (&'static str, &'static str, &'static [&'static str], &'static str)> {
        self.all_words().filter(|(_, word, ..)| !super::internal(word))
    }

    /// The engine's commands a film may write: not those the plan replaced
    /// (`fadein`: `transition fade in`).
    pub fn shown_commands(&'static self) -> impl Iterator<Item = (&'static str, &'static [&'static str], &'static str)> {
        self.command_words().filter(|(word, ..)| crate::plan::replaced(word).is_none())
    }

    /// The same, the library's own words with them, as the language reads them.
    pub(crate) fn all_words(&'static self) -> impl Iterator<Item = (&'static str, &'static str, &'static [&'static str], &'static str)> {
        let mut recipes: Vec<&'static Recipe> = Vec::new();
        for recipe in self.library() {
            if !recipes.iter().any(|known| known.name == recipe.name) {
                recipes.push(recipe);
            }
        }
        std::iter::once(&TEXT)
            .chain(self.groups.iter())
            .flat_map(|group| group.words.iter().map(|word| (group.name, word.word, word.forms, word.writes)))
            .chain(recipes.into_iter().map(|recipe| (recipe.from.as_str(), recipe.name.as_str(), &[][..], recipe.told_and_doc.as_str())))
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

/// Whether the film's own recipe of that name is said of someone: its first
/// value `who`, `*who`, or one of the cast (`a:pokemon`).
fn own_recipe_of_someone(word: &str, text: &str) -> bool {
    let head = format!("define {word} ");
    text.lines().filter_map(|line| line.trim_start().strip_prefix(head.as_str())).any(|rest| {
        let first = rest.split_whitespace().next().unwrap_or_default();
        first == "who" || first.starts_with("who:") || first.starts_with('*') || first.ends_with(":pokemon")
    })
}
