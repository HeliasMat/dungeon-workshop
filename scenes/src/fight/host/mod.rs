//! A fight played while a scene holds: the line `fight ...` read, the lines
//! of its sights played in their time on the scene's own stage, its menu,
//! the passages a film wrote for its moments; then the scene goes on with
//! the numbers it left. All who show a scene play it here.
//!
//! - `asked`: the line of a fight, read ([`Asked`]);
//! - here: the fight played on the stage ([`Fighting`]), and what it leaves.

use glam::Vec2;

use super::crowd::Rush;
use super::hand;
use super::stage::{Staging, Step};
use super::*;
use engine::{Call, Director, Hold, Stage, Weather};
use rules::dungeon::weather;

mod asked;
#[cfg(test)]
mod tests;

pub use asked::Asked;
pub(crate) use asked::*;

/// The game's weather, as the stage has it.
fn sky_of(stage: &Stage) -> u8 {
    match stage.light.weather {
        Weather::Rain => weather::RAIN,
        Weather::Sun => weather::SUNNY,
        Weather::Hail => weather::HAIL,
        Weather::Sand => weather::SANDSTORM,
        Weather::Mist => weather::FOG,
        _ => weather::CLEAR,
    }
}

/// How much rage a film has given someone: the number `rage` leaves.
pub(super) fn rage_number(who: Who) -> String {
    format!("rage_{who}").replace('.', "_")
}

/// What is being fought.
#[derive(Clone)]
enum Fought {
    Battle(Box<Battle>),
    Crowd(Box<Crowd>),
    /// A line of a film's own: all it is, it has been made into lines.
    Hand,
}

/// A moment a film wrote: when someone has that much left, the passage.
#[derive(Clone, Debug)]
struct Moment {
    who: Who,
    share: i32,
    label: String,
}

/// A fight going on in a scene.
#[derive(Clone)]
pub struct Fighting {
    asked: Asked,
    rules: Shared,
    fought: Fought,
    staging: Staging,
    species: Vec<(Who, u16)>,
    wait: f32,
    /// The round of a tournament.
    round: usize,
    moments: Vec<Moment>,
    /// A passage is being played; the story has left the fight's line.
    away: bool,
    /// A line of a film's own in such a passage, played by itself.
    aside: Option<Box<Fighting>>,
    /// What the menu offers, while it is up; what was chosen.
    offers: Option<Vec<Offer>>,
    shown: bool,
    choice: Option<Choice>,
    gone_home: bool,
    finished: bool,
}

/// The line of a fight, wherever a scene is shown: it starts, the story is
/// back to it past a passage, or (hurried) it is settled. (`rules`: those of
/// the species asked, if any; `ground`: a place's, by its level.)
pub fn called(
    going: &mut Option<Fighting>,
    fields: &[String],
    rules: &dyn Fn(&[u16]) -> Option<Rules>,
    ground: &dyn Fn(u16) -> Ground,
    other: &dyn Fn(Who) -> u16,
    director: &mut Director,
    stage: &mut Stage,
) {
    let asked = Asked::read(fields).map(|asked| asked.now_on(stage));
    if let Some(fight) = going.as_mut().filter(|_| !asked.as_ref().is_some_and(Asked::by_hand)) {
        fight.back(director, stage);
        return;
    }
    // (A line it cannot read, or species with no rules: told, never passed by unseen.)
    let Some(asked) = asked else {
        stage.wrong("unreadable fight: it does not play".to_string());
        director.menu_done(false);
        return;
    };
    let Some(rules) = rules(&asked.species(other)) else {
        stage.wrong("fight with no rules for these species: it does not play".to_string());
        director.menu_done(false);
        return;
    };
    let mut asked = asked;
    asked.lives_from(director.numbers());
    if director.hurried() && !asked.by_hand() {
        settled(&asked, rules, other, director);
        return;
    }
    let place = ground(asked.place);
    let fight = Fighting::start(asked, rules, place, stage, other);
    match going {
        Some(fight_going) => fight_going.aside = Some(Box::new(fight)),
        None => *going = Some(fight),
    }
}

impl Fighting {
    /// A fight as its line asks, on the stage as it is, on its place's ground.
    pub fn start(asked: Asked, rules: Rules, ground: Ground, stage: &Stage, other: &dyn Fn(Who) -> u16) -> Self {
        let rules: Shared = Arc::new(rules);
        let at: Spots = stage.cast.iter().map(|(who, actor)| (*who, actor.position)).collect();
        let mut staging = Staging::new(ground.clone(), asked.seed, at);
        staging.supporters = asked.supporters.clone();
        staging.public = asked.public.clone();
        staging.kept_hud = asked.kept;
        staging.in_tournament = asked.sort == Sort::Tournament;
        staging.great_bars = asked.great_bars;
        let species = asked.sides.iter().flatten().map(|who| (*who, asked.species_of(*who, other))).collect();
        let fought = match asked.sort {
            Sort::Crowd => {
                let side =
                    |at: usize| asked.sides[at].iter().filter_map(|who| Some((*who, asked.species_of(*who, other), staging.at.get(who).copied()?))).collect();
                let winner = asked.winner.and_then(|winner| asked.sides.iter().position(|side| side.contains(&winner)));
                Fought::Crowd(Box::new(Crowd::new([side(0), side(1)], winner, asked.seed, ground, rules.clone())))
            }
            Sort::Hand(line) => {
                hand::staged(line, &asked, &Battle::new(&asked.setup(0, sky_of(stage), other), rules.clone()), &rules, &mut staging);
                Fought::Hand
            }
            _ => Fought::Battle(Box::new(Battle::new(&asked.setup(0, sky_of(stage), other), rules.clone()))),
        };
        let moments = asked.moments.iter().map(|(who, share, label)| Moment { who: *who, share: *share, label: label.clone() }).collect();
        let mut fighting = Fighting {
            asked,
            rules,
            fought,
            staging,
            species,
            wait: 0.0,
            round: 0,
            moments,
            away: false,
            aside: None,
            offers: None,
            shown: false,
            choice: None,
            gone_home: false,
            finished: false,
        };
        if let Fought::Battle(battle) = &fighting.fought {
            fighting.staging.stand_ready(battle, battle.mode == Mode::Played);
            let names: Vec<(Who, String)> = fighting.asked.sides[1]
                .iter()
                .filter_map(|who| Some((*who, fighting.rules.kinds.get(&fighting.species.iter().find(|one| one.0 == *who)?.1)?.name.clone())))
                .collect();
            fighting.staging.names.extend(names);
            if fighting.asked.kept {
                for side in 0..2 {
                    let (who, life) = (battle.fighter(side).who, battle.fighter(side).stats);
                    fighting.staging.bar_kept(side, who, life.hp, life.max_hp);
                }
            }
            if fighting.asked.sort == Sort::Tournament {
                fighting.staging.round_told(0, fighting.asked.sides[1].len(), battle.fighter(1).who, false);
            }
        }
        fighting
    }

    /// Whether it is done with.
    pub fn over(&self) -> bool {
        self.finished
    }

    /// Those on its field.
    pub fn fighters(&self) -> Vec<Who> {
        self.asked.sides.iter().flatten().copied().collect()
    }

    /// Whether the menu is up.
    pub fn asking(&self) -> bool {
        self.offers.is_some()
    }

    /// The menu, once, as it comes up.
    pub fn menu(&mut self) -> Option<Vec<String>> {
        let offers = self.offers.as_ref().filter(|_| !self.shown)?;
        self.shown = true;
        Some(offers.iter().map(|offer| offer.said.clone()).collect())
    }

    /// Whoever plays has chosen.
    pub fn choose(&mut self, at: usize) {
        if let Some(offers) = self.offers.take() {
            self.choice = offers.get(at).or(offers.first()).map(|offer| offer.choice);
        }
    }

    /// The story has come back to the line of the fight, past a passage:
    /// what it changed counts now.
    fn back(&mut self, director: &mut Director, stage: &Stage) {
        self.away = false;
        self.took_on(director, stage);
    }

    /// What a film changed for the fight: rage, the sky.
    fn took_on(&mut self, director: &mut Director, stage: &Stage) {
        if let Fought::Battle(battle) = &mut self.fought {
            battle.sky = sky_of(stage);
            for who in self.asked.sides.iter().flatten() {
                if let Some(by) = director.numbers_mut().remove(&rage_number(*who)) {
                    battle.enraged(*who, by.round() as i32);
                }
            }
        }
    }

    /// A frame goes by: the lines that are due played.
    pub fn tick(&mut self, frames: f32, director: &mut Director, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        if let Some(aside) = &mut self.aside {
            aside.tick(frames, director, stage, focus, calls);
            if aside.over() {
                self.aside = None;
            }
            return;
        }
        if self.finished || self.away || director.hold() != Some(Hold::Host) {
            return;
        }
        self.wait -= frames;
        while self.wait <= 0.0 {
            match self.staging.steps.pop_front() {
                Some(Step::Beat(beat)) => calls.extend(director.play(beat, stage, focus)),
                Some(Step::Wait(frames)) => self.wait += frames,
                // (Off to the passage; the story comes back to the fight's line.)
                Some(Step::Passage(label)) => {
                    self.away = director.passage(&label);
                    return;
                }
                Some(Step::WaitFor(who)) => {
                    if stage.cast.get(&who).is_some_and(|actor| actor.busy()) {
                        self.staging.steps.push_front(Step::WaitFor(who));
                        self.wait = 0.0;
                        return;
                    }
                }
                None => {
                    self.wait = 0.0;
                    if !self.think(director, stage) {
                        return;
                    }
                }
            }
        }
    }

    /// The lines of what happens next, if it goes on by itself.
    fn think(&mut self, director: &mut Director, stage: &Stage) -> bool {
        self.took_on(director, stage);
        let staging = &mut self.staging;
        match &mut self.fought {
            Fought::Crowd(crowd) => {
                if crowd.over() {
                    return self.ends(director);
                }
                let rushes: Vec<Rush> = crowd.round();
                rushes.iter().for_each(|rush| staging.rush(rush));
                true
            }
            Fought::Battle(battle) => {
                if battle.over().is_some() {
                    return self.round_over(director, stage);
                }
                if let Some(at) = self.moments.iter().position(|moment| battle.share_left(moment.who).is_some_and(|left| left <= moment.share)) {
                    let moment = self.moments.remove(at);
                    self.away = director.passage(&moment.label);
                    return false;
                }
                let choice = self.choice.take();
                if battle.waiting() && choice.is_none() {
                    (self.offers, self.shown) = (Some(battle.offers()), false);
                    return false;
                }
                for sight in battle.step(choice) {
                    staging.sight(&sight, battle);
                }
                true
            }
            Fought::Hand => {
                self.finish(director);
                false
            }
        }
    }

    /// A fight is over: the next of a tournament, or all of it.
    fn round_over(&mut self, director: &mut Director, stage: &Stage) -> bool {
        let Fought::Battle(battle) = &self.fought else {
            return false;
        };
        let won = battle.over() == Some(0);
        if self.asked.sort == Sort::Tournament && won && self.round + 1 < self.asked.sides[1].len() {
            self.staging.round_lost_by(battle.fighter(1).who);
            self.round += 1;
            let foe = self.asked.sides[1][self.round];
            let species = self.species.clone();
            let other = |who: Who| species.iter().find(|one| one.0 == who).map_or(0, |one| one.1);
            let battle = Battle::new(&self.asked.setup(self.round, sky_of(stage), &other), self.rules.clone());
            self.staging.round_told(self.round, self.asked.sides[1].len(), foe, true);
            self.staging.stand_ready(&battle, true);
            self.fought = Fought::Battle(Box::new(battle));
            return true;
        }
        if self.asked.sort == Sort::Tournament {
            let champion = won;
            director.set_number("champion".to_string(), champion as i32);
            if champion && !self.staging.named_champion {
                self.staging.named_champion = true;
                self.staging.champion(self.asked.sides[0][0]);
                return true;
            }
        }
        self.ends(director)
    }

    /// Those who stand go back where they stood, then it is over. Whether
    /// there is more to play.
    fn ends(&mut self, director: &mut Director) -> bool {
        if !std::mem::replace(&mut self.gone_home, true) {
            self.staging.go_home();
            if !self.staging.steps.is_empty() {
                return true;
            }
        }
        self.finish(director);
        false
    }

    /// The numbers it leaves, and the story goes on.
    fn finish(&mut self, director: &mut Director) {
        if let Fought::Battle(battle) = &self.fought {
            left_numbers(battle, director);
        }
        self.finished = true;
        director.menu_done(false);
    }
}

/// What a fight leaves a film: who it went by, how many of the others stand.
fn left_numbers(battle: &Battle, director: &mut Director) {
    let left = battle.fighter(0).stats.hp;
    director.set_number("hp".to_string(), left);
    director.set_number("summary_hp".to_string(), left);
    director.set_number("hp2".to_string(), battle.teams[1].bodies.iter().filter(|body| !body.down).count() as i32);
}

/// A fight nobody sees, of a talk with no stage: settled at once, each
/// side's by itself.
pub fn settled(asked: &Asked, rules: Rules, other: &dyn Fn(Who) -> u16, director: &mut Director) {
    if asked.by_hand() {
        director.menu_done(false);
        return;
    }
    let rules: Shared = Arc::new(rules);
    let rounds = if asked.sort == Sort::Tournament { asked.sides[1].len() } else { 1 };
    let mut won = asked.sort != Sort::Crowd;
    for round in 0..rounds {
        if asked.sort == Sort::Crowd {
            break;
        }
        let mut battle = Battle::new(&asked.setup(round, weather::CLEAR, other), rules.clone());
        while battle.over().is_none() {
            battle.step(None);
        }
        left_numbers(&battle, director);
        won = battle.over() == Some(0);
        if !won {
            break;
        }
    }
    if asked.sort == Sort::Tournament {
        director.set_number("champion".to_string(), won as i32);
    }
    director.menu_done(false);
}
