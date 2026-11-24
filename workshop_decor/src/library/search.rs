//! What the Library shows, decided without drawing: a card's name and the
//! place it comes from, the game's places counted, the cards of a source
//! (of the sizes asked, those that answer what is sought, the best first),
//! and the search across all kinds at once: elements, backdrops, models,
//! the project's decors, each kind ranked on its own.

use decor::tags::{PlaceKind, Size};

use super::Source;
use crate::workshop::{BackdropInfo, PieceInfo};

/// The name a card shows: as our eyes saw it, else what it is and its number.
pub fn title_of(piece: &PieceInfo) -> String {
    if let Some(label) = &piece.label {
        return label.clone();
    }
    let number = piece.name.rsplit('_').next().unwrap_or_default().trim_start_matches('o');
    match piece.kind {
        'a' => format!("animated tiles {number}"),
        'o' => format!("scene object {number}"),
        _ => format!("element {number}"),
    }
}

/// Where a card says it comes from: its place, else what it is.
pub fn from_said(piece: &PieceInfo) -> String {
    match &piece.place {
        Some((_, place)) if piece.also.is_empty() => place.clone(),
        Some((_, place)) => format!("{place} +{}", piece.also.len()),
        None => "scenes".to_string(),
    }
}

/// A place of the game the pieces come from: its number, its name, its kind, how many pieces.
pub type Place = (i64, String, PlaceKind, usize);

/// The places of the game the pieces come from, by name.
pub fn places(pieces: &[PieceInfo]) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();
    for piece in pieces {
        if let Some((level, name)) = &piece.place {
            match out.iter_mut().find(|(known, ..)| known == level) {
                Some(place) => place.3 += 1,
                None => out.push((*level, name.clone(), piece.place_kind, 1)),
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    out
}

/// What the cards shown are chosen by.
pub struct Filter<'a> {
    pub source: Source,
    pub sought: &'a str,
    pub sizes: [bool; 3],
    pub favourites: &'a [String],
    pub recent: &'a [String],
}

/// Which of the three sizes a piece is (small, medium, large).
fn size_index(size: [f32; 2]) -> usize {
    match Size::of(size) {
        Size::Small => 0,
        Size::Medium => 1,
        Size::Large => 2,
    }
}

/// The cards shown: of the source, of the sizes asked; with something sought,
/// those that answer it, the best first (`decor::tags::rank`: name, tags,
/// places, id).
pub fn shown_of(pieces: &[PieceInfo], filter: &Filter) -> Vec<PieceInfo> {
    let by_names = |names: &[String]| -> Vec<PieceInfo> {
        names.iter().filter_map(|name| pieces.iter().find(|piece| piece.name == *name || piece.twins.contains(name)).cloned()).collect()
    };
    let chosen: Vec<PieceInfo> = match filter.source {
        Source::Favourites => by_names(filter.favourites),
        Source::Recent => by_names(filter.recent),
        Source::All => pieces.to_vec(),
        Source::Category(category) => pieces.iter().filter(|piece| piece.category == category).cloned().collect(),
        Source::Place(level) => pieces.iter().filter(|piece| piece.place.as_ref().is_some_and(|(known, _)| *known == level)).cloned().collect(),
        Source::Models | Source::Backdrops => Vec::new(),
    };
    let mut ranked: Vec<(u32, PieceInfo)> = chosen
        .into_iter()
        .filter(|piece| filter.sizes[size_index(piece.size)])
        .filter_map(|piece| {
            let places = piece.place.iter().map(|(_, name)| name.as_str()).chain(piece.also.iter().map(String::as_str)).collect::<Vec<_>>().join(" ");
            decor::tags::rank(filter.sought, &title_of(&piece), &piece.tags, &places, &piece.name).map(|rank| (rank, piece))
        })
        .collect();
    // (Stable: the same rank keeps the source's order.)
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, piece)| piece).collect()
}

/// The backdrops shown: those that answer what is sought (by the place's
/// name, its kind, its number), the best first; all, by name, if nothing is.
pub fn backdrops_of(backdrops: &[BackdropInfo], sought: &str) -> Vec<BackdropInfo> {
    let mut ranked: Vec<(u32, &BackdropInfo)> = backdrops
        .iter()
        .filter_map(|backdrop| {
            let kind = [backdrop.kind.shown().to_string()];
            decor::tags::rank(sought, &backdrop.place, &kind, "", &backdrop.level.to_string()).map(|rank| (rank, backdrop))
        })
        .collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, backdrop)| backdrop.clone()).collect()
}

/// What a search found, kind by kind, each the best first: the elements,
/// the backdrops, the models (their place in the list given), the decors (theirs).
#[derive(Debug, Default)]
pub struct Found {
    pub elements: Vec<PieceInfo>,
    pub backdrops: Vec<BackdropInfo>,
    pub models: Vec<usize>,
    pub decors: Vec<usize>,
}

impl Found {
    /// How many it found in all.
    pub fn count(&self) -> usize {
        self.elements.len() + self.backdrops.len() + self.models.len() + self.decors.len()
    }
}

/// Those of a list that answer what is sought, by their name (and their file's), the best first: their places in it.
fn ranked_names<'a>(sought: &str, named: impl Iterator<Item = (&'a str, &'a str)>) -> Vec<usize> {
    let mut ranked: Vec<(u32, usize)> =
        named.enumerate().filter_map(|(at, (name, file))| decor::tags::rank(sought, name, &[], "", file).map(|rank| (rank, at))).collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, at)| at).collect()
}

/// Everything that answers what is sought, of every kind: the elements (of
/// the sizes asked, wherever they are), the backdrops, the models, the decors
/// (each its title and its file's name).
pub fn search_all(
    sought: &str,
    pieces: &[PieceInfo],
    sizes: [bool; 3],
    backdrops: &[BackdropInfo],
    models: &[(String, Vec<String>)],
    decors: &[(String, String)],
) -> Found {
    if sought.trim().is_empty() {
        return Found::default();
    }
    let all = Filter { source: Source::All, sought, sizes, favourites: &[], recent: &[] };
    Found {
        elements: shown_of(pieces, &all),
        backdrops: backdrops_of(backdrops, sought),
        models: ranked_names(sought, models.iter().map(|(name, _)| (name.as_str(), ""))),
        decors: ranked_names(sought, decors.iter().map(|(title, file)| (title.as_str(), file.as_str()))),
    }
}

/// How many are shown, of how many.
pub fn many_said(total: usize, shown: usize) -> String {
    if total > shown { format!("{shown} of {total} (search to see the others)") } else { format!("{total} element(s)") }
}

#[cfg(test)]
mod tests {
    use super::{Filter, backdrops_of, from_said, many_said, places, search_all, shown_of, title_of};
    use crate::library::{Source, starred, where_said};
    use crate::workshop::{BackdropInfo, PieceInfo};
    use decor::tags::{Category, PlaceKind};

    fn piece(name: &str, category: Category, label: Option<&str>, tags: &[&str], place: Option<(i64, &str)>, size: [f32; 2]) -> PieceInfo {
        PieceInfo {
            name: name.to_string(),
            place: place.map(|(level, name)| (level, name.to_string())),
            place_kind: PlaceKind::Town,
            kind: name.chars().next().unwrap_or('e'),
            size,
            moves: false,
            twins: Vec::new(),
            also: Vec::new(),
            category,
            label: label.map(str::to_string),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
        }
    }

    fn pieces() -> Vec<PieceInfo> {
        let mut oak = piece("e1_1", Category::Nature, Some("big oak"), &["tree", "green"], Some((323, "Treasure Town")), [96.0, 120.0]);
        oak.twins = vec!["e9_4".to_string()];
        oak.also = vec!["Beach".to_string()];
        vec![
            oak,
            piece("e1_2", Category::Furniture, Some("barrel"), &["wood"], Some((323, "Treasure Town")), [16.0, 24.0]),
            piece("e2_1", Category::Nature, Some("oak sapling"), &["tree"], Some((11, "Beach")), [40.0, 48.0]),
            piece("o186", Category::SceneObjects, None, &[], None, [24.0, 24.0]),
        ]
    }

    fn backdrops() -> Vec<BackdropInfo> {
        vec![
            BackdropInfo { level: 11, place: "Beach".to_string(), kind: PlaceKind::Beach, size: [512.0, 384.0] },
            BackdropInfo { level: 323, place: "Treasure Town".to_string(), kind: PlaceKind::Town, size: [1024.0, 512.0] },
            BackdropInfo { level: 52, place: "Oak Grove".to_string(), kind: PlaceKind::Forest, size: [256.0, 192.0] },
        ]
    }

    fn names(shown: &[PieceInfo]) -> Vec<&str> {
        shown.iter().map(|piece| piece.name.as_str()).collect()
    }

    /// The cards of a source, of the sizes asked, those that answer what is sought, the best first.
    #[test]
    fn the_library_filters_and_ranks() {
        let pieces = pieces();
        let (favourites, recent) = (vec!["e9_4".to_string(), "e1_2".to_string()], vec!["o186".to_string()]);
        let filter = |source, sought, sizes| shown_of(&pieces, &Filter { source, sought, sizes, favourites: &favourites, recent: &recent });
        assert_eq!(names(&filter(Source::All, "", [true; 3])), ["e1_1", "e1_2", "e2_1", "o186"]);
        assert_eq!(names(&filter(Source::Category(Category::Nature), "", [true; 3])), ["e1_1", "e2_1"]);
        assert_eq!(names(&filter(Source::Place(323), "", [true; 3])), ["e1_1", "e1_2"]);
        // (A twin's name, among the favourites, is its element.)
        assert_eq!(names(&filter(Source::Favourites, "", [true; 3])), ["e1_1", "e1_2"]);
        assert_eq!(names(&filter(Source::Recent, "", [true; 3])), ["o186"]);
        assert!(filter(Source::Backdrops, "", [true; 3]).is_empty());
        assert_eq!(names(&filter(Source::All, "", [true, false, false])), ["e1_2", "o186"]);
        assert_eq!(names(&filter(Source::All, "", [false, false, true])), ["e1_1"]);
        // (Sought: the name's start first, then a whole word, a tag, a place.)
        assert_eq!(names(&filter(Source::All, "oak", [true; 3])), ["e2_1", "e1_1"]);
        assert_eq!(names(&filter(Source::All, "tree", [true; 3])), ["e1_1", "e2_1"]);
        assert_eq!(names(&filter(Source::All, "beach", [true; 3])), ["e1_1", "e2_1"]);
        assert_eq!(names(&filter(Source::All, "wood barrel", [true; 3])), ["e1_2"]);
        assert!(filter(Source::All, "castle", [true; 3]).is_empty());
    }

    /// One search, every kind: elements wherever they are, backdrops by place or kind, models, decors by title or file; each the best first.
    #[test]
    fn a_search_finds_every_kind_each_ranked() {
        let (pieces, backdrops) = (pieces(), backdrops());
        let models = vec![("oak row".to_string(), vec!["put e1_1 0 0".to_string()]), ("stall".to_string(), Vec::new())];
        let decors = vec![("The Oak Square".to_string(), "place_des_chenes".to_string()), ("Harbour".to_string(), "port".to_string())];
        let found = search_all("oak", &pieces, [true; 3], &backdrops, &models, &decors);
        assert_eq!(names(&found.elements), ["e2_1", "e1_1"]);
        assert_eq!(found.backdrops.iter().map(|one| one.level).collect::<Vec<_>>(), [52]);
        assert_eq!(found.models, [0]);
        assert_eq!(found.decors, [0]);
        assert_eq!(found.count(), 5);
        // (A backdrop by its kind or its number; a decor by its file.)
        assert_eq!(search_all("beach", &pieces, [true; 3], &backdrops, &models, &decors).backdrops.iter().map(|one| one.level).collect::<Vec<_>>(), [11]);
        assert_eq!(search_all("town", &pieces, [true; 3], &backdrops, &models, &decors).backdrops.iter().map(|one| one.level).collect::<Vec<_>>(), [323]);
        assert_eq!(search_all("port", &pieces, [true; 3], &backdrops, &models, &decors).decors, [1]);
        assert_eq!(backdrops_of(&backdrops, "32").iter().map(|one| one.level).collect::<Vec<_>>(), [323]);
        // (The sizes asked hold for the elements; nothing sought finds nothing.)
        assert!(search_all("oak", &pieces, [true, false, false], &backdrops, &models, &decors).elements.is_empty());
        assert_eq!(search_all("  ", &pieces, [true; 3], &backdrops, &models, &decors).count(), 0);
        assert_eq!(backdrops_of(&backdrops, "").len(), 3);
    }

    /// What a card says, the places counted, where one is.
    #[test]
    fn the_library_says_what_and_where() {
        let pieces = pieces();
        assert_eq!((title_of(&pieces[0]), from_said(&pieces[0])), ("big oak".to_string(), "Treasure Town +1".to_string()));
        assert_eq!((title_of(&pieces[3]), from_said(&pieces[3])), ("scene object 186".to_string(), "scenes".to_string()));
        assert_eq!(from_said(&pieces[2]), "Beach");
        let places = places(&pieces);
        assert_eq!(
            places.iter().map(|(level, name, _, count)| (*level, name.as_str(), *count)).collect::<Vec<_>>(),
            [(11, "Beach", 1), (323, "Treasure Town", 2)]
        );
        assert_eq!(where_said(Source::Place(323), &places), "The Game's Places › Town › Treasure Town");
        assert_eq!(where_said(Source::Place(9), &places), "The Game's Places › place 9");
        assert_eq!(where_said(Source::Category(Category::Furniture), &places), "Furniture & props");
        assert_eq!(where_said(Source::Recent, &places), "Recently Laid");
        assert_eq!(where_said(Source::Backdrops, &places), "Backdrops");
        assert_eq!(decor::tags::counts(pieces.iter().map(|piece| piece.category)), [2, 0, 0, 1, 0, 1, 0]);
        assert_eq!(many_said(700, 600), "600 of 700 (search to see the others)");
        assert_eq!(many_said(3, 3), "3 element(s)");
        let mut favourites = vec!["a".to_string()];
        starred(&mut favourites, "b".to_string());
        starred(&mut favourites, "a".to_string());
        assert_eq!(favourites, vec!["b".to_string()]);
    }
}
