//! Looking through the game's scripts: run by hand, with the ROM (`DUNGEON_ROM`).

mod histogram {
    /// How often each operation is used: `cargo test opcode_histogram -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn opcode_histogram() {
        use crate::script::{Script, find_opcodes};
        let rom = crate::nds::test_rom();
        let opcodes = find_opcodes(&rom).unwrap();
        let mut counts = vec![0usize; opcodes.len()];
        for path in rom.paths().filter(|p| p.ends_with(".ssb")) {
            let script = Script::parse(rom.file(path).unwrap(), &opcodes, 'e').unwrap();
            for operation in &script.operations {
                counts[operation.opcode as usize] += 1;
            }
        }
        let mut order: Vec<usize> = (0..opcodes.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(counts[i]));
        for i in order {
            println!("HIST {} {:#x} {} {:?}", counts[i], i, opcodes[i].name, opcodes[i].parameters);
        }
    }
}

mod entities {
    #[test]
    #[ignore]
    fn entity_dump() {
        let rom = crate::nds::test_rom();
        let entities = crate::script::load_entities(&rom).unwrap();
        let list: Vec<String> = entities
            .iter()
            .enumerate()
            .filter(|(i, e)| e.monster == 0 || *i < 12 || e.kind != 1)
            .map(|(i, e)| format!("{i}:{}:k{}:m{}", e.name, e.kind, e.monster))
            .collect();
        println!("ENT {}", list.join(" "));
    }
}

mod opening {
    /// Follows the game from its entry routine, playing every scene it
    /// asks for one after the other and taking first answers:
    /// `cargo test opening_trace -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn opening_trace() {
        use super::super::*;

        use crate::script::{find_opcodes, load_levels};
        let rom = crate::nds::test_rom();
        let opcodes = Arc::new(find_opcodes(&rom).unwrap());
        let levels = load_levels(&rom).unwrap();
        let parse = |path: &str| Some(Arc::new(Script::parse(rom.file(path)?, &opcodes, 'e').ok()?));
        let common = parse("SCRIPT/COMMON/unionall.ssb").unwrap();
        let program = |script: Arc<Script>, routine: usize| Program { script, common: Some(common.clone()), opcodes: opcodes.clone(), routine };
        let mut vars = Vars::default();
        vars.set(EXECUTE_SPECIAL_EPISODE_TYPE, NO_EPISODE);
        let entry = program(common.clone(), EVENT_DIVIDE);
        let mut dungeons = Vec::new();
        let (mut roaming, mut roamed) = (None::<(String, String)>, 0);
        // Scripts of the characters, areas and props in place in the level.
        let mut present: Vec<String> = Vec::new();
        // Scripts tried and levels seen, at each point of the story.
        let mut tried = std::collections::HashMap::<((i32, i32), String), u32>::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![(entry.clone(), Vm::start(&entry))];
        let mut shown = 0;
        // `DUNGEON_TRACE=200` goes on for that many scenes, lines unshown.
        let limit = std::env::var("DUNGEON_TRACE").ok().and_then(|n| n.parse().ok());
        let (quiet, limit) = (limit.is_some(), limit.unwrap_or(60));
        for _ in 0..2_000_000 {
            // Free roaming: talk to the station's characters and walk into
            // the level's areas. A script that plays a scene, enters a
            // dungeon or moves the story on is tried first, once for each
            // point of the story; failing that, one that leads to a level
            // not seen at this point of the story.
            if stack.is_empty() {
                let Some((folder, station)) = roaming.clone() else { break };
                if roamed >= 300 {
                    break;
                }
                let before = vars.scenario(SCENARIO_MAIN);
                visited.insert((before, folder.clone()));
                // What a script does first: `Ok` for the story, `Err` for
                // the level it leads to.
                let first = |path: &String| -> Option<Result<(), u16>> {
                    let talk = program(parse(path)?, 0);
                    let (mut vars, mut vm) = (vars.clone(), Vm::start(&talk));
                    (0..3000).find_map(|_| match vm.run(&talk, &mut vars, |_| 0) {
                        Event::Menu { .. } => {
                            vm.choose(0);
                            None
                        }
                        Event::Act(Act::Scene { .. }) | Event::EnterDungeon(_) => Some(Ok(())),
                        Event::EnterGround(level) => Some(Err(level)),
                        Event::Done => Some(Err(u16::MAX)),
                        _ => (vars.scenario(SCENARIO_MAIN) != before).then_some(Ok(())),
                    })
                };
                // Only the scripts of what the level has put in place.
                let _ = &station;
                let scripts: Vec<String> = present.clone();
                let does: Vec<(String, Result<(), u16>)> = scripts.into_iter().filter_map(|path| first(&path).map(|what| (path, what))).collect();
                let unseen = |what: &Result<(), u16>| match what {
                    Err(level) => levels.get(*level as usize).is_some_and(|level| !visited.contains(&(before, level.name.clone()))),
                    Ok(()) => false,
                };
                // Then back through levels already seen, the ways least
                // taken first, a few times each.
                // (A script is worth trying again once a job has been done.)
                let day = (before.0, before.1 * 1000 + vars.get(REQUEST_CLEAR_COUNT));
                let uses = |path: &String| tried.get(&(day, path.clone())).copied().unwrap_or(0);
                let story = does.iter().find(|(path, what)| what.is_ok() && uses(path) == 0);
                let new_level = || does.iter().find(|(_, what)| unseen(what));
                let old_level = || {
                    let ways = does.iter().filter(|(path, what)| matches!(what, Err(level) if (*level as usize) < levels.len()) && uses(path) < 3);
                    ways.min_by_key(|(path, _)| uses(path))
                };
                let Some((path, what)) = story.or_else(new_level).or_else(old_level) else {
                    println!("TRACE nothing more to try in {folder}/{station} at {before:?}");
                    println!("TRACE   what its scripts do first: {does:?}");
                    break;
                };
                println!("TRACE roaming {folder}/{station} at {before:?}: {path} ({what:?})");
                *tried.entry((day, path.clone())).or_insert(0) += 1;
                roamed += 1;
                let talk = program(parse(path).unwrap(), 0);
                let vm = Vm::start(&talk);
                stack.push((talk, vm));
            }
            let Some((program_now, vm)) = stack.last_mut() else { break };
            let event = vm.run(program_now, &mut vars, |_| 0);
            match &event {
                Event::Act(Act::Scene { level, name, station, sub }) => {
                    let folder = &levels[*level as usize].name;
                    println!("TRACE scene {folder}/{name} station {station} sub {sub}");
                    // The level's own scene first, then the station it calls.
                    if *station {
                        roaming = Some((folder.clone(), name.clone()));
                        // Another level: nothing of it is in place yet.
                        if !*sub {
                            present.clear();
                        }
                    }
                    shown += 1;
                    if !*sub {
                        stack.pop();
                    }
                    // A station is played by its first script.
                    let script = match *station {
                        true => format!("SCRIPT/{folder}/{name}00.ssb"),
                        false => format!("SCRIPT/{folder}/{name}.ssb"),
                    };
                    if let Some(script) = parse(&script) {
                        let inner = program(script, 0);
                        let vm = Vm::start(&inner);
                        stack.push((inner, vm));
                    }
                }
                Event::Message { text, .. } if !quiet => {
                    shown += 1;
                    println!("TRACE   says {:?}", text.chars().take(70).collect::<String>());
                }
                Event::Menu { options } => {
                    shown += 1;
                    println!("TRACE   asks {options:?}");
                    vm.choose(0);
                }
                Event::Act(Act::Ground(level)) => println!("TRACE   ground {}", levels[*level as usize].name),
                // A layer of the level's own scene or of its station: its
                // characters, areas and props, with the scripts they run.
                Event::Act(Act::Layer { cast, layer }) if *cast != Cast::Acting => {
                    if let Some((folder, station)) = &roaming {
                        let (name, extension) = match cast {
                            Cast::Common => (ENTER, "sse"),
                            _ => (station.as_str(), "sss"),
                        };
                        let file = rom.file(&format!("SCRIPT/{folder}/{name}.{extension}"));
                        if let Some(scene) = file.and_then(|data| crate::scene::Scene::parse(data).ok()) {
                            let layer = *layer as usize;
                            let before: usize = scene.layer_events.iter().take(layer).sum();
                            let count = scene.layer_events.get(layer).copied().unwrap_or(0);
                            let events = scene.events.iter().skip(before).take(count).map(|event| event.script);
                            let actors = scene.layers.get(layer).into_iter().flatten().filter_map(|actor| actor.script);
                            let objects = scene.objects.get(layer).into_iter().flatten().filter_map(|object| object.script);
                            for number in events.chain(actors).chain(objects) {
                                let path = format!("SCRIPT/{folder}/{name}{number:02}.ssb");
                                if !present.contains(&path) {
                                    present.push(path);
                                }
                            }
                        }
                    }
                }
                Event::EnterGround(level) => {
                    let folder = &levels[*level as usize].name;
                    println!("TRACE enter ground {folder}");
                    roaming = Some((folder.clone(), ENTER.to_string()));
                    present.clear();
                    shown += 1;
                    stack.clear();
                    if let Some(script) = parse(&format!("SCRIPT/{folder}/{ENTER}00.ssb")) {
                        let inner = program(script, 0);
                        let vm = Vm::start(&inner);
                        stack.push((inner, vm));
                    }
                }
                // Back from a dungeon it cleared, the game runs the routine
                // for that.
                Event::EnterDungeon(id) => {
                    println!("TRACE enter dungeon {id}, story at {:?}", vars.scenario(SCENARIO_MAIN));
                    shown += 1;
                    dungeons.push(*id);
                    vars.set(GROUND_START_MODE, DUNGEON_CLEARED);
                    vars.set(DUNGEON_RESULT, DUNGEON_CLEARED);
                    // In a dungeon open for jobs, the bot does one.
                    let job = getout_routine(&vars) == GETOUT_REQUEST_DUNGEON;
                    vars.set(JOB_DONE_THERE, job as i32);
                    if job {
                        vars.set(REQUEST_CLEAR_COUNT, vars.get(REQUEST_CLEAR_COUNT) + 1);
                    }
                    let getout = program(common.clone(), getout_routine(&vars));
                    stack = vec![(getout.clone(), Vm::start(&getout))];
                }
                Event::Done => {
                    println!("TRACE done (depth {})", stack.len());
                    stack.pop();
                }
                _ => {}
            }
            if shown > limit {
                break;
            }
        }
        println!("TRACE roamed {roamed} times");
        println!("TRACE dungeons entered: {dungeons:?}, story at {:?}", vars.scenario(SCENARIO_MAIN));
        // The story does not go round in circles: clearing a dungeon
        // leads to the next.
        if quiet {
            let mut different = dungeons.clone();
            different.dedup();
            assert!(different.len() >= 2 && different.len() * 2 > dungeons.len(), "{dungeons:?}");
        }
    }
}

mod find {
    /// Lists where an operation is used with given first parameters:
    /// `DUNGEON_FIND="flag_SetScenario 3 3 0" cargo test script_find -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn script_find() {
        use crate::script::{Script, find_opcodes};
        let Ok(wanted) = std::env::var("DUNGEON_FIND") else {
            return;
        };
        let mut words = wanted.split_whitespace();
        let name = words.next().unwrap();
        let parameters: Vec<i16> = words.filter_map(|word| word.parse().ok()).collect();
        let rom = crate::nds::test_rom();
        let opcodes = find_opcodes(&rom).unwrap();
        let mut paths: Vec<&str> = rom.paths().filter(|p| p.ends_with(".ssb")).collect();
        paths.sort();
        for path in paths {
            let script = Script::parse(rom.file(path).unwrap(), &opcodes, 'e').unwrap();
            for (at, operation) in script.operations.iter().enumerate() {
                let p: Vec<i16> = operation.parameters.iter().map(|&p| p as i16).collect();
                if opcodes[operation.opcode as usize].name == name && p.starts_with(&parameters) {
                    println!("FIND {path} @{} {p:?}", script.offsets[at]);
                }
            }
        }
    }
}
