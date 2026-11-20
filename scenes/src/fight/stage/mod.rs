//! What a fight looks like: each `Sight` as the lines of a scene, the one
//! place that says how. Blows run up to their target or throw themselves at
//! it, push, blink, shake the screen and show their numbers; bars go down
//! little by little, the log tells it all, those who watch react.

use std::collections::VecDeque;

use glam::Vec2;

use super::battle::{DIG, DIVE, FLY};
use super::crowd::Rush;
use super::*;
use crate::fight::said;
use crate::vocabulary::POKEMON;
use engine::{Beat, FlowBeat};
use rules::dungeon::DungeonRng;
use rules::dungeon::combat::matchup;

/// What the game says of a blow, by how the types fare.
fn fares_said(fares: u8) -> Option<&'static str> {
    match fares {
        matchup::IMMUNE => Some("It has no effect..."),
        matchup::NOT_VERY_EFFECTIVE => Some("It's not very effective..."),
        matchup::SUPER_EFFECTIVE => Some("It's super effective!"),
        _ => None,
    }
}

/// What friends shout, when it goes badly and when it goes well.
const HOLD_ON: [&str; 3] = ["Hang in there!", "Get up!", "Don't give up!"];
const WELL_DONE: [&str; 3] = ["Well done!", "Yes!", "Again!"];
/// What is shouted, sometimes.
const THRILLED: [&str; 3] = ["Take that!", "There you go!", "Here!"];
const HURT: [&str; 3] = ["Ow!", "Ouch!", "Argh!"];
/// From how much of its life a blow is felt: time stands nearly still, that
/// slow, for that many frames.
const FELT_SHARE: i32 = 18;
/// The frames the view takes to frame the field, once, as a fight starts.
const FRAMING: i32 = 40;
const FELT_SLOW: f32 = 0.3;
const FELT_FRAMES: i32 = 4;
/// How many of those who watch react, and how close the fighters step.
const CHEERING: usize = 14;
const STEP: i32 = 22;
/// The moves that take a moment to gather; how a type lights the screen, by
/// its number; what shakes the ground; from what power.
const GATHERED: i32 = 50;
const BIG: i32 = 40;
const SHAKING: [u8; 2] = [9, 13];
fn lit(kind: u8) -> Option<&'static str> {
    Some(match kind {
        2 => "orange",
        3 => "blue",
        4 => "green",
        5 => "yellow",
        6 => "cyan",
        11 => "pink",
        14..=16 => "purple",
        _ => return None,
    })
}
/// Solar Beam gathers for longer.
const SOLAR_BEAM: u16 = 151;
/// The sound of a blow that misses a foe, then a member of the team.
const MISS_SOUNDS: [u16; 2] = [4870, 4871];
/// What the game shows of a stat going up and down, one after the other.
const STAT_SIGHT_FRAMES: i32 = 30;
fn stat_sights(stat: Stat, up: bool) -> &'static [u16] {
    match (stat, up) {
        (Stat::Attack | Stat::SpAttack, true) => &[425, 2],
        (Stat::Attack | Stat::SpAttack, false) => &[404, 403],
        (Stat::Defense | Stat::SpDefense, true) => &[2, 400],
        (Stat::Defense | Stat::SpDefense, false) => &[399, 401],
        (Stat::Accuracy | Stat::Evasion, true) => &[396, 13],
        (Stat::Accuracy | Stat::Evasion, false) => &[397, 14],
        (Stat::Speed, true) => &[395],
        (Stat::Speed, false) => &[2],
    }
}
fn stat_name(stat: Stat) -> &'static str {
    match stat {
        Stat::Attack => "Attack",
        Stat::Defense => "Defense",
        Stat::SpAttack => "Sp. Atk.",
        Stat::SpDefense => "Sp. Def.",
        Stat::Accuracy => "Accuracy",
        Stat::Evasion => "Evasion",
        Stat::Speed => "Speed",
    }
}
/// The frames from a move's start to its blow (see `SHOT_FRAMES`).
const HIT_FRAMES: i32 = 16;
/// How long a blow pushes someone; how fast one runs, a frame; how far a
/// blow pushes, at the least and at the most; how far someone goes at what
/// it strikes.
const PUSHED_FOR: i32 = 14;
const RUNS: f32 = 2.6;
const PUSHED: (i32, i32) = (10, 30);
const LUNGE: f32 = 18.0;
/// The steps a bar goes down by; below how much of its life someone is in
/// danger.
const BAR_STEPS: i32 = 10;
const LOW_SHARE: i32 = 25;
/// How one frozen shows.
const FROZEN_COLOUR: [i32; 3] = [140, 190, 255];
/// How far from the fight one who was changed goes.
const BENCH: f32 = 60.0;
/// The points a win gives, by level; how often it gives a level, of a hundred.
const EXPERIENCE: i32 = 11;
const LEVELS_UP: i32 = 50;
/// In a crowd: one blow in so many shows its number; the frames they take
/// to go; those a fainted one blinks; those of a round; those who run take.
const NUMBERS: usize = 3;
const GOING: i32 = 24;
const FAINT_BLINK: i32 = 36;
const ROUND: i32 = 64;
const FLIGHT: i32 = 100;
const EDGE: i32 = 24;
/// The move Protect, shown when one shields itself.
pub(crate) const PROTECT: u16 = 301;

/// A line to play, as the host plays it.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Beat(Beat),
    /// So many frames go by.
    Wait(f32),
    /// Until someone has got where it was going.
    WaitFor(Who),
    /// A passage of the film, by its label.
    Passage(String),
}

/// A bar: what is left, the most, the number that holds it.
#[derive(Clone, Debug)]
struct Bar {
    left: i32,
    most: i32,
    var: &'static str,
}

/// A line for the engine, of fields of any kind.
macro_rules! add {
    ($self:ident, $($field:expr),+ $(,)?) => {
        $self.line(vec![$($field.to_string()),+])
    };
}
mod bars;
mod blows;
mod endings;
mod motions;
mod reactions;
mod rounds;

/// A fight being turned into lines.
#[derive(Clone, Debug, Default)]
pub struct Staging {
    pub steps: VecDeque<Step>,
    /// A line the engine would not read: never, but for a mistake here.
    pub unread: Vec<String>,
    /// Where each stands.
    pub at: Spots,
    /// The names as shown.
    pub names: HashMap<Who, String>,
    bars: HashMap<Who, Bar>,
    ground: Ground,
    /// Who the fight goes by, who cheers for it, who watches.
    lead: Option<Who>,
    pub supporters: Vec<Who>,
    pub public: Vec<Who>,
    chance: Option<DungeonRng>,
    cheered: usize,
    /// A little bar over each, all on the field.
    little_bars: bool,
    ailing: HashMap<Who, Ailment>,
    /// Where each side fights, and where each rests.
    slots: [Vec2; 2],
    benches: HashMap<Who, Vec2>,
    /// No hud of its own: a `combat` has it. In a tournament the music goes
    /// on.
    pub kept_hud: bool,
    pub in_tournament: bool,
    played: bool,
    /// The tournament's end has been told.
    pub named_champion: bool,
    /// Who fights for each side now.
    fighting: [Option<Who>; 2],
    /// Who has fallen, or gone off.
    flown: Vec<Who>,
    /// No log: no fight is going on, a line of the film is struck alone.
    quiet: bool,
    /// Where each stood when it started.
    home: Spots,
    /// Who the two great bars are for, all being on the field.
    pub great_bars: Option<(Who, Who)>,
    /// How fast time goes now, past what a blow does to it (none: as usual);
    /// the frames the next blow is slowed for.
    usual_tempo: Option<f32>,
    slow_hit: Option<i32>,
    /// Who stepped in, and where it comes from.
    stepped: HashMap<Who, Vec2>,
}

impl Staging {
    pub fn new(ground: Ground, seed: u32, at: Spots) -> Self {
        Staging { ground, chance: Some(DungeonRng::new(seed ^ 0x5151)), home: at.clone(), at, ..Staging::default() }
    }

    fn line(&mut self, fields: Vec<String>) {
        match Beat::parse(&fields, &POKEMON) {
            Ok(Beat::Flow(FlowBeat::Wait(frames))) => self.steps.push_back(Step::Wait(frames.0)),
            Ok(Beat::Flow(FlowBeat::WaitFor(who))) => self.steps.push_back(Step::WaitFor(who)),
            Ok(beat) => self.steps.push_back(Step::Beat(beat)),
            Err(_) => self.unread.push(fields.join(" ")),
        }
    }

    /// What time goes back to, past what a blow does to it.
    fn tempo_back(&self) -> f32 {
        self.usual_tempo.unwrap_or(1.0)
    }

    fn roll(&mut self, below: i32) -> i32 {
        self.chance.get_or_insert_with(|| DungeonRng::new(1)).rand_int(below.max(1))
    }

    fn pick<'a>(&mut self, words: &[&'a str]) -> &'a str {
        words[self.roll(words.len() as i32) as usize]
    }

    fn spot(&self, who: Who) -> Option<Vec2> {
        self.at.get(&who).copied()
    }

    fn named(&self, who: Who) -> String {
        format!("[CS:N]{}[CR]", self.names.get(&who).map_or("", String::as_str))
    }

    /// In the log of the fight.
    fn logged(&mut self, text: String) {
        if !self.quiet {
            add!(self, "battlelog", said::log_frames(), text);
        }
    }

    /// In the log of the fight, or as a line of the game where there is none.
    fn told(&mut self, text: String) {
        match self.quiet {
            false => self.logged(text),
            true => {
                let frames = said::read_frames(&text);
                add!(self, "do", "d", "-", "-", format!(" {text}"), frames);
            }
        }
    }

    fn after_logged(&mut self, delay: i32, text: String) {
        if !self.quiet {
            add!(self, "after", delay, "battlelog", 150, text);
        }
    }
}

/// When someone of a crowd strikes, in a round.
fn strike_delay(side: usize, place: usize) -> i32 {
    GOING + (place as i32 * 2) % (ROUND / 2) + side as i32 * 6
}

#[cfg(test)]
mod tests;
