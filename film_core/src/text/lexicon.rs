//! What the lexicon knows once its library's recipes are read
//! (`film/recipes/`, by `recipes::read_library`): every word a film may
//! write, the recipes with the groups' words, and whether a word is said of
//! someone first.

use std::collections::HashSet;

use crate::verbs::{Lexicon, Recipe};

impl Lexicon {
    /// The library's recipes, read once.
    pub fn library(&'static self) -> &'static [Recipe] {
        self.recipes.get_or_init(|| crate::text::recipes::read_library(self))
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
        let someone = |pieces: Vec<crate::grammar::Piece>| crate::grammar::performer_first(&pieces);
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

    /// Every word of the films' language: where it is read, its forms, what it
    /// writes (a recipe: its file in `film/recipes/`, no form, what it does).
    /// Not the library's own (`internal`).
    pub fn every_word(&'static self) -> impl Iterator<Item = (&'static str, &'static str, &'static [&'static str], &'static str)> {
        self.all_words().filter(|(_, word, ..)| !crate::verbs::internal(word))
    }

    /// The same, the library's own words with them, as the language reads them.
    pub(crate) fn all_words(&'static self) -> impl Iterator<Item = (&'static str, &'static str, &'static [&'static str], &'static str)> {
        let mut recipes: Vec<&'static Recipe> = Vec::new();
        for recipe in self.library() {
            if !recipes.iter().any(|known| known.name == recipe.name) {
                recipes.push(recipe);
            }
        }
        self.groups_of_words()
            .flat_map(|group| group.words.iter().map(|word| (group.name, word.word, word.forms, word.writes)))
            .chain(recipes.into_iter().map(|recipe| (recipe.from.as_str(), recipe.name.as_str(), &[][..], recipe.told_and_doc.as_str())))
    }
}

/// Whether the film's own recipe of that name is said of someone: its first
/// value `who`, `*who`, or one of the cast (`a:pokemon`).
fn own_recipe_of_someone(word: &str, text: &str) -> bool {
    let head = format!("define {word} ");
    text.lines().filter_map(|line| line.trim_start().strip_prefix(head.as_str())).any(|rest| {
        let first = rest.split_whitespace().next().unwrap_or_default();
        first == "who" || first.starts_with("who:") || first.starts_with('*') || first.ends_with(":pokemon")
    })
}
