//! The system's own dialogs (`rfd`): a file to open, several, a folder, a
//! file to write. Each waits for its answer (the window waits with it);
//! none chosen: `None`. Dropping a file on the window still works beside.

use std::path::{Path, PathBuf};

use rfd::FileDialog;

/// A dialog titled so, starting in that folder (or the one of that file).
fn dialog(title: &str, start: Option<&Path>) -> FileDialog {
    let mut dialog = FileDialog::new().set_title(title);
    if let Some(start) = start {
        let folder = if start.is_dir() { Some(start) } else { start.parent().filter(|parent| parent.is_dir()) };
        if let Some(folder) = folder {
            dialog = dialog.set_directory(folder);
        }
    }
    dialog
}

/// One file to open, of those kinds (`("PNG Images", &["png"])`).
pub fn file(title: &str, kinds: &[(&str, &[&str])], start: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = dialog(title, start);
    for (name, ends) in kinds {
        dialog = dialog.add_filter(*name, ends);
    }
    dialog.pick_file()
}

/// Several files to open, of those kinds.
pub fn files(title: &str, kinds: &[(&str, &[&str])], start: Option<&Path>) -> Vec<PathBuf> {
    let mut dialog = dialog(title, start);
    for (name, ends) in kinds {
        dialog = dialog.add_filter(*name, ends);
    }
    dialog.pick_files().unwrap_or_default()
}

/// A PNG to open.
pub fn png(title: &str, start: Option<&Path>) -> Option<PathBuf> {
    file(title, &[("PNG Images", &["png"])], start)
}

/// A folder.
pub fn folder(title: &str, start: Option<&Path>) -> Option<PathBuf> {
    dialog(title, start).pick_folder()
}

/// A file to write, named so at first.
pub fn save(title: &str, name: &str, kinds: &[(&str, &[&str])], start: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = dialog(title, start).set_file_name(name);
    for (kind, ends) in kinds {
        dialog = dialog.add_filter(*kind, ends);
    }
    dialog.save_file()
}
