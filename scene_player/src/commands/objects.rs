//! The things a film can use, told and shown on a sheet.

use std::sync::Arc;

use ::film::Context;
use image::{Rgba, RgbaImage};
use preview::art::Art;
use preview::looks::{Look, picture_of};

use ::film::tables::Tables as FilmTables;

use super::{out_folder, pictures::write_on};

/// How many a line of the sheet; how big a place on it.
const COLUMNS: u32 = 12;
const CELL: u32 = 56;

/// A thing listed: its word, what it is, what shows it.
struct Listed {
    word: String,
    kind: &'static str,
    number: Option<u16>,
    look: Look,
}

/// Those a word asks: with no word, every one that has a word of its own (the
/// game's other props by a number or a part of their name).
fn asked(word: &str, known: &FilmTables) -> Vec<Listed> {
    let key = ::film::words::plain(word);
    let number: Option<u16> = key.trim_start_matches('o').parse().ok();
    let fits = |name: &str| !key.is_empty() && name.contains(&key);
    let mut all = Vec::new();
    for (name, (file, _)) in known.drawings.iter().filter(|(name, _)| key.is_empty() || fits(name)) {
        all.push(Listed { word: name.clone(), kind: "image", number: None, look: Look::Drawing(file.clone()) });
    }
    let own_word = |name: &str| !name.contains(|letter: char| letter.is_ascii_digit());
    for (name, object) in known.props.iter().filter(|(name, object)| (key.is_empty() && own_word(name)) || fits(name) || number == Some(**object)) {
        all.push(Listed { word: name.clone(), kind: "prop", number: Some(*object), look: Look::Prop(*object) });
    }
    for (name, row) in known.items.iter().filter(|(name, _)| fits(name)) {
        let item: u16 = row.0.parse().unwrap_or(0);
        all.push(Listed { word: name.clone(), kind: "item", number: Some(item), look: Look::Item(item) });
    }
    all
}

/// What is listed, a picture each, on a sheet: each by its place in the list.
fn sheet(context: &Context, all: &[Listed], art: &Art, data: &str) -> Result<String, String> {
    let rows = (all.len() as u32).div_ceil(COLUMNS).max(1);
    let mut whole = RgbaImage::from_pixel(COLUMNS * CELL, rows * CELL, Rgba([72, 92, 72, 255]));
    for (at, listed) in all.iter().enumerate() {
        let (x, y) = (at as u32 % COLUMNS * CELL, at as u32 / COLUMNS * CELL);
        let fitted = picture_of(art, data, &listed.look).map(|picture| preview::looks::fitted(&picture, (CELL - 10) as usize));
        if let Some(picture) = fitted.and_then(|(wide, high, rgba)| RgbaImage::from_raw(wide as u32, high as u32, rgba)) {
            image::imageops::overlay(&mut whole, &picture, x as i64 + 1, y as i64 + 9);
        }
    }
    let mut flat = image::DynamicImage::ImageRgba8(whole).to_rgb8();
    for at in 0..all.len() as u32 {
        write_on(&mut flat, (at % COLUMNS * CELL, at / COLUMNS * CELL), &(at + 1).to_string(), 1);
    }
    let out = out_folder(context, "items")?.join("sheet.png");
    flat.save(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(out.display().to_string())
}

/// `scene_player items [word]`.
pub fn list(word: &str, context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    let art = Art::open_in(data, context.library.clone())?;
    let all = asked(word, &context.tables());
    if all.is_empty() {
        return Err(format!("no object for \"{word}\": scene_player items"));
    }
    for (at, listed) in all.iter().enumerate() {
        let number = listed.number.map_or(String::new(), |number| format!(" ({number})"));
        println!("{:>3}. {}: {}{number}", at + 1, listed.kind, listed.word);
    }
    println!("{} objects; sheet: {}", all.len(), sheet(context, &all, &art, data)?);
    if word.is_empty() {
        println!("And the game's items by their name (items apple), the other props by their number (items 186).");
    }
    Ok(())
}
