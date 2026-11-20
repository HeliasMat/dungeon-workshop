//! The windows and menus of the game's interface, as its code lays them
//! out (pmdsky-debug's `struct window_params` and the menu item tables of
//! overlay 31, the dungeon menus).

use super::nds::{NdsRom, RomError};

/// Where a window sits on its screen, in 8-pixel tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct WindowParams {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    /// 0: the main (touch) screen, 1: the other one.
    pub screen: u8,
    pub box_type: u8,
}

#[cfg(test)]
impl WindowParams {
    /// From a 16-byte `struct window_params`.
    pub fn parse(b: &[u8]) -> Self {
        Self { x: b[4], y: b[5], width: b[6], height: b[7], screen: b[8], box_type: b[9] }
    }
}

/// Bytes at `address` of the binary holding it (ARM9 or an overlay).
#[cfg(test)]
pub(crate) fn bytes_at(rom: &NdsRom, overlay: Option<u32>, address: u32, len: usize) -> Result<&[u8], RomError> {
    let (base, binary) = match overlay {
        Some(id) => rom.overlay(id),
        None => rom.arm9(),
    }
    .ok_or(RomError::Truncated)?;
    let off = address.checked_sub(base).ok_or(RomError::Truncated)? as usize;
    binary.get(off..off + len).ok_or(RomError::Truncated)
}

/// `FONT/frame0.wte`, the texture of the window frames (the first of the
/// five frame styles the options offer), in one of its sets of colours
/// (0: the usual blue, 2: the top menu's green):
/// 128x32 RGBA, colour 0 transparent. Layout from SkyTemple's
/// `graphics/wte` (GPL-3): a SIR0 container, 4 bits per pixel (low nibble
/// first), and a palette of 4-byte colours.
/// The colours of text (`FONT/text_pal.pal`: four bytes each), which the
/// colour tags of the game's text pick from (see `ui::code_color`).
pub fn load_text_colors(rom: &NdsRom) -> Result<Vec<[u8; 3]>, RomError> {
    let data = rom.file("FONT/text_pal.pal").ok_or(RomError::Truncated)?;
    Ok(data.as_chunks::<4>().0.iter().map(|color| [color[0], color[1], color[2]]).collect())
}

pub fn load_frame(rom: &NdsRom, variant: usize) -> Result<Vec<u8>, RomError> {
    load_frame_of(rom, 0, variant)
}

/// The window frame of a type the "Window" option chooses (`FONT/frame<n>
/// .wte`, 0 to 4), in a set of colours.
pub fn load_frame_of(rom: &NdsRom, kind: usize, variant: usize) -> Result<Vec<u8>, RomError> {
    const WIDTH: usize = 128;
    const HEIGHT: usize = 32;
    let data = rom.file(&format!("FONT/frame{kind}.wte")).ok_or(RomError::Truncated)?;
    let u32_at = |off: usize| {
        let b = data.get(off..off + 4).ok_or(RomError::Truncated)?;
        Ok::<_, RomError>(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    if data.get(0..4) != Some(b"SIR0") {
        return Err(RomError::Truncated);
    }
    let header = u32_at(4)?;
    if data.get(header..header + 4) != Some(b"WTE\0") {
        return Err(RomError::Truncated);
    }
    let (image, palette) = (u32_at(header + 4)?, u32_at(header + 0x18)?);
    let pixels = data.get(image..image + WIDTH * HEIGHT / 2).ok_or(RomError::Truncated)?;
    // Four sets of colours: blue, pink, green, yellow.
    let colors = palette + variant * 16 * 4;
    let colors = data.get(colors..colors + 16 * 4).ok_or(RomError::Truncated)?;
    let mut rgba = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for &byte in pixels {
        for index in [byte & 0xF, byte >> 4] {
            let color = &colors[index as usize * 4..][..3];
            rgba.extend([color[0], color[1], color[2], if index == 0 { 0 } else { 255 }]);
        }
    }
    Ok(rgba)
}

/// The font of the dungeon's top line (floor, level, HP, HP bar) in
/// `language`: a WTE texture of `DUNGEON/dungeon.bin`, one per language
/// (entries 1012, 1014... in the order e, f, g, i, s of the European ROM),
/// 128x16, in its second set of colours (the one the game shows), as RGBA.
pub fn load_hud(rom: &NdsRom, language: char) -> Result<Vec<u8>, RomError> {
    const WIDTH: usize = 128;
    const HEIGHT: usize = 16;
    const FIRST_ENTRY: usize = 1012;
    let index = "efgis".find(language).unwrap_or(0);
    let pack = rom.file("DUNGEON/dungeon.bin").ok_or(RomError::Truncated)?;
    let data = super::tileset::pack_entry(pack, FIRST_ENTRY + 2 * index)?;
    let u32_at = |off: usize| {
        let b = data.get(off..off + 4).ok_or(RomError::Truncated)?;
        Ok::<_, RomError>(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let header = u32_at(4)?;
    if data.get(header..header + 4) != Some(b"WTE\0") {
        return Err(RomError::Truncated);
    }
    let (image, palette) = (u32_at(header + 4)?, u32_at(header + 0x18)?);
    let pixels = data.get(image..image + WIDTH * HEIGHT / 2).ok_or(RomError::Truncated)?;
    let colors = data.get(palette + 16 * 4..palette + 32 * 4).ok_or(RomError::Truncated)?;
    let mut rgba = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for &byte in pixels {
        for index in [byte & 0xF, byte >> 4] {
            let color = &colors[index as usize * 4..][..3];
            rgba.extend([color[0], color[1], color[2], if index == 0 { 0 } else { 255 }]);
        }
    }
    Ok(rgba)
}

/// The picture behind the top screen in a dungeon, from `DUNGEON/sub.bgp`:
/// 256x192 RGBA. Layout from SkyTemple's `graphics/bgp` (GPL-3): AT4PX
/// compressed; a 32-byte header pointing to the palettes (16 colours of 4
/// bytes), the 8x8 tiles (4 bits per pixel, low nibble first) and the
/// 32x24 tile map (tile in the low 10 bits, flips, palette in the top 4).
pub fn load_backdrop(rom: &NdsRom) -> Result<Vec<u8>, RomError> {
    // The file is a sheet of the top screen's pieces; the checkerboard is
    // its strip 144 pixels down, which the game repeats over the screen.
    const STRIP: usize = 144;
    let sheet = load_bgp(rom.file("DUNGEON/sub.bgp").ok_or(RomError::Truncated)?)?;
    let mut rgba = vec![0u8; 256 * 192 * 4];
    for y in 0..192 {
        for x in 0..256 {
            let from = ((STRIP + y % 8) * 256 + x % 8) * 4;
            rgba[(y * 256 + x) * 4..][..4].copy_from_slice(&sheet[from..from + 4]);
        }
    }
    Ok(rgba)
}

/// A BGP picture as 256x192 RGBA.
pub fn load_bgp(packed: &[u8]) -> Result<Vec<u8>, RomError> {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 192;
    let data = super::px::decompress_at4px(packed)?;
    let u32_at = |off: usize| {
        let b = data.get(off..off + 4).ok_or(RomError::Truncated)?;
        Ok::<_, RomError>(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let (palette, tiles, map) = (u32_at(0)?, u32_at(8)?, u32_at(16)?);
    let mut rgba = vec![0u8; WIDTH * HEIGHT * 4];
    for cell in 0..(WIDTH / 8) * (HEIGHT / 8) {
        let entry = data.get(map + cell * 2..map + cell * 2 + 2).ok_or(RomError::Truncated)?;
        let entry = u16::from_le_bytes([entry[0], entry[1]]) as usize;
        let (tile, flip_x, flip_y, colors) = (entry & 0x3FF, entry & 0x400 != 0, entry & 0x800 != 0, entry >> 12);
        let pixels = data.get(tiles + tile * 32..tiles + tile * 32 + 32).ok_or(RomError::Truncated)?;
        for y in 0..8 {
            for x in 0..8 {
                let (sx, sy) = (if flip_x { 7 - x } else { x }, if flip_y { 7 - y } else { y });
                let index = (pixels[sy * 4 + sx / 2] >> (sx % 2 * 4)) & 0xF;
                let color = palette + (colors * 16 + index as usize) * 4;
                let color = data.get(color..color + 3).ok_or(RomError::Truncated)?;
                let at = ((cell / 32 * 8 + y) * WIDTH + cell % 32 * 8 + x) * 4;
                rgba[at..at + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
    }
    Ok(rgba)
}

/// `FONT/markfont.dat` with `FONT/markfont.pal`: the buttons, coin, hearts
/// and other icons that go with text, as `(width, height, RGBA)`. Layout
/// from SkyTemple's `graphics/fonts/graphic_font` (GPL-3): a table of
/// (width, height, offset) and one byte per pixel, 0x80 and up being a
/// colour of the palette.
/// The interface's cursors (`FONT/cursor.wan`): the yellow arrow of menus
/// (its first picture) and the grey one under a message that waits for a
/// key (its second), each cut to what it draws.
pub fn load_cursors(rom: &NdsRom) -> Result<(media::images::Picture, media::images::Picture), RomError> {
    let sprite = super::wan::Wan::parse(rom.file("FONT/cursor.wan").ok_or(RomError::Truncated)?)?;
    let cut = |meta_frame: usize| -> Result<media::images::Picture, RomError> {
        let picture = sprite.render(meta_frame).ok_or(RomError::Truncated)?;
        let drawn = |x: usize, y: usize| picture.rgba[(y * picture.width + x) * 4 + 3] != 0;
        let columns: Vec<usize> = (0..picture.width).filter(|&x| (0..picture.height).any(|y| drawn(x, y))).collect();
        let rows: Vec<usize> = (0..picture.height).filter(|&y| (0..picture.width).any(|x| drawn(x, y))).collect();
        let (Some(&left), Some(&right), Some(&top), Some(&bottom)) = (columns.first(), columns.last(), rows.first(), rows.last()) else {
            return Err(RomError::Truncated);
        };
        let mut rgba = Vec::new();
        for y in top..=bottom {
            let row = (y * picture.width + left) * 4..(y * picture.width + right + 1) * 4;
            rgba.extend_from_slice(&picture.rgba[row]);
        }
        Ok((right - left + 1, bottom - top + 1, rgba))
    };
    Ok((cut(0)?, cut(1)?))
}

pub fn load_marks(rom: &NdsRom) -> Result<Vec<media::images::Picture>, RomError> {
    let data = rom.file("FONT/markfont.dat").ok_or(RomError::Truncated)?;
    let palette = rom.file("FONT/markfont.pal").ok_or(RomError::Truncated)?;
    let table = data.get(2..4).ok_or(RomError::Truncated)?;
    let count = u16::from_le_bytes([table[0], table[1]]) as usize / 4;
    let mut marks = Vec::with_capacity(count);
    for entry in data.get(..count * 4).ok_or(RomError::Truncated)?.as_chunks::<4>().0 {
        let (width, height) = (entry[0] as usize, entry[1] as usize);
        let offset = u16::from_le_bytes([entry[2], entry[3]]) as usize;
        // Unused entries point nowhere.
        let Some(pixels) = data.get(offset..offset + width * height) else {
            marks.push((0, 0, Vec::new()));
            continue;
        };
        let mut rgba = Vec::with_capacity(pixels.len() * 4);
        for &pixel in pixels {
            let color = (pixel as usize).checked_sub(0x80).and_then(|index| palette.get(index * 4..index * 4 + 3));
            match color {
                Some(color) => rgba.extend([color[0], color[1], color[2], 255]),
                None => rgba.extend([0; 4]),
            }
        }
        marks.push((width, height, rgba));
    }
    Ok(marks)
}

/// The interface's texts in `language`. Their numbering differs between
/// regions, so each is found in the English text first.
pub fn load_strings(rom: &NdsRom, language: char) -> Result<rules::strings::UiStrings, RomError> {
    use super::text::Strings;
    let file = |letter: char| {
        let data = rom.file(&format!("MESSAGE/text_{letter}.str")).ok_or(RomError::Truncated)?;
        Strings::parse(data)
    };
    let english = file('e')?;
    let translated = file(language).ok();
    let text = translated.as_ref().unwrap_or(&english);
    let at = |index: Option<usize>| {
        let index = index.ok_or(RomError::Truncated)?;
        Ok::<_, RomError>(text.get(index).unwrap_or_default().to_string())
    };
    let block = |first: usize, count: usize| (0..count).map(|index| text.get(first + index).unwrap_or_default().to_string()).collect();
    // The 24 consecutive strings starting at the "moves" entry of the main
    // menu (see `ui::text`).
    let team = english.find_pair("Team", "Others").ok_or(RomError::Truncated)?;
    let menu = team.checked_sub(2).ok_or(RomError::Truncated)?;
    let weather = english.find_pair("Clear", "Sunny").ok_or(RomError::Truncated)?;
    Ok(rules::strings::UiStrings {
        menu: block(menu, rules::strings::text::COUNT),
        weather: block(weather, 8),
        bag_title: at(english.find("Treasure Bag"))?,
        moves_title: at(english.find("[string:0]'s moves"))?,
        // The dungeon's controls are the last of the lists that open so.
        controls: at(english.rposition(|text| text.starts_with("[CN]Controls\n\n  Walk:") && text.contains("Open menu:")))?,
        moves_hint: at(english.rposition(|text| text.starts_with("Shift moves:")))?,
        nothing_here: at(english.find("There is nothing at [string:0]'s feet."))?,
        leader: at(english.find("Leader"))?,
        // The name the game proposes for a new team.
        team_name: at(english.find("Poképals"))?,
        menu_controls: at(english.rposition(|text| text.starts_with("[CN]Controls\n\n  Move cursor:")))?,
        town_controls: at(english.rposition(|text| text.starts_with("[CN]Controls\n\n  Walk:") && text.contains("Talk")))?,
        log: {
            let mut log: Vec<String> = rules::strings::log_text::ENGLISH.iter().map(|wording| at(english.find(wording)).unwrap_or_default()).collect();
            let stats = english.find_pair("accuracy", "evasiveness");
            log.extend((0..6).map(|at| stats.and_then(|first| text.get(first + at)).unwrap_or_default().to_string()));
            log
        },
        // ("Dungeon Hints" and "Game Hints": each followed by the titles of
        // its eight hints, then their texts.)
        hints: ["Dungeon Hints", "Game Hints"]
            .into_iter()
            .flat_map(|title| {
                let first = english.find(title);
                (1..=2 * rules::strings::HINTS).map(move |at| first.map(|first| first + at))
            })
            .map(|at| at.and_then(|at| text.get(at)).unwrap_or_default().to_string())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sym;
    use crate::text::Strings;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_menu_art_is_the_expected_one() {
        let rom = crate::nds::test_rom();
        let frame = load_frame(&rom, 0).unwrap();
        // The green set: the same shape, greener than it is blue.
        let green = load_frame(&rom, 2).unwrap();
        let total = |picture: &[u8], channel: usize| -> u32 { picture.as_chunks::<4>().0.iter().take(128 * 24).map(|pixel| pixel[channel] as u32).sum() };
        assert!(total(&frame, 2) > total(&frame, 1) && total(&green, 1) > total(&green, 2));
        assert_eq!(frame.len(), 128 * 32 * 4);
        // The ring of the frame: opaque on its edge tiles, open in the middle.
        let alpha = |x: usize, y: usize| frame[(y * 128 + x) * 4 + 3];
        assert_eq!((alpha(12, 4), alpha(4, 12), alpha(12, 12)), (255, 255, 0));
        let strings = load_strings(&rom, 'e').unwrap();
        println!("dungeon messages: {:?}", strings.log);
        assert!(strings.log.iter().all(|text| !text.is_empty()));
        println!("hints: {:?}", &strings.hints[..rules::strings::HINTS]);
        assert!(strings.hints.len() == 4 * rules::strings::HINTS && strings.hints.iter().all(|text| !text.is_empty()));
        println!("{:?}", strings.menu);
        use rules::strings::text;
        let menu = &strings.menu;
        let main: Vec<&str> = text::MAIN.iter().map(|&index| menu[index].as_str()).collect();
        assert_eq!(main, ["Cap.", "Objets", "Equipe", "Autres", "Au sol", "Repos", "Quitter"]);
        assert!(menu[text::BELLY].starts_with("Estomac:"));
        assert!(menu[text::PLAY_TIME].starts_with("Tps de jeu:"));
        assert_eq!(menu[text::OTHERS_TITLE], "Autres");
        assert_eq!(menu[text::OTHERS.start], "Options");
        assert_eq!(menu[text::REST_TITLE], "Repos");
        assert_eq!(&menu[text::REST], ["Interrompre", "Abandonner"]);
        assert_eq!(&strings.weather[..2], ["Clair", "Ensoleillé"]);
        assert_eq!(strings.bag_title, "Sac à Trésor");
        assert_eq!(strings.moves_title, "Cap. de [string:0]");
        assert_eq!(strings.leader, "Meneur");
        assert!(strings.nothing_here.starts_with("Il n'y a rien aux pieds"));
        println!("{}\n{}", strings.controls, strings.moves_hint);
        assert!(strings.controls.starts_with("[CN]Commandes\n\n  Se déplacer:"));
        assert!(strings.controls.contains("Attaquer:") && strings.controls.contains("[M:B4]"));
        assert!(strings.moves_hint.starts_with("Modifier l'ordre des capacités:"));
        assert_eq!(strings.team_name, "Poképotes");
        assert!(strings.menu_controls.contains("Déplacer le curseur:"));
        assert!(strings.town_controls.contains("Parler au partenaire:"));
        // The pictures of the top menu.
        for name in ["BACK/s09p10a.bgp", "BACK/expback.bgp"] {
            assert_eq!(load_bgp(rom.file(name).unwrap()).unwrap().len(), 256 * 192 * 4);
        }
        // (`extractor dump bgp <rom> <folder> [name,...]` writes them out.)
        // The top screen's backdrop: the brown checkerboard seen in the
        // game, 4-pixel squares lit along their top and left.
        let backdrop = load_backdrop(&rom).unwrap();
        let pixel = |x: usize, y: usize| {
            let at = (y * 256 + x) * 4;
            [backdrop[at] & 0xF8, backdrop[at + 1] & 0xF8, backdrop[at + 2] & 0xF8]
        };
        println!("backdrop: {:?} {:?} {:?} {:?}", pixel(0, 0), pixel(1, 1), pixel(4, 0), pixel(5, 1));
        assert_eq!(pixel(0, 0), [144, 128, 56]);
        assert_eq!(pixel(1, 1), [120, 104, 48]);
        assert_eq!(pixel(4, 0), [72, 56, 24]);
        assert_eq!(pixel(5, 1), [96, 80, 40]);
        // The top line's font: white digits, then the coloured labels.
        let hud = load_hud(&rom, 'e').unwrap();
        assert_eq!(hud.len(), 128 * 16 * 4);
        let opaque = |tile: usize, color: fn(&[u8]) -> bool| {
            (0..64).any(|at| {
                let (x, y) = (tile % 16 * 8 + at % 8, tile / 16 * 8 + at / 8);
                let pixel = &hud[(y * 128 + x) * 4..][..4];
                pixel[3] == 255 && color(pixel)
            })
        };
        assert!(opaque(3, |p| p[0] > 240 && p[1] > 240 && p[2] > 240));
        assert!(opaque(10, |p| p[0] > 240 && p[1] < 160 && p[2] < 120));
        assert!(opaque(31, |p| p[1] > 240 && p[0] < 120));
        assert!(opaque(30, |p| p[0] > 240 && p[1] < 160));
        const POKE_MARK: usize = 3;
        // The icons: two wide words (START, SELECT), then the buttons; the
        // coin has colours.
        let marks = load_marks(&rom).unwrap();
        assert!(marks.len() > 60);
        assert_eq!((marks[0].0, marks[0].1, marks[2].0), (22, 11, 11));
        let coin = &marks[POKE_MARK];
        assert!(coin.2.as_chunks::<4>().0.iter().any(|pixel| pixel[3] == 255 && pixel[0] > 200));
        // The windows of the main menu, as the game's code places them.
        let window = |address: u32| WindowParams::parse(bytes_at(&rom, Some(31), address, 16).unwrap());
        let main = window(sym::DUNGEON_WINDOW_PARAMS_1.address[0]);
        assert_eq!((main.x, main.y, main.width, main.height), (2, 2, 7, 12));
        let place = window(sym::DUNGEON_WINDOW_PARAMS_2.address[0]);
        assert_eq!((place.x, place.y, place.width, place.height), (11, 4, 19, 2));
        let status = window(sym::DUNGEON_WINDOW_PARAMS_4.address[0]);
        assert_eq!((status.x, status.y, status.width, status.height), (2, 16, 28, 6));
    }

    /// Exploration: prints the dungeon menus' windows and items.
    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_dump_dungeon_menus() {
        let rom = crate::nds::test_rom();
        let text = Strings::parse(rom.file("MESSAGE/text_f.str").unwrap()).unwrap();
        let window = |overlay: Option<u32>, address: u32| WindowParams::parse(bytes_at(&rom, overlay, address, 16).unwrap());
        for (name, address) in [
            ("SIMPLE_MENU", sym::SIMPLE_MENU_DEFAULT_WINDOW_PARAMS),
            ("ADVANCED_MENU", sym::ADVANCED_MENU_DEFAULT_WINDOW_PARAMS),
            ("DIALOGUE_BOX", sym::DIALOGUE_BOX_DEFAULT_WINDOW_PARAMS),
            ("TEXT_BOX", sym::TEXT_BOX_DEFAULT_WINDOW_PARAMS),
            ("ALERT_BOX", sym::ALERT_BOX_DEFAULT_WINDOW_PARAMS),
            ("TEAM_SELECTION", sym::TEAM_SELECTION_MENU_DEFAULT_WINDOW_PARAMS),
        ] {
            println!("arm9 {name}: {:?}", window(None, address.address[0]));
        }
        println!("ov10 INVENTORY: {:?}", window(Some(10), sym::INVENTORY_MENU_DEFAULT_WINDOW_PARAMS.address[0]));
        println!("ov29 CONTROLS: {:?}", window(Some(29), sym::DUNGEON_CONTROLS_MENU_WINDOW_PARAMS.address[0]));
        for id in (0x8a4..0x8d4).chain(0x9a8..0x9b2).chain(0x9c4..0x9cc) {
            println!("str {id:#x}: {:?}", text.get(id).unwrap_or(""));
        }
        // The whole data block of the dungeon menus, 16 bytes at a time:
        // as a window, and as up to two (string id, result) menu items.
        // (The end, EU, is not named in pmdsky-debug.)
        let (start, end) = (sym::DUNGEON_WINDOW_PARAMS_1.address[0], 0x238_AE4C);
        let block = bytes_at(&rom, Some(31), start, (end - start) as usize).unwrap();
        for (index, chunk) in block.chunks(16).enumerate() {
            let address = start + index as u32 * 16;
            let words: Vec<u32> = chunk.as_chunks::<4>().0.iter().map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
            let strings: Vec<String> = words
                .iter()
                .filter(|&&word| (1..text.len() as u32).contains(&word) && word > 0x100)
                .map(|&word| format!("{word:#x}={:?}", text.get(word as usize).unwrap_or("")))
                .collect();
            let params = (chunk.len() == 16).then(|| WindowParams::parse(chunk));
            println!("{address:#x}: {words:x?} {params:?} {strings:?}");
        }
    }
}
