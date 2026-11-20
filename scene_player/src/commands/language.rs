//! The language itself: a film put in order (`format`).

use std::path::PathBuf;

use ::film::Context;

use super::json::{self, Value, json};

/// The files named, or every film, recipe and model of the project.
pub(super) fn files(context: &Context, words: &[String]) -> Vec<PathBuf> {
    if !words.is_empty() {
        return words.iter().map(PathBuf::from).collect();
    }
    let root = context.root();
    let mut found = Vec::new();
    for folder in ["library/films", "library/films/recipes", "film/recipes", "film/recipes/dungeon"] {
        let Ok(entries) = std::fs::read_dir(root.join(folder)) else { continue };
        let mut these: Vec<PathBuf> =
            entries.flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|extension| extension == "film")).collect();
        these.sort();
        found.extend(these);
    }
    found
}

/// Writes each file again with `change`; says which changed.
fn rewritten(context: &Context, words: &[String], change: impl Fn(&str) -> String) -> Result<(), String> {
    let mut changed = 0;
    for path in files(context, words) {
        let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let made = change(&text);
        if made != text {
            std::fs::write(&path, &made).map_err(|error| format!("{}: {error}", path.display()))?;
            println!("{}", path.display());
            changed += 1;
        }
    }
    println!("{changed} file(s) written again");
    Ok(())
}

fn range_json(range: &::film::service::Location) -> Value {
    json!({"start": {"line": range.start.line, "column": range.start.column}, "end": {"line": range.end.line, "column": range.end.column}})
}

/// `steps <film>`: every step the film goes through, from its text to the
/// engine's events, told one after the other.
pub fn steps(context: &Context, name: &str) -> Result<(), String> {
    let path = ::film::locate(&context.library, name).ok_or_else(|| format!("film not found: {name}"))?;
    let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    print!("{}", ::film::steps::steps(&text, context.tables()));
    Ok(())
}

/// What the language says of a film's text, for a program: what is wrong
/// (range, weight, code, message, notes, fixes as changes of the text),
/// then its plan.
pub fn analysis(context: &Context, file: &str, text: &str) -> Value {
    let told: Vec<Value> = ::film::service::diagnostics(text, context.tables())
        .iter()
        .map(|one| {
            let fixes: Vec<Value> = one
                .fixes
                .iter()
                .map(|fix| {
                    let changes: Vec<Value> = fix.edits.iter().map(|change| json!({"range": range_json(&change.span), "text": change.text})).collect();
                    json!({"title": fix.title, "changes": changes})
                })
                .collect();
            json!({
                "range": range_json(&one.span),
                "severity": one.severity.word(),
                "code": one.code,
                "message": one.message,
                "notes": one.notes,
                "corrections": fixes,
            })
        })
        .collect();
    let plan: Vec<Value> = ::film::service::outline(text)
        .iter()
        .map(|symbol| json!({"kind": symbol.kind, "name": symbol.name, "detail": symbol.detail, "line": symbol.line, "end": symbol.end_line}))
        .collect();
    json::answer("analyze", json!({"file": json::path(context.root(), file), "diagnostics": told, "plan": plan}))
}

/// `analyze <film> [--json]`: what the language says of it, and its plan.
pub fn analyze(context: &Context, name: &str, as_json: bool) -> Result<(), String> {
    let path = ::film::locate(&context.library, name).ok_or_else(|| format!("film not found: {name}"))?;
    let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    if as_json {
        json::print(&analysis(context, &path.display().to_string(), &text));
        return Ok(());
    }
    let told = ::film::service::diagnostics(&text, context.tables());
    let plan = ::film::service::outline(&text);
    let file = path.display().to_string();
    for one in &told {
        println!("{file}:{}:{}: {} {}: {}", one.span.start.line, one.span.start.column, one.severity.word(), one.code.unwrap_or_default(), one.message);
        for note in &one.notes {
            println!("  = {note}");
        }
        for fix in &one.fixes {
            println!("  = correction: {}", fix.title);
        }
    }
    if told.is_empty() {
        println!("{file}: nothing to say");
    }
    println!();
    for symbol in &plan {
        let indent = if matches!(symbol.kind, "scene" | "recipe") { "" } else { "  " };
        let detail = if symbol.detail.is_empty() { String::new() } else { format!(" — {}", symbol.detail) };
        println!("{indent}{} {} (lines {}-{}){detail}", symbol.kind, symbol.name, symbol.line, symbol.end_line);
    }
    Ok(())
}

/// The films of the project: the library's, then each game's.
fn everything(context: &Context) -> Vec<PathBuf> {
    let root = context.root();
    let mut found = Vec::new();
    let mut folders = vec![root.join("library/films")];
    if let Ok(games) = std::fs::read_dir(root.join("games")) {
        let mut games: Vec<PathBuf> = games.flatten().map(|entry| entry.path().join("films")).filter(|films| films.is_dir()).collect();
        games.sort();
        folders.extend(games);
    }
    for folder in folders {
        let kind = "film";
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        let mut these: Vec<PathBuf> =
            entries.flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|extension| extension == kind)).collect();
        these.sort();
        found.extend(these);
    }
    found
}

/// A beat by the line it writes: what it is, whatever its types inside; one
/// no line writes, as it is.
pub(super) fn beat_line(beat: &engine::Beat) -> String {
    let words = &scenes::vocabulary::POKEMON;
    // (A line filled as it plays is that line.)
    let filled = matches!(beat, engine::Beat::Flow(engine::FlowBeat::Filled(_)));
    match (beat.written(words), beat.fields(words)) {
        (Some(fields), _) => fields.join("\t"),
        (None, Some(fields)) if filled => fields.join("\t"),
        (None, Some(fields)) => format!("~ {}\t{beat:?}", fields.join("\t")),
        (None, None) => format!("? {beat:?}"),
    }
}

/// What the engine is given of a file, written out: its events and their
/// beats, what is wrong, how its lines were read (a film's).
/// With the engine's words of the lines a film still hands over as fields.
fn print_of(context: &Context, path: &std::path::Path) -> (String, Vec<String>) {
    let words = &scenes::vocabulary::POKEMON;
    let mut left = Vec::new();
    let mut made = String::new();
    match ::film::read_any_with(path, words, context.tables()) {
        Ok((events, problems)) => {
            for problem in &problems {
                made += &format!("problem {problem}\n");
            }
            // (Each beat by the line it writes: what it is, whatever its
            // types inside; one no line writes, as it is.)
            for event in &events {
                made += &format!("event {} {} {:?}\n", event.number, event.name, event.moment);
                for scene in &event.scenes {
                    made += &format!("  scene {} {}\n", scene.level, scene.name);
                    for beat in &scene.beats {
                        made += &format!("    {}\n", beat_line(beat));
                    }
                }
            }
        }
        Err(error) => made += &format!("unreadable {error}\n"),
    }
    if path.extension().is_some_and(|kind| kind == "film")
        && let Ok(written) = ::film::Written::of_file_with(path, context.tables())
    {
        left = written.left_as_fields.iter().map(|(_, fields)| fields[0].clone()).collect();
        for (number, read) in &written.read_as {
            made += &format!("line {number} read {} ({}) {:?}\n", read.word, read.family, read.values);
        }
    }
    (made, left)
}

/// `fingerprint [--compare]`: what the engine is given of every film, scene,
/// demo and trial, written in `target/fingerprints/` (never committed); with
/// `--compare`, what changed since, told; with `--fields`, the lines the
/// films still hand over as fields, by the engine's word.
pub fn fingerprint(context: &Context, compare: bool, by_word: bool) -> Result<(), String> {
    let folder = context.root().join("target/fingerprints");
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let (mut same, mut changed, mut new, mut left) = (0, Vec::new(), 0, Vec::new());
    for path in everything(context) {
        // (A game's film by its game too: `my_game.my_film.film.txt`.)
        let game =
            path.strip_prefix(context.root().join("games")).ok().and_then(|inside| inside.iter().next()).map(|game| format!("{}.", game.to_string_lossy()));
        let name = format!(
            "{}{}.{}.txt",
            game.unwrap_or_default(),
            path.file_stem().map_or(String::new(), |stem| stem.to_string_lossy().into_owned()),
            path.extension().map_or(String::new(), |kind| kind.to_string_lossy().into_owned())
        );
        let kept = folder.join(&name);
        let (made, fields_left) = print_of(context, &path);
        left.extend(fields_left);
        if compare {
            match std::fs::read_to_string(&kept) {
                Ok(before) if before == made => same += 1,
                Ok(before) => {
                    let at = before.lines().zip(made.lines()).position(|(a, b)| a != b).unwrap_or(before.lines().count().min(made.lines().count()));
                    let was = before.lines().nth(at).unwrap_or("(end)").trim().to_string();
                    let now = made.lines().nth(at).unwrap_or("(end)").trim().to_string();
                    changed.push(format!("{name}: line {} of the fingerprint\n  before: {was}\n  after: {now}", at + 1));
                }
                Err(_) => new += 1,
            }
            continue;
        }
        std::fs::write(&kept, made).map_err(|error| format!("{}: {error}", kept.display()))?;
        same += 1;
    }
    told_left(&left, by_word);
    if !compare {
        println!("{same} fingerprints in {}", folder.display());
        return Ok(());
    }
    for told in &changed {
        println!("{told}");
    }
    println!("{same} the same, {} changed, {new} without a fingerprint of before", changed.len());
    if changed.is_empty() { Ok(()) } else { Err(format!("{} fingerprint(s) changed", changed.len())) }
}

/// `format [files…]`: put in order, as the Workshop does.
pub fn format(context: &Context, words: &[String]) -> Result<(), String> {
    rewritten(context, words, ::film::formatted)
}

/// How many lines of the films still go as fields, not beats; asked, how
/// many of each engine's word, the most first.
fn told_left(left: &[String], by_word: bool) {
    println!("{} lines of the films still in fields (not in beats)", left.len());
    if !by_word {
        return;
    }
    let mut counted: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for word in left {
        *counted.entry(word.as_str()).or_default() += 1;
    }
    let mut counted: Vec<(&str, usize)> = counted.into_iter().collect();
    counted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (word, count) in counted {
        println!("  {count:>6}  {word}");
    }
}

/// `ids <films>`: each line said that has no id yet given one, written last
/// (`#k3f9`); the ids there kept.
pub fn ids(context: &Context, words: &[String]) -> Result<(), String> {
    rewritten(context, words, |text| ::film::service::with_line_ids(text).0)
}
