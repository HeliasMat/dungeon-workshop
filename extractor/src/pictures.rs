//! The pictures: maps, sprites, faces, things, and the names beside them.

use rules::dungeon::MoveSlot;
use shell::art_files::{GROUND_KINDS, GroundInfo};
use shell::images::Frame;

use super::rom_data::{PlaceArt, RomData, SpriteArt};
use super::{png, wrong};

/// A sheet of an animation: its size, its frames, their durations, its pixels.
pub(super) type Sheet = ((u32, u32), u32, Vec<u8>, Vec<u8>);

/// Everything else the game shows and counts with, as files of the
/// project's own in `<folder>`:
/// - `backdrops/<level>.png`: the picture of each level's map (both its
///   layers, first frame), and `backdrops.txt` with its size and name;
/// - `sprites/<monster>_<animation>.png`: an animation of a monster (0
///   walking, 1 standing, 2 hurt, 3 asleep, 4 attacking), a row per direction (from facing down, turning
///   counter-clockwise), a cell per frame with the monster's centre in the
///   middle; `sprites.txt` gives each file's cell size, its frames'
///   60ths of a second and where the shadow lies from the middle;
/// - `portraits/<monster>_<face>.png`: 40x40 faces;
/// - `monsters.txt`, `entities.txt`, `dungeons.txt`, `items.txt`,
///   `moves.txt`: names and numbers, tab-separated.
///
/// How many backdrops, monsters with sprites and portraits were written.
pub fn pictures(data: &RomData, folder: &str) -> std::io::Result<(usize, usize, usize)> {
    for sub in ["backdrops", "grounds/info", "grounds/pictures", "sprites", "portraits", "props"] {
        std::fs::create_dir_all(format!("{folder}/{sub}"))?;
    }
    // The levels' maps.
    let mut backdrops = Vec::new();
    for (level, named) in data.levels.iter().enumerate() {
        let Some(art) = data.level_art(level as u16) else {
            continue;
        };
        let Some(mut picture) = art.frames.first().cloned() else {
            continue;
        };
        // (Whole, for the game to show it as the original does: its
        // animated tiles, the layer one walks under, its palettes that
        // change with time. The one picture below is for the top screen.)
        write_ground(folder, level, &art)?;
        if let Some(upper) = art.upper.first() {
            for (at, over) in upper.as_chunks::<4>().0.iter().enumerate().filter(|(_, over)| over[3] != 0) {
                picture[at * 4..at * 4 + 4].copy_from_slice(over);
            }
        }
        png(&format!("{folder}/backdrops/{level}.png"), art.width as u32, art.height as u32, &picture)?;
        backdrops.push(format!("{level}\t{}\t{}\t{}", named.name, art.width, art.height));
    }
    std::fs::write(format!("{folder}/backdrops.txt"), backdrops.join("\n") + "\n")?;

    // The monsters: names, sprites, faces.
    let kao = data.rom.file("FONT/kaomado.kao");
    let (mut monsters, mut sheets, mut drawn, mut faces) = (Vec::new(), Vec::new(), 0, 0);
    // Where each one's head, hands and centre are (`points.txt`).
    let mut points = Vec::new();
    // The monsters that can stand in a scene: the scripts' characters,
    // whoever the team can be, the special episodes' own.
    let forms = super::rom_data::MONSTER_FORMS;
    let mut staged: Vec<usize> = data.entities.iter().map(|entity| entity.monster as usize % forms).collect();
    if let Some(quiz) = data.quiz() {
        staged.extend(quiz.heroes.iter().flatten().chain(&quiz.partners).map(|&monster| monster as usize % forms));
    }
    staged.extend(episode_characters(data).iter().map(|character| character.0 as usize % forms));
    staged.extend([super::rom_data::PLAYER_MD_INDEX, super::rom_data::PARTNER_MD_INDEX]);
    for monster in 1..data.monsters.species_count() {
        let name = data.monster_name(monster);
        monsters.push(format!("{monster}\t{name}"));
        // (Who can stand in a scene has the animations scenes play too.)
        if let Some(art) = data.sprite_with(monster, staged.contains(&(monster % super::rom_data::MONSTER_FORMS))) {
            drawn += 1;
            let at: Vec<String> = art.points.iter().map(|(x, y)| format!("{x},{y}")).collect();
            points.push(format!("{monster}\t{}", at.join("\t")));
            // In the order of `shell::images::Anim`: walk, idle, hurt, sleep, attack;
            // then those of scenes, by their group.
            for animation in 0..art.animations.len() {
                if let Some(sheet) = sheet(&art, animation, Fill::Last) {
                    let shadow = art.animations[animation].iter().flatten().next().map_or((0, 0), |frame| frame.shadow);
                    png(&format!("{folder}/sprites/{monster}_{animation}.png"), sheet.0.0 * sheet.1, sheet.0.1 * 8, &sheet.3)?;
                    let durations: Vec<String> = sheet.2.iter().map(u8::to_string).collect();
                    sheets.push(format!("{monster}\t{animation}\t{}\t{}\t{}\t{}\t{}", sheet.0.0, sheet.0.1, durations.join(","), shadow.0, shadow.1));
                }
            }
        }
        for face in 0..20 {
            let Some(rgba) = kao.and_then(|kao| rom_formats::kao::portrait(kao, monster, face * 2)) else {
                continue;
            };
            png(&format!("{folder}/portraits/{monster}_{face}.png"), 40, 40, &rgba)?;
            faces += 1;
        }
    }
    std::fs::write(format!("{folder}/monsters.txt"), monsters.join("\n") + "\n")?;
    std::fs::write(format!("{folder}/sprites.txt"), sheets.join("\n") + "\n")?;
    std::fs::write(format!("{folder}/points.txt"), points.join("\n") + "\n")?;
    // The scenes' things (signs, chests, doors...), as sprites are:
    // `props/<object>_<animation>.png`, and in `props.txt` a last number
    // for those lying on the ground (characters are seen over them).
    let mut props = Vec::new();
    for object in 0..data.objects.len() {
        let Some((art, flat)) = data.prop(object as u16) else {
            continue;
        };
        for animation in 0..art.animations.len() {
            if let Some(sheet) = sheet(&art, animation, Fill::Round) {
                png(&format!("{folder}/props/{object}_{animation}.png"), sheet.0.0 * sheet.1, sheet.0.1 * 8, &sheet.3)?;
                let durations: Vec<String> = sheet.2.iter().map(u8::to_string).collect();
                props.push(format!("{object}\t{animation}\t{}\t{}\t{}\t0\t0\t{}", sheet.0.0, sheet.0.1, durations.join(","), flat as u8));
            }
        }
    }
    std::fs::write(format!("{folder}/props.txt"), props.join("\n") + "\n")?;

    // Who the scripts' characters are, and the other names.
    let entities: Vec<String> = data.entities.iter().enumerate().map(|(id, entity)| format!("{id}\t{}\t{}", entity.name, entity.monster)).collect();
    std::fs::write(format!("{folder}/entities.txt"), entities.join("\n") + "\n")?;
    let dungeons: Vec<String> = (0..180u16).map(|id| format!("{id}\t{}\t{}", data.dungeon_name(id), data.dungeon_floors(id))).collect();
    std::fs::write(format!("{folder}/dungeons.txt"), dungeons.join("\n") + "\n")?;
    let names = &data.names;
    let numbered = |names: &[String]| names.iter().enumerate().map(|(id, name)| format!("{id}\t{name}")).collect::<Vec<_>>().join("\n") + "\n";
    std::fs::write(format!("{folder}/items.txt"), numbered(&names.items))?;
    std::fs::write(format!("{folder}/moves.txt"), numbered(&names.moves))?;
    Ok((backdrops.len(), drawn, faces))
}

/// What fills the row of a direction with fewer frames than the longest:
/// nothing (an effect, played once), its frames again (a thing that
/// loops), its last one (a Pokemon, who is not to vanish in mid-gesture).
#[derive(Clone, Copy)]
pub(super) enum Fill {
    Nothing,
    Round,
    Last,
}

/// An animation of a sprite as one picture: the cell's size, the frames
/// of a direction, their 60ths of a second, and the pixels (a row per
/// direction).
///
/// `fill`: what a direction with fewer frames than the longest shows for
/// those it lacks.
pub(super) fn sheet(art: &SpriteArt, animation: usize, fill: Fill) -> Option<Sheet> {
    let directions = art.animations.get(animation)?;
    let frames = directions.iter().map(Vec::len).max().filter(|&frames| frames > 0)?;
    // A cell holds every frame with its centre in the cell's middle.
    let reach = |pick: &dyn Fn(&Frame) -> i32| directions.iter().flatten().map(pick).max().unwrap_or(1).max(1);
    let half_wide = reach(&|frame| frame.origin.0.max(frame.wide as i32 - frame.origin.0));
    let half_high = reach(&|frame| frame.origin.1.max(frame.high as i32 - frame.origin.1));
    let (wide, high) = ((half_wide * 2) as usize, (half_high * 2) as usize);
    let mut pixels = vec![0u8; wide * frames * high * 8 * 4];
    for (row, direction) in directions.iter().enumerate().take(8) {
        let shown: Vec<_> = match fill {
            Fill::Nothing => direction.iter().collect(),
            Fill::Round => direction.iter().cycle().take(frames).collect(),
            Fill::Last => direction.iter().chain(std::iter::repeat_n(direction.last(), frames).flatten()).take(frames).collect(),
        };
        for (column, frame) in shown.into_iter().enumerate() {
            let (left, top) = (half_wide - frame.origin.0, half_high - frame.origin.1);
            for y in 0..frame.high {
                let from = y * frame.wide * 4;
                let x = column * wide + left as usize;
                let to = ((row * high + top as usize + y) * wide * frames + x) * 4;
                pixels[to..to + frame.wide * 4].copy_from_slice(&frame.rgba[from..from + frame.wide * 4]);
            }
        }
    }
    let durations = directions.iter().find(|direction| direction.len() == frames)?.iter().map(|frame| frame.ticks as u8).collect();
    Some(((wide as u32, high as u32), frames as u32, durations, pixels))
}

/// The characters of the special episodes, as the game's table has them:
/// monster, level, and the moves it gives them (those its species has
/// learnt by that level, the last four, when it gives none).
pub(super) fn episode_characters(data: &RomData) -> Vec<(u16, i32, Vec<MoveSlot>)> {
    let table = rom_formats::types::load_episode_characters(&data.rom).unwrap_or_default();
    let character = |row: &[u16; 10]| {
        let (monster, level) = (row[0], row[7] as i32);
        let learnt = data.learnset(monster as usize);
        let given = row[2..6].iter().filter(|&&id| id != 0).filter_map(|&id| Some(super::rom_data::move_slot(id, data.moves.get(id as usize)?)));
        let mut moves: Vec<_> = given.collect();
        if moves.is_empty() {
            let known: Vec<_> = learnt.iter().filter(|(at, _)| *at as i32 <= level).map(|(_, slot)| *slot).collect();
            moves = known[known.len().saturating_sub(4)..].to_vec();
        }
        (monster, level, moves)
    };
    table.as_chunks::<10>().0.iter().map(character).collect()
}

/// Writes a level's picture whole: its animated frames, the layer one
/// walks under, and what its palettes that change with time need.
fn write_ground(folder: &str, level: usize, art: &PlaceArt) -> std::io::Result<()> {
    let (wide, high) = (art.width as u32, art.height as u32);
    let kinds: [(&Vec<Vec<u8>>, usize); 5] = [(&art.frames, 4), (&art.upper, 4), (&art.indices, 1), (&art.upper_indices, 1), (&art.layers, 1)];
    let mut counts = [0; 5];
    for (kind, (pictures, bytes)) in kinds.iter().enumerate() {
        let whole = |picture: &&Vec<u8>| picture.len() == art.width * art.height * bytes;
        for (at, picture) in pictures.iter().filter(whole).enumerate() {
            let color = if *bytes == 4 { image::ColorType::Rgba8 } else { image::ColorType::L8 };
            image::save_buffer(format!("{folder}/grounds/pictures/{level}_{}{at}.png", GROUND_KINDS[kind]), picture, wide, high, color).map_err(wrong)?;
            counts[kind] = at + 1;
        }
    }
    let info = GroundInfo {
        width: art.width,
        height: art.height,
        frame_seconds: art.frame_seconds,
        counts,
        palettes: art.palettes.clone(),
        cycles: art.cycles.iter().map(|cycle| (cycle.palette, cycle.ticks, cycle.colours.clone())).collect(),
    };
    std::fs::write(format!("{folder}/grounds/info/{level}.bin"), bincode::serialize(&info).map_err(wrong)?)
}
