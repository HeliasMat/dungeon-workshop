//! Where the game's files are. Its folder holds layers: `rom/`, only what
//! `extractor` writes; `extra/`, what the tools add; `hand/`, what is made
//! by hand. A file is looked for in each, a small cost a file with no list
//! of them made. The lists of `indexes.txt` are `rom/`'s with `extra/`'s
//! over them, by key, then the places `decor` built over both
//! (`extra/decor_<list>`: each file has one writer, see `data/DATA.md`).
//! Under `extra/`, `built/`: the places a game built itself, on demand, as
//! it showed them (`BUILT`): what `decor` builds in `extra/` comes first.
//! A folder without layers (a new extraction tried
//! beside the one in use) is read as it is.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

/// The layers, the first that has a file giving it.
const LAYERS: [&str; 4] = ["hand", "extra", BUILT, "rom"];

/// The layer of the places a game built on demand, its decor not yet built
/// (`game`): a cache, under what `decor` writes in `extra/`.
pub const BUILT: &str = "built";

/// Where the places built on demand are written, for the game's files in that folder.
pub fn built_folder(folder: &str) -> PathBuf {
    Path::new(folder).join(BUILT)
}

/// What `decor` writes of a list, beside the tools' own in `extra/`:
/// `decor_town.txt` over `town.txt`.
pub const DECORS: &str = "decor_";

/// Each list by how many of a line's first fields are its key; 0: blocks,
/// each from a `place` line, by its number. (`film_core/src/tables/files.rs`
/// reads it too.)
const INDEXES: &str = include_str!("indexes.txt");

fn is_there(path: &str) -> bool {
    Path::new(path).exists()
}

/// Where a file of the game is, by its name within the folder (none has
/// it: where it would be, reading it then fails as any missing file).
pub fn path(folder: &str, file: &str) -> String {
    LAYERS.iter().map(|layer| format!("{folder}/{layer}/{file}")).find(|path| is_there(path)).unwrap_or_else(|| format!("{folder}/{file}"))
}

/// Whether the folder has that file.
pub fn has(folder: &str, file: &str) -> bool {
    is_there(&path(folder, file))
}

/// A file's bytes.
pub fn read(folder: &str, file: &str) -> Option<Vec<u8>> {
    std::fs::read(path(folder, file)).ok()
}

/// A line's key, as `key` says it is told.
fn key_of(line: &str, key: usize) -> Option<&str> {
    match key {
        0 => line.strip_prefix("place\t").map(|rest| rest.split('\t').next().unwrap_or(rest)),
        _ => Some(line.match_indices('\t').nth(key - 1).map_or(line, |(at, _)| &line[..at])),
    }
}

/// `rom` without what `extra` gives again, then `extra`.
fn merged(rom: &str, extra: &str, key: usize) -> String {
    let given: HashSet<&str> = extra.lines().filter_map(|line| key_of(line, key)).collect();
    let mut every = String::with_capacity(rom.len() + extra.len());
    let mut left_out = false;
    for line in rom.lines() {
        if let Some(key) = key_of(line, key) {
            left_out = given.contains(key);
        }
        if !left_out {
            every.push_str(line);
            every.push('\n');
        }
    }
    every + extra
}

/// A list's key, as `indexes.txt` tells it; none: not a list.
pub fn key(file: &str) -> Option<usize> {
    INDEXES.lines().find_map(|line| line.strip_prefix(file)?.strip_prefix('\t')?.parse().ok())
}

/// The files a list is made of, the one read first first: `rom/`'s, the
/// places built on demand (`built/`, as `decor` writes them), the tools' in
/// `extra/`, `decor`'s in `extra/`.
fn parts(folder: &str, file: &str) -> [String; 5] {
    [
        format!("{folder}/rom/{file}"),
        format!("{folder}/{BUILT}/{file}"),
        decor_part(&format!("{folder}/{BUILT}"), file),
        format!("{folder}/extra/{file}"),
        decor_part(&format!("{folder}/extra"), file),
    ]
}

/// What `decor` wrote of a list in that layer: `decor_<list>`.
fn decor_part(layer: &str, file: &str) -> String {
    format!("{layer}/{DECORS}{file}")
}

/// A text of the game, the lists merged (none there: nothing).
pub fn text(folder: &str, file: &str) -> String {
    let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default();
    let [rom, over @ ..] = parts(folder, file);
    match key(file).filter(|_| over.iter().any(|path| is_there(path))) {
        Some(key) => over.iter().fold(read(&rom), |every, path| merged(&every, &read(path), key)),
        None => read(&path(folder, file)),
    }
}

/// When the files a text is made of last changed.
type Stamp = [Option<SystemTime>; 6];

fn stamp(folder: &str, file: &str) -> Stamp {
    let when = |path: &str| std::fs::metadata(path).and_then(|meta| meta.modified()).ok();
    let [rom, built, built_decors, extra, decors] = parts(folder, file);
    [when(&rom), when(&built), when(&built_decors), when(&extra), when(&decors), when(&path(folder, file))]
}

/// [`text`], kept: read and merged once, and again only once one of its
/// files changes on disk (a place remade while a film plays).
pub fn text_kept(folder: &str, file: &str) -> Arc<str> {
    type Kept = Mutex<HashMap<(String, String), (Stamp, Arc<str>)>>;
    static KEPT: OnceLock<Kept> = OnceLock::new();
    let now = stamp(folder, file);
    let key = (folder.to_string(), file.to_string());
    let mut kept = KEPT.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((when, text)) = kept.get(&key)
        && *when == now
    {
        return text.clone();
    }
    let read: Arc<str> = text(folder, file).into();
    kept.insert(key, (now, read.clone()));
    read
}

/// The layers a folder has, with their names, nothing naming the folder
/// itself where it has none.
pub fn layers(folder: &str) -> Vec<(PathBuf, &'static str)> {
    let root = Path::new(folder);
    let there: Vec<(PathBuf, &'static str)> = LAYERS.iter().map(|&layer| (root.join(layer), layer)).filter(|(path, _)| path.is_dir()).collect();
    if there.is_empty() { vec![(root.to_path_buf(), "")] } else { there }
}

/// Where an extraction into `folder` writes: its `rom/`, if it has layers.
pub fn rom_folder(folder: &str) -> String {
    let layer = format!("{folder}/rom");
    if Path::new(&layer).is_dir() { layer } else { folder.to_string() }
}

/// The game's files a film shows from (`data/DATA.md`), as a packet of
/// it (`scene_player pack`) copies them, each in its layer.
pub mod film_files {
    /// What every film needs, whatever it shows.
    pub const FILES: [&str; 15] = [
        "interface.bin",
        "dungeons.bin",
        "sound.bin",
        "entities.txt",
        "monsters.txt",
        "items.txt",
        "sprites.txt",
        "props.txt",
        "effects.txt",
        "status.txt",
        "backdrops.txt",
        "town.txt",
        "ground.txt",
        "points.txt",
        "numbers.png",
    ];
    /// Those a film can do without (`decor`'s lists: none before a place is
    /// built; what the dungeons' rules add to `dungeons.bin`).
    pub const IF_ANY: [&str; 8] =
        ["credits.txt", "variants.txt", "rod_tips.txt", "decor_backdrops.txt", "decor_town.txt", "decor_ground.txt", "dungeon_extras.bin", "guests.txt"];
    /// The folders taken whole.
    pub const WHOLE: [&str; 2] = ["effects", "status"];
    /// The folders of numbered files: those of the Pokémon, the places shown.
    pub const OF_MONSTERS: [&str; 2] = ["sprites", "portraits"];
    pub const OF_PLACES: [&str; 3] = ["grounds/info", "grounds/pictures", "backdrops"];
}

/// The folder a program plays from, worked out once by its `main` and
/// handed down: the one `DUNGEON_DATA` names (to try a new extraction
/// beside the one in use), else the root's `data/` (the repository's, or
/// the one beside the programs of a release, where the first launch
/// extracts it). The only place that variable is read (the tests too go
/// through it: `crate::testing::data`).
pub fn folder(root: &Path) -> String {
    if let Ok(folder) = std::env::var("DUNGEON_DATA") {
        return folder;
    }
    root.join("data").to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own, with those files in it.
    fn folder_with(name: &str, files: &[(&str, &str)]) -> String {
        let folder = std::env::temp_dir().join(format!("dungeon_data_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        for (file, text) in files {
            let path = folder.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        folder.to_string_lossy().into_owned()
    }

    #[test]
    fn a_kept_text_is_read_once_and_again_when_it_changes() {
        let folder = folder_with(
            "kept",
            &[(
                "rom/town.txt",
                "place	1
solid	010
",
            )],
        );
        let first = text_kept(&folder, "town.txt");
        assert!(Arc::ptr_eq(&first, &text_kept(&folder, "town.txt")), "not read again");
        assert_eq!(crate::tables::solid_rows(&folder, 1), vec![vec![false, true, false]]);
        // (A place remade while a film plays: its walls read anew.)
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(
            format!("{folder}/rom/town.txt"),
            "place	1
solid	11
place	2
solid	1
",
        )
        .unwrap();
        assert_eq!(
            &*text_kept(&folder, "town.txt"),
            "place	1
solid	11
place	2
solid	1
"
        );
        assert_eq!(crate::tables::solid_rows(&folder, 1), vec![vec![true, true]]);
        assert_eq!(crate::tables::solid_rows(&folder, 2), vec![vec![true]]);
        assert!(crate::tables::solid_rows(&folder, 3).is_empty());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_file_comes_from_the_first_layer_that_has_it() {
        let folder =
            folder_with("layers", &[("rom/a.png", "rom"), ("extra/a.png", "extra"), ("rom/b.png", "rom"), ("hand/c.png", "hand"), ("rom/c.png", "rom")]);
        assert_eq!(read(&folder, "a.png").unwrap(), b"extra");
        assert_eq!(read(&folder, "b.png").unwrap(), b"rom");
        assert_eq!(read(&folder, "c.png").unwrap(), b"hand");
        assert!(!has(&folder, "d.png"));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_folder_without_layers_is_read_as_it_is() {
        let folder = folder_with("flat", &[("monsters.txt", "1\tBulbasaur\n"), ("sprites/1_0.png", "picture")]);
        assert_eq!(text(&folder, "monsters.txt"), "1\tBulbasaur\n");
        assert_eq!(read(&folder, "sprites/1_0.png").unwrap(), b"picture");
        assert_eq!(rom_folder(&folder), folder);
        assert_eq!(layers(&folder), vec![(PathBuf::from(&folder), "")]);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_list_is_the_rom_s_with_the_extra_lines_over_it() {
        let rom = "1\t0\tof the rom\n1\t1\treplaced\n2\t0\tof the rom\n";
        let extra = "1\t1\tnew\n3\t0\tadded\n";
        let folder = folder_with(
            "lists",
            &[("rom/sprites.txt", rom), ("extra/sprites.txt", extra), ("rom/items.txt", "1\tBerry\n"), ("extra/story.txt", "not a list\n")],
        );
        assert_eq!(text(&folder, "sprites.txt"), "1\t0\tof the rom\n2\t0\tof the rom\n1\t1\tnew\n3\t0\tadded\n");
        assert_eq!(text(&folder, "items.txt"), "1\tBerry\n");
        assert_eq!(text(&folder, "story.txt"), "not a list\n");
        assert_eq!(text(&folder, "nothing.txt"), "");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_places_decor_built_go_over_the_tools_and_the_rom_s() {
        let folder = folder_with(
            "decors",
            &[
                ("rom/backdrops.txt", "1\tONE\n2\tTWO\n"),
                ("extra/decor_backdrops.txt", "2\tOURS\n459\tNEW\n"),
                ("extra/ground.txt", "1\tjeu\t0\n459\tjeu\t1\n"),
                ("extra/decor_ground.txt", "459\tjeu\t0\n"),
            ],
        );
        assert_eq!(text(&folder, "backdrops.txt"), "1\tONE\n2\tOURS\n459\tNEW\n");
        assert_eq!(text(&folder, "ground.txt"), "1\tjeu\t0\n459\tjeu\t0\n");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_list_of_one_field_goes_by_its_number_not_by_its_start() {
        assert_eq!(merged("20\tTwenty\n2\tTwo\n", "2\tOther\n", 1), "20\tTwenty\n2\tOther\n");
    }

    #[test]
    fn a_place_is_replaced_whole() {
        let rom = "moment\t1\nplace\t1\tOne\t2\t2\nsolid\t00\nplace\t2\tTwo\t2\t2\nsolid\t11\nsolid\t11\nplace\t3\tThree\t2\t2\nsolid\t01\n";
        let extra = "place\t2\tTwo again\t2\t1\nsolid\t10\nplace\t4\tFour\t2\t1\nsolid\t00\n";
        let every = "moment\t1\nplace\t1\tOne\t2\t2\nsolid\t00\nplace\t3\tThree\t2\t2\nsolid\t01\nplace\t2\tTwo again\t2\t1\nsolid\t10\nplace\t4\tFour\t2\t1\nsolid\t00\n";
        assert_eq!(merged(rom, extra, 0), every);
    }

    #[test]
    fn an_extraction_goes_into_the_rom_layer() {
        let folder = folder_with("rom", &[("rom/a.txt", "")]);
        assert_eq!(rom_folder(&folder), format!("{folder}/rom"));
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
