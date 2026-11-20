//! The places walked in, and who stands there as the story goes.

use rom_formats::vm::{self, Act, Cast, Event, Program, Vars, Vm};

use super::rom_data::RomData;
use super::story::{Moment, read, read_at};

/// The places walked in, to `<folder>/town.txt`: `place <level> <name>
/// <tiles wide> <tiles high>` starts one, with `music <song>` for what is
/// heard there and a `solid` line per row of
/// its 8x8 tiles (`1` where one cannot walk); then `area <x> <y> <width>
/// <height> <examined>` for each area walked into (or looked at) and
/// `actor <entity> <x> <y> <direction> <leader>` for each character
/// standing there, each followed by what it does, as in the story's file,
/// with `menu <number>` for one of the game's own menus (a service, a
/// board, the crossroads). In the places the crossroads lead to, `moment
/// <chapter> <step>` then starts who stands there and what they say from
/// that moment of the story on, each time it changes. How many places
/// were written, and how many of them moment by moment.
///
/// `place_things.txt` tells what stands in those places the same way:
/// `place <level>`, `moment <chapter> <step>`, `thing <object> <x> <y>`.
pub fn town(data: &RomData, folder: &str, moments: &[Moment]) -> std::io::Result<(usize, usize)> {
    // Each place: its level, what it always has, and where it leads.
    let mut places: Vec<(u16, Vec<String>, Vec<u16>)> = Vec::new();
    for index in 0..data.zones().len() {
        let Some(zone) = data.load_zone(index) else {
            continue;
        };
        let mut lines = Vec::new();
        let name = data.zone_name(index).replace('\t', " ");
        lines.push(format!("place\t{}\t{name}\t{}\t{}", zone.level, zone.tiles_wide, zone.tiles_high));
        // The song heard there: the one the scripts run on coming in ask
        // for (the one most of them do, when they differ).
        let scripts = format!("SCRIPT/{}/enter", data.levels.get(zone.level as usize).map_or("", |level| level.name.as_str()));
        let mut songs: Vec<u16> = Vec::new();
        for path in data.rom.paths().filter(|path| path.starts_with(&scripts) && path.ends_with(".ssb")) {
            let script = data.rom.file(path).and_then(|file| rom_formats::script::Script::parse(file, &data.opcodes, super::rom_data::LANGUAGE).ok());
            for operation in script.iter().flat_map(|script| &script.operations) {
                let name = data.opcodes.get(operation.opcode as usize).map_or("", |opcode| opcode.name.as_str());
                if let (true, Some(&song)) = (name == "bgm_Play" || name == "bgm_PlayFadeIn", operation.parameters.first()) {
                    songs.push(song);
                }
            }
        }
        if let Some(&song) = songs.iter().max_by_key(|&&song| (songs.iter().filter(|&&other| other == song).count(), std::cmp::Reverse(song))) {
            lines.push(format!("music\t{song}"));
        }
        for row in zone.collision.chunks(zone.tiles_wide.max(1)) {
            lines.push(format!("solid\t{}", row.iter().map(|&solid| if solid { '1' } else { '0' }).collect::<String>()));
        }
        let mut leads = Vec::new();
        for area in &zone.exits {
            lines.push(format!("area\t{}\t{}\t{}\t{}\t{}", area.x, area.y, area.width, area.height, area.examine as u8));
            let from = lines.len();
            read(&area.script, Vm::start(&area.script), data, &mut lines);
            leads.extend(lines[from..].iter().filter_map(|line| line.strip_prefix("ground\t")?.parse::<u16>().ok()));
        }
        for actor in &zone.actors {
            lines.push(format!("actor\t{}\t{}\t{}\t{}\t{}", actor.entity, actor.x, actor.y, actor.direction, actor.leader as u8));
            if let Some(program) = &actor.script {
                read(program, Vm::start(program), data, &mut lines);
            }
        }
        // Who keeps a service stands there at some moments of the story
        // only: taken from wherever the place's other layouts have them.
        let folder = format!("SCRIPT/{}/", data.levels.get(zone.level as usize).map_or("", |level| level.name.as_str()));
        let mut layouts: Vec<&str> = data.rom.paths().filter(|path| path.starts_with(&folder) && path.ends_with(".sss")).collect();
        layouts.sort();
        let mut there: Vec<u16> = zone.actors.iter().map(|actor| actor.entity).collect();
        for layout in layouts {
            let Some(scene) = data.scene(zone.level, &layout[folder.len()..layout.len() - 4], true) else {
                continue;
            };
            for actor in scene.layers.iter().flatten() {
                if KEEPERS.contains(&actor.entity) && !there.contains(&actor.entity) {
                    there.push(actor.entity);
                    lines.push(format!("actor\t{}\t{}\t{}\t{}\t0", actor.entity, actor.x, actor.y, actor.direction));
                }
            }
        }
        places.push((zone.level, lines, leads));
    }
    // The places the crossroads lead to (two of its ways are set by hand
    // in the game: see `Reader::arrange`).
    const CROSSROADS: u16 = 224;
    let (mut reached, mut next) = (vec![CROSSROADS, 225, 325], vec![CROSSROADS, 225, 325]);
    while let Some(level) = next.pop() {
        for to in places.iter().filter(|place| place.0 == level).flat_map(|place| &place.2) {
            if !reached.contains(to) {
                reached.push(*to);
                next.push(*to);
            }
        }
    }
    let mut lines = Vec::new();
    let mut things = Vec::new();
    let mut detailed = 0;
    for (level, always, _) in &places {
        lines.extend(always.iter().cloned());
        if !reached.contains(level) {
            continue;
        }
        detailed += 1;
        // What is looked at there (a board...): whatever opens one of the
        // game's menus at some moment of the story, once.
        let mut boards: Vec<(i32, i32)> = Vec::new();
        for (_, state) in moments {
            for (x, y, width, height, script) in cast(data, *level, state).1 {
                let mut shown = Vec::new();
                read_at(&script, Vm::start(&script), data, &mut shown, state);
                shown.retain(|line| line.starts_with("menu"));
                if !shown.is_empty() && !boards.contains(&(x, y)) {
                    boards.push((x, y));
                    lines.push(format!("area\t{x}\t{y}\t{width}\t{height}\t1"));
                    lines.extend(shown);
                }
            }
        }
        things.push(format!("place\t{level}"));
        let mut last_things = Vec::new();
        for (moment, state) in moments {
            let now: Vec<String> = cast(data, *level, state).2.iter().map(|(object, x, y)| format!("thing\t{object}\t{x}\t{y}")).collect();
            if now != last_things {
                things.push(format!("moment\t{}\t{}", moment.0, moment.1));
                things.extend(now.iter().cloned());
                last_things = now;
            }
        }
        let mut last = Vec::new();
        for (moment, state) in moments {
            let mut now = Vec::new();
            for (entity, x, y, direction, leader, script) in cast(data, *level, state).0 {
                now.push(format!("actor\t{entity}\t{x}\t{y}\t{direction}\t{}", leader as u8));
                if let Some(program) = &script {
                    read_at(program, Vm::start(program), data, &mut now, state);
                }
            }
            if now != last && !now.is_empty() {
                lines.push(format!("moment\t{}\t{}", moment.0, moment.1));
                lines.extend(now.iter().cloned());
                last = now;
            }
        }
    }
    std::fs::write(format!("{folder}/town.txt"), lines.join("\n") + "\n")?;
    std::fs::write(format!("{folder}/place_things.txt"), things.join("\n") + "\n")?;
    Ok((places.len(), detailed))
}

/// The characters keeping a service: Kangaskhan, Kecleon and its brother,
/// Duskull, Chimecho (see `entities.txt`).
const KEEPERS: [u16; 5] = [91, 116, 117, 118, 119];

type Standing = (u16, i32, i32, u16, bool, Option<Program>);
/// Something looked at: its place, its size, its script.
type Looked = (i32, i32, i32, i32, Program);

/// Who stands in a place, what can be looked at there, the things it shows.
type Shown = (Vec<Standing>, Vec<Looked>, Vec<(u16, i32, i32)>);

/// Who stands in a level in a state of the story's variables: the level's own script
/// picks the layout the moment asks for, and that layout's script the
/// layers of characters it shows. And what can be looked at in those
/// layers, and the things they show.
fn cast(data: &RomData, level: u16, state: &Vars) -> Shown {
    /// Operations a script may run before it is taken to wait for the player.
    const ENOUGH: usize = 5_000;
    let mut vars = state.clone();
    vars.set(vm::GROUND_ENTER, level as i32);
    let mut found: Vec<Standing> = Vec::new();
    let mut looked: Vec<Looked> = Vec::new();
    let mut things: Vec<(u16, i32, i32)> = Vec::new();
    let Some(common) = data.scene(level, vm::ENTER, true) else {
        return (found, looked, things);
    };
    let mut station = None;
    let mut next = common.program.clone();
    for _ in 0..3 {
        let Some(program) = next.take() else {
            break;
        };
        let mut vm = Vm::start(&program);
        for _ in 0..ENOUGH {
            match vm.run(&program, &mut vars, |_| 0) {
                Event::Menu { .. } => vm.choose(0),
                Event::Act(Act::LoadStation { level, name }) => station = data.scene(level, &name, true),
                Event::Act(Act::Layer { cast, layer }) => {
                    if cast == Cast::Station && station.is_none() {
                        station = data.scene(level, "", true);
                    }
                    let scene = match cast {
                        Cast::Common => Some(&common),
                        Cast::Station => station.as_ref(),
                        Cast::Acting => None,
                    };
                    things.extend(
                        scene
                            .and_then(|scene| scene.things.get(layer as usize))
                            .into_iter()
                            .flatten()
                            .filter(|thing| !things.contains(thing))
                            .copied()
                            .collect::<Vec<_>>(),
                    );
                    for area in scene.and_then(|scene| scene.exits.get(layer as usize)).into_iter().flatten().filter(|area| area.examine) {
                        looked.push((area.x, area.y, area.width, area.height, area.script.clone()));
                    }
                    for actor in scene.and_then(|scene| scene.layers.get(layer as usize)).into_iter().flatten() {
                        if !found.iter().any(|known| known.0 == actor.entity) {
                            found.push((actor.entity, actor.x, actor.y, actor.direction, actor.leader, actor.script.clone()));
                        }
                    }
                }
                Event::Act(Act::Scene { level: to, name, station: true, .. }) => {
                    if to == level {
                        station = data.scene(to, &name, true);
                        next = station.as_ref().and_then(|scene| scene.program.clone());
                    }
                    break;
                }
                Event::Done | Event::EnterGround(_) | Event::EnterDungeon(_) => break,
                _ => {}
            }
        }
    }
    (found, looked, things)
}
