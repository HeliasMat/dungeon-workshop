//! The Script over the whole project: words sought and replaced in every film (a name renamed everywhere), a film
//! or a recipe opened by its name, the films opened kept as tabs, another film beside, the versions of a film (kept
//! as it is written, the last commit) compared line by line and taken back, notes in the margin (what is still to
//! do) that are no part of the film.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The films and recipes of the project, by their paths.
pub fn project_files(library: &assets::project::Project) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for folder in [library.films(), library.recipes(), library.root().join("film/recipes")] {
        walk(&folder, &mut found);
    }
    found.sort();
    found.dedup();
    found
}

fn walk(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            if folder.ends_with("recipes") || path.file_name().is_some_and(|name| name != "recipes") {
                walk(&path, found);
            }
        } else if path.extension().is_some_and(|end| end == "film") {
            found.push(path);
        }
    }
}

/// A word found: in that file, at that line (from 1) and column (from 1, in letters), the line as it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub text: String,
}

/// Where a word is in a line (letters' indices): as a whole word only, its case counting or not.
pub fn found_in_line(line: &str, sought: &str, whole: bool, case: bool) -> Vec<usize> {
    if sought.is_empty() {
        return Vec::new();
    }
    let (haystack, needle): (Vec<char>, Vec<char>) = if case {
        (line.chars().collect(), sought.chars().collect())
    } else {
        (line.to_lowercase().chars().collect(), sought.to_lowercase().chars().collect())
    };
    let letter = |at: usize| haystack.get(at).is_some_and(|letter| letter.is_alphanumeric() || *letter == '_');
    let mut found = Vec::new();
    let mut at = 0;
    while at + needle.len() <= haystack.len() {
        if haystack[at..at + needle.len()] == needle[..] && (!whole || ((at == 0 || !letter(at - 1)) && !letter(at + needle.len()))) {
            found.push(at);
            at += needle.len().max(1);
        } else {
            at += 1;
        }
    }
    found
}

/// The text with each found replaced (its letters counted as `found_in_line` counts them).
pub fn replaced_in(text: &str, sought: &str, with: &str, whole: bool, case: bool) -> (String, usize) {
    let mut count = 0;
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            let found = found_in_line(line, sought, whole, case);
            count += found.len();
            let letters: Vec<char> = line.chars().collect();
            let mut made = String::new();
            let mut at = 0;
            for start in found {
                made.extend(&letters[at..start]);
                made.push_str(with);
                at = start + sought.chars().count();
            }
            made.extend(&letters[at.min(letters.len())..]);
            made
        })
        .collect();
    (lines.join("\n"), count)
}

/// Every place a word is in the project's films (the open one as it is now, not as written).
pub fn search(library: &assets::project::Project, sought: &str, whole: bool, case: bool, open: Option<(&Path, &str)>) -> Vec<Hit> {
    let mut hits = Vec::new();
    for path in project_files(library) {
        let text = match open {
            Some((own, text)) if own == path => text.to_string(),
            _ => std::fs::read_to_string(&path).unwrap_or_default(),
        };
        for (at, line) in text.split('\n').enumerate() {
            for column in found_in_line(line, sought, whole, case) {
                hits.push(Hit { path: path.clone(), line: at + 1, column: column + 1, text: line.trim().to_string() });
            }
        }
    }
    hits
}

/// A film or a recipe sought by a few letters: those whose name has them in order, the closest first.
pub fn matching(files: &[PathBuf], typed: &str) -> Vec<PathBuf> {
    let typed = film::words::plain(typed);
    let mut scored: Vec<(usize, PathBuf)> = files
        .iter()
        .filter_map(|path| {
            let name = film::words::plain(&path.file_stem()?.to_string_lossy());
            let mut letters = name.chars();
            let fits = typed.chars().all(|wanted| letters.any(|letter| letter == wanted));
            fits.then(|| {
                (
                    if name.starts_with(&typed) {
                        0
                    } else if name.contains(&typed) {
                        1
                    } else {
                        2
                    } * 1000
                        + name.len(),
                    path.clone(),
                )
            })
        })
        .collect();
    scored.sort();
    scored.into_iter().map(|(_, path)| path).collect()
}

/// A line of a comparison: the same in both, only in the old, only in the new.
#[derive(Clone, Debug, PartialEq)]
pub enum Diff {
    Same(String),
    Gone(String),
    Came(String),
}

/// Two texts compared line by line (the longest run they share kept, the rest gone or come).
pub fn diff(old: &str, new: &str) -> Vec<Diff> {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.split('\n').collect(), new.split('\n').collect());
    if a.len() * b.len() > 9_000_000 {
        return a.iter().map(|line| Diff::Gone(line.to_string())).chain(b.iter().map(|line| Diff::Came(line.to_string()))).collect();
    }
    let mut lcs = vec![vec![0_u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            out.push(Diff::Same(a[i].to_string()));
            (i, j) = (i + 1, j + 1);
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] >= lcs[i + 1][j]) {
            out.push(Diff::Came(b[j].to_string()));
            j += 1;
        } else {
            out.push(Diff::Gone(a[i].to_string()));
            i += 1;
        }
    }
    out
}

/// Where a film's versions are kept.
fn versions_folder(root: &Path, film: &Path) -> PathBuf {
    let stem = film.file_stem().map_or_else(|| "film".to_string(), |stem| stem.to_string_lossy().into_owned());
    root.join("target").join("workshop").join("versions").join(stem)
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// A film's text kept as a version, if the last was kept more than two minutes ago or said otherwise (sixty kept).
pub fn keep_version(root: &Path, film: &Path, text: &str) {
    let folder = versions_folder(root, film);
    let kept = versions(root, film);
    if let Some((when, path)) = kept.first() {
        let same = std::fs::read_to_string(path).is_ok_and(|was| was == text);
        if same || now().saturating_sub(*when) < 120 {
            return;
        }
    }
    _ = std::fs::create_dir_all(&folder);
    _ = std::fs::write(folder.join(format!("{}.film", now())), text);
    for (_, old) in versions(root, film).into_iter().skip(60) {
        _ = std::fs::remove_file(old);
    }
}

/// A film's versions kept, the newest first: when (seconds since 1970), where.
pub fn versions(root: &Path, film: &Path) -> Vec<(u64, PathBuf)> {
    let mut found: Vec<(u64, PathBuf)> = std::fs::read_dir(versions_folder(root, film))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| Some((entry.path().file_stem()?.to_string_lossy().parse().ok()?, entry.path())))
        .collect();
    found.sort_by_key(|a| std::cmp::Reverse(a.0));
    found
}

/// How long ago, said.
pub fn ago(when: u64) -> String {
    let gone = now().saturating_sub(when);
    match gone {
        0..60 => "just now".to_string(),
        60..3600 => format!("{} min ago", gone / 60),
        3600..86_400 => format!("{} h ago", gone / 3600),
        _ => format!("{} days ago", gone / 86_400),
    }
}

/// The film as the last commit has it (none: not in git, or git not there).
pub fn committed(session: &workshop_base::session::Session, film: &Path) -> Option<String> {
    // (Asked at each frame the window is open: git asked again only every few seconds.)
    let mut kept = session.committed.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((when, text)) = kept.get(film)
        && when.elapsed().as_secs() < 5
    {
        return text.clone();
    }
    let text = asked_of_git(session.root(), film);
    kept.insert(film.to_path_buf(), (std::time::Instant::now(), text.clone()));
    text
}

fn asked_of_git(root: &Path, film: &Path) -> Option<String> {
    let relative = if film.is_absolute() { film.strip_prefix(root).ok()? } else { film }.to_string_lossy().replace('\\', "/");
    let out = std::process::Command::new("git").arg("-C").arg(root).arg("show").arg(format!("HEAD:{relative}")).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
}

/// A note of the margin: done or not (a thing to do), what it says.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub done: bool,
    pub said: String,
}

fn notes_file(library: &assets::project::Project, film: &Path) -> PathBuf {
    let stem = film.file_stem().map_or_else(|| "film".to_string(), |stem| stem.to_string_lossy().into_owned());
    library.kind("notes").join(format!("{stem}.notes"))
}

/// A film's notes, by their lines (from 1).
pub fn read_notes(library: &assets::project::Project, film: &Path) -> BTreeMap<usize, Note> {
    std::fs::read_to_string(notes_file(library, film))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let at = parts.next()?.parse().ok()?;
            let done = parts.next()? == "1";
            Some((at, Note { done, said: parts.next().unwrap_or_default().replace("\\n", "\n") }))
        })
        .collect()
}

/// A film's notes written (none: the file taken away).
pub fn write_notes(library: &assets::project::Project, film: &Path, notes: &BTreeMap<usize, Note>) {
    let file = notes_file(library, film);
    if notes.is_empty() {
        _ = std::fs::remove_file(file);
        return;
    }
    let said: String = notes.iter().map(|(line, note)| format!("{line}\t{}\t{}\n", u8::from(note.done), note.said.replace('\n', "\\n"))).collect();
    if let Some(folder) = file.parent() {
        _ = std::fs::create_dir_all(folder);
    }
    _ = std::fs::write(file, said);
}

/// The lines of comments that say something is still to do (`# TODO`, `# TO DO`).
pub fn todos(text: &str) -> Vec<(usize, String)> {
    text.split('\n')
        .enumerate()
        .filter_map(|(at, line)| {
            let comment = line.split_once('#')?.1.trim();
            let upper = comment.to_uppercase();
            (upper.starts_with("TODO") || upper.starts_with("TO DO")).then(|| (at + 1, comment.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_found_replaced_and_texts_compared() {
        assert_eq!(found_in_line("pikachu and pikachu2 Pikachu", "pikachu", true, false), vec![0, 21]);
        assert_eq!(found_in_line("pikachu and pikachu2", "pikachu", false, true), vec![0, 12]);
        let (made, count) = replaced_in("scene: beach pikachu piplup\npikachu: Hi pikachu2!", "pikachu", "eevee", true, true);
        assert_eq!((made.as_str(), count), ("scene: beach eevee piplup\neevee: Hi pikachu2!", 2));
        let compared = diff("a\nb\nc\nd", "a\nc\nx\nd");
        assert_eq!(compared, vec![Diff::Same("a".into()), Diff::Gone("b".into()), Diff::Same("c".into()), Diff::Came("x".into()), Diff::Same("d".into())]);
        let files = vec![PathBuf::from("the_duel.film"), PathBuf::from("the_fallen_star.film"), PathBuf::from("little_game.film")];
        assert_eq!(matching(&files, "star").first(), Some(&PathBuf::from("the_fallen_star.film")));
        assert_eq!(matching(&files, "lg"), vec![PathBuf::from("little_game.film")]);
        assert_eq!(
            todos("walk a\n# TODO: the camera\n# nothing\njump a # to do: higher"),
            vec![(2, "TODO: the camera".to_string()), (4, "to do: higher".to_string())]
        );
    }
}
