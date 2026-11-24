//! A film's text as written: read back to the same bytes, every line in its
//! outline once, each edit undone by the one it gives back.

use film_core::doc::{Doc, Edit, Kind};
/// Every film of the tests' corpus, its path and its text.
fn films() -> Vec<(String, String)> {
    crate::tests::corpus().into_iter().map(|path| (path.display().to_string(), std::fs::read_to_string(&path).unwrap_or_default())).collect()
}

#[test]
fn every_film_reads_back_to_the_same_bytes() {
    let films = films();
    assert!(!films.is_empty(), "no film in tests/games");
    for (name, text) in &films {
        assert!(Doc::read(text).text() == *text, "{name}: read back, it is no longer the same");
    }
    for text in ["", "a", "a\n", "a\r\nb\r\n", "a\nb", "\n\n", "x\r\ny\nz"] {
        assert_eq!(Doc::read(text).text(), text);
    }
}

#[test]
fn every_line_of_a_film_is_in_its_outline_once() {
    for (name, text) in &films() {
        let doc = Doc::read(text);
        let outline = doc.outline();
        let mut seen: Vec<usize> = outline.every_line().iter().map(|node| node.number).collect();
        seen.extend(outline.scenes.iter().map(|scene| scene.number));
        seen.extend(outline.scenes.iter().flat_map(|scene| scene.moments.iter().filter_map(|moment| moment.number)));
        seen.sort();
        let all: Vec<usize> = (1..=doc.len()).collect();
        assert_eq!(seen, all, "{name}: lines are missing from its outline, or are in it twice");
        let scenes = doc.numbered().filter(|(_, line)| matches!(film_core::doc::kind_of(line), Kind::Scene { .. })).count();
        assert_eq!(outline.scenes.len(), scenes, "{name}");
    }
}

#[test]
fn a_film_is_outlined_in_scenes_moments_and_blocks() {
    let text = "title: Trial\n# a word\n\ntransition: wipe\nscene: beach pikachu eevee\nput pikachu 300 200\nmoment: the start\npikachu (joyous): Hi!\nbeside wind\njump eevee\nend\nend\nscene: town\ntogether\njump pikachu\n";
    let outline = Doc::read(text).outline();
    assert_eq!(outline.head.len(), 4, "title, comment, empty line, transition");
    assert_eq!(outline.scenes.len(), 2);
    let beach = &outline.scenes[0];
    assert_eq!((beach.place.as_str(), beach.cast.clone(), beach.transition), ("beach", vec!["pikachu".to_string(), "eevee".to_string()], Some(4)));
    assert_eq!(beach.moments.len(), 2);
    assert_eq!(beach.moments[1].title, "the start");
    let lines = &beach.moments[1].lines;
    assert_eq!(lines[0].kind, Kind::Said { who: "pikachu".to_string(), face: Some("joyous".to_string()) });
    assert_eq!(lines[1].kind, Kind::Opens { word: "beside".to_string() });
    assert_eq!(lines[1].inner.len(), 2, "jump, and its end");
    assert_eq!(lines[2].kind, Kind::Fin, "an end that closes nothing is a line");
    // (A block not closed runs to the end of its scene.)
    let town = &outline.scenes[1];
    assert_eq!(town.moments[0].lines[0].inner.len(), 1);
}

#[test]
fn an_edit_is_undone_by_the_one_it_gives_back() {
    let text = "scene: beach pikachu\r\nput pikachu  300 200\r\njump pikachu";
    let edits = [
        Edit::Line { number: 3, text: "jump pikachu 12".to_string() },
        Edit::Insert { number: 2, text: "wait 1s".to_string() },
        Edit::Insert { number: 4, text: "end".to_string() },
        Edit::Remove { number: 3 },
        Edit::Remove { number: 1 },
        Edit::Words { number: 2, at: 1, long: 2, with: vec!["312".to_string(), "188".to_string()] },
        Edit::Words { number: 2, at: 1, long: 0, with: vec!["to".to_string()] },
        Edit::Words { number: 2, at: 1, long: 2, with: Vec::new() },
        Edit::Lines { number: 2, long: 2, with: vec!["jump pikachu".to_string(), "put pikachu  300 200".to_string()] },
        Edit::Lines { number: 1, long: 3, with: vec!["scene: town".to_string()] },
        Edit::Lines { number: 2, long: 0, with: vec!["wait 1s".to_string(), "wait 2s".to_string()] },
        Edit::Text { number: 2, long: 1, with: "jump pikachu\r\nwait 1s\r\n".to_string() },
        Edit::Text { number: 3, long: 1, with: "jump pikachu 12\r\n".to_string() },
        Edit::Text { number: 4, long: 0, with: "end".to_string() },
    ];
    for edit in edits {
        let mut doc = Doc::read(text);
        let undo = doc.apply(&edit).unwrap_or_else(|why| panic!("{edit:?}: {why}"));
        assert_ne!(doc.text(), text, "{edit:?}");
        doc.apply(&undo).expect("undone");
        assert_eq!(doc.text(), text, "{edit:?} then {undo:?}");
    }
    let mut doc = Doc::read(text);
    doc.apply(&Edit::Words { number: 2, at: 1, long: 2, with: vec!["312".to_string(), "188".to_string()] }).expect("words");
    assert_eq!(doc.line(2), Some("put pikachu  312 188"), "its spaces kept");
    assert_eq!(doc.revision(), 1);
    assert!(doc.apply(&Edit::Remove { number: 9 }).is_err());
}

#[test]
fn what_is_typed_is_made_by_the_least_edit() {
    let mut doc = Doc::read("scene: beach pikachu\r\njump pikachu\r\njoy pikachu\r\n");
    assert_eq!(doc.typed(), "scene: beach pikachu\njump pikachu\njoy pikachu\n");
    assert_eq!(doc.edit_to(&doc.typed()), None);
    // (A letter typed: that line alone, with the text's own endings.)
    let edit = doc.edit_to("scene: beach pikachu\njump pikachu 2\njoy pikachu\n").expect("an edit");
    assert_eq!(edit, Edit::Text { number: 2, long: 1, with: "jump pikachu 2\r\n".to_string() });
    // (A line begun at the end, after the last ending; one taken out.)
    for typed in ["scene: beach pikachu\njump pikachu\njoy pikachu\nd", "scene: beach pikachu\njoy pikachu\n", "", "a\n\n\nb"] {
        let was = doc.text();
        let undo = doc.apply(&doc.edit_to(typed).expect("an edit")).expect("made");
        assert_eq!(doc.typed(), typed);
        doc.apply(&undo).expect("undone");
        assert_eq!(doc.text(), was);
    }
}
