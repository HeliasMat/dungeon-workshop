use super::*;

#[test]
fn tags_are_stripped_from_dialogue() {
    assert_eq!(plain_text("[CS:N]Bob[CR]: hi,\n[hero]![K]", "Ana"), "Bob: hi, Ana!");
    assert_eq!(plain_text("no tags", "Ana"), "no tags");
    assert_eq!(plain_text("broken [tag", "Ana"), "broken [tag");
}

/// Needs a real dump.
#[test]
#[ignore]
fn real_rom_objects_have_sprites() {
    let rom = crate::nds::test_rom();
    let objects = load_objects(&rom).unwrap();
    let named: Vec<&String> = objects.iter().filter(|name| !name.is_empty() && *name != "NULL").collect();
    let (mut drawn, mut missing) = (0, Vec::new());
    for name in &named {
        let Some(file) = rom.file(&format!("GROUND/{name}.wan")) else {
            missing.push(name.as_str());
            continue;
        };
        let sprite = crate::wan::Wan::parse(file).unwrap();
        let visible = |index: usize| sprite.render(index).is_some_and(|p| p.rgba.as_chunks::<4>().0.iter().any(|px| px[3] != 0));
        drawn += (0..sprite.meta_frame_count()).any(visible) as usize;
    }
    println!("{} objects, {} named, {drawn} drawn, no file: {missing:?}", objects.len(), named.len());
    assert!(objects.len() > 500);
    assert_eq!(objects[11], "d01p11b1");
    assert!(drawn * 10 >= named.len() * 9);
    // (`extractor dump objects <rom> <folder>` puts them on a sheet.)
}

/// Needs a real dump.
#[test]
#[ignore]
fn real_rom_places_of_the_world_map_are_read() {
    let rom = crate::nds::test_rom();
    // The first dungeon; one standing where another does; none; the
    // guild; one of another map; past the table.
    assert_eq!(load_map_marker(&rom, 0), Some((268, 212, 180)));
    assert_eq!(load_map_marker(&rom, 3), load_map_marker(&rom, 2));
    assert_eq!(load_map_marker(&rom, 3), Some((268, 212, 196)));
    assert_eq!(load_map_marker(&rom, 10), None);
    assert_eq!(load_map_marker(&rom, 181), Some((268, 212, 180)));
    assert_eq!(load_map_marker(&rom, 177), Some((310, 188, 92)));
    assert_eq!(load_map_marker(&rom, 310), None);
    // Those of the main map fit in one of its eight pictures.
    let main = (0..310).filter_map(|marker| load_map_marker(&rom, marker)).filter(|place| place.0 == 268);
    assert!(main.clone().count() > 150);
    assert!(main.clone().all(|(_, x, y)| (0..504).contains(&x) && (0..336).contains(&y)));
}

/// Needs a real dump.
#[test]
#[ignore]
fn real_rom_moves_show_effects() {
    let rom = crate::nds::test_rom();
    let shown: Vec<[u16; 4]> = (0..MOVE_ANIMATION_COUNT as u16).filter_map(|id| load_move_animation(&rom, id)).collect();
    assert_eq!(shown.len(), MOVE_ANIMATION_COUNT);
    // Most moves show something, with effects the table of effects has.
    let some = shown.iter().filter(|effects| effects.iter().any(|&effect| effect != 0)).count();
    println!("{some} moves of {} show an effect; the first: {:?}", shown.len(), &shown[..6]);
    assert!(some * 2 > shown.len());
    assert!(shown.iter().flatten().all(|&effect| (effect as usize) < EFFECT_COUNT));
    assert_eq!(load_move_animation(&rom, MOVE_ANIMATION_COUNT as u16), None);
    // Their sounds: move 100's is the 2nd of the 1st group, a move without
    // one names the group of the system's sounds, and effect 22's is
    // the 21st of the 1st group.
    assert_eq!(move_sound(&rom, 100), Some(0x0102));
    assert_eq!(move_sound(&rom, 1), Some(0x3F00));
    assert_eq!(effect_sound(&rom, 22), Some(0x0115));
    // Traps and items too.
    let traps: Vec<u16> = (0..TRAP_ANIMATION_COUNT).filter_map(|trap| load_trap_animation(&rom, trap)).collect();
    let items: Vec<[u16; 2]> = (0..ITEM_ANIMATION_COUNT as u16).filter_map(|item| load_item_animation(&rom, item)).collect();
    println!("traps: {traps:?}; items: {:?}", &items[..120]);
    assert_eq!((traps.len(), items.len()), (TRAP_ANIMATION_COUNT, ITEM_ANIMATION_COUNT));
    assert!(traps.iter().chain(items.iter().flatten()).all(|&effect| (effect as usize) < EFFECT_COUNT));
    assert!(traps.iter().filter(|&&effect| effect != 0).count() > 10);
}

/// Needs a real dump.
#[test]
#[ignore]
fn real_rom_effects_are_read() {
    let rom = crate::nds::test_rom();
    // The marks of scenes are animations of the pack's second sprite:
    // "!" once, laughter until it is removed.
    assert_eq!(load_effect(&rom, 88).map(|e| (e.file, e.animation, e.repeats)), Some((1, 0, false)));
    assert_eq!(load_effect(&rom, 93).map(|e| (e.file, e.animation, e.repeats)), Some((1, 5, true)));
    assert_eq!(load_effect(&rom, 171).map(|effect| effect.animation), Some(7));
    // Confetti, on a spot of the scene: another sprite of the pack.
    assert_eq!(load_effect(&rom, 641).map(|e| (e.file, e.animation, e.repeats)), Some((250, 0, false)));
    // One drawn from a texture.
    assert_eq!(load_effect(&rom, 653).map(|e| (e.file, e.animation, e.repeats)), Some((267, 0, true)));
    // No effect; and the rain, which is no sprite.
    assert_eq!(load_effect(&rom, 0), None);
    assert_eq!(load_effect(&rom, 698), None);
    assert!(effect_is_rain(&rom, 698) && !effect_is_rain(&rom, 88));
    // Four pictures of rain, 128 by 64, with drops in each.
    let rain = load_rain(&rom).unwrap();
    assert_eq!(rain.len(), 4);
    for (width, height, rgba) in &rain {
        assert_eq!((*width, *height), (128, 64));
        let drawn = rgba.as_chunks::<4>().0.iter().filter(|pixel| pixel[3] != 0).count();
        assert!(drawn > 100 && drawn < width * height / 2, "{drawn}");
    }
    let drawn = (0..EFFECT_COUNT as u16).filter(|&id| load_effect(&rom, id).is_some()).count();
    println!("{drawn} effects drawn with a sprite");
    assert!(drawn > 300);
}

/// Needs a real dump.
#[test]
#[ignore]
fn real_rom_partner_talk_kinds_are_read() {
    let rom = crate::nds::test_rom();
    let table = load_partner_talk_kinds(&rom).unwrap();
    println!("{table:?}");
    // Five species that speak one way, five (female) another, and
    // the rest.
    assert_eq!(table.len(), 11);
    assert_eq!(table[0], (1, 7));
    assert_eq!(table[5], (3, 883));
    assert_eq!(table[10], (2, 0));
}

/// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
#[test]
#[ignore]
fn real_rom_script_tables_are_found() {
    let rom = crate::nds::test_rom();
    let opcodes = find_opcodes(&rom).unwrap();
    assert_eq!(opcodes.len(), OPCODE_COUNT);
    let variadic = opcodes.iter().filter(|o| o.parameters.is_none()).count();
    println!("opcodes: first {:?}, last {:?}, {variadic} variadic", opcodes[0], opcodes[OPCODE_COUNT - 1]);
    assert!(opcodes.iter().any(|o| o.name == "message_Talk"));

    let levels = load_levels(&rom).unwrap();
    let maps = crate::mapbg::map_list(rom.file("MAP_BG/bg_list.dat").unwrap());
    println!("{} levels, e.g. {:?}", levels.len(), &levels[..3]);
    assert!(levels.len() > 300);
    // Most levels name a map that exists, and have a script folder.
    let with_map = levels.iter().filter(|l| (l.map_id as usize) < maps.len()).count();
    let with_scripts = levels.iter().filter(|l| rom.paths().any(|p| p.starts_with(&format!("SCRIPT/{}/", l.name)))).count();
    println!("{with_map} with a map, {with_scripts} with scripts");
    assert!(with_map * 10 > levels.len() * 9 && with_scripts * 2 > levels.len());

    let animations = load_scene_animations(&rom).unwrap();
    println!("scene animations, e.g. {:?}", &animations[..8]);
    // Walking loops; standing is the walk held still.
    assert!(animations[1].group == 0 && animations[1].looped);
    assert!(animations[2].group == 0 && animations[2].frozen);
    let layouts = load_portrait_layouts(&rom).unwrap();
    println!("portrait layouts, e.g. {:?}", &layouts[..8]);
    assert_eq!(layouts[0], (2, 9, false));
    assert!(layouts.iter().all(|&(x, y, _)| (0..32).contains(&x) && (0..24).contains(&y)));

    // The shops' tables are among the game's 26 item tables.
    let shops = load_shop_tables(&rom).unwrap();
    println!("shop tables {shops:?}");
    assert!(shops.iter().flatten().all(|&table| (1..=26).contains(&table)));
    let lists = load_item_tables(&rom).unwrap();
    let items = crate::items::ItemData::parse(rom.file("BALANCE/item_p.bin").unwrap()).unwrap();
    for table in shops.iter().flatten() {
        let path = format!("item table {table}");
        let list = &lists[*table as usize - 1];
        // Everything a shop stocks has a price.
        let priced = |&(id, _): &(u16, u16)| items.prices(id as usize).is_some_and(|p| p.0 > 0);
        let stocked = list.items.iter().filter(|item| item.1 != 0).count();
        println!("{path}: {stocked} items");
        assert!(stocked > 3 && list.items.iter().filter(|item| item.1 != 0).all(priced), "{path}");
    }

    let entities = load_entities(&rom).unwrap();
    println!("{} entities, e.g. {:?}", entities.len(), &entities[..4]);
    assert!(entities.iter().filter(|e| !e.name.is_empty()).count() > 300);
}

/// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
#[test]
#[ignore]
fn real_rom_scripts_all_decode() {
    let rom = crate::nds::test_rom();
    let opcodes = find_opcodes(&rom).unwrap();
    let mut scripts: Vec<&str> = rom.paths().filter(|p| p.ends_with(".ssb")).collect();
    scripts.sort();
    let (mut operations, mut routines, mut strings, mut failed) = (0, 0, 0, Vec::new());
    let talk = opcodes.iter().position(|o| o.name == "message_Talk").unwrap() as u16;
    let mut talks = 0;
    for path in &scripts {
        match Script::parse(rom.file(path).unwrap(), &opcodes, 'e') {
            Ok(script) => {
                operations += script.operations.len();
                routines += script.routines.len();
                strings += script.strings.len();
                talks += script.operations.iter().filter(|o| o.opcode == talk).count();
                // Every routine starts on an operation (or at the end).
                assert!(script.routines.iter().all(|r| r.first_operation <= script.operations.len()), "{path}");
                // No string is cut short: none is empty when the script
                // has text.
                let empty = script.strings.iter().filter(|s| s.is_empty()).count();
                assert!(empty * 4 <= script.strings.len().max(4), "{path}: {empty} empty strings");
            }
            Err(_) => failed.push(*path),
        }
    }
    println!(
        "{} scripts: {routines} routines, {operations} operations, {strings} strings, {talks} message_Talk, {} failed {:?}",
        scripts.len(),
        failed.len(),
        &failed[..failed.len().min(5)]
    );
    assert!(failed.is_empty());
    assert!(operations > 100_000 && talks > 1000);
}
