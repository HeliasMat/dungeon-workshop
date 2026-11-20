//! The script engine's data: its opcode table (in the ARM9 binary), the
//! compiled scripts (`SCRIPT/**/*.ssb`) and the tables naming the levels and
//! the actors scripts refer to.
//!
//! Table layouts follow pmdsky-debug's `script_opcode`, `script_level` and
//! `script_entity` structs.

mod tables;
pub use tables::*;
#[cfg(test)]
mod tests;

use crate::bytes::{u16_at, u32_at};
use crate::nds::{NdsRom, RomError};

pub(crate) const OPCODE_COUNT: usize = 383;

/// One operation of the script language.
#[derive(Clone, Debug)]
pub struct Opcode {
    pub name: String,
    /// Number of 16-bit parameters; `None` when the count is the first
    /// parameter word (variadic opcodes).
    pub parameters: Option<u8>,
}

/// A null-terminated ASCII string at address `address` of the ARM9 binary.
fn c_string(base: u32, binary: &[u8], address: u32) -> Option<String> {
    let start = address.checked_sub(base)? as usize;
    let bytes = binary.get(start..)?;
    let end = bytes.iter().take(64).position(|&b| b == 0)?;
    let text = &bytes[..end];
    (end > 0 && text.iter().all(|b| b.is_ascii_graphic())).then(|| String::from_utf8_lossy(text).into_owned())
}

/// Finds the opcode table: 383 consecutive 8-byte entries `parameter
/// count, string index, 2 unknown bytes, name pointer` whose names are all
/// valid strings. Its address is not documented, so it is searched for, in
/// the ARM9 binary and in the ground-mode overlay (11); this works for
/// every region.
pub fn find_opcodes(rom: &NdsRom) -> Result<Vec<Opcode>, RomError> {
    [rom.arm9(), rom.overlay(11)].into_iter().flatten().find_map(|(base, binary)| find_opcodes_in(base, binary)).ok_or(RomError::Truncated)
}

fn find_opcodes_in(base: u32, binary: &[u8]) -> Option<Vec<Opcode>> {
    let entry = |off: usize| -> Option<Opcode> {
        let count = *binary.get(off)? as i8;
        let name = c_string(base, binary, u32_at(binary, off + 4)?)?;
        (-1..=32).contains(&count).then(|| Opcode { name, parameters: (count >= 0).then_some(count as u8) })
    };
    let mut off = 0;
    while off + OPCODE_COUNT * 8 <= binary.len() {
        // Count how many valid entries start here; skip past a failure.
        match (0..OPCODE_COUNT).find(|i| entry(off + i * 8).is_none()) {
            None => {
                return Some((0..OPCODE_COUNT).filter_map(|i| entry(off + i * 8)).collect());
            }
            Some(bad) => off += (bad + 1) * 8 - if bad == 0 { 4 } else { 0 },
        }
    }
    None
}

/// One routine of a script: where its code starts and what it is attached to.
#[derive(Clone, Copy, Debug)]
pub struct Routine {
    /// Index of its first operation in [`Script::operations`].
    pub first_operation: usize,
    pub kind: u16,
    /// The actor, object or performer the routine belongs to, per `kind`.
    pub target: u16,
}

#[derive(Clone, Debug)]
pub struct Operation {
    pub opcode: u16,
    pub parameters: Vec<u16>,
}

/// A compiled script (European layout: strings in five languages).
///
/// Layout, worked out on the ROM's scripts: a header of nine 16-bit words
/// (constant count, string count, length of the code block, length of the
/// constant strings, then the length of each language's strings, all in
/// words), the code block, the constants, and the string blocks. The code
/// block starts with the offset of its end and the routine count, followed
/// by three words per routine (code offset, kind, target) and the
/// operations: an opcode word, then its parameter words.
pub struct Script {
    pub routines: Vec<Routine>,
    pub operations: Vec<Operation>,
    /// Where each operation starts, in words from the start of the code
    /// block: what jumps give as their target.
    pub offsets: Vec<usize>,
    pub constants: Vec<String>,
    /// The strings of one language.
    pub strings: Vec<String>,
}

/// Number of languages in a European script, and their order.
const LANGUAGES: [char; 5] = ['e', 'f', 'g', 'i', 's'];
const HEADER_WORDS: usize = 4 + LANGUAGES.len();

impl Script {
    /// `language` is the letter used by the text files (e, f, g, i or s).
    pub fn parse(data: &[u8], opcodes: &[Opcode], language: char) -> Result<Self, RomError> {
        let word = |index: usize| u16_at(data, index * 2).ok_or(RomError::Truncated);
        let (constant_count, string_count) = (word(0)? as usize, word(1)? as usize);
        let (code_words, constant_words) = (word(2)? as usize, word(3)? as usize);
        let code = HEADER_WORDS;
        let code_end = code + word(code)? as usize;
        let routine_count = word(code + 1)? as usize;

        // Operations run from the end of the routine table to the end of
        // the code; remember where each one starts to resolve the routines.
        let mut operations = Vec::new();
        let mut starts = Vec::new();
        let mut position = code + 2 + routine_count * 3;
        while position < code_end {
            starts.push(position - code);
            let opcode = word(position)?;
            let known = opcodes.get(opcode as usize).ok_or(RomError::Truncated)?;
            position += 1;
            let count = match known.parameters {
                Some(count) => count as usize,
                None => {
                    position += 1;
                    word(position - 1)? as usize
                }
            };
            let parameters = (0..count).map(|i| word(position + i)).collect::<Result<_, _>>()?;
            position += count;
            operations.push(Operation { opcode, parameters });
        }
        if position != code_end {
            return Err(RomError::Truncated);
        }

        let mut routines = Vec::with_capacity(routine_count);
        for r in 0..routine_count {
            let entry = code + 2 + r * 3;
            let offset = word(entry)? as usize;
            routines.push(Routine {
                // A routine may start right at the end of the code (empty).
                first_operation: starts.partition_point(|&start| start < offset),
                kind: word(entry + 1)?,
                target: word(entry + 2)?,
            });
        }

        // Constants: one offset word each (inside the code block, after the
        // operations), then the null-terminated strings.
        let constants_start = (code + code_words) * 2;
        let constants_bytes = data.get(constants_start..constants_start + constant_words * 2).ok_or(RomError::Truncated)?;
        let constants = split_strings(constants_bytes, constant_count);

        // One block per language: an offset word per string, then the text.
        let language = LANGUAGES.iter().position(|&l| l == language).unwrap_or(0);
        let mut block = constants_start + constant_words * 2;
        for skipped in 0..language {
            block += word(4 + skipped)? as usize * 2;
        }
        let length = word(4 + language)? as usize * 2;
        let text = data.get(block..block + length).ok_or(RomError::Truncated)?;
        let strings = split_strings(text.get(string_count * 2..).unwrap_or(&[]), string_count);
        Ok(Self { routines, operations, offsets: starts, constants, strings })
    }
}

impl Script {
    /// Index of the operation starting at `offset` words of the code block
    /// (the number of operations if it is the end of the code).
    pub(crate) fn operation_at(&self, offset: usize) -> usize {
        self.offsets.partition_point(|&start| start < offset)
    }
}

/// Where a script sends the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) enum Destination {
    /// Index into the level table.
    Level(u16),
    /// Dungeon id.
    Dungeon(u16),
}

impl Script {
    /// The first place the script leaves for: the first parameter of its
    /// first `main_EnterGround` or `main_EnterDungeon`.
    // (For tests: the script's control flow is ignored, so a script choosing
    // between several places gives the first.)
    #[cfg(test)]
    pub(crate) fn destination(&self, opcodes: &[Opcode]) -> Option<Destination> {
        self.operations.iter().find_map(|operation| {
            let target = *operation.parameters.first()?;
            match opcodes.get(operation.opcode as usize)?.name.as_str() {
                "main_EnterGround" => Some(Destination::Level(target)),
                "main_EnterDungeon" => Some(Destination::Dungeon(target)),
                _ => None,
            }
        })
    }
}

/// Routine kinds: a script's own routine, and one attached to an actor.
#[cfg(test)]
const ROUTINE_GENERIC: u16 = 1;
const ROUTINE_ACTOR: u16 = 3;

impl Script {
    /// The text a string parameter refers to: parameters number the
    /// constants first, then the strings.
    pub fn text(&self, parameter: u16) -> Option<&str> {
        let index = (parameter as usize).checked_sub(self.constants.len())?;
        self.strings.get(index).map(String::as_str)
    }

    /// The routine talking to the actor `entity` runs: the one attached to
    /// it, else the script's first (a scene gives each of its actors a
    /// script file of their own).
    pub fn actor_routine(&self, entity: u16) -> usize {
        let attached = |r: &Routine| r.kind == ROUTINE_ACTOR && r.target == entity;
        self.routines.iter().position(attached).unwrap_or(0)
    }

    /// What the actor `entity` says when talked to: the text of every
    /// `message_Talk` of the routines attached to it, in order. A scene
    /// gives each of its actors a script file of their own, made of one
    /// generic routine: those count too.
    // (For tests: the script's control flow is ignored, lines of different
    // situations follow each other.)
    #[cfg(test)]
    pub(crate) fn talk_lines(&self, opcodes: &[Opcode], entity: u16) -> Vec<&str> {
        let mut starts: Vec<usize> = self.routines.iter().map(|r| r.first_operation).collect();
        starts.sort_unstable();
        let mut lines = Vec::new();
        for routine in &self.routines {
            let attached = routine.kind == ROUTINE_ACTOR && routine.target == entity;
            if !attached && routine.kind != ROUTINE_GENERIC {
                continue;
            }
            let end = starts.iter().copied().find(|&start| start > routine.first_operation).unwrap_or(self.operations.len());
            for operation in &self.operations[routine.first_operation.min(end)..end] {
                let name = opcodes.get(operation.opcode as usize).map(|o| o.name.as_str());
                if name == Some("message_Talk")
                    && let Some(text) = operation.parameters.last().and_then(|&p| self.text(p))
                {
                    lines.push(text);
                }
            }
        }
        lines
    }
}

/// Removes the game's formatting tags (`[K]`, `[CS:N]`...) from a line of
/// dialogue, putting `hero` where the leader's name goes.
// (For tests: the other name tags are dropped, colours and pauses ignored.)
#[cfg(test)]
pub(crate) fn plain_text(text: &str, hero: &str) -> String {
    let mut plain = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        plain.push_str(&rest[..open]);
        let Some(close) = rest[open..].find(']') else {
            rest = &rest[open..];
            break;
        };
        if &rest[open..open + close + 1] == "[hero]" {
            plain.push_str(hero);
        }
        rest = &rest[open + close + 1..];
    }
    plain.push_str(rest);
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The first `count` null-terminated strings of `bytes`, decoded like the
/// game text (a single-byte Western encoding).
fn split_strings(bytes: &[u8], count: usize) -> Vec<String> {
    bytes.split(|&b| b == 0).take(count).map(super::text::decode_text).collect()
}
