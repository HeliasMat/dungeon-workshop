//! What our own eyes say of the elements of the game's places and of the
//! places themselves, written by hand in `library/tables/` (our words: no
//! picture, no text of the game's):
//! - `piece_tags.txt`: each element offered (`e156_3`), its category, a name
//!   one reads (`big oak`), a few tags (`tree, green`);
//! - `place_kinds.txt`: each place (a level of the game, `323`, or one of the
//!   project's decors, `plage_crique`), the kind of place it is.
//!
//! ```text
//! e156_3 | nature | big oak | tree, green
//! 323 | town
//! ```
//!
//! What is sought among them, how they are ranked and counted, by size: all
//! here, no window needed.

use std::collections::BTreeMap;
use std::path::Path;

/// The file of the elements' labels, and of the places' kinds, in a library's tables.
pub const PIECE_TAGS: &str = "piece_tags.txt";
pub const PLACE_KINDS: &str = "place_kinds.txt";

/// What an element is, as the library sorts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Nature,
    Water,
    Buildings,
    Furniture,
    Animated,
    SceneObjects,
    Other,
}

/// Each category: its word in the file, its name shown.
pub const CATEGORIES: [(Category, &str, &str); 7] = [
    (Category::Nature, "nature", "Nature"),
    (Category::Water, "water", "Water"),
    (Category::Buildings, "buildings", "Buildings"),
    (Category::Furniture, "furniture", "Furniture & props"),
    (Category::Animated, "animated", "Animated"),
    (Category::SceneObjects, "scene", "Scene objects"),
    (Category::Other, "other", "Other"),
];

impl Category {
    pub fn word(self) -> &'static str {
        CATEGORIES.iter().find(|(known, ..)| *known == self).map_or("other", |(_, word, _)| word)
    }

    pub fn shown(self) -> &'static str {
        CATEGORIES.iter().find(|(known, ..)| *known == self).map_or("Other", |(.., shown)| shown)
    }

    pub fn of_word(word: &str) -> Option<Category> {
        CATEGORIES.iter().find(|(_, known, _)| *known == word).map(|(category, ..)| *category)
    }

    /// Its place in `CATEGORIES`.
    pub fn index(self) -> usize {
        CATEGORIES.iter().position(|(known, ..)| *known == self).unwrap_or(CATEGORIES.len() - 1)
    }
}

/// An element as our eyes saw it.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub category: Category,
    pub name: String,
    pub tags: Vec<String>,
}

/// The category an element goes in: animated tiles (`a…`) and what the game
/// animates are Animated, the scenes' things (`o…`) Scene objects, whatever
/// its label says; the others as labelled, else Other.
pub fn category_of(name: &str, moves: bool, label: Option<&Label>) -> Category {
    match name.chars().next() {
        Some('a') => Category::Animated,
        Some('o') => Category::SceneObjects,
        _ if moves => Category::Animated,
        _ => label.map_or(Category::Other, |label| label.category),
    }
}

/// The lines of a file of ours, each cut at `|`, without its comments and blank lines: (line number, its fields).
fn fields(text: &str) -> impl Iterator<Item = (usize, Vec<&str>)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|(at, line)| (at + 1, line.split('|').map(str::trim).collect()))
}

/// The labels of a `piece_tags.txt`, by element; what is wrong, by line.
pub fn read_labels(text: &str) -> Result<BTreeMap<String, Label>, String> {
    let mut labels = BTreeMap::new();
    for (line, fields) in fields(text) {
        let [id, category, name, rest @ ..] = fields.as_slice() else {
            return Err(format!("{PIECE_TAGS}:{line}: id | category | name | tags"));
        };
        let category = Category::of_word(category).ok_or_else(|| format!("{PIECE_TAGS}:{line}: no category \"{category}\""))?;
        let tags = rest.first().map_or_else(Vec::new, |tags| tags.split(',').map(str::trim).filter(|tag| !tag.is_empty()).map(str::to_string).collect());
        if labels.insert(id.to_string(), Label { category, name: name.to_string(), tags }).is_some() {
            return Err(format!("{PIECE_TAGS}:{line}: \"{id}\" a second time"));
        }
    }
    Ok(labels)
}

/// An element's label: its own line, else (set aside as almost like
/// another: the same tent at another time of day) that one's.
pub fn label_of<'a>(name: &str, labels: &'a BTreeMap<String, Label>, aside: &std::collections::HashMap<String, String>) -> Option<&'a Label> {
    labels.get(name).or_else(|| aside.get(name).and_then(|why| why.strip_prefix("almost like ")).and_then(|twin| labels.get(twin.trim())))
}

/// The labels of a library's tables (none if it has no file, or a wrong one).
pub fn labels(tables: &Path) -> BTreeMap<String, Label> {
    read_labels(&std::fs::read_to_string(tables.join(PIECE_TAGS)).unwrap_or_default()).unwrap_or_default()
}

/// How large an element is, as the library filters them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    /// Its larger side: up to 32 pixels small (a flower, a pot), up to 96 medium (a bush, a stall), else large (a house).
    pub fn of([w, h]: [f32; 2]) -> Size {
        match w.max(h) {
            side if side <= 32.0 => Size::Small,
            side if side <= 96.0 => Size::Medium,
            _ => Size::Large,
        }
    }

    pub fn shown(self) -> &'static str {
        match self {
            Size::Small => "small",
            Size::Medium => "medium",
            Size::Large => "large",
        }
    }
}

/// How well an element answers what is sought (lower is better), none if
/// not at all: each word sought must be found in its name, its tags, its
/// place or its id; a word that is the name's start, a whole tag or a whole
/// word of its name ranks it first.
pub fn rank(sought: &str, name: &str, tags: &[String], place: &str, id: &str) -> Option<u32> {
    let (name, place) = (name.to_lowercase(), place.to_lowercase());
    let mut total = 0;
    for word in sought.to_lowercase().split_whitespace() {
        let tags_have = |whole: bool| tags.iter().any(|tag| if whole { tag.to_lowercase() == word } else { tag.to_lowercase().starts_with(word) });
        let score = if name == word {
            0
        } else if name.starts_with(word) {
            1
        } else if name.split(|letter: char| !letter.is_alphanumeric()).any(|part| part == word) || tags_have(true) {
            2
        } else if name.split(|letter: char| !letter.is_alphanumeric()).any(|part| part.starts_with(word)) || tags_have(false) {
            3
        } else if name.contains(word) {
            4
        } else if place.contains(word) {
            5
        } else if id.starts_with(word) {
            6
        } else {
            return None;
        };
        total += score;
    }
    Some(total)
}

/// How many of each category, in the order of `CATEGORIES`.
pub fn counts(categories: impl IntoIterator<Item = Category>) -> [usize; 7] {
    let mut counts = [0; 7];
    for category in categories {
        counts[category.index()] += 1;
    }
    counts
}

/// A kind of place, as the lists of decors group them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PlaceKind {
    Town,
    Beach,
    Forest,
    Cave,
    Guild,
    Dungeon,
    Other,
}

/// Each kind of place: its word in the file, its name shown.
pub const PLACE_KINDS_SHOWN: [(PlaceKind, &str, &str); 7] = [
    (PlaceKind::Town, "town", "Town"),
    (PlaceKind::Beach, "beach", "Beach & sea"),
    (PlaceKind::Forest, "forest", "Forest & grove"),
    (PlaceKind::Cave, "cave", "Cave & mountain"),
    (PlaceKind::Guild, "guild", "Guild & interiors"),
    (PlaceKind::Dungeon, "dungeon", "Dungeon entrances"),
    (PlaceKind::Other, "other", "Other"),
];

impl PlaceKind {
    pub fn shown(self) -> &'static str {
        PLACE_KINDS_SHOWN.iter().find(|(known, ..)| *known == self).map_or("Other", |(.., shown)| shown)
    }

    pub fn of_word(word: &str) -> Option<PlaceKind> {
        PLACE_KINDS_SHOWN.iter().find(|(_, known, _)| *known == word).map(|(kind, ..)| *kind)
    }
}

/// The kinds of places of a `place_kinds.txt`: by a level of the game
/// (`323`) or a decor's name (`plage_crique`); and a name of ours for the
/// places the game leaves unnamed (`268 | guild | a den inside the bluff`).
#[derive(Clone, Debug, Default)]
pub struct PlaceKinds {
    pub by_key: BTreeMap<String, PlaceKind>,
    pub names: BTreeMap<String, String>,
}

impl PlaceKinds {
    pub fn read(text: &str) -> Result<PlaceKinds, String> {
        let (mut by_key, mut names) = (BTreeMap::new(), BTreeMap::new());
        for (line, fields) in fields(text) {
            let [key, kind, rest @ ..] = fields.as_slice() else {
                return Err(format!("{PLACE_KINDS}:{line}: place | kind"));
            };
            let kind = PlaceKind::of_word(kind).ok_or_else(|| format!("{PLACE_KINDS}:{line}: no kind \"{kind}\""))?;
            if by_key.insert(key.to_string(), kind).is_some() {
                return Err(format!("{PLACE_KINDS}:{line}: \"{key}\" a second time"));
            }
            if let Some(name) = rest.first().filter(|name| !name.is_empty()) {
                names.insert(key.to_string(), name.to_string());
            }
        }
        Ok(PlaceKinds { by_key, names })
    }

    /// Those of a library's tables (none if it has no file, or a wrong one).
    pub fn of(tables: &Path) -> PlaceKinds {
        PlaceKinds::read(&std::fs::read_to_string(tables.join(PLACE_KINDS)).unwrap_or_default()).unwrap_or_default()
    }

    /// A place of the game's kind, by its level.
    pub fn of_level(&self, level: i64) -> PlaceKind {
        self.by_key.get(&level.to_string()).copied().unwrap_or(PlaceKind::Other)
    }

    /// A decor's kind: its own line, else the place of the game it is built on (`level`), else Other.
    pub fn of_decor(&self, name: &str, level: Option<i64>) -> PlaceKind {
        self.by_key.get(name).copied().or_else(|| level.and_then(|level| self.by_key.get(&level.to_string()).copied())).unwrap_or(PlaceKind::Other)
    }
}

/// Things grouped by kind of place, in the order of the kinds, each group in the order given; empty groups left out.
pub fn grouped<T>(things: Vec<T>, kind: impl Fn(&T) -> PlaceKind) -> Vec<(PlaceKind, Vec<T>)> {
    let mut groups: Vec<(PlaceKind, Vec<T>)> = PLACE_KINDS_SHOWN.iter().map(|(kind, ..)| (*kind, Vec::new())).collect();
    for thing in things {
        let at = PLACE_KINDS_SHOWN.iter().position(|(known, ..)| *known == kind(&thing)).unwrap_or(PLACE_KINDS_SHOWN.len() - 1);
        groups[at].1.push(thing);
    }
    groups.retain(|(_, group)| !group.is_empty());
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_read_and_say_what_is_wrong() {
        let read = read_labels("# ours\ne1_1 | nature | big oak | tree, green\n\ne1_2 | furniture | barrel\n").expect("read");
        assert_eq!(read["e1_1"], Label { category: Category::Nature, name: "big oak".to_string(), tags: vec!["tree".to_string(), "green".to_string()] });
        assert!(read["e1_2"].tags.is_empty());
        assert!(read_labels("e1_1 | trees | oak\n").unwrap_err().contains("no category"));
        assert!(read_labels("e1_1 | nature\n").is_err());
        assert!(read_labels("e1_1 | nature | a\ne1_1 | nature | b\n").unwrap_err().contains("second time"));
        let aside: std::collections::HashMap<String, String> = [("e9_1".to_string(), "almost like e1_1".to_string())].into_iter().collect();
        assert_eq!(label_of("e9_1", &read, &aside).map(|label| label.name.as_str()), Some("big oak"));
        assert_eq!(label_of("e9_2", &read, &aside), None);
    }

    #[test]
    fn some_elements_classify_themselves() {
        let label = Label { category: Category::Buildings, name: "mill".to_string(), tags: Vec::new() };
        assert_eq!(category_of("a3_1", false, None), Category::Animated);
        assert_eq!(category_of("o186", false, Some(&label)), Category::SceneObjects);
        assert_eq!(category_of("e3_1", true, Some(&label)), Category::Animated);
        assert_eq!(category_of("e3_2", false, Some(&label)), Category::Buildings);
        assert_eq!(category_of("e3_3", false, None), Category::Other);
        assert_eq!(counts([Category::Nature, Category::Other, Category::Nature]), [2, 0, 0, 0, 0, 0, 1]);
        assert_eq!((Size::of([16.0, 32.0]), Size::of([40.0, 20.0]), Size::of([97.0, 8.0])), (Size::Small, Size::Medium, Size::Large));
    }

    #[test]
    fn the_search_ranks_the_name_first() {
        let tags = vec!["tree".to_string(), "green".to_string()];
        let rank_of = |sought| rank(sought, "big oak", &tags, "Treasure Town", "e12_3");
        assert_eq!(rank_of("big oak"), Some(1 + 2));
        assert_eq!(rank_of("big"), Some(1));
        assert_eq!(rank_of("oak"), Some(2));
        assert_eq!(rank_of("tree"), Some(2));
        assert_eq!(rank_of("gre"), Some(3));
        assert_eq!(rank_of("treasure"), Some(5));
        assert_eq!(rank_of("e12"), Some(6));
        assert_eq!(rank_of("barrel"), None);
        assert_eq!(rank_of("oak barrel"), None);
        assert_eq!(rank_of(""), Some(0));
    }

    #[test]
    fn places_are_grouped_by_kind() {
        let kinds = PlaceKinds::read("323 | town\nplage_crique | beach\n# a note\n268 | guild | a den\n").expect("read");
        assert_eq!(kinds.names.get("268").map(String::as_str), Some("a den"));
        assert!(!kinds.names.contains_key("323"));
        assert_eq!(kinds.of_level(323), PlaceKind::Town);
        assert_eq!(kinds.of_level(5), PlaceKind::Other);
        assert_eq!(kinds.of_decor("plage_crique", None), PlaceKind::Beach);
        assert_eq!(kinds.of_decor("mon_bourg", Some(323)), PlaceKind::Town);
        assert!(PlaceKinds::read("1 | moon\n").is_err());
        let groups = grouped(vec![("a", PlaceKind::Other), ("b", PlaceKind::Town), ("c", PlaceKind::Town)], |thing| thing.1);
        let said: Vec<(PlaceKind, Vec<&str>)> = groups.into_iter().map(|(kind, group)| (kind, group.into_iter().map(|thing| thing.0).collect())).collect();
        assert_eq!(said, [(PlaceKind::Town, vec!["b", "c"]), (PlaceKind::Other, vec!["a"])]);
    }
}
