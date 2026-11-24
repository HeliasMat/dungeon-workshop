//! Safe saves: a file written beside first (`<file>.tmp`), the one it
//! replaces kept (`<file>.bak`, one), then put in its place. A file opened
//! whose save was cut short (its `.tmp` left) or whose `.bak` is newer than
//! itself is told, and the window offers to take it back; what it was
//! before being taken back stays as `<file>.before`.

use std::path::{Path, PathBuf};

use crate::session::Session;

/// The file kept beside one: `<file>.<end>`.
pub fn beside(path: &Path, end: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{end}"));
    path.with_file_name(name)
}

/// Written safely: beside first, the last version kept as `.bak`, then in its place.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::io::Write;
    let temporary = beside(path, "tmp");
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(contents.as_ref())?;
        file.sync_all()?;
    }
    // (The `.bak` given the time the last version was written, so it stays
    // older than the file: a copy keeps it on Windows, not on Linux or macOS.)
    if path.exists() {
        let backup = beside(path, "bak");
        std::fs::copy(path, &backup)?;
        if let Ok(written) = std::fs::metadata(path).and_then(|meta| meta.modified()) {
            std::fs::File::options().write(true).open(&backup)?.set_modified(written)?;
        }
    }
    std::fs::rename(&temporary, path)
}

/// What is wrong with a file about to be opened: its save cut short, its `.bak` newer.
pub fn trouble(path: &Path) -> Option<Trouble> {
    let temporary = beside(path, "tmp");
    if temporary.is_file() {
        return Some(Trouble::Interrupted);
    }
    let changed = |path: &Path| std::fs::metadata(path).and_then(|meta| meta.modified()).ok();
    let backup = beside(path, "bak");
    match (changed(&backup), changed(path)) {
        (Some(kept), Some(own)) if kept > own => Some(Trouble::NewerBackup),
        (Some(_), None) => Some(Trouble::NewerBackup),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trouble {
    /// Its `.tmp` is left: a save did not end.
    Interrupted,
    /// Its `.bak` is newer than it (or it is gone).
    NewerBackup,
}

/// What the window is asked: the files to offer to restore; those restored,
/// to be read again (kept by the session, `Session::safe`).
#[derive(Default)]
pub struct Asked {
    offered: Vec<(PathBuf, Trouble)>,
    seen: Vec<PathBuf>,
    restored: Vec<PathBuf>,
}

/// A file being opened: told to the window if something is wrong with it (once a session).
pub fn check(session: &Session, path: &Path) {
    let Some(trouble) = trouble(path) else {
        return;
    };
    if let Ok(mut asked) = session.safe.lock()
        && !asked.seen.iter().any(|seen| seen == path)
    {
        asked.seen.push(path.to_path_buf());
        asked.offered.push((path.to_path_buf(), trouble));
    }
}

/// The first file to offer to restore.
pub fn offered(session: &Session) -> Option<(PathBuf, Trouble)> {
    session.safe.lock().ok()?.offered.first().cloned()
}

/// What is chosen for it: the save cut short taken (`.tmp`), the last version (`.bak`), or the file kept as it is.
pub fn answer(session: &Session, path: &Path, from: Option<&str>) -> Result<String, String> {
    if let Ok(mut asked) = session.safe.lock() {
        asked.offered.retain(|(offered, _)| offered != path);
    }
    let temporary = beside(path, "tmp");
    let Some(end) = from else {
        // (Kept as it is: the `.tmp` left put away, so as not to be asked again.)
        if temporary.is_file() {
            std::fs::rename(&temporary, beside(path, "tmp.abandoned")).map_err(|why| why.to_string())?;
        }
        return Ok(format!("kept as it is: {}", path.display()));
    };
    let source = beside(path, end);
    let read = std::fs::read(&source).map_err(|why| format!("{}: {why}", source.display()))?;
    if read.is_empty() {
        return Err(format!("{} is empty: nothing is taken back", source.display()));
    }
    // (What it was before kept too, as `.before`.)
    if path.exists() {
        _ = std::fs::copy(path, beside(path, "before"));
    }
    std::fs::write(path, read).map_err(|why| format!("{}: {why}", path.display()))?;
    if end == "tmp" {
        _ = std::fs::remove_file(&temporary);
    } else if temporary.is_file() {
        _ = std::fs::rename(&temporary, beside(path, "tmp.abandoned"));
    }
    if let Ok(mut asked) = session.safe.lock() {
        asked.restored.push(path.to_path_buf());
    }
    Ok(format!("taken back from {}: {}", source.file_name().unwrap_or_default().to_string_lossy(), path.display()))
}

/// The files restored since asked last: to be read again where they are open.
pub fn restored(session: &Session) -> Vec<PathBuf> {
    session.safe.lock().map(|mut asked| std::mem::take(&mut asked.restored)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save keeps the last version as `.bak` and leaves no `.tmp`; a save cut short is told.
    #[test]
    fn safe_save_keeps_the_last_version() {
        let folder = std::env::temp_dir().join(format!("safe_save_{}", std::process::id()));
        _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("a.film");
        write(&path, "one").unwrap();
        write(&path, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
        assert_eq!(std::fs::read_to_string(beside(&path, "bak")).unwrap(), "one");
        assert!(!beside(&path, "tmp").exists());
        assert_eq!(trouble(&path), None);
        std::fs::write(beside(&path, "tmp"), "three").unwrap();
        assert_eq!(trouble(&path), Some(Trouble::Interrupted));
        answer(&crate::session::Session::for_tests(), &path, Some("tmp")).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "three");
        assert_eq!(std::fs::read_to_string(beside(&path, "before")).unwrap(), "two");
        _ = std::fs::remove_dir_all(&folder);
    }
}
