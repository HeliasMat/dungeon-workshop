//! The Story family: answers offered by a test and once, words chosen in a
//! line said, values put in, going from one moment or scene to another,
//! counting what was played, the lines' ids.

use super::{engine_lines, starts};

/// The film's engine lines and what is wrong, the old order aside.
fn written(text: &str) -> (Vec<Vec<String>>, Vec<String>) {
    engine_lines(text)
}

fn has(lines: &[Vec<String>], words: &[&str]) -> bool {
    lines.iter().any(|fields| starts(fields, words))
}

#[test]
fn an_answer_is_offered_by_its_test_and_once() {
    let film = "scene: beach pikachu\nlet apples = 2\nchoice\noption \"Give an apple\" if apples > 0 once\npikachu: Thanks!\noption \"Leave\"\npikachu: Bye.\noption Old style\npikachu: Yes.\nend\n";
    let (lines, problems) = written(film);
    assert!(problems.is_empty(), "{problems:?}");
    let ask = lines.iter().find(|fields| fields[0] == "ask").expect("asked");
    assert_eq!(ask[1], "[if _apples > 0 and _once_3_1 == 0]Give an apple|Leave|Old style", "{ask:?}");
    // (Taken, never again.)
    let answer = lines.iter().position(|fields| starts(fields, &["answer", "0"])).expect("the first answer");
    assert!(starts(&lines[answer + 1], &["set", "_once_3_1", "1"]), "{:?}", &lines[answer..answer + 3]);
    // (A test the language does not read is told.)
    let (_, problems) = written("scene: beach pikachu\nchoice\noption \"Go\" if\npikachu: Yes.\nend\n");
    assert!(!problems.is_empty());
}

#[test]
fn words_chosen_in_a_line_said_and_values_put_in() {
    let film = "scene: beach pikachu\nkeep met = 0\npikachu: I {if met: know | do not know} you. You have {money} Poké.\n";
    let (lines, problems) = written(film);
    assert!(problems.is_empty(), "{problems:?}");
    let said: Vec<&Vec<String>> = lines.iter().filter(|fields| fields[0] == "say").collect();
    assert_eq!(said.len(), 2, "{lines:?}");
    assert!(said[0][3].contains("I know you. You have {money}"), "{:?}", said[0]);
    assert!(said[1][3].contains("I do not know you."), "{:?}", said[1]);
    let at = lines.iter().position(|fields| fields[0] == "if").expect("the test");
    assert!(starts(&lines[at], &["if", "met"]), "{:?}", lines[at]);
    assert!(has(&lines, &["else"]) && has(&lines, &["endif"]));
}

#[test]
fn go_on_to_a_moment_or_a_scene_and_count_them() {
    let film = "scene: beach pikachu\nmoment: the bet\npikachu: Again?\nif seen(the_bet) < 2\n  go moment the bet\nend\ngo moment the end\nscene: town pikachu\npikachu: Here.\nmoment: the end\npikachu: Done, {seen(beach)} time.\n";
    let (lines, problems) = written(film);
    assert!(problems.is_empty(), "{problems:?}");
    assert!(has(&lines, &["label", "_moment_the_bet"]) && has(&lines, &["calc", "seen_the_bet", "seen_the_bet", "+", "1"]), "{lines:?}");
    assert!(has(&lines, &["if", "seen_the_bet", "<", "2"]) && has(&lines, &["goto", "_moment_the_bet"]));
    // (Another scene's: that scene, then on to it once it is set.)
    assert!(has(&lines, &["set", "_go_moment", "2"]) && has(&lines, &["gotoscene", "scene2"]), "{lines:?}");
    assert!(has(&lines, &["if", "_go_moment", "==", "2"]) && has(&lines, &["goto", "_moment_the_end"]));
    assert!(has(&lines, &["calc", "seen_beach", "seen_beach", "+", "1"]) && has(&lines, &["calc", "seen_town", "seen_town", "+", "1"]));
    let said = lines.iter().find(|fields| fields[0] == "say" && fields[3].contains("Done")).expect("said");
    assert!(said[3].contains("{seen_beach}"), "{said:?}");
    // (A film that goes nowhere is written as before: no label, no count.)
    let (plain, _) = written("scene: beach pikachu\nmoment: the bet\npikachu: Hi.\n");
    assert!(!has(&plain, &["label", "_moment_the_bet"]));
    // (What is not there is told.)
    let (_, problems) = written("scene: beach pikachu\nmoment: the bet\ngo moment the bat\n");
    assert!(problems.iter().any(|problem| problem.contains("no moment “the_bat”")), "{problems:?}");
    let (_, problems) = written("scene: beach pikachu\ngo scene forest\n");
    assert!(problems.iter().any(|problem| problem.contains("no scene in “forest”")), "{problems:?}");
    let (_, problems) = written("scene: beach pikachu\nmoment: the bet\nif seen(the_bat) > 0\n  pikachu: Hi.\nend\n");
    assert!(problems.iter().any(|problem| problem.contains("seen(the_bat)")), "{problems:?}");
    // (What a game keeps of it.)
    assert_eq!(film_core::plot::counters(film).into_iter().collect::<Vec<_>>(), ["seen_beach", "seen_the_bet", "seen_the_end", "seen_town"]);
    assert!(film_core::plot::counters("scene: beach pikachu\nmoment: a\n").is_empty());
}

#[test]
fn each_line_said_is_given_an_id_that_stays() {
    let film = "scene: beach pikachu\npikachu: Hello! | fr: Bonjour !\npikachu (happy): Kept. #kept\nchoice\noption \"Yes\"\npikachu: Inside. &\nend\n";
    let (made, given) = crate::service::with_line_ids(film);
    assert_eq!(given, 2, "{made}");
    let ids: Vec<String> =
        made.lines().filter_map(|line| line.split(" #").nth(1).map(|id| id.split_whitespace().next().unwrap_or_default().to_string())).collect();
    assert_eq!(ids.len(), 3, "{made}");
    assert!(made.contains("pikachu (happy): Kept. #kept\n") && made.contains(" &\n"), "{made}");
    assert!(made.lines().any(|line| line.starts_with("pikachu: Hello! | fr: Bonjour ! #")), "{made}");
    // (Again: nothing new; the words changed, the id stays.)
    assert_eq!(crate::service::with_line_ids(&made), (made.clone(), 0));
    let edited = made.replace("Hello!", "Hi there!");
    assert_eq!(crate::service::with_line_ids(&edited).1, 0);
    // (It plays as before: the id is not said.)
    let (lines, problems) = written(&made);
    assert!(problems.is_empty(), "{problems:?}");
    assert!(lines.iter().filter(|fields| fields[0] == "say").all(|fields| !fields[3].contains('#')), "{lines:?}");
}

/// An optional family's words come with its library: `uses dungeon`,
/// `uses guild`, `uses fishing` take in their recipes; a word of one said
/// without it is told, its fix the line that takes it in.
#[test]
fn an_optional_family_comes_with_uses() {
    for library in ["dungeon", "guild", "fishing"] {
        let film = format!("uses {library}\nscene: beach pikachu\npikachu: Hi.\n");
        let written = crate::tests::compile(&film);
        assert!(written.problems.iter().all(|problem| problem.is_remark()), "{library}: {:?}", written.problems);
    }
    let without = "scene: beach pikachu\nbite\n";
    let told: Vec<_> = crate::tests::compile(without).problems.into_iter().filter(|problem| problem.code == Some(film_core::codes::USES)).collect();
    assert_eq!(told.len(), 1, "{told:?}");
    let fixed = crate::service::applied(without, &told[0].fixes[0].edits);
    assert!(fixed.starts_with("uses fishing\nscene:"), "{fixed}");
    assert!(crate::tests::compile(&fixed).problems.iter().all(|problem| problem.code != Some(film_core::codes::USES)));
}
