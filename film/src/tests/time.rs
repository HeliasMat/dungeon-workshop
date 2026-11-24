use film_core::time::*;

/// The plan of a film's text, and each of its lines placed without playing.
fn planned(text: &str) -> (Plan, Vec<Place>) {
    let written = crate::tests::compile(text);
    let places = written.plan.resolve();
    (written.plan, places)
}

/// The line of the plan of that line of the text (the first shown).
fn line(plan: &Plan, number: usize) -> usize {
    plan.shown().find(|(_, line)| line.number == number).map(|(at, _)| at).unwrap_or_else(|| panic!("line {number}: {:?}", plan.lines))
}

#[test]
fn a_line_starts_at_the_end_of_the_one_before() {
    let (plan, places) = planned("scene: beach pikachu\n  wait 2s\n  wait 1s\n");
    let (first, second) = (line(&plan, 2), line(&plan, 3));
    assert_eq!(plan.lines[first].length, Length::Known(120));
    assert_eq!((plan.lines[second].placement.clone(), plan.lines[second].thread), (Placement::After, 0));
    assert_eq!(places[second].start, places[first].end);
    assert_eq!(places[second].end - places[second].start, 60);
    assert!(!places[second].at_least);
    assert_eq!(plan.cause(second), Cause::After(first));
}

#[test]
fn a_line_with_and_keeps_its_handle_and_leaves_the_cursor() {
    let (plan, places) = planned("scene: beach pikachu\n  pikachu walkto 100 100 &\n  wait 1s\n");
    let (walk, wait) = (line(&plan, 2), line(&plan, 3));
    let walking = &plan.lines[walk];
    assert_eq!((walking.placement.clone(), walking.waits), (Placement::With, false));
    assert!(matches!(&walking.length, Length::Event { handle: Handle::Who(_), .. }), "{walking:?}");
    assert_eq!(places[wait].start, places[walk].start, "the cursor did not wait");
    assert_eq!(walking.lane, Lane::Who("pikachu".to_string()));
}

#[test]
fn a_walk_waits_for_who_walks_by_the_registry() {
    let (plan, places) = planned("scene: beach pikachu\n  pikachu walkto 100 100\n  wait 1s\n");
    let walk = line(&plan, 2);
    assert!(plan.lines[walk].waits);
    assert!(matches!(&plan.lines[walk].length, Length::Event { handle: Handle::Who(_), .. }));
    assert!(places[line(&plan, 3)].at_least, "after an event: ≥");
}

#[test]
fn a_line_said_beside_the_film_is_one_line_of_its_thread() {
    let (plan, _) = planned("scene: beach pikachu\n  pikachu: Oh! &\n  wait 1s\n");
    let said: Vec<&PlanLine> = plan.shown().map(|(_, line)| line).filter(|line| line.number == 2).collect();
    assert_eq!(said.len(), 1, "its bearer is not shown: {:?}", plan.lines);
    assert_eq!((said[0].thread, said[0].placement.clone()), (1, Placement::With));
    assert!(matches!(said[0].length, Length::Event { handle: Handle::Reading, at_least } if at_least > 0));
}

#[test]
fn a_line_at_a_time_of_the_scene_is_placed_by_its_clock() {
    let (plan, places) = planned("scene: beach pikachu\n  at 3s :wait 1s\n  wait 5s\n");
    let at = plan.shown().find(|(_, line)| line.number == 2 && line.thread == 1).map(|(at, _)| at).expect("in its thread");
    assert_eq!(plan.lines[at].placement, Placement::At { clock: Clock::Scene, offset: 180 });
    assert_eq!(places[at].start, 180, "the scene starts at 0");
    assert!(matches!(plan.cause(at), Cause::Clock { clock: Clock::Scene, offset: 180, .. }));
}

#[test]
fn a_line_at_an_event_waits_for_it() {
    let (plan, places) = planned("scene: beach pikachu\n  pikachu walkto 100 100 &\n  when pikachu arrives :wait 1s\n");
    let at = plan.shown().find(|(_, line)| line.number == 3).map(|(at, _)| at).expect("its line");
    assert_eq!(plan.lines[at].placement, Placement::When("pikachu arrives".to_string()));
    assert!(places[at].at_least);
}

#[test]
fn together_ends_with_the_longest() {
    let (plan, places) = planned("scene: beach pikachu\n  together\n    wait 1s\n    wait 3s\n  end\n  wait 1s\n");
    let after = line(&plan, 6);
    let start = places[line(&plan, 3)].start;
    assert_eq!(places[after].start - start, 180, "{:?}", plan.lines);
}

#[test]
fn the_registry_says_each_word_s_time() {
    assert_eq!(crate::LEXICON.word_time("engine", "go"), WordTime::Lasts);
    assert_eq!(crate::LEXICON.word_time("engine", "walkto"), WordTime::Until);
    assert_eq!(crate::LEXICON.word_time("engine", "quake"), WordTime::Instant);
    assert_eq!(crate::LEXICON.word_time("commands", "runto"), WordTime::Until);
    assert_eq!(crate::LEXICON.word_time("body", "wait"), WordTime::Until);
    assert!(crate::LEXICON.engine_times().any(|(word, time)| word == "go" && time == WordTime::Lasts));
}

#[test]
fn a_line_from_here_is_placed_so_long_after_the_cursor() {
    let (plan, places) = planned("scene: beach pikachu\n  wait 1s\n  at +2s :wait 1s\n  wait 5s\n");
    let at = plan.shown().find(|(_, line)| line.number == 3 && line.thread == 1).map(|(at, _)| at).expect("in its thread");
    assert_eq!(plan.lines[at].placement, Placement::At { clock: Clock::Here, offset: 120 });
    assert_eq!(places[at].start - places[line(&plan, 2)].end, 120);
    assert_eq!(places[line(&plan, 4)].start, places[line(&plan, 2)].end, "the cursor did not wait");
}
