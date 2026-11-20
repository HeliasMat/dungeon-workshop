//! What a fight's line is made of: who is in it and on what side, what the
//! film knows of them, the bars, who watches, the passages played at a
//! moment of it; a fight ending.

use std::collections::BTreeSet;

use crate::film::Film;
use crate::ir::Instr;
use crate::names::species;
use crate::screen::capitalized;
use crate::words::plain;
use crate::{emit, fields};

/// The level of who no line gives one.
const USUAL_LEVEL: i32 = 10;

/// The number of the monster someone is now, evolved or not.
pub(super) fn species_of(film: &mut Film, who: &str) -> i32 {
    let form = film.staging.evolved.iter().find(|(_, old)| *old == who).map_or(who.to_string(), |(new, _)| new.clone());
    let code = film.actor(&form).unwrap_or_default();
    species(&code).and_then(|number| number.parse().ok()).unwrap_or(-1)
}

pub(super) type Sides = [Vec<String>; 2];

/// What opens a line of a fight.
pub(super) fn head_of(film: &mut Film, sort: &str, sides: &Sides, level: i32) -> Vec<String> {
    let seed = film.draw();
    let mut fields = fields!["fight", sort, "level", level, "seed", seed, "place", film.level, "side"];
    fields.extend(sides[0].iter().cloned());
    fields.push("side".to_string());
    fields.extend(sides[1].iter().cloned());
    fields
}

pub(super) fn everyone(sides: &Sides) -> BTreeSet<String> {
    sides.iter().flatten().cloned().collect()
}

/// What the film knows of those in a line.
pub(super) fn about_those_in_it(film: &mut Film, those: &BTreeSet<String>) -> Vec<String> {
    let mut fields = Vec::new();
    for who in those {
        if let Some(level) = film.combat.levels.get(who) {
            fields.extend(fields!["lv", who, level]);
        }
        if film.staging.evolved.values().any(|old| old == who) {
            fields.extend(fields!["species", who, species_of(film, who)]);
        }
    }
    fields
}

/// Who a fight going on goes by, and its bars.
pub(super) fn bars_said(film: &Film) -> Vec<String> {
    let mut fields = Vec::new();
    if let Some(lead) = &film.combat.lead {
        fields.extend(fields!["lead", lead]);
    }
    for (who, most) in &film.combat.bars {
        fields.extend(fields!["bar", who, most]);
    }
    fields
}

/// All those of the film's groups, who watch if they stand there.
pub(super) fn group_members(film: &Film) -> BTreeSet<String> {
    film.groups.values().flatten().cloned().collect()
}

pub(super) fn watching(film: &Film, those: &BTreeSet<String>) -> Vec<String> {
    named_after("audience", group_members(film).into_iter().filter(|name| !those.contains(name)))
}

/// A word and the names after it; nothing if there are none.
pub(super) fn named_after(word: &str, names: impl IntoIterator<Item = String>) -> Vec<String> {
    let names: Vec<String> = names.into_iter().collect();
    if names.is_empty() { names } else { std::iter::once(word.to_string()).chain(names).collect() }
}

/// A line of the film's own.
pub(super) fn by_hand(film: &mut Film, sort: &str, sides: Sides, said: Vec<String>) {
    let those = everyone(&sides);
    let mut fields = head_of(film, sort, &sides, USUAL_LEVEL);
    fields.extend(about_those_in_it(film, &those));
    fields.extend(bars_said(film));
    fields.extend(watching(film, &those));
    fields.extend(said);
    film.host(fields);
}

/// What was written to come in the fight, each passage under its label, gone
/// round: the fight sends the story there at the right moment.
pub(super) fn passages(film: &mut Film, number: usize) -> Vec<String> {
    let mut said = Vec::new();
    if film.combat.moments.is_empty() && film.combat.asides.is_empty() {
        return said;
    }
    film.combat.written += 1;
    let written = film.combat.written;
    let past = format!("fight{written}_past");
    film.beat(emit::goto(past.clone()));
    for (at, (who, share, lines)) in std::mem::take(&mut film.combat.moments).into_iter().enumerate() {
        let label = format!("fight{written}_{at}");
        film.beat(emit::label(label.clone()));
        for line in lines {
            film.play(&Instr { number, span: None, ..line });
        }
        film.beat(emit::back());
        said.extend(fields!["when", who, share, label]);
    }
    for (label, number, line) in std::mem::take(&mut film.combat.asides) {
        film.beat(emit::label(label.clone()));
        film.play(&Instr { number, span: None, ..line });
        film.beat(emit::back());
    }
    film.beat(emit::label(past));
    said
}

pub(super) struct Whole<'a> {
    pub sort: &'a str,
    pub level: i32,
    pub winner: Option<String>,
    /// (A `fight` shows it, its hud going on.)
    pub kept: bool,
    /// (What it says past all that.)
    pub more: Vec<String>,
}

/// The line of a whole fight.
pub(super) fn fight_line(film: &mut Film, number: usize, sides: Sides, whole: Whole) {
    let said = passages(film, number);
    let mut fields = head_of(film, whole.sort, &sides, whole.level);
    let those = everyone(&sides);
    if let Some(winner) = whole.winner {
        fields.extend(fields!["winner", winner]);
    }
    if whole.kept {
        fields.push("keep".to_string());
        fields.extend(bars_said(film));
    }
    fields.extend(about_those_in_it(film, &those));
    if whole.sort != "brawl" {
        fields.extend(watching(film, &those));
    }
    fields.extend(said);
    fields.extend(whole.more);
    film.host(fields);
}

/// All a fight showed goes; what lasts stays.
pub(super) fn closed(film: &mut Film) {
    let mut fields = fields!["fight", "end"];
    fields.extend(bars_said(film));
    film.host(fields);
    film.combat.end();
}

pub(super) fn named(film: &Film, word: &str) -> String {
    let key = plain(word);
    let name = film.staging.renamed.get(&key).unwrap_or(&key);
    format!("[CS:N]{}[CR]", capitalized(name))
}
