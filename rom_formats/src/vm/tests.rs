use super::*;
use crate::script::{Operation, Routine};

/// A script from operations given by name; every operation takes one
/// word of code per parameter plus one, as in the game's files.
pub(crate) fn script(operations: &[(&str, &[u16])], strings: &[&str]) -> Program {
    let mut opcodes: Vec<Opcode> = Vec::new();
    let (mut code, mut offsets, mut offset) = (Vec::new(), Vec::new(), 0);
    for (name, parameters) in operations {
        let opcode = match opcodes.iter().position(|known| known.name == *name) {
            Some(index) => index,
            None => {
                opcodes.push(Opcode { name: name.to_string(), parameters: Some(parameters.len() as u8) });
                opcodes.len() - 1
            }
        };
        offsets.push(offset);
        offset += 1 + parameters.len();
        code.push(Operation { opcode: opcode as u16, parameters: parameters.to_vec() });
    }
    let script = Script {
        routines: vec![Routine { first_operation: 0, kind: 1, target: 0 }],
        operations: code,
        offsets,
        constants: Vec::new(),
        strings: strings.iter().map(|text| text.to_string()).collect(),
    };
    Program { script: Arc::new(script), common: None, opcodes: Arc::new(opcodes), routine: 0 }
}

/// A spoken line: by a character with its portrait, or by no one known.
fn message(speaker: Option<u16>, text: &str) -> Event {
    let face = speaker.map(|_| 0);
    Event::Message { speaker: Some(Face { entity: speaker.unwrap_or(NOBODY), face, mode: 0, named: true }), text: text.to_string() }
}

#[test]
fn branches_follow_the_variables() {
    // if var 7 == 2: "two" else "other"; offsets count code words.
    let program = script(
        &[("message_SetFace", &[42, 0, 0]), ("Branch", &[7, 2, 11]), ("message_Talk", &[1]), ("End", &[]), ("message_Talk", &[0]), ("End", &[])],
        &["two", "other"],
    );
    let mut vars = Vars::default();
    let mut vm = Vm::start(&program);
    assert_eq!(vm.run(&program, &mut vars, |_| 0), message(Some(42), "other"));
    assert_eq!(vm.run(&program, &mut vars, |_| 0), Event::Done);
    vars.set(7, 2);
    let mut vm = Vm::start(&program);
    assert_eq!(vm.run(&program, &mut vars, |_| 0), message(Some(42), "two"));
}

#[test]
fn switches_pick_a_case_or_fall_through() {
    let program = script(
        &[
            ("Switch", &[3]),
            ("Case", &[1, 12]),
            ("CaseValue", &[3, 5, 15]),
            ("message_Talk", &[2]),
            ("End", &[]),
            ("message_Talk", &[0]),
            ("End", &[]),
            ("message_Talk", &[1]),
            ("End", &[]),
        ],
        &["one", "more than five", "default"],
    );
    let line = |value: i32| {
        let mut vars = Vars::default();
        vars.set(3, value);
        Vm::start(&program).run(&program, &mut vars, |_| 0)
    };
    assert_eq!(line(1), message(None, "one"));
    assert_eq!(line(9), message(None, "more than five"));
    assert_eq!(line(4), message(None, "default"));
}

#[test]
fn menus_wait_for_an_answer() {
    let program = script(
        &[
            ("message_SwitchMenu", &[0, 1]),
            ("CaseMenu", &[0, 12]),
            ("CaseMenu", &[1, 16]),
            ("message_Talk", &[2]),
            ("End", &[]),
            ("flag_Set", &[9, 1]),
            ("End", &[]),
            ("main_EnterDungeon", &[3, 0]),
        ],
        &["Yes", "No", "nothing"],
    );
    let mut vars = Vars::default();
    let mut vm = Vm::start(&program);
    let menu = Event::Menu { options: vec!["Yes".to_string(), "No".to_string()] };
    assert_eq!(vm.run(&program, &mut vars, |_| 0), menu);
    vm.choose(1);
    assert_eq!(vm.run(&program, &mut vars, |_| 0), Event::EnterDungeon(3));
    // The other answer sets a variable and ends.
    let mut vm = Vm::start(&program);
    vm.run(&program, &mut vars, |_| 0);
    vm.choose(0);
    assert_eq!(vm.run(&program, &mut vars, |_| 0), Event::Done);
    assert_eq!(vars.get(9), 1);
}

#[test]
fn calls_return_and_loops_end() {
    let program = script(&[("Call", &[5]), ("End", &[]), ("Jump", &[2]), ("flag_CalcValue", &[5, 2, 3]), ("Return", &[])], &[]);
    let mut vars = Vars::default();
    let mut vm = Vm::start(&program);
    assert_eq!(vm.run(&program, &mut vars, |_| 0), Event::Done);
    assert_eq!(vars.get(5), 3);
    // A routine spinning on itself is cut short.
    let program = self::script(&[("Jump", &[0])], &[]);
    assert_eq!(Vm::start(&program).run(&program, &mut vars, |_| 0), Event::Done);
}

/// Runs every script of the ROM's scenes, always taking the first
/// answer. Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
#[test]
#[ignore]
fn real_rom_scripts_run() {
    use crate::script::find_opcodes;
    let rom = crate::nds::test_rom();
    let opcodes = Arc::new(find_opcodes(&rom).unwrap());
    let common = Script::parse(rom.file("SCRIPT/COMMON/unionall.ssb").unwrap(), &opcodes, 'e').unwrap();
    let common = Some(Arc::new(common));
    let mut paths: Vec<&str> = rom.paths().filter(|p| p.ends_with(".ssb")).collect();
    paths.sort();
    let show = std::env::var("DUNGEON_SCRIPT_SHOW").unwrap_or_default();
    let (mut messages, mut menus, mut grounds, mut dungeons, mut cut) = (0, 0, 0, 0, 0);
    let mut empty = Vec::new();
    let (mut acts, mut services) = (0, 0);
    for path in &paths {
        let script = Arc::new(Script::parse(rom.file(path).unwrap(), &opcodes, 'e').unwrap());
        // Jumps land on operations.
        for operation in &script.operations {
            let name = opcodes[operation.opcode as usize].name.as_str();
            if matches!(name, "Jump" | "Call") {
                let target = operation.parameters[0] as usize;
                assert!(script.offsets.contains(&target), "{path}: {name} to {target}");
            }
        }
        let shown = !show.is_empty() && path.contains(&show);
        for routine in 0..script.routines.len() {
            let mut vars = Vars::default();
            let program = Program { script: script.clone(), common: common.clone(), opcodes: opcodes.clone(), routine };
            let mut vm = Vm::start(&program);
            // A script asking for ever is cut short.
            let mut ended = false;
            for _ in 0..5000 {
                let event = vm.run(&program, &mut vars, |_| 0);
                if shown {
                    println!("{path} routine {routine}: {event:?}");
                }
                match event {
                    Event::Message { text, .. } => {
                        if text.is_empty() {
                            empty.push(*path);
                        }
                        messages += 1;
                    }
                    Event::Menu { options } => {
                        assert!(options.iter().all(|o| !o.is_empty()), "{path}");
                        menus += 1;
                        vm.choose(0);
                    }
                    Event::Wait(_) | Event::Lives(..) | Event::WaitFor(_) | Event::Unlock(_) => {}
                    Event::GameMenu(_) => services += 1,
                    Event::Act(_) => acts += 1,
                    Event::EnterGround(_) => grounds += 1,
                    Event::EnterDungeon(_) => dungeons += 1,
                    Event::Done => {
                        ended = true;
                        break;
                    }
                }
            }
            cut += !ended as usize;
        }
    }
    println!(
        "{} scripts: {acts} staging operations, {messages} messages, {menus} menus, {grounds} levels and {dungeons} dungeons entered, {cut} routines cut short",
        paths.len()
    );
    empty.dedup();
    println!("{} scripts with empty messages: {:?}", empty.len(), &empty[..empty.len().min(8)]);
    println!("{services} uses of the bank and the storage");
    assert!(services > 4);
    assert!(messages > 10_000 && menus > 50 && grounds > 50 && dungeons > 5);
    // Characters idling in a loop never end.
    assert!(cut * 5 < paths.len());
}

#[test]
fn the_crossroads_lead_to_the_dungeon_chosen() {
    // The crossroads' script: choose, then leave for the dungeon
    // chosen or say why not. The shared script's routine for entering
    // it is its 670th.
    let mut program = script(
        &[("main_EnterAdventure", &[0x7FFF, 30]), ("Case", &[0, 9]), ("message_Talk", &[0]), ("End", &[]), ("main_EnterAdventure", &[251, 30]), ("Hold", &[])],
        &["staying"],
    );
    let entering = script(&[("main_EnterDungeon", &[0x7FFF, 30])], &[]).script;
    let mut common = Arc::try_unwrap(entering).ok().expect("the only holder");
    common.routines = vec![common.routines[0]; ENTER_FREE_DUNGEON + 1];
    // (Both scripts name their operations alike.)
    let mut opcodes = (*program.opcodes).clone();
    opcodes.push(Opcode { name: "main_EnterDungeon".to_string(), parameters: Some(2) });
    common.operations[0].opcode = opcodes.len() as u16 - 1;
    (program.common, program.opcodes) = (Some(Arc::new(common)), Arc::new(opcodes));
    let mut vars = Vars::default();
    let mut vm = Vm::start(&program);
    assert_eq!(vm.run(&program, &mut vars, |_| 0), Event::GameMenu(rules::jobs::DEPARTURE));
    // A dungeon chosen: the script leaves for it.
    vars.set(DUNGEON_SELECT, 4);
    assert_eq!(vm.clone().run(&program, &mut vars, |_| 0), Event::EnterDungeon(4));
    // None: the script goes on without leaving.
    vars.set(DUNGEON_SELECT, -1);
    assert_eq!(vm.run(&program, &mut vars, |_| 0), message(None, "staying"));
    // An entrance checks its own dungeon, which is then the one chosen.
    let program = script(&[("main_EnterAdventure", &[6, 30]), ("Case", &[0, 7]), ("End", &[]), ("flag_Set", &[9, 1])], &[]);
    Vm::start(&program).run(&program, &mut vars, |_| 0);
    assert_eq!((vars.get(DUNGEON_SELECT), vars.get(9)), (6, 1));
}

#[test]
fn variables_survive_a_save() {
    let mut vars = Vars::default();
    vars.set(7, -3);
    vars.set_at(SCENARIO_MAIN, 1, 12);
    let loaded = Vars::from_text(&format!("level 203\n{}", vars.to_text()));
    assert_eq!((loaded.get(7), loaded.get_at(SCENARIO_MAIN, 1), loaded.get(9)), (-3, 12, 0));
}

#[test]
fn story_progress_compares_chapter_then_step() {
    let program = script(
        &[("flag_SetScenario", &[2, 4, 1]), ("BranchScenarioNowAfter", &[2, 4, 0, 12]), ("message_Talk", &[0]), ("End", &[]), ("message_Talk", &[1])],
        &["before", "after"],
    );
    let mut vars = Vars::default();
    let event = Vm::start(&program).run(&program, &mut vars, |_| 0);
    assert_eq!(event, message(None, "after"));
}
