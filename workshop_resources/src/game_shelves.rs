//! The shelves of the game's own things, sorted as a film looks for them:
//! the attacks (each move and the effects it shows), the statuses, the
//! weathers, the other effects; the portraits, by Pokémon and face; the
//! items; the backdrops and the tilesets.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::pictures::{effects, face_name};
use super::{Item, Shelf};
use film::words::plain;
use workshop_base::session::Session;

/// What each of a move's four effects is (`rules::tables::Tables::move_effects`).
pub(super) const EFFECT_ROLES: [&str; 4] = ["before it", "as it strikes", "on the target", "what flies"];

/// Which of a move's effects stands for it on a card: on the target, as it strikes, what flies, before it.
const CARD_ROLES: [usize; 4] = [2, 1, 3, 0];

/// The weathers a film's sky has (`sky <word>`, `film_view/src/sky.rs`): the word, its name, what it is.
pub(super) const WEATHERS: [(&str, &str, &str); 7] = [
    ("rain", "Rain", "rain falls · weather rain heavy: under a storm's light"),
    ("sun", "Sun", "the sun shines hard"),
    ("hail", "Hail", "hail falls"),
    ("snow", "Snow", "snow falls"),
    ("sand", "Sandstorm", "a sandstorm blows"),
    ("fog", "Fog", "a mist lies"),
    ("off", "Clear", "the fair weather back"),
];

/// The game's tables (`dungeons.bin`), if they are there.
fn book(session: &Session) -> Option<std::sync::Arc<workshop_dungeon::Offered>> {
    workshop_dungeon::offered(session)
}

/// The effects the game has (`effects.txt`), by their numbers.
fn effect_ids(session: &Session) -> BTreeSet<u16> {
    effects(session).iter().map(|effect| effect.id).collect()
}

/// A move's effects, by their roles (`None`: it has none there, or the game draws none).
pub(super) fn move_effects(session: &Session, move_id: i64) -> [Option<u16>; 4] {
    let known = effect_ids(session);
    let row = book(session).and_then(|offered| offered.book.tables.move_effects.get(move_id as usize).copied()).unwrap_or_default();
    row.map(|id| (id != 0 && known.contains(&id)).then_some(id))
}

/// The effect a move's card shows.
pub(super) fn card_effect(session: &Session, move_id: i64) -> Option<u16> {
    let effects = move_effects(session, move_id);
    CARD_ROLES.iter().find_map(|role| effects[*role])
}

/// Each move a film can name (`effect <Move> on <someone>`), by the number
/// the film takes for it; those that show some effect.
pub(super) fn attacks(session: &Session) -> Vec<Item> {
    let tables = session.tables();
    let known = effect_ids(session);
    let Some(offered) = book(session) else { return Vec::new() };
    let rows = &offered.book.tables.move_effects;
    tables
        .moves
        .values()
        .filter_map(|(id, name)| {
            let id: i64 = id.parse().ok()?;
            let row = rows.get(id as usize)?;
            let shown: Vec<String> = row
                .iter()
                .zip(EFFECT_ROLES)
                .filter(|(effect, _)| **effect != 0 && known.contains(effect))
                .map(|(effect, role)| format!("{role}: {effect}"))
                .collect();
            if shown.is_empty() || name.is_empty() || name.starts_with('[') {
                return None;
            }
            let about = format!("move No. {id} · effect{} {}", if shown.len() > 1 { "s" } else { "" }, shown.join(", "));
            Some(Item { shelf: Shelf::Attacks, word: plain(name), label: name.clone(), about, number: id, file: None, order: id })
        })
        .collect()
}

/// The status icons the game draws (`status.txt`), as a film names them (`status <someone> <name>`).
pub(super) fn statuses(session: &Session) -> Vec<Item> {
    assets::tables::statuses(session.data())
        .into_iter()
        .enumerate()
        .filter(|(_, status)| film::emit::status_icon(&status.name).is_some())
        .map(|(at, status)| {
            let about = format!("{} image{} · {}×{}", status.frames, if status.frames > 1 { "s" } else { "" }, status.wide, status.high);
            let label = status.name.replace('_', " ");
            Item { shelf: Shelf::Statuses, word: status.name, label, about, number: at as i64, file: None, order: at as i64 }
        })
        .collect()
}

/// The skies a film has.
pub(super) fn weathers() -> Vec<Item> {
    WEATHERS
        .iter()
        .enumerate()
        .map(|(at, (word, label, about))| Item {
            shelf: Shelf::Weather,
            word: (*word).to_string(),
            label: (*label).to_string(),
            about: (*about).to_string(),
            number: at as i64,
            file: None,
            order: at as i64,
        })
        .collect()
}

/// The effects no move shows: those of the traps, the items, the scenes (`effect <someone> <number>`).
pub(super) fn other_effects(session: &Session) -> Vec<Item> {
    let offered = book(session);
    let tables = offered.as_ref().map(|offered| &offered.book.tables);
    let by_moves: BTreeSet<u16> = tables.map(|tables| tables.move_effects.iter().flatten().copied().collect()).unwrap_or_default();
    let mut uses: HashMap<u16, String> = HashMap::new();
    if let Some(tables) = tables {
        for effect in tables.trap_effects.iter().filter(|effect| **effect != 0) {
            uses.entry(*effect).or_insert_with(|| "a trap's".to_string());
        }
        for (item, effects) in tables.item_effects.iter().enumerate() {
            let name = tables.items.get(item).filter(|name| !name.is_empty() && !name.starts_with('['));
            for effect in effects.iter().filter(|effect| **effect != 0) {
                if let Some(name) = name {
                    uses.entry(*effect).or_insert_with(|| format!("the item {name}'s"));
                }
            }
        }
    }
    effects(session)
        .iter()
        .filter(|effect| !by_moves.contains(&effect.id))
        .map(|effect| {
            let images = effect.durations.len();
            let used = uses.get(&effect.id).map_or(String::new(), |used| format!("{used} · "));
            let about = format!("{used}{images} image{} · {}×{}", if images > 1 { "s" } else { "" }, effect.wide, effect.high);
            let id = i64::from(effect.id);
            Item { shelf: Shelf::Effects, word: effect.id.to_string(), label: format!("Effect {}", effect.id), about, number: id, file: None, order: id }
        })
        .collect()
}

/// The faces each Pokémon has a portrait of (`portraits/<number>_<face>.png`), by its number.
fn portrait_files(session: &Session) -> BTreeMap<i64, BTreeSet<u16>> {
    let mut faces: BTreeMap<i64, BTreeSet<u16>> = BTreeMap::new();
    let folder = assets::data::path(session.data(), "portraits");
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((number, face)) = name.strip_suffix(".png").and_then(|stem| stem.split_once('_')) else { continue };
        if let (Ok(number), Ok(face)) = (number.parse(), face.parse()) {
            faces.entry(number).or_default().insert(face);
        }
    }
    faces
}

/// A portrait's word: who speaks, and the face a film names (`pikachu (happy)`).
pub(super) fn portrait_word(who: &str, face: u16) -> String {
    format!("{who} ({})", face_name(face))
}

/// A portrait's face, of its word.
pub(super) fn portrait_face(word: &str) -> Option<u16> {
    let face = word.rsplit_once(" (")?.1.strip_suffix(')')?;
    scenes::vocabulary::POKEMON.faces.iter().find(|(said, _)| *said == face).map(|(_, number)| *number)
}

/// Each Pokémon's faces (and its variants'), each its own card; those a film can name.
pub(super) fn portraits(session: &Session, people: &[Item]) -> Vec<Item> {
    let files = portrait_files(session);
    let named: BTreeSet<u16> = scenes::vocabulary::POKEMON.faces.iter().map(|(_, number)| *number).collect();
    let order = super::pictures::face_order();
    let rank = |face: u16| order.iter().position(|known| *known == face).unwrap_or(99) as i64;
    people
        .iter()
        .flat_map(|person| {
            let mut faces: Vec<u16> = files.get(&person.number).cloned().unwrap_or_default().into_iter().collect();
            faces.sort_by_key(|face| rank(*face));
            faces.into_iter().filter(|face| named.contains(face)).map(move |face| Item {
                shelf: Shelf::Portraits,
                word: portrait_word(&person.word, face),
                label: format!("{} · {}", person.label, face_name(face).replace('_', " ")),
                about: format!("{}'s portrait, face No. {face}", person.label),
                number: person.number,
                file: None,
                order: person.order * 100 + rank(face),
            })
        })
        .collect()
}

/// The game's items a film can name (`hold <someone> <Item>`), by their numbers.
pub(super) fn items(session: &Session) -> Vec<Item> {
    let tables = session.tables();
    let offered = book(session);
    let prices = offered.as_ref().map(|offered| offered.book.tables.item_prices.clone()).unwrap_or_default();
    tables
        .items
        .values()
        .filter_map(|(id, name)| {
            let id: i64 = id.parse().ok()?;
            if id == 0 || name.trim().is_empty() || name.starts_with('[') || !name.chars().any(char::is_alphabetic) {
                return None;
            }
            let price = prices.get(id as usize).filter(|(buy, _)| *buy > 0).map_or(String::new(), |(buy, sell)| format!(" · {buy} P (sold {sell} P)"));
            Some(Item {
                shelf: Shelf::Items,
                word: plain(name),
                label: name.clone(),
                about: format!("item No. {id}{price}"),
                number: id,
                file: None,
                order: id,
            })
        })
        .collect()
}

/// An item's icon: 16 by 16, as the game draws it on a floor.
pub(super) fn item_icon(session: &Session, id: i64) -> Option<Vec<u8>> {
    let offered = book(session)?;
    offered.book.tables.item_art.get(id as usize).filter(|rgba| rgba.len() == 16 * 16 * 4).cloned()
}

/// The word a backdrop takes in a film (`scene: <word>`), and its name: ours,
/// a place of the game, a place `decor` made, a picture.
fn backdrop_words(session: &Session) -> HashMap<u16, (String, String)> {
    let tables = session.tables();
    let mut words: HashMap<u16, (String, String)> = HashMap::new();
    let pretty = |word: &str| {
        let spaced = word.replace('_', " ");
        let mut letters = spaced.chars();
        letters.next().map(|first| first.to_uppercase().chain(letters).collect()).unwrap_or_default()
    };
    for (word, (level, ..)) in film::tables::PLACES {
        words.entry(level).or_insert_with(|| (word.to_string(), pretty(word)));
    }
    for (word, level) in &tables.levels {
        let name = if level.name.trim().is_empty() { pretty(word) } else { level.name.clone() };
        words.entry(level.level).or_insert_with(|| (word.clone(), name));
    }
    for (word, (level, ..)) in &tables.decors {
        words.entry(*level).or_insert_with(|| (word.clone(), pretty(word)));
    }
    // (A picture's shortest word: `eclipse`, not `eclipse_2`.)
    let mut pictures: Vec<(&String, &film::tables::Picture)> =
        tables.pictures.iter().filter(|(_, picture)| !film::tables::NOT_FOR_FILMS.contains(&picture.category.as_str())).collect();
    pictures.sort_by_key(|(word, _)| (word.len(), (*word).clone()));
    for (word, picture) in pictures {
        words.entry(picture.level).or_insert_with(|| (word.clone(), format!("{} ({})", pretty(word), picture.category)));
    }
    words
}

/// The places' and pictures' backdrops a film can open (`scene: <word>`).
pub(super) fn backdrops(session: &Session) -> Vec<Item> {
    let words = backdrop_words(session);
    assets::tables::backdrops(session.data())
        .into_iter()
        .filter_map(|backdrop| {
            let (word, name) = words.get(&backdrop.level)?.clone();
            let about = format!("level {} ({}) · {}×{}", backdrop.level, backdrop.name, backdrop.width, backdrop.height);
            let level = i64::from(backdrop.level);
            Some(Item { shelf: Shelf::Backdrops, word, label: name, about, number: level, file: None, order: level })
        })
        .collect()
}

/// A tileset's first look among the game's dungeons: the dungeon, the floor
/// of those a look can take (`look: Beach Cave 2`), by tileset.
fn tileset_looks(session: &Session) -> HashMap<u8, (String, usize)> {
    let Some(offered) = book(session) else { return HashMap::new() };
    let known = game::dungeon_check::Known::of(&offered.book);
    let mut looks: HashMap<u8, (String, usize)> = HashMap::new();
    for (id, name, _) in &known.looks {
        let Some(dungeon) = offered.book.tables.dungeons.get(*id as usize) else { continue };
        for (at, floor) in dungeon.floors.iter().filter(|floor| floor.fixed.is_none()).enumerate() {
            looks.entry(floor.tileset).or_insert_with(|| (name.clone(), at + 1));
        }
    }
    looks
}

/// The game's dungeon tilesets (`tilesets/<number>.png`), by the first dungeon floor that has each.
pub(super) fn tilesets(session: &Session) -> Vec<Item> {
    let looks = tileset_looks(session);
    let folder = assets::data::path(session.data(), "tilesets");
    let mut numbers: Vec<u16> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(".png")?.parse().ok())
        .collect();
    numbers.sort_unstable();
    numbers
        .into_iter()
        .map(|number| {
            let look = u8::try_from(number).ok().and_then(|tileset| looks.get(&tileset));
            let (label, about) = match look {
                Some((name, floor)) => (format!("{name} {floor}"), format!("tileset No. {number} · the look of {name}, floor {floor}")),
                None => (format!("Tileset {number}"), format!("tileset No. {number} · no floor a look takes has it")),
            };
            let word = match look {
                Some((name, floor)) => format!("{name} {floor}"),
                None => number.to_string(),
            };
            Item { shelf: Shelf::Tilesets, word, label, about, number: i64::from(number), file: None, order: i64::from(number) }
        })
        .collect()
}
