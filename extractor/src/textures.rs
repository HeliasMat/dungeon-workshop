//! The sounds, the effects, and the small pictures of `dungeon.bin` and
//! `manpu_su.sma`.

use super::pictures::{Fill, sheet};
use super::png;
use super::rom_data::RomData;

/// The songs, short tunes and sounds, in the project's own library
/// (`<folder>/sound.bin`, see `music::Library`: the project's own
/// synthesizer plays them), and what moves, traps and items show (`<folder>/effects/<id>.png`, the
/// frames side by side, and `effects.txt`: see `art_files::effect_file`), and the icons of
/// statuses (see [`status_icons`]). How many songs, tunes and sounds, and
/// how many effects were written.
pub fn sounds_and_effects(data: &RomData, folder: &str) -> std::io::Result<(usize, usize)> {
    std::fs::create_dir_all(format!("{folder}/effects"))?;
    let library = data.library();
    let files = library.songs.len() + library.jingles.len() + library.sounds.len();
    std::fs::write(format!("{folder}/sound.bin"), bincode::serialize(library).map_err(std::io::Error::other)?)?;
    let mut lines = Vec::new();
    for id in 0..700 {
        let Some(effect) = data.effect(id) else {
            continue;
        };
        let Some(((wide, high), frames, durations, pixels)) = sheet(&effect.sprite, 0, Fill::Nothing) else {
            continue;
        };
        // (The sheet is a character's: the first of its eight rows, or all
        // of them for an effect that faces the way of the move.)
        let directions = effect.sprite.animations.first().map_or(1, Vec::len).clamp(1, 8) as u32;
        let rows = (wide * frames * high * 4 * directions) as usize;
        png(&format!("{folder}/effects/{id}.png"), wide * frames, high * directions, &pixels[..rows])?;
        let durations: Vec<String> = durations.iter().map(u8::to_string).collect();
        let (repeats, screen) = (effect.repeats as u8, effect.screen as u8);
        lines.push(format!(
            "{id}\t{wide}\t{high}\t{}\t{}\t{repeats}\t{screen}\t{}\t{}\t{directions}",
            durations.join(","),
            effect.ticks,
            effect.sound,
            effect.point
        ));
    }
    std::fs::write(format!("{folder}/effects.txt"), lines.join("\n") + "\n")?;
    status_icons(data, folder)?;
    weather_textures(data, folder)?;
    damage_digits(data, folder)?;
    Ok((files, lines.len()))
}

/// The textures drawn over a floor in a sandstorm and in fog, from
/// `DUNGEON/dungeon.bin` (entries 1005 and 1001, pmdsky-debug's notes on
/// `LoadWeather3DFiles`; which is which by eye): `<folder>/weather/
/// <name>.png`.
fn weather_textures(data: &RomData, folder: &str) -> std::io::Result<usize> {
    const TEXTURES: [(&str, usize); 2] = [("sandstorm", 1005), ("fog", 1001)];
    let Some(pack) = data.rom.file("DUNGEON/dungeon.bin") else {
        return Ok(0);
    };
    std::fs::create_dir_all(format!("{folder}/weather"))?;
    let mut written = 0;
    for (name, entry) in TEXTURES {
        let Some(texture) = rom_formats::tileset::pack_entry(pack, entry).ok().and_then(Wte::read) else {
            continue;
        };
        let mut rgba = vec![0u8; texture.wide * texture.high * 4];
        for y in 0..texture.high {
            for x in 0..texture.wide {
                if let Some(rgb) = texture.colour(&texture.colours, texture.index(x, y)) {
                    rgba[(y * texture.wide + x) * 4..(y * texture.wide + x) * 4 + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                }
            }
        }
        png(&format!("{folder}/weather/{name}.png"), texture.wide as u32, texture.high as u32, &rgba)?;
        written += 1;
    }
    Ok(written)
}

/// A 3D engine texture (WTE) with four bits a pixel, low first. Layout as
/// As SkyTemple's `graphics/wte` reads it: a SIR0 file
/// whose content is "WTE", where its pixels are and how many bytes, the
/// width's power of two, the depth, the size, where its colours are and how
/// many.
struct Wte {
    wide: usize,
    high: usize,
    stride: usize,
    indices: Vec<usize>,
    /// Its colours as RGB, or none (they are another file's).
    colours: Vec<[u8; 3]>,
}

impl Wte {
    fn read(file: &[u8]) -> Option<Self> {
        let word = |at: usize| file.get(at..at + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        let half = |at: usize| file.get(at..at + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as usize);
        let header = if file.starts_with(b"SIR0") { word(4) } else { 0 };
        if file.get(header..header + 4) != Some(&b"WTE\0"[..]) {
            return None;
        }
        let (pixels, length) = (word(header + 4), word(header + 8));
        let (stride, depth) = (8 << (file[header + 0xC] & 7), file[header + 0xD]);
        let (wide, high) = (half(header + 0x14), half(header + 0x16));
        let (colours, count) = (word(header + 0x18), word(header + 0x1C));
        let colours = (0..count).filter_map(|index| file.get(colours + index * 4..colours + index * 4 + 3).map(|rgb| [rgb[0], rgb[1], rgb[2]])).collect();
        let indices = file.get(pixels..pixels + length).unwrap_or_default().iter().flat_map(|b| [(b & 15) as usize, (b >> 4) as usize]).collect();
        // (A file of colours only has no depth.)
        (depth == 3 || length == 0).then_some(Self { wide, high, stride, indices, colours })
    }

    fn index(&self, x: usize, y: usize) -> usize {
        self.indices.get(y * self.stride + x).copied().unwrap_or(0)
    }

    /// The colour of an index (0 is see-through).
    fn colour(&self, colours: &[[u8; 3]], index: usize) -> Option<[u8; 3]> {
        (index != 0).then(|| colours.get(index).copied()).flatten()
    }
}

/// The digits shown over monsters (the original's `DisplayAnimatedNumbers`
/// draws them with its 3D engine): `DUNGEON/dungeon.bin`'s entry 1007 (the
/// first of five, by language, all alike: 0 to 9, plus, minus, "MISS", 8
/// pixels high) in its colours of entry 1024, rows 3, 6 and 10 (the
/// automatic yellow, red and green): `<folder>/numbers.png`, the three
/// one below the other.
fn damage_digits(data: &RomData, folder: &str) -> std::io::Result<usize> {
    const DIGITS: usize = 1007;
    const COLOURS: usize = 1024;
    const ROWS: [usize; 3] = [3, 6, 10];
    const HIGH: usize = 8;
    let Some(pack) = data.rom.file("DUNGEON/dungeon.bin") else {
        return Ok(0);
    };
    let read = |entry: usize| rom_formats::tileset::pack_entry(pack, entry).ok().and_then(Wte::read);
    let (Some(digits), Some(colours)) = (read(DIGITS), read(COLOURS)) else {
        return Ok(0);
    };
    let wide = digits.wide;
    let mut rgba = vec![0u8; wide * HIGH * ROWS.len() * 4];
    for (at, row) in ROWS.iter().enumerate() {
        let palette = colours.colours.get(row * 16..row * 16 + 16).unwrap_or_default();
        for y in 0..HIGH {
            for x in 0..wide {
                if let Some(rgb) = digits.colour(palette, digits.index(x, y)) {
                    let to = ((at * HIGH + y) * wide + x) * 4;
                    rgba[to..to + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                }
            }
        }
    }
    png(&format!("{folder}/numbers.png"), wide as u32, (HIGH * ROWS.len()) as u32, &rgba)?;
    Ok(1)
}

/// The icons shown over monsters for their statuses, from
/// `SYSTEM/manpu_su.sma` (layout from skytemple-files' notes on the
/// format: a SIR0 file whose content header points to 12-byte animations
/// (width and height in 8x8 blocks, start in the pixels, frames), 4-bit
/// pixels block by block, and 16 palettes): `<folder>/status/<name>.png`,
/// the frames side by side, and `status.txt`: `<name> <width> <height>
/// <frames>`. How many were written.
fn status_icons(data: &RomData, folder: &str) -> std::io::Result<usize> {
    /// The picture of the file and the palette each status shows: the
    /// colours pmdsky-debug's notes give them (`struct status_icon_flags`),
    /// matched by eye. (The code choosing them was not found.)
    const ICONS: [(&str, usize, usize); 25] = [
        ("sleep", 16, 6),
        ("burn", 2, 0),
        ("poison", 3, 11),
        ("toxic", 3, 7),
        ("curse", 3, 6),
        ("frozen", 4, 0),
        ("confused", 5, 0),
        ("cringe", 6, 10),
        ("protect", 9, 10),
        ("safeguard", 9, 4),
        ("reflect", 9, 0),
        ("light_screen", 9, 3),
        ("endure", 9, 2),
        ("low_hp", 8, 0),
        ("lowered", 10, 3),
        ("sleepless", 1, 0),
        ("taunt", 7, 0),
        ("encore", 8, 0),
        ("embargo", 8, 3),
        ("sure_shot", 11, 0),
        ("set_damage", 11, 5),
        ("focus_energy", 11, 6),
        ("cross_eyed", 13, 0),
        ("grudge", 9, 7),
        ("heal_block", 15, 3),
    ];
    let Some(file) = data.rom.file("SYSTEM/manpu_su.sma") else {
        return Ok(0);
    };
    let word = |at: usize| file.get(at..at + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let half = |at: usize| file.get(at..at + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let header = word(4);
    let (animations, count, pixels_at, palettes_at) = (word(header + 4), word(header + 8), word(header + 12), word(header + 20));
    let nibbles: Vec<u8> = file.get(pixels_at..palettes_at).unwrap_or_default().iter().flat_map(|b| [b & 15, b >> 4]).collect();
    std::fs::create_dir_all(format!("{folder}/status"))?;
    let mut lines = Vec::new();
    for (name, icon, palette) in ICONS.into_iter().filter(|icon| icon.1 < count) {
        let at = animations + icon * 12;
        let (blocks_wide, blocks_high) = (file.get(at).copied().unwrap_or(0) as usize, file.get(at + 1).copied().unwrap_or(0) as usize);
        let (start, frames) = (half(at + 4) * 2, half(at + 8));
        let (wide, high) = (blocks_wide * 8, blocks_high * 8);
        if wide == 0 || high == 0 || frames == 0 {
            continue;
        }
        let mut strip = vec![0u8; wide * frames * high * 4];
        for frame in 0..frames {
            for y in 0..high {
                for x in 0..wide {
                    let block = (y / 8) * blocks_wide + x / 8;
                    let index = nibbles.get(start + frame * wide * high + block * 64 + (y % 8) * 8 + x % 8).copied().unwrap_or(0);
                    let colour = palettes_at + palette * 64 + index as usize * 4;
                    if let (true, Some(rgb)) = (index != 0, file.get(colour..colour + 3)) {
                        let to = (y * wide * frames + frame * wide + x) * 4;
                        strip[to..to + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                    }
                }
            }
        }
        png(&format!("{folder}/status/{name}.png"), (wide * frames) as u32, high as u32, &strip)?;
        lines.push(format!("{name}\t{wide}\t{high}\t{frames}"));
    }
    std::fs::write(format!("{folder}/status.txt"), lines.join("\n") + "\n")?;
    Ok(lines.len())
}
