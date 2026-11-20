//! The game's places as the decors read them, from the game's files (data/,
//! its layers, as the game reads them): the pictures of each frame (`f`,
//! `u` for the layer walked under), which palette colour each pixel shows
//! (`i`, `v`) for the colours that change with time, its walls (ground.txt,
//! the game's grid where it has one).

use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::fault::Diagnostic;
use crate::pixels::{Grid, Mask, Picture, TILE};

/// A palette that changes with time: its number, how many 60ths a step, the steps.
pub type Cycle = (u64, (u16, Vec<Vec<u8>>));

/// What a place's info file says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Info {
    pub width: u64,
    pub height: u64,
    pub seconds: f32,
    pub counts: [u64; 5],
    /// Its palettes, 48 bytes each.
    pub palettes: Vec<Vec<u8>>,
    /// The palettes that change with time, in the order the file has them:
    /// each one's number, how many 60ths a step, the steps.
    pub cycles: Vec<Cycle>,
}

impl Info {
    /// As `media/src/art_files.rs` reads it (bincode).
    pub fn read(data: &[u8]) -> Option<Info> {
        let mut at = 0;
        let mut take = |count: usize| -> Option<&[u8]> {
            let part = data.get(at..at + count)?;
            at += count;
            Some(part)
        };
        let word = |bytes: &[u8]| u64::from_le_bytes(bytes.try_into().unwrap_or_default());
        let (width, height) = (word(take(8)?), word(take(8)?));
        let seconds = f32::from_le_bytes(take(4)?.try_into().ok()?);
        let mut counts = [0u64; 5];
        for count in &mut counts {
            *count = word(take(8)?);
        }
        let many = word(take(8)?) as usize;
        let palettes = (0..many).map(|_| take(48).map(<[u8]>::to_vec)).collect::<Option<Vec<_>>>()?;
        let mut cycles = Vec::new();
        if let Some(count) = take(8) {
            for _ in 0..word(count) {
                let palette = word(take(8)?);
                let ticks = u16::from_le_bytes(take(2)?.try_into().ok()?);
                let steps = word(take(8)?) as usize;
                let colours = (0..steps).map(|_| take(48).map(<[u8]>::to_vec)).collect::<Option<Vec<_>>>()?;
                cycles.retain(|(known, _): &Cycle| *known != palette);
                cycles.push((palette, (ticks, colours)));
            }
        }
        Some(Info { width, height, seconds, counts, palettes, cycles })
    }

    /// Written as the game reads it, its cycles by their palette.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(self.width.to_le_bytes());
        out.extend(self.height.to_le_bytes());
        out.extend(self.seconds.to_le_bytes());
        for count in self.counts {
            out.extend(count.to_le_bytes());
        }
        out.extend((self.palettes.len() as u64).to_le_bytes());
        for palette in &self.palettes {
            out.extend(palette);
        }
        out.extend((self.cycles.len() as u64).to_le_bytes());
        let mut cycles: Vec<&Cycle> = self.cycles.iter().collect();
        cycles.sort_by_key(|(palette, _)| *palette);
        for (palette, (ticks, steps)) in cycles {
            out.extend(palette.to_le_bytes());
            out.extend(ticks.to_le_bytes());
            out.extend((steps.len() as u64).to_le_bytes());
            for step in steps {
                out.extend(step);
            }
        }
        out
    }
}

/// A place of the game, read once.
pub struct Source {
    pub level: i64,
    pub info: Info,
    pub frames: Vec<Picture>,
    /// Its upper layer (one see-through frame if it has none).
    pub upper: Vec<Picture>,
    pub indices: Option<Vec<Grid<u8>>>,
    pub upper_indices: Option<Vec<Grid<u8>>>,
    /// What is seen, the upper layer over the lower, frame by frame; and which
    /// palette colour each pixel of that shows.
    pub merged: Vec<Picture>,
    pub merged_index: Option<Vec<Grid<u8>>>,
    pub walls: Option<Mask>,
    tile_counts: OnceCell<TileCounts>,
}

/// How often each whole tile of a place is seen, in the order first seen.
#[derive(Default)]
pub struct TileCounts {
    pub order: Vec<(Vec<u8>, usize)>,
    at: HashMap<Vec<u8>, usize>,
}

impl TileCounts {
    pub fn get(&self, tile: &[u8]) -> Option<usize> {
        self.at.get(tile).map(|at| self.order[*at].1)
    }

    pub fn total(&self) -> usize {
        self.order.iter().map(|(_, count)| count).sum()
    }
}

/// A picture of the game's files, in colours and how much each is seen.
pub fn picture_at(path: &str) -> Option<Picture> {
    let read = image::open(path).ok()?.to_rgba8();
    let (w, h) = (read.width() as usize, read.height() as usize);
    Some(Grid { w, h, cells: read.pixels().map(|pixel| pixel.0).collect() })
}

/// A picture of the game's files, as one number a pixel.
fn gray_at(path: &str) -> Option<Grid<u8>> {
    let read = image::open(path).ok()?.to_luma8();
    let (w, h) = (read.width() as usize, read.height() as usize);
    Some(Grid { w, h, cells: read.into_raw() })
}

impl Source {
    fn read(game: &Game, level: i64) -> Result<Source, Diagnostic> {
        let file = |name: String| assets::data::path(&game.folder, &name);
        let info = assets::data::read(&game.folder, &format!("grounds/info/{level}.bin"))
            .and_then(|data| Info::read(&data))
            .ok_or_else(|| Diagnostic::said(format!("place {level}: no info")))?;
        let counts = info.counts;
        let pictures = |kind: &str, count: u64| -> Result<Vec<Picture>, Diagnostic> {
            (0..count)
                .map(|at| {
                    picture_at(&file(format!("grounds/pictures/{level}_{kind}{at}.png")))
                        .ok_or_else(|| Diagnostic::said(format!("place {level}: no image {kind}{at}")))
                })
                .collect()
        };
        let grays = |kind: &str, count: u64| -> Result<Vec<Grid<u8>>, Diagnostic> {
            (0..count)
                .map(|at| {
                    gray_at(&file(format!("grounds/pictures/{level}_{kind}{at}.png")))
                        .ok_or_else(|| Diagnostic::said(format!("place {level}: no image {kind}{at}")))
                })
                .collect()
        };
        let frames = pictures("f", counts[0].max(1))?;
        let upper = if counts[1] > 0 { pictures("u", counts[1])? } else { vec![Picture::new(frames[0].w, frames[0].h)] };
        let changing = !info.cycles.is_empty();
        let indices = if counts[2] > 0 && changing { Some(grays("i", counts[2])?) } else { None };
        let upper_indices = if counts[3] > 0 && changing { Some(grays("v", counts[3])?) } else { None };
        let mut merged = frames.clone();
        let mut merged_index = indices.clone();
        for frame in 0..merged.len() {
            let over = &upper[frame % upper.len()];
            let blank = Grid::new(over.w, over.h);
            let upper_index = upper_indices.as_ref().map_or(&blank, |all| &all[frame % all.len()]);
            for at in 0..over.cells.len() {
                if over.cells[at][3] > 0 {
                    merged[frame].cells[at] = over.cells[at];
                    if let Some(index) = &mut merged_index {
                        let count = index.len();
                        index[frame % count].cells[at] = upper_index.cells[at];
                    }
                }
            }
        }
        let walls = game.walls().get(&level).cloned();
        Ok(Source { level, info, frames, upper, indices, upper_indices, merged, merged_index, walls, tile_counts: OnceCell::new() })
    }

    /// What is seen, frame 0.
    pub fn picture(&self) -> &Picture {
        &self.merged[0]
    }

    /// The colours seen at x, y, frame 0.
    pub fn rgb(&self, x: usize, y: usize) -> [u8; 3] {
        let [r, g, b, _] = self.picture().get(x, y);
        [r, g, b]
    }

    /// How often each whole 8x8 tile (its colours, row by row) is seen, frame 0.
    pub fn tile_counts(&self) -> &TileCounts {
        self.tile_counts.get_or_init(|| {
            let mut counts = TileCounts::default();
            for tile in tile_bytes(self.picture()) {
                match counts.at.get(&tile) {
                    Some(at) => counts.order[*at].1 += 1,
                    None => {
                        counts.at.insert(tile.clone(), counts.order.len());
                        counts.order.push((tile, 1));
                    }
                }
            }
            counts
        })
    }
}

/// The colours of the tile at x, y of a picture, row by row (RGB).
pub fn tile_at(picture: &Picture, x: usize, y: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(TILE * TILE * 3);
    for dy in 0..TILE {
        for dx in 0..TILE {
            out.extend_from_slice(&picture.get(x + dx, y + dy)[..3]);
        }
    }
    out
}

/// Each whole tile of a picture, row by row.
pub fn tile_bytes(picture: &Picture) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut y = 0;
    while y + TILE <= picture.h {
        let mut x = 0;
        while x + TILE <= picture.w {
            out.push(tile_at(picture, x, y));
            x += TILE;
        }
        y += TILE;
    }
    out
}

/// The game's files, and its places read from them, once each.
pub struct Game {
    pub folder: String,
    sources: std::cell::RefCell<HashMap<i64, Rc<Source>>>,
    walls: OnceCell<HashMap<i64, Mask>>,
}

impl Game {
    pub fn new(folder: &str) -> Game {
        Game { folder: folder.to_string(), sources: Default::default(), walls: OnceCell::new() }
    }

    /// The places read, forgotten (read again when asked): to go through
    /// them all without keeping them all.
    pub fn forget(&self) {
        self.sources.borrow_mut().clear();
    }

    /// A place of the game, read once.
    pub fn source(&self, level: i64) -> Result<Rc<Source>, Diagnostic> {
        if let Some(known) = self.sources.borrow().get(&level) {
            return Ok(known.clone());
        }
        let read = Rc::new(Source::read(self, level)?);
        self.sources.borrow_mut().insert(level, read.clone());
        Ok(read)
    }

    /// A table of the game, by its fields: the lists merged as the game merges them.
    pub fn rows(&self, file: &str) -> Vec<Vec<String>> {
        assets::tables::rows_of(&self.folder, file)
    }

    /// Each place's walls, rows of tiles (ground.txt).
    pub fn walls(&self) -> &HashMap<i64, Mask> {
        self.walls.get_or_init(|| {
            let mut found = HashMap::new();
            for ground in assets::tables::grounds(&self.folder) {
                let lines: Vec<&str> = ground.rows.split('/').collect();
                let wide = lines.iter().map(|line| line.len()).max().unwrap_or(0);
                let mut mask = Mask::new(wide, lines.len());
                for (y, line) in lines.iter().enumerate() {
                    for (x, letter) in line.chars().enumerate() {
                        mask.set(x, y, letter == '1');
                    }
                }
                found.insert(ground.level as i64, mask);
            }
            found
        })
    }
}
