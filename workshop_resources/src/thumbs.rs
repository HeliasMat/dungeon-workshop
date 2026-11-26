//! The pictures of the Pokémon's cards, made on a thread of their own: each
//! one's portrait and its body standing (the first image of its idle
//! animation, facing down, cut to what is drawn). The bodies are kept in
//! `target/workshop/pictures/catalog/` between launches (made again when
//! their sheet is newer): a catalog of 1,500 cards opens at once.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;
use workshop_base::session::{Session, Sheets};

/// Pictures made into textures in one frame, at most.
const TAKEN_EACH_FRAME: usize = 40;
/// The idle animation's sheet, else the walk's.
const IDLE: u8 = 1;
const WALK: u8 = 0;

/// A card's pictures: its portrait, its body standing.
#[derive(Clone, Default)]
pub struct Thumb {
    pub face: Option<egui::TextureHandle>,
    pub body: Option<egui::TextureHandle>,
}

/// What the thread made of a monster: its portrait's pixels, its body's (each: width, height, RGBA).
type Made = (i64, Option<(u32, u32, Vec<u8>)>, Option<(u32, u32, Vec<u8>)>);

/// The thread, what was asked of it, what is made.
pub struct Thumbs {
    ask: Option<Sender<i64>>,
    made: Option<Receiver<Made>>,
    asked: HashSet<i64>,
    pub kept: HashMap<i64, Thumb>,
}

impl Thumbs {
    pub fn new() -> Thumbs {
        Thumbs { ask: None, made: None, asked: HashSet::new(), kept: HashMap::new() }
    }

    /// Everything made again (new pictures imported).
    pub fn forget(&mut self) {
        self.asked.clear();
        self.kept.clear();
    }

    /// A monster's card pictures: made if they are, else asked of the thread.
    pub fn of(&mut self, session: &Session, number: i64) -> Option<&Thumb> {
        if number < 0 {
            return None;
        }
        if !self.kept.contains_key(&number) && self.asked.insert(number) {
            if self.ask.is_none() {
                self.start(session);
            }
            if let Some(ask) = &self.ask {
                _ = ask.send(number);
            }
        }
        self.kept.get(&number)
    }

    /// What the thread made, made textures (so many each frame); whether more is coming.
    pub fn take(&mut self, context: &egui::Context) -> bool {
        let Some(made) = &self.made else { return false };
        for (number, face, body) in made.try_iter().take(TAKEN_EACH_FRAME) {
            let texture = |pixels: Option<(u32, u32, Vec<u8>)>, what: &str| {
                pixels.map(|(wide, high, rgba)| {
                    let image = egui::ColorImage::from_rgba_unmultiplied([wide as usize, high as usize], &rgba);
                    context.load_texture(format!("card {what} {number}"), image, egui::TextureOptions::NEAREST)
                })
            };
            self.kept.insert(number, Thumb { face: texture(face, "face"), body: texture(body, "body") });
        }
        let waiting = self.asked.len() > self.kept.len();
        if waiting {
            context.request_repaint_after(std::time::Duration::from_millis(30));
        }
        waiting
    }

    fn start(&mut self, session: &Session) {
        let (ask, asked) = std::sync::mpsc::channel::<i64>();
        let (made, taken) = std::sync::mpsc::channel::<Made>();
        let data = session.data().to_string();
        let kept = session.root().join("target").join("workshop").join("pictures").join("catalog");
        let sheets = super::pictures::sprite_sheets(session);
        std::thread::spawn(move || {
            _ = std::fs::create_dir_all(&kept);
            for number in asked {
                let face = read(Path::new(&assets::data::path(&data, &format!("portraits/{number}_0.png"))));
                let body = standing(&data, &kept, &sheets, number);
                if made.send((number, face, body)).is_err() {
                    return;
                }
            }
        });
        (self.ask, self.made) = (Some(ask), Some(taken));
    }
}

/// A picture's pixels.
fn read(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let picture = image::open(path).ok()?.to_rgba8();
    Some((picture.width(), picture.height(), picture.into_raw()))
}

/// A monster's body standing: kept, or cut from its sheet and kept.
fn standing(data: &str, kept: &Path, sheets: &Arc<Sheets>, number: i64) -> Option<(u32, u32, Vec<u8>)> {
    let (file, sheet) = [IDLE, WALK].into_iter().find_map(|file| Some((file, sheets.get(&(u16::try_from(number).ok()?, file))?)))?;
    let source = PathBuf::from(assets::data::path(data, &format!("sprites/{number}_{file}.png")));
    let made = kept.join(format!("{number}.png"));
    let newer = |made: &Path| -> bool {
        let time = |path: &Path| std::fs::metadata(path).and_then(|meta| meta.modified()).ok();
        matches!((time(made), time(&source)), (Some(made), Some(source)) if made >= source)
    };
    if newer(&made)
        && let Some(found) = read(&made)
    {
        return Some(found);
    }
    let whole = image::open(&source).ok()?.to_rgba8();
    let cut = first_cell(&whole, sheet.wide, sheet.high)?;
    _ = cut.save(&made);
    Some((cut.width(), cut.height(), cut.into_raw()))
}

/// A sheet's first image, facing down, cut to what is drawn in it.
pub fn first_cell(whole: &image::RgbaImage, wide: u32, high: u32) -> Option<image::RgbaImage> {
    let (wide, high) = (wide.min(whole.width()), high.min(whole.height()));
    if wide == 0 || high == 0 {
        return None;
    }
    let cell = image::imageops::crop_imm(whole, 0, 0, wide, high).to_image();
    let drawn: Vec<(u32, u32)> = cell.enumerate_pixels().filter(|(_, _, pixel)| pixel.0[3] > 0).map(|(x, y, _)| (x, y)).collect();
    let (left, right) = (drawn.iter().map(|(x, _)| *x).min()?, drawn.iter().map(|(x, _)| *x).max()?);
    let (top, bottom) = (drawn.iter().map(|(_, y)| *y).min()?, drawn.iter().map(|(_, y)| *y).max()?);
    Some(image::imageops::crop_imm(&cell, left, top, right - left + 1, bottom - top + 1).to_image())
}
