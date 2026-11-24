//! What to do with the one chosen: a favourite, edited, painted, duplicated,
//! shown in its folder; a thing drawn renamed (in the films too, if asked)
//! or put in the Workshop's bin.

use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};

use super::super::grid::opened;
use super::super::{Asked, Item, Resources, Shelf};
use super::{plural, word_replaced};
use workshop_base::theme;
use workshop_decor::Decors;

/// The row of what to do with it.
pub(super) fn buttons(ui: &mut egui::Ui, resources: &mut Resources, item: &Item) {
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        let key = item.key();
        let favourite = resources.favourites.contains(&key);
        if ui.selectable_label(favourite, if favourite { "★ Favourite" } else { "☆ Favourite" }).clicked() {
            resources.favourite(&key);
        }
        dungeon_buttons(ui, resources, item);
        if let Some(file) = &item.file {
            file_buttons(ui, resources, item, file);
        }
        if ui.button("Copy the Word").clicked() {
            ui.ctx().copy_text(item.word.clone());
            resources.said = Some(format!("copied: {}", item.word));
        }
    });
}

/// A dungeon or a fight: opened in its editor, first; nothing to paint.
fn dungeon_buttons(ui: &mut egui::Ui, resources: &mut Resources, item: &Item) {
    if item.shelf == Shelf::Dungeons
        && ui
            .add(egui::Button::new(RichText::new("Edit").color(theme::GROUND)).fill(theme::ASSETS))
            .on_hover_text("open it in the Dungeon space (and try it)")
            .clicked()
    {
        resources.asked = Some(Asked::Dungeon(workshop_dungeon::dungeons::Open::Dungeon(item.word.clone())));
    }
    if item.shelf == Shelf::Bosses
        && ui
            .add(egui::Button::new(RichText::new("Edit the Fight").color(theme::GROUND)).fill(theme::ASSETS))
            .on_hover_text("open it in the Dungeon space")
            .clicked()
    {
        resources.asked = Some(Asked::Dungeon(workshop_dungeon::dungeons::Open::Fight(item.word.clone())));
    }
}

/// What is done with its file: painted, duplicated, shown, made again, renamed, put away.
fn file_buttons(ui: &mut egui::Ui, resources: &mut Resources, item: &Item, file: &Path) {
    let dungeon = matches!(item.shelf, Shelf::Dungeons | Shelf::Bosses);
    if !dungeon
        && ui
            .add(egui::Button::new(RichText::new("Paint").color(theme::GROUND)).fill(theme::ASSETS))
            .on_hover_text("the image opened in your drawing program")
            .clicked()
    {
        resources.said = Some(opened(file));
    }
    if !dungeon && ui.button("Duplicate").on_hover_text("a copy, under another word").clicked() {
        resources.said = Some(duplicated(resources, item, file));
    }
    if ui.button("Show in Folder").clicked() {
        resources.said = Some(file.parent().map_or_else(String::new, opened));
    }
    if item.shelf == Shelf::Drawn && ui.button("Paint Here").on_hover_text("in the Workshop, pixel by pixel, its images one by one").clicked() {
        match workshop_picture::Painting::open(&resources.session, &item.word, file, item.number.max(1) as usize) {
            Some(painting) => resources.open_making(crate::resources::Maker::Picture(Box::new(painting))),
            None => resources.said = Some("the image could not be read".to_string()),
        }
    }
    if item.shelf == Shelf::Sounds && ui.button("Edit").on_hover_text("make it again in the Workshop").clicked() {
        resources.open_making(crate::resources::Maker::Sound(Box::new(workshop_sound::SoundMaking::open(&resources.session, &item.word))));
    }
    if item.shelf == Shelf::Drawn && ui.button("Rename…").clicked() {
        resources.renaming = Some((item.word.clone(), true));
    }
    if item.shelf == Shelf::Drawn && ui.button("Delete…").clicked() {
        resources.trashing = true;
    }
}

/// A file's name, and what follows its first dot (or `none`).
fn name_and_rest(file: &Path, none: &str) -> (String, String) {
    let name = file.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let rest = name.split_once('.').map_or(none.to_string(), |(_, rest)| rest.to_string());
    (name, rest)
}

/// A copy of its file under a word not yet taken (`_copy`, then `2`s); said.
fn duplicated(resources: &mut Resources, item: &Item, file: &Path) -> String {
    let (_, rest) = name_and_rest(file, "");
    let mut word = format!("{}_copy", item.word);
    while file.with_file_name(format!("{word}.{rest}")).exists() {
        word.push('2');
    }
    match std::fs::copy(file, file.with_file_name(format!("{word}.{rest}"))) {
        Ok(_) => {
            resources.lists.remove(&Shelf::Drawn);
            resources.chosen = Some(format!("{}:{word}", Shelf::Drawn.key()));
            format!("copied as “{word}”")
        }
        Err(why) => why.to_string(),
    }
}

/// A thing drawn named anew (its file, and the films that write it, if
/// asked), or put in the bin.
pub(super) fn renamed_or_put_away(ui: &mut egui::Ui, resources: &mut Resources, decors: &mut Decors, item: &Item, file: PathBuf) {
    let films: Vec<PathBuf> = resources.used_in(&item.word).into_iter().filter(|path| path.extension().is_some_and(|own| own == "film")).collect();
    match renaming_rows(ui, resources, films.len()) {
        Some(true) => {
            let (name, in_films) = resources.renaming.take().unwrap_or_default();
            let said = renamed(resources, decors, item, &file, &name, if in_films { &films } else { &[] });
            resources.said = Some(said);
        }
        Some(false) => resources.renaming = None,
        None => {}
    }
    // (Put away: in the Workshop's bin, to be taken back by hand.)
    if resources.trashing {
        bin_row(ui, resources, &file, films.len());
    }
}

/// The new name asked, and whether the films are changed too; whether it is
/// done (renamed) or given up.
fn renaming_rows(ui: &mut egui::Ui, resources: &mut Resources, films: usize) -> Option<bool> {
    let mut done = None;
    let Some((name, in_films)) = &mut resources.renaming else {
        return None;
    };
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let field = ui.add(egui::TextEdit::singleline(name).desired_width(140.0));
        if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            done = Some(true);
        }
        if ui.button("Rename").clicked() {
            done = Some(true);
        }
        if ui.button("Cancel").clicked() {
            done = Some(false);
        }
    });
    if films > 0 {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.checkbox(in_films, theme::text(format!("also in the {films} film{} that write it", plural(films)), theme::SMALL, theme::TEXT));
        });
    }
    done
}

/// Its file renamed for the word asked, and the word in these films; said.
fn renamed(resources: &mut Resources, decors: &mut Decors, item: &Item, file: &Path, name: &str, films: &[PathBuf]) -> String {
    let word = film::words::plain(&name.trim().replace([' ', '-'], "_"));
    let (_, rest) = name_and_rest(file, "png");
    let target = file.with_file_name(format!("{word}.{rest}"));
    if word.is_empty() || word == item.word {
        return "the same word".to_string();
    }
    if target.exists() || resources.list(Shelf::Drawn, decors).iter().any(|known| known.word == word) {
        return format!("“{word}” is already taken");
    }
    if let Err(why) = std::fs::rename(file, &target) {
        return why.to_string();
    }
    let mut changed = 0;
    for path in films {
        if let Ok(text) = std::fs::read_to_string(path) {
            let made = word_replaced(&text, &item.word, &word);
            if made != text && std::fs::write(path, made).is_ok() {
                changed += 1;
            }
        }
    }
    resources.lists.remove(&Shelf::Drawn);
    resources.pictures.remove(&item.key());
    resources.corpus = None;
    resources.chosen = Some(format!("{}:{word}", Shelf::Drawn.key()));
    renamed_said(&word, changed)
}

/// What is said of a thing renamed, and in how many films.
pub(super) fn renamed_said(word: &str, changed: usize) -> String {
    format!("renamed “{word}”{}", if changed > 0 { format!(", and in {changed} film{}", plural(changed)) } else { String::new() })
}

/// What is said of the films that still write a thing put away.
pub(super) fn still_written(films: usize) -> String {
    if films == 0 { String::new() } else { format!(" ({films} film{} still write{} it)", plural(films), if films > 1 { "" } else { "s" }) }
}

/// Put in the Workshop's bin? Yes or no.
fn bin_row(ui: &mut egui::Ui, resources: &mut Resources, file: &Path, films: usize) {
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text(format!("Put it in the Workshop's bin?{}", still_written(films)), theme::SMALL, theme::WRONG));
        if ui.button("Yes").clicked() {
            resources.said = Some(put_in_bin(resources, file));
            resources.trashing = false;
        }
        if ui.button("No").clicked() {
            resources.trashing = false;
        }
    });
}

/// Its file moved into the bin (with the time before its name if one is
/// already there); said.
fn put_in_bin(resources: &mut Resources, file: &Path) -> String {
    let bin = workshop_base::project::bin(resources.session.root());
    _ = std::fs::create_dir_all(&bin);
    let mut to = bin.join(file.file_name().unwrap_or_default());
    if to.exists() {
        let (name, _) = name_and_rest(file, "");
        to = bin.join(format!("{}_{name}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs())));
    }
    match std::fs::rename(file, &to) {
        Ok(()) => {
            resources.lists.remove(&Shelf::Drawn);
            resources.chosen = None;
            format!("put in the bin: {}", to.display())
        }
        Err(why) => why.to_string(),
    }
}
