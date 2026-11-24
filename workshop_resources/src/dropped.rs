//! Files dropped on the window: pictures joining the things drawn, or the
//! one being made.

use std::path::PathBuf;

use eframe::egui;

use super::shelves::drawn_folders;
use super::{Resources, Shelf};

/// A PNG dropped on the window: among the things drawn, by the word its name makes (never over another).
pub(super) fn take_dropped(resources: &mut Resources, context: &egui::Context) {
    let dropped: Vec<PathBuf> = context.input(|input| input.raw.dropped_files.iter().filter_map(|file| file.path.clone()).collect());
    take_files(resources, dropped);
}

/// PNGs dropped or chosen: among the things drawn (while painting: pasted in it).
pub(super) fn take_files(resources: &mut Resources, dropped: Vec<PathBuf>) {
    // (While painting: the picture pasted in it, to be moved.)
    if let Some(crate::Maker::Picture(painting)) = resources.makings.get_mut(resources.making_at).filter(|_| resources.making_full) {
        if let Some(picture) = dropped.iter().find_map(|path| image::open(path).ok()) {
            painting.paste(picture.to_rgba8());
        }
        return;
    }
    let mut taken = Vec::new();
    for path in dropped.into_iter().filter(|path| path.extension().is_some_and(|own| own.eq_ignore_ascii_case("png"))) {
        let stem = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
        let (name, cells) = stem
            .rsplit_once('.')
            .filter(|(_, cells)| cells.parse::<u32>().is_ok())
            .map_or((stem.clone(), String::new()), |(name, cells)| (name.to_string(), format!(".{cells}")));
        let mut word = film::words::plain(&name.replace([' ', '-'], "_"));
        let folder = drawn_folders(&resources.session)[0].clone();
        while std::fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().split('.').next() == Some(word.as_str()))
        {
            word.push('2');
        }
        if std::fs::copy(&path, folder.join(format!("{word}{cells}.png"))).is_ok() {
            taken.push(word);
        }
    }
    if let Some(last) = taken.last() {
        resources.lists.remove(&Shelf::Drawn);
        resources.shelf = Shelf::Drawn;
        resources.chosen = Some(format!("{}:{last}", Shelf::Drawn.key()));
        resources.said = Some(format!("imported: {}", taken.join(", ")));
    }
}

/// A PNG dropped while painting: pasted in it, to be moved.
pub(super) fn take_dropped_while_making(resources: &mut Resources, context: &egui::Context) {
    let dropped: Vec<PathBuf> = context.input(|input| input.raw.dropped_files.iter().filter_map(|file| file.path.clone()).collect());
    if let Some(crate::Maker::Picture(painting)) = resources.makings.get_mut(resources.making_at)
        && let Some(picture) = dropped.iter().find_map(|path| image::open(path).ok())
    {
        painting.paste(picture.to_rgba8());
    }
}
