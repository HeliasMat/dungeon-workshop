//! The language read by `language`, then written as the words play it.

use film_core::text::front::internal;

/// Every file of recipes and every film reads with nothing wrong: what
/// is wrong in a recipe of the library is told nowhere else.
#[test]
fn every_file_reads_in_the_language() {
    let root = crate::tests::root();
    let mut wrong = Vec::new();
    for folder in ["film/recipes", "film/recipes/dungeon", "film/recipes/fishing", "tests/games/test_village/films/recipes"] {
        let Ok(found) = std::fs::read_dir(root.join(folder)) else { continue };
        for path in found.flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|extension| extension == "film")) {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for (number, told) in internal(&crate::LEXICON, &text, 0, film_core::tables::ENGLISH).diagnostics {
                wrong.push(format!("{}:{number}: {}", path.display(), told.message));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn written(text: &str) -> Vec<String> {
    let read = internal(&crate::LEXICON, text, 0, film_core::tables::ENGLISH);
    assert!(read.diagnostics.is_empty(), "{:?}", read.diagnostics);
    // (Each block's lines after its opening, then its `end`: as written.)
    fn flat(lines: &[film_core::ir::Instr], out: &mut Vec<String>) {
        for instr in lines {
            out.push(instr.text.clone());
            if let Some(inner) = &instr.inner {
                flat(inner, out);
                out.push("end".to_string());
            }
        }
    }
    let mut out = Vec::new();
    flat(&read.lines, &mut out);
    out.retain(|line| !line.is_empty());
    out
}

#[test]
fn values_tests_and_loops_are_the_engines() {
    let lines = written(
        "scene: beach a\nlet n = 2\nn = n * 3 + 1\nif n > 5 and chance(50)\n  jump a\nelse\n  a: Nothing.\nend\nwhile n > 0\n  n = n - 1\nend\na: {n} left.\n",
    );
    assert_eq!(
        lines,
        [
            "scene: beach a",
            ">set _n 2",
            ">calc _t1 _n * 3",
            ">calc _t2 _t1 + 1",
            ">set _n _t2",
            ">random _t3 0 99",
            ">set _t4 0",
            ">if _t3 < 50",
            ">set _t4 1",
            ">endif",
            ">if _n > 5 and _t4 != 0",
            "jump a",
            ">else",
            "a: Nothing.",
            ">endif",
            ">while _n > 0",
            ">calc _t5 _n - 1",
            ">set _n _t5",
            ">endwhile",
            "a: {_n} left.",
        ]
    );
}

#[test]
fn the_words_blocks_are_written_as_the_words_read_them() {
    let lines = written(
        "scene: beach a b\nchoice\noption Yes\n  a: Yes.\noption No\nend\nwhen a arrives : b jump\nfor each q, rank in [a, b]\n  q: I am {rank}.\nend\nevery random(3s, 4s) : sound click\nmemory\n  a: Before.\nend\n",
    );
    assert_eq!(
        lines,
        [
            "scene: beach a b",
            "choice Yes | No",
            "_answer 1",
            "a: Yes.",
            "_answer 2",
            "end",
            "when a arrives : jump b",
            "a: I am 1.",
            "b: I am 2.",
            "every 3s~4s sound click",
            "memory",
            "a: Before.",
            "end",
        ]
    );
}

#[test]
fn a_departure_and_countdowns_named() {
    let lines = written("scene: beach a b\ntimer bomb 3s\nwhen a leaves : b jump\nwhen timer(bomb) = 0 : b: Boom!\n");
    assert_eq!(
        lines,
        ["scene: beach a b", "timer bomb 3s", "beside", ">waitmoves a", "jump b", "end", "beside", ">waituntil timer_bomb == 0", "b: Boom!", "end",]
    );
    let (played, problems) =
        super::engine_lines("scene: beach pikachu piplup\ntimer bomb 3s\npikachu walk to right &\nwhen pikachu leaves : piplup jump\nwait 1s\n");
    assert!(problems.is_empty(), "{problems:?}");
    assert!(played.iter().any(|fields| fields[0] == "waitmoves") && played.iter().any(|fields| super::starts(fields, &["timer", "bomb"])), "{played:?}");
}

/// Every command of the engine is a word of the films, by its own name:
/// one that had no form of its own before is read by the form its docs say.
#[test]
fn every_command_of_the_engine_is_written_in_a_film() {
    let refused: Vec<&str> = engine::commands()
        .iter()
        .filter(|own| {
            let (_, problems) = super::engine_lines(&format!("scene: beach pikachu\n{own}\n"));
            problems.iter().any(|problem| problem.contains("does not know"))
        })
        .copied()
        .collect();
    assert!(refused.is_empty(), "{refused:?}");
    let (lines, problems) = super::engine_lines("scene: beach pikachu\nmusicspeed 120\n");
    assert!(problems.is_empty() && lines.iter().any(|fields| fields[0] == "musicspeed"), "{problems:?} {lines:?}");
}

/// A recipe's calculation binds as the language's do: `x + 2 * 3` is 7.
#[test]
fn a_recipe_reckons_each_sign_as_tight_as_it_binds() {
    let (lines, problems) = super::engine_lines("define lift x:number\n  rise pikachu {x+2*3} 1s\nend\nscene: beach pikachu\nlift 1\n");
    assert!(problems.is_empty(), "{problems:?}");
    assert!(lines.iter().any(|fields| super::starts(fields, &["rise", "pikachu", "7"])), "{lines:?}");
}

#[test]
fn decimals_are_kept() {
    let lines = written("scene: beach a\nlet x = 7 / 2\nlet y = x * 0.5\na: {y}\n");
    assert_eq!(lines, ["scene: beach a", ">set _x 3.5", ">calc _t1 _x * 0.5", ">set _y _t1", "a: {_y}"]);
}

#[test]
fn a_recipe_gives_back_a_value_from_anywhere() {
    // (A recipe stays as the language wrote it: its lines are written at each call.)
    let lines = written("define signe x:number -> number\n  if x < 0\n    return -1\n  end\n  return 1\nend\n");
    assert_eq!(lines, ["define signe x:number"]);
    // (Each call its own mark: called twice, two marks.)
    let (played, problems) = super::engine_lines(
        "define signe x:number -> number\n  if x < 0\n    return -1\n  end\n  return 1\nend\nscene: beach pikachu\nlet a = signe(-3)\nlet b = signe(2)\npikachu: {a} {b}\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
    let marks: Vec<&String> = played.iter().filter(|fields| fields[0] == "label" && fields[1].starts_with("_returned")).map(|fields| &fields[1]).collect();
    assert!(marks.len() == 2 && marks[0] != marks[1], "{marks:?}");
}

#[test]
fn a_recipe_gives_back_a_value() {
    let film = "define double x:number -> number\n  return x * 2\nend\n\nscene: beach pikachu\nlet n = double(4)\n";
    assert_eq!(written(film), ["define double x:number", "scene: beach pikachu", "double 4", ">set _n _return_double"]);
    // (Called with 4: worked out with what it was given, at once.)
    let (played, problems) = super::engine_lines(film);
    assert!(problems.is_empty(), "{problems:?}");
    assert!(played.iter().any(|fields| super::starts(fields, &["set", "_return_double", "8"])), "{played:?}");
}

/// A line every so often: a word's line, the engine's own `every`; a line
/// said, in a block of its own that says it each time; before a mark,
/// nothing, and it is told so.
#[test]
fn a_line_with_a_time_of_its_own() {
    assert_eq!(written("scene: beach pikachu\nevery 2s : sound click\n"), ["scene: beach pikachu", "every 2s sound click"]);
    let (lines, problems) = super::engine_lines("scene: beach pikachu\nevery 1s : pikachu: Oh\n");
    assert!(problems.is_empty(), "{problems:?}");
    assert!(lines.iter().any(|fields| fields[0] == "while") && lines.iter().any(|fields| fields[0] == "say"), "{lines:?}");
    let (_, problems) = super::engine_lines("scene: beach pikachu\nmark m\nat m - 2s : jump pikachu\n");
    assert!(problems.iter().any(|problem| problem.contains("nothing plays before a mark")), "{problems:?}");
    // (A time found as it plays: its typed wait, its number by its name.)
    let film = crate::tests::compile("scene: beach pikachu\nlet d = 30\nafter d : jump pikachu\n");
    assert!(film.problems.is_empty(), "{:?}", film.problems);
    let found: Vec<&engine::Beat> = film.events[0].scenes.iter().flat_map(|scene| &scene.beats).filter(|beat| beat.vars() == ["_d"]).collect();
    assert!(matches!(found[..], [engine::Beat::Flow(engine::FlowBeat::WaitFound(_))]), "{found:?}");
}

/// A name a recipe is given, in its words said: as the game writes it.
#[test]
fn a_recipe_says_a_name_as_the_game_writes_it() {
    let (lines, problems) = super::engine_lines("define hello who:pokemon\n  who: I am {who}!\nend\nscene: beach pikachu\npikachu hello\n");
    assert!(problems.is_empty(), "{problems:?}");
    assert!(lines.iter().any(|fields| fields[0] == "say" && fields[3].contains("I am [CS:N]Pikachu[CR]!")), "{lines:?}");
}

/// Who first, then the word (`pikachu walk to right`); a line said stays
/// one; a word not said of someone keeps its order.
#[test]
fn a_line_may_say_who_first() {
    let new = "scene: beach pikachu eevee\npikachu walk to right\neevee jump\npikachu attack ThunderShock at eevee\neevee emote joy\npikachu: Hello!\nframe pikachu eevee\nwait 1s\n";
    let new = crate::tests::compile(new);
    let wrong =
        |written: &crate::Written| written.problems.iter().filter(|problem| !problem.is_remark()).map(|problem| problem.message.clone()).collect::<Vec<_>>();
    assert!(wrong(&new).is_empty(), "{:?}", wrong(&new));
    assert!(crate::LEXICON.subject_first("pikachu", Some("walk"), ""));
    assert!(!crate::LEXICON.subject_first("walk", Some("pikachu"), ""));
    assert!(!crate::LEXICON.subject_first("pikachu", Some("music"), ""));
}

/// One order: who acts first. A line written the word first, its word said
/// of someone, is wrong (`E-ORDER`) with its fix; the fix made, nothing is
/// told and the film is the same. Who first, a word where nobody acts, a
/// line said: nothing told.
#[test]
fn the_old_order_is_told_with_its_fix() {
    let old =
        "scene: beach pikachu eevee\nwalk pikachu to right\njump eevee\nemote eevee joy &\nlook pikachu right\nframe pikachu eevee\nmusic beach\nwait 1s\n";
    let ordered = |text: &str| {
        crate::service::diagnostics(text, crate::tests::tables()).into_iter().filter(|told| told.code == Some(film_core::codes::OLD_ORDER)).collect::<Vec<_>>()
    };
    let told = ordered(old);
    let lines: Vec<usize> = told.iter().map(|one| one.span.start.line).collect();
    assert_eq!(lines, [2, 3, 4, 5], "{told:?}");
    assert!(told.iter().all(|one| one.is_error() && one.fixes.len() == 1), "{told:?}");
    let edits: Vec<language::Edit> = told.iter().flat_map(|one| one.fixes[0].edits.clone()).collect();
    let new = crate::service::applied(old, &edits);
    assert_eq!(
        new,
        "scene: beach pikachu eevee\npikachu walk to right\neevee jump\neevee emote joy &\npikachu look right\nframe pikachu eevee\nmusic beach\nwait 1s\n"
    );
    assert!(ordered(&new).is_empty(), "{:?}", ordered(&new));
    // (A team of several cannot be said who first: nothing told.)
    assert!(ordered("scene: beach pikachu eevee\nexit pikachu eevee\n").is_empty());
    assert_eq!(ordered("scene: beach pikachu eevee\nexit pikachu\n").len(), 1);
}

/// An engine's command reads a value left out by its kind (`bounce pikachu
/// 10`: a height, not the very word `height`), and someone or a point where
/// its docs say either (`lightning pikachu`, `lightning @ 100 80`).
#[test]
fn an_engine_command_reads_what_it_may_leave_out_by_its_kind() {
    for line in ["pikachu bounce 10", "pikachu bounce 10 0.5s", "pikachu bounce", "pikachu lightning", "lightning @ 100 80"] {
        let (played, problems) = super::engine_lines(&format!("scene: beach pikachu\n{line}\nwait 1s\n"));
        assert!(problems.is_empty(), "{line}: {problems:?}");
        let word = line.split_whitespace().find(|word| ["bounce", "lightning"].contains(word)).unwrap_or_default();
        assert!(played.iter().any(|fields| fields[0] == word), "{line}: {played:?}");
    }
    // (A value left out still says what it must be.)
    let (_, problems) = super::engine_lines("scene: beach pikachu\npikachu bounce high\n");
    assert!(!problems.is_empty());
}

/// A command of the engine's own that its form fits but that does not read
/// is told by the film at the very word that is wrong, with what is asked:
/// no longer by the engine's parser ("the engine does not read this line
/// (“particles” badly written (see COMMANDS.md))").
#[test]
fn an_engine_command_badly_written_is_told_at_its_word() {
    let film = "scene: beach pikachu\nparticles fireworks @ 300 150 40 pink\n";
    let written = crate::tests::compile(film);
    let problems: Vec<&engine::Diagnostic> = written.problems.iter().filter(|problem| !problem.is_remark()).collect();
    assert_eq!(problems.len(), 1, "{problems:?}");
    let problem = problems[0];
    assert!(problem.message.starts_with("“fireworks” is not a kind of particles (dust, sparks, bubbles…) (kind)"), "{}", problem.message);
    let at = film.find("fireworks").expect("its word");
    assert_eq!(problem.span.bytes, Some(language::Span::new(at, at + "fireworks".len())));
}
