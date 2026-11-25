//! A fight going on, a turn at a time.

mod blows;
mod directed;
mod turns;

use std::collections::HashMap;

use super::*;
use rules::attack::{self, Attack, Field, Happened, Outcome, Party, Tuning};
use rules::dungeon::combat::{FIXED_ONE, matchup};
use rules::dungeon::status::{MoveEffect, apply_effect, move_effect};
use rules::dungeon::{DungeonRng, Pos, weather};

/// Below how much of its life someone eats if it can; how much of it a
/// berry gives back.
const LOW_SHARE: i32 = 25;
const BERRY_SHARE: i32 = 3;
/// The share of its life a burn or poison, then the sky, takes each turn.
const WEARING_SHARE: i32 = 16;
const HARSH_SHARE: i32 = 16;
/// Turns of a bout that plays itself at most, then of one that is played;
/// all on the field, as many for each of them.
const LONGEST_BOUT: u32 = 16;
const LONGEST_PLAYED: u32 = 80;
const LONGEST_TOGETHER: u32 = LONGEST_BOUT * 4;
/// What happens now and then, in a hundred: a foe shields itself when hurt,
/// uses what does not strike; someone steps aside; two strike together.
const FOE_SHIELDS: i32 = 30;
const FOE_DEEDS: i32 = 25;
const STEPS_ABOUT: i32 = 45;
const TOGETHER: i32 = 18;
/// How many times more often a foe uses its best move.
const BEST_MOVE: usize = 2;
/// How many times whoever plays may shield itself.
const SHIELDS_GIVEN: i32 = 2;
/// The chance of a paralysed one to lose its turn; the turns paralysis and
/// confusion last.
const PARALYSIS: i32 = 25;
const AILING_TURNS: i32 = 3;
/// What a blow is worth from who held on (as 8.8), and on who is to win.
const SURGE: f32 = 1.7;
const SPARED: f32 = 0.55;
/// The most of someone's life a foe's blow takes in a fight one plays, of
/// a hundred.
pub(super) const HARDEST_BLOW: i32 = 20;
/// The most of a foe's life a blow of who plays takes: a fight lasts.
const BEST_BLOW: i32 = 30;
/// How many of the other's turns who plays needs to win, at its best, in a
/// fair fight, alone and in a team; the power it is weighed by; the most.
pub(super) const FAIR_TURNS: f32 = 0.75;
pub(super) const TEAM_TURNS: f32 = 1.1;
const FAIR_POWER: f32 = 0.8;
const MOST_EDGE: f32 = 3.0;
/// Moves whose users leave the ground, and come back on the target.
pub(crate) const FLY: u16 = 153;
pub(crate) const DIVE: u16 = 156;
pub(crate) const DIG: u16 = 8;
/// Experience of a win.
const POWER_OF_NOTHING: i32 = 60;

/// Whether a fight knows what to do with a move, as chosen by itself.
pub(super) fn of_use(known: &Move) -> bool {
    use MoveEffect::*;
    if known.strikes() {
        return true;
    }
    matches!(
        move_effect(known.slot.id),
        Some(
            UserAttack(_)
                | UserDefense(_)
                | UserSpAttack(_)
                | UserSpDefense(_)
                | CalmMind
                | CosmicPower
                | BulkUp
                | DragonDance
                | BoostAll
                | UserSpeedUp
                | HealQuarter
                | HealHalf
                | HealBy(_)
                | HealFlat(_)
                | Explode(_)
        )
    )
}

/// The move as a blow: one without power of its own is given some.
fn struck_with(known: &Move) -> MoveSlot {
    let power = if known.slot.power > 0 { known.slot.power } else { POWER_OF_NOTHING };
    MoveSlot { power, ..known.slot }
}

/// Someone in the fight.
#[derive(Clone, Debug)]
pub struct Body {
    pub who: Who,
    pub stats: Stats,
    pub statuses: Statuses,
    pub moves: Vec<Move>,
    seed: bool,
    /// It waits off the fight (see `Stated::Change`).
    pub(crate) benched: bool,
    /// Its shield is up; it must rest; its one deed as a foe is used; down.
    guarded: bool,
    resting: bool,
    deed_used: bool,
    pub down: bool,
    species: u16,
}

impl Body {
    fn new(entrant: &Entrant, rules: &Rules) -> Self {
        let kind = rules.kinds.get(&entrant.species).cloned().unwrap_or_default();
        let mut stats = kind.stats(entrant.level);
        if let Some((left, most)) = entrant.life {
            (stats.hp, stats.max_hp) = (left, most);
        }
        let chosen = entrant.moves.iter().filter_map(|&id| rules.moves.get(id as usize)).cloned();
        let moves: Vec<Move> = chosen.collect();
        let moves = if moves.is_empty() { rules.known(entrant.species, entrant.level) } else { moves };
        Body {
            who: entrant.who,
            stats,
            statuses: Statuses::for_species(kind.movement),
            moves,
            seed: entrant.seed,
            benched: entrant.waits,
            guarded: false,
            resting: false,
            deed_used: false,
            down: false,
            species: entrant.species,
        }
    }

    fn fighter(&self) -> rules::dungeon::combat::Fighter {
        self.statuses.fighter(&self.stats, true)
    }

    /// What holds it from choosing anything.
    fn stopped(&self) -> Option<Ailment> {
        match () {
            _ if self.statuses.frozen_turns > 0 => Some(Ailment::Frozen),
            _ if self.statuses.asleep() => Some(Ailment::Asleep),
            _ => None,
        }
    }

    fn ailment(&self) -> Option<Ailment> {
        let statuses = &self.statuses;
        match () {
            _ if statuses.frozen_turns > 0 => Some(Ailment::Frozen),
            _ if statuses.asleep() => Some(Ailment::Asleep),
            _ if statuses.burn_turns > 0 => Some(Ailment::Burnt),
            _ if statuses.poisoned => Some(Ailment::Poisoned),
            _ if statuses.paralysis_turns > 0 => Some(Ailment::Paralysed),
            _ if statuses.confused_turns > 0 => Some(Ailment::Confused),
            _ => None,
        }
    }

    fn stages(&self) -> [(Stat, i32); 6] {
        let statuses = &self.statuses;
        [
            (Stat::Attack, statuses.attack_stage),
            (Stat::Defense, statuses.defense_stage),
            (Stat::SpAttack, statuses.sp_attack_stage),
            (Stat::SpDefense, statuses.sp_defense_stage),
            (Stat::Accuracy, statuses.accuracy_stage),
            (Stat::Evasion, statuses.evasion_stage),
        ]
    }

    /// An ailment lasts a little here: long ones would end a fight alone.
    fn kept_short(&mut self) {
        let statuses = &mut self.statuses;
        statuses.frozen_turns = statuses.frozen_turns.min(1);
        statuses.sleep_turns = statuses.sleep_turns.min(1);
        statuses.cringe_turns = statuses.cringe_turns.min(1);
        statuses.paralysis_turns = statuses.paralysis_turns.min(AILING_TURNS);
        statuses.confused_turns = statuses.confused_turns.min(AILING_TURNS);
        // (Shields and rests are its own here; a turn counts down neither.)
        statuses.shield = None;
        statuses.paused_turns = 0;
    }

    /// Someone in an attack.
    fn party(&mut self, team: bool) -> Party<'_> {
        Party { stats: &mut self.stats, status: &mut self.statuses, team, leader: false, at: Pos::new(0, 0), id: 0 }
    }

    /// Its life goes up, to its most.
    fn healed(&mut self, by: i32) -> Strike {
        let left = (self.stats.hp + by).min(self.stats.max_hp);
        let strike = Strike { taken: self.stats.hp - left, left, held: false };
        self.stats.hp = left;
        strike
    }
}

/// A side: who is on it, who fights, what it can still do.
#[derive(Clone, Debug)]
pub struct Team {
    pub bodies: Vec<Body>,
    pub at: usize,
    berries: i32,
    shields: i32,
}

impl Team {
    fn standing(&self) -> impl Iterator<Item = usize> + '_ {
        self.bodies.iter().enumerate().filter(|(_, body)| !body.down && !body.benched).map(|(at, _)| at)
    }
}

/// Which menu whoever plays is at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Menu {
    Main,
    Changing,
}

/// A fight going on.
#[derive(Clone, Debug)]
pub struct Battle {
    rules: Shared,
    rng: DungeonRng,
    pub mode: Mode,
    pub teams: [Team; 2],
    pub sky: u8,
    winner: Option<usize>,
    /// Who is to win and is the last of its side; who has held on.
    spared: Option<Who>,
    surging: Option<Who>,
    side: usize,
    turns: u32,
    opened: bool,
    finished: Option<usize>,
    menu: Menu,
    /// What the blows of the two sides are worth, set or by who meets who.
    edges: Option<(f32, f32)>,
    worked_out: HashMap<(Who, Who), (f32, f32)>,
    /// The most of its life a foe's blow takes, of a hundred.
    hardest: i32,
    /// What a film stated in advance, and is still to come.
    direction: Option<Direction>,
}

impl Battle {
    pub fn new(setup: &Setup, rules: Shared) -> Self {
        let team = |at: usize| {
            let mut bodies: Vec<Body> = setup.sides[at].iter().map(|entrant| Body::new(entrant, &rules)).collect();
            if at == 1 {
                for body in &mut bodies {
                    body.stats.max_hp = (body.stats.max_hp as f32 * setup.foes_life) as i32;
                    body.stats.hp = (body.stats.hp as f32 * setup.foes_life) as i32;
                }
            }
            let count = bodies.len() as i32;
            let played = setup.mode == Mode::Played;
            Team { bodies, at: 0, berries: if played { 1 } else { count }, shields: if played && at == 0 { SHIELDS_GIVEN } else { count } }
        };
        Battle {
            rng: DungeonRng::new(setup.seed),
            mode: setup.mode,
            teams: [team(0), team(1)],
            sky: setup.sky,
            winner: setup.winner,
            spared: None,
            surging: None,
            side: 0,
            turns: 0,
            opened: false,
            finished: None,
            menu: Menu::Main,
            edges: setup.edges,
            worked_out: HashMap::new(),
            hardest: setup.hardest,
            direction: setup.direction.clone(),
            rules,
        }
    }

    /// The side that won, once one has.
    pub fn over(&self) -> Option<usize> {
        self.finished
    }

    /// Who fights for a side now.
    pub(crate) fn fighter(&self, side: usize) -> &Body {
        &self.teams[side].bodies[self.teams[side].at]
    }

    /// Someone in the fight.
    pub fn body(&self, who: Who) -> Option<&Body> {
        self.teams.iter().flat_map(|team| &team.bodies).find(|body| body.who == who)
    }

    /// How much of its life someone has left, of a hundred; none if it
    /// has fallen.
    pub(crate) fn share_left(&self, who: Who) -> Option<i32> {
        let body = self.body(who).filter(|body| !body.down)?;
        (self.teams.iter().flat_map(|team| team.standing().map(|at| team.bodies[at].who)).any(|other| other == who))
            .then_some(body.stats.hp * 100 / body.stats.max_hp.max(1))
    }

    /// Whether whoever plays is to choose before the next step.
    pub fn waiting(&self) -> bool {
        self.opened
            && self.finished.is_none()
            && self.mode == Mode::Played
            && self.side == 0
            && (self.menu == Menu::Changing || self.can_choose())
            && self.turns < LONGEST_PLAYED
    }

    fn can_choose(&self) -> bool {
        let me = self.fighter(0);
        me.stopped().is_none() && !me.resting
    }

    /// What it can choose from.
    pub fn offers(&self) -> Vec<Offer> {
        let (team, me, foe) = (&self.teams[0], self.fighter(0), self.fighter(1));
        if self.menu == Menu::Changing {
            let others = team.standing().filter(|&at| at != team.at).map(|at| Offer { said: self.name_of(&team.bodies[at]), choice: Choice::Change(at) });
            return others.chain(std::iter::once(Offer { said: "Back".to_string(), choice: Choice::Back })).collect();
        }
        let told = |known: &Move| match self.fares(known, foe) {
            matchup::IMMUNE => " x",
            matchup::NOT_VERY_EFFECTIVE => " -",
            matchup::SUPER_EFFECTIVE => " +",
            _ => "",
        };
        let moves = me
            .moves
            .iter()
            .enumerate()
            .map(|(at, known)| Offer { said: format!("{}{}", known.name, if known.strikes() { told(known) } else { "" }), choice: Choice::Move(at) });
        let mut offers: Vec<Offer> = moves.collect();
        if team.berries > 0 {
            offers.push(Offer { said: "Oran Berry".to_string(), choice: Choice::Berry });
        }
        if team.shields > 0 {
            offers.push(Offer { said: "Protect".to_string(), choice: Choice::Shield });
        }
        if team.bodies.len() > 1 {
            offers.push(Offer { said: "Switch".to_string(), choice: Choice::Changing });
        }
        offers
    }

    /// Someone as the game calls it.
    pub fn name_of(&self, body: &Body) -> String {
        self.rules.kinds.get(&body.species).map_or(String::new(), |kind| kind.name.clone())
    }

    /// How a move fares against someone.
    pub(crate) fn fares(&self, known: &Move, other: &Body) -> u8 {
        self.rules.chart.as_ref().map_or(matchup::NEUTRAL, |chart| chart.overall(known.slot.type_id, &other.stats))
    }

    /// What a blow takes, about.
    pub fn worth(&self, me: &Body, known: &Move, other: &Body) -> i32 {
        if !known.strikes() {
            return 0;
        }
        attack::foreseen(&self.rules.constants, self.rules.chart.as_ref(), &me.fighter(), &other.fighter(), &struck_with(known), self.sky, Default::default())
    }

    /// What gives a fight one plays a fair chance, whoever is in it: both
    /// sides' blows weighed so that who plays, at its best, needs a little
    /// fewer turns than the other.
    fn edges_of(&mut self, me: usize, foe: usize) -> (f32, f32) {
        if let Some(edges) = self.edges {
            return edges;
        }
        if self.mode != Mode::Played {
            return (1.0, 1.0);
        }
        let (mine, theirs) = (self.teams[0].bodies[me].clone(), self.teams[1].bodies[foe].clone());
        if let Some(edges) = self.worked_out.get(&(mine.who, theirs.who)) {
            return *edges;
        }
        let best = |one: &Body, other: &Body| one.moves.iter().map(|known| self.worth(one, known, other)).max().unwrap_or(1).max(1) as f32;
        let (life_me, life_foe) = (mine.stats.max_hp as f32, theirs.stats.max_hp as f32);
        let turns = (life_foe / best(&mine, &theirs)) / (life_me / best(&theirs, &mine).min(life_me * HARDEST_BLOW as f32 / 100.0));
        let fair = if self.teams[0].bodies.len() > 1 { TEAM_TURNS } else { FAIR_TURNS };
        let weight = (turns / fair).powf(FAIR_POWER);
        let edges = (weight.clamp(1.0 / MOST_EDGE, MOST_EDGE), (1.0 / weight).clamp(1.0 / MOST_EDGE, MOST_EDGE));
        self.worked_out.insert((mine.who, theirs.who), edges);
        edges
    }

    /// Someone is hurt by rage: stronger, to the end of the fight.
    pub(crate) fn enraged(&mut self, who: Who, by: i32) {
        for body in self.teams.iter_mut().flat_map(|team| &mut team.bodies).filter(|body| body.who == who) {
            body.statuses.attack_stage = (body.statuses.attack_stage + by).min(20);
            body.statuses.sp_attack_stage = (body.statuses.sp_attack_stage + by).min(20);
        }
    }
}

/// The stat stages that changed, and which way.
fn stages_changed(before: [(Stat, i32); 6], after: [(Stat, i32); 6]) -> Vec<(Stat, bool)> {
    before.into_iter().zip(after).filter(|(was, now)| was.1 != now.1).map(|((stat, was), (_, now))| (stat, now > was)).collect()
}
