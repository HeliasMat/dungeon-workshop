//! What the game has to film in: Pokémon, places, pictures, grounds.

use std::collections::BTreeMap;

use image::RgbImage;

use ::film::Context;
use ::film::check::closest;
use ::film::tables::{PLACES, game_file};
use ::film::words::{SPOTS, plain};

use super::{out_folder, pictures::write_on};

pub fn search(context: &Context, words: &[&String]) -> Result<(), String> {
    let tables = context.tables();
    for word in words {
        let key = plain(word);
        println!("== {word}");
        let mut hits: Vec<String> = tables.monsters.iter().filter(|(name, _)| name.contains(&key)).map(|(name, row)| format!("{name} ({})", row.1)).collect();
        if hits.is_empty() {
            hits = closest(&key, tables.monsters.keys().map(String::as_str), 5, 0.6);
        }
        hits.truncate(12);
        println!("Pokémon: {}", if hits.is_empty() { "-".to_string() } else { hits.join(", ") });
        let mut places: Vec<&str> = tables.place_names_known().filter(|name| name.contains(&key)).collect();
        places.truncate(12);
        println!("Places: {}", if places.is_empty() { "-".to_string() } else { places.join(", ") });
    }
    Ok(())
}

/// A place by a word, or what to do.
fn place(context: &Context, word: &str) -> Result<(u16, i32, i32), String> {
    context.tables().place_of(word)
}

pub fn describe(context: &Context, word: &str) -> Result<(), String> {
    let (level, x, y) = place(context, word)?;
    let tables = context.tables();
    let ground = tables.grounds.get(&level);
    // (The middle as the films have it: on the ground, the nearest free tile.)
    let free = |at: (i32, i32)| {
        ground.map_or(at, |ground| {
            let found = ground.free(glam::Vec2::new(at.0 as f32, at.1 as f32));
            (found.x as i32, found.y as i32)
        })
    };
    let (x, y) = free((x, y));
    println!("level {level}, middle ({x}, {y})");
    let walled = |at: (i32, i32)| ground.is_some_and(|ground| ground.blocked(glam::Vec2::new(at.0 as f32, at.1 as f32), 1));
    let said = |at: (i32, i32)| match walled(at) {
        // (On a wall: where one is put instead, the nearest free tile.)
        true => format!(" (wall: put at {:?})", free(at)),
        false => String::new(),
    };
    let spots: Vec<String> = SPOTS.iter().map(|(name, (dx, dy))| format!("{name} ({}, {}){}", x + dx, y + dy, said((x + dx, y + dy)))).collect();
    println!("spots: {}", spots.join(", "));
    if let Some(ways) = tables.entrances.get(&level).filter(|ways| !ways.is_empty()) {
        let ways: Vec<String> = ways.iter().map(|(name, way_x, way_y)| format!("{name} ({way_x}, {way_y})")).collect();
        println!("entrances (enter a via …, exit a via …): {}", ways.join(", "));
    }
    let names: Vec<&str> = PLACES.iter().map(|(name, _)| *name).chain(tables.decors.keys().map(String::as_str)).collect();
    println!("known places: {}", names.join(", "));
    Ok(())
}

pub fn show_images(context: &Context, asked: &[&String]) -> Result<(), String> {
    let mut by_category: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let tables = context.tables();
    for (name, picture) in &tables.pictures {
        by_category.entry(&picture.category).or_default().push(name);
    }
    if let Some(unknown) = asked.iter().find(|word| !by_category.contains_key(word.as_str())) {
        let known: Vec<&str> = by_category.keys().copied().collect();
        return Err(format!("unknown category: {unknown} ({})", known.join(", ")));
    }
    for (category, names) in by_category {
        if asked.is_empty() || asked.iter().any(|word| *word == category) {
            println!("{category}: {}", names.join(", "));
        }
    }
    Ok(())
}

fn backdrop(context: &Context, word: &str) -> Result<(u16, RgbImage), String> {
    let (level, _, _) = place(context, word)?;
    let file = game_file(&context.data, &format!("backdrops/{level}.png"));
    let picture = image::open(&file).map_err(|error| format!("{}: {error}", file.display()))?;
    Ok((level, picture.to_rgb8()))
}

/// A place twice as big, a line every 50 pixels with its number.
pub fn grid(context: &Context, word: &str) -> Result<(), String> {
    const STEP: u32 = 50;
    let (_, picture) = backdrop(context, word)?;
    let (wide, high) = picture.dimensions();
    let mut big = image::imageops::resize(&picture, wide * 2, high * 2, image::imageops::FilterType::Nearest);
    let red = image::Rgb([255, 40, 40]);
    for x in (0..wide).step_by(STEP as usize) {
        (0..big.height()).for_each(|y| big.put_pixel(x * 2, y, red));
        write_on(&mut big, (x * 2 + 1, 1), &x.to_string(), 1);
    }
    for y in (0..high).step_by(STEP as usize) {
        (0..big.width()).for_each(|x| big.put_pixel(x, y * 2, red));
        write_on(&mut big, (1, y * 2 + 1), &y.to_string(), 1);
    }
    let out = out_folder(context, word)?.join("grid.png");
    big.save(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    println!("{wide} x {high}: {}", out.display());
    Ok(())
}

/// A place, what cannot be walked on in red.
pub fn ground(context: &Context, word: &str) -> Result<(), String> {
    let (level, mut picture) = backdrop(context, word)?;
    match context.tables().grounds.get(&level) {
        Some(rows) => {
            let (wide, high) = picture.dimensions();
            for (tile_y, row) in rows.rows().iter().enumerate() {
                for (tile_x, _) in row.iter().enumerate().filter(|(_, solid)| **solid) {
                    for y in (0..8).filter(|y| tile_y as u32 * 8 + y < high) {
                        for x in (0..8).filter(|x| (x + y) % 3 == 0 && tile_x as u32 * 8 + x < wide) {
                            picture.put_pixel(tile_x as u32 * 8 + x, tile_y as u32 * 8 + y, image::Rgb([255, 0, 0]));
                        }
                    }
                }
            }
        }
        None => println!("no ground known here: library/tables/grounds.txt, then decor game_grounds"),
    }
    let out = out_folder(context, word)?.join("ground.png");
    picture.save(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    println!("{}", out.display());
    Ok(())
}
