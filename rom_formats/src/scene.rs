//! Scene files (`SCRIPT/<level>/*.ssa`, `.sse`, `.sss`): who and what
//! stands where in a level.
//!
//! All offsets are counted in 16-bit words. The header gives the layer
//! count and the positions of the layer list and of the entry tables; each
//! layer then gives, per kind of entry (actors, objects, performers,
//! events...), a count and where its entries start. Actors, objects,
//! performers and events (with their triggers) are read.
//! Layout from memory of the community's format notes, checked against
//! every scene file of the ROM in the tests.

use super::nds::RomError;

const LAYER_WORDS: usize = 10;
const ACTOR_WORDS: usize = 8;
const EVENT_WORDS: usize = 8;
const PERFORMER_WORDS: usize = 10;
const OBJECT_WORDS: usize = 10;

/// A character placed in a scene.
#[derive(Clone, Copy, Debug)]
pub struct SceneActor {
    /// Index into the script engine's entity table.
    pub entity: u16,
    /// 1 to 8, starting facing down and turning counter-clockwise; 0: none.
    pub direction: u16,
    /// Position in pixels.
    pub x: i32,
    pub y: i32,
    /// Number of the scene's script this actor's routines are in, if any.
    pub script: Option<u16>,
}

/// An area that runs a script when the player walks into it (an exit, in
/// practice).
#[derive(Clone, Copy, Debug)]
pub struct SceneEvent {
    /// Position and size in pixels.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Number of the scene's script that the event runs.
    pub script: u16,
    /// The shared routine that handles the event, and the two words that
    /// follow it in the event's trigger (skytemple-files' `SsaTrigger`).
    pub routine: u16,
    pub unknown: (u16, u16),
}

/// A prop placed in a scene; with a script, it runs it when the leader
/// touches or examines it.
#[derive(Clone, Copy, Debug)]
pub struct SceneObject {
    /// Number in the game's table of objects.
    pub object: u16,
    /// Position of its centre and size of its box, in pixels.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Number of the scene's script it runs, if any.
    pub script: Option<u16>,
}

pub struct Scene {
    /// Actors of each layer.
    pub layers: Vec<Vec<SceneActor>>,
    /// Events of all layers.
    pub events: Vec<SceneEvent>,
    /// How many of them each layer has, in order.
    pub layer_events: Vec<usize>,
    /// Objects of each layer.
    pub objects: Vec<Vec<SceneObject>>,
    /// Performers (invisible spots scripts refer to) of each layer: their
    /// number and position in pixels.
    pub performers: Vec<Vec<(u16, i32, i32)>>,
}

impl Scene {
    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        let word = |index: usize| -> Result<u16, RomError> {
            let b = data.get(index * 2..index * 2 + 2).ok_or(RomError::Truncated)?;
            Ok(u16::from_le_bytes([b[0], b[1]]))
        };
        let layer_count = word(0)? as usize;
        let layer_list = word(1)? as usize;
        let mut layers = Vec::with_capacity(layer_count);
        let mut events = Vec::new();
        let mut performers = Vec::with_capacity(layer_count);
        let mut layer_events = Vec::with_capacity(layer_count);
        let mut objects = Vec::with_capacity(layer_count);
        for layer in 0..layer_count {
            let entry = layer_list + layer * LAYER_WORDS;
            // Events: size and position in 8-pixel tiles, half-tile offsets,
            // then the position of a 4-word trigger whose last word is the
            // script number (worked out on the ROM's scenes).
            let (event_count, first_event) = (word(entry + 6)? as usize, word(entry + 7)? as usize);
            layer_events.push(event_count);
            // Objects: number, direction, box size in tiles, then the
            // position like an actor's, the script number and one unknown
            // word (skytemple-files' `ssa_sse_sss` model).
            let (count, first) = (word(entry + 2)? as usize, word(entry + 3)? as usize);
            let mut props = Vec::with_capacity(count);
            for i in 0..count {
                let o = first + i * OBJECT_WORDS;
                let half = |offset: u16| if offset >= 2 { 4 } else { 0 };
                let script = word(o + 8)?;
                props.push(SceneObject {
                    object: word(o)?,
                    width: word(o + 2)? as i16 as i32 * 8,
                    height: word(o + 3)? as i16 as i32 * 8,
                    x: word(o + 4)? as i32 * 8 + half(word(o + 6)?),
                    y: word(o + 5)? as i32 * 8 + half(word(o + 7)?),
                    script: (script != 0xFFFF).then_some(script),
                });
            }
            objects.push(props);
            for i in 0..event_count {
                let e = first_event + i * EVENT_WORDS;
                let trigger = word(e + 6)? as usize;
                events.push(SceneEvent {
                    width: word(e)? as i32 * 8,
                    height: word(e + 1)? as i32 * 8,
                    x: word(e + 2)? as i32 * 8,
                    y: word(e + 3)? as i32 * 8,
                    script: word(trigger + 3)?,
                    routine: word(trigger)?,
                    unknown: (word(trigger + 1)?, word(trigger + 2)?),
                });
            }
            // Performers: number, direction, a size, then the position
            // like an actor's (skytemple-files' `ssa_sse_sss` model).
            let (count, first) = (word(entry + 4)? as usize, word(entry + 5)? as usize);
            let mut spots = Vec::with_capacity(count);
            for i in 0..count {
                let p = first + i * PERFORMER_WORDS;
                let half = |offset: u16| if offset >= 2 { 4 } else { 0 };
                spots.push((word(p)?, word(p + 4)? as i32 * 8 + half(word(p + 6)?), word(p + 5)? as i32 * 8 + half(word(p + 7)?)));
            }
            performers.push(spots);
            let (count, first) = (word(entry)? as usize, word(entry + 1)? as usize);
            let mut actors = Vec::with_capacity(count);
            for i in 0..count {
                // Entity, direction, position in 8-pixel tiles, half-tile
                // offsets, script number, one unknown word.
                let a = first + i * ACTOR_WORDS;
                let half = |offset: u16| if offset >= 2 { 4 } else { 0 };
                let script = word(a + 6)?;
                actors.push(SceneActor {
                    entity: word(a)?,
                    direction: word(a + 1)?,
                    x: word(a + 2)? as i32 * 8 + half(word(a + 4)?),
                    y: word(a + 3)? as i32 * 8 + half(word(a + 5)?),
                    script: (script != 0xFFFF).then_some(script),
                });
            }
            layers.push(actors);
        }
        Ok(Self { layers, events, layer_events, objects, performers })
    }
}

#[cfg(test)]
mod dump {
    /// Lists the events of a scene file:
    /// `DUNGEON_SCENE_DUMP=SCRIPT/T01P04A/enter.sse cargo test scene_dump -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn scene_dump() {
        let Ok(path) = std::env::var("DUNGEON_SCENE_DUMP") else {
            return;
        };
        let rom = crate::nds::test_rom();
        let scene = super::Scene::parse(rom.file(&path).unwrap()).unwrap();
        println!("SCENE {path}: events per layer {:?}", scene.layer_events);
        for event in &scene.events {
            println!("SCENE   {event:?}");
        }
        for (layer, objects) in scene.objects.iter().enumerate() {
            for object in objects {
                println!("SCENE   layer {layer} {object:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::script::{Destination, Script, find_opcodes, load_entities};
    use std::collections::BTreeMap;

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_scenes_place_known_actors() {
        let rom = crate::nds::test_rom();
        let entities = load_entities(&rom).unwrap();
        let mut scenes: Vec<&str> = rom.paths().filter(|p| p.ends_with(".ssa") || p.ends_with(".sse") || p.ends_with(".sss")).collect();
        scenes.sort();
        let (mut actors, mut odd, mut failed) = (0, 0, 0);
        for path in &scenes {
            let Ok(scene) = Scene::parse(rom.file(path).unwrap()) else {
                failed += 1;
                continue;
            };
            for actor in scene.layers.iter().flatten() {
                actors += 1;
                // A plausible actor: a known entity, a direction, a position
                // inside the largest maps, a small script number.
                let plausible = (actor.entity as usize) < entities.len()
                    && actor.direction <= 8
                    && (0..2048).contains(&actor.x)
                    && (0..2048).contains(&actor.y)
                    && actor.script.is_none_or(|s| s < 100);
                odd += !plausible as usize;
            }
        }
        println!("{} scene files, {failed} unreadable, {actors} actors, {odd} implausible", scenes.len());
        assert!(scenes.len() > 1000 && actors > 2000);
        assert!(failed * 100 < scenes.len() && odd * 100 < actors);

        // The town square: its resident actors, and the kinds of routines of
        // their scripts.
        let scene = Scene::parse(rom.file("SCRIPT/T00P01/enter.sse").unwrap()).unwrap();
        for (layer, actors) in scene.layers.iter().enumerate() {
            let names: Vec<&str> = actors.iter().map(|a| entities[a.entity as usize].name.as_str()).collect();
            println!("layer {layer}: {names:?}");
        }
        println!("first actor: {:?}", scene.layers.iter().flatten().next());
        // Which scene files of the level hold other characters?
        for path in scenes.iter().filter(|p| p.starts_with("SCRIPT/T00P01/")) {
            let scene = Scene::parse(rom.file(path).unwrap()).unwrap();
            let mut names: Vec<&str> = scene
                .layers
                .iter()
                .flatten()
                .map(|a| entities[a.entity as usize].name.as_str())
                .filter(|n| !n.starts_with("PLAYER") && !n.starts_with("ATTENDANT"))
                .collect();
            names.sort();
            names.dedup();
            if !names.is_empty() {
                println!("  {path}: {} layers, {names:?}", scene.layers.len());
            }
        }
        let opcodes = find_opcodes(&rom).unwrap();
        let mut kinds: BTreeMap<u16, usize> = BTreeMap::new();
        let mut scripts: Vec<&str> = rom.paths().filter(|p| p.starts_with("SCRIPT/T00P01/enter") && p.ends_with(".ssb")).collect();
        scripts.sort();
        for path in &scripts {
            let script = Script::parse(rom.file(path).unwrap(), &opcodes, 'e').unwrap();
            for routine in &script.routines {
                *kinds.entry(routine.kind).or_default() += 1;
            }
        }
        println!("{} enter scripts in T00P01, routine kinds {kinds:?}", scripts.len());

        // Across the game, how many placed actors have something to say in
        // the script their scene names for them?
        let (mut with_script, mut talkative) = (0, 0);
        for path in scenes.iter().filter(|p| p.ends_with(".sss")) {
            let scene = Scene::parse(rom.file(path).unwrap()).unwrap();
            let stem = path.trim_end_matches(".sss");
            for actor in scene.layers.iter().flatten() {
                let Some(number) = actor.script else { continue };
                let Some(data) = rom.file(&format!("{stem}{number:02}.ssb")) else {
                    continue;
                };
                with_script += 1;
                let script = Script::parse(data, &opcodes, 'e').unwrap();
                talkative += !script.talk_lines(&opcodes, actor.entity).is_empty() as usize;
            }
        }
        println!("{with_script} station actors with a script file, {talkative} with dialogue");
        // Exits: the events of every level's "enter" scene, and where their
        // scripts lead.
        let levels = crate::script::load_levels(&rom).unwrap();
        let (mut events, mut to_level, mut to_dungeon, mut bad) = (0, 0, 0, 0);
        for path in scenes.iter().filter(|p| p.ends_with("/enter.sse")) {
            let scene = Scene::parse(rom.file(path).unwrap()).unwrap();
            let folder = path.trim_end_matches("enter.sse");
            for event in &scene.events {
                events += 1;
                // Areas lie inside the largest maps.
                bad += !((0..2048).contains(&event.x) && (0..2048).contains(&event.y)) as usize;
                let Some(data) = rom.file(&format!("{folder}enter{:02}.ssb", event.script)) else {
                    continue;
                };
                let script = Script::parse(data, &opcodes, 'e').unwrap();
                match script.destination(&opcodes) {
                    Some(Destination::Level(id)) => {
                        to_level += 1;
                        bad += (id as usize >= levels.len()) as usize;
                    }
                    Some(Destination::Dungeon(id)) => {
                        to_dungeon += 1;
                        bad += (id as usize >= 180) as usize;
                    }
                    None => {}
                }
            }
        }
        println!("{events} events: {to_level} lead to a level, {to_dungeon} to a dungeon, {bad} implausible");
        // The rest run scripts that do something else (shops, cutscenes...).
        assert!(to_level > 50 && to_dungeon > 5 && bad == 0);
        assert!(talkative > 200);
    }
}
