//! The wiki's tests: every word has its page, every example plays with no
//! problem; a page made, its example's line, a recipe's source, the words
//! it goes with.

use super::*;

/// Every word of the language, of the engine and of the library's recipes.
fn every_word() -> Vec<String> {
    let mut words: Vec<String> = film::every_word().map(|(_, word, ..)| word.to_string()).collect();
    words.extend(film::shown_commands().map(|(word, ..)| word.to_string()));
    words.retain(|word| !word.is_empty());
    words.sort();
    words.dedup();
    words
}

#[test]
fn every_word_has_its_page() {
    let session = Session::for_tests();
    let pages = pages(&session);
    let missing: Vec<String> = every_word().into_iter().filter(|word| !pages.iter().any(|page| &page.word == word)).collect();
    assert!(missing.is_empty(), "words with no page in the wiki: {missing:?}");
    // (A word of the language says what it does for an author: a page written in workshop_wiki/wiki/, not what it
    // writes for the engine.)
    let unwritten: Vec<&str> = pages
        .iter()
        .filter(|page| matches!(page.origin, Some(Origin::Word(_))) && page.explained.starts_with(UNWRITTEN))
        .map(|page| page.word.as_str())
        .collect();
    assert!(unwritten.is_empty(), "words of the language with no page written in workshop_wiki/wiki/: {unwritten:?}");
    for page in pages.iter() {
        assert!(!page.explained.trim().is_empty(), "“{}” says nothing of what it does", page.word);
        assert!(page.example.contains("scene:"), "“{}”: no scene in its example: {}", page.word, page.example);
        assert!(page.focus >= 1 && page.focus <= page.example.lines().count(), "“{}”: line {}", page.word, page.focus);
    }
}

/// What is wrong with an example, as the Script would say it (the details aside, and the remark that the rod or the
/// rifle is drawn on a variant only: variants are painted from one's own copy of the game, none ships).
fn problems(session: &Session, film: &str) -> Vec<String> {
    film::service::diagnostics(film, session.tables())
        .into_iter()
        .filter(|told| told.is_error() || told.is_remark() && !told.message.contains("it is only drawn on"))
        .map(|told| format!("line {}: {}", told.span.start.line, told.message))
        .collect()
}

#[test]
fn every_example_plays_with_no_problem() {
    let session = Session::for_tests();
    let pages = pages(&session);
    let wrong: Vec<String> = pages
        .iter()
        .filter_map(|page| {
            let told = problems(&session, &page.example);
            match page.retired {
                // (A word a film does not take as written: its line is refused, as its page says.)
                true => told.is_empty().then(|| format!("{} (retired): its line plays: give it an example\n{}", page.word, page.example)),
                false => (!told.is_empty())
                    .then(|| format!("{} ({}): {}\n{}", page.word, if page.written { "written" } else { "made" }, told.join("; "), page.example)),
            }
        })
        .collect();
    assert!(wrong.is_empty(), "{} examples with problems:\n\n{}", wrong.len(), wrong.join("\n"));
    assert!(pages.iter().filter(|page| page.retired).count() < 60, "a few words not for films, no more");
}

#[test]
fn the_pages_written_are_words_and_many() {
    let written = written::every_page();
    let words = every_word();
    let unknown: Vec<&String> = written.keys().filter(|word| !words.contains(word)).collect();
    assert!(unknown.is_empty(), "pages written of no word: {unknown:?}");
    let heads = written::PAGES.iter().map(|text| text.lines().filter(|line| line.starts_with("== ")).count()).sum::<usize>();
    assert_eq!(heads, written.len(), "a word written twice in workshop_wiki/wiki/");
    assert!(written.values().filter(|page| !page.example.is_empty()).count() >= 60, "at least sixty examples by hand");
}

#[test]
fn a_page_is_made_from_its_entry() {
    let entry = Entry {
        word: "jump".to_string(),
        family: 2,
        form: "who:pokemon".to_string(),
        doc: "jump".to_string(),
        subject: true,
        forms: vec!["who:pokemon".to_string(), "who:pokemon water x:number y:number".to_string()],
        origin: Origin::Word("body".to_string()),
    };
    let made = page(&entry, None);
    assert!(!made.written && made.focus_line() == "pikachu jump", "{}", made.example);
    assert_eq!(made.forms.len(), 2);
    assert_eq!(made.forms[0].shown, "who jump");
    let by_hand = written::Written {
        explained: "A hop. Two.".to_string(),
        example: "scene: beach pikachu piplup\npiplup: Up!\npikachu jump\n".to_string(),
        retired: false,
    };
    let written = page(&entry, Some(&by_hand));
    assert!(written.written);
    assert_eq!((written.focus, written.focus_line()), (3, "pikachu jump"));
    assert_eq!(written.one_line(), "A hop.");
}

#[test]
fn the_line_of_the_word_its_related_words_a_recipe_source() {
    assert_eq!(focus_of("scene: beach pikachu\nwait 1s\npikachu walk to right\n", "walk"), 3);
    assert_eq!(focus_of("scene: beach pikachu\nwalk pikachu to right\n", "walk"), 2);
    assert_eq!(focus_of("scene: beach pikachu\nwait 1s\n", "scene"), 1);
    let recipes = "# Joy.\ndefine joy who\n  emote who joy\nend\n\n# A tornado.\ndefine tornado who\ntogether\nspin who\nend\nrise who 0\nend\n";
    assert_eq!(recipe_in(recipes, "joy").as_deref(), Some("# Joy.\ndefine joy who\n  emote who joy\nend"));
    assert_eq!(recipe_in(recipes, "tornado").as_deref(), Some("# A tornado.\ndefine tornado who\ntogether\nspin who\nend\nrise who 0\nend"));
    assert_eq!(recipe_in(recipes, "fear"), None);
    let page = |word: &str, family: usize, example: &str| Page { word: word.to_string(), family, example: example.to_string(), ..Page::default() };
    let pages = vec![
        page("walk", 1, "scene: beach pikachu\npikachu walk to right\nwait 1s\n"),
        page("wait", 5, ""),
        page("run", 1, ""),
        page("run_in", 1, ""),
        page("walk_in", 1, ""),
    ];
    assert_eq!(related(&pages, &pages[0], 3), ["wait", "walk_in", "run"]);
}

/// What playing an example finds, as `scene_player check` tells it: what is
/// wrong and the remarks (not the details), each said; the art given back.
fn played(session: &Session, example: &str, art: preview::art::Art) -> (Vec<String>, Option<preview::art::Art>) {
    let written = film::compile_with(example, session.tables());
    let film = match preview::film::Film::of_written(&written, art) {
        Ok(film) => film,
        Err(failed) => return (vec![format!("it does not play: {}", failed.1)], Some(failed.0)),
    };
    let mut pass = preview::played::Pass::new(film);
    let mut seen = engine::Watch::default();
    seen.crowd = written.crowd.iter().filter_map(|code| engine::Who::parse(code)).collect();
    while pass.step() {
        pass.film.watched(&mut seen);
    }
    let told = seen
        .problems
        .iter()
        .chain(seen.remarks.iter().filter(|remark| !remark.small))
        .map(|seen| format!("line {}: {} ({})", seen.line, seen.text, seen.kind))
        .collect();
    (told, Some(pass.film.parts().0))
}

/// The quality gate of the vocabulary (`film::vocabulary`): every word of the
/// catalogue is a word of the vocabulary or an advanced one (an engine's command),
/// none of the old vocabulary nor of the library's own; each has its page in
/// the Wiki, with an example that reads with no problem and no remark
/// and, where the game's files are there, plays with nothing to say (as
/// `scene_player check` tells it). The core's words and the optional families'
/// have a page written by hand. The recipes and commands of `OTHERS` the
/// language does not have yet are pending: listed, not asked. The pages, family by family:
/// `gate`, one test each (`gates!`). (What is left to a person: see
/// CONTRIBUTING.md, "The quality gate".)
#[test]
fn every_word_passes_the_gate() {
    let session = Session::for_tests();
    let pages = pages(&session);
    let catalogue = crate::catalogue::catalogue(&session);
    let known = |word: &str| catalogue.iter().any(|entry| entry.word == word);
    let pending = film::vocabulary::pending(&known);
    println!("{} words of the vocabulary to build:", pending.len());
    for planned in &pending {
        println!("  {} ({})", planned.word, planned.family);
    }
    for (family, name) in crate::catalogue::FAMILIES.iter().enumerate() {
        println!("{name}: {} words", catalogue.iter().filter(|entry| entry.family == family).count());
    }
    let strays: Vec<&str> = catalogue
        .iter()
        .filter(|entry| {
            let planned = film::vocabulary::planned(&entry.word).is_some_and(|planned| crate::catalogue::FAMILIES[entry.family] == planned.family);
            let advanced = entry.family == crate::catalogue::ADVANCED && film::vocabulary::planned(&entry.word).is_none();
            !(planned || advanced) || film::vocabulary::replaced(&entry.word).is_some() || film::internal(&entry.word)
        })
        .map(|entry| entry.word.as_str())
        .collect();
    assert!(strays.is_empty(), "words of the catalogue neither of the plan nor advanced: {strays:?}");
    assert!(pages.iter().all(|page| page.family < crate::catalogue::FAMILIES.len()), "a page of no family the gate goes through");
}

/// The gate's pages of one family (of `catalogue::FAMILIES`), the `part`th
/// of `parts` (one in `parts` of them): each a test of its own, so that they
/// run side by side.
fn gate(family: usize, part: usize, parts: usize) {
    let session = Session::for_tests();
    let pages = pages(&session);
    let mut art = preview::art::Art::open_in(session.data(), session.library().clone()).ok().filter(|_| !session.tables().monsters.is_empty());
    let mut wrong = Vec::new();
    for page in pages.iter().filter(|page| !page.retired && page.family == family).skip(part).step_by(parts) {
        let planned = film::vocabulary::planned(&page.word).filter(|planned| planned.family != film::vocabulary::ADVANCED);
        if planned.is_some() && !page.written {
            wrong.push(format!("{}: no page written by hand in workshop_wiki/wiki/", page.word));
        }
        let read = problems(&session, &page.example);
        if !read.is_empty() {
            wrong.push(format!("{} (read): {}\n{}", page.word, read.join("; "), page.example));
            continue;
        }
        if let Some(given) = art.take() {
            let (told, back) = played(&session, &page.example, given);
            art = back;
            if !told.is_empty() {
                wrong.push(format!("{} (played): {}\n{}", page.word, told.join("; "), page.example));
            }
        }
    }
    assert!(wrong.is_empty(), "{} words do not pass the gate:\n\n{}", wrong.len(), wrong.join("\n\n"));
}

/// One test of the gate per family, in the order of `catalogue::FAMILIES`.
macro_rules! gates {
    ($($name:ident),* $(,)?) => {
        const GATED: &[&str] = &[$(stringify!($name)),*];
        gates!(@each 0usize; $($name),*);
    };
    (@each $at:expr; $name:ident $(, $rest:ident)*) => {
        #[test]
        fn $name() {
            gate($at, 0, 1);
        }
        gates!(@each $at + 1; $($rest),*);
    };
    (@each $at:expr;) => {};
}

gates!(
    gate_structure,
    gate_speech,
    gate_presence,
    gate_formations,
    gate_movement,
    gate_turning,
    gate_body,
    gate_emotions,
    gate_camera,
    gate_screen,
    gate_sound,
    gate_effects,
    gate_objects,
    gate_fights,
    gate_time,
    gate_story,
    gate_dungeon,
    gate_guild,
    gate_fishing,
);

/// The advanced words, the most by far: in parts of their own.
macro_rules! advanced {
    ($($name:ident = $part:expr),* $(,)?) => {
        const ADVANCED_PARTS: usize = [$($part),*].len();
        $(
            #[test]
            fn $name() {
                gate(crate::catalogue::ADVANCED, $part, ADVANCED_PARTS);
            }
        )*
    };
}

advanced!(
    gate_advanced_0 = 0,
    gate_advanced_1 = 1,
    gate_advanced_2 = 2,
    gate_advanced_3 = 3,
    gate_advanced_4 = 4,
    gate_advanced_5 = 5,
    gate_advanced_6 = 6,
    gate_advanced_7 = 7,
);

/// The gate's tests are the families', one each, named after them (the
/// advanced words' apart, `advanced!`).
#[test]
fn the_gate_has_a_test_per_family() {
    let names: Vec<String> = crate::catalogue::FAMILIES[..crate::catalogue::ADVANCED].iter().map(|family| format!("gate_{}", family.to_lowercase())).collect();
    assert_eq!(GATED, names.iter().map(String::as_str).collect::<Vec<_>>());
}
