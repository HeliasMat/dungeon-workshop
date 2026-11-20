//! The Pokémon a film may name: the game's, those only the community draws,
//! our variants; how big each shows, and each generation.

use std::collections::{HashMap, HashSet};

use super::Tables;
use assets::tables::{SheetRow, numbered};

use super::files::number;
use crate::words::plain;

/// The first species only the community draws.
const FIRST_OF_OURS: i32 = 2000;
/// The sheet of the animation `Vise`.
const AIM_SHEET: u8 = 55;
/// The last of each generation, in the Pokédex.
const LAST_OF_EACH: [i32; 9] = [151, 251, 386, 493, 649, 721, 809, 905, 1025];

/// How big each shows: the first cell of its sheets, without what a cell
/// leaves round it.
fn bodies(sheets: &[SheetRow]) -> HashMap<String, (i32, i32)> {
    const ROOM: i32 = 16;
    let mut found = HashMap::new();
    for row in sheets {
        found.entry(row.number.to_string()).or_insert(((row.sheet.wide as i32 - ROOM).max(8), (row.sheet.high as i32 - ROOM).max(8)));
    }
    found
}

/// The monsters, those drawn, those with a portrait alone, the variants and
/// the generations.
pub(super) fn read_monsters(tables: &mut Tables) {
    let folder = tables.data.clone();
    let sheets = assets::tables::sheets(&folder, "sprites.txt");
    tables.bodies = bodies(&sheets);
    for (id, name) in numbered(&folder, "monsters.txt") {
        let suffix = if id == 32 { "_m" } else { "" };
        tables.monsters.entry(plain(&name) + suffix).or_insert((id.to_string(), name));
    }
    let drawn: HashSet<String> = sheets.iter().map(|row| row.number.to_string()).collect();
    let code = |tables: &Tables, name: &str| number(&tables.monsters[name].0);
    let mut with: Vec<String> = tables
        .monsters
        .iter()
        .filter(|(name, row)| drawn.contains(&row.0) && !name.is_empty() && !name.starts_with("type"))
        .map(|(name, _)| name.clone())
        .collect();
    with.sort_by_key(|name| code(tables, name));
    let sprited: HashSet<&String> = with.iter().collect();
    let ours = std::path::Path::new(&tables.data).join("hand/portraits");
    for (name, row) in &tables.monsters {
        let id = number(&row.0);
        let file = ours.join(format!("{:04}/Normal.png", id - FIRST_OF_OURS));
        if !sprited.contains(name) && id > FIRST_OF_OURS && file.exists() {
            tables.faces_alone.insert(name.clone(), file.to_string_lossy().into_owned());
        }
    }
    tables.with_sprites = with;
    read_generations(tables);
    read_variants(tables, &sheets);
    read_forms(tables);
}

/// The forms of the story that share a name with another (Dialga as the
/// game shows it at Temporal Tower, number 552): a word each.
const FORMS: [(&str, &str); 1] = [("552", "dialga_primal")];

/// Each monster whose name another has before it, reachable by its name and
/// its number (`dialga_552`), and the forms of the story by their word; none
/// of them in the lists of Pokémon (`with_sprites`).
fn read_forms(tables: &mut Tables) {
    for (id, shown) in numbered(&tables.data, "monsters.txt") {
        let (id, name) = (id.to_string(), plain(&shown));
        if tables.monsters.get(&name).is_some_and(|known| known.0 != id) && !name.is_empty() {
            tables.monsters.entry(format!("{name}_{id}")).or_insert((id, shown));
        }
    }
    for (number, word) in FORMS {
        if let Some(named) = tables.monsters.values().find(|named| named.0 == number).cloned() {
            tables.monsters.insert(word.to_string(), named);
        }
    }
}

/// The variants (painted by hand, `variants.txt`): a word of their own, the size and the
/// name of whose they are; no generation, no place in a list of Pokémon.
fn read_variants(tables: &mut Tables, sheets: &[SheetRow]) {
    let folder = tables.data.clone();
    for variant in assets::tables::variants(&folder) {
        let (number, of) = (variant.number.to_string(), variant.of.to_string());
        if let Some(body) = tables.bodies.get(&of).copied() {
            tables.bodies.insert(number.clone(), body);
        }
        let name = tables.monsters.values().find(|named| named.0 == of).map_or(String::new(), |named| named.1.clone());
        tables.monsters.insert(plain(&variant.word), (number.clone(), name));
        tables.variants.entry(of).or_default().push(plain(&variant.word));
        // (The ways painted all through: `variants.txt`, else the table itself.)
        let table = std::fs::read_to_string(std::path::Path::new(&tables.data).join("hand").join(&variant.word).join("table.txt")).unwrap_or_default();
        let told = table.lines().map(|line| line.split('#').next().unwrap_or("")).find_map(|line| line.trim().strip_prefix("views ").map(str::to_string));
        let ways: Vec<String> = variant.ways.or(told).unwrap_or_default().split([',', ' ']).filter(|way| !way.is_empty()).map(plain).collect();
        if !ways.is_empty() {
            tables.views.insert(number, ways);
        }
    }
    tables.aiming = sheets.iter().filter(|row| row.animation == AIM_SHEET).map(|row| row.number.to_string()).collect();
    // (A rod drawn in the pictures: its tips, `pointe` in the table.)
    tables.fishing = assets::tables::tips(&folder).into_keys().map(|(monster, _, _)| monster.to_string()).collect();
}

/// The Pokémon of each generation, in the order of the Pokédex.
fn read_generations(tables: &mut Tables) {
    let dex: HashMap<String, i32> = numbered(&tables.data, "dex.txt").into_iter().map(|(id, at)| (id.to_string(), number(&at))).collect();
    let dex_of = |id: &str| if number(id) > FIRST_OF_OURS { number(id) - FIRST_OF_OURS } else { dex.get(id).copied().unwrap_or(0) };
    let mut all: Vec<(i32, String)> =
        tables.with_sprites.iter().chain(tables.faces_alone.keys()).map(|name| (dex_of(&tables.monsters[name].0), name.clone())).collect();
    all.sort_by_key(|(at, _)| *at);
    let mut seen = HashSet::new();
    for (at, name) in all {
        if at != 0 && seen.insert(at) {
            let generation = LAST_OF_EACH.iter().position(|last| at <= *last).map_or(9, |place| place + 1);
            tables.generations.entry(generation).or_default().push(name);
        }
    }
}
