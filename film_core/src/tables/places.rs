//! Places and what can stand in them: grounds, the places made by
//! the `decor` builder, the game's pictures and animations, props, our own
//! pictures.

use std::collections::HashMap;

use engine::Ground;

use super::files::{lines_of, number, table};
use super::{Animation, Picture, Tables};
use crate::words::plain;

/// The size of a picture the game does not tell.
const SCREEN: (i32, i32) = (256, 192);

/// Where one may walk in each place; the places made by the `decor` builder,
/// and their ways in and out.
pub(super) fn read_grounds(tables: &mut Tables) {
    let folder = tables.data.clone();
    for ground in assets::tables::grounds(&folder) {
        if let Some(parsed) = Ground::parse(&ground.rows) {
            tables.grounds.insert(ground.level, parsed);
        }
    }
    for place in assets::tables::built(&folder) {
        tables.decors.insert(plain(&place.name), (place.level, place.middle.0, place.middle.1));
        tables.entrances.insert(place.level, place.entrances.iter().map(|(name, x, y)| (plain(name), *x, *y)).collect());
    }
}

/// Who lives in the places of this project's (`town.txt`, `actor m<species>`).
pub(super) fn read_residents(tables: &mut Tables) {
    const FACING: [&str; 8] = ["down", "downright", "right", "upright", "up", "upleft", "left", "downleft"];
    let names: HashMap<String, String> = tables.monsters.iter().map(|(name, (number, _))| (number.clone(), name.clone())).collect();
    for actor in assets::tables::actors(&tables.data) {
        let Some(name) = actor.entity.strip_prefix('m').and_then(|species| names.get(species)) else {
            continue;
        };
        let facing = FACING[(actor.facing.max(1) as usize - 1) % 8].to_string();
        let resident = super::Resident {
            name: name.clone(),
            x: actor.x,
            y: actor.y,
            facing,
            wanders: actor.wanders,
            animation: actor.animation.clone(),
            still: actor.still,
        };
        tables.residents.entry(actor.level).or_default().push(resident);
    }
}

/// The game's pictures and animations, by their words (`library/tables/images.txt`).
pub(super) fn read_catalogue(tables: &mut Tables) {
    let sizes: HashMap<i32, (i32, i32)> =
        assets::tables::backdrops(&tables.data).into_iter().map(|backdrop| (backdrop.level as i32, (backdrop.width, backdrop.height))).collect();
    for line in lines_of(&tables.project.root().join("library/tables/images.txt")).iter().filter(|line| !line.starts_with('#')) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.first() == Some(&"anim") && f.len() > 7 {
            let (level, category, name, columns, rows) = (number(f[1]), f[2], f[3], number(f[4]), number(f[5]));
            let size = sizes.get(&level).copied().unwrap_or(SCREEN);
            let (wide, high) = (size.0 / columns, size.1 / rows);
            let cases: Vec<(i32, i32)> = f[7].split(',').map(number).map(|case| (case % columns * wide + wide / 2, case / columns * high + high / 2)).collect();
            let picture = |(x, y): (i32, i32)| Picture { level: level as u16, category: category.to_string(), x, y };
            tables.pictures.insert(name.to_string(), picture(cases[0]));
            for (at, case) in cases.iter().enumerate() {
                tables.pictures.insert(format!("{name}_{}", at + 1), picture(*case));
            }
            tables.animations.insert(name.to_string(), Animation { frames: number(f[6]), cases });
        } else if f.len() >= 3 {
            let (wide, high) = sizes.get(&number(f[0])).copied().unwrap_or(SCREEN);
            let (x, y) = if f.len() > 4 { (number(f[3]), number(f[4])) } else { (wide / 2, high / 2) };
            tables.pictures.insert(f[2].to_string(), Picture { level: number(f[0]) as u16, category: f[1].to_string(), x, y });
        }
    }
    tables.picture_levels = tables.pictures.values().map(|picture| picture.level).collect();
}

/// The things a film can use: props, pictures of our own.
pub(super) fn read_things(tables: &mut Tables) {
    for row in assets::tables::sheets(&tables.data, "props.txt").into_iter().filter(|row| row.animation == 0) {
        tables.prop_frames.insert(row.number, row.sheet.durations.iter().map(|&ticks| i32::from(ticks)).sum());
    }
    for row in table(&tables.data, "objects.txt").into_iter().filter(|row| row.len() > 1) {
        if tables.prop_frames.contains_key(&(number(&row[0]) as u16)) {
            tables.props.insert(plain(&row[1]), number(&row[0]) as u16);
        }
    }
    for line in lines_of(&tables.project.root().join("library/tables/objects.txt")).iter().filter(|line| !line.starts_with('#')) {
        if let [object, word] = line.split_whitespace().collect::<Vec<&str>>()[..] {
            tables.props.insert(word.to_string(), number(object) as u16);
        }
    }
    let mut folders = tables.project.folders("images");
    folders.push(std::path::Path::new(&tables.data).join("hand"));
    for folder in folders {
        for file in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let file = file.file_name().to_string_lossy().into_owned();
            let Some(stem) = file.strip_suffix(".png") else {
                continue;
            };
            let (name, cells) = stem.rsplit_once('.').and_then(|(name, cells)| Some((name, cells.parse().ok()?))).unwrap_or((stem, 1));
            tables.drawings.insert(plain(name), (file.clone(), cells));
        }
    }
}
