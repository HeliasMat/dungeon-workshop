//! What is wrong with a dungeon of one's own, in the spirit of SkyTemple's
//! dungeon validator: floors where nobody is met or nothing lies, weights of
//! nothing, Pokémon and items the game does not know, a look, a song or a
//! fight that do not exist, floors said beyond the last, a look the
//! generator often fails on; then what the rules say of it.
//!
//! Shared by the Workshop's dungeon editor (which shows each problem where
//! it is, and puts the plain ones right by a button) and by the check of a
//! whole game (`project_check`), which reads the dungeon from its file.

use std::path::Path;

use language::Diagnostic;
use rules::dungeon::{DungeonRng, generate_floor};
use rules::own::plain;
use rules::run::Book;
use rules::tables::Floor as FloorTable;

/// The floors something is on (from 1, both ends in); none: all of them.
pub type Floors = Option<(usize, usize)>;

/// A Pokémon met there: its species, its level, how often, on which floors.
#[derive(Clone, Debug, PartialEq)]
pub struct Foe {
    pub species: String,
    pub level: i32,
    pub often: u16,
    pub floors: Floors,
}

/// An item lying there: its name, how often, on which floors.
#[derive(Clone, Debug, PartialEq)]
pub struct Lying {
    pub item: String,
    pub often: u16,
    pub floors: Floors,
}

/// `Pidgey 6 x2 floors 2-3`: what is said, and the floors.
pub fn on_floors(said: &str) -> (&str, Floors) {
    let words: Vec<&str> = said.split_whitespace().collect();
    if words.len() >= 3 && matches!(plain(words[words.len() - 2]).as_str(), "floor" | "floors") {
        let range = words[words.len() - 1];
        let (low, high) = range.split_once('-').unwrap_or((range, range));
        if let (Ok(low), Ok(high)) = (low.trim().parse::<usize>(), high.trim().parse::<usize>()) {
            let cut = said.rfind(words[words.len() - 2]).unwrap_or(said.len());
            return (said[..cut].trim(), Some((low.min(high).max(1), low.max(high))));
        }
    }
    (said, None)
}

/// `Apple x3`: what is said, and how often (1 if not said).
fn weighed(part: &str) -> (String, u16) {
    match part.rsplit_once(" x").and_then(|(name, weight)| Some((name.trim().to_string(), weight.trim().parse::<u16>().ok()?))) {
        Some(found) => found,
        None => (part.trim().to_string(), 1),
    }
}

/// What a dungeon says, as far as its check looks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dungeon {
    pub name: String,
    pub floors: usize,
    pub look: String,
    pub look_floor: usize,
    pub music: String,
    /// Its boss fight, a file of its own (`fight: <word>`; empty: none).
    pub fight: String,
    /// A boss fight written in it (`boss: …`).
    pub boss: bool,
    pub foes: Vec<Foe>,
    pub items: Vec<Lying>,
}

impl Dungeon {
    /// As a `.dungeon` file says it.
    pub fn read(text: &str) -> Dungeon {
        let mut dungeon = Dungeon { floors: 3, look_floor: 1, ..Dungeon::default() };
        for line in text.lines() {
            let Some((key, value)) = line.trim().split('#').next().unwrap_or_default().split_once(':') else {
                continue;
            };
            let value = value.trim();
            match plain(key).as_str() {
                "name" => dungeon.name = value.to_string(),
                "floors" => dungeon.floors = value.parse().unwrap_or(3),
                "look" => match value.rsplit_once(' ').and_then(|(name, floor)| Some((name, floor.parse::<usize>().ok()?))) {
                    Some((name, floor)) => (dungeon.look, dungeon.look_floor) = (name.to_string(), floor),
                    None => (dungeon.look, dungeon.look_floor) = (value.to_string(), 1),
                },
                "music" => dungeon.music = value.to_string(),
                "fight" => dungeon.fight = value.to_string(),
                "boss" => dungeon.boss = true,
                "pokemon" => {
                    for part in value.split(',').filter(|part| !part.trim().is_empty()) {
                        let (part, floors) = on_floors(part.trim());
                        let (said, often) = weighed(part);
                        let (species, level) = match said.rsplit_once(' ').and_then(|(name, level)| Some((name.to_string(), level.parse::<i32>().ok()?))) {
                            Some(found) => found,
                            None => (said, 5),
                        };
                        dungeon.foes.push(Foe { species, level, often, floors });
                    }
                }
                "items" => {
                    for part in value.split(',').filter(|part| !part.trim().is_empty()) {
                        let (part, floors) = on_floors(part.trim());
                        let (item, often) = weighed(part);
                        dungeon.items.push(Lying { item, often, floors });
                    }
                }
                _ => {}
            }
        }
        dungeon
    }
}

/// What the game's tables offer: the dungeons whose floors can be taken
/// (their number, their name, how many floors), the Pokémon met in its
/// dungeons, its items.
#[derive(Clone, Debug, Default)]
pub struct Known {
    pub looks: Vec<(u16, String, usize)>,
    pub species: Vec<(u16, String)>,
    pub items: Vec<String>,
}

impl Known {
    /// As the game's tables have them.
    pub fn of(book: &Book) -> Known {
        let tables = &book.tables;
        let looks = tables
            .dungeon_names
            .iter()
            .enumerate()
            .filter_map(|(id, name)| {
                let floors = tables.dungeons.get(id)?.floors.iter().filter(|floor| floor.fixed.is_none()).count();
                (floors > 0 && !name.is_empty()).then(|| (id as u16, name.clone(), floors))
            })
            .collect();
        let met: std::collections::BTreeSet<u16> =
            tables.dungeons.iter().flat_map(|dungeon| &dungeon.floors).flat_map(|floor| &floor.enemies).map(|enemy| enemy.species).collect();
        let mut species: Vec<(u16, String)> =
            met.into_iter().filter_map(|id| Some((id, tables.species.get(id as usize)?.name.clone()))).filter(|(_, name)| !name.is_empty()).collect();
        species.sort_by_key(|a| plain(&a.1));
        species.dedup_by(|a, b| plain(&a.1) == plain(&b.1));
        let mut items: Vec<String> = tables.items.iter().filter(|name| !name.is_empty() && !name.starts_with('[')).cloned().collect();
        items.sort_by_key(|name| plain(name));
        items.dedup();
        Known { looks, species, items }
    }
}

/// The game's floor a look says (`Apple Woods`, `12`, and a floor of it).
pub fn look_floor<'a>(known: &Known, book: &'a Book, look: &str, floor: usize) -> Option<&'a FloorTable> {
    let id = look.trim().parse::<u16>().ok().or_else(|| known.looks.iter().find(|(_, name, _)| plain(name) == plain(look)).map(|(id, ..)| *id))?;
    let shaped: Vec<&FloorTable> = book.tables.dungeons.get(id as usize)?.floors.iter().filter(|floor| floor.fixed.is_none()).collect();
    shaped.get(floor.max(1).min(shaped.len()).checked_sub(1)?).copied()
}

/// Where a problem is: a field of the dungeon, a Pokémon or an item of its
/// lists (by their rank), a floor (from 1), its file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    Name,
    Floors,
    Look,
    Music,
    Fight,
    Foes,
    Foe(usize),
    Item(usize),
    Floor(usize),
    File,
}

impl Place {
    /// The key of the line of the file it is on (`pokemon`), if one.
    pub fn key(self) -> Option<&'static str> {
        Some(match self {
            Place::Name => "name",
            Place::Floors => "floors",
            Place::Look => "look",
            Place::Music => "music",
            Place::Fight => "fight",
            Place::Foes | Place::Foe(_) | Place::Floor(_) => "pokemon",
            Place::Item(_) => "items",
            Place::File => return None,
        })
    }
}

/// How a problem is put right.
#[derive(Clone, Copy, Debug)]
pub enum Fix {
    OneFloor,
    /// The Pokémon met on the look's own floor.
    LookFoes,
    /// That Pokémon (that item) met on that floor too.
    FoeOn(usize, usize),
    ItemOn(usize, usize),
    RemoveFoe(usize),
    RemoveItem(usize),
    FoeOften(usize),
    ItemOften(usize),
    FoeLevel(usize),
    /// Its floors cut to the dungeon's.
    FoeFloors(usize),
    ItemFloors(usize),
    LookDefault,
    LookFloor(usize),
    NoMusic,
    NoFight,
}

impl Fix {
    pub fn said(self) -> &'static str {
        match self {
            Fix::OneFloor => "one floor",
            Fix::LookFoes => "take those of the look's floor",
            Fix::FoeOn(..) | Fix::ItemOn(..) => "stretch it to this floor (the nearest)",
            Fix::RemoveFoe(_) | Fix::RemoveItem(_) => "remove it",
            Fix::FoeOften(_) | Fix::ItemOften(_) => "often: 1",
            Fix::FoeLevel(_) => "a level from 1 to 100",
            Fix::FoeFloors(_) | Fix::ItemFloors(_) => "its floors cut at the last",
            Fix::LookDefault => "the look of Apple Woods",
            Fix::LookFloor(_) => "the look's last floor",
            Fix::NoMusic => "the look's",
            Fix::NoFight => "no fight",
        }
    }
}

/// The codes of what the check finds (those the rules find are
/// `rules::own::codes`).
pub mod codes {
    /// The dungeon does not play.
    pub const GRAVE: &str = "E-DUNGEON-CHECK";
    /// It plays, but likely not as meant.
    pub const REMARK: &str = "R-DUNGEON-CHECK";
}

/// A problem: what is wrong (an error: the dungeon does not play; a
/// remark: it plays, but likely not as meant), where in the dungeon it is,
/// how to put it right.
#[derive(Clone, Debug)]
pub struct Problem {
    pub told: Diagnostic,
    pub place: Place,
    pub fix: Option<Fix>,
}

impl Problem {
    fn new(said: String, place: Place, fix: Option<Fix>, grave: bool) -> Problem {
        let told = match grave {
            true => Diagnostic::said(said).coded(codes::GRAVE),
            false => Diagnostic::said(said).coded(codes::REMARK).as_remark(),
        };
        Problem { told, place, fix }
    }

    /// The dungeon does not play.
    pub fn grave(&self) -> bool {
        self.told.is_error()
    }
}

/// Of floors said (from 1, both ends in), the one nearest a floor: its rank.
fn nearest(ranges: impl Iterator<Item = Floors>, at: usize) -> Option<usize> {
    ranges
        .enumerate()
        .filter_map(|(rank, floors)| floors.map(|(low, high)| (rank, if at < low { low - at } else { at.saturating_sub(high) })))
        .min_by_key(|(_, far)| *far)
        .map(|(rank, _)| rank)
}

/// What the rules say that is not said here already: the lines of
/// Pokémon, items, look, music, fight, and their floors, are.
fn said_by_rules(played: &str, wrong: Vec<Diagnostic>) -> Vec<Diagnostic> {
    const OURS: [&str; 5] = ["pokemon", "items", "look", "music", "fight"];
    const SAID: [&str; 4] = ["it needs a name", "nobody lives there", "0 floors", "floor "];
    let lines: Vec<&str> = played.lines().collect();
    wrong
        .into_iter()
        .filter(|why| !SAID.iter().any(|said| why.to_string().starts_with(said)))
        .filter(|why| {
            let number = Some(why.line()).filter(|line| *line > 0);
            let key = number.and_then(|number| lines.get(number - 1)).and_then(|line| line.split_once(':')).map(|(key, _)| plain(key));
            key.is_none_or(|key| !OURS.contains(&key.as_str()))
        })
        .collect()
}

/// Everything wrong with a dungeon: by the game's tables (none: its data
/// is missing), its text as played (its fight read in: `dive::resolved`),
/// the folder its fights are in.
pub fn problems(dungeon: &Dungeon, tables: Option<(&Known, &Book)>, played: &str, fights: &Path) -> Vec<Problem> {
    let Some((offered, book)) = tables else {
        return vec![Problem::new("the game's data is missing (extractor)".to_string(), Place::File, None, true)];
    };
    let mut found = Vec::new();
    let mut say = |said: String, place, fix, grave| found.push(Problem::new(said, place, fix, grave));
    let count = dungeon.floors;
    let fight = !dungeon.fight.is_empty() || dungeon.boss;
    if dungeon.name.trim().is_empty() {
        say("it needs a name".to_string(), Place::Name, None, true);
    }
    if count == 0 && !fight {
        say("0 floors: only with a boss fight at the end".to_string(), Place::Floors, Some(Fix::OneFloor), true);
    }
    // --- Its look, its music, its fight ---
    let spec = look_floor(offered, book, &dungeon.look, dungeon.look_floor);
    match offered.looks.iter().find(|(_, name, _)| plain(name) == plain(&dungeon.look)) {
        _ if spec.is_none() => say(format!("\"{}\" is not a game dungeon to take the look of", dungeon.look), Place::Look, Some(Fix::LookDefault), true),
        Some((_, _, floors)) if dungeon.look_floor > *floors => {
            say(format!("the look has only {floors} floors (floor {} asked)", dungeon.look_floor), Place::Look, Some(Fix::LookFloor(*floors)), false)
        }
        _ => {}
    }
    if !dungeon.music.is_empty() && dungeon.music.parse::<u16>().is_err() && !rules::songs::SONGS.iter().any(|song| plain(song.word) == plain(&dungeon.music)) {
        say(format!("\"{}\" is not a song (its word, or its number)", dungeon.music), Place::Music, Some(Fix::NoMusic), true);
    }
    if !dungeon.fight.is_empty() && !fights.join(format!("{}.fight", dungeon.fight)).exists() {
        say(format!("the fight \"{}\" does not exist (library/fights/{}.fight)", dungeon.fight, dungeon.fight), Place::Fight, Some(Fix::NoFight), true);
    }
    // (What the generator makes of the look: often its one room of when it fails?)
    if let Some(spec) = spec.filter(|_| count > 0) {
        let failed = (1..=16).filter(|seed| generate_floor(&mut DungeonRng::new(*seed), &spec.props).fallback).count();
        if failed >= 4 {
            say(format!("the generator often fails on this look ({failed} seeds out of 16: a single room)"), Place::Look, None, false);
        }
        if spec.props.enemy_density == 0 && !dungeon.foes.is_empty() {
            say("the look brings nobody on arriving (Pokémon density 0): they only come afterwards".to_string(), Place::Look, None, false);
        }
        if spec.props.item_density == 0 && !dungeon.items.is_empty() {
            say("the look puts no item on the ground (item density 0)".to_string(), Place::Look, None, false);
        }
    }
    // --- The Pokémon ---
    let known = |name: &str| offered.species.iter().any(|(_, known)| plain(known) == plain(name));
    if count > 0 && dungeon.foes.is_empty() {
        say("nobody lives there".to_string(), Place::Foes, spec.filter(|spec| !spec.enemies.is_empty()).map(|_| Fix::LookFoes), true);
    }
    for (rank, foe) in dungeon.foes.iter().enumerate() {
        if !known(&foe.species) {
            say(format!("\"{}\" is not a Pokémon the game's dungeons meet", foe.species), Place::Foe(rank), Some(Fix::RemoveFoe(rank)), true);
        }
        if !(1..=100).contains(&foe.level) {
            say(format!("{}: level {}, from 1 to 100", foe.species, foe.level), Place::Foe(rank), Some(Fix::FoeLevel(rank)), false);
        }
        if foe.often == 0 {
            say(format!("{}: often 0, it is never met", foe.species), Place::Foe(rank), Some(Fix::FoeOften(rank)), false);
        }
        if let Some((low, high)) = foe.floors.filter(|(_, high)| *high > count && count > 0) {
            say(format!("{}: floors {low}-{high}, the dungeon has only {count}", foe.species), Place::Foe(rank), Some(Fix::FoeFloors(rank)), false);
        }
    }
    let on = |floors: Floors, at: usize| floors.is_none_or(|(low, high)| low <= at && at <= high);
    for at in 1..=count {
        let here: Vec<&Foe> = dungeon.foes.iter().filter(|foe| on(foe.floors, at)).collect();
        if here.is_empty() && !dungeon.foes.is_empty() {
            let fix = nearest(dungeon.foes.iter().map(|foe| foe.floors), at).map(|rank| Fix::FoeOn(rank, at));
            say(format!("floor {at} has nobody"), Place::Floor(at), fix, true);
        } else if !here.is_empty() && here.iter().all(|foe| foe.often == 0) {
            say(format!("floor {at}: all those met there are at often 0"), Place::Floor(at), None, true);
        }
    }
    // --- The items ---
    for (rank, lying) in dungeon.items.iter().enumerate() {
        if !offered.items.iter().any(|name| plain(name) == plain(&lying.item)) {
            say(format!("\"{}\" is not an item of the game", lying.item), Place::Item(rank), Some(Fix::RemoveItem(rank)), true);
        }
        if lying.often == 0 {
            say(format!("{}: often 0, it is never put down", lying.item), Place::Item(rank), Some(Fix::ItemOften(rank)), false);
        }
        if let Some((low, high)) = lying.floors.filter(|(_, high)| *high > count && count > 0) {
            say(format!("{}: floors {low}-{high}, the dungeon has only {count}", lying.item), Place::Item(rank), Some(Fix::ItemFloors(rank)), false);
        }
    }
    if !dungeon.items.is_empty() {
        for at in 1..=count {
            let here: Vec<&Lying> = dungeon.items.iter().filter(|lying| on(lying.floors, at)).collect();
            if here.iter().all(|lying| lying.often == 0) {
                let fix = nearest(dungeon.items.iter().map(|lying| lying.floors), at).map(|rank| Fix::ItemOn(rank, at));
                say(format!("floor {at} has no item on the ground"), Place::Floor(at), fix.filter(|_| here.is_empty()), false);
            }
        }
    }
    // --- What the rules say besides ---
    if let Err(wrong) = rules::own::read(played, &book.tables) {
        found.extend(said_by_rules(played, wrong).into_iter().map(|told| Problem { told, place: Place::File, fix: None }));
    }
    found
}

/// The problems of a dungeon's file (`<word>.dungeon`), each with the line
/// it is on (from 1): its fights and tilesets that project's (its own, else the shared library's).
pub fn of_file(path: &Path, tables: Option<(&Known, &Book)>, project: &crate::Project) -> Result<Vec<Diagnostic>, crate::Error> {
    let text = std::fs::read_to_string(path).map_err(|why| crate::Error::file(path, why))?.replace("\r\n", "\n");
    let dungeon = Dungeon::read(&text);
    let played = crate::dive::resolved_for(project, &text);
    let fights = project.fight(&dungeon.fight).parent().map_or_else(|| project.fights(), Path::to_path_buf);
    let line_of = |problem: &Problem| {
        let keyed = problem.place.key().and_then(|key| {
            text.lines().position(|line| line.split('#').next().unwrap_or_default().split_once(':').is_some_and(|(said, _)| plain(said) == key))
        });
        // (What the rules say, by its line of the text played: the same line if no fight was read in before it.)
        let said = Some(problem.told.line()).filter(|line| *line > 0);
        keyed.map(|at| at + 1).or(said.filter(|line| *line <= text.lines().count())).unwrap_or(1)
    };
    let at_line = |problem: Problem| {
        let line = line_of(&problem);
        // (The message as the rules said it, “line 3: …”, its line now that of the file.)
        let said = Diagnostic { severity: language::Severity::Error, ..problem.told.clone() }.to_string();
        Diagnostic { message: said, span: language::Location::line(line), ..problem.told }
    };
    Ok(problems(&dungeon, tables, &played, &fights).into_iter().map(at_line).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dungeon_file_is_read_for_its_check() {
        let dungeon = Dungeon::read("name: Forest\nfloors: 2\nlook: Beach Cave 3\npokemon: Caterpie 3 x3 floors 1-2, Pidgey 4\nitems: Apple x3\nfight: boss\n");
        assert_eq!((dungeon.name.as_str(), dungeon.floors, dungeon.look.as_str(), dungeon.look_floor), ("Forest", 2, "Beach Cave", 3));
        assert_eq!(dungeon.foes[0], Foe { species: "Caterpie".to_string(), level: 3, often: 3, floors: Some((1, 2)) });
        assert_eq!(dungeon.items[0], Lying { item: "Apple".to_string(), often: 3, floors: None });
        assert_eq!(dungeon.fight, "boss");
        let missing = problems(&dungeon, None, "", Path::new("."));
        assert!(missing[0].grave() && missing[0].place == Place::File);
    }
}
