//! The whole of SpriteCollab at once (`scene_player spritecollab all`):
//! every folder fetched ([`fetch_all`]), then every species and form
//! imported ([`import_all`]); later, only what changed on GitHub
//! ([`update`]).
//!
//! - The game's Pokémon get the animations and faces they lack (never over
//!   the game's own, `rom/` and `hand/`, unless asked); their second gender
//!   (600 further on) the community's female folder if there is one.
//! - The Pokémon past the game's get 2000 + their Pokédex number, as
//!   [`import_new`](super::import_new) gives them.
//! - The forms (Alolan Vulpix, Pikachu's caps...) become variants
//!   (`variants.txt`), numbered from 5000 and kept by their word
//!   (`vulpix_alola`), so that a second import gives them the same numbers;
//!   but the forms the game has as Pokémon of its own, in the community's
//!   order (Unown's letters, Castform's weathers, Deoxys...), go to those.
//!   Shiny ones are left.
//!
//! What is done is written as it goes, `data/extra/spritecollab_done.txt`
//! (the commit, then each folder done): a run cut short goes on where it
//! stopped, and an update does again only the folders that changed.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::{
    FACES, NEW, Over, Places, REPOSITORY, Sheets, VARIANTS, add_animations, add_faces, dex_of, git, git_said, plain_word, said, sparse, write_credits,
};

/// Where a bulk import is: data/extra/spritecollab_done.txt.
const DONE: &str = "spritecollab_done.txt";
/// How many folders between two saves of the lists.
const SAVE_EVERY: usize = 25;
/// How many Pokédex numbers between two lines of progress.
const TELL_EVERY: usize = 50;
/// The game's second gender: its monsters this many further on.
const SECOND: u32 = 600;
/// The Pokédex numbers whose forms the game has, as Pokémon of their own,
/// in the community's order: Unown, Castform, Kecleon, Deoxys, Burmy,
/// Wormadam, Cherrim, Shellos, Gastrodon, Shaymin.
const IN_ORDER: [u32; 10] = [201, 351, 352, 386, 412, 413, 421, 422, 423, 492];

/// A folder of the community's: a species (form 0) or one of its forms, by Pokédex number.
pub type Unit = (u32, u32);

/// What an import did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    /// Pokémon past the game's, given a number.
    pub species: usize,
    /// Forms given as variants.
    pub forms: usize,
    /// The game's own Pokémon looked at.
    pub monsters: usize,
    pub animations: usize,
    pub faces: usize,
    /// Folders done by an earlier run, left.
    pub before: usize,
}

impl std::fmt::Display for Totals {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            out,
            "{} new species, {} forms as variants, {} of the game's Pokémon looked at: {} animations and {} faces added ({} folders done before, left as they were)",
            self.species, self.forms, self.monsters, self.animations, self.faces, self.before
        )
    }
}

// --- Fetching ---

/// The whole repository into the clone (its every folder, a few GB): cloned
/// if it is not there, made whole if it was only some folders. Run again,
/// it goes on with what is missing.
pub fn fetch_all(places: &Places, progress: &mut dyn FnMut(&str)) -> Result<(), String> {
    let clone = &places.clone;
    if !clone.join(".git").is_dir() {
        let parent = clone.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|why| said(parent, why))?;
        let name = clone.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| "spritecollab".to_string());
        progress(&format!("cloning {REPOSITORY} into {} (its list of files first)", clone.display()));
        git(parent, &["clone", "--depth", "1", "--filter=blob:none", "--no-checkout", REPOSITORY, &name], None)?;
    }
    if sparse(clone) {
        progress("the clone had only some folders: all of them asked now");
        git(clone, &["sparse-checkout", "disable"], None)?;
    }
    let files = git_said(clone, &["ls-tree", "-r", "--name-only", "HEAD"])?.lines().count();
    progress(&format!("checking out {files} files (a few GB, the long part; if it stops, run it again to go on)"));
    git(clone, &["reset", "--hard", "HEAD"], None)?;
    progress(&format!("fetched, at commit {}", head(clone)));
    Ok(())
}

/// The commit the clone is at (none: nothing).
fn head(clone: &Path) -> String {
    git_said(clone, &["rev-parse", "HEAD"]).map(|said| said.trim().to_string()).unwrap_or_default()
}

/// What changed on GitHub since the last import, fetched; the folders it
/// touches to be imported again. How many.
pub fn update(places: &Places, progress: &mut dyn FnMut(&str)) -> Result<usize, String> {
    let clone = &places.clone;
    if !clone.join(".git").is_dir() || sparse(clone) {
        fetch_all(places, progress)?;
        return Ok(0);
    }
    let old = head(clone);
    progress("asking GitHub what changed");
    git(clone, &["fetch", "--depth", "1", "origin", "HEAD"], None)?;
    let new = git_said(clone, &["rev-parse", "FETCH_HEAD"])?.trim().to_string();
    let mut done = Done::read(places);
    let since = if done.commit.is_empty() { old.clone() } else { done.commit.clone() };
    let paths = if since == new {
        String::new()
    } else {
        // (The commit of the last import may be gone from a shallow clone: the clone's own then.)
        git_said(clone, &["diff", "--name-only", &since, &new]).or_else(|_| git_said(clone, &["diff", "--name-only", &old, &new]))?
    };
    if old != new {
        progress(&format!("checking out commit {new}"));
        git(clone, &["reset", "--hard", &new], None)?;
    }
    let changed = units_of(paths.lines());
    let again = changed.iter().filter(|unit| done.units.remove(unit)).count();
    done.commit = new;
    done.write(places)?;
    progress(&format!("{} folders changed on GitHub, {again} of them imported before: to do again", changed.len()));
    Ok(changed.len())
}

/// The folders that paths of the repository are in (`sprite/0025/0001/Idle-Anim.png`: 25, form 1).
pub fn units_of<'a>(paths: impl Iterator<Item = &'a str>) -> BTreeSet<Unit> {
    paths
        .filter_map(|path| {
            let parts: Vec<&str> = path.trim().split('/').collect();
            if parts.len() < 3 || !["sprite", "portrait"].contains(&parts[0]) {
                return None;
            }
            let dex = number(parts[1])?;
            // (A folder under it, not a file of its own: a form, `0000` being the species' shiny or female ones.)
            let form = if parts.len() > 3 { number(parts[2])? } else { 0 };
            Some((dex, form))
        })
        .collect()
}

/// A folder's name as a number: four digits.
fn number(name: &str) -> Option<u32> {
    (name.len() == 4 && name.bytes().all(|byte| byte.is_ascii_digit())).then(|| name.parse().ok())?
}

// --- Where the import is ---

/// The folders imported, and the commit they were imported from.
struct Done {
    commit: String,
    units: BTreeSet<Unit>,
}

impl Done {
    fn read(places: &Places) -> Done {
        let text = std::fs::read_to_string(places.extra.join(DONE)).unwrap_or_default();
        let mut done = Done { commit: String::new(), units: BTreeSet::new() };
        for line in text.lines() {
            if let Some(commit) = line.strip_prefix("commit ") {
                done.commit = commit.trim().to_string();
            } else if let Some((dex, form)) = line.split_once(' ')
                && let (Some(dex), Some(form)) = (number(dex), number(form))
            {
                done.units.insert((dex, form));
            }
        }
        done
    }

    fn write(&self, places: &Places) -> Result<(), String> {
        let mut text = format!("commit {}\n", self.commit);
        for (dex, form) in &self.units {
            text.push_str(&format!("{dex:04} {form:04}\n"));
        }
        std::fs::create_dir_all(&places.extra).map_err(|why| said(&places.extra, why))?;
        let path = places.extra.join(DONE);
        std::fs::write(&path, text).map_err(|why| said(&path, why))
    }
}

/// A list of data/extra by number (`monsters.txt`...), the lines with none kept as they are.
struct List {
    file: &'static str,
    numbered: BTreeMap<u32, String>,
    other: Vec<String>,
}

impl List {
    fn read(places: &Places, file: &'static str) -> List {
        let mut list = List { file, numbered: BTreeMap::new(), other: Vec::new() };
        for line in std::fs::read_to_string(places.extra.join(file)).unwrap_or_default().lines().filter(|line| !line.trim().is_empty()) {
            match line.split('\t').next().and_then(|first| first.trim().parse().ok()) {
                Some(key) => _ = list.numbered.insert(key, line.to_string()),
                None => list.other.push(line.to_string()),
            }
        }
        list
    }

    fn set(&mut self, key: u32, rest: &str) {
        self.numbered.insert(key, format!("{key}\t{rest}"));
    }

    fn write(&self, places: &Places) -> Result<(), String> {
        let lines: Vec<String> = self.other.iter().chain(self.numbered.values()).cloned().collect();
        super::write_lines(places, self.file, &lines)
    }
}

/// The species' names and their forms', by Pokédex number (`tracker.json`).
fn forms_named(clone: &Path) -> BTreeMap<u32, (String, BTreeMap<u32, String>)> {
    let Some(tracker) = std::fs::read_to_string(clone.join("tracker.json")).ok().and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok()) else {
        return BTreeMap::new();
    };
    let mut named = BTreeMap::new();
    for (key, species) in tracker.as_object().into_iter().flatten() {
        let Some(dex) = number(key) else { continue };
        let name = species.get("name").and_then(|name| name.as_str()).unwrap_or_default().to_string();
        let forms = species
            .get("subgroups")
            .and_then(|forms| forms.as_object())
            .into_iter()
            .flatten()
            .filter_map(|(form, told)| Some((number(form)?, told.get("name")?.as_str()?.to_string())))
            .collect();
        named.insert(dex, (name, forms));
    }
    named
}

/// The folder of a unit, of sprites or of portraits; `female`: its second gender's.
fn folder(clone: &Path, kind: &str, (dex, form): Unit, female: bool) -> PathBuf {
    let mut folder = clone.join(kind).join(format!("{dex:04}"));
    if form > 0 || female {
        folder = folder.join(format!("{form:04}"));
    }
    if female { folder.join("0000").join("0002") } else { folder }
}

/// Whether a folder has something to import: animations, or faces.
fn has_any(sprites: &Path, portraits: &Path) -> bool {
    sprites.join("AnimData.xml").is_file() || FACES.iter().any(|face| portraits.join(format!("{face}.png")).is_file())
}

/// The forms of a species the clone has (0: the species itself), each with something to import.
fn forms_of(clone: &Path, dex: u32) -> BTreeSet<u32> {
    let mut forms = BTreeSet::new();
    if has_any(&folder(clone, "sprite", (dex, 0), false), &folder(clone, "portrait", (dex, 0), false)) {
        forms.insert(0);
    }
    for kind in ["sprite", "portrait"] {
        for entry in std::fs::read_dir(clone.join(kind).join(format!("{dex:04}"))).into_iter().flatten().flatten() {
            let Some(form) = number(&entry.file_name().to_string_lossy()).filter(|form| *form > 0) else { continue };
            if has_any(&folder(clone, "sprite", (dex, form), false), &folder(clone, "portrait", (dex, form), false)) {
                forms.insert(form);
            }
        }
    }
    forms
}

/// The Pokédex numbers the clone has folders of.
fn dexes_of(clone: &Path) -> BTreeSet<u32> {
    ["sprite", "portrait"]
        .iter()
        .flat_map(|kind| std::fs::read_dir(clone.join(kind)).into_iter().flatten().flatten())
        .filter_map(|entry| number(&entry.file_name().to_string_lossy()))
        .filter(|dex| *dex > 0)
        .collect()
}

/// Every species and form of the clone imported (see the module): what the
/// game lacks, over what an earlier import wrote (`Over::Ours`), or over
/// everything (`Over::All`, every folder done again). The folders done by an
/// earlier run are left; progress told as it goes.
pub fn import_all(places: &Places, over: Over, progress: &mut dyn FnMut(&str)) -> Result<Totals, String> {
    let clone = &places.clone;
    if !clone.join("sprite").is_dir() && !clone.join("portrait").is_dir() {
        return Err(format!("{}: no SpriteCollab there, fetch it first (scene_player spritecollab all)", clone.display()));
    }
    let mut done = Done::read(places);
    if over == Over::All {
        done.units.clear();
    }
    let at = head(clone);
    if !at.is_empty() {
        done.commit = at;
    }
    let named = forms_named(clone);
    let dex_of = dex_of(&places.data);
    let mut game: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (monster, dex) in &dex_of {
        if *dex > 0 && *monster < SECOND {
            game.entry(*dex).or_default().push(*monster);
        }
    }
    game.values_mut().for_each(|monsters| monsters.sort_unstable());
    let mut monsters = List::read(places, "monsters.txt");
    let mut dexes = List::read(places, "dex.txt");
    let mut variants = List::read(places, "variants.txt");
    // (A form keeps its number by its word; a new one takes the next.)
    let mut words: HashMap<String, u32> = assets::tables::variants(&places.data).into_iter().map(|variant| (variant.word, u32::from(variant.number))).collect();
    let taken = assets::data::text(&places.data, "monsters.txt").lines().filter_map(|line| line.split('\t').next()?.trim().parse::<u32>().ok()).max();
    let mut next = words.values().copied().chain(taken).filter(|number| *number >= u32::from(VARIANTS)).max().map_or(u32::from(VARIANTS), |last| last + 1);
    let mut sheets = Sheets::read(places);
    let mut totals = Totals::default();
    let all = dexes_of(clone);
    let mut unsaved = 0;
    progress(&format!("importing {} Pokédex numbers from {}", all.len(), clone.display()));
    for (count, dex) in all.iter().copied().enumerate() {
        let (species, forms) = named.get(&dex).cloned().unwrap_or_default();
        let species = if species.is_empty() { format!("No. {dex:04}") } else { species };
        let own = game.get(&dex).cloned().unwrap_or_default();
        for form in forms_of(clone, dex) {
            let unit = (dex, form);
            if done.units.contains(&unit) {
                totals.before += 1;
                continue;
            }
            // (Whom the folder goes to, and the folder of each: the second gender its own if there is one.)
            let mut whom: Vec<(u32, bool)> = Vec::new();
            let game_form = if form == 0 {
                own.first()
            } else if IN_ORDER.contains(&dex) {
                own.get(form as usize)
            } else {
                None
            };
            if let Some(monster) = game_form.copied() {
                whom.push((monster, false));
                if dex_of.get(&(monster + SECOND)) == Some(&dex) {
                    whom.push((monster + SECOND, true));
                }
                totals.monsters += 1;
            } else {
                let of = own.first().copied().unwrap_or(NEW + dex);
                if form == 0 {
                    monsters.set(of, &species);
                    dexes.set(of, &dex.to_string());
                    whom.push((of, false));
                    totals.species += 1;
                } else {
                    let word = form_word(&species, &forms, form);
                    let number = *words.entry(word.clone()).or_insert_with(|| {
                        next += 1;
                        next - 1
                    });
                    let name: Vec<String> = word.split('_').filter(|part| !part.is_empty()).map(|part| part[..1].to_uppercase() + &part[1..]).collect();
                    monsters.set(number, &name.join(" "));
                    variants.set(number, &format!("{of}\t{word}"));
                    whom.push((number, false));
                    totals.forms += 1;
                }
            }
            for (monster, female) in whom {
                let mut sprites = folder(clone, "sprite", unit, female);
                let mut portraits = folder(clone, "portrait", unit, female);
                if female && !sprites.join("AnimData.xml").is_file() {
                    sprites = folder(clone, "sprite", unit, false);
                }
                if female && !portraits.join(format!("{}.png", FACES[0])).is_file() {
                    portraits = folder(clone, "portrait", unit, false);
                }
                // (The Pokémon of the game's alone have sheets of their own to keep.)
                let over = if monster < NEW { over } else { Over::All };
                totals.animations += add_animations(places, monster, &sprites, over, &mut sheets)?;
                totals.faces += add_faces(places, monster, &portraits, over)?;
            }
            done.units.insert(unit);
            unsaved += 1;
            if unsaved >= SAVE_EVERY {
                save(places, &sheets, [&monsters, &dexes, &variants], &done)?;
                unsaved = 0;
            }
        }
        if (count + 1) % TELL_EVERY == 0 || count + 1 == all.len() {
            progress(&format!("No. {dex:04} ({} of {}): {totals}", count + 1, all.len()));
        }
    }
    save(places, &sheets, [&monsters, &dexes, &variants], &done)?;
    write_credits(places)?;
    progress("credits written to data/extra/spritecollab_credits.txt (CC BY-NC 4.0: non-commercial use, credit given)");
    Ok(totals)
}

/// The word a form takes as a variant (`vulpix_alola`, `eevee_alternate_two`), of its species' name and its forms' names.
fn form_word(species: &str, forms: &BTreeMap<u32, String>, form: u32) -> String {
    let told = forms.get(&form).map(|name| plain_word(name)).filter(|name| !name.is_empty()).unwrap_or_else(|| format!("form{form}"));
    super::variant_word(&format!("{species}_{told}"))
}

/// The words the import gives the forms (`eevee_alternate_two`), each with its folder.
pub fn form_words(clone: &Path) -> HashMap<String, Unit> {
    let mut words = HashMap::new();
    for (dex, (species, forms)) in forms_named(clone) {
        let species = if species.is_empty() { format!("No. {dex:04}") } else { species };
        for form in forms.keys().filter(|form| **form > 0) {
            words.insert(form_word(&species, &forms, *form), (dex, *form));
        }
    }
    words
}

/// A folder of the clone not imported yet: where it is, its species' name,
/// its form's (none: the species itself), the variant word it will take (a form's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missing {
    pub unit: Unit,
    pub species: String,
    pub form: String,
    pub word: String,
}

/// Every folder of the clone with something to import that no import has
/// done yet (`spritecollab_done.txt`): what [`import_all`] would add.
pub fn not_imported(places: &Places) -> Vec<Missing> {
    let clone = &places.clone;
    let done = Done::read(places);
    let named = forms_named(clone);
    let mut missing = Vec::new();
    for dex in dexes_of(clone) {
        let (species, forms) = named.get(&dex).cloned().unwrap_or_default();
        let species = if species.is_empty() { format!("No. {dex:04}") } else { species };
        for form in forms_of(clone, dex).into_iter().filter(|form| !done.units.contains(&(dex, *form))) {
            let (told, word) = if form == 0 {
                (String::new(), String::new())
            } else {
                (forms.get(&form).cloned().unwrap_or_else(|| format!("Form {form}")), form_word(&species, &forms, form))
            };
            missing.push(Missing { unit: (dex, form), species: species.clone(), form: told, word });
        }
    }
    missing
}

/// One who made a part of a sprite or a portrait: their name, where to find them, when, what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credit {
    pub portrait: bool,
    pub name: String,
    pub contact: String,
    pub date: String,
    /// Still in the sprite (`CUR`), or replaced since (`OLD`).
    pub current: bool,
    pub what: Vec<String>,
}

/// The credits of every folder imported (`data/extra/spritecollab_credits.txt`),
/// the authors named after `credit_names.txt` of the clone.
pub fn credits(places: &Places) -> HashMap<Unit, Vec<Credit>> {
    let names: HashMap<String, (String, String)> = std::fs::read_to_string(places.clone.join("credit_names.txt"))
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let (name, discord, contact) = (fields.next()?.trim(), fields.next()?.trim(), fields.next().unwrap_or("").trim());
            Some((discord.to_string(), (name.to_string(), contact.to_string())))
        })
        .collect();
    let text = std::fs::read_to_string(places.extra.join("spritecollab_credits.txt")).unwrap_or_default();
    credits_in(&text, &names)
}

/// The credits of a credits file's text, the authors named by `names` (their Discord id: name, contact).
pub fn credits_in(text: &str, names: &HashMap<String, (String, String)>) -> HashMap<Unit, Vec<Credit>> {
    let mut found: HashMap<Unit, Vec<Credit>> = HashMap::new();
    for line in text.lines() {
        let Some((head, entries)) = line.strip_prefix('#').and_then(|line| line.split_once(": ")) else { continue };
        let Some((folder, kind)) = head.split_once(' ') else { continue };
        let mut parts = folder.split('/');
        let Some(dex) = parts.next().and_then(number) else { continue };
        let Some(form) = parts.next().map_or(Some(0), number) else { continue };
        for entry in entries.split("; ") {
            let fields: Vec<&str> = entry.split('\t').collect();
            if fields.len() < 5 {
                continue;
            }
            let author = fields[1].trim();
            let (name, contact) = names.get(author).cloned().unwrap_or_else(|| (author.to_string(), String::new()));
            let what = fields[4].split(',').map(str::trim).filter(|what| !what.is_empty() && !what.ends_with('^')).map(str::to_string).collect();
            found.entry((dex, form)).or_default().push(Credit {
                portrait: kind == "portrait",
                name,
                contact,
                date: fields[0].split(' ').next().unwrap_or_default().to_string(),
                current: fields[2] == "CUR",
                what,
            });
        }
    }
    found
}

/// What is imported so far written: the sheets, the lists, where it is (last: a run cut short does the rest again).
fn save(places: &Places, sheets: &Sheets, lists: [&List; 3], done: &Done) -> Result<(), String> {
    sheets.write(places)?;
    for list in lists {
        list.write(places)?;
    }
    done.write(places)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    /// A sprite folder of SpriteCollab's: an Idle of 2 frames of 16x16, 8 rows, its shadow.
    fn sprite_folder(folder: &Path, also: &str) {
        std::fs::create_dir_all(folder).unwrap();
        let mut xml = String::from("<AnimData><Anims>");
        for name in ["Idle", also] {
            xml.push_str(&format!(
                "<Anim><Name>{name}</Name><FrameWidth>16</FrameWidth><FrameHeight>16</FrameHeight><Durations><Duration>10</Duration><Duration>12</Duration></Durations></Anim>"
            ));
            let mut sheet = RgbaImage::new(32, 16 * 8);
            sheet.put_pixel(8, 8, image::Rgba([200, 40, 40, 255]));
            sheet.save(folder.join(format!("{name}-Anim.png"))).unwrap();
        }
        xml.push_str("</Anims></AnimData>");
        std::fs::write(folder.join("AnimData.xml"), xml).unwrap();
    }

    fn face(folder: &Path) {
        std::fs::create_dir_all(folder).unwrap();
        RgbaImage::from_pixel(40, 40, image::Rgba([10, 20, 30, 255])).save(folder.join("Normal.png")).unwrap();
    }

    /// A game of one Pokémon (Pikachu, its Idle its own) and a clone with Pikachu, a form of it, and a later one.
    fn fixture(name: &str) -> (PathBuf, Places) {
        let root = std::env::temp_dir().join(format!("spritecollab_bulk_{name}_{}", std::process::id()));
        _ = std::fs::remove_dir_all(&root);
        let rom = root.join("data").join("rom");
        std::fs::create_dir_all(rom.join("sprites")).unwrap();
        std::fs::write(rom.join("dex.txt"), "25\t25\n625\t25\n").unwrap();
        std::fs::write(rom.join("monsters.txt"), "25\tPikachu\n625\tPikachu\n").unwrap();
        // (The game's own Idle of Pikachu: 5 + 7.)
        std::fs::write(rom.join("sprites.txt"), "25\t12\t16\t16\t10\t0\t0\n").unwrap();
        let clone = root.join("clone");
        sprite_folder(&clone.join("sprite").join("0025"), "Sit");
        face(&clone.join("portrait").join("0025"));
        sprite_folder(&clone.join("sprite").join("0025").join("0002"), "Sit");
        sprite_folder(&clone.join("sprite").join("0495"), "Sit");
        face(&clone.join("portrait").join("0495"));
        std::fs::write(
            clone.join("tracker.json"),
            r#"{"0025": {"name": "Pikachu", "subgroups": {"0000": {"name": ""}, "0002": {"name": "Rock_Star"}}}, "0495": {"name": "Snivy", "subgroups": {}}}"#,
        )
        .unwrap();
        let data = root.join("data");
        let places = Places { data: data.display().to_string(), extra: data.join("extra"), clone };
        (root, places)
    }

    /// Every folder imported once: the game's sheets kept, the new species and the form numbered, the faces added; a second run does nothing again.
    #[test]
    fn all_imported_once() {
        let (root, places) = fixture("once");
        let totals = import_all(&places, Over::Ours, &mut |_| {}).unwrap();
        assert_eq!((totals.species, totals.forms, totals.monsters, totals.before), (1, 1, 1, 0), "{totals}");
        let sheets = std::fs::read_to_string(places.extra.join("sprites.txt")).unwrap();
        // (Pikachu: its own Idle (12) kept; Sit (5 + 23); its second gender too; Idle's dungeon one (1) it lacks.)
        assert!(!sheets.lines().any(|line| line.starts_with("25\t12\t")), "{sheets}");
        assert!(sheets.lines().any(|line| line.starts_with("25\t28\t")), "{sheets}");
        assert!(sheets.lines().any(|line| line.starts_with("625\t28\t")), "{sheets}");
        assert!(sheets.lines().any(|line| line.starts_with("2495\t12\t")), "{sheets}");
        assert!(sheets.lines().any(|line| line.starts_with("5000\t28\t")), "{sheets}");
        assert!(places.extra.join("portraits").join("2495_0.png").is_file());
        let monsters = std::fs::read_to_string(places.extra.join("monsters.txt")).unwrap();
        assert!(monsters.contains("2495\tSnivy") && monsters.contains("5000\tPikachu Rock Star"), "{monsters}");
        let variants = std::fs::read_to_string(places.extra.join("variants.txt")).unwrap();
        assert!(variants.contains("5000\t25\tpikachu_rock_star"), "{variants}");
        assert!(std::fs::read_to_string(places.extra.join("dex.txt")).unwrap().contains("2495\t495"));
        let again = import_all(&places, Over::Ours, &mut |_| {}).unwrap();
        assert_eq!((again.animations, again.faces, again.before), (0, 0, 3), "{again}");
        _ = std::fs::remove_dir_all(&root);
    }

    /// A run cut short goes on: the folders not marked done are imported, and a form keeps its number.
    #[test]
    fn goes_on_where_it_stopped() {
        let (root, places) = fixture("resume");
        import_all(&places, Over::Ours, &mut |_| {}).unwrap();
        // (As if it had stopped before the form and the new species.)
        let done = std::fs::read_to_string(places.extra.join(DONE)).unwrap();
        let kept: Vec<&str> = done.lines().filter(|line| !line.starts_with("0025 0002") && !line.starts_with("0495")).collect();
        std::fs::write(places.extra.join(DONE), kept.join("\n")).unwrap();
        let totals = import_all(&places, Over::Ours, &mut |_| {}).unwrap();
        assert_eq!((totals.species, totals.forms, totals.before), (1, 1, 1), "{totals}");
        let variants = std::fs::read_to_string(places.extra.join("variants.txt")).unwrap();
        assert_eq!(variants.lines().count(), 1, "{variants}");
        assert!(variants.contains("5000\t25\tpikachu_rock_star"), "{variants}");
        _ = std::fs::remove_dir_all(&root);
    }

    /// What is not imported yet, with the word a form will take; nothing once all is imported.
    #[test]
    fn not_imported_then_all_imported() {
        let (root, places) = fixture("missing");
        let missing = not_imported(&places);
        let units: Vec<Unit> = missing.iter().map(|one| one.unit).collect();
        assert_eq!(units, vec![(25, 0), (25, 2), (495, 0)]);
        assert_eq!((missing[1].form.as_str(), missing[1].word.as_str()), ("Rock_Star", "pikachu_rock_star"));
        assert_eq!(missing[2].species, "Snivy");
        assert_eq!(form_words(&places.clone).get("pikachu_rock_star"), Some(&(25, 2)));
        import_all(&places, Over::Ours, &mut |_| {}).unwrap();
        assert!(not_imported(&places).is_empty());
        _ = std::fs::remove_dir_all(&root);
    }

    /// The credits read: by folder, the authors named, the mirrored faces left.
    #[test]
    fn credits_read() {
        let names = HashMap::from([("<@!1>".to_string(), ("Audino".to_string(), "https://github.com/audinowho".to_string()))]);
        let text = "Sprites by…\n#0025 sprite: 2020-10-07 17:58:43.1\tCHUNSOFT\tCUR\tUnspecified\tWalk,Idle\n\
                    #0025/0002 portrait: 2021-01-01 10:00:00.0\t<@!1>\tOLD\tPMDCollab_1\tNormal,Normal^; 2022-02-02 1:1:1.0\t<@!9>\tCUR\tPMDCollab_1\tHappy\n";
        let credits = credits_in(text, &names);
        assert_eq!(credits[&(25, 0)][0].name, "CHUNSOFT");
        assert_eq!(credits[&(25, 0)][0].what, vec!["Walk", "Idle"]);
        let form = &credits[&(25, 2)];
        assert_eq!((form[0].name.as_str(), form[0].portrait, form[0].current, form[0].date.as_str()), ("Audino", true, false, "2021-01-01"));
        assert_eq!(form[0].what, vec!["Normal"]);
        assert_eq!(form[1].name, "<@!9>", "an author not named: as written");
    }

    /// The folders a change touches: a species', a form's, the species' shiny ones as its own.
    #[test]
    fn changed_folders() {
        let paths =
            ["sprite/0025/Idle-Anim.png", "portrait/0037/0001/Normal.png", "sprite/0025/0000/0001/Walk-Anim.png", "tracker.json", "sprite/0495/AnimData.xml"];
        let units: Vec<Unit> = units_of(paths.into_iter()).into_iter().collect();
        assert_eq!(units, vec![(25, 0), (37, 1), (495, 0)]);
    }
}
