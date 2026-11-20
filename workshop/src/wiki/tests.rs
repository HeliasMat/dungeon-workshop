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
    // (A word of the language says what it does for an author: a page written in workshop/wiki/, not what it
    // writes for the engine.)
    let unwritten: Vec<&str> = pages
        .iter()
        .filter(|page| matches!(page.origin, Some(Origin::Word(_))) && page.explained.starts_with(UNWRITTEN))
        .map(|page| page.word.as_str())
        .collect();
    assert!(unwritten.is_empty(), "words of the language with no page written in workshop/wiki/: {unwritten:?}");
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
    assert_eq!(heads, written.len(), "a word written twice in workshop/wiki/");
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

/// The quality gate of the vocabulary (`film::plan`): every word of the
/// catalogue is a word of the plan or an advanced one (an engine's command),
/// none of the old vocabulary nor of the library's own; each has its page in
/// the Wiki, with an example that reads with no problem and no remark
/// and, where the game's files are there, plays with nothing to say (as
/// `scene_player check` tells it). The core's words and the optional families'
/// have a page written by hand. The words of the plan the language does not
/// have yet are pending: listed, not asked. (What is left to a person: see
/// CONTRIBUTING.md, "The quality gate".)
#[test]
fn every_word_of_the_v2_families_passes_the_gate() {
    let session = Session::for_tests();
    let pages = pages(&session);
    let catalogue = crate::catalogue::catalogue(&session);
    let known = |word: &str| catalogue.iter().any(|entry| entry.word == word);
    let pending = film::plan::pending(&known);
    println!("{} words of the plan to build:", pending.len());
    for planned in &pending {
        println!("  {} ({}): {}", planned.word, planned.family, planned.form);
    }
    for (family, name) in crate::catalogue::FAMILIES.iter().enumerate() {
        println!("{name}: {} words", catalogue.iter().filter(|entry| entry.family == family).count());
    }
    let strays: Vec<&str> = catalogue
        .iter()
        .filter(|entry| {
            let planned = film::plan::planned(&entry.word).is_some_and(|planned| crate::catalogue::FAMILIES[entry.family] == planned.family);
            let advanced = entry.family == crate::catalogue::ADVANCED && film::plan::planned(&entry.word).is_none();
            !(planned || advanced) || film::plan::replaced(&entry.word).is_some() || film::internal(&entry.word)
        })
        .map(|entry| entry.word.as_str())
        .collect();
    assert!(strays.is_empty(), "words of the catalogue neither of the plan nor advanced: {strays:?}");
    let mut art = preview::art::Art::open_in(session.data(), session.library().clone()).ok().filter(|_| !session.tables().monsters.is_empty());
    let mut wrong = Vec::new();
    for page in pages.iter().filter(|page| !page.retired) {
        let planned = film::plan::planned(&page.word).filter(|planned| planned.family != film::plan::ADVANCED);
        if planned.is_some() && !page.written {
            wrong.push(format!("{}: no page written by hand in workshop/wiki/", page.word));
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
