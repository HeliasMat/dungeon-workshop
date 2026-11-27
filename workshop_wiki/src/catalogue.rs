//! The words of the language by family, as one looks for one: each word in
//! the family a reader would look in ("Movement", "Speech"...), its
//! form, an example of a line with it. And the game's moves, offered after
//! `move <who> `, found by what is typed of them (no spaces, no accents).

use std::sync::Arc;

use workshop_base::session::Session;

/// The families as they are shown, in that order: the core's
/// (`film::vocabulary`), the optional ones (`uses dungeon`), the advanced
/// words last (folded).
pub const FAMILIES: [&str; 20] = film::vocabulary::FAMILIES;

/// The family of the advanced words, folded by default.
pub const ADVANCED: usize = FAMILIES.len() - 1;

/// A family's name as shown.
pub fn family_shown(family: usize) -> String {
    FAMILIES.get(family).copied().unwrap_or_default().to_string()
}

/// A word of the catalogue: itself, its family (of `FAMILIES`), its first
/// form (without the word), what it is for.
#[derive(Clone, Debug)]
pub struct Entry {
    pub word: String,
    pub family: usize,
    pub form: String,
    pub doc: String,
    /// Said of someone, who written first (`pikachu walk to right`).
    pub subject: bool,
    /// All its forms (without the word), of every family that reads it, in turn.
    pub forms: Vec<String>,
    /// Where it comes from: a family of the language, a recipe, the engine.
    pub origin: Origin,
}

/// Where a word comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Origin {
    /// A word of the language, read by that family of the code (`moving`, `said`…).
    Word(String),
    /// A recipe of the library: its file's name (`gestures`, `dungeon/floor`), its line there.
    Recipe { from: String, line: usize },
    /// One of the engine's own commands.
    Command,
}

impl Entry {
    /// Its form as it is written: who first if it is said of someone.
    pub fn shown(&self) -> String {
        film::shown_form(&self.word, &self.form)
    }
}

/// The family a word goes in: its own (`film::vocabulary`), else the advanced
/// words (an engine's command).
fn family_of(word: &str) -> usize {
    let family = film::vocabulary::family_of(word);
    FAMILIES.iter().position(|known| *known == family).unwrap_or(ADVANCED)
}

/// Every word of the language (and the library's recipes), once each, in
/// their families; the engine's commands last.
pub fn catalogue(session: &Session) -> Arc<Vec<Entry>> {
    session.kept.get(|| {
        let mut entries: Vec<Entry> = Vec::new();
        let mut put = |entry: Entry| {
            if !entries.iter().any(|known| known.word == entry.word) {
                entries.push(entry);
            }
        };
        // (The user's own recipe files are theirs, not the language's: no page.)
        for recipe in film::recipes_for(session.library(), "").into_iter().filter(|recipe| !recipe.not_used) {
            let form = recipe.shape.strip_prefix(recipe.name.as_str()).unwrap_or_default().trim().to_string();
            let subject = film::takes_someone_first(&recipe.name, "");
            put(Entry {
                word: recipe.name.clone(),
                family: family_of(&recipe.name),
                forms: vec![form.clone()],
                form,
                doc: recipe.doc.clone(),
                subject,
                origin: Origin::Recipe { from: recipe.from.clone(), line: recipe.line },
            });
        }
        for (code, word, forms, writes) in film::every_word() {
            if word.is_empty() || forms.is_empty() && code.contains('/') {
                continue;
            }
            put(Entry {
                word: word.to_string(),
                family: family_of(word),
                form: simplest(forms),
                doc: writes.to_string(),
                subject: film::takes_someone_first(word, ""),
                forms: all_forms(word),
                origin: Origin::Word(code.to_string()),
            });
        }
        for (word, forms, writes) in film::shown_commands() {
            put(Entry {
                word: word.to_string(),
                family: family_of(word),
                form: simplest(forms),
                doc: writes.to_string(),
                subject: film::takes_someone_first(word, ""),
                forms: all_forms(word),
                origin: Origin::Command,
            });
        }
        entries.sort_by(|a, b| (a.family, &a.word).cmp(&(b.family, &b.word)));
        entries
    })
}

/// The form shown of a word: the one that starts with someone if there is
/// one (`walk who:pokemon to where:place`, not its formations'), else the first.
fn simplest(forms: &[&str]) -> String {
    forms.iter().find(|form| form.starts_with("who")).or(forms.first()).copied().unwrap_or_default().to_string()
}

/// Every form of a word, of every family that reads it (the language's,
/// then the engine's), each once, in turn.
fn all_forms(word: &str) -> Vec<String> {
    let language = film::every_word().filter(|(_, known, ..)| *known == word).flat_map(|(_, _, forms, _)| forms.iter());
    let engine = film::command_words().filter(|(known, ..)| *known == word).flat_map(|(_, forms, _)| forms.iter());
    let mut all: Vec<String> = Vec::new();
    for form in language.chain(engine) {
        if !all.iter().any(|known| known == form) {
            all.push(form.to_string());
        }
    }
    all
}

/// The words of a family.
pub fn of_family(session: &Session, family: usize) -> Vec<Entry> {
    catalogue(session).iter().filter(|entry| entry.family == family).cloned().collect()
}

/// A word by its name.
#[cfg(test)]
pub fn entry(session: &Session, word: &str) -> Option<Entry> {
    catalogue(session).iter().find(|entry| entry.word == word).cloned()
}

/// An example of a line with the word, its form filled in: Pikachu then
/// Eevee for whoever, a place, a second, a number, an Apple for a thing, a
/// few words said; an optional part left out, a choice its first word; who
/// first if it is said of someone (`pikachu walk to right`).
pub fn example(entry: &Entry) -> String {
    let written = example_word_first(entry);
    match entry.subject {
        true => {
            let mut words = written.splitn(3, ' ');
            let (word, who, rest) = (words.next().unwrap_or_default(), words.next().unwrap_or_default(), words.next().unwrap_or_default());
            format!("{who} {word} {rest}").trim_end().to_string()
        }
        false => written,
    }
}

/// The names a recipe gives to whom it is said of.
const PEOPLE: [&str; 12] = ["who", "other", "a", "b", "c", "target", "friend", "him", "leader", "one", "from", "with"];

/// The same, the word first.
fn example_word_first(entry: &Entry) -> String {
    use film::grammar::{Kind, Piece};
    let mut people = ["pikachu", "eevee"].into_iter().cycle();
    let mut words = vec![entry.word.clone()];
    for piece in film::grammar::read(&entry.form) {
        match piece {
            // (A recipe's word named as someone (`who`, `other`, `a`...): someone.)
            Piece::One { kind: Kind::Word, name, .. } if PEOPLE.contains(&name.as_str()) => words.push(people.next().unwrap_or("pikachu").to_string()),
            Piece::One { kind, .. } | Piece::Words { kind, .. } => words.push(
                match kind {
                    Kind::Someone | Kind::Code => people.next().unwrap_or("pikachu"),
                    Kind::Place => "right",
                    Kind::Time => "1s",
                    Kind::Number => "2",
                    Kind::Coordinate => "200",
                    Kind::Thing => "Apple",
                    Kind::Property => "opacity",
                    Kind::Word => "Hello!",
                }
                .to_string(),
            ),
            Piece::Team(_) => {
                words.push("pikachu".to_string());
                words.push("eevee".to_string());
            }
            Piece::Choice(choices) => words.extend(choices.first().cloned()),
            Piece::Maybe(_) => {}
        }
    }
    words.join(" ")
}

/// After `move <who> `: where the move's name starts (bytes into
/// `before`), who uses it, what of it is typed; none once a whole move is
/// written and more follows (its target then).
pub fn attack_rest(session: &Session, before: &str) -> Option<(usize, String, String)> {
    let moves = moves(session);
    let lead = before.len() - before.trim_start().len();
    let (first, after_first) = before[lead..].split_once(char::is_whitespace)?;
    let second_lead = after_first.len() - after_first.trim_start().len();
    let rest_of = &after_first[second_lead..];
    let second_end = rest_of.find(char::is_whitespace)?;
    let second = &rest_of[..second_end];
    // (`move pikachu …` or, who first, `pikachu move …`.)
    let who = match (film::words::plain(first).as_str(), film::words::plain(second).as_str()) {
        ("move", _) => second.to_string(),
        (_, "move") => first.to_string(),
        _ => return None,
    };
    let typed = rest_of[second_end..].trim_start();
    let start = before.len() - typed.len();
    // (A whole move already, and a space after it: what follows is not a move.)
    let squeezed = squeeze(typed);
    let whole = moves.iter().any(|name| {
        let name_squeezed = squeeze(name);
        squeezed.len() > name_squeezed.len() && squeezed.starts_with(&name_squeezed) && typed.ends_with(char::is_whitespace)
    }) || moves.iter().any(|name| typed.len() > name.len() && typed.starts_with(name.as_str()) && typed[name.len()..].starts_with(' '));
    (!whole).then_some((start, who, typed.to_string()))
}

/// A name as compared while typed: plain letters, no spaces nor marks.
fn squeeze(name: &str) -> String {
    film::words::plain(name).chars().filter(|letter| letter.is_alphanumeric()).collect()
}

/// The game's moves by their names, each once, in order.
fn moves(session: &Session) -> Arc<Vec<String>> {
    session.moves.get(|| {
        let mut names: Vec<String> =
            session.tables().moves.values().filter(|(id, _)| id != "0").map(|(_, name)| name.clone()).filter(|name| !name.is_empty()).collect();
        names.sort_by_key(|name| squeeze(name));
        names.dedup();
        names
    })
}

/// The moves whose name starts as what is typed.
pub fn move_choices(session: &Session, typed: &str) -> Vec<(String, String)> {
    let sought = squeeze(typed);
    moves(session).iter().filter(|name| squeeze(name).starts_with(&sought)).map(|name| (name.clone(), "move".to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_after_move_and_words_by_family() {
        let session = Session::for_tests();
        let (start, who, typed) = attack_rest(&session, "move pikachu Iro").expect("a move");
        assert_eq!((start, who.as_str(), typed.as_str()), (13, "pikachu", "Iro"));
        assert!(attack_rest(&session, "walk pikachu to").is_none());
        if catalogue(&session).is_empty() {
            return;
        }
        let walk = entry(&session, "walk").expect("walk");
        assert_eq!(FAMILIES[walk.family], "Movement");
        assert!(example(&walk).starts_with("pikachu walk"), "{}", example(&walk));
        // (An engine's command the vocabulary does not name: advanced; an old word: none.)
        assert_eq!(FAMILIES[0], "Structure");
        assert_eq!(family_shown(ADVANCED), "Advanced");
        assert_eq!(entry(&session, "musicspeed").map(|musicspeed| musicspeed.family), Some(ADVANCED));
        assert!(entry(&session, "dungeon_hud").is_none() && entry(&session, "fadein").is_none() && entry(&session, "_bobber").is_none());
        assert_eq!(entry(&session, "emote").map(|emote| FAMILIES[emote.family]), Some("Emotions"));
    }
}
