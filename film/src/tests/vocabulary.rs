//! The vocabulary, derived from the words: each declared once, in a family,
//! with its DS status (`film_core::vocabulary`).

use film_core::ds::Export;
use film_core::vocabulary::{FAMILIES, OTHERS};

use crate::LEXICON;

/// Every declaration of a word written for films: the text's and the groups'.
fn declared() -> impl Iterator<Item = (&'static str, &'static film_core::Word)> {
    let text = film_core::verbs::TEXT_WORDS.iter().map(|word| ("text", word));
    text.chain(LEXICON.groups().iter().flat_map(|group| group.words.iter().map(move |word| (group.name, word))))
}

#[test]
fn each_word_is_declared_once_with_its_family_and_its_ds_status() {
    let mut wrong = Vec::new();
    let with_family: Vec<(&str, &film_core::Word)> = declared().filter(|(_, word)| word.family.is_some()).collect();
    for (at, (group, word)) in with_family.iter().enumerate() {
        let family = word.family.unwrap_or_default();
        if let Some((other, _)) = with_family[..at].iter().find(|(_, known)| known.word == word.word) {
            wrong.push(format!("“{}”: its family said twice ({other}, {group}): once, the other declarations only add forms", word.word));
        }
        if !FAMILIES.contains(&family) {
            wrong.push(format!("“{}” ({group}): no family “{family}” (vocabulary::FAMILIES)", word.word));
        }
        if OTHERS.iter().any(|other| other.word == word.word) {
            wrong.push(format!("“{}”: both a Word ({group}) and in vocabulary::OTHERS", word.word));
        }
    }
    for (group, word) in declared() {
        if word.family.is_some() != word.ds.is_some() {
            wrong.push(format!("“{}” ({group}): a family and a DS status, both or neither (.family, .ds_direct...)", word.word));
        }
    }
    for word in film_core::verbs::engine::WORDS {
        if word.family.is_some() || word.ds.is_some() {
            wrong.push(format!("“{}”: an engine's form has no family (vocabulary::OTHERS has its command)", word.word));
        }
    }
    for planned in LEXICON.vocabulary() {
        let ds = planned.ds;
        if ds.how.is_empty() {
            wrong.push(format!("{}: no DS note", planned.word));
        }
        if (ds.export == Export::Close) != ds.lost.is_some() {
            wrong.push(format!("{}: a loss said if and only if it exports with one", planned.word));
        }
        if ds.fix.is_some() && ds.export.exports() {
            wrong.push(format!("{}: it exports, no fix needed", planned.word));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_vocabulary_says_who_comes_first_the_family_and_the_old_words() {
    assert_eq!(LEXICON.who_first("walk"), Some(true));
    assert_eq!(LEXICON.who_first("music"), Some(false));
    assert_eq!(LEXICON.who_first("fight"), Some(false));
    assert_eq!(LEXICON.who_first("hug"), Some(true));
    assert_eq!(LEXICON.who_first("dungeon_hud"), None);
    assert_eq!(LEXICON.family_of("walk"), "Movement");
    assert_eq!(LEXICON.family_of("hug"), "Body");
    assert_eq!(LEXICON.family_of("musicspeed"), film_core::vocabulary::ADVANCED);
    assert_eq!(LEXICON.replaced("fadein"), Some("transition fade in [d]"));
    assert_eq!(LEXICON.replaced("walk"), None);
    assert_eq!(LEXICON.replaced("rank"), None);
    assert_eq!(crate::ds::status("take_off"), Export::Direct);
    assert!(crate::ds::refused("throw").contains("attack"));
    let families: Vec<&str> = LEXICON.vocabulary().iter().map(|planned| planned.family).collect();
    assert!(
        families.windows(2).all(|pair| FAMILIES.iter().position(|f| *f == pair[0]) <= FAMILIES.iter().position(|f| *f == pair[1])),
        "in the families' order"
    );
}
