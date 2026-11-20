//! Each recipe checked once, alone, where it is written (`text::checked`).

/// What is wrong in a film, not what is to look at: its line, its message.
fn wrong(film: &str) -> Vec<(usize, String)> {
    crate::tests::compile(film)
        .problems
        .into_iter()
        .filter(|problem| !problem.is_remark())
        .filter(|problem| problem.line() > 0 && problem.span.file.is_none())
        .map(|problem| (problem.line(), problem.message))
        .collect()
}

/// A word not known, a form that does not fit (a duration where someone
/// is wanted): each told once, on the recipe's own line, though it is never
/// called; on its words there.
#[test]
fn a_recipe_is_checked_where_it_is_written_though_never_called() {
    let film = "define broken who:pokemon d:duration\n  jumpp who\n  d look left\n  who look left\nend\nscene: beach pikachu\nwait 1s\n";
    let told = wrong(film);
    assert_eq!(told.iter().filter(|(line, said)| *line == 2 && said.contains("unknown word “jumpp”")).count(), 1, "{told:?}");
    assert_eq!(told.iter().filter(|(line, said)| *line == 3 && said.contains("d look left")).count(), 1, "{told:?}");
    assert_eq!(told.len(), 2, "{told:?}");
    let problem = crate::tests::compile(film).problems.into_iter().find(|problem| problem.message.contains("jumpp")).expect("told");
    assert_eq!(problem.span.bytes.map(|bytes| bytes.of(film)), Some("jumpp who"));
    // (Called twice: still told once, where it is written.)
    let called = format!("{film}broken pikachu 1s\nbroken pikachu 2s\n");
    assert_eq!(wrong(&called).iter().filter(|(line, said)| *line == 2 && said.contains("jumpp")).count(), 1);
}

/// Typed names, a reckoning, `if` and `if not` a name left out, an untyped
/// name, a recipe calling another: nothing told.
#[test]
fn a_correct_recipe_is_not_told() {
    let film = "define hop who:pokemon where:place [other:pokemon] turns:number=2\n  who walk to where\n  wait {turns*0.5s}\n  if other\n    who face other\n  end\n  if not other\n    who look left\n  end\n  greet who joy\nend\ndefine greet who:word mood:word\n  who emote mood\nend\nscene: beach pikachu\nwait 1s\n";
    assert_eq!(wrong(film), []);
}

/// A word of the first line written without its kind (`define cheer who`) is
/// a word the call must say as it is, not a name: its lines using it as a
/// name (`emote who joy`, `{who}`) are told, with how to write it; a word of
/// the first line that a line's own word writes too (`walk who to there`) is not.
#[test]
fn a_word_of_the_first_line_used_as_a_name_is_told() {
    let film = "define cheer who\n  who emote joy 50f\nend\ndefine shout who:word\n  say {who} loud\nend\ndefine stroll who:pokemon to\n  who walk to right\nend\nscene: beach pikachu\nwait 1s\n";
    let told = wrong(film);
    assert!(told.iter().any(|(line, said)| *line == 2 && said.contains("“who” is written as it is") && said.contains("who:word")), "{told:?}");
    assert!(!told.iter().any(|(line, _)| (5..=8).contains(line)), "{told:?}");
}

/// A recipe's line that fits as written but not with what a call gives it:
/// told at the call, with the recipe line it comes from.
#[test]
fn a_failure_at_a_call_says_which_recipe_line() {
    let film = "define stare who:word\n  who look left\nend\nscene: beach pikachu\nstare 3\n";
    let told = wrong(film);
    assert!(told.iter().any(|(line, said)| *line == 5 && said.contains("in recipe `stare` (line 2)")), "{told:?}");
}

/// The library's recipes and the project's shared ones, each checked alone:
/// nothing wrong in them.
#[test]
fn every_shared_recipe_is_checked_alone() {
    let mut known: std::collections::HashMap<String, Vec<crate::text::recipes::Recipe>> = Default::default();
    let mut all = crate::text::recipes::library().to_vec();
    for name in crate::shared_files(&crate::tests::project()).iter().filter(|name| name.as_str() == "welcome") {
        let text = crate::text::recipes::shared_text(&crate::tests::project(), name).expect("read");
        all.extend(crate::text::recipes::recipes_of(&text, name));
    }
    for recipe in &all {
        known.entry(recipe.name.clone()).or_default().push(recipe.clone());
    }
    let mut wrong = Vec::new();
    for recipe in &all {
        let mut alone = crate::Film::new();
        for (line, message) in crate::text::checked::wrong_in(&mut alone, recipe, &known) {
            wrong.push(format!("{}: {message}", crate::text::recipes::told_at(&recipe.file, line)));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
