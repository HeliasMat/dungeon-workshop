//! A line of the timeline changed in the text (`film_core::time::replace`).

use film_core::doc::{Doc, Edit};
use film_core::time::*;

const FILM: &str = "scene: beach pikachu eevee\n  pikachu walk to 200 120 in 2s\n  eevee: Wait!  # she runs\n  at 3s : eevee jump\n  when pikachu arrives\n    pikachu jump\n  end\n  wait 1s\n";

fn changed(number: usize, change: Change) -> String {
    replaced_line(FILM, number, &change).unwrap_or_else(|why| panic!("{why}"))
}

/// The text with that line made so.
fn edited(number: usize, line: &str) -> String {
    FILM.lines().enumerate().map(|(at, old)| if at + 1 == number { line.to_string() } else { old.to_string() }).collect::<Vec<_>>().join("\n") + "\n"
}

/// The plan's line of that line of the text, as compiled (the last).
fn planned(text: &str, number: usize) -> film_core::time::PlanLine {
    let plan = crate::tests::compile(text).plan;
    plan.shown().map(|(_, line)| line.clone()).filter(|line| line.number == number).last().unwrap_or_else(|| panic!("{number} : {:?}", plan.lines))
}

#[test]
fn a_line_is_placed_with_and_after_and_back() {
    let with = changed(2, Change::Placement(Placement::With));
    assert_eq!(with, "  pikachu walk to 200 120 in 2s &");
    let text = edited(2, &with);
    assert_eq!(planned(&text, 2).placement, Placement::With);
    assert_eq!(replaced_line(&text, 2, &Change::Placement(Placement::After)).as_deref(), Ok("  pikachu walk to 200 120 in 2s"));
}

#[test]
fn a_line_is_placed_at_a_time_and_its_comment_kept() {
    let at = changed(3, Change::Placement(Placement::At { clock: Clock::Here, offset: 90 }));
    assert_eq!(at, "  at +1.5s : eevee: Wait!  # she runs", "a \"#\" said is said");
    let text = edited(3, &at);
    assert_eq!(planned(&text, 3).placement, Placement::At { clock: Clock::Here, offset: 90 });
    assert_eq!(replaced_line(&text, 3, &Change::Placement(Placement::After)).as_deref(), Ok("  eevee: Wait! # she runs"));
}

#[test]
fn an_anchored_line_and_a_block_change_their_clock() {
    assert_eq!(changed(4, Change::Placement(Placement::When("pikachu arrives".to_string()))), "  when pikachu arrives : eevee jump");
    assert_eq!(changed(4, Change::Placement(Placement::With)), "  eevee jump &");
    assert_eq!(changed(5, Change::Placement(Placement::At { clock: Clock::Scene, offset: 120 })), "  at 2s");
    assert!(replaced_line(FILM, 5, &Change::Placement(Placement::After)).is_err());
}

#[test]
fn a_comment_at_the_end_is_kept() {
    let text = "scene: beach pikachu
  pikachu jump  # up high
";
    assert_eq!(replaced_line(text, 2, &Change::Placement(Placement::With)).as_deref(), Ok("  pikachu jump & # up high"));
}

#[test]
fn how_long_a_line_lasts_is_its_time_written() {
    assert_eq!(changed(2, Change::Length(150)), "  pikachu walk to 200 120 in 2.5s");
    assert_eq!(changed(8, Change::Length(30)), "  wait 0.5s");
    assert_eq!(planned(&edited(8, "  wait 0.5s"), 8).length, Length::Known(30));
    assert!(replaced_line(FILM, 3, &Change::Length(30)).is_err(), "a line said has no time written");
    let doc = Doc::read(FILM);
    assert_eq!(replace(&doc, 8, &Change::Length(90)), Ok(Edit::Line { number: 8, text: "  wait 1.5s".to_string() }));
}

#[test]
fn what_a_line_plays_is_read_without_its_placing() {
    assert_eq!(played_words("  at 3s : eevee jump"), "eevee jump");
    assert_eq!(played_words("when pikachu arrives : pikachu: Ah!"), "pikachu: Ah!");
    assert_eq!(played_words("pikachu walk to right &"), "pikachu walk to right");
    assert_eq!(played_words("when pikachu arrives"), "when pikachu arrives");
}
