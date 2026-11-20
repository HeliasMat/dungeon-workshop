//! Runs the game's compiled scripts (`rom_formats::script::Script`).
//!
//! The operations and what they do follow the SkyTemple project's
//! ExplorerScript (`explorerscript/ssb_converting`: `ssb_special_ops.py`,
//! the decompiler's `if`, `switch` and `flag` handlers; MIT licence): which
//! operations jump and on what condition, how switches and their cases
//! chain, how variables are assigned. Parameter meanings of the staging
//! operations (moves, turns, fades, scenes) are those named by
//! skytemple-files' `pmd2scriptdata.xml`.
//!
//! What a script asks of the game comes out as `Event`s (staging, sound
//! and music, effects, the top screen...); operations this engine has no
//! use for yet are stepped over. One `Vm` runs one routine; [`crate::acting::Acting`] runs
//! a scene's routines side by side.

mod vars;
pub use vars::*;
mod names;
pub use names::*;
mod acts;
pub use acts::*;
mod branches;
mod characters;
mod flags;
mod flow;
mod numbers;
use numbers::*;
mod scenes;
mod screen;
mod talk;

use std::collections::HashMap;
use std::sync::Arc;

use crate::script::{Opcode, Script};

/// A routine of a script, ready to be run.
#[derive(Clone)]
pub struct Program {
    pub script: Arc<Script>,
    /// The game's shared routines (`SCRIPT/COMMON/unionall.ssb`), which
    /// scripts call by number.
    pub common: Option<Arc<Script>>,
    pub opcodes: Arc<Vec<Opcode>>,
    pub routine: usize,
}

/// The dungeons the story has opened, with their mode: to visit (1), for
/// jobs (2), or both (3).
pub fn open_dungeons(vars: &Vars) -> Vec<(u16, i32)> {
    let mode = |dungeon: u16| (dungeon, vars.get_at(DUNGEON_MODES, dungeon));
    (0..DUNGEONS).chain([WAY_TO_SPRING]).map(mode).filter(|&(_, mode)| mode > 0).collect()
}

/// The routine to run when the dungeon just left is left.
// (For tests: a dungeon the story has opened for jobs is taken to be left as
// one.)
#[cfg(test)]
pub(crate) fn getout_routine(vars: &Vars) -> usize {
    match vars.get(DUNGEON_ENTER_MODE) {
        MODE_REQUEST | MODE_OPEN_AND_REQUEST => GETOUT_REQUEST_DUNGEON,
        _ => GETOUT_SCENARIO_DUNGEON,
    }
}

/// One routine being run.
#[derive(Clone, Debug, PartialEq)]
pub struct Vm {
    /// Whether the next operation is in the shared script.
    common: bool,
    /// Index of the next operation.
    pc: usize,
    calls: Vec<(bool, usize)>,
    /// Where a routine made of a single operation ends.
    stop: Option<(bool, usize)>,
    speaker: Option<Face>,
    /// The value the current `Switch` compares its cases with.
    switch: i32,
    /// Where each option of the menu being shown leads.
    menu: Vec<usize>,
    /// The player is choosing a dungeon: the cases that follow read
    /// whether one was chosen.
    choosing: bool,
}

/// Operations run at most between two events: a script looping forever
/// without asking anything is stopped.
const STEP_LIMIT: usize = 20_000;

fn name_at<'a>(script: &'a Script, program: &'a Program, index: usize) -> &'a str {
    let opcode = script.operations.get(index).and_then(|operation| program.opcodes.get(operation.opcode as usize));
    opcode.map_or("", |opcode| opcode.name.as_str())
}

/// One operation about to run, with where it comes from.
struct Op<'a> {
    script: &'a Script,
    program: &'a Program,
    name: &'a str,
    p: &'a [u16],
}

impl<'a> Op<'a> {
    fn at(&self, index: usize) -> u16 {
        self.p.get(index).copied().unwrap_or(0)
    }

    fn value(&self, index: usize) -> i32 {
        signed(self.at(index))
    }

    fn text(&self, parameter: u16) -> String {
        self.script.text(parameter).unwrap_or_default().to_string()
    }

    fn constant(&self, parameter: u16) -> String {
        self.script.constants.get(parameter as usize).cloned().unwrap_or_default()
    }

    /// The name of the operation at that place of its script.
    fn name_at(&self, index: usize) -> &'a str {
        name_at(self.script, self.program, index)
    }
}

/// What comes after an operation.
enum Step {
    /// The next one.
    On,
    /// The one that parameter points to.
    Jump(usize),
    /// None yet: the game is asked something.
    Ask(Event),
    /// None: the routine is over.
    End,
}

impl Step {
    fn jump_if(taken: bool, parameter: usize) -> Self {
        if taken { Self::Jump(parameter) } else { Self::On }
    }
}

fn act(act: Act) -> Event {
    Event::Act(act)
}

impl Vm {
    /// Ready to run the program's routine.
    pub fn start(program: &Program) -> Self {
        Self::at(false, Self::first(&program.script, program.routine))
    }

    /// Ready to run routine `routine` of the shared script.
    pub fn start_common(program: &Program, routine: usize) -> Self {
        match &program.common {
            Some(common) => Self::at(true, Self::first(common, routine)),
            None => Self::at(false, usize::MAX),
        }
    }

    fn first(script: &Script, routine: usize) -> usize {
        script.routines.get(routine).map_or(usize::MAX, |r| r.first_operation)
    }

    fn at(common: bool, pc: usize) -> Self {
        Self { common, pc, calls: Vec::new(), stop: None, speaker: None, switch: 0, menu: Vec::new(), choosing: false }
    }

    /// Who says a line: the character scripts named last or, for a spoken
    /// line without one, nobody known.
    fn voice(&self, named: bool) -> Option<Face> {
        match self.speaker {
            Some(speaker) => Some(Face { named, ..speaker }),
            None if named => Some(Face { entity: NOBODY, face: None, mode: 0, named }),
            None => None,
        }
    }

    /// Answers the menu last returned by [`Self::run`].
    pub fn choose(&mut self, option: usize) {
        if let Some(&target) = self.menu.get(option) {
            self.pc = target;
        }
        self.menu.clear();
    }

    /// Runs until the script asks something of the game. `random(n)` draws
    /// a number below `n`.
    pub fn run(&mut self, program: &Program, vars: &mut Vars, mut random: impl FnMut(i32) -> i32) -> Event {
        if std::mem::take(&mut self.choosing) {
            self.switch = (vars.get(DUNGEON_SELECT) < 0) as i32;
        }
        for _ in 0..STEP_LIMIT {
            if self.calls.is_empty() && self.stop == Some((self.common, self.pc)) {
                break;
            }
            let script: &Script = match (self.common, &program.common) {
                (true, Some(common)) => common,
                _ => &program.script,
            };
            let Some(operation) = script.operations.get(self.pc) else {
                break;
            };
            let op = Op { script, program, name: name_at(script, program, self.pc), p: &operation.parameters };
            self.pc += 1;
            match self.step(&op, vars, &mut random) {
                Step::On => {}
                // (Jumps give the offset of their target in the code.)
                Step::Jump(parameter) => self.pc = script.operation_at(op.at(parameter) as usize),
                Step::Ask(event) => return event,
                Step::End => break,
            }
        }
        self.pc = usize::MAX;
        Event::Done
    }

    /// One operation, by its family. (Those of no family are stepped over.)
    fn step(&mut self, op: &Op<'_>, vars: &mut Vars, random: &mut dyn FnMut(i32) -> i32) -> Step {
        let asked = |event: Option<Event>| event.map(Step::Ask);
        self.flow(op)
            .or_else(|| branches::branch(op, vars))
            .or_else(|| self.game_menu(op))
            .or_else(|| self.enter_adventure(op, vars))
            .or_else(|| self.switch(op, vars, random))
            .or_else(|| self.case(op, vars))
            .or_else(|| self.talk(op, vars, random))
            .or_else(|| flags::flags(op, vars))
            .or_else(|| asked(flags::enter(op, vars)))
            .or_else(|| asked(characters::characters(op)))
            .or_else(|| asked(screen::screen(op)))
            .or_else(|| asked(screen::backdrop(op)))
            .or_else(|| asked(scenes::scenes(op, vars)))
            .or_else(|| asked(scenes::waits(op)))
            .unwrap_or(Step::On)
    }
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod rom_tests;
