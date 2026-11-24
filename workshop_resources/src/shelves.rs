//! The things on each shelf of the Library: listed, sought, filtered, the
//! favourites and the folders of the things drawn.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{Item, Shelf, game_shelves};
use workshop_base::session::Session;
use workshop_decor::Decors;

/// A Pokémon's number in the Pokédex, by its number in the game (those of the project: theirs, less 2000).
pub(super) fn dex_of(session: &Session, number: i64) -> i64 {
    let dex = session.dex.get(|| {
        assets::data::text(session.data(), "dex.txt")
            .lines()
            .filter_map(|line| {
                let (number, at) = line.split_once('\t')?;
                Some((number.trim().parse().ok()?, at.trim().parse().ok()?))
            })
            .collect()
    });
    dex.get(&number).copied().unwrap_or(if number > 2000 { number - 2000 } else { number })
}

/// Where the favourites are kept.
pub(super) fn favourites_file(session: &Session) -> PathBuf {
    session.root().join("target").join("workshop").join("resource_favourites.txt")
}

/// The folders the things drawn are in (ours first).
pub(super) fn drawn_folders(session: &Session) -> [PathBuf; 2] {
    [session.library().images(), std::path::Path::new(session.data()).join("hand")]
}

/// A shelf's things, read from the game's tables and the project's files.
pub(super) fn made_list(session: &Session, shelf: Shelf, decors: &Decors) -> Vec<Item> {
    match shelf {
        Shelf::Pieces => pieces(decors),
        _ => listed(session, shelf),
    }
}

/// A shelf's things, but the decor's pieces (the Decor space's).
pub(super) fn listed(session: &Session, shelf: Shelf) -> Vec<Item> {
    let tables = session.tables();
    match shelf {
        Shelf::Pokemon => pokemon(session),
        Shelf::Variants => variants(session),
        Shelf::Drawn => drawn(session),
        Shelf::Props => {
            tables.props.iter().map(|(word, number)| item(shelf, word, word, format!("game prop No. {number}"), i64::from(*number), None)).collect()
        }
        Shelf::Songs => rules::songs::SONGS
            .iter()
            .map(|song| item(shelf, song.word, song.word, format!("{} · {}", song.title, song.moods), i64::from(song.number), None))
            .collect(),
        Shelf::Sounds => sounds(session),
        Shelf::Dungeons => dungeons(session),
        Shelf::Bosses => bosses(session),
        Shelf::Effects => game_shelves::other_effects(session),
        Shelf::Attacks => game_shelves::attacks(session),
        Shelf::Statuses => game_shelves::statuses(session),
        Shelf::Weather => game_shelves::weathers(),
        Shelf::Portraits => game_shelves::portraits(session, &[pokemon(session), variants(session)].concat()),
        Shelf::Items => game_shelves::items(session),
        Shelf::Backdrops => game_shelves::backdrops(session),
        Shelf::Tilesets => game_shelves::tilesets(session),
        Shelf::Pieces | Shelf::Favourites => Vec::new(),
    }
}

/// The decor's pieces, as the Decor space has read them.
fn pieces(decors: &Decors) -> Vec<Item> {
    let shelf = Shelf::Pieces;
    decors
        .pieces
        .iter()
        .enumerate()
        .map(|(at, piece)| {
            let place = piece.place.as_ref().map_or_else(|| "the project's".to_string(), |(_, name)| name.clone());
            let kind = match piece.kind {
                'a' => "animated tiles",
                'o' => "scene thing",
                _ => "over",
            };
            item(shelf, &piece.name, &piece.name, format!("{kind} · {place} · {}×{}", piece.size[0], piece.size[1]), at as i64, None)
        })
        .collect()
}

/// A thing of a shelf, sorted by its number.
fn item(shelf: Shelf, word: &str, label: &str, about: String, number: i64, file: Option<PathBuf>) -> Item {
    Item { shelf, word: word.to_string(), label: label.to_string(), about, number, file, order: number }
}

/// Each species once, by the word its name makes; its first number. (Those the game draws only: the names of
/// later Pokémon are known, not their bodies.)
fn pokemon(session: &Session) -> Vec<Item> {
    let tables = session.tables();
    let drawn: std::collections::HashSet<&String> = tables.with_sprites.iter().collect();
    let mut by_name: BTreeMap<String, (i64, String)> = BTreeMap::new();
    for (word, (number, name)) in tables.monsters.iter().filter(|(word, _)| drawn.contains(word)) {
        let Ok(number) = number.parse::<i64>() else { continue };
        if number == 0 || name.is_empty() || name.starts_with('?') || film::words::plain(name) != *word {
            continue;
        }
        let kept = by_name.entry(name.clone()).or_insert((number, word.clone()));
        if number < kept.0 {
            *kept = (number, word.clone());
        }
    }
    by_name
        .into_iter()
        .map(|(name, (number, word))| {
            let dex = dex_of(session, number);
            // (Those of the game that are no Pokémon (a decoy, a statue): last.)
            let order = if dex > 0 { dex } else { 100_000 + number };
            let about = if dex > 0 { format!("Pokédex No. {dex}") } else { format!("the game's, No. {number}") };
            Item { order, ..item(Shelf::Pokemon, &word, &name, about, number, None) }
        })
        .collect()
}

/// The variants: each its own number, its own body; sorted with its species.
fn variants(session: &Session) -> Vec<Item> {
    let tables = session.tables();
    tables
        .variants
        .iter()
        .flat_map(|(of, words)| {
            let species = tables.monsters.values().find(|(number, _)| number == of).map_or(String::new(), |(_, name)| name.clone());
            let number = of.parse::<i64>().unwrap_or(0);
            words.iter().map(move |word| (word.clone(), species.clone(), number))
        })
        .map(|(word, species, number)| {
            let own = tables.monsters.get(&word).and_then(|(own, _)| own.parse::<i64>().ok()).unwrap_or(number);
            Item {
                order: dex_of(session, number),
                ..item(Shelf::Variants, &word, &word, format!("variant of {species} (Pokédex No. {})", dex_of(session, number)), own, None)
            }
        })
        .collect()
}

/// A drawn picture's name and how many images it holds, of its file's stem (`smoke.4`); none for a large one.
pub(super) fn drawn_name(stem: &str) -> Option<(&str, u32)> {
    if stem.ends_with("_grand") {
        return None;
    }
    Some(stem.rsplit_once('.').and_then(|(name, cells)| Some((name, cells.parse::<u32>().ok()?))).unwrap_or((stem, 1)))
}

/// The pictures drawn, of our folder first: each word once.
fn drawn(session: &Session) -> Vec<Item> {
    let mut found: BTreeMap<String, Item> = BTreeMap::new();
    for folder in drawn_folders(session) {
        for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let file = entry.file_name().to_string_lossy().into_owned();
            let Some((name, cells)) = file.strip_suffix(".png").and_then(drawn_name) else { continue };
            let word = film::words::plain(name);
            found.entry(word.clone()).or_insert_with(|| Item {
                order: 0,
                ..item(Shelf::Drawn, &word, name, format!("{cells} image{}", if cells > 1 { "s" } else { "" }), i64::from(cells), Some(entry.path()))
            });
        }
    }
    found.into_values().collect()
}

/// One's own sounds first, then the game's noises.
fn sounds(session: &Session) -> Vec<Item> {
    let (data, library) = (session.data(), session.library());
    let own = shell::own_sounds::names_in(&library.sounds()).into_iter().map(|name| {
        let seconds = shell::own_sounds::seconds(data, library, &name).unwrap_or(0.0);
        let file = shell::own_sounds::path(data, library, &name);
        Item { order: -1, ..item(Shelf::Sounds, &name, &name, format!("own sound · {seconds:.2} s"), -1, Some(file)) }
    });
    own.chain(rules::songs::NOISES.iter().map(|noise| item(Shelf::Sounds, noise.word, noise.word, noise.when.to_string(), i64::from(noise.number), None)))
        .collect()
}

/// The dungeons written: each its name, floors, look, those met; the first of them pictured.
fn dungeons(session: &Session) -> Vec<Item> {
    workshop_dungeon::summaries(session)
        .into_iter()
        .map(|dungeon| {
            let floors = if dungeon.floors == 0 {
                "the fight alone".to_string()
            } else {
                format!("{} floor{}", dungeon.floors, if dungeon.floors > 1 { "s" } else { "" })
            };
            let boss = dungeon.boss.as_ref().map_or(String::new(), |boss| format!(" · boss: {} Lv. {}", boss.species, boss.level));
            let met = if dungeon.foes.is_empty() { String::new() } else { format!(" · {}", dungeon.foes.join(", ")) };
            let picture = dungeon.boss.as_ref().and_then(|boss| boss.id).or(dungeon.first).map_or(0, i64::from);
            let about = format!("{floors} · look: {}{boss}{met}", if dungeon.look.is_empty() { "—" } else { &dungeon.look });
            Item { order: 0, ..item(Shelf::Dungeons, &dungeon.word, &dungeon.name, about, picture, Some(dungeon.path)) }
        })
        .collect()
}

/// The boss fights: each dungeon that has one, by its boss.
fn bosses(session: &Session) -> Vec<Item> {
    workshop_dungeon::fight_summaries(session)
        .into_iter()
        .filter_map(|dungeon| {
            let boss = dungeon.boss?;
            let title = if boss.title.is_empty() { String::new() } else { format!(" — {}", boss.title) };
            let about = format!(
                "{} Lv. {}{title} · HP ×{} · {} guard{} · {} phase{} · “{}”",
                boss.species,
                boss.level,
                boss.life,
                boss.guards,
                if boss.guards > 1 { "s" } else { "" },
                boss.phases,
                if boss.phases > 1 { "s" } else { "" },
                dungeon.name
            );
            Some(Item { order: 0, ..item(Shelf::Bosses, &dungeon.word, &boss.species, about, boss.id.map_or(0, i64::from), Some(dungeon.path)) })
        })
        .collect()
}

/// The words a filter keeps: a generation's Pokémon, a mood's songs.
pub(super) fn filter_words(session: &Session, shelf: Shelf, filter: &str) -> std::collections::HashSet<String> {
    match shelf {
        Shelf::Pokemon => {
            filter.parse::<usize>().ok().and_then(|generation| session.tables().generations.get(&generation).cloned()).unwrap_or_default().into_iter().collect()
        }
        Shelf::Songs => {
            rules::songs::SONGS.iter().filter(|song| song.moods.split_whitespace().any(|mood| mood == filter)).map(|song| song.word.to_string()).collect()
        }
        _ => std::collections::HashSet::new(),
    }
}

/// The filters a shelf has: its generations, its moods (the most frequent).
pub(super) fn filters_of(session: &Session, shelf: Shelf) -> Vec<(String, String)> {
    match shelf {
        Shelf::Pokemon => session.tables().generations.keys().map(|generation| (generation.to_string(), format!("Gen. {generation}"))).collect(),
        Shelf::Songs => {
            let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
            for mood in rules::songs::SONGS.iter().flat_map(|song| song.moods.split_whitespace()) {
                *counted.entry(mood).or_default() += 1;
            }
            let mut moods: Vec<(&str, usize)> = counted.into_iter().collect();
            moods.sort_by_key(|(mood, count)| (std::cmp::Reverse(*count), *mood));
            moods.into_iter().take(14).map(|(mood, _)| (mood.to_string(), mood.to_string())).collect()
        }
        _ => Vec::new(),
    }
}
