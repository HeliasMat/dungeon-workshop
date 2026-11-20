//! How a floor looks: the picture of its tiles, made once, and everything
//! in the colours of its weather, as the original shows it: every colour
//! part goes through the weather's colour map.

use std::collections::HashMap;

use rules::dungeon::{FLOOR_HEIGHT, FLOOR_WIDTH, Floor, Pos, Tile, stack};
use rules::run::{Book, Dungeon};
use rules::tables::Tables;
use shell::art::{Art, CHUNK, Tileset};
use shell::images::{Frame, console_colours};

/// Bits of a tile's neighbours sharing its terrain: S, SE, E, NE, N, NW, W, SW.
const NEIGHBOURS: [(i32, i32); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];

pub const WIDE: usize = FLOOR_WIDTH as usize * CHUNK;
pub const HIGH: usize = FLOOR_HEIGHT as usize * CHUNK;

/// The floor's picture: as drawn, and in the weather's colours.
#[derive(Default)]
pub struct FloorPicture {
    /// The floor and the times its tiles changed it was drawn for, and the
    /// weather it is tinted for.
    drawn: Option<(u32, u32)>,
    tinted: Option<u8>,
    plain: Vec<u8>,
    pub shown: Vec<u8>,
}

fn terrain(floor: &Floor, at: Pos) -> usize {
    match floor.tile(at) {
        Tile::Wall => 0,
        Tile::Secondary => 1,
        Tile::Floor { .. } | Tile::Stairs { .. } => 2,
    }
}

/// Draws every tile's chunk, chosen from its terrain and its neighbours.
fn paint(floor: &Floor, tileset: Option<&Tileset>, pixels: &mut Vec<u8>) {
    pixels.clear();
    pixels.resize(WIDE * HIGH * 4, 0);
    let Some(tileset) = tileset else {
        return;
    };
    for at in Floor::positions() {
        let here = terrain(floor, at);
        let bits = NEIGHBOURS
            .iter()
            .enumerate()
            .filter(|(_, (dx, dy))| terrain(floor, Pos::new(at.x + dx, at.y + dy)) == here)
            .fold(0, |bits, (bit, _)| bits | 1 << bit);
        // (The original rolls the variation with the dungeon's RNG.)
        let variation = (at.x * 7 + at.y * 13) as usize % 3;
        let Some(chunk) = tileset.chunk((at.x, at.y), here, bits, variation) else {
            continue;
        };
        for row in 0..CHUNK {
            let to = ((at.y as usize * CHUNK + row) * WIDE + at.x as usize * CHUNK) * 4;
            pixels[to..to + CHUNK * 4].copy_from_slice(&chunk.rgba[row * CHUNK * 4..(row + 1) * CHUNK * 4]);
        }
    }
}

/// A floor alone, drawn with its tileset in clear weather, as the game shows
/// it (`WIDE` by `HIGH` pixels, RGBA): for an editor's look at a floor.
pub fn drawn(floor: &Floor, tileset: Option<&Tileset>) -> Vec<u8> {
    let mut pixels = Vec::new();
    paint(floor, tileset, &mut pixels);
    console_colours(&mut pixels);
    pixels
}

/// Every colour part looked up in a weather's map (256 RGBX entries).
fn remap(rgba: &mut [u8], map: Option<&Vec<u8>>) {
    if let Some(map) = map {
        for pixel in rgba.as_chunks_mut::<4>().0 {
            for part in 0..3 {
                pixel[part] = map[pixel[part] as usize * 4 + part];
            }
        }
    }
    console_colours(rgba);
}

impl FloorPicture {
    /// Forgotten: it is drawn again.
    pub fn stale(&mut self) {
        self.drawn = None;
    }

    /// Up to date with the dungeon.
    pub fn refresh(&mut self, dungeon: &Dungeon, book: &Book, art: &Art) {
        let key = (dungeon.generation, dungeon.repaints);
        let weather = dungeon.weather.current;
        if self.drawn != Some(key) {
            let tileset = dungeon.spec(book).and_then(|spec| art.tileset(spec.tileset));
            paint(&dungeon.floor, tileset.as_deref(), &mut self.plain);
            self.drawn = Some(key);
            self.tinted = None;
        }
        if self.tinted != Some(weather) {
            self.shown.clone_from(&self.plain);
            remap(&mut self.shown, book.tables.weather_colors.get(weather as usize));
            self.tinted = Some(weather);
        }
    }
}

/// What is drawn on a floor, in its weather's colours, each made the first
/// time it shows.
#[derive(Default)]
pub struct Looks {
    weather: Option<u8>,
    map: Option<Vec<u8>>,
    /// By where the picture is (the art keeps them until `forget_sprites`,
    /// and these go with it).
    frames: HashMap<*const Frame, Frame>,
    items: HashMap<u16, Option<Frame>>,
    traps: HashMap<usize, Option<Frame>>,
}

impl Looks {
    /// All made anew in another weather, or on a new floor.
    pub fn forget(&mut self) {
        self.frames.clear();
        self.items.clear();
        self.traps.clear();
        self.weather = None;
    }

    pub fn weather(&mut self, weather: u8, tables: &Tables) {
        if self.weather != Some(weather) {
            self.forget();
            self.weather = Some(weather);
            self.map = tables.weather_colors.get(weather as usize).cloned();
        }
    }

    pub fn frame(&mut self, frame: &Frame) -> &Frame {
        let map = self.map.as_ref();
        self.frames.entry(frame as *const Frame).or_insert_with(|| {
            let mut tinted = frame.clone();
            remap(&mut tinted.rgba, map);
            tinted
        })
    }

    fn still(picture: Option<&Vec<u8>>, side: usize, map: Option<&Vec<u8>>) -> Option<Frame> {
        let mut rgba = picture.filter(|picture| picture.len() == side * side * 4)?.clone();
        remap(&mut rgba, map);
        Some(Frame::still(side, side, (side as i32 / 2, side as i32 / 2), rgba))
    }

    /// An item's picture, of its number or of a pile.
    pub fn item(&mut self, item: u16, tables: &Tables) -> Option<&Frame> {
        let (id, map) = (stack::id(item), self.map.as_ref());
        self.items.entry(id).or_insert_with(|| Self::still(tables.item_art.get(id as usize), 16, map)).as_ref()
    }

    /// A trap's picture, or the stairs' (`Tables::stairs_tile`).
    pub fn trap(&mut self, kind: usize, tables: &Tables) -> Option<&Frame> {
        let map = self.map.as_ref();
        self.traps.entry(kind).or_insert_with(|| Self::still(tables.trap_art.get(kind), CHUNK, map)).as_ref()
    }
}
