//! WAN sprites (monsters, items, effects, the interface's cursors), stored
//! as SIR0 containers.
//!
//! A sprite is a set of images, one palette, "meta-frames" (groups of
//! positioned image pieces forming one picture) and animation groups made of
//! sequences (one per facing direction) of timed meta-frame references.
//!
//! Characters' sprites have 4-bit images, a table of offsets after their
//! meta-frames, and pieces that name their image. The interface's sprites
//! (`FONT/*.wan`, sprite type 0; worked out on `cursor.wan` and
//! `pagewait.wan`) differ: their images may be 8 bits a pixel, whose high
//! half is the palette slot the game loads their 16 colours in; they have no
//! offset table, the meta-frames ending where the sequences start; and
//! their pieces give a tile number in 128-byte units instead of an image.

use super::nds::RomError;
use super::px::decompress_pkdpx;
use crate::bytes::{offset_at, u16_le};

/// One image placed inside a meta-frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Piece {
    pub image: usize,
    /// Top-left corner, relative to the sprite's centre.
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
    pub flip_x: bool,
    pub flip_y: bool,
    pub palette: usize,
}

/// The markers of a step (`AnimFrame::flag`).
pub const HIT_FLAG: u8 = 2;
pub const RETURN_FLAG: u8 = 1;

#[derive(Clone, Copy, Debug)]
pub struct AnimFrame {
    /// In 1/60 s.
    pub duration: u8,
    /// Its markers: 2, the step a blow lands on; 1, the one it is back in
    /// place from (as SkyTemple reads them).
    pub flag: u8,
    pub meta_frame: usize,
    pub offset: (i16, i16),
    /// Where the character's shadow lies, from the sprite's centre.
    pub shadow: (i16, i16),
}

/// A rendered meta-frame.
pub struct WanPicture {
    pub width: usize,
    pub height: usize,
    /// Position of the sprite's centre inside the picture.
    pub origin: (i32, i32),
    pub rgba: Vec<u8>,
}

/// A picture of a sprite drawn from one texture: where its top-left corner
/// is from the sprite's centre, and the part of the texture it shows.
#[derive(Clone, Copy, Debug)]
struct Quad {
    left: i32,
    top: i32,
    u: usize,
    v: usize,
    width: usize,
    height: usize,
}

pub struct Wan {
    /// For the sprites drawn from one texture (type 3, the "WAT" of
    /// effects: worked out on the ROM's): the texture's width and its
    /// pixels as RGBA, and one part of it per picture.
    texture: Option<(usize, Vec<u8>)>,
    quads: Vec<Quad>,
    /// Images are 8 bits a pixel instead of 4.
    wide: bool,
    images: Vec<Vec<u8>>,
    palette: Vec<[u8; 3]>,
    meta_frames: Vec<Vec<Piece>>,
    /// Characters' sprites: by meta-frame, where its head, its two hands
    /// and its centre are (the table of offsets), as the game reads them.
    offsets: Vec<[(i16, i16); 4]>,
    /// `animations[group][direction]` is a sequence of frames.
    animations: Vec<Vec<Vec<AnimFrame>>>,
}

/// Width and height from the OAM shape (attr0 bits 14-15) and size
/// (attr1 bits 14-15).
const PIECE_SIZES: [[(usize, usize); 4]; 3] =
    [[(8, 8), (16, 16), (32, 32), (64, 64)], [(16, 8), (32, 8), (32, 16), (64, 32)], [(8, 16), (8, 32), (16, 32), (32, 64)]];

impl Wan {
    /// Entry `index` of a sprite pack such as `MONSTER/monster.bin`.
    pub fn from_pack(pack: &[u8], index: usize) -> Result<Self, RomError> {
        if index >= offset_at(pack, 4)? {
            return Err(RomError::Truncated);
        }
        let (off, len) = (offset_at(pack, 8 + index * 8)?, offset_at(pack, 12 + index * 8)?);
        let entry = pack.get(off..off + len).ok_or(RomError::Truncated)?;
        // `MONSTER/m_ground.bin` stores its sprites as they are.
        match entry.starts_with(b"SIR0") {
            true => Self::parse(entry),
            false => Self::parse(&decompress_pkdpx(entry)?),
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        if data.get(0..4) != Some(b"SIR0") {
            return Err(RomError::Truncated);
        }
        let header = offset_at(data, 4)?;
        let (anim_info, image_info) = (offset_at(data, header)?, offset_at(data, header + 4)?);
        let kind = u16_le(data, header + 8)?;
        if kind == TEXTURE_SPRITE {
            return Self::parse_texture(data, anim_info, image_info);
        }
        let character = kind == CHARACTER_SPRITE;
        let wide = u16_le(data, image_info + 10)? != 0;

        let (image_table, palette_info) = (offset_at(data, image_info)?, offset_at(data, image_info + 4)?);
        let image_count = u16_le(data, image_info + 14)? as usize;
        let mut images = Vec::with_capacity(image_count);
        for i in 0..image_count {
            images.push(assemble_image(data, offset_at(data, image_table + i * 4)?)?);
        }
        // Where each image starts when they are laid out one after the
        // other, each from a new unit, as the pieces of the sprites that
        // are not characters refer to them (found on the interface's
        // cursor, the scenes' objects and the effects' bubbles).
        let mut starts = Vec::with_capacity(image_count);
        let mut next = 0;
        for image in &images {
            starts.push(next);
            next += image.len().div_ceil(TILE_UNIT);
        }

        // RGBX colours, running up to the palette info block.
        let palette_data = offset_at(data, palette_info)?;
        let palette = data.get(palette_data..palette_info).ok_or(RomError::Truncated)?.as_chunks::<4>().0.iter().map(|c| [c[0], c[1], c[2]]).collect();

        let (frame_table, offset_table, group_table) = (offset_at(data, anim_info)?, offset_at(data, anim_info + 4)?, offset_at(data, anim_info + 8)?);
        let group_count = u16_le(data, anim_info + 12)? as usize;

        let mut animations = Vec::with_capacity(group_count);
        let mut first_sequences = group_table;
        for g in 0..group_count {
            let (sequences, count) = (offset_at(data, group_table + g * 8)?, offset_at(data, group_table + g * 8 + 4)?);
            let mut group = Vec::new();
            for s in 0..if sequences == 0 { 0 } else { count } {
                group.push(parse_sequence(data, offset_at(data, sequences + s * 4)?)?);
            }
            if sequences != 0 {
                first_sequences = first_sequences.min(sequences);
            }
            animations.push(group);
        }

        // The meta-frame pointer table runs up to the offsets table or,
        // without one, to the first table of sequences.
        let frames_end = if offset_table != 0 { offset_table } else { first_sequences };
        let tiles = (!character).then_some(starts.as_slice());
        let mut meta_frames = Vec::new();
        for entry in (frame_table..frames_end).step_by(4) {
            meta_frames.push(parse_meta_frame(data, offset_at(data, entry)?, tiles)?);
        }
        let offsets = match offset_table {
            0 => Vec::new(),
            _ => (0..meta_frames.len())
                .map(|frame| {
                    let at = offset_table + frame * 16;
                    let point = |n: usize| -> Option<(i16, i16)> { Some((u16_le(data, at + n * 4).ok()? as i16, u16_le(data, at + n * 4 + 2).ok()? as i16)) };
                    Some([point(0)?, point(1)?, point(2)?, point(3)?])
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default(),
        };
        Ok(Self { texture: None, quads: Vec::new(), wide, images, palette, meta_frames, offsets, animations })
    }

    /// Where the head, the hands and the centre are on a meta-frame.
    pub fn offsets(&self, meta_frame: usize) -> Option<[(i16, i16); 4]> {
        self.offsets.get(meta_frame).copied()
    }

    /// A sprite drawn from one texture. Its image table has one entry, the
    /// texture's data, width and height; each of its pictures is 40 bytes
    /// of 16-bit numbers: two unused, an offset, half the size shown, the
    /// scale (one for all of the game's), the part of the texture (corner
    /// and size), then the console's texture parameters, whose bits 8 to
    /// 10 are the texture's format.
    fn parse_texture(data: &[u8], anim_info: usize, image_info: usize) -> Result<Self, RomError> {
        let (image_table, palette_info) = (offset_at(data, image_info)?, offset_at(data, image_info + 4)?);
        let image = offset_at(data, image_table)?;
        let (pixels, width, height) = (offset_at(data, image)?, u16_le(data, image + 4)? as usize, u16_le(data, image + 6)? as usize);
        let palette_data = offset_at(data, palette_info)?;
        let colours: Vec<&[u8; 4]> = data.get(palette_data..palette_info).ok_or(RomError::Truncated)?.as_chunks::<4>().0.iter().collect();

        let (frame_table, group_table) = (offset_at(data, anim_info)?, offset_at(data, anim_info + 8)?);
        let group_count = u16_le(data, anim_info + 12)? as usize;
        let mut animations = Vec::with_capacity(group_count);
        let mut first_sequences = group_table;
        for g in 0..group_count {
            let sequences = offset_at(data, group_table + g * 8)?;
            let count = u16_le(data, group_table + g * 8 + 4)? as usize;
            let mut group = Vec::new();
            for s in 0..if sequences == 0 { 0 } else { count } {
                group.push(parse_sequence(data, offset_at(data, sequences + s * 4)?)?);
            }
            if sequences != 0 {
                first_sequences = first_sequences.min(sequences);
            }
            animations.push(group);
        }

        let (mut quads, mut format) = (Vec::new(), 0);
        for entry in (frame_table..first_sequences).step_by(4) {
            let at = offset_at(data, entry)?;
            let signed = |index: usize| u16_le(data, at + index * 2).map(|value| value as i16 as i32);
            let size = |index: usize| u16_le(data, at + index * 2).map(|value| value as usize);
            // As smoke rising shows, the offset is counted from the
            // picture's centre towards the sprite's.
            quads.push(Quad { left: -signed(2)? - signed(4)?, top: -signed(3)? - signed(5)?, u: size(8)?, v: size(9)?, width: size(10)?, height: size(11)? });
            format = u16_le(data, at + 24)? >> 8 & 7;
        }

        // The console's texture formats: indexed colours, some with a
        // level of transparency in the high bits of each byte.
        let texel = |index: usize| -> Option<(usize, u8)> {
            let byte = |at: usize| data.get(pixels + at).copied();
            let solid = |colour: u8| (colour as usize, if colour == 0 { 0 } else { 255 });
            Some(match format {
                1 => (byte(index)? as usize & 31, (byte(index)? >> 5) * 36),
                2 => solid(byte(index / 4)? >> (index % 4 * 2) & 3),
                3 => solid(byte(index / 2)? >> (index % 2 * 4) & 15),
                4 => solid(byte(index)?),
                6 => (byte(index)? as usize & 7, ((byte(index)? >> 3) as u16 * 255 / 31) as u8),
                _ => return None,
            })
        };
        let mut rgba = vec![0u8; width * height * 4];
        for (index, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let (colour, alpha) = texel(index).ok_or(RomError::Truncated)?;
            if let Some(rgb) = colours.get(colour).filter(|_| alpha != 0) {
                pixel[..3].copy_from_slice(&rgb[..3]);
                pixel[3] = alpha;
            }
        }
        Ok(Self {
            texture: Some((width, rgba)),
            quads,
            wide: false,
            images: Vec::new(),
            palette: Vec::new(),
            meta_frames: Vec::new(),
            offsets: Vec::new(),
            animations,
        })
    }

    /// An effect sprite of 8 bits a pixel, with the palette it is drawn
    /// with: the rows the pack's `base` gives all of them (its last
    /// sprite: pmdsky-debug's `LoadPaletteBase`), then the sprite's own
    /// three as rows 13 to 15. Its pieces name the row they use.
    pub fn with_base(mut self, base: &Wan) -> Self {
        const OWN_FROM: usize = 13 * 16;
        if self.wide {
            let mut palette: Vec<[u8; 3]> = base.palette.iter().copied().take(OWN_FROM).collect();
            palette.resize(OWN_FROM, [0; 3]);
            palette.extend(self.palette.iter().copied().take(48));
            self.palette = palette;
        }
        self
    }

    /// How many pictures it has (meta-frames).
    pub fn meta_frame_count(&self) -> usize {
        self.meta_frames.len().max(self.quads.len())
    }

    /// Number of direction sequences of each animation group (0 = unused).
    pub fn group_sizes(&self) -> Vec<usize> {
        self.animations.iter().map(Vec::len).collect()
    }

    pub fn animation(&self, group: usize, direction: usize) -> &[AnimFrame] {
        self.animations.get(group).and_then(|g| g.get(direction)).map_or(&[], Vec::as_slice)
    }

    /// Draws a meta-frame, cropped to the bounding box of its pieces.
    pub fn render(&self, meta_frame: usize) -> Option<WanPicture> {
        if let Some((wide, texture)) = &self.texture {
            let quad = self.quads.get(meta_frame)?;
            let mut rgba = Vec::with_capacity(quad.width * quad.height * 4);
            for y in quad.v..quad.v + quad.height {
                let from = (y * wide + quad.u) * 4;
                rgba.extend_from_slice(texture.get(from..from + quad.width * 4)?);
            }
            return Some(WanPicture { width: quad.width, height: quad.height, origin: (-quad.left, -quad.top), rgba });
        }
        let pieces = self.meta_frames.get(meta_frame)?;
        let left = pieces.iter().map(|p| p.x).min()?;
        let top = pieces.iter().map(|p| p.y).min()?;
        let width = (pieces.iter().map(|p| p.x + p.width as i32).max()? - left) as usize;
        let height = (pieces.iter().map(|p| p.y + p.height as i32).max()? - top) as usize;
        let mut rgba = vec![0u8; width * height * 4];
        for piece in pieces {
            let Some(pixels) = self.images.get(piece.image) else {
                continue;
            };
            for y in 0..piece.height {
                for x in 0..piece.width {
                    // 8x8 tiles in row order. With 4 bits a pixel, two
                    // pixels per byte: the low half of a byte is the pixel
                    // on the left. With 8, the low half is the colour.
                    let tile = y / 8 * (piece.width / 8) + x / 8;
                    let at = match self.wide {
                        true => tile * 64 + y % 8 * 8 + x % 8,
                        false => tile * 32 + y % 8 * 4 + x % 8 / 2,
                    };
                    let Some(&byte) = pixels.get(at) else {
                        continue;
                    };
                    let colour = if self.wide || x % 2 == 0 { byte & 0xF } else { byte >> 4 } as usize;
                    let Some(rgb) = self.palette.get(piece.palette * 16 + colour) else {
                        continue;
                    };
                    if colour == 0 {
                        continue;
                    }
                    let px = piece.x - left + if piece.flip_x { piece.width - 1 - x } else { x } as i32;
                    let py = piece.y - top + if piece.flip_y { piece.height - 1 - y } else { y } as i32;
                    let o = (py as usize * width + px as usize) * 4;
                    rgba[o..o + 3].copy_from_slice(rgb);
                    rgba[o + 3] = 255;
                }
            }
        }
        Some(WanPicture { width, height, origin: (-left, -top), rgba })
    }
}

/// An image is a list of `(source, length)` runs; a null source is a run of
/// transparent bytes.
fn assemble_image(data: &[u8], mut entry: usize) -> Result<Vec<u8>, RomError> {
    let mut pixels = Vec::new();
    loop {
        let (source, amount) = (offset_at(data, entry)?, u16_le(data, entry + 4)? as usize);
        if source == 0 && amount == 0 {
            return Ok(pixels);
        }
        if source == 0 {
            pixels.resize(pixels.len() + amount, 0);
        } else {
            pixels.extend_from_slice(data.get(source..source + amount).ok_or(RomError::Truncated)?);
        }
        entry += 12;
    }
}

/// The sprite of `EFFECT/effect.bin` whose palette is the base of the
/// effects of 8 bits a pixel.
pub const EFFECT_BASE: usize = 292;

/// The sprite type of characters (0: the interface and the scenes'
/// objects, 2: effects), and the size in bytes of the units the pieces of
/// the others count tiles in.
const CHARACTER_SPRITE: u16 = 1;
const TEXTURE_SPRITE: u16 = 3;
const TILE_UNIT: usize = 128;

/// 10-byte pieces `image, unknown, attr0 (y), attr1 (x), attr2`; bit 11 of
/// attr1 marks the last piece. `tiles`: for an interface sprite, where each
/// image starts, to find the one a piece's tile number (attr2) means.
fn parse_meta_frame(data: &[u8], mut off: usize, tiles: Option<&[usize]>) -> Result<Vec<Piece>, RomError> {
    let mut pieces: Vec<Piece> = Vec::new();
    loop {
        let image = u16_le(data, off)? as i16;
        let (attr0, attr1, attr2) = (u16_le(data, off + 4)?, u16_le(data, off + 6)?, u16_le(data, off + 8)?);
        let (width, height) = PIECE_SIZES[(attr0 >> 14) as usize % 3][(attr1 >> 14) as usize];
        pieces.push(Piece {
            // Known gap: for a character, -1 means "same image as the previous
            // piece" here; the original may instead continue in the
            // previous piece's tiles.
            image: match (image < 0, tiles) {
                (true, Some(starts)) => {
                    let wanted = (attr2 & 0x3FF) as usize;
                    starts.iter().position(|&start| start == wanted).unwrap_or(0)
                }
                (true, None) => pieces.last().map_or(0, |p| p.image),
                (false, _) => image as usize,
            },
            x: (attr1 & 0x1FF) as i32 - 256,
            y: (attr0 & 0x3FF) as i32 - 512,
            width,
            height,
            flip_x: attr1 & 0x1000 != 0,
            flip_y: attr1 & 0x2000 != 0,
            palette: (attr2 >> 12) as usize,
        });
        if attr1 & 0x0800 != 0 {
            return Ok(pieces);
        }
        off += 10;
    }
}

/// 12-byte frames `duration, flag, meta-frame, offset x, offset y, shadow
/// x, shadow y`, closed by a zero duration.
fn parse_sequence(data: &[u8], mut off: usize) -> Result<Vec<AnimFrame>, RomError> {
    let mut frames = Vec::new();
    loop {
        let duration = *data.get(off).ok_or(RomError::Truncated)?;
        if duration == 0 {
            return Ok(frames);
        }
        frames.push(AnimFrame {
            duration,
            flag: data.get(off + 1).copied().unwrap_or(0),
            meta_frame: u16_le(data, off + 2)? as usize,
            offset: (u16_le(data, off + 4)? as i16, u16_le(data, off + 6)? as i16),
            shadow: (u16_le(data, off + 8)? as i16, u16_le(data, off + 10)? as i16),
        });
        off += 12;
    }
}

#[cfg(test)]
mod tests {
    /// Where a few monsters' head and centre are, standing facing down.
    #[test]
    #[ignore]
    fn real_rom_offsets_are_around_the_centre() {
        let rom = crate::nds::test_rom();
        let pack = rom.file("MONSTER/monster.bin").unwrap();
        for sprite in [1usize, 4, 7, 25, 150] {
            let wan = Wan::from_pack(pack, sprite).unwrap();
            let frame = wan.animation(7, 0)[0].meta_frame;
            println!("sprite {sprite}: {:?}, offset {:?}", wan.offsets(frame), wan.animation(7, 0)[0].offset);
        }
    }

    use super::*;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_monster_sprites_decode() {
        let rom = crate::nds::test_rom();
        let pack = rom.file("MONSTER/monster.bin").unwrap();
        let count = offset_at(pack, 4).unwrap();
        let (mut failed, mut drawable) = (Vec::new(), 0);
        for index in 0..count {
            let Ok(wan) = Wan::from_pack(pack, index) else {
                failed.push(index);
                continue;
            };
            // Walking in all 8 directions, every frame naming a real meta-frame
            // that draws at least one opaque pixel.
            let complete = (0..8).all(|direction| {
                let frames = wan.animation(0, direction);
                !frames.is_empty() && frames.iter().all(|f| wan.render(f.meta_frame).is_some_and(|p| p.rgba.as_chunks::<4>().0.iter().any(|px| px[3] != 0)))
            });
            drawable += complete as usize;
        }
        for name in ["MONSTER/monster.bin", "MONSTER/m_attack.bin"] {
            let pack = rom.file(name).unwrap();
            for index in [1, 25, 150] {
                let sizes = Wan::from_pack(pack, index).map(|w| w.group_sizes());
                println!("{name} sprite {index}: groups {sizes:?}");
            }
        }
        println!("{count} sprites, {drawable} with a full walk animation, failed: {failed:?}");
        assert!(failed.is_empty());
        assert!(drawable * 10 >= count * 9);
    }

    /// The effects scenes put on characters (a bubble with a mark, sweat
    /// drops, laughter...) are the animations of one sprite of the effect
    /// pack. Needs a real dump; `extractor dump emotes <rom> <folder>`
    /// writes a sheet of them.
    #[test]
    #[ignore]
    fn real_rom_emotes_decode() {
        let rom = crate::nds::test_rom();
        let pack = rom.file("EFFECT/effect.bin").unwrap();
        let sprite = Wan::from_pack(pack, 1).unwrap().with_base(&Wan::from_pack(pack, EFFECT_BASE).unwrap());
        // One group: its sequences are the effects' animations.
        let count = sprite.group_sizes()[0];
        println!("{} pictures, {count} animations", sprite.meta_frame_count());
        assert!(count >= 23);
        for group in 0..count {
            let frames = sprite.animation(0, group);
            let drawn = frames
                .iter()
                .filter_map(|frame| sprite.render(frame.meta_frame))
                .filter(|picture| picture.rgba.as_chunks::<4>().0.iter().any(|px| px[3] != 0))
                .count();
            // The first fifteen are the common ones: all are seen.
            assert!(group >= 15 || drawn > 0, "animation {group} draws nothing");
        }
    }

    /// The effects drawn from a texture: smoke of nine pictures growing
    /// from one corner of its texture, a halo seen through. Needs a real
    /// dump.
    #[test]
    #[ignore]
    fn real_rom_texture_sprites_decode() {
        let rom = crate::nds::test_rom();
        let pack = rom.file("EFFECT/effect.bin").unwrap();
        let smoke = Wan::from_pack(pack, 260).unwrap();
        assert_eq!(smoke.meta_frame_count(), 9);
        let frames = smoke.animation(0, 0);
        assert_eq!(frames.len(), 9);
        let sizes: Vec<(usize, usize)> = frames.iter().map(|frame| smoke.render(frame.meta_frame).map(|p| (p.width, p.height)).unwrap()).collect();
        println!("{sizes:?}");
        assert_eq!(sizes[0], (16, 16));
        assert!(sizes[8].0 > sizes[0].0);
        // It rises: its last picture reaches higher over the sprite's
        // centre than its first.
        let top = |index: usize| -smoke.render(frames[index].meta_frame).unwrap().origin.1;
        assert!(top(8) < top(0));
        let halo = Wan::from_pack(pack, 267).unwrap().render(0).unwrap();
        assert_eq!((halo.width, halo.height), (256, 256));
        let alphas: std::collections::HashSet<u8> = halo.rgba.as_chunks::<4>().0.iter().map(|pixel| pixel[3]).collect();
        assert!(alphas.len() > 4, "{alphas:?}");
        for file in [262, 263, 264, 265, 266] {
            let sprite = Wan::from_pack(pack, file).unwrap();
            let seen = sprite.render(0).is_some_and(|p| p.rgba.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0));
            assert!(seen, "{file}");
        }
    }

    /// The colours a picture draws, and its rows as text (`#` drawn).
    fn drawn(picture: &WanPicture) -> (Vec<[u8; 3]>, Vec<String>) {
        let mut colours = Vec::new();
        let mut rows = Vec::new();
        for row in picture.rgba.chunks_exact(picture.width * 4) {
            let mut text = String::new();
            for pixel in row.as_chunks::<4>().0 {
                text.push(if pixel[3] != 0 { '#' } else { '.' });
                let colour = [pixel[0], pixel[1], pixel[2]];
                if pixel[3] != 0 && !colours.contains(&colour) {
                    colours.push(colour);
                }
            }
            rows.push(text);
        }
        colours.sort();
        (colours, rows)
    }

    /// The interface's sprites: 8 bits a pixel, no offset table, pieces
    /// naming tiles. Needs a real dump.
    #[test]
    #[ignore]
    fn real_rom_interface_sprites_decode() {
        let rom = crate::nds::test_rom();
        let cursor = Wan::parse(rom.file("FONT/cursor.wan").unwrap()).unwrap();
        assert_eq!(cursor.meta_frame_count(), 4);

        // The menus' cursor: a yellow triangle pointing right, 5 columns
        // from the second, widest on its fifth row.
        let (colours, rows) = drawn(&cursor.render(0).unwrap());
        for row in &rows {
            println!("{row}");
        }
        println!("{colours:x?}");
        assert_eq!(rows.len(), 16);
        assert_eq!(&rows[0][..7], ".#.....");
        assert_eq!(&rows[4][..7], ".#####.");
        assert!(colours.contains(&[0xF7, 0xDF, 0x00]));
        assert!(colours.iter().all(|colour| colour[2] <= colour[0]), "yellows and white only");

        // The arrow under a message: greys, 11 columns wide on its first
        // row, a point on its last.
        let (colours, rows) = drawn(&cursor.render(1).unwrap());
        for row in &rows {
            println!("{row}");
        }
        println!("{colours:x?}");
        let widths: Vec<usize> = rows.iter().map(|row| row.matches('#').count()).filter(|&width| width > 0).collect();
        assert_eq!(widths, [11, 11, 9, 7, 5, 3, 1]);
        assert!(colours.iter().all(|colour| colour[0] == colour[1] && colour[0] >= 0xA7), "greys only");

        // Every other sprite of the interface draws something in each of
        // its pictures.
        for name in ["pagewait", "alert", "page_l", "page_r", "w_icon", "cursor16"] {
            let sprite = Wan::parse(rom.file(&format!("FONT/{name}.wan")).unwrap()).unwrap();
            let count = sprite.meta_frame_count();
            let visible = (0..count).filter(|&index| sprite.render(index).is_some_and(|p| p.rgba.as_chunks::<4>().0.iter().any(|px| px[3] != 0))).count();
            println!("{name}: {count} pictures, {visible} drawn, groups {:?}", sprite.group_sizes());
            assert!(count > 0 && visible == count, "{name}");
        }
    }
}
