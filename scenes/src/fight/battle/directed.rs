//! A fight a film states in advance: its acts played in turn, each with what
//! the film says of it; what it does not say comes from the rules, all on the
//! field. The rest by chance, or it stops there.

use super::*;

/// What a blow is worth when it is said to be critical, as the game has it.
const CRITICAL: f32 = 1.5;
/// What a thing thrown takes, when the film does not say.
const THROWN: i32 = 10;
/// How fast time goes in a blow said to be slow.
const SLOW_ACT: f32 = 0.5;

impl Battle {
    /// The next turn the film wrote; or, past the last, the end if it said
    /// no more. Whether the turn is played.
    pub(super) fn written_turn(&mut self, out: &mut Vec<Sight>) -> bool {
        let Some(direction) = &mut self.direction else {
            return false;
        };
        let Some(stated) = direction.acts.pop_front() else {
            if direction.goes_on {
                return false;
            }
            let won = self.ahead();
            self.finished = Some(won);
            out.push(Sight::Won { side: won, level: self.fighter(won).stats.level });
            return true;
        };
        let act = match stated {
            Stated::Act(act) => act,
            Stated::Passage(label) => {
                out.push(Sight::Passage(label));
                return true;
            }
            Stated::Pause(frames) => {
                out.push(Sight::Waits(frames));
                return true;
            }
            Stated::Log(text) => {
                out.push(Sight::Says(text));
                return true;
            }
            Stated::Change { out: leaving, comes } => {
                self.change(leaving, comes, out);
                return true;
            }
            Stated::Uses { who, thing, name, worth } => {
                if let Some(me) = self.standing_place(who) {
                    let body = &mut self.teams[me.0].bodies[me.1];
                    let healed = body.healed(worth.unwrap_or(body.stats.max_hp / BERRY_SHARE));
                    out.push(Sight::Uses { who, thing, name, healed });
                    self.turns += 1;
                }
                return true;
            }
            Stated::Flings { who, thing, name, worth, target } => {
                if let (Some(_), Some(foe)) = (self.standing_place(who), self.standing_place(target)) {
                    let strike = self.take(foe, worth.unwrap_or(THROWN));
                    out.push(Sight::Flings { who, thing, name, target, strike });
                    self.turns += 1;
                    self.falls(out);
                }
                return true;
            }
        };
        // (Who has fallen does nothing: the next one plays.)
        let Some(me) = self.standing_place(act.who) else {
            return self.written_turn(out);
        };
        self.spared = None;
        let body = &mut self.teams[me.0].bodies[me.1];
        (body.guarded, body.resting) = (false, false);
        self.acts(me, &act, out);
        self.wears(me, out);
        self.turns += 1;
        self.falls(out);
        true
    }

    /// Someone gives its place to one who waits.
    fn change(&mut self, leaving: Who, comes: Who, out: &mut Vec<Sight>) {
        let Some((side, at)) = self.standing_place(leaving) else {
            return;
        };
        let Some(next) = self.teams[side].bodies.iter().position(|body| body.who == comes && body.benched && !body.down) else {
            return;
        };
        self.teams[side].bodies[at].benched = true;
        self.teams[side].bodies[next].benched = false;
        if self.teams[side].at == at {
            self.teams[side].at = next;
        }
        let life = self.teams[side].bodies[next].stats;
        out.push(Sight::Swaps { out: leaving, comes, left: life.hp, most: life.max_hp });
    }

    /// Who is to win is kept standing, whoever else falls.
    pub(super) fn chosen_to_stand(&self) -> Option<Who> {
        let who = self.direction.as_ref()?.chosen?;
        self.standing_place(who).map(|_| who)
    }

    /// The most turns the fight has, all told, if the film said.
    pub(super) fn turns_said(&self) -> Option<u32> {
        self.direction.as_ref()?.turns
    }

    /// Time is up: who is to win ends it, a blow for each still standing.
    pub(super) fn last_blows(&mut self, out: &mut Vec<Sight>) {
        let won = self.ahead();
        let at = self.chosen_to_stand().and_then(|who| self.standing_place(who)).map(|place| place.1).or(self.teams[won].standing().next());
        self.spared = None;
        for foe in self.teams[1 - won].standing().collect::<Vec<usize>>() {
            let hitter = &self.teams[won].bodies[at.unwrap_or(self.teams[won].at)];
            let hit = hitter.moves.iter().max_by_key(|known| known.slot.power).cloned().unwrap_or_else(|| self.rules.plain_move());
            let who = hitter.who;
            let sight = self.finishing_blow((1 - won, foe), hit, who);
            out.push(sight);
        }
        self.falls(out);
    }

    fn finishing_blow(&mut self, foe: (usize, usize), hit: Move, who: Who) -> Sight {
        let left = self.teams[foe.0].bodies[foe.1].stats.hp;
        let strike = self.take(foe, left);
        let landing = self.told_landing(foe, &hit, vec![strike]);
        Sight::Blow(blow_of(who, hit, vec![Landing { critical: true, ..landing }]))
    }

    /// The side to win: as said, else the one with most of its life left.
    fn ahead(&self) -> usize {
        let life =
            |side: usize| self.teams[side].bodies.iter().filter(|body| !body.benched).map(|body| body.stats.hp * 100 / body.stats.max_hp.max(1)).sum::<i32>();
        self.winner.unwrap_or(usize::from(life(1) > life(0)))
    }

    fn standing_place(&self, who: Who) -> Option<(usize, usize)> {
        (0..2).find_map(|side| Some((side, self.teams[side].bodies.iter().position(|body| body.who == who && !body.down && !body.benched)?)))
    }

    /// Those an act is on: those it names still standing, else the first foe;
    /// all of them if it says so.
    fn aimed_at(&self, side: usize, act: &Act) -> Vec<(usize, usize)> {
        let foes = 1 - side;
        let standing = || self.teams[foes].standing().map(|at| (foes, at));
        if act.all {
            return standing().collect();
        }
        let named: Vec<(usize, usize)> = act.targets.iter().filter_map(|who| self.standing_place(*who)).filter(|place| place.0 == foes).collect();
        if named.is_empty() { standing().take(1).collect() } else { named }
    }

    fn acts(&mut self, me: (usize, usize), act: &Act, out: &mut Vec<Sight>) {
        let who = self.teams[me.0].bodies[me.1].who;
        let Some(known) = self.rules.moves.get(act.power as usize).cloned() else {
            return;
        };
        if act.power == crate::fight::stage::PROTECT {
            self.teams[me.0].bodies[me.1].guarded = true;
            out.push(Sight::Shields { who });
            return;
        }
        let mut aims = self.aimed_at(me.0, act);
        let Some(&first) = aims.first() else {
            return;
        };
        // (Another of the same side, in its way.)
        let stepping = act.taken_by.and_then(|taker| self.standing_place(taker)).filter(|place| place.0 == first.0 && *place != first);
        if let (Some(place), Some(taker)) = (stepping, act.taken_by) {
            out.push(Sight::StepsBetween { who: taker, target: self.teams[first.0].bodies[first.1].who, from: who });
            aims = vec![place];
        }
        let first = aims[0];
        if act.slow {
            out.push(Sight::Tempo(SLOW_ACT));
        }
        if let Some(frames) = act.slowed_hit {
            out.push(Sight::SlowHit(frames));
        }
        self.acts_on(me, act, known, aims, first, out);
        if act.slow {
            out.push(Sight::Tempo(1.0));
        }
        if let (Some(_), Some(taker)) = (stepping, act.taken_by) {
            out.push(Sight::StepsBack { who: taker });
        }
    }

    /// What it does to those it is on.
    fn acts_on(&mut self, me: (usize, usize), act: &Act, known: Move, aims: Vec<(usize, usize)>, first: (usize, usize), out: &mut Vec<Sight>) {
        let exploding = matches!(move_effect(known.slot.id), Some(MoveEffect::Explode(_)));
        if !known.strikes() || (exploding && act.told == Told::Real) {
            let sight = self.blow_with(me, first, known);
            out.push(sight);
            return;
        }
        let mut landings = Vec::new();
        let mut struck: Option<Blow> = None;
        for foe in aims {
            if act.told == Told::Shielded && !self.teams[foe.0].bodies[foe.1].guarded {
                out.push(Sight::Shields { who: self.teams[foe.0].bodies[foe.1].who });
            }
            match self.told_blow(me, foe, &known, act.told) {
                Sight::Blow(blow) => {
                    let said = blow
                        .landings
                        .iter()
                        .cloned()
                        .map(|landing| Landing { critical: landing.critical || (act.critical && !landing.strikes.is_empty()), ..landing });
                    landings.extend(said);
                    struck.get_or_insert(blow);
                }
                other => out.push(other),
            }
        }
        if let Some(blow) = struck {
            out.push(Sight::Blow(Blow { landings, ..blow }));
        }
    }

    /// A blow as the film says it went.
    fn told_blow(&mut self, me: (usize, usize), foe: (usize, usize), known: &Move, told: Told) -> Sight {
        let who = self.teams[me.0].bodies[me.1].who;
        let sure = sure_of(known);
        match told {
            Told::Real => self.blow_with(me, foe, sure),
            Told::Critical => {
                let (mut landing, stats, _) = self.lands(me, foe, &sure, CRITICAL, None);
                landing.critical = !landing.strikes.is_empty();
                Sight::Blow(Blow { stats, ..blow_of(who, sure, vec![landing]) })
            }
            Told::Misses => {
                let landing = self.told_landing(foe, known, Vec::new());
                Sight::Blow(blow_of(who, sure, vec![Landing { missed: true, ..landing }]))
            }
            Told::Shielded => {
                self.teams[foe.0].bodies[foe.1].guarded = false;
                let landing = self.told_landing(foe, known, Vec::new());
                Sight::Blow(blow_of(who, sure, vec![Landing { blocked: true, ..landing }]))
            }
            Told::Falls => {
                let left = self.teams[foe.0].bodies[foe.1].stats.hp;
                let strike = self.take(foe, left);
                let landing = self.told_landing(foe, known, vec![strike]);
                Sight::Blow(blow_of(who, sure, vec![landing]))
            }
            Told::Takes(amount) => {
                let strike = self.take(foe, amount);
                let landing = self.told_landing(foe, known, vec![strike]);
                Sight::Blow(blow_of(who, sure, vec![landing]))
            }
        }
    }

    /// What lands of a blow whose numbers were said.
    fn told_landing(&self, foe: (usize, usize), known: &Move, strikes: Vec<Strike>) -> Landing {
        let other = &self.teams[foe.0].bodies[foe.1];
        Landing {
            target: other.who,
            strikes,
            missed: false,
            blocked: false,
            critical: false,
            matchup: self.fares(known, other),
            ailment: None,
            stats: Vec::new(),
        }
    }
}

/// A move that does not miss: the film said it lands.
fn sure_of(known: &Move) -> Move {
    let slot = MoveSlot { accuracy: 125, accuracy2: 125, ..known.slot };
    Move { slot, ..known.clone() }
}

fn blow_of(who: Who, power: Move, landings: Vec<Landing>) -> Blow {
    Blow { who, power, landings, drained: None, recoil: None, spent: false, blast: None, stats: Vec::new(), beside: false }
}
