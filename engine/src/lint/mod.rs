//! What is wrong in a scene file, told line by line: each line read once as
//! the beat it is (`read`), the rules then on those beats.

use std::collections::{HashMap, HashSet};

use glam::Vec2;

use crate::beat::*;
use crate::language::Script;
use crate::problem::ParseError;
use crate::vocabulary::Vocabulary;
use diagnostics::Diagnostic;

mod commands;
mod numbers;
mod places;
mod ranges;
mod read;
#[cfg(test)]
mod tests;
mod ways;

use commands::commands;
pub(crate) use numbers::braces_as_one;
use numbers::numbers_never_given;
use places::outside_backdrops;
use read::{Flow, Is, Read, read};
use ways::{choices, scenes_gone_to, ways};

/// Checks scene files, as much as it is told of what they are for.
#[derive(Clone, Debug, Default)]
pub struct Checker {
    sizes: HashMap<u16, (f32, f32)>,
    host: Option<fn(&[String]) -> bool>,
    words: &'static Vocabulary,
}

impl Checker {
    pub fn new(words: &'static Vocabulary) -> Self {
        Checker { words, ..Checker::default() }
    }

    /// The backdrops' sizes (level: width, height): who is put or sent off
    /// the picture shown is told too.
    pub fn with_sizes(self, sizes: HashMap<u16, (f32, f32)>) -> Self {
        Checker { sizes, ..self }
    }

    /// Whether the host understands a line that is its own: those it does
    /// not are told too.
    pub fn with_host(self, host: fn(&[String]) -> bool) -> Self {
        Checker { host: Some(host), ..self }
    }

    /// All that is wrong in a scene's lines (read by the language), each
    /// read once as its beat; played through if it reads well, for what
    /// goes wrong only when it plays. Remarks last.
    pub fn check_script(&self, script: Script) -> Vec<Diagnostic> {
        let Script { lines, problems: read_wrong } = script;
        let reads = read(&lines, self.words);
        let mut problems = lint(read_wrong, &lines, &reads, self.words);
        problems.extend(outside_backdrops(&reads, &self.sizes));
        if let Some(host) = self.host {
            problems.extend(host_lines(&reads, host, self.words));
        }
        if problems.is_empty() {
            let events = crate::read_events(lines.into_iter().map(|(_, fields)| fields), self.words);
            let rehearsal = crate::rehearsal::rehearse(events, self.words);
            problems.extend(rehearsal.problems);
            problems.extend(rehearsal.remarks);
        }
        problems
    }
}

/// The lines of the host it does not understand. (Those filled when played,
/// and numbers given by name, are read as they will be: a number.)
fn host_lines(reads: &[Read], host: fn(&[String]) -> bool, words: &Vocabulary) -> Vec<Diagnostic> {
    let mut problems = Vec::new();
    for read in reads {
        let (Is::Beat(Beat::Host(fields) | Beat::Course(Course::Host(fields))) | Is::Filled(Ok(Beat::Host(fields) | Beat::Course(Course::Host(fields))))) =
            &read.is
        else {
            continue;
        };
        let word = fields[0].as_str();
        // (As written, its numbers filled as 1.)
        let mut sample: Vec<String> = read.fields.iter().map(|field| braces_as_one(field)).collect();
        if let Some(field) = words.host_numbers.iter().find(|(named, _)| *named == word).and_then(|(_, at)| sample.get_mut(*at)) {
            *field = "1".to_string();
        }
        if !host(&sample) {
            problems.push(Diagnostic::at(read.number, format!("“{word}” badly written: {}", read.fields.join(" "))));
        }
    }
    problems
}

/// What is wrong in a file read, rule after rule: what the reading
/// found, the numbers, then the rest by line.
fn lint(read_wrong: Vec<Diagnostic>, lines: &[(usize, Vec<String>)], reads: &[Read], words: &'static Vocabulary) -> Vec<Diagnostic> {
    let mut by_line = commands(reads, words);
    by_line.extend(ways(reads));
    by_line.extend(choices(reads));
    by_line.extend(scenes_gone_to(reads));
    by_line.extend(ranges::past_advised(reads));
    by_line.sort_by_key(Diagnostic::line);
    let mut problems = read_wrong;
    // (The numbers by the names written: some only filled as it plays.)
    problems.extend(numbers_never_given(lines, words));
    problems.extend(by_line);
    problems
}
