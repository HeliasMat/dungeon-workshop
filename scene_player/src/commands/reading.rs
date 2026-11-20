//! A film read for what it holds: who plays in it (`cast`), where a
//! word is used (`usages`), what changed for the engine (`diff`).

use ::film::Context;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ::film::Written;
use engine::{Beat, Course, Role, Who};

use super::json::{self, Value, json};
use super::language::beat_line;
use super::renaming::{called, lines_read};

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn found(context: &Context, name: &str) -> Result<PathBuf, String> {
    ::film::locate(&context.library, name).ok_or_else(|| format!("film not found: {name}"))
}

/// The place of each scene, as its `scene:` line writes it (when the film
/// has as many as the engine is given).
fn places_of(text: &str, scenes: usize) -> Vec<String> {
    let written: Vec<String> = language::parse(text).file.scenes.iter().map(|scene| scene.args.first().map_or(String::new(), |arg| arg.text.clone())).collect();
    if written.len() == scenes { written } else { Vec::new() }
}

/// The names its `scene:` line gives.
fn listed_of(text: &str) -> Vec<Vec<String>> {
    language::parse(text).file.scenes.iter().map(|scene| scene.args.iter().skip(1).map(|arg| arg.text.clone()).collect()).collect()
}

/// Someone, not a spot, a thing or an item.
fn someone(who: &Who) -> bool {
    matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. })
}

#[derive(Default, Clone, Copy)]
struct Part {
    lines: usize,
    beats: usize,
}

/// `cast <film>`: who is on stage in each scene, how many lines
/// each says and in how many beats each acts; then the whole film.
pub fn cast(context: &Context, name: &str, as_json: bool) -> Result<(), String> {
    let say = |line: String| {
        if !as_json {
            println!("{line}");
        }
    };
    let roles = |parts: &[(String, Part)]| -> Vec<Value> {
        parts.iter().map(|(name, part)| json!({"name": name, "lines_said": part.lines, "beats": part.beats})).collect()
    };
    let mut scenes_told: Vec<Value> = Vec::new();
    let path = found(context, name)?;
    let text = read(&path)?;
    let written = ::film::compile_with(&text, context.tables());
    // (A name the film gives, `merchant = kecleon`, before its species'.)
    let given: Vec<String> = language::parse(&text)
        .file
        .head
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            language::ast::Kind::Naming { name, .. } => Some(name.text.clone()),
            _ => None,
        })
        .collect();
    let called_by = |who: &Who| {
        let code = who.to_string();
        let mut names = written.names.iter().filter(|(known, _)| *known == code).map(|(_, name)| name);
        let first = names.clone().next();
        names.find(|name| given.contains(name)).or(first).map_or(code, |name| name.clone())
    };
    let scenes: Vec<&engine::Scene> = written.events.iter().flat_map(|event| &event.scenes).collect();
    let places = places_of(&text, scenes.len());
    let listed = if places.is_empty() { Vec::new() } else { listed_of(&text) };
    let mut film: BTreeMap<String, Part> = BTreeMap::new();
    for (at, scene) in scenes.iter().enumerate() {
        let (mut parts, mut came, mut told): (BTreeMap<Who, Part>, Vec<Who>, usize) = (BTreeMap::new(), Vec::new(), 0);
        for beat in &scene.beats {
            if let Beat::Course(Course::Say { entity, .. }) = beat {
                match entity {
                    Some(who) => parts.entry(*who).or_default().lines += 1,
                    None => told += 1,
                }
            }
            let mut acting = beat.naming(Role::Acts);
            acting.sort();
            acting.dedup();
            for who in acting.into_iter().filter(someone) {
                parts.entry(who).or_default().beats += 1;
            }
            for who in beat.naming(Role::Comes).into_iter().filter(someone) {
                parts.entry(who).or_default();
                if !came.contains(&who) && !written.crowd.contains(&who.to_string()) {
                    came.push(who);
                }
            }
        }
        let place = places.get(at).cloned().unwrap_or_else(|| scene.name.clone());
        say(format!("scene {} — {place}", at + 1));
        let mut shown: Vec<(String, Part)> =
            parts.iter().filter(|(who, _)| !written.crowd.contains(&who.to_string())).map(|(who, part)| (called_by(who), *part)).collect();
        shown.sort_by(|a, b| (b.1.lines + b.1.beats).cmp(&(a.1.lines + a.1.beats)).then(a.0.cmp(&b.0)));
        for (name, part) in &shown {
            say(format!("  {name:<16} {:>3} line(s)  {:>4} beat(s)", part.lines, part.beats));
            let all = film.entry(name.clone()).or_default();
            all.lines += part.lines;
            all.beats += part.beats;
        }
        let crowd = parts.keys().filter(|who| written.crowd.contains(&who.to_string())).count();
        if crowd > 0 {
            say(format!("  (and {crowd} of the crowd)"));
        }
        if told > 0 {
            say(format!("  story            {told:>3} line(s)"));
        }
        // (On the `scene:` line, or brought in, and nothing from them.)
        let mut idle: Vec<String> = came.iter().filter(|who| parts.get(who).is_none_or(|part| part.lines + part.beats == 0)).map(called_by).collect();
        for name in listed.get(at).into_iter().flatten() {
            if !idle.contains(name)
                && !shown.iter().any(|(known, part)| known == name && part.lines + part.beats > 0)
                && written.names.iter().any(|(_, known)| known == name)
            {
                idle.push(name.clone());
            }
        }
        for name in &idle {
            say(format!("  ! {name}: present but silent and still"));
        }
        scenes_told.push(json!({"number": at + 1, "place": place, "roles": roles(&shown), "crowd": crowd, "story": told, "silent": idle}));
    }
    let mut all: Vec<(String, Part)> = film.into_iter().collect();
    all.sort_by(|a, b| (b.1.lines + b.1.beats).cmp(&(a.1.lines + a.1.beats)).then(a.0.cmp(&b.0)));
    say(format!("\nthe whole film ({} scene(s))", scenes.len()));
    for (name, part) in &all {
        say(format!("  {name:<16} {:>3} line(s)  {:>4} beat(s)", part.lines, part.beats));
    }
    if as_json {
        json::print(&json::answer("cast", json!({"film": stem_of(&path), "scenes": scenes_told, "whole_film": roles(&all)})));
    }
    Ok(())
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map_or(String::new(), |stem| stem.to_string_lossy().into_owned())
}

/// `usages <word> [--max N]`: every line of the films and recipes that
/// calls that word, film by film.
pub fn usages(context: &Context, word: &str, most: usize, as_json: bool) -> Result<(), String> {
    let say = |line: String| {
        if !as_json {
            println!("{line}");
        }
    };
    let mut files_told: Vec<Value> = Vec::new();
    let wanted = ::film::words::plain(word);
    let root = context.root().to_path_buf();
    let (mut count, mut shown, mut films) = (0, 0, 0);
    for path in super::language::files(context, &[]) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let written = Written::of_file_with(&path, context.tables()).ok();
        let mut here = Vec::new();
        for (number, _, line) in lines_read(&text) {
            let word = match written.as_ref().and_then(|written| written.read_as.get(&number)) {
                Some(read) => Some(read.word.clone()),
                None => called(&line).map(str::to_string),
            };
            if word.is_some_and(|word| ::film::words::plain(&word) == wanted) {
                here.push(number);
            }
        }
        if here.is_empty() {
            continue;
        }
        films += 1;
        count += here.len();
        let shown_path = path.strip_prefix(&root).unwrap_or(&path).display().to_string().replace('\\', "/");
        let stem = path.file_stem().map_or(String::new(), |stem| stem.to_string_lossy().into_owned());
        if shown < most {
            say(format!("{shown_path} ({})", here.len()));
        }
        let lines: Vec<&str> = text.lines().collect();
        let used: Vec<Value> = here.iter().map(|number| json!({"line": number, "text": lines.get(number - 1).map_or("", |line| line.trim())})).collect();
        files_told.push(json!({"path": shown_path, "film": stem, "lines": used}));
        for number in here {
            if shown < most {
                say(format!("  {stem}:{number}  {}", lines.get(number - 1).map_or("", |line| line.trim())));
            }
            shown += 1;
        }
    }
    if count == 0 {
        let words: Vec<&str> = ::film::every_word().map(|(_, word, ..)| word).collect();
        return Err(format!("no line uses “{word}”{}", ::film::check::nearby(word, words.into_iter())));
    }
    if as_json {
        json::print(&json::answer("usages", json!({"word": word, "total": count, "files": files_told})));
        return Ok(());
    }
    if count > most {
        say(format!("… and {} more", count - most));
    }
    say(format!("{count} line(s) in {films} file(s)"));
    Ok(())
}

/// What the engine is given of a film's text, scene by scene, each beat by
/// its line (the lines the film counts left out: they move with any line).
fn given(context: &Context, text: &str) -> Vec<(String, Vec<String>)> {
    let written = ::film::compile_with(text, context.tables());
    let scenes: Vec<&engine::Scene> = written.events.iter().flat_map(|event| &event.scenes).collect();
    let places = places_of(text, scenes.len());
    scenes
        .iter()
        .enumerate()
        .map(|(at, scene)| {
            let beats = scene.beats.iter().filter(|beat| !matches!(beat, Beat::Flow(engine::FlowBeat::Line { .. }))).map(beat_line).collect();
            (places.get(at).cloned().unwrap_or_else(|| scene.name.clone()), beats)
        })
        .collect()
}

/// What turns `old` into `new`: each line kept, gone or come.
#[derive(Clone, Copy, PartialEq)]
enum Step {
    Kept(usize),
    Gone(usize),
    Came(usize),
}

fn steps(old: &[String], new: &[String]) -> Vec<Step> {
    let (n, m) = (old.len(), new.len());
    let mut longest = vec![0u32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            longest[at(i, j)] = if old[i] == new[j] { longest[at(i + 1, j + 1)] + 1 } else { longest[at(i + 1, j)].max(longest[at(i, j + 1)]) };
        }
    }
    let (mut i, mut j, mut made) = (0, 0, Vec::new());
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            made.push(Step::Kept(i));
            i += 1;
            j += 1;
        } else if j < m && (i == n || longest[at(i, j + 1)] >= longest[at(i + 1, j)]) {
            made.push(Step::Came(j));
            j += 1;
        } else {
            made.push(Step::Gone(i));
            i += 1;
        }
    }
    made
}

/// `diff <film> [--from <rev>]`: what the engine is given now
/// against the film as git kept it.
pub fn diff(context: &Context, name: &str, since: Option<&str>, as_json: bool) -> Result<(), String> {
    let say = |line: String| {
        if !as_json {
            println!("{line}");
        }
    };
    let told = |state: &str, scenes: Vec<Value>, relative: &str, since: &str| {
        if as_json {
            json::print(&json::answer("diff", json!({"film": relative, "since": since, "state": state, "scenes": scenes})));
        }
    };
    let since = since.unwrap_or("HEAD");
    let path = found(context, name)?;
    let now = read(&path)?;
    let root = context.root().to_path_buf();
    let full = |path: &Path| std::fs::canonicalize(path).map_err(|error| format!("{}: {error}", path.display()));
    let relative = full(&path)?.strip_prefix(full(&root)?).map_err(|_| format!("{} is not in the repository", path.display()))?.to_path_buf();
    let relative = relative.display().to_string().replace('\\', "/");
    let shown = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("show")
        .arg(format!("{since}:{relative}"))
        .output()
        .map_err(|error| format!("git: {error}"))?;
    let now_given = given(context, &now);
    if !shown.status.success() {
        let beats: usize = now_given.iter().map(|(_, beats)| beats.len()).sum();
        say(format!("{relative}: new, not in {since} ({} scene(s), {beats} beats)", now_given.len()));
        told("new", Vec::new(), &relative, since);
        return Ok(());
    }
    let was = String::from_utf8_lossy(&shown.stdout).replace("\r\n", "\n");
    if was == now.replace("\r\n", "\n") {
        say(format!("{relative}: the same text as in {since}"));
        told("same_text", Vec::new(), &relative, since);
        return Ok(());
    }
    let was_given = given(context, &was);
    if was_given == now_given {
        say(format!("{relative}: the text has changed since {since}, but the engine gets the same thing"));
        told("same_engine", Vec::new(), &relative, since);
        return Ok(());
    }
    let mut summary = Vec::new();
    let mut scenes_told: Vec<Value> = Vec::new();
    for at in 0..was_given.len().max(now_given.len()) {
        let empty = (String::new(), Vec::new());
        let (place, before) = was_given.get(at).unwrap_or(&empty);
        let (place_now, after) = now_given.get(at).unwrap_or(&empty);
        let place = if place_now.is_empty() { place } else { place_now };
        if before == after {
            continue;
        }
        let made = steps(before, after);
        let near = |index: usize| made[index.saturating_sub(2)..(index + 3).min(made.len())].iter().any(|step| !matches!(step, Step::Kept(_)));
        let (gone, came) = (made.iter().filter(|step| matches!(step, Step::Gone(_))).count(), made.iter().filter(|step| matches!(step, Step::Came(_))).count());
        let state = match (was_given.get(at), now_given.get(at)) {
            (None, _) => " (new)",
            (_, None) => " (gone)",
            _ => "",
        };
        say(format!("=== scene {} — {place}{state}", at + 1));
        let mut skipped = false;
        for (index, step) in made.iter().enumerate() {
            if !near(index) {
                skipped = true;
                continue;
            }
            if skipped {
                say("  …".to_string());
                skipped = false;
            }
            match *step {
                Step::Kept(i) => say(format!("  {}", cut(&before[i]))),
                Step::Gone(i) => say(format!("- {}", cut(&before[i]))),
                Step::Came(j) => say(format!("+ {}", cut(&after[j]))),
            }
        }
        summary.push(format!("scene {}: {} beats changed ({gone} removed, {came} added)", at + 1, gone.max(came)));
        let removed: Vec<&String> = made.iter().filter_map(|step| if let Step::Gone(i) = step { Some(&before[*i]) } else { None }).collect();
        let added: Vec<&String> = made.iter().filter_map(|step| if let Step::Came(j) = step { Some(&after[*j]) } else { None }).collect();
        let state = match state {
            " (new)" => "new",
            " (gone)" => "gone",
            _ => "changed",
        };
        scenes_told.push(json!({"number": at + 1, "place": place, "state": state, "removed": removed, "added": added}));
    }
    told("change", scenes_told, &relative, since);
    say(String::new());
    say(format!("since {since}:"));
    for line in summary {
        say(format!("  {line}"));
    }
    Ok(())
}

/// A beat's line, cut when it is too long to be read (a ground's rows).
fn cut(line: &str) -> String {
    const LONGEST: usize = 160;
    match line.char_indices().nth(LONGEST) {
        Some((at, _)) => format!("{}…", &line[..at]),
        None => line.to_string(),
    }
}
