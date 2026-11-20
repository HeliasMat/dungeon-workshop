//! Two crowds at it, round after round, until one is down or runs for it:
//! ranks going up to the others, those at the front striking.

use glam::Vec2;

use super::*;
use rules::dungeon::DungeonRng;
use rules::dungeon::combat::{FIXED_ONE, calc_damage_with};

/// Life, the most a blow takes there, the level they are counted at.
pub(crate) const HEALTH: i32 = 34;
const MOST_BLOW: i32 = 18;
const LEVEL: i32 = 12;
/// How close they stop, how far a round takes them, how much room each
/// keeps, how close they must be to strike.
const NEAR: f32 = 22.0;
const STRIDE: f32 = 48.0;
const ROOM: f32 = 18.0;
const FRONT: f32 = 96.0;
/// How much more those who are to win do.
const FAVOUR: f32 = 1.4;
/// The most rounds; how many times fewer a side must be to run.
const ROUNDS: u32 = 18;
const ROUT: usize = 3;
/// The reaches of the game that strike those around; how far, how many more.
const AROUND: [u8; 2] = [2, 3];
const AROUND_REACH: f32 = 46.0;
const AROUND_MOST: usize = 4;
/// How far from the edge of the place those who run stop; the far end of a
/// place whose ground is not known.
const EDGE: f32 = 24.0;
const FAR_END: f32 = 1400.0;

/// What happens in a round.
#[derive(Clone, Debug, PartialEq)]
pub enum Rush {
    /// A round starts.
    Starts,
    /// Someone goes up to the others (`place`: the how manieth to go).
    Strides { who: Who, to: Vec2, place: usize },
    /// Someone strikes, `place` of its side, and `side`.
    Strikes { who: Who, other: Who, id: u16, side: usize, place: usize, around: bool },
    /// A blow gets to someone: what is left, whether it falls.
    Takes { one: Who, by: Who, taken: i32, left: i32, side: usize, place: usize },
    /// A round's end.
    Ends,
    /// Those left of a side run for the edge of the place.
    Runs { who: Who, to: Vec2 },
    /// All who run are gone.
    Ran,
    /// Those who won are glad.
    Glad { who: Who },
}

/// Two crowds.
#[derive(Clone)]
pub struct Crowd {
    rules: Shared,
    sides: [Vec<Who>; 2],
    species: HashMap<Who, u16>,
    health: HashMap<Who, i32>,
    at: Spots,
    ground: Ground,
    rng: DungeonRng,
    /// The side that is to win, if one is.
    chosen: Option<usize>,
    rounds: u32,
    done: bool,
}

impl Crowd {
    pub fn new(sides: [Vec<(Who, u16, Vec2)>; 2], chosen: Option<usize>, seed: u32, ground: Ground, rules: Shared) -> Self {
        let everyone = || sides.iter().flatten();
        Crowd {
            rules,
            species: everyone().map(|(who, species, _)| (*who, *species)).collect(),
            health: everyone().map(|(who, ..)| (*who, HEALTH)).collect(),
            at: everyone().map(|(who, _, at)| (*who, *at)).collect(),
            sides: [sides[0].iter().map(|one| one.0).collect(), sides[1].iter().map(|one| one.0).collect()],
            ground,
            rng: DungeonRng::new(seed),
            chosen,
            rounds: 0,
            done: false,
        }
    }

    pub fn over(&self) -> bool {
        self.done
    }

    fn standing(&self) -> [Vec<Who>; 2] {
        [0, 1].map(|side| self.sides[side].iter().copied().filter(|who| self.health[who] > 0).collect())
    }

    /// A round: all that happens in it.
    pub fn round(&mut self) -> Vec<Rush> {
        let mut out = Vec::new();
        let standing = self.standing();
        if self.done {
            return out;
        }
        if standing.iter().any(Vec::is_empty) {
            self.ends(&mut out);
            return out;
        }
        if let Some(routed) = (0..2).find(|&side| standing[side].len() * ROUT < standing[1 - side].len()) {
            self.run_away(&standing, routed, &mut out);
            self.ends(&mut out);
            return out;
        }
        if self.rounds >= ROUNDS {
            // (It has lasted long enough: who is the fewer gives way.)
            let fewer = usize::from(standing[0].len() >= standing[1].len());
            self.run_away(&standing, self.chosen.map_or(fewer, |side| 1 - side), &mut out);
            self.ends(&mut out);
            return out;
        }
        self.rounds += 1;
        out.push(Rush::Starts);
        self.advance(&standing, &mut out);
        self.blows(&standing, &mut out);
        out.push(Rush::Ends);
        out
    }

    fn nearest(&self, who: Who, others: &[Who]) -> Option<Who> {
        let from = self.at[&who];
        others.iter().copied().min_by(|a, b| self.at[a].distance(from).total_cmp(&self.at[b].distance(from)))
    }

    /// First all go up to the others: the front first, the ranks behind
    /// taking the room it leaves.
    fn advance(&mut self, standing: &[Vec<Who>; 2], out: &mut Vec<Rush>) {
        for side in 0..2 {
            let enemies = &standing[1 - side];
            let far = |who: &Who| self.nearest(*who, enemies).map_or(0.0, |other| self.at[who].distance(self.at[&other]));
            let mut ranks: Vec<(f32, Who)> = standing[side].iter().map(|who| (far(who), *who)).collect();
            ranks.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (place, (_, who)) in ranks.into_iter().enumerate() {
                self.stride(who, enemies, place, out);
            }
        }
    }

    /// Someone goes up to the others, to a place of its own, as far as it
    /// can in a round.
    fn stride(&mut self, who: Who, enemies: &[Who], place: usize, out: &mut Vec<Rush>) {
        let Some(other) = self.nearest(who, enemies) else {
            return;
        };
        let (start, aim) = (self.at[&who], self.at[&other]);
        let far = start.distance(aim).max(1.0);
        let gone = (far - NEAR).min(STRIDE);
        let goal = (start + (aim - start) * gone / far).floor();
        let standing = self.standing();
        let free = standing.iter().flatten().filter(|one| **one != who).all(|one| self.at[one].distance(goal) >= ROOM);
        if gone > 2.0 && free && !(self.ground.known() && (self.ground.blocked(goal, 1) || !self.ground.clear(start, goal))) {
            self.at.insert(who, goal);
            out.push(Rush::Strides { who, to: goal, place });
        }
    }

    /// What a blow takes there. (Those who are to win give more than they
    /// take.)
    fn worth(&mut self, who: Who, other: Who, known: &Move, side: usize) -> i32 {
        let rules = self.rules.clone();
        let stats = |one: Who| rules.kinds.get(&self.species[&one]).map(|kind| kind.stats(LEVEL)).unwrap_or_else(|| Kind::default().stats(LEVEL));
        let fighter = |one: Who| Statuses::default().fighter(&stats(one), true);
        let power = known.slot.power.max(40);
        let damage = calc_damage_with(
            &mut self.rng,
            &rules.constants,
            rules.chart.as_ref(),
            &fighter(who),
            &fighter(other),
            known.slot.type_id,
            power,
            0,
            FIXED_ONE,
            known.slot.category == 1,
            0,
        );
        let blow = damage.amount.clamp(1, MOST_BLOW);
        match self.chosen {
            Some(chosen) => ((blow as f32 * if side == chosen { FAVOUR } else { 1.0 / FAVOUR }) as i32).max(1),
            None => blow,
        }
    }

    /// Then the blows, once they stand: only those at the front strike, the
    /// nearest.
    fn blows(&mut self, standing: &[Vec<Who>; 2], out: &mut Vec<Rush>) {
        let rules = self.rules.clone();
        for side in 0..2 {
            for (place, &who) in standing[side].iter().enumerate() {
                let enemies: Vec<Who> = standing[1 - side].iter().copied().filter(|one| self.health[one] > 0).collect();
                if self.health[&who] <= 0 {
                    continue;
                }
                let Some(other) = self.nearest(who, &enemies).filter(|other| self.at[other].distance(self.at[&who]) <= FRONT) else {
                    continue;
                };
                let known = rules.known(self.species[&who], LEVEL);
                let known = known[self.rng.rand_int(known.len() as i32) as usize].clone();
                let taken = self.worth(who, other, &known, side);
                let around = AROUND.contains(&known.slot.range);
                out.push(Rush::Strikes { who, other, id: known.slot.id, side, place, around });
                let mut struck = vec![other];
                if around {
                    let near = enemies.iter().copied().filter(|one| *one != other && self.at[one].distance(self.at[&other]) < AROUND_REACH);
                    struck.extend(near.take(AROUND_MOST));
                }
                for one in struck {
                    let left = self.health[&one] - taken;
                    self.health.insert(one, left);
                    out.push(Rush::Takes { one, by: who, taken, left, side, place });
                }
            }
        }
    }

    /// Few left, against many: they run for it, round what stands in the
    /// way, then out of sight.
    fn run_away(&mut self, standing: &[Vec<Who>; 2], routed: usize, out: &mut Vec<Rush>) {
        let others = &standing[1 - routed];
        let middle = others.iter().map(|one| self.at[one].x).sum::<f32>() / others.len().max(1) as f32;
        let far_end = self.ground.wide().map_or(FAR_END, |wide| wide - EDGE);
        for &who in &standing[routed] {
            self.health.insert(who, 0);
            let from = self.at[&who];
            let to = self.ground.free(Vec2::new(if from.x < middle { EDGE } else { far_end }, from.y));
            self.at.insert(who, to);
            out.push(Rush::Runs { who, to });
        }
        out.push(Rush::Ran);
    }

    /// Those who won are glad.
    fn ends(&mut self, out: &mut Vec<Rush>) {
        self.done = true;
        let standing = self.standing();
        let won = self.chosen.unwrap_or(usize::from(standing[1].len() > standing[0].len()));
        out.extend(standing[won].iter().map(|who| Rush::Glad { who: *who }));
    }
}
