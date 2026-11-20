//! The project played or checked: where its films, decors, dungeons,
//! fights, sounds, images, tilesets and recipes are. The shared library
//! (`library/`) is a project of its own; a game is a folder of its own,
//! `games/<name>/` (its `<name>.game`, `films/`, `decors/`, `dungeons/`,
//! `fights/`, `music/`, `sounds/`, `images/`, `tilesets/`), as is a game
//! unpacked from a `.dwpack`. A game's file is looked for in its folder
//! first, then in the shared library: a game may use the library's places,
//! films, pictures and sounds.
//!
//! A project is a value: each program's `main` finds the root once
//! ([`find_root`], or a folder it is told) and hands the [`Project`] down;
//! no library code looks for it by itself.
//!
//! The root is the same layout in the repository and in a release unzipped
//! (`tools/release.sh`): `library/`, `games/`, `film/recipes/` and
//! `data/` side by side, the programs at its top in a release.

use std::path::{Path, PathBuf};

/// The shared library's folder, under the root.
pub const LIBRARY: &str = "library";

/// The folder of a project, and the root it belongs to (the shared library
/// is the root's `library/`: each file is looked for there too, after the
/// project's own). The default one is nowhere: empty paths.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Project {
    folder: PathBuf,
    root: PathBuf,
}

/// Whether a folder is a root: it has the shared library's films.
pub fn is_root(folder: &Path) -> bool {
    folder.join(LIBRARY).join("films").is_dir()
}

/// The root the programs work in, for a program's `main`: the first folder
/// up from where it is run, then up from the program itself (a release
/// unzipped, its programs at its top), that [`is_root`]. None: neither.
pub fn find_root() -> Option<PathBuf> {
    let here = std::env::current_dir().ok();
    let exe = std::env::current_exe().ok();
    here.iter().chain(exe.iter()).flat_map(|start| start.ancestors()).find(|folder| is_root(folder)).map(Path::to_path_buf)
}

/// The folders a new game is made of, under its own (`games/<name>/`).
pub const GAME_FOLDERS: [&str; 8] = ["films", "decors", "dungeons", "fights", "music", "sounds", "images", "tilesets"];

impl Project {
    /// The shared library of that root: `<root>/library`.
    pub fn library(root: impl Into<PathBuf>) -> Project {
        let root = root.into();
        Project { folder: root.join(LIBRARY), root }
    }

    /// The shared library of the root [`find_root`] finds, for a program's
    /// `main`; what to say when there is none.
    pub fn found() -> Result<Project, String> {
        find_root().map(Project::library).ok_or_else(|| {
            format!("no {LIBRARY}/ found, neither up from here nor beside the program: run it from the repository or from the folder of a release")
        })
    }

    /// The project in that folder (laid as `library/` is), of the same root.
    pub fn at(&self, folder: impl Into<PathBuf>) -> Project {
        Project { folder: folder.into(), root: self.root.clone() }
    }

    /// The project a game's file is in (`<folder>/<name>.game`), of the same root.
    pub fn of_game(&self, game: &Path) -> Project {
        match game.parent().filter(|folder| !folder.as_os_str().is_empty()) {
            Some(folder) => self.at(folder),
            None => self.at("."),
        }
    }

    /// The shared library of its root.
    pub fn the_library(&self) -> Project {
        Project::library(&self.root)
    }

    /// Its root: `library/`, `games/`, `film/recipes/`, `data/` side by side.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Its folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Whether it is the shared library itself.
    pub fn is_library(&self) -> bool {
        same(&self.folder, &self.shared())
    }

    /// Its folder of that kind (`films`, `decors`…).
    pub fn kind(&self, kind: &str) -> PathBuf {
        self.folder.join(kind)
    }

    pub fn films(&self) -> PathBuf {
        self.kind("films")
    }

    pub fn decors(&self) -> PathBuf {
        self.kind("decors")
    }

    pub fn dungeons(&self) -> PathBuf {
        self.kind("dungeons")
    }

    pub fn fights(&self) -> PathBuf {
        self.kind("fights")
    }

    pub fn sounds(&self) -> PathBuf {
        self.kind("sounds")
    }

    pub fn images(&self) -> PathBuf {
        self.kind("images")
    }

    pub fn tilesets(&self) -> PathBuf {
        self.kind("tilesets")
    }

    pub fn music(&self) -> PathBuf {
        self.kind("music")
    }

    /// The recipes of its own (`films/recipes`).
    pub fn recipes(&self) -> PathBuf {
        self.films().join("recipes")
    }

    /// Where the files of that kind (`images`, `sounds`…) are looked for, in
    /// turn: the project's, then the shared library's (if it is another).
    pub fn folders(&self, kind: &str) -> Vec<PathBuf> {
        let mut folders = vec![self.kind(kind)];
        if !self.is_library() {
            folders.push(self.shared().join(kind));
        }
        folders
    }

    /// The first of those folders that has that file, else the project's.
    pub fn find(&self, kind: &str, file: &str) -> PathBuf {
        let folders = self.folders(kind);
        folders.iter().map(|folder| folder.join(file)).find(|path| path.exists()).unwrap_or_else(|| self.kind(kind).join(file))
    }

    /// A file of that kind by its word (`films`, `intro`, `film`):
    /// `films/intro.film`, the project's, else the shared library's.
    pub fn file(&self, kind: &str, word: &str, extension: &str) -> PathBuf {
        self.find(kind, &format!("{word}.{extension}"))
    }

    pub fn film(&self, word: &str) -> PathBuf {
        self.file("films", word, "film")
    }

    pub fn decor(&self, word: &str) -> PathBuf {
        self.file("decors", word, "decor")
    }

    pub fn dungeon(&self, word: &str) -> PathBuf {
        self.file("dungeons", word, "dungeon")
    }

    pub fn fight(&self, word: &str) -> PathBuf {
        self.file("fights", word, "fight")
    }

    /// The words of the files of that kind with that extension, the
    /// project's and the shared library's, each once, in order.
    pub fn names(&self, kind: &str, extension: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .folders(kind)
            .into_iter()
            .flat_map(|folder| std::fs::read_dir(folder).into_iter().flatten().flatten())
            .filter(|entry| entry.path().is_file())
            .filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(&format!(".{extension}")).map(str::to_string))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// The shared library's folder (`<root>/library`).
    pub fn shared(&self) -> PathBuf {
        self.root.join(LIBRARY)
    }

    /// Where the games are, a folder each: `<root>/games`.
    pub fn games_folder(&self) -> PathBuf {
        self.root.join("games")
    }

    /// The folders of every game, in order (`games/<name>/`).
    pub fn game_folders(&self) -> Vec<PathBuf> {
        let mut folders: Vec<PathBuf> =
            std::fs::read_dir(self.games_folder()).into_iter().flatten().flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
        folders.sort();
        folders
    }

    /// A game by its path, else by its name: `games/<name>/<name>.game`, then
    /// `library/<name>.game` (`my_game`, `my_game.game`).
    pub fn find_game(&self, name: &str) -> Option<PathBuf> {
        let path = Path::new(name);
        if path.is_file() && path.extension().is_some_and(|end| end == "game") {
            return Some(path.to_path_buf());
        }
        let word = name.trim_end_matches(".game");
        if word.is_empty() || word.contains(['/', '\\']) {
            return None;
        }
        let file = format!("{word}.game");
        [self.games_folder().join(word).join(&file), self.shared().join(&file)].into_iter().find(|path| path.is_file())
    }

    /// Every game there is: `games/<name>/<name>.game` (a folder each), then
    /// the `.game` files of `library/`, each list by name.
    pub fn games(&self) -> Vec<PathBuf> {
        let files_in = |folder: PathBuf| {
            let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|end| end == "game") && path.is_file())
                .collect();
            found.sort();
            found
        };
        let mut found: Vec<PathBuf> = self.game_folders().into_iter().flat_map(files_in).collect();
        found.extend(files_in(self.shared()));
        found
    }

    /// A new game's folder made: `games/<word>/`, its folders, its `<word>.game`
    /// written with that text. Its file; none made if the folder is there already.
    pub fn new_game(&self, word: &str, text: &str) -> Result<PathBuf, String> {
        let folder = self.games_folder().join(word);
        if folder.exists() {
            return Err(format!("{} is there already", folder.display()));
        }
        for kind in GAME_FOLDERS {
            std::fs::create_dir_all(folder.join(kind)).map_err(|why| format!("{}: {why}", folder.join(kind).display()))?;
        }
        let file = folder.join(format!("{word}.game"));
        std::fs::write(&file, text).map_err(|why| format!("{}: {why}", file.display()))?;
        Ok(file)
    }
}

/// Whether two folders are the same one, however written.
fn same(one: &Path, other: &Path) -> bool {
    one == other || matches!((one.canonicalize(), other.canonicalize()), (Ok(one), Ok(other)) if one == other)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repository's root, where the tests run from.
    fn repository() -> Project {
        crate::testing::library()
    }

    #[test]
    fn a_project_looks_in_its_own_folders_then_the_library_s() {
        let library = repository();
        assert!(library.is_library());
        assert_eq!(library.folders("images"), vec![library.images()]);
        let game = library.of_game(Path::new("target/packs/small/small.game"));
        assert_eq!(game.films(), Path::new("target/packs/small/films"));
        assert_eq!(game.folders("sounds"), vec![game.sounds(), library.sounds()]);
        assert!(is_root(library.root()));
    }

    #[test]
    fn a_game_s_file_is_its_own_first_then_the_shared_library_s() {
        let folder = std::env::temp_dir().join(format!("dw_project_{}", std::process::id()));
        _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("films")).expect("a folder");
        std::fs::write(folder.join("films/intro.film"), "title: Intro\n").expect("a film");
        let game = repository().at(&folder);
        assert_eq!(game.film("intro"), folder.join("films/intro.film"));
        // (Neither its own nor the library's: where its own would be.)
        assert_eq!(game.film("nowhere"), folder.join("films/nowhere.film"));
        assert!(game.names("films", "film").contains(&"intro".to_string()));
        assert_eq!(game.find_game("no/such/game"), None);
        _ = std::fs::remove_dir_all(&folder);
    }
}
