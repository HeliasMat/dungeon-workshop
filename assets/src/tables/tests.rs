//! The typed tables read the project's real files as the readers they
//! replaced did (each old reader copied here as it was); without the
//! game's data, nothing to compare.

use std::collections::{HashMap, HashSet};

use super::*;

/// The project's data, if it has been extracted (the folder `DUNGEON_DATA`
/// names, else `data/`; not the fixture).
fn real() -> Option<String> {
    let folder = crate::testing::data();
    crate::testing::extracted(&folder).then_some(folder)
}

/// `shell::art::rows` as it was: every line, empty or not.
fn old_rows(folder: &str, file: &str) -> Vec<Vec<String>> {
    data::text(folder, file).lines().map(|line| line.split('\t').map(str::to_string).collect()).collect()
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_backdrops_are_read_as_before() {
    let folder = real().expect("the extracted data/");
    let new = backdrops(&folder);
    assert!(new.len() > 400);
    // (preview/src/check.rs)
    let text = data::text(&folder, "backdrops.txt");
    let size = |line: &str| {
        let fields: Vec<&str> = line.split('\t').collect();
        Some((fields.first()?.parse::<u16>().ok()?, (fields.get(2)?.parse::<f32>().ok()?, fields.get(3)?.parse::<f32>().ok()?)))
    };
    let old: HashMap<u16, (f32, f32)> = text.lines().filter_map(size).collect();
    assert_eq!(old, new.iter().map(|backdrop| (backdrop.level, (backdrop.width as f32, backdrop.height as f32))).collect());
    // (film_core/src/tables/places.rs and story.rs)
    let old: HashMap<i32, (i32, i32)> =
        rows_of(&folder, "backdrops.txt").iter().filter(|row| row.len() > 3).map(|row| (number(&row[0]), (number(&row[2]), number(&row[3])))).collect();
    assert_eq!(old, new.iter().map(|backdrop| (backdrop.level as i32, (backdrop.width, backdrop.height))).collect());
    // (a level by its name)
    let level_of = |code: &str| {
        text.lines().find_map(|line| {
            let mut fields = line.split('\t');
            let level = fields.next()?.parse::<u16>().ok()?;
            (fields.next()? == code).then_some(level)
        })
    };
    for backdrop in &new {
        assert_eq!(level_of(&backdrop.name), new.iter().find(|other| other.name == backdrop.name).map(|other| other.level));
    }
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_grounds_are_read_as_before() {
    let folder = real().expect("the extracted data/");
    // (film_core/src/tables/places.rs, decor/src/source.rs)
    let old: Vec<(u16, String)> =
        rows_of(&folder, "ground.txt").into_iter().filter(|row| row.len() > 2).filter_map(|row| Some((row[0].parse().ok()?, row[2].clone()))).collect();
    let new: Vec<(u16, String)> = grounds(&folder).into_iter().map(|ground| (ground.level, ground.rows)).collect();
    assert!(new.len() > 300);
    assert_eq!(old, new);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_places_built_are_read_as_before() {
    let folder = real().expect("the extracted data/");
    // (film_core/src/tables/places.rs)
    let mut old = Vec::new();
    for row in rows_of(&folder, "decors.txt").into_iter().filter(|row| row.len() > 3) {
        let ways: Vec<(String, i32, i32)> = row
            .get(5)
            .map_or("", String::as_str)
            .split(';')
            .filter_map(|way| {
                let mut parts = way.split(',');
                let (x, y) = (parts.next()?.trim().parse().ok()?, parts.next()?.trim().parse().ok()?);
                Some((parts.next()?.to_string(), x, y))
            })
            .collect();
        old.push((row[0].clone(), number(&row[1]) as u16, number(&row[2]), number(&row[3]), ways));
    }
    let new: Vec<_> = built(&folder).into_iter().map(|place| (place.name, place.level, place.middle.0, place.middle.1, place.entrances)).collect();
    // (As many as the project has: each of library/decors built.)
    assert!(new.len() > 30, "{}", new.len());
    assert_eq!(old, new);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_names_and_the_characters_are_read_as_before() {
    let folder = real().expect("the extracted data/");
    // (stage_view/src/stage/cast.rs)
    let number = |field: Option<&String>| field.and_then(|field| field.parse::<u16>().ok());
    for file in ["monsters.txt", "items.txt", "moves.txt"] {
        let old: HashMap<u16, String> = old_rows(&folder, file).into_iter().filter_map(|row| Some((number(row.first())?, row.get(1)?.clone()))).collect();
        assert_eq!(old, numbered(&folder, file).into_iter().collect(), "{file}");
    }
    let old: Vec<(String, u16)> =
        old_rows(&folder, "entities.txt").into_iter().filter_map(|row| Some((row.get(1)?.clone(), number(row.get(2)).unwrap_or(0)))).collect();
    assert_eq!(old, entities(&folder));
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_sheets_are_read_as_before() {
    let folder = real().expect("the extracted data/");
    // (media/src/art_files.rs: `sheets_of`, and the things lying flat)
    type Old = HashMap<(u16, u8), (u32, u32, Vec<u8>, (i32, i32))>;
    let old_sheets = |file: &str| -> Old {
        let sheet = |line: &str| {
            let fields: Vec<&str> = line.split('\t').collect();
            let number = |at: usize| fields.get(at).and_then(|field| field.parse::<i32>().ok());
            let durations = fields.get(4)?.split(',').filter_map(|ticks| ticks.parse().ok()).collect();
            let shadow = (number(5).unwrap_or(0), number(6).unwrap_or(0));
            Some(((number(0)? as u16, number(1)? as u8), (number(2)? as u32, number(3)? as u32, durations, shadow)))
        };
        data::text(&folder, file).lines().filter_map(sheet).collect()
    };
    for file in ["sprites.txt", "props.txt"] {
        let new: Old = sheets(&folder, file)
            .into_iter()
            .map(|row| ((row.number, row.animation), (row.sheet.wide, row.sheet.high, row.sheet.durations, row.sheet.shadow)))
            .collect();
        assert!(new.len() > 10, "{file}");
        assert_eq!(old_sheets(file), new, "{file}");
    }
    let flat = |line: &str| {
        let fields: Vec<&str> = line.split('\t').collect();
        (fields.get(7) == Some(&"1")).then(|| fields.first()?.parse::<u16>().ok()).flatten()
    };
    let old: HashSet<u16> = data::text(&folder, "props.txt").lines().filter_map(flat).collect();
    assert_eq!(old, sheets(&folder, "props.txt").into_iter().filter(|row| row.flat).map(|row| row.number).collect());
    // (film_core/src/tables/monsters.rs: how big each shows, who is drawn, who aims)
    let rows = rows_of(&folder, "sprites.txt");
    let mut old: HashMap<String, (i32, i32)> = HashMap::new();
    for row in rows.iter().filter(|row| row.len() > 3) {
        old.entry(row[0].clone()).or_insert(((number(&row[2]) - 16).max(8), (number(&row[3]) - 16).max(8)));
    }
    let mut new: HashMap<String, (i32, i32)> = HashMap::new();
    for row in sheets(&folder, "sprites.txt") {
        new.entry(row.number.to_string()).or_insert(((row.sheet.wide as i32 - 16).max(8), (row.sheet.high as i32 - 16).max(8)));
    }
    assert_eq!(old, new);
    let drawn: HashSet<String> = rows.iter().map(|row| row[0].clone()).collect();
    assert_eq!(drawn, sheets(&folder, "sprites.txt").iter().map(|row| row.number.to_string()).collect());
    let aiming: HashSet<String> = rows.iter().filter(|row| row.len() > 1 && row[1] == "55").map(|row| row[0].clone()).collect();
    assert_eq!(aiming, sheets(&folder, "sprites.txt").iter().filter(|row| row.animation == 55).map(|row| row.number.to_string()).collect());
    // (film_core/src/tables/places.rs: the steps of each thing)
    let old: HashMap<u16, i32> = rows_of(&folder, "props.txt")
        .into_iter()
        .filter(|row| row.len() > 4 && row[1] == "0")
        .map(|row| (number(&row[0]) as u16, row[4].split(',').map(number).sum()))
        .collect();
    let new: HashMap<u16, i32> = sheets(&folder, "props.txt")
        .into_iter()
        .filter(|row| row.animation == 0)
        .map(|row| (row.number, row.sheet.durations.iter().map(|&ticks| i32::from(ticks)).sum()))
        .collect();
    assert_eq!(old, new);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_effects_and_the_statuses_are_read_as_before() {
    let folder = real().expect("the extracted data/");
    // (media/src/art_files.rs: `effect_file`, its numbers; the rain of shell/src/art.rs)
    type Old = (u32, u32, Vec<(usize, u8)>, f32, bool, bool, u16, u8, u32);
    let text = data::text(&folder, "effects.txt");
    let lines: HashMap<u16, &str> = text.lines().filter_map(|line| Some((line.split('\t').next()?.parse().ok()?, line))).collect();
    let old = |line: &str| -> Option<Old> {
        let fields: Vec<&str> = line.split('\t').collect();
        let number = |at: usize| fields.get(at).and_then(|field| field.parse::<u32>().ok());
        let steps = fields.get(3)?.split(',').enumerate().filter_map(|(at, ticks)| Some((at, ticks.parse().ok()?))).collect();
        Some((
            number(1)?,
            number(2)?,
            steps,
            number(4)? as f32,
            number(5)? != 0,
            number(6)? != 0,
            number(7)? as u16,
            number(8).unwrap_or(3) as u8,
            number(9).unwrap_or(1).clamp(1, 8),
        ))
    };
    let new: HashMap<u16, EffectRow> = effects(&folder).into_iter().map(|effect| (effect.id, effect)).collect();
    assert!(new.len() > 100);
    let old: HashMap<u16, Old> = lines.iter().filter_map(|(id, line)| Some((*id, old(line)?))).collect();
    let made = |e: &EffectRow| -> Old {
        let steps = e.durations.iter().copied().enumerate().collect();
        (e.wide, e.high, steps, e.ticks as f32, e.repeats, e.screen, e.sound, e.point, e.ways)
    };
    assert_eq!(old, new.iter().map(|(id, effect)| (*id, made(effect))).collect());
    let rain: Vec<&str> = lines[&697].split('\t').collect();
    assert_eq!(rain[3].split(',').next().unwrap().parse::<f32>().unwrap(), new[&697].durations[0] as f32);
    // (media/src/art_files.rs: `status_icons`, its numbers)
    let old: Vec<(String, u32, u32, u32)> = data::text(&folder, "status.txt")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let number = |at: usize| fields.get(at).and_then(|field| field.parse::<u32>().ok()).unwrap_or(0);
            (fields[0].to_string(), number(1), number(2), number(3))
        })
        .collect();
    assert!(!old.is_empty());
    assert_eq!(old, statuses(&folder).into_iter().map(|status| (status.name, status.wide, status.high, status.frames)).collect::<Vec<_>>());
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_variants_the_tips_and_who_lives_where_are_read_as_before() {
    let folder = real().expect("the extracted data/");
    // (shell/src/art.rs: `variants_of`; film_core/src/tables/monsters.rs: the rows of three fields)
    let old: HashMap<u16, u16> =
        rows_of(&folder, "variants.txt").into_iter().filter_map(|row| Some((row.first()?.parse().ok()?, row.get(1)?.parse().ok()?))).collect();
    assert_eq!(old, variants(&folder).into_iter().map(|variant| (variant.number, variant.of)).collect());
    let old: Vec<(String, String, String, Option<String>)> = rows_of(&folder, "variants.txt")
        .into_iter()
        .filter(|row| row.len() > 2)
        .map(|row| (row[0].clone(), row[1].clone(), row[2].clone(), row.get(3).cloned()))
        .collect();
    let new: Vec<_> = variants(&folder).into_iter().map(|variant| (variant.number.to_string(), variant.of.to_string(), variant.word, variant.ways)).collect();
    assert_eq!(old, new);
    // (film: who fishes, by the first field of `rod_tips.txt`)
    let old: HashSet<String> = rows_of(&folder, "rod_tips.txt").into_iter().filter_map(|row| row.into_iter().next()).collect();
    assert_eq!(old, tips(&folder).into_keys().map(|(monster, _, _)| monster.to_string()).collect());
    // (film: the monsters and the Pokédex, by their numbers as written)
    for file in ["monsters.txt", "dex.txt"] {
        let old: Vec<(String, String)> = rows_of(&folder, file).into_iter().filter(|row| row.len() > 1).map(|row| (row[0].clone(), row[1].clone())).collect();
        assert_eq!(old, numbered(&folder, file).into_iter().map(|(id, name)| (id.to_string(), name)).collect::<Vec<_>>(), "{file}");
    }
    // (film_core/src/tables/places.rs: `read_residents`)
    let (mut level, mut old) = (0, Vec::new());
    for row in rows_of(&folder, "town.txt") {
        match row[0].as_str() {
            "place" => level = number(&row[1]) as u16,
            "actor" if row.len() > 4 => {
                old.push((level, row[1].clone(), number(&row[2]), number(&row[3]), number(&row[4]), row.get(6).map_or(0, |wide| number(wide))))
            }
            _ => {}
        }
    }
    assert!(old.len() > 100);
    assert_eq!(old, actors(&folder).into_iter().map(|actor| (actor.level, actor.entity, actor.x, actor.y, actor.facing, actor.wanders)).collect::<Vec<_>>());
}

#[test]
fn a_ground_s_tiles_are_its_rows() {
    let ground = GroundRows { level: 240, how: "jeu".to_string(), rows: "01/10/".to_string() };
    assert_eq!(ground.tiles(), vec![vec![false, true], vec![true, false]]);
}
