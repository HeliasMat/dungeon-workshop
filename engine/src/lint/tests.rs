use super::*;
use crate::language::read_scene;
use crate::vocabulary::SAMPLE;

/// What is wrong in a file read, its lines read as the checker reads them.
fn lint_script(script: Script, words: &'static Vocabulary) -> Vec<Diagnostic> {
    let reads = read(&script.lines, words);
    lint(script.problems, &script.lines, &reads, words)
}

#[test]
fn scene_files_are_checked() {
    let good = "event\t0\tEVENT_M01_01\t1\t1\nscene\t11\ta\nput\t0\t10\t10\tdown\ngo\t0\t20\t20\t10\t0\nsay\t0\t0\t Hello!\nsaid\nwait\t10\nfadeout\t30\n";
    assert_eq!(lint_script(read_scene(good, None, &SAMPLE), &SAMPLE), Vec::<Diagnostic>::new());
    let bad = "scene\t11\ta\ngo\t10\t20\t20\t10\t0\nsaid\njmup\t0\nask\tYes|No\nscene\t12\tb\n";
    let problems = lint_script(read_scene(bad, None, &SAMPLE), &SAMPLE);
    assert_eq!(problems.len(), 4, "{problems:?}");
    assert!(problems[0].says("character 10"));
    assert!(problems[1].says("“said” without a line"));
    assert!(problems[2].says("jmup"));
    assert!(problems[3].says("answer"));
}

#[test]
fn who_is_sent_off_the_backdrop_is_told() {
    let text = "backdrop\t11\t1\t-\nput\t0\t10\t10\tdown\nwalk\t0\t900\t10\npath\t0\t10\t10\t20\t-100\npath\t0\t10\t-40\n";
    let sizes = HashMap::from([(11, (792.0, 384.0))]);
    let problems = outside_backdrops(&read(&read_scene(text, None, &SAMPLE).lines, &SAMPLE), &sizes);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].line() == 3 && problems[1].line() == 4);
}

#[test]
fn numbers_never_given_are_told() {
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nuses\telsewhere\nset\tapples\t2\nif\telsewhere\nendif\nif\taples > 0\nendif\nwaituntil\tapples >= 3 or money > 10\nhud\t{apples} {score}\nask\tA|[if item_109 > 0]B|[if bonus]C\nanswer\t0\nanswer\t1\nanswer\t2\nend\n";
    let problems = lint_script(read_scene(text, None, &SAMPLE), &SAMPLE);
    let told: Vec<&Diagnostic> = problems.iter().filter(|problem| problem.says("never given")).collect();
    assert_eq!(told.len(), 3, "{problems:?}");
    assert!(told[0].says("“aples”") && told[1].says("“score”") && told[2].says("“bonus”"));
}

#[test]
fn lines_with_numbers_put_in_are_checked() {
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nset\tx\t10\nput\t0\t{x}\t20\tdown\nwalk\t0\t{x}\nrandom\tor\t0\t2\n";
    let problems = lint_script(read_scene(text, None, &SAMPLE), &SAMPLE);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].says("“or”") && problems[1].says("walk"));
}

#[test]
fn lines_and_rings_off_the_backdrop_are_told() {
    let lines = read_scene(
        "backdrop	1	1
lineup	100	50	-60	0	0	10	20	30
ring	100	100	200	0	10
",
        None,
        &SAMPLE,
    )
    .lines;
    let sizes = HashMap::from([(1, (200.0, 200.0))]);
    let problems = outside_backdrops(&read(&lines, &SAMPLE), &sizes);
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(problems[0].says("-80") && problems[1].says("300") && problems[2].says("-100"));
}

#[test]
fn words_compared_are_not_numbers() {
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nsettext\tword\tpikachu\nif\tword == pikachu or clock > 10\nendif\n";
    let problems = lint_script(read_scene(text, None, &SAMPLE), &SAMPLE);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn answers_after_their_end_are_told() {
    let bad = "scene\t1\ta\nask\tA|B\nanswer\t0\nend\nanswer\t1\nend\n";
    let problems = lint_script(read_scene(bad, None, &SAMPLE), &SAMPLE);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].line(), 5);
    let good = "scene\t1\ta\nask\tA|B\nanswer\t0\nask\tC|D\nanswer\t0\nanswer\t1\nend\nanswer\t1\nend\n";
    assert_eq!(lint_script(read_scene(good, None, &SAMPLE), &SAMPLE), Vec::<Diagnostic>::new());
}

#[test]
fn ways_through_a_scene_are_checked() {
    let good = "scene\t1\ta\nset\tn\t1\nif\tn\t>\t0\nlabel\tx\nelse\ngoto\tx\nendif\n";
    assert_eq!(lint_script(read_scene(good, None, &SAMPLE), &SAMPLE), Vec::<Diagnostic>::new());
    let bad = "scene\t1\ta\nuses\tn\nif\tn\nendif\nendif\ngoto\tnowhere\nif\tn\nscene\t2\tb\n";
    let problems = lint_script(read_scene(bad, None, &SAMPLE), &SAMPLE);
    assert_eq!(problems.len(), 3, "{problems:?}");
}
