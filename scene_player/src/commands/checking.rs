//! What is wrong in a film: as it reads, as it plays.

use std::sync::Arc;

use ::film::Context;
use std::fmt::Write;
use std::path::Path;

use engine::{Diagnostic, Seen, Watch};

use super::Piece;
use super::json::{self, Value, json};
use preview::played::{Pass, Timeline, seconds};

/// What is wrong in a film by reading it: its words, then the engine's lines.
pub fn read_through(piece: &Piece, data: &str) -> Vec<Diagnostic> {
    // (The game's story is read as the game reads it: nothing to refuse.)
    if preview::film::story_event(&piece.path).is_some() {
        return Vec::new();
    }
    match &piece.written {
        Some(written) => preview::check::read_through(written, data),
        // (A scene of the engine's: its lines as the engine reads them, then checked.)
        None => match ::film::lines_of(Path::new(&piece.path), &scenes::vocabulary::POKEMON, piece.tables.clone()) {
            Ok((lines, problems)) => preview::check::checker(data).check_script(engine::Script { lines, problems }),
            Err(error) => vec![Diagnostic::said(error)],
        },
    }
}

/// A problem read, as a line: its file, its line and column, the words it is about.
pub fn located(piece: &Piece, problem: &Diagnostic) -> String {
    json::Told::read(problem).located(piece.tables.project.root(), &piece.path, &piece.text, &|text, mentions| piece.says(text, mentions))
}

/// What is wrong, as it reads: told, how many; none if it can be played.
pub fn refused(piece: &Piece, data: &str) -> Result<Vec<Diagnostic>, String> {
    let problems = read_through(piece, data);
    let mut wrong: Vec<&Diagnostic> = problems.iter().filter(|problem| !problem.is_remark()).collect();
    // (In the order of the file, as a compiler tells them.)
    wrong.sort_by_key(|problem| match problem.line() {
        0 => usize::MAX,
        line => line,
    });
    for problem in &wrong {
        println!("{}", located(piece, problem));
    }
    match wrong.len() {
        0 => Ok(problems),
        count => Err(format!("{}: {count} problem(s), the film does not play", piece.name)),
    }
}

/// What a rehearsal found, and when things happen.
pub struct Told {
    pub timeline: Timeline,
    pub wrong: Vec<Seen>,
    pub remarks: Vec<Seen>,
}

/// A film played at once, watched.
pub fn rehearsed(piece: &Piece) -> Result<Told, String> {
    rehearsed_film(piece, piece.film()?)
}

/// That film of the piece played at once, watched.
pub fn rehearsed_film(piece: &Piece, film: preview::film::Film) -> Result<Told, String> {
    let mut pass = Pass::new(film);
    let mut seen = Watch::default();
    seen.crowd = piece.written.iter().flat_map(|written| written.crowd.iter().filter_map(|code| engine::Who::parse(code))).collect();
    while pass.step() {
        pass.film.watched(&mut seen);
    }
    Ok(Told { timeline: pass.timeline, wrong: seen.problems, remarks: seen.remarks })
}

/// A number of something, with its plural.
fn count(number: usize, word: &str) -> String {
    format!("{number} {word}{}", if number > 1 { "s" } else { "" })
}

/// The film's summary, then scene by scene what counts, written in `out`.
/// What was wrong.
pub fn show(out: &mut String, piece: &Piece, told: &Told, problems: &[Diagnostic], all: bool) -> usize {
    let read_remarks: Vec<&Diagnostic> = problems.iter().filter(|problem| problem.is_remark()).collect();
    let (small, large): (Vec<&Seen>, Vec<&Seen>) = told.remarks.iter().partition(|seen| seen.small);
    let timeline = &told.timeline;
    // (How lines were read, `there`, `team`: details too.)
    let notes: &[(usize, String)] = piece.written.as_ref().map_or(&[], |written| written.notes.as_slice());
    // (What reading the film found to look at: details too.)
    let details = small.len() + notes.len() + read_remarks.len();
    let more = if details == 0 { String::new() } else { format!(", {} (--all)", count(details, "detail")) };
    _ = writeln!(
        out,
        "{}: {} s, {}, {}, {}{more}",
        piece.name,
        seconds(timeline.frames),
        count(timeline.scenes.len(), "scene"),
        count(told.wrong.len(), "problem"),
        count(large.len(), "remark")
    );
    for problem in read_remarks.iter().filter(|_| all) {
        _ = writeln!(out, "  - {}", located(piece, problem));
    }
    for (line, note) in notes.iter().filter(|_| all) {
        _ = writeln!(out, "  - {}", located(piece, &Diagnostic::at(*line, note.clone()).as_remark()));
    }
    let shown: Vec<&Seen> = told.wrong.iter().chain(large).chain(small.into_iter().filter(|_| all)).collect();
    let mut number = 0;
    for (at, scene) in timeline.scenes.iter().enumerate() {
        _ = writeln!(out, "scene {}: from {} s to {} s", at + 1, seconds(scene.start), seconds(timeline.end_of(at + 1)));
        let mut here: Vec<&&Seen> = shown.iter().filter(|seen| timeline.scene_at(seen.frame as usize) == at + 1).collect();
        here.sort_by_key(|seen| seen.frame);
        for seen in here {
            number += usize::from(!seen.small);
            let mark = if seen.small { "-".to_string() } else { format!("{number}.") };
            let told = json::Told::seen(seen, !seen.small, at + 1);
            _ = writeln!(out, "  {mark} {}", told.located(piece.tables.project.root(), &piece.path, &piece.text, &|text, mentions| piece.says(text, mentions)));
        }
    }
    told.wrong.len()
}

/// Each film read and played through, what is wrong in each told: several
/// at once, one a core, told in the order given.
pub fn check(films: &[&String], context: &Arc<Context>, all: bool) -> Result<(), String> {
    let data = context.data.as_str();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done = std::sync::Mutex::new(vec![None; films.len()]);
    let workers = std::thread::available_parallelism().map_or(1, |cores| cores.get()).min(films.len()).max(1);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let at = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(film) = films.get(at) else {
                        return;
                    };
                    let mut out = String::new();
                    let played = Piece::find(context, film).and_then(|piece| played_through(&mut out, &piece, data, all));
                    done.lock().unwrap_or_else(|poisoned| poisoned.into_inner())[at] = Some((out, played));
                }
            });
        }
    });
    let mut wrong = Vec::new();
    for (out, played) in done.into_inner().unwrap_or_else(|poisoned| poisoned.into_inner()).into_iter().flatten() {
        print!("{out}");
        if let Err(why) = played {
            println!("{why}");
            wrong.push(why);
        }
    }
    match wrong.as_slice() {
        [] => Ok(()),
        [one] => Err(one.clone()),
        many => Err(format!("{} films of {} do not play: {}", many.len(), films.len(), many.join("; "))),
    }
}

/// A film read, played through, and told in `out`.
pub fn played_through(out: &mut String, piece: &Piece, data: &str, all: bool) -> Result<(), String> {
    let problems = refused(piece, data)?;
    let told = rehearsed(piece)?;
    match show(out, piece, &told, &problems, all) {
        0 => Ok(()),
        wrong => Err(format!("{}: {wrong} problem(s)", piece.name)),
    }
}

/// What is wrong in a film, for a program: what reading it found, then
/// (`play`, and nothing wrong read) what playing it through saw; its
/// length and its scenes. See "For agents" in `film/FILMS.md`.
pub fn checked(piece: &Piece, data: &str, play: bool) -> Value {
    let problems = read_through(piece, data);
    let says = |text: &str, mentions: &[engine::Mention]| piece.says(text, mentions);
    let mut wrong: Vec<Value> = Vec::new();
    let mut remarks: Vec<Value> = Vec::new();
    let mut details: Vec<Value> = Vec::new();
    for problem in &problems {
        let told = json::Told::read(problem).json(&piece.text, &says);
        if problem.is_remark() { remarks.push(told) } else { wrong.push(told) }
    }
    let notes: &[(usize, String)] = piece.written.as_ref().map_or(&[], |written| written.notes.as_slice());
    for (line, note) in notes {
        let problem = Diagnostic::at(*line, note.clone()).as_remark();
        let mut told = json::Told::read(&problem);
        told.weight = "detail";
        details.push(told.json(&piece.text, &says));
    }
    let mut fields = json!({"film": piece.name, "path": json::path(piece.tables.project.root(), &piece.path)});
    if play && wrong.is_empty() {
        match piece.film().and_then(|film| rehearsed_film(piece, film)) {
            Ok(told) => {
                let timeline = &told.timeline;
                for (seen, is_wrong) in told.wrong.iter().map(|seen| (seen, true)).chain(told.remarks.iter().map(|seen| (seen, false))) {
                    let value = json::Told::seen(seen, is_wrong, timeline.scene_at(seen.frame as usize)).json(&piece.text, &says);
                    match (is_wrong, seen.small) {
                        (true, _) => wrong.push(value),
                        (false, true) => details.push(value),
                        (false, false) => remarks.push(value),
                    }
                }
                let scenes: Vec<Value> = timeline
                    .scenes
                    .iter()
                    .enumerate()
                    .map(|(at, scene)| json!({"number": at + 1, "start_s": json::seconds(scene.start), "end_s": json::seconds(timeline.end_of(at + 1))}))
                    .collect();
                fields["plays"] = json!(true);
                fields["duration_s"] = json!(json::seconds(timeline.frames));
                fields["scenes"] = json!(scenes);
            }
            Err(why) => wrong.push(json!({"code": ::film::codes::READ_WRONG, "severity": "error", "source": "play", "line": 0, "column": 1, "message": why})),
        }
    } else {
        fields["plays"] = json!(false);
    }
    fields["ok"] = json!(wrong.is_empty());
    fields["problems"] = json!(wrong);
    fields["remarks"] = json!(remarks);
    fields["details"] = json!(details);
    fields
}

/// `check <films…> --json [--no-play]`: one line of JSON, every film;
/// fails if one has problems.
pub fn check_json(films: &[&String], context: &Arc<Context>, play: bool) -> Result<(), String> {
    let data = context.data.as_str();
    let mut checked_films = Vec::new();
    let mut wrong = 0;
    for film in films {
        let value = match Piece::find(context, film) {
            Ok(piece) => checked(&piece, data, play),
            Err(why) => {
                json!({"film": film, "ok": false, "problems": [{"code": ::film::codes::READ_WRONG, "severity": "erreur", "line": 0, "column": 1, "message": why}], "remarks": [], "details": []})
            }
        };
        wrong += usize::from(value["ok"] != json!(true));
        checked_films.push(value);
    }
    json::print(&json::answer("check", json!({"films": checked_films})));
    match wrong {
        0 => Ok(()),
        wrong => Err(format!("{wrong} film(s) of {} with problems", films.len())),
    }
}

/// Films or scene files read only, not played: quick, no remarks on the picture.
pub fn read_only(files: &[&String], context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    let (mut wrong, mut remarks) = (0, 0);
    for file in files {
        let piece = Piece::find(context, file)?;
        for problem in read_through(&piece, data) {
            println!("{}{}", if problem.is_remark() { "remark: " } else { "" }, located(&piece, &problem));
            wrong += usize::from(!problem.is_remark());
            remarks += usize::from(problem.is_remark());
        }
    }
    println!("{}; {}; {}", count(files.len(), "file"), count(wrong, "problem"), count(remarks, "remark"));
    match wrong {
        0 => Ok(()),
        _ => Err(format!("{wrong} problem(s)")),
    }
}

/// `check-game <game>`: a whole game checked (`game::project_check`),
/// each problem as `file:line: message`, then how many; fails if there are.
pub fn check_game(game: &str, context: &Arc<Context>) -> Result<(), String> {
    // (A path, else a name: games/<name>/<name>.game, then library/<name>.game.)
    let path = match Path::new(game) {
        path if path.is_file() => path.to_path_buf(),
        _ => context.library.find_game(game).unwrap_or_else(|| context.library.shared().join(format!("{game}.game"))),
    };
    let report = game::project_check::check(context, &path)?;
    // (Said from the root of the project, as the films' checks say them.)
    let root = format!("{}/", context.root().display().to_string().replace('\\', "/"));
    let shown = |said: String| said.replace('\\', "/").replace(&root, "");
    for note in &report.notes {
        println!("note: {note}");
    }
    for problem in &report.problems {
        println!("{}", shown(problem.to_string()));
    }
    let game = shown(path.display().to_string());
    match report.problems.len() {
        0 => {
            println!("{game}: 0 problems");
            Ok(())
        }
        1 => Err(format!("{game}: 1 problem")),
        many => Err(format!("{game}: {many} problems")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every film compiled and played through, as `check` does: over two minutes.
    /// (Skipped where the game's data is not there.)
    #[test]
    #[ignore]
    fn all_films_compile_and_rehearse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let data = root.join("data").to_string_lossy().into_owned();
        if !Path::new(&data).exists() {
            return;
        }
        let context = Context::new(game::Project::library(&root), data.clone());
        let mut films: Vec<_> = std::fs::read_dir(context.library.films())
            .expect("the films")
            .flatten()
            .map(|file| file.path())
            .filter(|path| path.extension().is_some_and(|kind| kind == "film"))
            .collect();
        films.sort();
        let mut wrong = Vec::new();
        for path in &films {
            let piece = Piece::find(&context, &path.to_string_lossy()).expect("a film");
            let problems: Vec<String> = read_through(&piece, &data).iter().filter(|problem| !problem.is_remark()).map(|problem| piece.told(problem)).collect();
            let told = if problems.is_empty() { rehearsed(&piece).expect("played").wrong } else { Vec::new() };
            wrong.extend(problems.into_iter().chain(told.into_iter().map(|seen| seen.text)).map(|problem| format!("{} : {problem}", piece.name)));
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        assert!(films.len() >= 40, "{}", films.len());
    }
}
