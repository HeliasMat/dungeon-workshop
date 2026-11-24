//! A film's lines read in their parts and made again (`film_core::lines`).

use film_core::lines::*;

#[test]
fn a_line_said_is_read_in_its_parts_and_made_again() {
    let said = Said::read("  pikachu (joyous, fast): Hi, Eevee!").expect("said");
    assert_eq!(
        said,
        Said { lead: "  ".to_string(), who: "pikachu".to_string(), marks: vec!["joyous".to_string(), "fast".to_string()], text: "Hi, Eevee!".to_string() }
    );
    assert_eq!(said.line(), "  pikachu (joyous, fast): Hi, Eevee!");
    assert_eq!(said.face(), Some("joyous"));
    assert_eq!(said.with_face(Some("sad")).line(), "  pikachu (sad, fast): Hi, Eevee!");
    assert_eq!(said.with_face(None).with_mark("fast", false).line(), "  pikachu: Hi, Eevee!");
    assert_eq!(Said::read("eevee: Yes").expect("said").with_mark("slow", true).line(), "eevee (slow): Yes");
    assert_eq!(Said::read("pikachu walk to right"), None);
}

#[test]
fn the_lines_of_the_projects_films_are_read_and_made_again_as_they_are() {
    // (Each line said of the project's films: read, then written as it was.)
    let mut seen = 0;
    for path in crate::tests::corpus() {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        for line in text.lines() {
            if let Some(said) = Said::read(line) {
                seen += 1;
                assert_eq!(Said::read(&said.line()), Some(said.clone()), "{}: {line}", path.display());
            }
        }
    }
    assert!(seen > 0, "no line said read");
}

#[test]
fn a_mark_is_set_before_a_line() {
    let text = "scene: town krabby\n  krabby jump\nmark m1\n";
    assert_eq!(new_mark(text), "m2");
    assert_eq!(with_mark_before(text, 2, "m2"), "scene: town krabby\n  mark m2\n  krabby jump\nmark m1\n");
    assert_eq!(marks(text), vec![(3, "m1".to_string())]);
}

#[test]
fn every_line_placed_by_the_workshop_is_played_with_no_problem() {
    use film_core::time::{Change, Clock, Placement};
    for placement in [
        Placement::After,
        Placement::With,
        Placement::When("krabby arrives".to_string()),
        Placement::At { clock: Clock::Mark("m1".to_string()), offset: 90 },
        Placement::At { clock: Clock::Here, offset: 60 },
        Placement::At { clock: Clock::Scene, offset: 120 },
    ] {
        let before = "scene: town krabby piplup\nmark m1\nkrabby walk to right &\npiplup jump\npiplup: Right.\n";
        let line = film_core::time::replaced_line(before, 4, &Change::Placement(placement)).expect("placed");
        let film = before.replace("piplup jump", &line);
        let wrong: Vec<String> =
            crate::tests::compile(&film).problems.iter().filter(|problem| !problem.is_remark()).map(|problem| problem.to_string()).collect();
        assert!(wrong.is_empty(), "\"{line}\": {wrong:?}");
    }
}

#[test]
fn a_name_is_made_another_where_it_names_not_in_what_is_said() {
    let text = "pinchy = krabby\nscene: beach pinchy piplup\npinchy (joyous): Hi pinchy!\npinchy walk to piplup\nwhen pinchy arrives : piplup: There you are pinchy\n# pinchy here\nmark pinchy\n";
    let made = renamed(text, "pinchy", "pincers");
    assert_eq!(
        made,
        "pincers = krabby\nscene: beach pincers piplup\npincers (joyous): Hi pinchy!\npincers walk to piplup\nwhen pincers arrives : piplup: There you are pinchy\n# pinchy here\nmark pincers\n"
    );
    assert_eq!(renamed("pinchy jump &\n", "pinchy", "pincers"), "pincers jump &\n");
}

#[test]
fn a_films_blocks_are_put_in_step() {
    let text = "scene: beach pikachu piplup\n    pikachu jump\ntogether\npikachu jump\n      piplup jump\n   end\nwhen pikachu arrives\nchoice\noption Yes\npikachu: Yes!\nend\nend\n\n# done\n";
    assert_eq!(
        formatted(text),
        "scene: beach pikachu piplup\npikachu jump\ntogether\n  pikachu jump\n  piplup jump\nend\nwhen pikachu arrives\n  choice\n  option Yes\n    pikachu: Yes!\n  end\nend\n\n# done\n"
    );
    assert_eq!(formatted(&formatted(text)), formatted(text), "a second time: nothing changes");
}

#[test]
fn a_film_folds_its_scenes_moments_and_blocks() {
    let text = "title: Trial\nscene: beach a\nmoment: one\ntogether\na jump\nend\nmoment: two\na jump\nscene: town a\na jump\n\n";
    assert_eq!(crate::service::folds(text), vec![(2, 8), (3, 6), (4, 6), (7, 8), (9, 10)]);
}

#[test]
fn lines_chosen_are_wrapped_put_beside_or_made_a_recipe() {
    let text = "scene: beach pikachu piplup\npikachu jump\npiplup jump\npikachu: Done.\n";
    assert_eq!(wrapped(text, 2, 3, "together"), "scene: beach pikachu piplup\ntogether\n  pikachu jump\n  piplup jump\nend\npikachu: Done.\n");
    let beside_them = beside(text, 2, 3);
    assert_eq!(beside_them, "scene: beach pikachu piplup\npikachu jump &\npiplup jump &\npikachu: Done.\n");
    assert_eq!(beside(&beside_them, 2, 3), text, "again: in the film's course");
    assert_eq!(
        extracted(text, 2, 3, "hop_about"),
        "define hop_about\n  pikachu jump\n  piplup jump\nend\n\nscene: beach pikachu piplup\nhop_about\npikachu: Done.\n"
    );
    for made in [wrapped(text, 2, 3, "together"), beside_them, extracted(text, 2, 3, "hop_about")] {
        let wrong: Vec<String> =
            crate::tests::compile(&made).problems.iter().filter(|problem| !problem.is_remark()).map(|problem| problem.to_string()).collect();
        assert!(wrong.is_empty(), "{made}\n{wrong:?}");
    }
}
