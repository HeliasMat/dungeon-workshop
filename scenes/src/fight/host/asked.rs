//! The line of a fight read ([`Asked`]): its kind, who is on each side,
//! their lives, levels and moves, and what the fight is set up with.

use std::collections::BTreeMap;

use super::super::hand::{Line, Written};
use super::super::*;
use engine::Stage;

/// The kinds of a fight, as the line says them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sort {
    Played,
    Alone,
    Together,
    Tournament,
    Crowd,
    /// Stated in advance, all on the field.
    Directed,
    /// A line of a film's own.
    Hand(Line),
}

/// The most of its life a foe's blow takes, of a hundred, by what is fought
/// (a first setting, not yet balanced): one foe, a
/// team, a tournament, a fight in acts; what blows are worth in those last
/// two; how much more life each foe of a tournament has than the
/// one before; how much stronger each fight is.
pub(crate) const ALONE_BLOW: i32 = 20;
pub(crate) const TEAM_BLOW: i32 = 34;
pub(crate) const TOURNEY_BLOW: i32 = 26;
pub(crate) const ACTS_BLOW: i32 = 80;
pub(crate) const TOURNEY_EDGES: (f32, f32) = (1.0, 1.08);
pub(crate) const ACTS_EDGES: (f32, f32) = (1.0, 1.2);
pub(crate) const TOURNEY_LIFE: f32 = 0.3;
pub(crate) const ROUND_LEVELS: i32 = 4;

/// The line of a fight, read.
#[derive(Clone, Debug)]
pub struct Asked {
    pub(crate) sort: Sort,
    pub(crate) sides: [Vec<Who>; 2],
    pub(crate) level: i32,
    pub(crate) seed: u32,
    /// The place, for its ground.
    pub(crate) place: u16,
    pub(crate) winner: Option<Who>,
    /// An `open` fight shows it: its hud, and what it says of each.
    pub(crate) kept: bool,
    pub(crate) lives: HashMap<Who, (i32, i32)>,
    pub(crate) levels: HashMap<Who, i32>,
    pub(crate) moves: HashMap<Who, Vec<u16>>,
    pub(crate) species: HashMap<Who, u16>,
    pub(crate) seeds: Vec<Who>,
    pub(crate) supporters: Vec<Who>,
    pub(crate) public: Vec<Who>,
    /// All who stand on the stage are of the second side, but these.
    pub(crate) presents: Option<Vec<Who>>,
    pub(crate) moments: Vec<(Who, i32, String)>,
    pub(crate) written: Written,
    /// What a film stated in advance.
    pub(crate) direction: Direction,
    /// Those who wait off the field; who the two great bars are for.
    pub(crate) waiting: Vec<Who>,
    pub(crate) great_bars: Option<(Who, Who)>,
}

/// A thing's name, in the fields of a line (`name:Oran Berry`).
pub(super) fn name_of_thing(fields: &[String]) -> Option<String> {
    let field = fields.get(2)?;
    field.strip_prefix("name:").map(str::to_string)
}

/// The words that open a part of the line.
pub(super) const KEYS: &[&str] = &[
    "side",
    "level",
    "seed",
    "place",
    "support",
    "when",
    "power",
    "takes",
    "crit",
    "act",
    "scene",
    "pause",
    "note",
    "change",
    "uses",
    "throws",
    "reserve",
    "great_bars",
    "hp",
    "turns",
    "rest",
    "winner",
    "keep",
    "lv",
    "knows",
    "species",
    "reviver",
    "audience",
    "present",
    "lead",
    "bar",
    "floor",
    "heals",
    "miss",
    "times",
    "ailment",
    "together",
    "up",
    "down",
    "rage",
];

/// An act of a line: `act <who> <move> [<targets> | all] [misses | critical | ko
/// | protect | -30] [slow] [slowmo <frames>] [intercepts <who>]`.
pub(super) fn act_read(rest: &[String]) -> Option<Act> {
    let mut act = Act::of(Who::parse(rest.first()?)?, rest.get(1)?.parse().ok()?);
    let mut fields = rest[2..].iter();
    while let Some(field) = fields.next() {
        match field.as_str() {
            "all" => act.all = true,
            "misses" => act.told = Told::Misses,
            "critical" => act.critical = true,
            "ko" => act.told = Told::Falls,
            "protect" => act.told = Told::Shielded,
            "slow" => act.slow = true,
            "slowmo" => act.slowed_hit = Some(fields.next()?.parse().ok()?),
            "intercepts" => act.taken_by = Some(Who::parse(fields.next()?)?),
            word => match word.strip_prefix('-') {
                Some(number) => act.told = Told::Takes(number.parse().ok()?),
                None => act.targets.push(Who::parse(word)?),
            },
        }
    }
    // (Said critical, and no more: as the rules have it.)
    if act.critical && act.told == Told::Real {
        act.told = Told::Critical;
    }
    Some(act)
}

impl Asked {
    /// A line: `fight your_turn level 40 seed 7 place 240 side a b side c d
    /// support e when c 60 repere1`; or of a film's own: `fight blow side a
    /// side b power 85 takes 12 crit` (see `hand`).
    /// What depends on who is on the stage now: who watches, who of a crowd
    /// fights, who stands by.
    pub(super) fn now_on(mut self, stage: &Stage) -> Self {
        self.public.retain(|who| stage.cast.contains_key(who));
        if self.sort == Sort::Crowd {
            self.sides.iter_mut().for_each(|side| side.retain(|who| stage.cast.contains_key(who)));
        }
        if let Some(left_out) = self.presents.take() {
            let others: Vec<Who> = stage.bodies_here().into_iter().filter(|who| !left_out.contains(who) && !self.sides[0].contains(who)).collect();
            self.sides[1].extend(others);
        }
        self
    }

    pub fn read(fields: &[String]) -> Option<Self> {
        let sort = match fields.get(1)?.as_str() {
            "your_turn" => Sort::Played,
            "alone" => Sort::Alone,
            "double" => Sort::Together,
            "tournament" => Sort::Tournament,
            "brawl" => Sort::Crowd,
            "directed" => Sort::Directed,
            word => Sort::Hand(Line::read(word)?),
        };
        let mut asked = Asked {
            sort,
            sides: [Vec::new(), Vec::new()],
            level: 30,
            seed: 1,
            place: 0,
            winner: None,
            kept: false,
            lives: HashMap::new(),
            levels: HashMap::new(),
            moves: HashMap::new(),
            species: HashMap::new(),
            seeds: Vec::new(),
            supporters: Vec::new(),
            public: Vec::new(),
            presents: None,
            moments: Vec::new(),
            written: Written::default(),
            direction: Direction::default(),
            waiting: Vec::new(),
            great_bars: None,
        };
        let (mut at, mut sides) = (2, 0);
        while at < fields.len() {
            let key = fields[at].as_str();
            let end = (at + 1..fields.len()).find(|&place| KEYS.contains(&fields[place].as_str())).unwrap_or(fields.len());
            let rest = &fields[at + 1..end];
            let who = |place: usize| rest.get(place).and_then(|field| Who::parse(field));
            let number = |place: usize| rest.get(place).and_then(|field| field.parse::<i32>().ok());
            let everyone = || rest.iter().filter_map(|field| Who::parse(field));
            match key {
                "side" => {
                    if let Some(side) = asked.sides.get_mut(sides) {
                        side.extend(everyone());
                    }
                    sides += 1;
                }
                "level" => asked.level = number(0)?,
                "seed" => asked.seed = number(0)? as u32,
                "place" => asked.place = number(0)? as u16,
                "winner" => asked.winner = who(0),
                "keep" => asked.kept = true,
                "lv" => _ = asked.levels.insert(who(0)?, number(1)?),
                "knows" => _ = asked.moves.insert(who(0)?, rest[1..].iter().filter_map(|field| field.parse().ok()).collect()),
                "species" => _ = asked.species.insert(who(0)?, number(1)? as u16),
                "reviver" => asked.seeds.extend(everyone()),
                "support" => asked.supporters.extend(everyone()),
                "audience" => asked.public.extend(everyone()),
                "present" => asked.presents = Some(everyone().collect()),
                "when" => asked.moments.push((who(0)?, number(1)?, rest.get(2)?.clone())),
                "bar" => asked.written.bars.push((who(0)?, number(1)?)),
                "rage" => asked.written.raging.extend(everyone()),
                "act" => asked.direction.acts.push_back(Stated::Act(act_read(rest)?)),
                "scene" => asked.direction.acts.push_back(Stated::Passage(rest.first()?.clone())),
                "pause" => asked.direction.acts.push_back(Stated::Pause(number(0)?)),
                "note" => asked.direction.acts.push_back(Stated::Log(rest.join(" "))),
                "change" => asked.direction.acts.push_back(Stated::Change { out: who(0)?, comes: who(1)? }),
                "uses" => asked.direction.acts.push_back(Stated::Uses { who: who(0)?, thing: who(1)?, name: name_of_thing(rest)?, worth: number(3) }),
                "throws" => asked.direction.acts.push_back(Stated::Flings {
                    who: who(0)?,
                    thing: who(1)?,
                    name: name_of_thing(rest)?,
                    worth: number(3),
                    target: who(4)?,
                }),
                "reserve" => asked.waiting.extend(everyone()),
                "great_bars" => asked.great_bars = Some((who(0)?, who(1)?)),
                "hp" => _ = asked.lives.insert(who(0)?, (number(1)?, number(1)?)),
                "turns" => asked.direction.turns = Some(number(0)? as u32),
                "rest" => asked.direction.goes_on = true,
                key => asked.written.read(key, rest)?,
            }
            at = end;
        }
        (matches!(sort, Sort::Hand(_)) || (!asked.sides[0].is_empty() && !asked.sides[1].is_empty())).then_some(asked)
    }

    /// Whether it is a line of a film's own.
    pub(super) fn by_hand(&self) -> bool {
        matches!(self.sort, Sort::Hand(_))
    }

    /// What is left in the bars a line goes on with, by their numbers. (A
    /// fight opening has them whole.)
    pub(super) fn lives_from(&mut self, numbers: &BTreeMap<String, f64>) {
        let opens = self.sort == Sort::Hand(Line::Opens);
        for (at, (who, most)) in self.written.bars.iter().enumerate() {
            let left = numbers.get(["hp", "hp2"][at.min(1)]).map(|left| left.round() as i32).filter(|_| !opens);
            self.lives.insert(*who, (left.unwrap_or(*most), *most));
        }
    }

    /// The species of someone.
    pub(super) fn species_of(&self, who: Who, other: &dyn Fn(Who) -> u16) -> u16 {
        match (self.species.get(&who), who) {
            (Some(species), _) => *species,
            (None, Who::Monster(species) | Who::Another { species, .. }) => species,
            (None, who) => other(who),
        }
    }

    /// The species all in the fight are of.
    pub fn species(&self, other: &dyn Fn(Who) -> u16) -> Vec<u16> {
        self.sides.iter().flatten().map(|who| self.species_of(*who, other)).collect()
    }

    pub(super) fn entrant(&self, who: Who, side: usize, level: i32, other: &dyn Fn(Who) -> u16) -> Entrant {
        // (In an `open` fight, those who follow have the life of the first.)
        let first = self.kept.then(|| self.lives.get(&self.sides[side][0])).flatten();
        let life = self.lives.get(&who).or(first).map(|(_, most)| (self.lives.get(&who).map_or(*most, |(left, _)| *left), *most));
        Entrant {
            who,
            species: self.species_of(who, other),
            level: self.levels.get(&who).copied().unwrap_or(level),
            life,
            moves: self.moves.get(&who).cloned().unwrap_or_default(),
            seed: self.seeds.contains(&who),
            waits: self.waiting.contains(&who),
        }
    }

    pub(crate) fn setup(&self, round: usize, sky: u8, other: &dyn Fn(Who) -> u16) -> Setup {
        let tournament = self.sort == Sort::Tournament;
        let level = self.level + if tournament { round as i32 * ROUND_LEVELS } else { 0 };
        let first: Vec<Entrant> = self.sides[0].iter().map(|who| self.entrant(*who, 0, level, other)).collect();
        let second = match tournament {
            true => vec![self.entrant(self.sides[1][round], 1, level, other)],
            false => self.sides[1].iter().map(|who| self.entrant(*who, 1, level, other)).collect(),
        };
        let mode = match self.sort {
            Sort::Played | Sort::Tournament => Mode::Played,
            Sort::Together | Sort::Directed => Mode::Together,
            _ => Mode::Alone,
        };
        let winner = self.winner.and_then(|winner| self.sides.iter().position(|side| side.contains(&winner)));
        // (A fight with its acts lasts long enough to see them.)
        let acts = mode == Mode::Played && !self.moments.is_empty();
        let edges = (tournament || acts).then_some(if acts { ACTS_EDGES } else { TOURNEY_EDGES });
        let hardest = match () {
            _ if tournament => TOURNEY_BLOW,
            _ if acts => ACTS_BLOW,
            _ if self.sides[0].len() > 1 => TEAM_BLOW,
            _ => ALONE_BLOW,
        };
        let foes_life = if tournament { 1.0 + round as f32 * TOURNEY_LIFE } else { 1.0 };
        let chosen = self.winner;
        let direction = (self.sort == Sort::Directed).then(|| Direction { chosen, ..self.direction.clone() });
        Setup { mode, sides: [first, second], winner, seed: self.seed.wrapping_add(round as u32), sky, edges, foes_life, hardest, direction }
    }
}
