//! A dungeon's tileset, as the game's folder has it: its chunks of 24
//! pixels, and the table choosing one for a tile.

use assets::data;

use crate::images::Frame;

/// Side of a tileset's chunk, and how many stand in a row of its picture.
pub const CHUNK: usize = 24;
pub const CHUNKS_IN_ROW: usize = 32;

/// A tileset: its chunks, and the table choosing one from a tile's terrain
/// and the neighbours like it.
pub struct Tileset {
    /// By `terrain * 768 + neighbour_bits * 3 + variation` (terrain 0 wall,
    /// 1 secondary, 2 floor); for a whole picture (a boss's floor,
    /// [`Tileset::PICTURE`] long), the chunk at each place, row by row.
    pub chunk_for: Vec<u16>,
    pub chunks: Vec<Frame>,
}

impl Tileset {
    /// The places of a whole picture: 32 by 32 chunks from the floor's corner.
    pub const PICTURE: usize = 32 * 32;

    /// The chunk of a tile at a place of the floor, if it has one.
    pub fn chunk(&self, (x, y): (i32, i32), terrain: usize, neighbours: usize, variation: usize) -> Option<&Frame> {
        let chunk = match self.chunk_for.len() == Self::PICTURE {
            // (A whole picture: what it has there, whatever the tile is.)
            true if (0..32).contains(&x) && (0..32).contains(&y) => self.chunk_for[y as usize * 32 + x as usize],
            true => return None,
            false => *self.chunk_for.get(terrain * 768 + neighbours * 3 + variation)?,
        };
        self.chunks.get(chunk as usize)
    }
}

/// A tileset of the game, by its number, read from its folder.
pub fn read_tileset(folder: &str, id: u8) -> Option<Tileset> {
    let (count, chunk_for): (usize, Vec<u16>) = bincode::deserialize(&data::read(folder, &format!("tilesets/{id}.bin"))?).ok()?;
    let picture = image::open(data::path(folder, &format!("tilesets/{id}.png"))).ok()?.to_rgba8();
    let chunk = |at: usize| {
        let mut rgba = Vec::with_capacity(CHUNK * CHUNK * 4);
        for y in 0..CHUNK {
            let from = (((at / CHUNKS_IN_ROW) * CHUNK + y) * CHUNKS_IN_ROW * CHUNK + (at % CHUNKS_IN_ROW) * CHUNK) * 4;
            rgba.extend_from_slice(&picture.as_raw()[from..from + CHUNK * 4]);
        }
        Frame::still(CHUNK, CHUNK, (CHUNK as i32 / 2, CHUNK as i32 / 2), rgba)
    };
    Some(Tileset { chunk_for, chunks: (0..count).map(chunk).collect() })
}
