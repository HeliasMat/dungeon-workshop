//! The project's files: its films and its decors, read from their folders,
//! each with what is shown of it (its title, when it changed, what it is
//! about), the last changed first.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What a file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Film,
    Decor,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Film => "Film",
            Kind::Decor => "Decor",
        }
    }
}

/// A file of the project.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
    /// Its title (`title:`, `name`), else its file's name.
    pub title: String,
    pub changed: Option<SystemTime>,
    /// The places a film's scenes are in; a decor's size.
    pub about: String,
    pub lines: usize,
    /// Its text; how many scenes, those in them, the places they are in (their words).
    pub text: String,
    pub scenes: usize,
    pub cast: Vec<String>,
    pub places: Vec<String>,
}

impl Entry {
    /// A file read: its first words, for the list.
    fn read(path: &Path, kind: Kind) -> Entry {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let said = |key: &str| text.lines().find_map(|line| line.trim().strip_prefix(key).map(|rest| rest.trim().to_string()));
        let title = match kind {
            Kind::Film => said("title:"),
            Kind::Decor => said("name "),
        };
        let about = match kind {
            Kind::Film => {
                let mut places: Vec<String> = text
                    .lines()
                    .filter_map(|line| line.trim().strip_prefix("scene:"))
                    .filter_map(|rest| rest.split_whitespace().next())
                    .map(str::to_string)
                    .collect();
                places.dedup();
                places.join(", ")
            }
            Kind::Decor => text
                .lines()
                .find_map(|line| line.trim().strip_prefix("decor:"))
                .map(|rest| rest.split_whitespace().skip(1).take(2).collect::<Vec<_>>().join(" × "))
                .unwrap_or_default(),
        };
        let changed = std::fs::metadata(path).and_then(|data| data.modified()).ok();
        let scene_lines: Vec<Vec<&str>> =
            text.lines().filter_map(|line| line.trim().strip_prefix("scene:")).map(|rest| rest.split_whitespace().collect()).collect();
        let mut places: Vec<String> = scene_lines.iter().filter_map(|words| words.first()).map(|word| word.to_string()).collect();
        places.dedup();
        let mut cast: Vec<String> = Vec::new();
        for name in scene_lines.iter().flat_map(|words| words.iter().skip(1)) {
            if !cast.iter().any(|known| known == name) {
                cast.push(name.to_string());
            }
        }
        let scenes = scene_lines.len();
        let lines = text.lines().count();
        Entry { path: path.to_path_buf(), kind, title: title.unwrap_or(name), changed, about, lines, text, scenes, cast, places }
    }

    /// A file of the Workshop's bin: what it was, by its extension.
    pub fn binned(path: &Path) -> Entry {
        let kind = if path.extension().is_some_and(|own| own == "decor") { Kind::Decor } else { Kind::Film };
        let mut entry = Entry::read(path, kind);
        entry.about = path.extension().map(|own| own.to_string_lossy().into_owned()).unwrap_or_default();
        entry
    }

    /// How long ago it changed, in a few words.
    pub fn age(&self) -> String {
        let Some(gone) = self.changed.and_then(|changed| SystemTime::now().duration_since(changed).ok()) else {
            return String::new();
        };
        match gone.as_secs() {
            seconds if seconds < 60 => "just now".to_string(),
            seconds if seconds < 3600 => format!("{} min ago", seconds / 60),
            seconds if seconds < 86_400 => format!("{} h ago", seconds / 3600),
            seconds if seconds < 2 * 86_400 => "yesterday".to_string(),
            seconds => format!("{} days ago", seconds / 86_400),
        }
    }
}

/// The project: its films and its decors; the recipes its films share; the Workshop's bin.
#[derive(Default)]
pub struct Project {
    pub films: Vec<Entry>,
    pub decors: Vec<Entry>,
    pub shared: Vec<Entry>,
    pub binned: Vec<Entry>,
}

/// The Workshop's bin: what is put away there, to be taken back.
pub fn bin(root: &Path) -> PathBuf {
    root.join("target").join("workshop").join("bin")
}

impl Project {
    /// Read from their folders (the session's library), what the session
    /// read of the files forgotten first: they may have changed.
    pub fn read(session: &crate::session::Session) -> Project {
        session.reload();
        let library = session.library();
        let mut shared: Vec<Entry> =
            film::shared_files(library).iter().filter_map(|name| film::shared_path(library, name)).map(|path| Entry::read(&path, Kind::Film)).collect();
        shared.sort_by(|a, b| a.title.cmp(&b.title));
        let mut binned: Vec<Entry> = std::fs::read_dir(bin(session.root()))
            .into_iter()
            .flatten()
            .flatten()
            .map(|file| file.path())
            .filter(|path| path.is_file())
            .map(|path| Entry::binned(&path))
            .collect();
        binned.sort_by_key(|entry| std::cmp::Reverse(entry.changed));
        Project { films: entries(&library.films(), "film", Kind::Film), decors: entries(&library.decors(), "decor", Kind::Decor), shared, binned }
    }

    /// Any of its files, by its path.
    pub fn entry(&self, path: &Path) -> Option<&Entry> {
        self.films.iter().chain(&self.decors).chain(&self.shared).chain(&self.binned).find(|entry| entry.path == path)
    }

    /// All of them, the last changed first.
    pub fn recent(&self) -> Vec<&Entry> {
        let mut all: Vec<&Entry> = self.films.iter().chain(&self.decors).collect();
        all.sort_by_key(|entry| std::cmp::Reverse(entry.changed));
        all
    }
}

/// The files of a folder of that kind, the last changed first.
fn entries(folder: &Path, extension: &str, kind: Kind) -> Vec<Entry> {
    let mut found: Vec<Entry> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|file| file.path())
        .filter(|path| path.extension().is_some_and(|own| own == extension))
        .map(|path| Entry::read(&path, kind))
        .collect();
    found.sort_by_key(|entry| std::cmp::Reverse(entry.changed));
    found
}

/// The kinds of places (`library/tables/place_kinds.txt`), read again only when the file changes.
pub fn place_kinds(library: &assets::project::Project) -> std::sync::Arc<decor::tags::PlaceKinds> {
    type Kept = std::sync::Mutex<Option<(std::path::PathBuf, Option<std::time::SystemTime>, std::sync::Arc<decor::tags::PlaceKinds>)>>;
    static KEPT: std::sync::OnceLock<Kept> = std::sync::OnceLock::new();
    let tables = decor::pieces_folder(library);
    let path = tables.join(decor::tags::PLACE_KINDS);
    let when = std::fs::metadata(&path).and_then(|meta| meta.modified()).ok();
    let mut kept = KEPT.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((known, stamp, kinds)) = kept.as_ref()
        && *known == path
        && *stamp == when
    {
        return kinds.clone();
    }
    let kinds = std::sync::Arc::new(decor::tags::PlaceKinds::of(&tables));
    *kept = Some((path, when, kinds.clone()));
    kinds
}
