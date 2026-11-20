//! The pictures of the things: read from their files, the Pokémon's sheets
//! and animations, the effects, painted in a room; the faces.

use std::path::PathBuf;

use eframe::egui::{self, Color32, FontId, Rect, vec2};

use super::{Item, Resources, Sheet, Shelf};
use crate::decors::Decors;
use crate::making::fitted;
use crate::session::{Session, Sheets};
use crate::theme;

/// The widest a picture is made a texture (wider ones are cut: some cards take no more).
const LARGEST: u32 = 8192;

/// A picture read: whole, its steps side by side.
pub(super) fn read_png(context: &egui::Context, path: &PathBuf, name: &str, step: u32) -> Option<egui::TextureHandle> {
    let mut picture = image::open(path).ok()?.to_rgba8();
    // (Too wide for a texture (an effect of many steps side by side): as many whole steps as fit.)
    if picture.width() > LARGEST || picture.height() > LARGEST {
        let step = step.clamp(1, LARGEST);
        let wide = (picture.width().min(LARGEST) / step * step).max(step.min(picture.width()));
        picture = image::imageops::crop_imm(&picture, 0, 0, wide, picture.height().min(LARGEST)).to_image();
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([picture.width() as usize, picture.height() as usize], picture.as_raw());
    let options =
        egui::TextureOptions { magnification: egui::TextureFilter::Nearest, minification: egui::TextureFilter::Linear, ..egui::TextureOptions::NEAREST };
    Some(context.load_texture(name, image, options))
}

/// The sheets of a list (`sprites.txt`, `props.txt`), by number and animation: read once a session.
fn sheets_of(session: &Session, file: &'static str) -> std::sync::Arc<Sheets> {
    let kept = if file == "sprites.txt" { &session.sprites } else { &session.props };
    kept.get(|| assets::tables::sheets(session.data(), file).into_iter().map(|row| ((row.number, row.animation), row.sheet)).collect())
}

/// A body's animations the game draws: their files, their words (`Attack`, `Sleep`…).
pub(super) fn animations_of(session: &Session, number: i64) -> Vec<(u8, String)> {
    let sprites = sheets_of(session, "sprites.txt");
    scenes::vocabulary::POKEMON
        .animations
        .iter()
        .filter_map(|(name, group)| {
            let file = u8::try_from(*group).ok()?.checked_add(5)?;
            sprites.contains_key(&(number as u16, file)).then(|| (file, name.to_string()))
        })
        .collect()
}

/// A body's animation as a sheet (its eight ways in rows).
pub(super) fn body_sheet(session: &Session, context: &egui::Context, number: i64, file: u8) -> Option<Sheet> {
    let sprites = sheets_of(session, "sprites.txt");
    let sheet = sprites.get(&(number as u16, file))?;
    let path = PathBuf::from(assets::data::path(session.data(), &format!("sprites/{number}_{file}.png")));
    let texture = read_png(context, &path, &format!("body:{number}:{file}"), sheet.wide)?;
    fitting(Sheet { texture, cell: vec2(sheet.wide as f32, sheet.high as f32), count: sheet.durations.len().max(1), ticks: sheet.durations.clone() })
}

/// The game's effects, read once a session.
pub(super) fn effects(session: &Session) -> std::sync::Arc<Vec<assets::tables::EffectRow>> {
    session.effects.get(|| assets::tables::effects(session.data()))
}

/// A sheet with no more steps than its texture holds (cut when it was too wide).
fn fitting(sheet: Sheet) -> Option<Sheet> {
    let held = (sheet.texture.size_vec2().x / sheet.cell.x.max(1.0)).floor().max(1.0) as usize;
    Some(Sheet { count: sheet.count.min(held), ..sheet })
}

/// A thing's picture, as the game (or we) drew it.
pub(super) fn sheet_of(session: &Session, context: &egui::Context, item: &Item) -> Option<Sheet> {
    let data = session.data().to_string();
    let tables = session.tables();
    let key = item.key();
    match item.shelf {
        Shelf::Dungeons | Shelf::Bosses => {
            let path = PathBuf::from(assets::data::path(&data, &format!("portraits/{}_0.png", item.number)));
            let texture = read_png(context, &path, &key, 0)?;
            let cell = texture.size_vec2();
            Some(Sheet { texture, cell, count: 1, ticks: vec![60] })
        }
        Shelf::Pokemon | Shelf::Variants => {
            // (Its portrait; none in the game: its body, as it walks.)
            let path = PathBuf::from(assets::data::path(&data, &format!("portraits/{}_0.png", item.number)));
            if let Some(texture) = read_png(context, &path, &key, 0) {
                let cell = texture.size_vec2();
                return Some(Sheet { texture, cell, count: 1, ticks: vec![60] });
            }
            let sprites = sheets_of(session, "sprites.txt");
            let sheet = sprites.get(&(item.number as u16, 0))?;
            let texture = read_png(context, &PathBuf::from(assets::data::path(&data, &format!("sprites/{}_0.png", item.number))), &key, sheet.wide)?;
            fitting(Sheet { texture, cell: vec2(sheet.wide as f32, sheet.high as f32), count: sheet.durations.len().max(1), ticks: sheet.durations.clone() })
        }
        Shelf::Drawn => {
            let count = item.number.max(1) as usize;
            let whole = image::image_dimensions(item.file.as_ref()?).ok()?;
            let step = whole.0 / count as u32;
            let texture = read_png(context, item.file.as_ref()?, &key, step)?;
            fitting(Sheet { texture, cell: vec2(step as f32, whole.1.min(LARGEST) as f32), count, ticks: vec![12; count] })
        }
        Shelf::Props => {
            let number = tables.props.get(&item.word)?;
            let props = sheets_of(session, "props.txt");
            let sheet = props.get(&(*number, 0))?;
            let texture = read_png(context, &PathBuf::from(assets::data::path(&data, &format!("props/{number}_0.png"))), &key, sheet.wide)?;
            fitting(Sheet { texture, cell: vec2(sheet.wide as f32, sheet.high as f32), count: sheet.durations.len().max(1), ticks: sheet.durations.clone() })
        }
        Shelf::Effects => effect_sheet(session, context, item.word.parse().ok()?, &key),
        Shelf::Attacks => effect_sheet(session, context, super::game_shelves::card_effect(session, item.number)?, &key),
        Shelf::Statuses => {
            let status = assets::tables::statuses(&data).into_iter().find(|status| status.name == item.word)?;
            let texture = read_png(context, &PathBuf::from(assets::data::path(&data, &format!("status/{}.png", status.name))), &key, status.wide)?;
            let count = status.frames.max(1) as usize;
            fitting(Sheet { texture, cell: vec2(status.wide as f32, status.high as f32), count, ticks: vec![STATUS_TICKS; count] })
        }
        Shelf::Portraits => {
            let face = super::game_shelves::portrait_face(&item.word)?;
            still(read_png(context, &PathBuf::from(assets::data::path(&data, &format!("portraits/{}_{face}.png", item.number))), &key, 0)?)
        }
        Shelf::Items => {
            let rgba = super::game_shelves::item_icon(session, item.number)?;
            let image = egui::ColorImage::from_rgba_unmultiplied([16, 16], &rgba);
            still(context.load_texture(key, image, egui::TextureOptions::NEAREST))
        }
        Shelf::Backdrops => still(read_within(context, &PathBuf::from(assets::data::path(&data, &format!("backdrops/{}.png", item.number))), &key, 480)?),
        Shelf::Tilesets => still(read_within(context, &PathBuf::from(assets::data::path(&data, &format!("tilesets/{}.png", item.number))), &key, 768)?),
        _ => None,
    }
}

/// The 60ths each image of a status icon stays, as the stage plays them.
const STATUS_TICKS: u8 = 4;

/// A picture of one image only.
fn still(texture: egui::TextureHandle) -> Option<Sheet> {
    let cell = texture.size_vec2();
    Some(Sheet { texture, cell, count: 1, ticks: vec![60] })
}

/// A large picture read, made smaller (by whole pixels dropped) so that it is no wider nor higher than that.
fn read_within(context: &egui::Context, path: &PathBuf, name: &str, largest: u32) -> Option<egui::TextureHandle> {
    let mut picture = image::open(path).ok()?.to_rgba8();
    let side = picture.width().max(picture.height());
    if side > largest {
        let (wide, high) = ((picture.width() * largest / side).max(1), (picture.height() * largest / side).max(1));
        picture = image::imageops::resize(&picture, wide, high, image::imageops::FilterType::Nearest);
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([picture.width() as usize, picture.height() as usize], picture.as_raw());
    Some(context.load_texture(name, image, egui::TextureOptions::NEAREST))
}

/// One of the game's effects as a sheet, by its number in `effects.txt`.
pub(super) fn effect_sheet(session: &Session, context: &egui::Context, id: u16, key: &str) -> Option<Sheet> {
    let effects = effects(session);
    let effect = effects.iter().find(|effect| effect.id == id)?;
    let texture = read_png(context, &PathBuf::from(assets::data::path(session.data(), &format!("effects/{}.png", effect.id))), key, effect.wide)?;
    fitting(Sheet { texture, cell: vec2(effect.wide as f32, effect.high as f32), count: effect.durations.len().max(1), ticks: effect.durations.clone() })
}

/// A thing's picture painted in a room: its step; a piece of the decor by its art; a sound, its sign.
pub(super) fn paint_item(
    painter: &egui::Painter,
    resources: &mut Resources,
    decors: &mut Decors,
    context: &egui::Context,
    item: &Item,
    room: Rect,
    step: usize,
) {
    match item.shelf {
        Shelf::Songs | Shelf::Sounds => {
            let heard = resources.heard.as_deref() == Some(item.key().as_str());
            let side = room.width().min(room.height()) * 0.45;
            crate::icons::wave(painter, Rect::from_center_size(room.center(), vec2(side, side)), if heard { theme::ASSETS } else { theme::WEAK });
        }
        Shelf::Weather => match item.watched_film().map(|film| resources.examples.picture(context, &film)) {
            Some(Ok(Some(texture))) => {
                painter.image(
                    texture.id(),
                    fitted(texture.size_vec2(), room.shrink(4.0)),
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Some(Err(_)) => {
                painter.text(room.center(), egui::Align2::CENTER_CENTER, "—", FontId::proportional(theme::SMALL), theme::FAINT);
            }
            _ => {
                painter.text(room.center(), egui::Align2::CENTER_CENTER, "…", FontId::proportional(theme::SMALL), theme::FAINT);
            }
        },
        Shelf::Pieces => {
            if let Some(art) = decors.art_now(&item.word) {
                crate::decors::paint_thumb(painter, &art, room.shrink(4.0));
            }
        }
        _ => match resources.picture(context, item) {
            Some(sheet) => {
                painter.image(sheet.texture.id(), fitted(sheet.cell, room.shrink(4.0)), sheet.uv(step), Color32::WHITE);
            }
            None => {
                painter.text(room.center(), egui::Align2::CENTER_CENTER, "…", FontId::proportional(theme::SMALL), theme::FAINT);
            }
        },
    }
}

/// The faces a species has a portrait of (their numbers), the first first.
pub(super) fn faces_of(session: &Session, number: i64) -> Vec<u16> {
    let folder = session.data();
    let mut faces: Vec<u16> = (0..20).filter(|face| assets::data::has(folder, &format!("portraits/{number}_{face}.png"))).collect();
    if faces.is_empty() {
        faces.push(0);
    }
    faces
}

/// A face's word, as a film writes it.
pub(super) fn face_name(face: u16) -> String {
    scenes::vocabulary::POKEMON.faces.iter().find(|(_, number)| *number == face).map_or_else(|| face.to_string(), |(word, _)| (*word).to_string())
}
