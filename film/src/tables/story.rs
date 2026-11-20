//! What the story's scenes say of its places: what stands in them, where
//! someone is put most, where each is on the world map; and the names the
//! game gives its places.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::files::{game_file, number, table};
use super::{Level, Tables};
use crate::screen::without_marks;
use crate::words::plain;

/// How things and items are numbered among those of the stage.
const THINGS: u32 = 0x4000;
const ITEMS: u32 = 0x8000;
/// How often a thing stands in a place for it to be the place's own.
const OFTEN: f32 = 0.4;

/// How many times each was seen, by place.
type Counted<T> = HashMap<String, BTreeMap<T, u32>>;

pub(super) fn read_story(tables: &mut Tables) {
    let text = std::fs::read_to_string(game_file(&tables.data, "story.txt")).unwrap_or_default();
    let mut seen: Counted<(String, i32, i32)> = HashMap::new();
    let mut shown: HashMap<String, u32> = HashMap::new();
    let (mut backdrop, mut here, mut place) = (String::new(), HashSet::new(), String::new());
    let mut usual: Counted<(i32, i32)> = HashMap::new();
    let mut heroes: Counted<(i32, i32)> = HashMap::new();
    let mut map: Counted<(i32, i32)> = HashMap::new();
    let mut last: Option<(i32, i32)> = None;
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "backdrop" if f.len() > 1 => {
                (backdrop, place) = (f[1].to_string(), f[1].to_string());
                here = HashSet::new();
                *shown.entry(backdrop.clone()).or_default() += 1;
            }
            "scene" if f.len() > 1 => place = f[1].to_string(),
            "put" if f.len() > 3 => {
                let (Ok(x), Ok(y)) = (f[2].parse::<i32>(), f[3].parse::<i32>()) else {
                    continue;
                };
                *usual.entry(place.clone()).or_default().entry((x, y)).or_default() += 1;
                if matches!(f[1], "0" | "10") {
                    *heroes.entry(place.clone()).or_default().entry((x, y)).or_default() += 1;
                }
                let thing = f[1].parse::<u32>().ok().filter(|who| (THINGS..ITEMS).contains(who));
                if thing.is_some() && !backdrop.is_empty() && here.insert((f[1].to_string(), x, y)) {
                    *seen.entry(backdrop.clone()).or_default().entry((f[1].to_string(), x, y)).or_default() += 1;
                }
            }
            "map" if f.len() >= 3 => match f[1] {
                "look" if f.len() >= 4 => last = Some((number(f[2]), number(f[3]))),
                "name" if f[2] != "-" => {
                    if let Some(at) = last {
                        *map.entry(plain(f[2])).or_default().entry(at).or_default() += 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    for (level, things) in seen {
        let times = shown.get(&level).copied().unwrap_or(0);
        let kept = things.into_iter().filter(|(_, seen)| *seen >= 2.max((times as f32 * OFTEN).ceil() as u32)).map(|(thing, _)| thing).collect();
        tables.things.insert(level, kept);
    }
    for (level, spots) in usual {
        let mut spots: Vec<((i32, i32), u32)> = spots.into_iter().collect();
        spots.sort_by_key(|(_, times)| std::cmp::Reverse(*times));
        tables.usual_spots.insert(level, spots.into_iter().map(|(spot, _)| spot).collect());
    }
    for (name, spots) in map {
        if let Some((at, _)) = spots.into_iter().max_by_key(|(_, times)| *times) {
            tables.map_spots.insert(name, at);
        }
    }
    read_levels(tables, &heroes);
}

/// Each level's name as the game shows it, its usual middle (where the story
/// puts the team there most).
fn read_levels(tables: &mut Tables, heroes: &Counted<(i32, i32)>) {
    let data = tables.data.clone();
    let by_number =
        |file: &str| table(&data, file).into_iter().filter(|row| row.len() > 1).map(|row| (number(&row[0]), row[1].clone())).collect::<HashMap<i32, String>>();
    let names = by_number("place_names.txt");
    let sizes: HashMap<String, (i32, i32)> =
        assets::tables::backdrops(&tables.data).into_iter().map(|backdrop| (backdrop.level.to_string(), (backdrop.width, backdrop.height))).collect();
    for row in table(&tables.data, "levels.txt").into_iter().filter(|row| row.len() > 3) {
        let place = number(&row[3]);
        let name = names.get(&place);
        let (Some(size), Some(spots), Some(name)) = (sizes.get(&row[0]), heroes.get(&row[0]), name) else {
            continue;
        };
        let middle = |of: fn(&(i32, i32)) -> i32| {
            let mut all: Vec<i32> = spots.iter().flat_map(|(at, times)| std::iter::repeat_n(of(at), *times as usize)).collect();
            all.sort_unstable();
            all[all.len() / 2]
        };
        let shown = without_marks(name);
        let level = Level { level: number(&row[0]) as u16, x: middle(|at| at.0), y: middle(|at| at.1), name: shown.clone(), size: *size };
        tables.levels.insert(plain(&shown.replace(' ', "_")), level);
    }
    let mut places: HashSet<String> = HashSet::new();
    places.extend(table(&tables.data, "place_names.txt").into_iter().filter(|row| row.len() > 1 && row[1].trim().contains(' ')).map(|row| row[1].clone()));
    tables.place_names = places.into_iter().collect();
    tables.place_names.sort_by_key(|name| (std::cmp::Reverse(name.len()), name.clone()));
}
