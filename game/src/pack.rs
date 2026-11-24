//! A game put in one file to share (`<game>.dwpack`): a zip of the game's
//! own files only (its `.game`, the places, films, dungeons, fights, recipes
//! it reaches, its own music, sounds, pictures and tilesets) and a small
//! manifest (`dwpack.txt`). Nothing of the game's data (`data/`) goes in:
//! whoever plays it reads their own, extracted from their own cartridge. A
//! picture or a sound that comes from there is refused, by its name.
//!
//! What goes in is what is named where it is read, never a word that only
//! happens to be a file's name (a line of dialogue saying “arrival”
//! takes no film):
//!
//! - the `.game`: its `home:`, and its actions (`play`, `go`, `dungeon`,
//!   `fight`, `open`), in its chapters and their rules;
//! - a place (`decors/<name>.decor`): its `[life]` (its exits, `talk film`,
//!   the actions of its `on … end`) and its `image` lines;
//! - a film: the files it `uses` or `include`s (the project's, not the
//!   library's, which every player has), and what it plays as it is
//!   compiled: the pictures of one's own it draws (`film::Written::pictures`,
//!   those its words pick by themselves too: `film::LEXICON.own_pictures()`), its own
//!   sounds and music (`own:<name>`: `sounds/<name>.wav`, and the
//!   `music/<name>.music` it was made from), the files its lines give;
//! - a dungeon or a fight: its `fight:` and its `tileset:`.
//!
//! `library/place_songs.txt` never goes in: it is not a project's
//! file but the repository's, read from there by everyone
//! (the root's library), and it only says which of the game's songs the
//! game's own places play; a game's places say their music in their
//! `[life]` (`music n`).
//!
//! The zip is made and read by the system's `tar` (bsdtar: Windows, macOS),
//! else `zip` and `unzip`, as `spritecollab` does: no zip crate here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use assets::project::Project;

use crate::error::{Error, PackWrong};
use crate::game_file::{self, Action, GameFile};

/// The version of the pack's layout, in its manifest.
pub const FORMAT: u32 = 1;

/// The manifest's name, at the root of the pack.
pub const MANIFEST: &str = "dwpack.txt";

/// The endings of pictures and sounds, refused when they come from the data.
const MEDIA: [&str; 6] = ["png", "wav", "ogg", "mp3", "gif", "bmp"];

/// A game packed: where, its title, its files (as in the pack, the manifest left out).
#[derive(Clone, Debug)]
pub struct Packed {
    pub path: PathBuf,
    pub title: String,
    pub files: Vec<String>,
}

/// A pack's manifest, read back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub format: u32,
    pub title: String,
    pub author: String,
    /// The `.game` in the pack.
    pub game: String,
    pub files: Vec<String>,
}

impl Manifest {
    /// As it is written in the pack.
    pub fn text(&self) -> String {
        let mut text = String::from("# A game of Dungeon Workshop: its own files only, nothing of the game's data.\n");
        text.push_str(&format!("format: {}\ntitle: {}\nauthor: {}\ngame: {}\n", self.format, self.title, self.author, self.game));
        for file in &self.files {
            text.push_str(&format!("file: {file}\n"));
        }
        text
    }

    /// From its text.
    pub fn read(text: &str) -> Manifest {
        let mut manifest = Manifest::default();
        for line in text.lines().filter(|line| !line.trim_start().starts_with('#')) {
            let Some((key, value)) = line.split_once(':') else { continue };
            let value = value.trim().to_string();
            match key.trim() {
                "format" => manifest.format = value.parse().unwrap_or(0),
                "title" => manifest.title = value,
                "author" => manifest.author = value,
                "game" => manifest.game = value,
                "file" => manifest.files.push(value),
                _ => {}
            }
        }
        manifest
    }
}

/// A path with its `.` and `..` gone, made whole from where we are.
fn plain_path(path: &Path) -> PathBuf {
    let whole = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut plain = PathBuf::new();
    for part in whole.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => _ = plain.pop(),
            other => plain.push(other),
        }
    }
    plain
}

/// A path as a pack writes it: `films/arrival.film`.
fn slashed(path: &Path) -> String {
    path.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// Whether that file is there, by that very name (`Opening` is not
/// `opening.film`, even where the disk does not care).
fn is_named(path: &Path) -> bool {
    let (Some(folder), Some(name)) = (path.parent(), path.file_name()) else { return false };
    path.is_file() && std::fs::read_dir(folder).into_iter().flatten().flatten().any(|entry| entry.file_name() == name)
}

/// Whether a word is a picture's or a sound's file (`face.png`).
fn is_media(word: &str) -> bool {
    Path::new(word).extension().is_some_and(|ending| MEDIA.contains(&ending.to_string_lossy().to_ascii_lowercase().as_str()))
}

/// The words of a line, its comment left out.
fn words_of_line(line: &str) -> Vec<&str> {
    line.split('#').next().unwrap_or_default().split_whitespace().collect()
}

/// What finds the files a game reaches, by what reads them.
struct Finder {
    project: Project,
    folder: PathBuf,
    /// The game's data, and the repository's `data/`: nothing there goes in.
    data: Vec<PathBuf>,
    /// What its films are compiled with (its own pictures and sounds known).
    tables: Arc<::film::tables::Tables>,
    /// The shared library (`library`): what the game takes from it goes in too.
    shared: PathBuf,
    /// Each file as the pack holds it, and where it is read from.
    found: BTreeMap<String, PathBuf>,
    /// What was read already (`film:intro`, `place:town`…).
    read: BTreeSet<String>,
}

impl Finder {
    /// That file of the project's (`films/intro.film`), taken if it is there, else the
    /// shared library's, as the game reads it; whether it is.
    fn take(&mut self, file: String) -> bool {
        let own = self.folder.join(&file);
        let shared = self.shared.join(&file);
        let there = [own, shared].into_iter().find(|path| is_named(path));
        if let Some(path) = &there {
            self.found.entry(file).or_insert_with(|| path.clone());
        }
        there.is_some()
    }

    /// Whether that one was read already (and is now).
    fn seen(&mut self, what: String) -> bool {
        !self.read.insert(what)
    }

    /// Whether that path is in the game's data.
    fn in_data(&self, path: &Path) -> bool {
        self.data.iter().any(|data| path.starts_with(data))
    }

    /// A file's text (its line ends as written on any system).
    fn text(&self, file: &str) -> Result<String, Error> {
        let path = self.found.get(file).cloned().unwrap_or_else(|| self.folder.join(file));
        std::fs::read_to_string(&path).map(|text| text.replace("\r\n", "\n")).map_err(|why| Error::file(&path, why))
    }

    // ----- the game -----

    fn game(&mut self, from: &str, game: &GameFile) -> Result<(), Error> {
        if let Some((place, _)) = &game.home {
            self.place(from, place)?;
        }
        for chapter in &game.chapters {
            self.actions(from, &chapter.actions)?;
            for rule in &chapter.rules {
                self.actions(from, &rule.actions)?;
            }
        }
        Ok(())
    }

    /// What actions name: films, places, dungeons and fights.
    fn actions(&mut self, from: &str, actions: &[Action]) -> Result<(), Error> {
        for action in actions {
            match action {
                Action::Play(film) => self.film(from, film)?,
                Action::Go(place, _) => self.place(from, place)?,
                Action::Dungeon(word) | Action::Fight(word) => self.dungeon(from, word)?,
                Action::Open(word) if self.project.decor(word).is_file() => self.place(from, word)?,
                Action::Open(word) => self.dungeon(from, word)?,
                Action::If(branches) => {
                    for (_, actions) in branches {
                        self.actions(from, actions)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ----- the places -----

    /// A place: its file, and what its life and its pictures name.
    fn place(&mut self, _from: &str, name: &str) -> Result<(), Error> {
        let file = format!("decors/{name}.decor");
        if !self.take(file.clone()) || self.seen(format!("place:{name}")) {
            return Ok(());
        }
        // (One that does not read is the check's to tell: nothing more is taken.)
        let Ok(decor) = decor::lang::parse(&self.text(&file)?, &file) else {
            return Ok(());
        };
        for line in decor.lines.iter().filter(|line| line.verb == "image") {
            if let Some(picture) = line.words.first() {
                self.media(&file, picture)?;
            }
        }
        for way in &decor.ways_out {
            self.place(&file, &way.to)?;
        }
        for resident in &decor.residents {
            if let Some(film) = resident.talk.as_deref().and_then(|talk| talk.strip_prefix("film ")) {
                self.film(&file, film.trim())?;
            }
        }
        for rule in &decor.rules {
            let (_, read) = game_file::read_actions(&format!("{}\n{}end\n", rule.head, rule.body), &mut Vec::new());
            for rule in &read {
                self.actions(&file, &rule.actions)?;
            }
        }
        Ok(())
    }

    // ----- the films -----

    /// A film of the game's: its file, the files it takes in, and what it plays compiled.
    fn film(&mut self, _from: &str, name: &str) -> Result<(), Error> {
        let file = format!("films/{name}.film");
        if !self.take(file.clone()) || self.seen(format!("film:{name}")) {
            return Ok(());
        }
        let text = self.text(&file)?;
        self.film_text(&file, &text)?;
        let written = ::film::compile_with(&text, self.tables.clone());
        for picture in &written.pictures {
            self.media(&file, picture)?;
        }
        for (_, fields) in &written.lines {
            for field in fields {
                match field.strip_prefix("own:") {
                    Some(sound) => self.sound(sound),
                    None if is_media(field) || field.ends_with(".puppet") => self.media(&file, field)?,
                    None => {}
                }
            }
        }
        Ok(())
    }

    /// What a film's text names itself: the files it uses or takes in
    /// (read in turn), the pictures and sounds its lines give by their files.
    fn film_text(&mut self, file: &str, text: &str) -> Result<(), Error> {
        for line in text.lines() {
            let words = words_of_line(line);
            if let ["uses" | "include", name] = words.as_slice() {
                let Some(path) = ::film::shared_path(&self.project, name) else {
                    continue;
                };
                let path = plain_path(&path);
                let Some(inside) = [&self.folder, &self.shared].into_iter().find_map(|folder| path.strip_prefix(folder).ok()).map(Path::to_path_buf) else {
                    continue;
                };
                let shared = slashed(&inside);
                if self.take(shared.clone()) && !self.seen(format!("shared:{shared}")) {
                    let text = self.text(&shared)?;
                    self.film_text(&shared, &text)?;
                }
                continue;
            }
            for word in words.iter().map(|word| word.trim_matches(|c: char| matches!(c, '"' | '\'' | ',' | ';' | '(' | ')'))) {
                if is_media(word) || word.ends_with(".puppet") {
                    self.media(file, word)?;
                }
            }
        }
        Ok(())
    }

    /// A sound of one's own, by its name: its file, and the music it was made from.
    fn sound(&mut self, name: &str) {
        if self.take(format!("sounds/{name}.wav")) {
            self.take(format!("music/{name}.music"));
        }
    }

    /// A picture or a sound by its file, as a line gives it: a file of the
    /// project's, taken (a puppet with its pictures); one of the data's, refused.
    fn media(&mut self, from: &str, word: &str) -> Result<(), Error> {
        let word = word.trim_start_matches("own:");
        if word.contains(['/', '\\']) {
            return self.path(from, word);
        }
        for kind in ["images", "sounds", "tilesets"] {
            let file = format!("{kind}/{word}");
            if self.take(file.clone()) {
                if word.ends_with(".puppet") && !self.seen(format!("puppet:{file}")) {
                    self.puppet(&file)?;
                }
                return Ok(());
            }
        }
        // (By its name, a picture or a sound that only the data has: one of the game's.)
        if is_media(word) {
            for data in &self.data {
                for layer in ["hand", "hand/sounds", "extra", "rom"] {
                    let there = data.join(layer).join(word);
                    if there.is_file() {
                        return Err(Error::FromTheData { from: from.to_string(), word: word.to_string(), there });
                    }
                }
            }
        }
        Ok(())
    }

    /// A puppet's pictures, beside it.
    fn puppet(&mut self, file: &str) -> Result<(), Error> {
        let folder = Path::new(file).parent().map(slashed).unwrap_or_default();
        for line in self.text(file)?.lines() {
            for word in words_of_line(line).into_iter().filter(|word| is_media(word)) {
                self.take(format!("{folder}/{word}"));
            }
        }
        Ok(())
    }

    /// A file given by its path: a file of the project's, taken; one of the
    /// data's, refused; a picture from elsewhere, refused too (it would not
    /// be in the pack).
    fn path(&mut self, from: &str, word: &str) -> Result<(), Error> {
        let written = Path::new(word);
        let media = is_media(word);
        let places = match written.is_absolute() {
            true => vec![plain_path(written)],
            false => vec![plain_path(&self.folder.join(written)), plain_path(&self.project.root().join(written))],
        };
        for path in places {
            if self.in_data(&path) && (media || path.exists()) {
                return Err(Error::FromTheData { from: from.to_string(), word: word.to_string(), there: path });
            }
            if let Ok(inside) = path.strip_prefix(&self.folder)
                && path.is_file()
            {
                self.take(slashed(inside));
                return Ok(());
            }
            if media && path.is_file() {
                return Err(Error::Outside { from: from.to_string(), word: word.to_string() });
            }
        }
        Ok(())
    }

    // ----- the dungeons -----

    /// A dungeon of one's own (else a boss fight): its file, its fight, its tileset.
    fn dungeon(&mut self, _from: &str, word: &str) -> Result<(), Error> {
        let file = match self.take(format!("dungeons/{word}.dungeon")) {
            true => format!("dungeons/{word}.dungeon"),
            false => format!("fights/{word}.fight"),
        };
        if !self.take(file.clone()) || self.seen(format!("dive:{file}")) {
            return Ok(());
        }
        for line in self.text(&file)?.lines() {
            let Some((key, value)) = line.split('#').next().unwrap_or_default().split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "fight" => self.dungeon(&file, value)?,
                "tileset" => self.tileset(value),
                _ => {}
            }
        }
        Ok(())
    }

    /// A tileset of one's own by its word: its picture, and its frames (`<word>.1.png`…).
    fn tileset(&mut self, word: &str) {
        self.take(format!("tilesets/{word}.png"));
        let frames: Vec<String> = [self.folder.join("tilesets"), self.shared.join("tilesets")]
            .into_iter()
            .flat_map(std::fs::read_dir)
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|file| {
                file.strip_prefix(word)
                    .and_then(|rest| rest.strip_prefix('.'))
                    .and_then(|rest| rest.strip_suffix(".png"))
                    .is_some_and(|frame| !frame.is_empty() && frame.chars().all(|digit| digit.is_ascii_digit()))
            })
            .collect();
        for frame in frames {
            self.take(format!("tilesets/{frame}"));
        }
    }
}

/// The files of the game at `game` (as in the pack: `films/arrival.film`),
/// the `.game` first; or the first file that cannot go in, and why. `data`:
/// the game's data folder (nothing in it goes in). What it takes from the
/// shared library goes in too, at the same place: the pack plays alone.
pub fn files_of(context: &::film::Context, game: &Path) -> Result<Vec<String>, Error> {
    Ok(found_files(context, game)?.into_iter().map(|(file, _)| file).collect())
}

/// As `files_of`, each with the file it is read from (the game's own, else the shared library's).
fn found_files(context: &::film::Context, game: &Path) -> Result<Vec<(String, PathBuf)>, Error> {
    let read = GameFile::open(game)?;
    let project = context.library.of_game(game);
    let folder = plain_path(&read.folder);
    let shared = plain_path(&project.shared());
    let mut data = vec![plain_path(Path::new(&context.data)), plain_path(&project.root().join("data"))];
    data.dedup();
    let first = game.file_name().map(|name| name.to_string_lossy().into_owned()).ok_or(Error::Nameless)?;
    let tables = context.tables_for(&project);
    let mut finder = Finder { tables, project, folder, shared, data, found: BTreeMap::new(), read: BTreeSet::new() };
    finder.game(&first, &read)?;
    let mut files: Vec<(String, PathBuf)> = finder.found.into_iter().filter(|(file, _)| *file != first).collect();
    files.insert(0, (first, game.to_path_buf()));
    Ok(files)
}

/// The manifest of the game at `game`, with its files.
fn manifest_of(game: &Path, files: Vec<String>) -> Result<Manifest, Error> {
    let project = GameFile::open(game)?;
    let title = if project.title.is_empty() { project.name.clone() } else { project.title.clone() };
    Ok(Manifest { format: FORMAT, title, author: project.author, game: files.first().cloned().unwrap_or_default(), files })
}

/// A program run with no console window of its own (Windows; elsewhere,
/// nothing to do), as `spritecollab` runs git.
#[cfg(windows)]
fn no_window(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn no_window(_: &mut std::process::Command) {}

/// A program run, with no window; whether it went well.
fn ran(program: &str, arguments: &[&std::ffi::OsStr], at: Option<&Path>) -> bool {
    let mut command = std::process::Command::new(program);
    command.args(arguments);
    if let Some(at) = at {
        command.current_dir(at);
    }
    no_window(&mut command);
    command.output().is_ok_and(|out| out.status.success())
}

/// The game at `game` packed into `out` (a `.dwpack`, written over); `data`:
/// the game's data folder, of which nothing goes in. What the Workshop's
/// "Export the game…" calls.
pub fn pack_game(context: &::film::Context, game: &Path, out: &Path) -> Result<Packed, Error> {
    let found = found_files(context, game)?;
    let files: Vec<String> = found.iter().map(|(file, _)| file.clone()).collect();
    let manifest = manifest_of(game, files)?;
    // (Laid out apart first, the manifest beside the files, then zipped.)
    let stem = game.file_stem().map_or("game".into(), |stem| stem.to_string_lossy().into_owned());
    // (Each its own: two packed at once by one program do not share it.)
    static PACKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let made = PACKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staging = std::env::temp_dir().join(format!("dwpack_{}_{made}_{stem}", std::process::id()));
    _ = std::fs::remove_dir_all(&staging);
    for (file, from) in &found {
        let to = staging.join(file);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|why| Error::file(parent, why))?;
        }
        std::fs::copy(from, &to).map_err(|why| Error::File { path: file.clone(), why })?;
    }
    std::fs::write(staging.join(MANIFEST), manifest.text())?;
    let out = plain_path(out);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|why| Error::file(parent, why))?;
    }
    _ = std::fs::remove_file(&out);
    let tops: BTreeSet<String> = manifest.files.iter().chain([&MANIFEST.to_string()]).map(|file| file.split('/').next().unwrap_or(file).to_string()).collect();
    let tops: Vec<&std::ffi::OsStr> = tops.iter().map(std::ffi::OsStr::new).collect();
    let mut with_tar: Vec<&std::ffi::OsStr> = vec!["--format".as_ref(), "zip".as_ref(), "-cf".as_ref(), out.as_os_str(), "-C".as_ref(), staging.as_os_str()];
    with_tar.extend(&tops);
    let mut with_zip: Vec<&std::ffi::OsStr> = vec!["-q".as_ref(), "-r".as_ref(), "-X".as_ref(), out.as_os_str()];
    with_zip.extend(&tops);
    let zipped = ran("tar", &with_tar, None) || ran("zip", &with_zip, Some(&staging));
    _ = std::fs::remove_dir_all(&staging);
    if !zipped || !out.is_file() {
        return Err(Error::Pack { pack: out, wrong: PackWrong::NotZipped });
    }
    Ok(Packed { path: out, title: manifest.title, files: manifest.files })
}

/// The pack at `pack` unpacked into `into` (emptied first): its `.game`, and its manifest.
pub fn unpack(pack: &Path, into: &Path) -> Result<(PathBuf, Manifest), Error> {
    let pack = plain_path(pack);
    if !pack.is_file() {
        return Err(Error::Pack { pack, wrong: PackWrong::Missing });
    }
    _ = std::fs::remove_dir_all(into);
    std::fs::create_dir_all(into).map_err(|why| Error::file(into, why))?;
    // (Both keep what they write inside the folder.)
    let unpacked = ran("tar", &["-xf".as_ref(), pack.as_os_str(), "-C".as_ref(), into.as_os_str()], None)
        || ran("unzip", &["-q".as_ref(), pack.as_os_str(), "-d".as_ref(), into.as_os_str()], None);
    if !unpacked {
        return Err(Error::Pack { pack, wrong: PackWrong::NotUnpacked });
    }
    let Ok(text) = std::fs::read_to_string(into.join(MANIFEST)) else {
        return Err(Error::Pack { pack, wrong: PackWrong::NoManifest });
    };
    let manifest = Manifest::read(&text);
    if manifest.format > FORMAT {
        return Err(Error::Pack { pack, wrong: PackWrong::Later(manifest.format) });
    }
    let game = into.join(&manifest.game);
    if manifest.game.is_empty() || manifest.game.contains(['/', '\\']) || !game.is_file() {
        return Err(Error::Pack { pack, wrong: PackWrong::NoGame(manifest.game) });
    }
    Ok((game, manifest))
}

/// Where a pack is unpacked to be played: `<root>/target/packs/<its name>/`.
pub fn cache_of(root: &Path, pack: &Path) -> PathBuf {
    let stem = pack.file_stem().map_or("game".into(), |stem| stem.to_string_lossy().into_owned());
    root.join("target").join("packs").join(stem)
}

/// The pack at `pack` ready to play: unpacked in its cache; its `.game`
/// (whose folder is the project played: `Project::of_game`).
pub fn open_pack(root: &Path, pack: &Path) -> Result<PathBuf, Error> {
    let (game, _) = unpack(pack, &cache_of(root, pack))?;
    Ok(game)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> PathBuf {
        crate::testing::root().join("tests/games/test_village/test_village.game")
    }

    fn data() -> PathBuf {
        crate::testing::root().join("data")
    }

    #[test]
    fn the_example_game_packs_its_own_files_and_nothing_of_the_data() {
        let files = files_of(&crate::testing::context_on(&data().to_string_lossy()), &example()).expect("the example packs");
        assert_eq!(files[0], "test_village.game");
        for wanted in [
            "films/arrival.film",
            "films/back_home.film",
            "films/thanks.film",
            "decors/test_market.decor",
            "dungeons/test_forest.dungeon",
            "fights/test_forest_boss.fight",
        ] {
            assert!(files.iter().any(|file| file == wanted), "{wanted} not in {files:?}");
        }
        assert!(files.iter().all(|file| !file.starts_with("data/") && !file.contains("..")), "{files:?}");
    }

    /// A game of one film, in a folder of its own, the film saying that line.
    fn small_game(name: &str, line: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("dwpack_test_{name}_{}", std::process::id()));
        _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("films")).expect("a folder");
        std::fs::write(folder.join("small.game"), "title: Small\nauthor: Someone\n\nchapter 1 \"One\"\n  play opening\n  finish\nend\n").expect("the game");
        std::fs::write(folder.join("films/opening.film"), format!("title: Opening\n\nscene: test_market pikachu\n{line}\n")).expect("the film");
        folder.join("small.game")
    }

    #[test]
    fn a_picture_of_the_data_is_refused_by_its_name() {
        let data = std::env::temp_dir().join(format!("dwpack_test_data_{}", std::process::id()));
        std::fs::create_dir_all(data.join("hand")).expect("a data folder");
        std::fs::write(data.join("hand/ripped_face.png"), b"not really a picture").expect("a picture");
        let by_path = small_game("path", &format!("image face {}", data.join("hand/ripped_face.png").display()));
        let refused = files_of(&crate::testing::context_on(&data.to_string_lossy()), &by_path).expect_err("a picture of the data").to_string();
        assert!(refused.contains("films/opening.film") && refused.contains("ripped_face.png"), "{refused}");
        let by_name = small_game("name", "image face ripped_face.png");
        let refused = files_of(&crate::testing::context_on(&data.to_string_lossy()), &by_name).expect_err("a picture of the data, by its name").to_string();
        assert!(refused.contains("ripped_face.png") && refused.contains("game's data"), "{refused}");
        let fine = small_game("fine", "pikachu: Hello!");
        assert_eq!(files_of(&crate::testing::context_on(&data.to_string_lossy()), &fine).expect("nothing of the data"), ["small.game", "films/opening.film"]);
    }

    #[test]
    fn a_line_of_dialogue_naming_a_film_does_not_pack_it() {
        let game = small_game("said", "pikachu: Shall we watch the_ending tonight?");
        let folder = game.parent().expect("its folder").to_path_buf();
        std::fs::write(folder.join("films/the_ending.film"), "title: The End\n\nscene: test_market pikachu\npikachu: Bye!\n").expect("a film");
        assert_eq!(files_of(&crate::testing::context_on(&data().to_string_lossy()), &game).expect("packed"), ["small.game", "films/opening.film"]);
        // (Played, it goes in.)
        std::fs::write(&game, "title: Small\n\nchapter 1\n  play opening\n  play the_ending\n  finish\nend\n").expect("the game");
        assert_eq!(
            files_of(&crate::testing::context_on(&data().to_string_lossy()), &game).expect("packed"),
            ["small.game", "films/opening.film", "films/the_ending.film"]
        );
        _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    #[ignore = "needs the extracted data/ (run with --ignored)"]
    fn a_word_s_own_picture_goes_in_though_no_line_names_it() {
        let game = small_game("bobber", "pikachu cast at 100 80");
        let folder = game.parent().expect("its folder").to_path_buf();
        let film = folder.join("films/opening.film");
        std::fs::write(
            &film,
            "title: Opening
uses fishing

scene: test_market pikachu
pikachu cast at 100 80
",
        )
        .expect("the film");
        std::fs::create_dir_all(folder.join("images")).expect("its images");
        let picture = crate::testing::root().join("library/images/bobber.4.png");
        std::fs::copy(&picture, folder.join("images/bobber.4.png")).expect("the bobber");
        std::fs::copy(&picture, folder.join("images/unused.png")).expect("a picture no one draws");
        let files = files_of(&crate::testing::context_on(&data().to_string_lossy()), &game).expect("packed");
        assert_eq!(files, ["small.game", "films/opening.film", "images/bobber.4.png"]);
        _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_pack_unpacks_to_the_same_game() {
        let out = std::env::temp_dir().join(format!("dwpack_test_trip_{}", std::process::id()));
        let packed = pack_game(&crate::testing::context_on(&data().to_string_lossy()), &example(), &out.join("test_village.dwpack")).expect("packed");
        let (game, manifest) = unpack(&packed.path, &out.join("unpacked")).expect("unpacked");
        assert_eq!((manifest.format, manifest.title.as_str(), &manifest.files), (FORMAT, "Test Village", &packed.files));
        let (before, after) = (GameFile::open(&example()).expect("the example"), GameFile::open(&game).expect("the game unpacked"));
        assert_eq!(std::fs::read_to_string(example()).ok(), std::fs::read_to_string(&game).ok());
        assert_eq!((before.title, before.author, before.team, before.chapters.len()), (after.title, after.author, after.team, after.chapters.len()));
        for file in &packed.files {
            assert!(out.join("unpacked").join(file).is_file(), "{file} not unpacked");
        }
        _ = std::fs::remove_dir_all(&out);
    }
}
