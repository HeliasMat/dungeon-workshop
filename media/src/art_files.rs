//! The pictures of the game's folder, read: sprites, effects, status icons,
//! places. Whoever shows them makes of them what it needs.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use assets::tables::EffectRow;

use assets::data;

/// A picture of a sprite or an effect.
#[derive(Clone, Debug)]
pub struct Cell {
    /// Its 60ths of a second.
    pub ticks: u8,
    pub wide: u32,
    pub high: u32,
    /// Where a shadow lies under it, from its middle.
    pub shadow: (i32, i32),
    pub rgba: Vec<u8>,
}

pub use assets::tables::Sheet;

/// The animations' files, by number and animation.
pub type Sheets = HashMap<(u16, u8), Sheet>;

/// A cell of a picture, by its column and row.
fn cell(picture: &image::RgbaImage, column: u32, row: u32, wide: u32, high: u32) -> Option<Vec<u8>> {
    let mut rgba = Vec::with_capacity((wide * high * 4) as usize);
    for y in row * high..(row + 1) * high {
        let from = ((y * picture.width() + column * wide) * 4) as usize;
        rgba.extend_from_slice(picture.as_raw().get(from..from + (wide * 4) as usize)?);
    }
    Some(rgba)
}

/// The monsters' sheets (`sprites.txt`).
pub fn sheets(folder: &str) -> Sheets {
    assets::tables::sheets(folder, "sprites.txt").into_iter().map(|row| ((row.number, row.animation), row.sheet)).collect()
}

/// The scenes' things' (`props.txt`), and which of them lie on the ground.
pub fn prop_sheets(folder: &str) -> (Sheets, HashSet<u16>) {
    let rows = assets::tables::sheets(folder, "props.txt");
    let flats = rows.iter().filter(|row| row.flat).map(|row| row.number).collect();
    (rows.into_iter().map(|row| ((row.number, row.animation), row.sheet)).collect(), flats)
}

/// One of someone's animations, `[way faced][step]`.
pub fn animation(folder: &str, kind: &str, sheets: &Sheets, number: u16, animation: u8) -> Option<Vec<Vec<Cell>>> {
    let sheet = sheets.get(&(number, animation))?;
    let picture = image::open(data::path(folder, &format!("{kind}/{number}_{animation}.png"))).ok()?.to_rgba8();
    let step = |way: u32, step: u32| {
        let ticks = sheet.durations.get(step as usize).copied().unwrap_or(8).max(1);
        Some(Cell { ticks, wide: sheet.wide, high: sheet.high, shadow: sheet.shadow, rgba: cell(&picture, step, way, sheet.wide, sheet.high)? })
    };
    Some((0..8).map(|way| (0..sheet.durations.len() as u32).filter_map(|at| step(way, at)).collect()).collect())
}

/// An effect of the game: its steps by way faced, and how it plays.
#[derive(Clone, Debug)]
pub struct EffectFile {
    pub ways: Vec<Vec<Cell>>,
    pub ticks: f32,
    pub repeats: bool,
    pub screen: bool,
    pub sound: u16,
    pub point: u8,
}

/// An effect, from its line of `effects.txt`.
pub fn effect_file(folder: &str, effect: &EffectRow) -> Option<EffectFile> {
    let (wide, high) = (effect.wide, effect.high);
    let picture = image::open(data::path(folder, &format!("effects/{}.png", effect.id))).ok()?.to_rgba8();
    let step = |way: u32, (at, &ticks): (usize, &u8)| Some(Cell { ticks, wide, high, shadow: (0, 0), rgba: cell(&picture, at as u32, way, wide, high)? });
    Some(EffectFile {
        ways: (0..effect.ways).map(|way| effect.durations.iter().enumerate().filter_map(|at| step(way, at)).collect()).collect(),
        ticks: effect.ticks as f32,
        repeats: effect.repeats,
        screen: effect.screen,
        sound: effect.sound,
        point: effect.point,
    })
}

/// The pictures of each status icon, by name (`status.txt`).
pub fn status_icons(folder: &str) -> HashMap<String, Vec<(u32, u32, Vec<u8>)>> {
    let mut icons = HashMap::new();
    for status in assets::tables::statuses(folder) {
        let (wide, high) = (status.wide, status.high);
        let Ok(strip) = image::open(data::path(folder, &format!("status/{}.png", status.name))) else {
            continue;
        };
        let strip = strip.to_rgba8();
        icons.insert(status.name, (0..status.frames).filter_map(|at| Some((wide, high, cell(&strip, at, 0, wide, high)?))).collect());
    }
    icons
}

/// What a place's files tell of it: its size, how fast its pictures go,
/// how many files of each kind (`f` the pictures of the layer walked on,
/// `u` those of the layer walked under, then, a byte a pixel, `i` and `v`
/// which palette and colour each pixel of those shows and `l` the same for
/// each layer alone), its palettes, and those that change with time:
/// palette, 60ths of a second a step, its colours at each step.
#[derive(Serialize, Deserialize)]
pub struct GroundInfo {
    pub width: usize,
    pub height: usize,
    pub frame_seconds: f32,
    pub counts: [usize; 5],
    pub palettes: Vec<[[u8; 3]; 16]>,
    pub cycles: Vec<Cycle>,
}

/// Shadows are black ovals on the ground (measured on the original for
/// the middle size: 14 pixels by 6); each row's width, by size.
// Known gap: the small and the large ones are by eye.
pub const SHADOWS: [&[usize]; 3] = [&[4, 8, 10, 8, 4], &[6, 12, 14, 14, 12, 6], &[10, 18, 22, 22, 22, 18, 10]];

/// A palette that changes with time: which, 60ths of a second a step, its
/// colours at each step.
pub type Cycle = (usize, u16, Vec<[[u8; 3]; 16]>);

/// The kinds of a place's files, by their letter.
pub const GROUND_KINDS: [char; 5] = ['f', 'u', 'i', 'v', 'l'];

/// A place's pictures, of each kind in the order of [`GROUND_KINDS`].
pub struct GroundFile {
    pub info: GroundInfo,
    pub pictures: [Vec<Vec<u8>>; 5],
}

/// A place's files, if it has any.
pub fn ground_file(folder: &str, level: u16) -> Option<GroundFile> {
    let info: GroundInfo = bincode::deserialize(&data::read(folder, &format!("grounds/info/{level}.bin"))?).ok()?;
    let pictures = |kind: usize| -> Vec<Vec<u8>> {
        let picture = |at: usize| {
            let picture = image::open(data::path(folder, &format!("grounds/pictures/{level}_{}{at}.png", GROUND_KINDS[kind]))).ok()?;
            Some(if kind < 2 { picture.to_rgba8().into_raw() } else { picture.to_luma8().into_raw() })
        };
        (0..info.counts[kind]).filter_map(picture).collect()
    };
    let pictures = std::array::from_fn(pictures);
    Some(GroundFile { info, pictures })
}

pub use assets::tables::solid_rows;
