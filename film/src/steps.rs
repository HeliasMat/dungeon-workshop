//! Every step a film goes through, told one after the other, as text
//! (`scene_player steps <film>`): its text, what the language reads, the
//! tree of its lines, its recipes played out, its scenes, the words read,
//! the beats they make, the engine's events, what is wrong.

use std::fmt::Write;

use language::ast::{Kind, Opening, Stmt};

use film_core::compile::scenes::read;
use film_core::film::Film;
use film_core::ir::{Instr, Shape};
use film_core::text::front::{holds, internal};
use film_core::text::recipes::unfold;

/// Every step of a film, told.
pub fn steps(text: &str, tables: std::sync::Arc<film_core::tables::Tables>) -> String {
    let mut out = String::new();
    let title = |out: &mut String, step: &str, what: &str| {
        let _ = writeln!(out, "\n=== {step}  ({what})");
    };

    title(&mut out, "1. The text", "what you write");
    for (at, line) in text.lines().enumerate() {
        let _ = writeln!(out, "{:>4} | {line}", at + 1);
    }

    title(&mut out, "2. The language", "language: each line read once, as a tree");
    let parsed = language::parse_with(text, &holds);
    let source = language::Source::new(text);
    let line = |span: language::Span| source.position(span.start).line;
    for stmt in &parsed.file.head {
        told_stmt(&mut out, stmt, 1, &line);
    }
    for scene in &parsed.file.scenes {
        let args: Vec<&str> = scene.args.iter().map(|arg| arg.text.as_str()).collect();
        let _ = writeln!(out, "{:>4}   scene: {}", line(scene.span), args.join(" "));
        for stmt in &scene.body {
            told_stmt(&mut out, stmt, 2, &line);
        }
    }
    for wrong in &parsed.diagnostics {
        let _ = writeln!(out, "{:>4}   ! {}", line(wrong.bytes()), wrong.message);
    }

    title(&mut out, "3. The film's tree", "text::front: each line what it is (ir::Shape), a recipe kept as it is");
    for instr in internal(&crate::LEXICON, text, 0, film_core::tables::ENGLISH).lines.iter().filter(|instr| instr.shape != Shape::Blank) {
        told_instr(&mut out, instr, 1);
    }

    title(&mut out, "4. The recipes played", "text::recipes: each call written with what it is given, “there” and “team” put in");
    let mut film = Film::with(&crate::LEXICON, tables.clone());
    for instr in unfold(text, &mut film).iter().filter(|instr| instr.shape != Shape::Blank) {
        told_instr(&mut out, instr, 1);
    }

    title(&mut out, "5. The scenes", "compile::scenes: each scene, its place, those it names");
    let mut film = Film::with(&crate::LEXICON, tables.clone());
    for (at, scene) in read(text, &mut film).iter().enumerate() {
        let (level, x, y) = scene.place;
        let _ = writeln!(
            out,
            "      scene {}: place {level} (middle {x} {y}), {}; opened by “{}”; {} lines",
            at + 1,
            if scene.listed.is_empty() { "no one named".to_string() } else { scene.listed.join(", ") },
            scene.transition,
            scene.lines.len()
        );
    }

    let written = crate::compile_with(text, tables);
    title(&mut out, "6. The words read", "grammar::bind: the form that fits, its typed values, where they are written");
    for (number, read) in &written.read_as {
        let values: Vec<String> = read
            .values
            .iter()
            .filter(|value| !value.words.is_empty())
            .map(|value| format!("{}={} ({:?})", value.name, value.words.join(" "), value.kind))
            .collect();
        let _ = writeln!(out, "{number:>4}   “{}” of the family {}: {}", read.word, read.family, values.join(", "));
    }

    title(&mut out, "7. The beats written", "emit: what each line has the engine play, typed (written here in its words)");
    let mut last = None;
    for (number, fields) in &written.lines {
        if fields.first().is_some_and(|word| word == "filmline") {
            continue;
        }
        let shown = if Some(*number) == last { "    ".to_string() } else { format!("{number:>4}") };
        last = Some(*number);
        let _ = writeln!(out, "{shown}   {}", fields.join(" "));
    }

    title(&mut out, "8. The engine's events", "Written::events: what the engine plays, with no language read again");
    for event in &written.events {
        let _ = writeln!(out, "      event {} “{}”", event.number, event.name);
        for scene in &event.scenes {
            let _ = writeln!(out, "        scene “{}”, place {}: {} beats", scene.name, scene.level, scene.beats.len());
            for beat in scene.beats.iter().take(6) {
                let _ = writeln!(out, "          {beat:?}");
            }
            if scene.beats.len() > 6 {
                let _ = writeln!(out, "          … ({} more)", scene.beats.len() - 6);
            }
        }
    }

    title(&mut out, "9. What is wrong", "the problems, each on its line (and its words, when it quotes them)");
    if written.problems.is_empty() {
        let _ = writeln!(out, "      nothing");
    }
    for problem in &written.problems {
        let at = problem.span.bytes.map_or(String::new(), |bytes| format!(" [“{}”]", bytes.of(text)));
        let _ = writeln!(out, "      {problem}{at}");
    }
    out
}

/// A line as the language read it.
fn told_stmt(out: &mut String, stmt: &Stmt, depth: usize, line: &impl Fn(language::Span) -> usize) {
    let indent = "  ".repeat(depth);
    let said = match &stmt.kind {
        Kind::Blank | Kind::Comment(_) => return,
        Kind::Title(title) => format!("title: {title}"),
        Kind::Moment(moment) => format!("moment: {moment}"),
        Kind::Uses(name) => format!("uses {}", name.text),
        Kind::Includes(name) => format!("include {}", name.text),
        Kind::For(target) => format!("written for {}", target.text),
        Kind::Naming { name, species } => format!("name given: {} = {}", name.text, species.text),
        Kind::Said(said) => format!("line of {}: “{}”", said.who.text, said.text),
        Kind::Call(call) => {
            let args: Vec<&str> = call.args.iter().map(|arg| arg.text.as_str()).collect();
            format!("word “{}”: {}", call.word.text, args.join(" "))
        }
        Kind::Let { kept, name, .. } => format!("{} {}", if *kept { "keep" } else { "let" }, name.text),
        Kind::Set { name, .. } => format!("{} = …", name.text),
        Kind::Return(_) => "return".to_string(),
        Kind::Break => "break".to_string(),
        Kind::Continue => "next".to_string(),
        Kind::Block(block) => {
            let opening = match &block.opening {
                Opening::Recipe(recipe) => format!("recipe “{}”", recipe.form.word.text),
                Opening::Word(call) => format!("block of the word “{}”", call.word.text),
                other => format!("block {}", format!("{other:?}").split(['(', ' ', '{']).next().unwrap_or("")),
            };
            let _ = writeln!(out, "{:>4} {indent}{opening}", line(stmt.span));
            for inner in &block.body {
                told_stmt(out, inner, depth + 1, line);
            }
            for part in &block.parts {
                for inner in &part.body {
                    told_stmt(out, inner, depth + 1, line);
                }
            }
            return;
        }
        Kind::Unreadable(text) => format!("unreadable: {text}"),
    };
    let beside = if stmt.beside.is_some() { "  (beside the film)" } else { "" };
    let _ = writeln!(out, "{:>4} {indent}{said}{beside}", line(stmt.span));
}

/// A line of the film's tree, what it is.
fn told_instr(out: &mut String, instr: &Instr, depth: usize) {
    let indent = "  ".repeat(depth);
    let said = match &instr.shape {
        Shape::Word { verb, words, wait } => format!("Word “{verb}” {}{}", words.join(" "), if *wait { "" } else { "  (beside)" }),
        Shape::Said { who, brackets, words, beside } => {
            let manner = brackets.as_ref().map_or(String::new(), |brackets| format!(" ({brackets})"));
            format!("Line of {who}{manner}: “{words}”{}", if *beside { "  (beside)" } else { "" })
        }
        Shape::Engine(fields) => format!("Engine: {}", fields.join(" ")),
        Shape::Beat(_) => format!("Beat: {}", instr.text.trim_start_matches('>')),
        Shape::Arrives { who, then } => format!("When {} arrives{}", who.join(" "), then.as_ref().map_or(String::new(), |then| format!(": {}", then.text))),
        Shape::AtMark { name, after, then } if name.starts_with('@') => {
            let clock = if name == film_core::ir::MUSIC_CLOCK { "of the music" } else { "of the scene" };
            format!("At {after} frames {clock}{}", then.as_ref().map_or(String::new(), |then| format!(": {}", then.text)))
        }
        Shape::AtMark { name, after, then } => {
            format!("At the mark {name} + {after} frames{}", then.as_ref().map_or(String::new(), |then| format!(": {}", then.text)))
        }
        Shape::WhenLife { who, left, lines } => format!("When {who} has {left} % of health: {} lines", lines.len()),
        Shape::Choice { options } => format!("Choice: {}", options.join(" | ")),
        Shape::Title(title) => format!("Title: {title}"),
        Shape::Transition(how) => format!("Transition: {how}"),
        Shape::Naming { name, species } => format!("Name given: {name} = {species}"),
        Shape::Opens { place, listed } => format!("Scene: {place}, with {}", if listed.is_empty() { "no one".to_string() } else { listed.join(", ") }),
        Shape::Recipe(recipe) => format!("Recipe “{}” (kept as it is, written at each call)", recipe.form.word.text),
        Shape::Blank => return,
    };
    let _ = writeln!(out, "{:>4} {indent}{said}", instr.number);
    for inner in instr.inner.iter().flatten().filter(|inner| inner.shape != Shape::Blank) {
        told_instr(out, inner, depth + 1);
    }
}
