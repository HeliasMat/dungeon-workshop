//! Animations and portraits made by the PMDCollab community (SpriteCollab,
//! <https://github.com/PMDCollab/SpriteCollab>), added to the game's sprites
//! and faces in `data/extra/`; and a Pokémon's sheets written out or read
//! back in SpriteBot's layout.
//!
//! - **Fetching** ([`fetch`]): only the folders asked for, by a sparse git
//!   clone into `sources/spritecollab/` (never committed); git is the only
//!   thing needed, and the network.
//! - **Importing** ([`import_monsters`], [`import_new`]): each monster's
//!   animations the game does not have (or all, `replace`) written to
//!   `data/extra/sprites/` and `data/extra/sprites.txt` in the game's own
//!   format: a row per direction, a cell per frame, the monster's centre in
//!   the middle, the frames' 60ths of a second and where the shadow lies;
//!   the faces it lacks to `data/extra/portraits/`. The animations are
//!   numbered as the original game's groups (Walk 0, Attack 1, Sleep 5,
//!   Hurt 6, Idle 7 ... Sit 23 ...), after the five the dungeons use.
//!   Pokémon of later generations get monster numbers of their own,
//!   2000 + their Pokédex number, their names (SpriteCollab's
//!   `tracker.json`) in `data/extra/monsters.txt`, their numbers in
//!   `data/extra/dex.txt`.
//! - **SpriteBot's layout** ([`export`], [`import_from`]):
//!   `sprite/AnimData.xml` and a `<Anim>-Anim.png` (and `<Anim>-Shadow.png`)
//!   per animation, `portrait/<Face>.png` and the sheet of all the faces,
//!   `portrait_sheet.png` (5 by 4 faces of 40x40, after skytemple-files'
//!   `kao/sprite_bot_sheet.py`, GPL-3). A folder or a zip of one; as a
//!   monster's, or as a variant of its own, numbered from 5000.
//!
//! The sprites are under CC BY-NC 4.0 (non-commercial use, credit given):
//! the credits of every monster imported are written to
//! `data/extra/spritecollab_credits.txt`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use image::RgbaImage;

/// Where the community's sprites are fetched from.
pub const REPOSITORY: &str = "https://github.com/PMDCollab/SpriteCollab";
/// The monster numbers of the Pokémon the game does not have: this plus their Pokédex number.
pub const NEW: u32 = 2000;
/// The last Pokédex number the game has.
pub const LAST_IN_GAME: u32 = 492;
/// The numbers of the variants imported from a SpriteBot folder.
pub const VARIANTS: u16 = 5000;
/// Past the dungeons' five, the groups (see the scenes' animations).
const SCENE: u32 = 5;
/// How far below the middle of their cells the community's sprites have a
/// monster's centre (the game's have it in the middle).
const LOWER: u32 = 4;
/// A face's side.
const FACE: u32 = 40;

/// The original game's groups of animations, by the community's names (for
/// the sprites whose AnimData.xml gives no index).
pub const GROUPS: [(&str, u32); 35] = [
    ("Walk", 0),
    ("Attack", 1),
    ("Kick", 2),
    ("Shoot", 3),
    ("Strike", 4),
    ("Sleep", 5),
    ("Hurt", 6),
    ("Idle", 7),
    ("Swing", 8),
    ("Double", 9),
    ("Hop", 10),
    ("Charge", 11),
    ("Rotate", 12),
    ("EventSleep", 13),
    ("Wake", 14),
    ("Eat", 15),
    ("Tumble", 16),
    ("Pose", 17),
    ("Pull", 18),
    ("Pain", 19),
    ("Float", 20),
    ("DeepBreath", 21),
    ("Nod", 22),
    ("Sit", 23),
    ("LookUp", 24),
    ("Sink", 25),
    ("Trip", 26),
    ("Laying", 27),
    ("LeapForth", 28),
    ("Head", 29),
    ("Cringe", 30),
    ("LostBalance", 31),
    ("TumbleBack", 32),
    ("Faint", 33),
    ("HitGround", 34),
];

/// The game's own first five (walk, idle, hurt, sleep, attack), by group.
const DUNGEON: [(u32, u32); 5] = [(0, 0), (7, 1), (6, 2), (5, 3), (1, 4)];

/// The faces of their portraits by name, in the game's order; their mirrored ones (`Normal^.png`) are left.
pub const FACES: [&str; 20] = [
    "Normal",
    "Happy",
    "Pain",
    "Angry",
    "Worried",
    "Sad",
    "Crying",
    "Shouting",
    "Teary-Eyed",
    "Determined",
    "Joyous",
    "Inspired",
    "Surprised",
    "Dizzy",
    "Special0",
    "Special1",
    "Sigh",
    "Stunned",
    "Special2",
    "Special3",
];

/// Where the files are: the data's folder (its layers), its `extra/`, the clone.
#[derive(Clone, Debug)]
pub struct Places {
    pub data: String,
    pub extra: PathBuf,
    pub clone: PathBuf,
}

impl Places {
    /// The data's folder, its `extra/` layer, and the clone the caller names
    /// (the Workshop: `DUNGEON_SPRITECOLLAB`, else `<root>/sources/spritecollab`).
    pub fn new(data: &str, clone: &Path) -> Places {
        Places { data: data.to_string(), extra: Path::new(data).join("extra"), clone: clone.to_path_buf() }
    }

    /// A file of the game, wherever its layer is.
    fn game_file(&self, file: &str) -> PathBuf {
        PathBuf::from(assets::data::path(&self.data, file))
    }

    fn sprite_folder(&self, dex: u32) -> PathBuf {
        self.clone.join("sprite").join(format!("{dex:04}"))
    }

    fn portrait_folder(&self, dex: u32) -> PathBuf {
        self.clone.join("portrait").join(format!("{dex:04}"))
    }
}

fn said(path: &Path, why: impl std::fmt::Display) -> String {
    format!("{}: {why}", path.display())
}

// --- Fetching ---

/// git run in the clone, quietly.
fn git(clone: &Path, arguments: &[&str], input: Option<&str>) -> Result<(), String> {
    use std::io::Write;
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(clone).args(arguments).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
    if input.is_some() {
        command.stdin(std::process::Stdio::piped());
    }
    no_window(&mut command);
    let mut child = command.spawn().map_err(|why| format!("git not found ({why}): install git to fetch from SpriteCollab"))?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(input.as_bytes()).map_err(|why| why.to_string())?;
    }
    let out = child.wait_with_output().map_err(|why| why.to_string())?;
    if out.status.success() { Ok(()) } else { Err(failed(&String::from_utf8_lossy(&out.stderr))) }
}

/// A program run with no console window of its own (Windows; elsewhere, nothing to do).
#[cfg(windows)]
pub fn no_window(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    // (No console window of its own.)
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
pub fn no_window(_: &mut std::process::Command) {}

/// Why git failed, plainly: the network first.
fn failed(stderr: &str) -> String {
    let all = stderr.to_lowercase();
    let network = ["could not resolve host", "unable to access", "failed to connect", "timed out", "connection", "network"];
    let end: Vec<&str> = stderr.lines().filter(|line| !line.trim().is_empty()).collect();
    let end = end[end.len().saturating_sub(4)..].join("\n");
    if network.iter().any(|word| all.contains(word)) {
        return format!("the network seems to be unavailable (SpriteCollab is on GitHub): try again once connected\n{end}");
    }
    end
}

/// The folders of these Pokédex numbers, alone (not their forms or shiny
/// ones), into the clone (`tracker`: the community's list of all,
/// `tracker.json`, too).
pub fn fetch(places: &Places, dexes: &[u32], tracker: bool) -> Result<(), String> {
    let clone = &places.clone;
    if !clone.join(".git").is_dir() {
        if let Some(parent) = clone.parent() {
            std::fs::create_dir_all(parent).map_err(|why| said(parent, why))?;
        }
        let parent = clone.parent().unwrap_or(Path::new("."));
        let name = clone.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| "spritecollab".to_string());
        git(parent, &["clone", "--depth", "1", "--filter=blob:none", "--no-checkout", REPOSITORY, &name], None)?;
        git(clone, &["sparse-checkout", "init", "--no-cone"], None)?;
    }
    // (Those fetched before kept, for their credits.)
    let listed = std::process::Command::new("git")
        .arg("-C")
        .arg(clone)
        .args(["sparse-checkout", "list"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut patterns: Vec<String> = vec!["/LICENSE.md".to_string(), "/credit_names.txt".to_string()];
    if tracker {
        patterns.push("/tracker.json".to_string());
    }
    patterns.extend(listed);
    for dex in dexes {
        for kind in ["sprite", "portrait"] {
            patterns.push(format!("/{kind}/{dex:04}/*"));
            patterns.push(format!("!/{kind}/{dex:04}/*/"));
        }
    }
    let mut seen = HashSet::new();
    patterns.retain(|pattern| seen.insert(pattern.clone()));
    // (On its input: too many for a command line.)
    git(clone, &["sparse-checkout", "set", "--no-cone", "--stdin"], Some(&(patterns.join("\n") + "\n")))?;
    git(clone, &["checkout"], None)
}

// --- Reading the community's folders ---

/// The text inside `<name>…</name>`, the first in a block.
fn tag(block: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let start = block.find(&open)? + open.len();
    let end = block[start..].find(&format!("</{name}>"))? + start;
    Some(block[start..end].trim().to_string())
}

/// An animation of a folder: its group, its cells' size, its frames' durations, the name of its file.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub group: u32,
    pub wide: u32,
    pub high: u32,
    pub durations: Vec<u32>,
    pub file: String,
}

/// A sprite folder's animations (`AnimData.xml`), by name; those copied from another given its frames.
pub fn animations(folder: &Path) -> BTreeMap<String, Animation> {
    let Ok(text) = std::fs::read_to_string(folder.join("AnimData.xml")) else {
        return BTreeMap::new();
    };
    let mut found = BTreeMap::new();
    let mut copies = Vec::new();
    for block in text.split("<Anim>").skip(1) {
        let block = block.split("</Anim>").next().unwrap_or(block);
        let Some(name) = tag(block, "Name") else { continue };
        let group =
            tag(block, "Index").and_then(|index| index.parse().ok()).or_else(|| GROUPS.iter().find(|(known, _)| *known == name).map(|(_, group)| *group));
        let Some(group) = group else { continue };
        if let Some(copy) = tag(block, "CopyOf") {
            copies.push((name, group, copy));
            continue;
        }
        let durations: Vec<u32> = block.split("<Duration>").skip(1).filter_map(|rest| rest.split("</Duration>").next()?.trim().parse().ok()).collect();
        let (Some(wide), Some(high)) =
            (tag(block, "FrameWidth").and_then(|said| said.parse().ok()), tag(block, "FrameHeight").and_then(|said| said.parse().ok()))
        else {
            continue;
        };
        found.insert(name.clone(), Animation { group, wide, high, durations, file: name });
    }
    for (name, group, copy) in copies {
        if let Some(of) = found.get(&copy).cloned() {
            found.insert(name, Animation { group, ..of });
        }
    }
    found
}

fn open(path: &Path) -> Result<RgbaImage, String> {
    Ok(image::open(path).map_err(|why| said(path, why))?.to_rgba8())
}

fn save(picture: &RgbaImage, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|why| said(parent, why))?;
    }
    picture.save(path).map_err(|why| said(path, why))
}

fn crop(picture: &RgbaImage, x: u32, y: u32, wide: u32, high: u32) -> RgbaImage {
    let mut cut = RgbaImage::new(wide, high);
    image::imageops::replace(
        &mut cut,
        &image::imageops::crop_imm(picture, x, y, wide.min(picture.width().saturating_sub(x)), high.min(picture.height().saturating_sub(y))).to_image(),
        0,
        0,
    );
    cut
}

/// An animation in the game's format: its picture, eight rows, and where
/// the shadow lies from the middle of a cell.
fn convert(folder: &Path, anim: &Animation) -> Result<(RgbaImage, (i32, i32)), String> {
    let sheet = open(&folder.join(format!("{}-Anim.png", anim.file)))?;
    let (wide, high) = (anim.wide, anim.high);
    let frames = anim.durations.len().max(1) as u32;
    let rows = (sheet.height() / high.max(1)).max(1);
    // (Their cells have the monster LOWER pixels below where the game's have
    // its centre: each is made that much taller at the bottom, the middle
    // going down with it.)
    let mut picture = RgbaImage::new(wide * frames, (high + 2 * LOWER) * 8);
    for direction in 0..8 {
        let row = crop(&sheet, 0, (direction % rows) * high, wide * frames, high);
        image::imageops::replace(&mut picture, &row, 0, i64::from(direction * (high + 2 * LOWER)));
    }
    let mut shadow = (0, 0);
    if let Ok(cells) = open(&folder.join(format!("{}-Shadow.png", anim.file))) {
        let cell = crop(&cells, 0, 0, wide, high);
        'found: for y in 0..high {
            for x in 0..wide {
                let pixel = cell.get_pixel(x, y).0;
                if pixel[..3] == [255, 255, 255] && pixel[3] > 0 {
                    shadow = (x as i32 - wide as i32 / 2, y as i32 - high as i32 / 2 - LOWER as i32);
                    break 'found;
                }
            }
        }
    }
    Ok((picture, shadow))
}

// --- The lists of data/extra ---

/// The lines of a list of data/extra so far.
fn extra_lines(places: &Places, file: &str) -> Vec<String> {
    std::fs::read_to_string(places.extra.join(file)).unwrap_or_default().lines().map(str::to_string).collect()
}

fn write_lines(places: &Places, file: &str, lines: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(&places.extra).map_err(|why| said(&places.extra, why))?;
    let path = places.extra.join(file);
    let mut text = lines.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    std::fs::write(&path, text).map_err(|why| said(&path, why))
}

/// A list of data/extra without the lines of those numbers.
fn lines_kept(places: &Places, file: &str, numbers: &HashSet<String>) -> Vec<String> {
    extra_lines(places, file).into_iter().filter(|line| !line.trim().is_empty() && !numbers.contains(line.split('\t').next().unwrap_or_default())).collect()
}

/// The sheets the game has (every layer), and those of data/extra, by monster and animation.
struct Sheets {
    known: HashSet<(u32, u32)>,
    ours: BTreeMap<(u32, u32), Vec<String>>,
}

impl Sheets {
    fn read(places: &Places) -> Sheets {
        let known = assets::data::text(&places.data, "sprites.txt")
            .lines()
            .filter_map(|line| {
                let fields: Vec<&str> = line.split('\t').collect();
                (fields.len() >= 7).then(|| Some((fields[0].parse().ok()?, fields[1].parse().ok()?)))?
            })
            .collect();
        let ours = extra_lines(places, "sprites.txt")
            .into_iter()
            .filter_map(|line| {
                let fields: Vec<String> = line.split('\t').map(str::to_string).collect();
                (fields.len() >= 7).then(|| Some(((fields[0].parse().ok()?, fields[1].parse().ok()?), fields.clone())))?
            })
            .collect();
        Sheets { known, ours }
    }

    fn write(&self, places: &Places) -> Result<(), String> {
        let lines: Vec<String> = self.ours.values().map(|fields| fields.join("\t")).collect();
        write_lines(places, "sprites.txt", &lines)
    }
}

/// A sprite folder's animations (SpriteCollab's layout) as a monster's, in
/// data/extra/sprites: those it lacks, or all (`replace`). How many.
fn add_animations(places: &Places, monster: u32, folder: &Path, replace: bool, sheets: &mut Sheets) -> Result<usize, String> {
    let mut added = 0;
    for anim in animations(folder).values() {
        let dungeon = DUNGEON.iter().find(|(group, _)| *group == anim.group).map(|(_, index)| *index);
        for index in std::iter::once(SCENE + anim.group).chain(dungeon) {
            if sheets.known.contains(&(monster, index)) && !replace {
                continue;
            }
            let Ok((picture, shadow)) = convert(folder, anim) else {
                continue;
            };
            save(&picture, &places.extra.join("sprites").join(format!("{monster}_{index}.png")))?;
            sheets.known.insert((monster, index));
            let durations: Vec<String> = anim.durations.iter().map(u32::to_string).collect();
            sheets.ours.insert(
                (monster, index),
                vec![
                    monster.to_string(),
                    index.to_string(),
                    anim.wide.to_string(),
                    (anim.high + 2 * LOWER).to_string(),
                    durations.join(","),
                    shadow.0.to_string(),
                    shadow.1.to_string(),
                ],
            );
            added += 1;
        }
    }
    Ok(added)
}

/// A portrait folder's faces (`Normal.png`...) as a monster's, those the
/// game lacks or all (`replace`). How many.
fn add_faces(places: &Places, monster: u32, folder: &Path, replace: bool) -> Result<usize, String> {
    let mut faces = 0;
    for (face, name) in FACES.iter().enumerate() {
        let source = folder.join(format!("{name}.png"));
        let file = format!("portraits/{monster}_{face}.png");
        if !source.is_file() || (!replace && places.game_file(&file).is_file()) {
            continue;
        }
        let portrait = open(&source)?;
        if portrait.dimensions() == (FACE, FACE) {
            save(&portrait, &places.extra.join(&file))?;
            faces += 1;
        }
    }
    Ok(faces)
}

/// The monsters of each Pokédex number (`dex.txt`, every layer).
pub fn dex_of(data: &str) -> HashMap<u32, u32> {
    assets::data::text(data, "dex.txt")
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
        })
        .collect()
}

/// The credits of every sprite fetched so far, written to data/extra/spritecollab_credits.txt.
fn write_credits(places: &Places) -> Result<(), String> {
    let mut lines = vec![
        "Sprites by the PMDCollab community (https://github.com/PMDCollab/SpriteCollab), CC BY-NC 4.0.".to_string(),
        "Contributors' names: sources/spritecollab/credit_names.txt.".to_string(),
    ];
    let mut dexes: Vec<String> =
        std::fs::read_dir(places.clone.join("sprite")).into_iter().flatten().flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect();
    dexes.sort();
    for dex in dexes {
        for kind in ["sprite", "portrait"] {
            let credit = places.clone.join(kind).join(&dex).join("credits.txt");
            if let Ok(text) = std::fs::read_to_string(&credit) {
                let said: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
                lines.push(format!("#{dex} {kind}: {}", said.join("; ")));
            }
        }
    }
    write_lines(places, "spritecollab_credits.txt", &lines)
}

/// These monsters' sprites and faces fetched and added (those the game
/// lacks, or all: `replace`). What was done.
pub fn import_monsters(places: &Places, monsters: &[u32], replace: bool) -> Result<String, String> {
    let dex_of = dex_of(&places.data);
    let mut dexes: Vec<u32> = monsters.iter().filter_map(|monster| dex_of.get(monster).copied()).filter(|dex| *dex > 0).collect();
    // (A monster of one's own, 2000 and on, is its Pokédex number past the game's.)
    dexes.extend(monsters.iter().filter(|monster| !dex_of.contains_key(monster) && **monster > NEW).map(|monster| monster - NEW));
    dexes.sort();
    dexes.dedup();
    if dexes.is_empty() {
        return Err("no Pokédex number for these monsters (dex.txt)".to_string());
    }
    fetch(places, &dexes, false)?;
    let mut sheets = Sheets::read(places);
    let (mut added, mut faces) = (0, 0);
    for monster in monsters {
        let Some(dex) = dex_of.get(monster).copied().or_else(|| monster.checked_sub(NEW)).filter(|dex| *dex > 0) else {
            continue;
        };
        added += add_animations(places, *monster, &places.sprite_folder(dex), replace, &mut sheets)?;
        faces += add_faces(places, *monster, &places.portrait_folder(dex), replace)?;
    }
    sheets.write(places)?;
    write_credits(places)?;
    Ok(format!("{added} animations and {faces} faces added to data/extra (credits: data/extra/spritecollab_credits.txt)"))
}

/// The community's names of every Pokémon, by Pokédex number (`tracker.json`).
pub fn tracker_names(clone: &Path) -> BTreeMap<u32, String> {
    let Some(tracker) = std::fs::read_to_string(clone.join("tracker.json")).ok().and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok()) else {
        return BTreeMap::new();
    };
    tracker
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| Some((key.parse::<u32>().ok().filter(|dex| *dex > 0)?, value.get("name")?.as_str()?.to_string())))
        .collect()
}

/// Pokémon the game does not have, given monster numbers (2000 + their
/// Pokédex number) and names (data/extra/monsters.txt, data/extra/dex.txt),
/// then their sprites and faces added. What was done.
pub fn import_new(places: &Places, dexes: &[u32]) -> Result<String, String> {
    let wanted: Vec<u32> = dexes.iter().copied().filter(|dex| *dex > LAST_IN_GAME).collect();
    if wanted.is_empty() {
        return Err(format!("only Pokédex numbers past {LAST_IN_GAME} are new to the game"));
    }
    fetch(places, &wanted, true)?;
    let names = tracker_names(&places.clone);
    for (file, value) in [("monsters.txt", true), ("dex.txt", false)] {
        let numbers: HashSet<String> = wanted.iter().map(|dex| (NEW + dex).to_string()).collect();
        let mut lines = lines_kept(places, file, &numbers);
        for dex in &wanted {
            let name = names.get(dex).cloned().unwrap_or_else(|| format!("No. {dex:04}"));
            lines.push(format!("{}\t{}", NEW + dex, if value { name } else { dex.to_string() }));
        }
        write_lines(places, file, &lines)?;
    }
    let monsters: Vec<u32> = wanted.iter().map(|dex| NEW + dex).collect();
    let mut sheets = Sheets::read(places);
    let (mut added, mut faces) = (0, 0);
    for (monster, dex) in monsters.iter().zip(&wanted) {
        added += add_animations(places, *monster, &places.sprite_folder(*dex), true, &mut sheets)?;
        faces += add_faces(places, *monster, &places.portrait_folder(*dex), true)?;
    }
    sheets.write(places)?;
    write_credits(places)?;
    let numbers: Vec<String> = monsters.iter().map(|monster| format!("m{monster}")).collect();
    Ok(format!("{} added ({}): {added} animations, {faces} faces", numbers.len(), numbers.join(", ")))
}

// --- SpriteBot's layout ---

/// Whether the last `lines` of every cell's height are transparent.
fn bottom_empty(picture: &RgbaImage, high: u32, rows: u32, lines: u32) -> bool {
    (0..rows).all(|row| {
        let low = ((row + 1) * high).min(picture.height());
        (low.saturating_sub(lines)..low).all(|y| (0..picture.width()).all(|x| picture.get_pixel(x, y).0[3] == 0))
    })
}

/// A monster's sprites and faces in SpriteBot's layout, in `target`:
/// `sprite/AnimData.xml`, `<Anim>-Anim.png` and `<Anim>-Shadow.png`,
/// `portrait/<Face>.png` and `portrait_sheet.png`. What was written.
pub fn export(places: &Places, monster: u32, target: &Path) -> Result<String, String> {
    let rows: BTreeMap<u32, Vec<String>> = assets::data::text(&places.data, "sprites.txt")
        .lines()
        .filter_map(|line| {
            let fields: Vec<String> = line.split('\t').map(str::to_string).collect();
            (fields.len() >= 7 && fields[0] == monster.to_string()).then(|| Some((fields[1].parse().ok()?, fields)))?
        })
        .collect();
    // (A group's scene animation before its dungeon one: the same, longer.)
    let mut chosen: BTreeMap<u32, u32> = BTreeMap::new();
    for index in rows.keys() {
        let group = if *index >= SCENE { Some(index - SCENE) } else { DUNGEON.iter().find(|(_, own)| own == index).map(|(group, _)| *group) };
        if let Some(group) = group
            && (!chosen.contains_key(&group) || *index >= SCENE)
        {
            chosen.insert(group, *index);
        }
    }
    let sprites = target.join("sprite");
    std::fs::create_dir_all(&sprites).map_err(|why| said(&sprites, why))?;
    let mut xml = String::from("<?xml version='1.0' encoding='utf-8'?>\n<AnimData>\n\t<ShadowSize>1</ShadowSize>\n\t<Anims>\n");
    let mut made = 0;
    for (group, index) in &chosen {
        let fields = &rows[index];
        let path = places.game_file(&format!("sprites/{monster}_{index}.png"));
        let Ok(picture) = open(&path) else {
            continue;
        };
        let number = |at: usize| fields[at].trim().parse::<i64>().unwrap_or(0);
        let (wide, high) = (number(2).max(1) as u32, number(3).max(1) as u32);
        let durations: Vec<u32> = fields[4].split(',').filter_map(|duration| duration.trim().parse().ok()).collect();
        let shadow = (number(5) as i32, number(6) as i32);
        let lines = (picture.height() / high).max(1);
        let frames = (durations.len() as u32).min(picture.width() / wide).max(1);
        // (Theirs have the centre LOWER pixels below the middle of a cell,
        // ours in it: the cells cut or grown so that it is so, the empty rows
        // at the bottom that an import adds taken back.)
        let (theirs, down) = if high > 2 * LOWER && bottom_empty(&picture, high, lines, 2 * LOWER) {
            (high - 2 * LOWER, 0)
        } else if bottom_empty(&picture, high, lines, LOWER) {
            (high, LOWER)
        } else {
            (high + 2 * LOWER, 2 * LOWER)
        };
        let name = GROUPS.iter().find(|(_, own)| own == group).map_or_else(|| format!("Anim{group}"), |(name, _)| name.to_string());
        let mut sheet = RgbaImage::new(wide * frames, theirs * lines);
        let mut shadows = RgbaImage::new(sheet.width(), sheet.height());
        for row in 0..lines {
            let band = crop(&picture, 0, row * high, wide * frames, high);
            let mut cell = RgbaImage::new(wide * frames, theirs);
            image::imageops::replace(&mut cell, &band, 0, i64::from(down));
            image::imageops::replace(&mut sheet, &cell, 0, i64::from(row * theirs));
            for frame in 0..frames {
                let x = (frame * wide + wide / 2) as i32 + shadow.0;
                let y = (row * theirs + theirs / 2 + LOWER) as i32 + shadow.1;
                if (0..shadows.width() as i32).contains(&x) && (0..shadows.height() as i32).contains(&y) {
                    shadows.put_pixel(x as u32, y as u32, image::Rgba([255, 255, 255, 255]));
                }
            }
        }
        save(&sheet, &sprites.join(format!("{name}-Anim.png")))?;
        save(&shadows, &sprites.join(format!("{name}-Shadow.png")))?;
        let told: String = durations.iter().take(frames as usize).map(|duration| format!("\t\t\t\t<Duration>{duration}</Duration>\n")).collect();
        xml.push_str(&format!(
            "\t\t<Anim>\n\t\t\t<Name>{name}</Name>\n\t\t\t<Index>{group}</Index>\n\t\t\t<FrameWidth>{wide}</FrameWidth>\n\t\t\t<FrameHeight>{theirs}</FrameHeight>\n\t\t\t<Durations>\n{told}\t\t\t</Durations>\n\t\t</Anim>\n"
        ));
        made += 1;
    }
    xml.push_str("\t</Anims>\n</AnimData>\n");
    let data = sprites.join("AnimData.xml");
    std::fs::write(&data, xml).map_err(|why| said(&data, why))?;
    // (The faces, one by one and on SpriteBot's sheet: 5 a row, 40x40.)
    let portraits = target.join("portrait");
    std::fs::create_dir_all(&portraits).map_err(|why| said(&portraits, why))?;
    let mut sheet = RgbaImage::new(FACE * 5, FACE * 4);
    let (mut used, mut faces) = ((0, 0), 0);
    for (face, name) in FACES.iter().enumerate() {
        let Ok(portrait) = open(&places.game_file(&format!("portraits/{monster}_{face}.png"))) else {
            continue;
        };
        save(&portrait, &portraits.join(format!("{name}.png")))?;
        let at = (FACE * (face as u32 % 5), FACE * (face as u32 / 5));
        image::imageops::replace(&mut sheet, &crop(&portrait, 0, 0, FACE, FACE), i64::from(at.0), i64::from(at.1));
        used = (used.0.max(at.0 + FACE), used.1.max(at.1 + FACE));
        faces += 1;
    }
    if faces > 0 {
        save(&crop(&sheet, 0, 0, used.0, used.1), &target.join("portrait_sheet.png"))?;
    }
    Ok(format!("{made} animations, {faces} faces written to {}", target.display()))
}

/// A folder of SpriteBot's layout: the one given, or a zip's, unpacked in a
/// folder of its own (to be removed after: the second value).
fn opened(source: &Path) -> Result<(PathBuf, bool), String> {
    let zipped = source.is_file() && source.extension().is_some_and(|end| end.eq_ignore_ascii_case("zip"));
    if !zipped {
        return Ok((source.to_path_buf(), false));
    }
    let folder = std::env::temp_dir().join(format!("spritebot_{}", std::process::id()));
    _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).map_err(|why| said(&folder, why))?;
    // (tar reads zips on Windows and macOS (bsdtar); unzip elsewhere. Both
    // keep what they write inside the folder.)
    let unpacked = [
        ("tar", vec!["-xf".into(), source.as_os_str().to_owned(), "-C".into(), folder.as_os_str().to_owned()]),
        ("unzip", vec!["-q".into(), source.as_os_str().to_owned(), "-d".into(), folder.as_os_str().to_owned()]),
    ]
    .into_iter()
    .any(|(program, arguments): (&str, Vec<std::ffi::OsString>)| {
        let mut command = std::process::Command::new(program);
        command.args(arguments);
        no_window(&mut command);
        command.output().is_ok_and(|out| out.status.success())
    });
    if !unpacked {
        _ = std::fs::remove_dir_all(&folder);
        return Err(format!("{}: could not be unpacked (tar or unzip)", source.display()));
    }
    Ok((folder, true))
}

/// Every file under a folder.
fn walked(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            walked(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// Where the animations are (the folder with AnimData.xml), where the faces
/// are (a folder with Normal.png), the sheet of faces if there is one.
fn found_in(folder: &Path) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
    let mut files = Vec::new();
    walked(folder, &mut files);
    files.sort();
    let named = |name: &str| files.iter().find(|file| file.file_name().is_some_and(|own| own == name)).and_then(|file| file.parent().map(Path::to_path_buf));
    let sheet = files
        .iter()
        .find(|file| {
            let low = file.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            low.ends_with(".png") && (low.contains("sheet") || low.starts_with("portrait"))
        })
        .cloned();
    (named("AnimData.xml"), named(&format!("{}.png", FACES[0])), sheet)
}

/// The faces of SpriteBot's sheet (5 a row, 40x40) as a monster's.
fn faces_of_sheet(places: &Places, path: &Path, monster: u32) -> Result<usize, String> {
    let sheet = open(path)?;
    if sheet.width() % FACE != 0 || sheet.height() % FACE != 0 || sheet.width() > FACE * 5 || sheet.height() > FACE * 8 {
        return Err(format!("{}: not a sheet of faces of 40x40, 5 a row", path.display()));
    }
    let mut faces = 0;
    for face in 0..FACES.len() as u32 {
        let at = (FACE * (face % 5), FACE * (face / 5));
        if at.0 + FACE > sheet.width() || at.1 + FACE > sheet.height() {
            continue;
        }
        let portrait = crop(&sheet, at.0, at.1, FACE, FACE);
        if portrait.pixels().all(|pixel| pixel.0[3] == 0) {
            continue;
        }
        save(&portrait, &places.extra.join("portraits").join(format!("{monster}_{face}.png")))?;
        faces += 1;
    }
    Ok(faces)
}

/// A word as a variant is named: small letters, no accents, `_` for spaces.
fn plain_word(word: &str) -> String {
    let plain: String = word
        .trim()
        .to_lowercase()
        .chars()
        .map(|letter| match letter {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            ' ' | '-' => '_',
            other => other,
        })
        .filter(|letter| letter.is_ascii_alphanumeric() || *letter == '_')
        .collect();
    plain
}

/// Whom a SpriteBot folder goes to: a monster (in place of its sheets), or
/// a variant of a species, under a word.
#[derive(Clone, Debug)]
pub enum Target {
    Monster(u32),
    Variant { word: String, of: u32 },
}

/// A folder (or zip) of SpriteBot's layout in place of a monster's
/// animations and faces, or as a variant of a species. What was done.
pub fn import_from(places: &Places, source: &Path, into: Target) -> Result<String, String> {
    let (folder, unpacked) = opened(source)?;
    let done = (|| {
        let (sprites, portraits, sheet) = found_in(&folder);
        if sprites.is_none() && portraits.is_none() && sheet.is_none() {
            return Err(format!("{}: no AnimData.xml, no Normal.png, no sheet of faces", source.display()));
        }
        let monster = match into {
            Target::Monster(monster) => monster,
            Target::Variant { word, of } => {
                let word = plain_word(&word);
                if word.is_empty() {
                    return Err("a variant needs a word".to_string());
                }
                let told = assets::data::text(&places.data, "variants.txt");
                let same = told.lines().find_map(|line| {
                    let fields: Vec<&str> = line.split('\t').collect();
                    (fields.len() > 2 && fields[2] == word).then(|| fields[0].parse::<u16>().ok())?
                });
                let taken = assets::data::text(&places.data, "monsters.txt")
                    .lines()
                    .filter_map(|line| line.split('\t').next()?.parse::<u16>().ok())
                    .filter(|number| *number >= VARIANTS)
                    .max();
                let monster = same.unwrap_or_else(|| taken.map_or(VARIANTS, |last| last + 1));
                let name: String =
                    word.split('_').filter(|part| !part.is_empty()).map(|part| part[..1].to_uppercase() + &part[1..]).collect::<Vec<_>>().join(" ");
                let numbers = HashSet::from([monster.to_string()]);
                let mut lines = lines_kept(places, "monsters.txt", &numbers);
                lines.push(format!("{monster}\t{name}"));
                write_lines(places, "monsters.txt", &lines)?;
                let mut lines = lines_kept(places, "variants.txt", &numbers);
                lines.push(format!("{monster}\t{of}\t{word}"));
                write_lines(places, "variants.txt", &lines)?;
                u32::from(monster)
            }
        };
        let added = match &sprites {
            Some(sprites) => {
                let mut sheets = Sheets::read(places);
                let added = add_animations(places, monster, sprites, true, &mut sheets)?;
                sheets.write(places)?;
                added
            }
            None => 0,
        };
        let mut faces = match &portraits {
            Some(portraits) => add_faces(places, monster, portraits, true)?,
            None => 0,
        };
        if faces == 0
            && let Some(sheet) = &sheet
        {
            faces = faces_of_sheet(places, sheet, monster)?;
        }
        Ok(format!("monster {monster}: {added} animations, {faces} faces written to data/extra"))
    })();
    if unpacked {
        _ = std::fs::remove_dir_all(&folder);
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A monster written out for SpriteBot and read back as a variant keeps its animation and faces.
    #[test]
    fn spritebot_round_trip() {
        let root = std::env::temp_dir().join(format!("spritebot_test_{}", std::process::id()));
        _ = std::fs::remove_dir_all(&root);
        let data = root.join("data");
        let rom = data.join("rom");
        std::fs::create_dir_all(rom.join("sprites")).unwrap();
        std::fs::create_dir_all(rom.join("portraits")).unwrap();
        // (An idle of 2 frames of 16x16 in 8 rows, a face.)
        let mut sheet = RgbaImage::new(32, 16 * 8);
        for y in 4..12 {
            for x in 4..12 {
                sheet.put_pixel(x, y, image::Rgba([200, 40, 40, 255]));
            }
        }
        sheet.save(rom.join("sprites").join("25_1.png")).unwrap();
        std::fs::write(rom.join("sprites.txt"), "25\t1\t16\t16\t10,12\t0\t3\n").unwrap();
        RgbaImage::from_pixel(40, 40, image::Rgba([10, 20, 30, 255])).save(rom.join("portraits").join("25_0.png")).unwrap();
        std::fs::write(rom.join("monsters.txt"), "25\tPikachu\n").unwrap();
        let places = Places { data: data.display().to_string(), extra: data.join("extra"), clone: root.join("clone") };
        let target = root.join("out");
        let said = export(&places, 25, &target).unwrap();
        assert!(said.starts_with("1 animations, 1 faces"), "{said}");
        let read = animations(&target.join("sprite"));
        assert_eq!(read["Idle"].group, 7);
        assert_eq!(read["Idle"].durations, vec![10, 12]);
        let said = import_from(&places, &target, Target::Variant { word: "Pikachu Hat".to_string(), of: 25 }).unwrap();
        assert!(said.starts_with("monster 5000: 2 animations, 1 faces"), "{said}");
        assert!(std::fs::read_to_string(data.join("extra").join("variants.txt")).unwrap().contains("5000\t25\tpikachu_hat"));
        assert!(data.join("extra").join("sprites").join("5000_1.png").is_file());
        _ = std::fs::remove_dir_all(&root);
    }
}
