//! For the tests of every block, and for nothing else: the repository's
//! root, its shared library and its data folder, found the one way (the
//! programs find theirs in their `main`: `project::find_root`,
//! `data::folder`).

use std::path::{Path, PathBuf};

use crate::project::Project;

/// The repository's root (every block's folder is one level below it).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The repository's shared library (`library/`).
pub fn library() -> Project {
    Project::library(root())
}

/// The repository's data folder, as a program's `main` works it out
/// (`DUNGEON_DATA`, else `data/`: `data::folder`); where `DUNGEON_DATA`
/// names none and `data/` holds no extraction (on CI), the made-up
/// [`fixture`]. (`DUNGEON_DATA=tests/fixture_data` runs the tests as CI
/// does, the extracted `data/` there or not.)
pub fn data() -> String {
    let folder = crate::data::folder(&root());
    if std::env::var_os("DUNGEON_DATA").is_some() || extracted(&folder) {
        return folder;
    }
    fixture().to_string_lossy().into_owned()
}

/// Whether a data folder holds an extraction of the game (its tables of
/// dungeons), not the fixture's few lists.
pub fn extracted(folder: &str) -> bool {
    crate::data::has(folder, "dungeons.bin")
}

/// The tests' own data (`tests/fixture_data/`): a few lists written by
/// hand in the formats of `data/DATA.md` (species, moves, items, places'
/// sizes and grounds), no picture, no sound, nothing else of the game's.
pub fn fixture() -> PathBuf {
    root().join("tests").join("fixture_data")
}
