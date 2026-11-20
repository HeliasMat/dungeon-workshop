//! Where the game's files are, and their lines as a film reads them: the
//! lists merged by `assets` (see `data/DATA.md`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::Named;
use crate::words::plain;

/// A file's lines; none if it is not there.
pub(super) fn lines_of(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path).map_or(Vec::new(), |text| text.lines().map(str::to_string).collect())
}

/// Where a file of the game is in the data folder, of `hand`, `extra`,
/// `rom`, else in the folder itself (one read out of the game with no
/// layers): `assets::data::path`.
pub fn game_file(data: &str, file: &str) -> PathBuf {
    PathBuf::from(assets::data::path(data, file))
}

/// Lines cut in fields, but the empty and the comments.
fn rows(lines: Vec<String>) -> Vec<Vec<String>> {
    lines.iter().filter(|line| !line.trim().is_empty() && !line.starts_with('#')).map(|line| line.split('\t').map(str::to_string).collect()).collect()
}

/// A list of the game, merged (`rom`'s, `extra`'s, `decor`'s: `assets::data::text`), in fields.
pub(super) fn game_rows(data: &str, file: &str) -> Vec<Vec<String>> {
    assets::tables::rows_of(data, file)
}

/// A table the extraction wrote (`data/rom`: `levels.txt`, `objects.txt`, `place_names.txt`), in fields.
pub(super) fn table(data: &str, file: &str) -> Vec<Vec<String>> {
    rows(lines_of(Path::new(&game_file(data, file))))
}

/// A list of the game's names (moves, items), by their words.
pub(super) fn named(data: &str, file: &str) -> BTreeMap<String, Named> {
    let mut found = BTreeMap::new();
    for row in game_rows(data, file).into_iter().filter(|row| row.len() > 1) {
        found.entry(plain(&row[1].replace(' ', ""))).or_insert((row[0].clone(), row[1].clone()));
    }
    found
}

/// A field as a number; 0 if it is none.
pub(super) fn number(field: &str) -> i32 {
    field.parse().unwrap_or(0)
}

/// The folder of our own sounds, beside the films (`sound my_noise`).
pub const OWN_SOUNDS: &str = "library/sounds";

/// The names of a project's own sounds (`<project>/sounds/<name>.wav`, then
/// the repository's), read with its tables (`Tables::own_sounds`).
pub(super) fn own_sounds_of(project: &assets::project::Project) -> Vec<String> {
    let mut names: Vec<String> = project
        .folders("sounds")
        .into_iter()
        .flat_map(|folder| std::fs::read_dir(folder).into_iter().flatten().flatten())
        .filter_map(|file| file.file_name().to_string_lossy().strip_suffix(".wav").map(str::to_string))
        .collect();
    names.sort();
    names.dedup();
    names
}
