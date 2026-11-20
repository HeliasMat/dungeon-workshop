//! `extractor dump <what> <rom> …`: development aids that write some of the
//! ROM's pictures as they are read, or list what it holds, to look at them
//! by eye. The formats (`rom_formats`) only read and return; what is
//! written, and where, is decided here. Pictures are raw RGBA, as they
//! were when these lived in the formats' tests behind `DUNGEON_DUMP`.
//!
//! - `portraits <rom> <folder>`: a few portraits (`kao_<monster>_<face>.rgba`, 40x40)
//! - `title-sprites <rom> <folder>`: the title's sprites, each meta-frame
//!   (`<name>_<frame>.raw`: width and height on 4 bytes each, then the pixels)
//! - `maps <rom> <folder> [name,…]`: the top-left 256x192 of every map
//!   (`maps.rgba`) with their names (`maps.txt`); those named, in full
//!   (`<name>_<width>x<height>.rgba`)
//! - `tiles <rom> <folder> <code,…>`: the tile entries of those maps
//!   (`<code>.tiles`: a line a layer then a row, `entry:chunk` in hexadecimal)
//!   and the list of maps (`cartes.txt`: each its BMA, BPC, BPL, BPAs)
//! - `bgp <rom> <folder> [name,…]`: pictures of the menus, 256x192
//!   (`DUNGEON/sub` if none is named; `<name with _ for />.rgba`)
//! - `objects <rom> <folder> [first]`: the first picture of 120 objects on a sheet
//!   (`objects.rgba`, 15 cells of 96 pixels a row) and their names and sizes (`objects.txt`)
//! - `effects <rom> <folder>`: every picture of the effects' first sprites
//!   (`<sprite>_<frame>_<width>x<height>.rgba`)
//! - `emotes <rom> <folder> [sprite]`: the effects scenes put on characters, a sheet
//!   (`emotes.rgba`: a row per animation, 12 cells of 64 pixels); sprite 1 by default
//! - `script <rom> <file> [routine,…]`: a script listed (`SCRIPT/T00P01/m01a01a.ssb`)
//! - `offsets <rom> [sprite] [group]`: a monster's first frames, their size,
//!   origin, offset and shadow (sprite 1, group 7 by default)

use std::path::Path;

use rom_formats::NdsRom;
use rom_formats::mapbg::{MapBackground, map_list};
use rom_formats::wan::{EFFECT_BASE, Wan};

pub const HELP: &str = "extractor dump <what> <rom> ...   development aids, written as raw RGBA:
    portraits <rom> <folder>
    title-sprites <rom> <folder>
    maps <rom> <folder> [name,...]
    tiles <rom> <folder> <code,...>
    bgp <rom> <folder> [name,...]
    objects <rom> <folder> [first]
    effects <rom> <folder>
    emotes <rom> <folder> [sprite]
    script <rom> <file> [routine,...]
    offsets <rom> [sprite] [group]";

/// `arguments`: what follows `dump`.
pub fn run(arguments: &[String]) -> Result<(), String> {
    let (Some(what), Some(rom)) = (arguments.first(), arguments.get(1)) else {
        return Err(HELP.to_string());
    };
    let rom = NdsRom::open(rom).map_err(|why| format!("{rom}: {why:?}"))?;
    let third = arguments.get(2).map(String::as_str);
    let fourth = arguments.get(3).map(String::as_str);
    let folder = || third.map(Path::new).ok_or_else(|| format!("extractor dump {what}: the folder to write in is missing\n{HELP}"));
    let list =
        |text: Option<&str>| text.map(|text| text.split(',').filter(|word| !word.is_empty()).map(str::to_string).collect::<Vec<_>>()).unwrap_or_default();
    let number = |text: Option<&str>, otherwise: usize| text.and_then(|text| text.parse().ok()).unwrap_or(otherwise);
    match what.as_str() {
        "portraits" => portraits(&rom, folder()?),
        "title-sprites" => title_sprites(&rom, folder()?),
        "maps" => maps(&rom, folder()?, &list(fourth)),
        "tiles" => tiles(&rom, folder()?, &list(fourth)),
        "bgp" => bgp(&rom, folder()?, &list(fourth)),
        "objects" => objects(&rom, folder()?, number(fourth, 0)),
        "effects" => effects(&rom, folder()?),
        "emotes" => emotes(&rom, folder()?, number(fourth, 1)),
        "script" => script(&rom, third.ok_or_else(|| format!("extractor dump script: the script's file is missing\n{HELP}"))?, &list(fourth)),
        "offsets" => offsets(&rom, number(third, 1), number(fourth, 7)),
        _ => Err(format!("extractor dump: no {what}\n{HELP}")),
    }
}

fn file<'a>(rom: &'a NdsRom, name: &str) -> Result<&'a [u8], String> {
    rom.file(name).ok_or_else(|| format!("no {name} in the ROM"))
}

fn write(path: std::path::PathBuf, bytes: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::write(&path, bytes).map_err(|why| format!("{}: {why}", path.display()))
}

fn wan(data: &[u8], index: usize) -> Result<Wan, String> {
    Wan::from_pack(data, index).map_err(|why| format!("sprite {index}: {why:?}"))
}

fn portraits(rom: &NdsRom, folder: &Path) -> Result<(), String> {
    let kao = file(rom, "FONT/kaomado.kao")?;
    for (index, sub) in [(25, 0), (25, 2), (1, 0), (1, 1)] {
        let data = rom_formats::kao::portrait(kao, index, sub).unwrap_or_default();
        write(folder.join(format!("kao_{index}_{sub}.rgba")), data)?;
    }
    Ok(())
}

fn title_sprites(rom: &NdsRom, folder: &Path) -> Result<(), String> {
    for name in ["a", "b", "c", "d", "e"].iter().flat_map(|letter| (1..=4).map(move |n| format!("s05p02{letter}{n}"))) {
        let Some(data) = rom.file(&format!("GROUND/{name}.wan")) else { continue };
        let Ok(wan) = Wan::parse(data) else {
            println!("{name}: not parsed");
            continue;
        };
        println!("{name}: {} meta-frames, groups {:?}", wan.meta_frame_count(), wan.group_sizes());
        for frame in 0..wan.meta_frame_count().min(40) {
            if let Some(picture) = wan.render(frame) {
                let mut bytes = (picture.width as u32).to_le_bytes().to_vec();
                bytes.extend((picture.height as u32).to_le_bytes());
                bytes.extend(&picture.rgba);
                write(folder.join(format!("{name}_{frame}.raw")), bytes)?;
            }
        }
    }
    Ok(())
}

fn maps(rom: &NdsRom, folder: &Path, whole: &[String]) -> Result<(), String> {
    let maps = map_list(file(rom, "MAP_BG/bg_list.dat")?);
    let (mut pixels, mut names) = (Vec::new(), Vec::new());
    for entry in &maps {
        let Ok(map) = MapBackground::load(rom, entry) else {
            continue;
        };
        let Some(frame) = map.flat(0) else {
            continue;
        };
        for y in 0..192 {
            for x in 0..256 {
                let at = (y * map.width + x) * 4;
                match (x < map.width && y < map.height, frame.get(at..at + 4)) {
                    (true, Some(pixel)) => pixels.extend_from_slice(pixel),
                    _ => pixels.extend([0, 0, 0, 255]),
                }
            }
        }
        names.push(entry.bma.clone());
    }
    for name in whole {
        let Some(entry) = maps.iter().find(|entry| &entry.bma == name) else {
            continue;
        };
        let map = MapBackground::load(rom, entry).map_err(|why| format!("{name}: {why:?}"))?;
        let Some(flat) = map.flat(0) else { continue };
        write(folder.join(format!("{name}_{}x{}.rgba", map.width, map.height)), flat)?;
    }
    write(folder.join("maps.rgba"), pixels)?;
    write(folder.join("maps.txt"), names.join("\n"))
}

fn tiles(rom: &NdsRom, folder: &Path, codes: &[String]) -> Result<(), String> {
    let maps = map_list(file(rom, "MAP_BG/bg_list.dat")?);
    for code in codes {
        let Some(entry) = maps.iter().find(|entry| &entry.bma == code) else {
            continue;
        };
        let tiles = MapBackground::tile_entries(rom, entry).map_err(|why| format!("{code}: {why:?}"))?;
        let mut text = format!("{} {} {}\n", tiles.wide, tiles.high, tiles.entries.len());
        for (entries, chunks) in tiles.entries.iter().zip(&tiles.chunks) {
            for row in 0..tiles.high {
                let said: Vec<String> = (0..tiles.wide).map(|x| format!("{:04x}:{}", entries[row * tiles.wide + x], chunks[row * tiles.wide + x])).collect();
                text.push_str(&said.join(" "));
                text.push('\n');
            }
        }
        write(folder.join(format!("{code}.tiles")), text)?;
    }
    // (And the list of maps: each its BPL, BPC, BMA and BPA files.)
    let listed: Vec<String> = maps.iter().map(|entry| format!("{} {} {} {}", entry.bma, entry.bpc, entry.bpl, entry.bpas.join(","))).collect();
    write(folder.join("cartes.txt"), listed.join("\n"))
}

fn bgp(rom: &NdsRom, folder: &Path, names: &[String]) -> Result<(), String> {
    let names = if names.is_empty() { vec!["DUNGEON/sub".to_string()] } else { names.to_vec() };
    for name in names {
        let picture = rom_formats::menus::load_bgp(file(rom, &format!("{name}.bgp"))?).map_err(|why| format!("{name}: {why:?}"))?;
        write(folder.join(format!("{}.rgba", name.replace('/', "_"))), picture)?;
    }
    Ok(())
}

fn objects(rom: &NdsRom, folder: &Path, first: usize) -> Result<(), String> {
    const CELL: usize = 96;
    let objects = rom_formats::script::load_objects(rom).map_err(|why| format!("the objects: {why:?}"))?;
    let named: Vec<&String> = objects.iter().filter(|name| !name.is_empty() && *name != "NULL").collect();
    let mut sheet = vec![0u8; 15 * CELL * 8 * CELL * 4];
    let mut sizes = Vec::new();
    for (cell, name) in named.iter().skip(first).take(120).enumerate() {
        let Some(data) = rom.file(&format!("GROUND/{name}.wan")) else {
            continue;
        };
        let Ok(sprite) = Wan::parse(data) else { continue };
        let frame = sprite.animation(0, 0).first().map_or(0, |frame| frame.meta_frame);
        let Some(picture) = sprite.render(frame) else {
            continue;
        };
        // Its size, and where its reference point is in it.
        let (size, groups) = ((picture.width, picture.height), sprite.group_sizes());
        sizes.push(format!("{cell} {name} {size:?} at {:?} groups {groups:?}", picture.origin));
        for y in 0..picture.height.min(CELL) {
            for x in 0..picture.width.min(CELL) {
                let from = (y * picture.width + x) * 4;
                let to = ((cell / 15 * CELL + y) * 15 * CELL + cell % 15 * CELL + x) * 4;
                sheet[to..to + 4].copy_from_slice(&picture.rgba[from..from + 4]);
            }
        }
    }
    write(folder.join("objects.rgba"), sheet)?;
    write(folder.join("objects.txt"), sizes.join("\n"))
}

fn effects(rom: &NdsRom, folder: &Path) -> Result<(), String> {
    let pack = file(rom, "EFFECT/effect.bin")?;
    let base = wan(pack, EFFECT_BASE)?;
    for sprite in [0usize, 1, EFFECT_BASE] {
        let Ok(wan) = Wan::from_pack(pack, sprite) else { continue };
        let wan = if sprite == EFFECT_BASE { wan } else { wan.with_base(&base) };
        for frame in 0..wan.meta_frame_count() {
            if let Some(picture) = wan.render(frame) {
                write(folder.join(format!("{sprite}_{frame}_{}x{}.rgba", picture.width, picture.height)), &picture.rgba)?;
            }
        }
    }
    Ok(())
}

fn emotes(rom: &NdsRom, folder: &Path, file_number: usize) -> Result<(), String> {
    const CELL: usize = 64;
    let pack = file(rom, "EFFECT/effect.bin")?;
    let sprite = wan(pack, file_number)?.with_base(&wan(pack, EFFECT_BASE)?);
    // One group: its sequences are the effects' animations.
    let count = sprite.group_sizes().first().copied().unwrap_or(0);
    println!("{} pictures, {count} animations", sprite.meta_frame_count());
    let mut sheet = vec![0u8; 12 * CELL * count * CELL * 4];
    for group in 0..count {
        let frames = sprite.animation(0, group);
        for (cell, frame) in frames.iter().enumerate() {
            let Some(picture) = sprite.render(frame.meta_frame) else {
                continue;
            };
            if cell == 0 {
                let time: u32 = frames.iter().map(|frame| frame.duration as u32).sum();
                println!("{group}: {} frames in {time} ticks, first {:?} at {:?}", frames.len(), (picture.width, picture.height), picture.origin);
            }
            for y in 0..picture.height.min(CELL) {
                for x in 0..picture.width.min(CELL) {
                    let from = (y * picture.width + x) * 4;
                    let to = ((group * CELL + y) * 12 * CELL + cell.min(11) * CELL + x) * 4;
                    sheet[to..to + 4].copy_from_slice(&picture.rgba[from..from + 4]);
                }
            }
        }
    }
    write(folder.join("emotes.rgba"), sheet)
}

fn script(rom: &NdsRom, path: &str, keep: &[String]) -> Result<(), String> {
    use rom_formats::script::{Script, find_opcodes};
    let keep: Vec<usize> = keep.iter().filter_map(|n| n.parse().ok()).collect();
    let opcodes = find_opcodes(rom).map_err(|why| format!("the operations: {why:?}"))?;
    let script = Script::parse(file(rom, path)?, &opcodes, 'e').map_err(|why| format!("{path}: {why:?}"))?;
    println!("{} routines, constants {:?}", script.routines.len(), script.constants);
    let mut starts: Vec<usize> = script.routines.iter().map(|r| r.first_operation).collect();
    starts.push(script.operations.len());
    for (index, routine) in script.routines.iter().enumerate() {
        if !keep.is_empty() && !keep.contains(&index) {
            continue;
        }
        println!("routine {index} kind {} target {}", routine.kind, routine.target);
        let end = starts.iter().copied().filter(|&s| s > routine.first_operation).min();
        for at in routine.first_operation..end.unwrap_or(script.operations.len()) {
            let operation = &script.operations[at];
            let name = &opcodes[operation.opcode as usize].name;
            let p: Vec<i16> = operation.parameters.iter().map(|&p| p as i16).collect();
            // What message operations say.
            let said = operation.parameters.last().and_then(|&text| script.text(text));
            let said = said.filter(|_| name.starts_with("message_") || name.starts_with("Case"));
            println!("  @{} {name} {p:?} {}", script.offsets[at], said.unwrap_or("").replace('\n', " "));
        }
    }
    Ok(())
}

fn offsets(rom: &NdsRom, index: usize, group: usize) -> Result<(), String> {
    let wan = wan(file(rom, "MONSTER/monster.bin")?, index)?;
    for frame in wan.animation(group, 0).iter().take(3) {
        let Some(picture) = wan.render(frame.meta_frame) else { continue };
        let rows: Vec<usize> = (0..picture.height).filter(|y| (0..picture.width).any(|x| picture.rgba[(y * picture.width + x) * 4 + 3] != 0)).collect();
        let (top, bottom) = (rows.first().copied().unwrap_or(0) as i32, rows.last().copied().unwrap_or(0) as i32);
        println!(
            "size {}x{} origin {:?} offset {:?} shadow {:?} drawn rows {} to {} from the origin",
            picture.width,
            picture.height,
            picture.origin,
            frame.offset,
            frame.shadow,
            top - picture.origin.1,
            bottom - picture.origin.1
        );
    }
    Ok(())
}
