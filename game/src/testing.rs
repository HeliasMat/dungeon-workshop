//! What the tests play with: the repository's root, its shared library,
//! its data folder, a context made of them.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use assets::project::Project;

/// The repository's root (the tests run from `game/`).
pub fn root() -> PathBuf {
    assets::testing::root()
}

/// The repository's shared library.
pub fn library() -> Project {
    assets::testing::library()
}

/// The repository's data folder (or the one `DUNGEON_DATA` names).
pub fn data_folder() -> String {
    assets::testing::data()
}

/// The context of the repository's library and data, made once for the tests.
pub fn context() -> Arc<film::Context> {
    static MADE: OnceLock<Arc<film::Context>> = OnceLock::new();
    MADE.get_or_init(|| Arc::new(film::Context::new(library(), data_folder()))).clone()
}

/// A context of the repository's library on that data folder.
pub fn context_on(data: &str) -> Arc<film::Context> {
    Arc::new(film::Context::new(library(), data))
}
