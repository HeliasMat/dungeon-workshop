//! What a fight keeps as a film goes.

use std::collections::BTreeMap;

/// What the lines of a fight are written with; a directed fight's moments
/// are written before it starts, and last until it ends.
#[derive(Default)]
pub struct Combat {
    /// (Who it goes by, for the log and the first bar; none out of a fight.)
    pub lead: Option<String>,
    /// (The most of each bar, in order.)
    pub bars: Vec<(String, i32)>,
    pub moments: Vec<(String, i32, Vec<crate::ir::Instr>)>,
    /// (The lines a directed fight has between two blows.)
    pub asides: Vec<(String, usize, crate::ir::Instr)>,
    /// (What each is worth, whatever fight it is in; it lasts, as `written`.)
    pub levels: BTreeMap<String, i32>,
    /// (How many fights wrote passages, to name them.)
    pub written: usize,
}

impl Combat {
    /// All of it goes but what lasts: the one way to end a fight.
    pub fn end(&mut self) {
        *self = Combat { levels: std::mem::take(&mut self.levels), written: self.written, ..Combat::default() };
    }
}
