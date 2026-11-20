//! A fight a film writes itself, a line at a time, what the fight's own
//! rules leave out being said there: who strikes, with what, how much it
//! takes. Each line becomes the `Sight`s any fight has, shown the same way;
//! what lasts from one line to the next is in the numbers of the bars.

use std::collections::HashMap;

use super::battle::Body;
use super::host::Asked;
use super::stage::{PROTECT, Staging};
use super::*;
use rules::dungeon::combat::matchup;

/// What a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// Its bars come up.
    Opens,
    Blow,
    Stat,
    Falls,
    /// Two strike at once.
    Together,
    /// Who was beaten joins the other.
    Joins,
    Berry,
    Rage,
    /// All a fight showed goes.
    Closes,
}

impl Line {
    pub fn read(word: &str) -> Option<Self> {
        Some(match word {
            "open" => Line::Opens,
            "blow" => Line::Blow,
            "stat" => Line::Stat,
            "ko" => Line::Falls,
            "combo" => Line::Together,
            "recruit" => Line::Joins,
            "berry" => Line::Berry,
            "rage" => Line::Rage,
            "end" => Line::Closes,
            _ => return None,
        })
    }
}

/// How much stronger rage makes someone, in stages.
const RAGE: i32 = 2;

/// What a line says, past those it is about.
#[derive(Clone, Debug, Default)]
pub struct Written {
    /// Who the fight goes by; none, out of one.
    pub lead: Option<Who>,
    /// The bars, in order, each with its most.
    pub bars: Vec<(Who, i32)>,
    pub floor: Option<i32>,
    pub power: Option<u16>,
    /// What each strike takes, if the line says; less than 0, it heals.
    pub taken: Option<i32>,
    pub critical: bool,
    pub missed: bool,
    pub times: Option<i32>,
    pub ailment: Option<Ailment>,
    /// It goes on at once.
    pub beside: bool,
    pub stat: Option<(Stat, bool)>,
    pub raging: Vec<Who>,
}

impl Written {
    /// A part of a line.
    pub fn read(&mut self, key: &str, rest: &[String]) -> Option<()> {
        let number = rest.first().and_then(|field| field.parse::<i32>().ok());
        match key {
            "lead" => self.lead = rest.first().and_then(|field| Who::parse(field)),
            "floor" => self.floor = Some(number?),
            "power" => self.power = Some(number? as u16),
            "takes" => self.taken = Some(number?),
            "heals" => self.taken = Some(-number?),
            "crit" => self.critical = true,
            "miss" => self.missed = true,
            "times" => self.times = Some(number?),
            "ailment" => self.ailment = Some(ailment(rest.first()?)?),
            "together" => self.beside = true,
            "up" | "down" => self.stat = Some((stat(rest.first()?)?, key == "up")),
            _ => return None,
        }
        Some(())
    }
}

fn ailment(word: &str) -> Option<Ailment> {
    Some(match word {
        "burn" => Ailment::Burnt,
        "freeze" => Ailment::Frozen,
        "paralyze" => Ailment::Paralysed,
        "sleep" => Ailment::Asleep,
        "poison" => Ailment::Poisoned,
        "confuse" => Ailment::Confused,
        _ => return None,
    })
}

fn stat(word: &str) -> Option<Stat> {
    Some(match word {
        "attack" => Stat::Attack,
        "defense" => Stat::Defense,
        "sp_attack" => Stat::SpAttack,
        "sp_defense" => Stat::SpDefense,
        "accuracy" => Stat::Accuracy,
        "evasion" => Stat::Evasion,
        "speed" => Stat::Speed,
        _ => return None,
    })
}

/// A move written, on those it is for.
struct Blows<'a> {
    written: &'a Written,
    battle: &'a Battle,
    power: Move,
    /// What is left of each, this far in the line.
    left: HashMap<Who, i32>,
}

impl Blows<'_> {
    fn landing(&mut self, me: &Body, target: Who, taken: Option<i32>) -> Option<Landing> {
        let written = self.written;
        let other = self.battle.body(target)?;
        let each = taken.unwrap_or_else(|| self.battle.worth(me, &self.power, other).max(1));
        let left = self.left.entry(target).or_insert(other.stats.hp);
        let mut strikes = Vec::new();
        for _ in 0..written.times.unwrap_or(1).max(1) {
            if !written.missed {
                *left = (*left - each).max(0);
                strikes.push(Strike { taken: each, left: *left, held: false });
            }
        }
        Some(Landing {
            target,
            strikes,
            missed: written.missed,
            blocked: false,
            critical: written.critical,
            matchup: self.battle.fares(&self.power, other),
            ailment: written.ailment,
            stats: Vec::new(),
        })
    }

    fn blow(&mut self, who: Who, targets: &[Who], taken: Option<i32>, beside: bool) -> Option<Sight> {
        let me = self.battle.body(who)?;
        let landings: Vec<Landing> = targets.iter().filter_map(|target| self.landing(me, *target, taken)).collect();
        Some(Sight::Blow(Blow { who, power: self.power.clone(), landings, drained: None, recoil: None, spent: false, blast: None, stats: Vec::new(), beside }))
    }

    /// Those with a bar it brought down.
    fn fallen(&self) -> Vec<Sight> {
        let fell = |who: &Who| self.left.get(who) == Some(&0) && self.battle.body(*who).is_some_and(|body| body.stats.hp > 0);
        self.written.bars.iter().map(|(who, _)| *who).filter(fell).map(|who| Sight::Falls { who, more: false }).collect()
    }
}

/// A move that strikes no one.
fn deed(asked: &Asked, battle: &Battle, who: Who, power: Move, healed: Option<Strike>) -> Sight {
    let target = asked.sides[1].first().filter(|_| healed.is_none());
    let landing = target.map(|target| {
        let matchup = battle.body(*target).map_or(matchup::NEUTRAL, |other| battle.fares(&power, other));
        Landing {
            target: *target,
            strikes: Vec::new(),
            missed: asked.written.missed,
            blocked: false,
            critical: false,
            matchup,
            ailment: asked.written.ailment,
            stats: Vec::new(),
        }
    });
    Sight::Deed(Deed { who, power, healed, stats: Vec::new(), landing })
}

/// Life given back to someone.
fn healing(body: &Body, by: i32) -> Strike {
    let left = (body.stats.hp + by).min(body.stats.max_hp);
    Strike { taken: body.stats.hp - left, left, held: false }
}

/// What a move written does.
fn struck(asked: &Asked, battle: &Battle, rules: &Rules) -> Vec<Sight> {
    let written = &asked.written;
    let (Some(&who), Some(id)) = (asked.sides[0].first(), written.power) else {
        return Vec::new();
    };
    let Some(power) = rules.moves.get(id as usize).cloned() else {
        return Vec::new();
    };
    let (taken, targets) = (written.taken, &asked.sides[1]);
    if let (Some(by @ ..0), Some(body)) = (taken, battle.body(who)) {
        return vec![deed(asked, battle, who, power, Some(healing(body, -by)))];
    }
    if power.slot.id == PROTECT {
        return vec![Sight::Shields { who }];
    }
    if targets.is_empty() || (!power.strikes() && taken.is_none()) {
        return vec![deed(asked, battle, who, power, None)];
    }
    let mut blows = Blows { written, battle, power, left: HashMap::new() };
    let mut sights: Vec<Sight> = blows.blow(who, targets, taken, written.beside).into_iter().collect();
    if !written.beside {
        sights.extend(blows.fallen());
    }
    sights
}

/// Two strike at once: each its blow, the one beside the other, sharing
/// what the line says.
fn together(asked: &Asked, battle: &Battle, rules: &Rules) -> Vec<Sight> {
    let written = &asked.written;
    let ([first, second], Some(power)) = (&asked.sides[0][..], written.power.and_then(|id| rules.moves.get(id as usize).cloned())) else {
        return Vec::new();
    };
    let share = written.taken.map(|taken| (taken / 2).max(1));
    let mut blows = Blows { written, battle, power, left: HashMap::new() };
    let mut sights = vec![Sight::Partners { first: *first, second: *second }];
    sights.extend(blows.blow(*second, &asked.sides[1], share, true));
    sights.extend(blows.blow(*first, &asked.sides[1], share, false));
    sights.extend(blows.fallen());
    sights
}

/// A line, as what shows of it.
pub fn staged(line: Line, asked: &Asked, battle: &Battle, rules: &Rules, staging: &mut Staging) {
    let written = &asked.written;
    let life = |who: Who| asked.lives.get(&who).copied();
    let bars: Vec<(Who, i32, i32)> = written.bars.iter().filter_map(|(who, _)| life(*who).map(|(left, most)| (*who, left, most))).collect();
    staging.by_hand(battle, written.lead, &bars);
    let first = asked.sides[0].first().copied();
    let sights = match (line, first) {
        (Line::Opens, _) if bars.len() > 1 => vec![Sight::Opens { first: bars[0].0, second: bars[1].0 }],
        (Line::Opens, Some(who)) => {
            let (left, most) = life(who).unwrap_or_default();
            staging.top_line(who, asked.level, (left, most), written.floor, &asked.sides[1]);
            Vec::new()
        }
        (Line::Blow, _) => struck(asked, battle, rules),
        (Line::Stat, Some(who)) => {
            if let Some((stat, up)) = written.stat {
                staging.stat(who, stat, up);
            }
            Vec::new()
        }
        (Line::Falls, Some(who)) => vec![Sight::Falls { who, more: false }],
        (Line::Together, _) => together(asked, battle, rules),
        (Line::Joins, Some(who)) => {
            if let Some(leader) = asked.sides[1].first() {
                staging.joins(who, *leader);
            }
            Vec::new()
        }
        (Line::Berry, Some(who)) => battle.body(who).map(|body| Sight::Berry { who, healed: healing(body, -written.taken.unwrap_or(0)) }).into_iter().collect(),
        (Line::Rage, Some(who)) => {
            staging.rages(who, &super::host::rage_number(who), RAGE);
            Vec::new()
        }
        (Line::Closes, _) => {
            staging.closes(&written.raging, written.lead.and_then(life).map(|(left, _)| left));
            Vec::new()
        }
        _ => Vec::new(),
    };
    for sight in &sights {
        staging.sight(sight, battle);
    }
}
