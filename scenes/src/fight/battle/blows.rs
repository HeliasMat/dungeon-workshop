//! A move used, and all it does.

use super::*;

impl Battle {
    /// A move used.
    pub(super) fn blow(&mut self, me: (usize, usize), foe: (usize, usize), at: usize) -> Sight {
        let known = self.teams[me.0].bodies[me.1].moves.get(at).cloned().unwrap_or_else(|| self.rules.plain_move());
        self.blow_with(me, foe, known)
    }

    /// That move used, whether it knows it or not.
    pub(super) fn blow_with(&mut self, me: (usize, usize), foe: (usize, usize), known: Move) -> Sight {
        let who = self.teams[me.0].bodies[me.1].who;
        if let Some(effect @ MoveEffect::Explode(_)) = move_effect(known.slot.id) {
            return self.blast(me, foe, known, effect);
        }
        if !known.strikes() {
            return self.deed(me, foe, known);
        }
        let (edge, cap) = self.weighed(me, foe);
        let (landing, stats, outcome) = self.lands(me, foe, &known, edge, cap);
        let dealt: i32 = landing.strikes.iter().map(|strike| strike.taken).sum();
        let hit = !landing.missed && !landing.blocked && dealt > 0;
        let lost = outcome.happened.iter().find_map(|happened| if let Happened::Aftermath(lost) = happened { Some(*lost) } else { None }).filter(|_| hit);
        let left = self.held_on(me);
        let told = lost.map(|lost| Strike { taken: lost, left, held: false });
        let (drained, recoil) = (told.filter(|strike| strike.taken < 0), told.filter(|strike| strike.taken > 0));
        let spent = hit && outcome.happened.contains(&Happened::Rests);
        self.teams[me.0].bodies[me.1].resting = spent;
        Sight::Blow(Blow { who, power: known, landings: vec![landing], drained, recoil, spent, blast: None, stats, beside: false })
    }

    /// What its side's blows are worth here, and the most they take.
    fn weighed(&mut self, me: (usize, usize), foe: (usize, usize)) -> (f32, Option<i32>) {
        let played = self.mode == Mode::Played;
        let (mine, theirs) = if me.0 == 0 { (me.1, foe.1) } else { (foe.1, me.1) };
        let edges = self.edges_of(mine, theirs);
        let edge = if me.0 == 0 { edges.0 } else { edges.1 };
        let life = self.teams[foe.0].bodies[foe.1].stats.max_hp;
        let share = if me.0 == 1 { self.hardest } else { BEST_BLOW };
        (edge, played.then_some(life * share / 100))
    }

    /// The attack itself: `attack::resolve`, on the two, under this sky.
    fn resolved(&mut self, me: (usize, usize), foe: (usize, usize), attack: Attack, tuning: Tuning) -> Outcome {
        let rules = self.rules.clone();
        let [first, second] = &mut self.teams;
        let (mine, theirs) = if me.0 == 0 { (first, second) } else { (second, first) };
        let mut bonds = 0;
        let mut field = Field {
            rng: &mut self.rng,
            constants: &rules.constants,
            chart: rules.chart.as_ref(),
            moves: &rules.slots,
            weather: self.sky,
            gravity: false,
            mud_sport: false,
            water_sport: false,
            secret_power: 9,
            bonds: &mut bonds,
            sides: Default::default(),
        };
        attack::resolve(&mut mine.bodies[me.1].party(me.0 == 0), &mut theirs.bodies[foe.1].party(foe.0 == 0), &attack, tuning, &mut field)
    }

    /// Whether it gets there, and what it does there; what the user's
    /// stages did.
    pub(super) fn lands(
        &mut self,
        me: (usize, usize),
        foe: (usize, usize),
        known: &Move,
        edge: f32,
        cap: Option<i32>,
    ) -> (Landing, Vec<(Stat, bool)>, Outcome) {
        let body = self.teams[me.0].bodies[me.1].clone();
        let other = self.teams[foe.0].bodies[foe.1].clone();
        let fares = self.fares(known, &other);
        let mut landing = Landing {
            target: other.who,
            strikes: Vec::new(),
            missed: false,
            blocked: false,
            critical: false,
            matchup: fares,
            ailment: None,
            stats: Vec::new(),
        };
        // (Who held on is not touched again: it dodges, or it is shielded.)
        let held_on = self.surging == Some(other.who);
        if other.guarded || (held_on && self.rng.rand_int(2) == 0) {
            self.teams[foe.0].bodies[foe.1].guarded = false;
            landing.blocked = true;
            return (landing, Vec::new(), Outcome::default());
        }
        if held_on {
            landing.missed = true;
            return (landing, Vec::new(), Outcome::default());
        }
        if fares == matchup::IMMUNE {
            return (landing, Vec::new(), Outcome::default());
        }
        let worth = edge * if self.surging == Some(body.who) { SURGE } else { 1.0 } * if self.spared == Some(other.who) { SPARED } else { 1.0 };
        let attack = Attack { linked: false, slot: struck_with(known), effect: None, damage_mult: FIXED_ONE };
        let outcome = self.resolved(me, foe, attack, Tuning { worth, cap, staged: false });
        landing.missed = !outcome.landed;
        let mut left = other.stats.hp;
        for hurt in outcome.happened.iter().filter_map(|happened| if let Happened::Hurt(damage) = happened { Some(damage) } else { None }) {
            landing.critical |= hurt.critical;
            let before = left;
            left = (left - hurt.amount).max(0);
            let held = left == 0 && self.spared == Some(other.who);
            left = left.max(held as i32);
            landing.strikes.push(Strike { taken: before - left, left, held });
        }
        self.held_on(foe);
        let stats = self.seen_after(me, foe, &body, &other, &mut landing);
        (landing, stats, outcome)
    }

    /// Life left past an attack: never less than none, nor than one for who
    /// is to win, who comes back stronger.
    fn held_on(&mut self, on: (usize, usize)) -> i32 {
        let body = &mut self.teams[on.0].bodies[on.1];
        if body.stats.hp <= 0 && self.spared == Some(body.who) {
            body.stats.hp = 1;
            self.surging = Some(body.who);
        }
        body.stats.hp = body.stats.hp.max(0);
        body.stats.hp
    }

    /// What an attack left on the two, each kept short; the user's stages.
    fn seen_after(&mut self, me: (usize, usize), foe: (usize, usize), body: &Body, other: &Body, landing: &mut Landing) -> Vec<(Stat, bool)> {
        self.teams[me.0].bodies[me.1].kept_short();
        let target = &mut self.teams[foe.0].bodies[foe.1];
        target.kept_short();
        landing.stats.extend(stages_changed(other.stages(), target.stages()));
        if target.ailment() != other.ailment() {
            landing.ailment = target.ailment();
        }
        stages_changed(body.stages(), self.teams[me.0].bodies[me.1].stages())
    }

    /// A move that does not strike.
    fn deed(&mut self, me: (usize, usize), foe: (usize, usize), known: Move) -> Sight {
        let rules = self.rules.clone();
        let who = self.teams[me.0].bodies[me.1].who;
        let Some(effect) = known.slot.effect_in(&rules.constants, self.sky, &self.teams[me.0].bodies[me.1].stats) else {
            return Sight::Deed(Deed { who, power: known, healed: None, stats: Vec::new(), landing: None });
        };
        let on_itself = matches!(known.slot.target, rules::dungeon::combat::TARGET_PARTY | rules::dungeon::combat::TARGET_TEAMMATES) || effect.on_user();
        if on_itself {
            let body = &mut self.teams[me.0].bodies[me.1];
            let (before, hp) = (body.stages(), body.stats.hp);
            apply_effect(effect, &mut self.rng, &rules.constants, &mut body.stats, &mut body.statuses);
            body.kept_short();
            let left = body.stats.hp;
            let healed = (left > hp).then_some(Strike { taken: hp - left, left, held: false });
            let stats = stages_changed(before, body.stages());
            return Sight::Deed(Deed { who, power: known, healed, stats, landing: None });
        }
        let body = self.teams[me.0].bodies[me.1].clone();
        let other = self.teams[foe.0].bodies[foe.1].clone();
        let mut landing = Landing {
            target: other.who,
            strikes: Vec::new(),
            missed: false,
            blocked: false,
            critical: false,
            matchup: matchup::NEUTRAL,
            ailment: None,
            stats: Vec::new(),
        };
        if other.guarded {
            self.teams[foe.0].bodies[foe.1].guarded = false;
            landing.blocked = true;
        } else {
            let outcome = self.resolved(me, foe, Attack { linked: false, slot: known.slot, effect: Some(effect), damage_mult: FIXED_ONE }, Tuning::default());
            landing.missed = !outcome.landed;
            self.held_on(foe);
            self.held_on(me);
            self.seen_after(me, foe, &body, &other, &mut landing);
        }
        Sight::Deed(Deed { who, power: known, healed: None, stats: Vec::new(), landing: Some(landing) })
    }

    /// It blows up.
    fn blast(&mut self, me: (usize, usize), foe: (usize, usize), known: Move, effect: MoveEffect) -> Sight {
        let who = self.teams[me.0].bodies[me.1].who;
        let other = self.teams[foe.0].bodies[foe.1].who;
        let fares = self.fares(&known, &self.teams[foe.0].bodies[foe.1].clone());
        let attack = Attack { linked: false, slot: known.slot, effect: Some(effect), damage_mult: FIXED_ONE };
        let outcome = self.resolved(me, foe, attack, Tuning { staged: true, ..Tuning::default() });
        let costs = outcome.happened.iter().find_map(|happened| if let Happened::Blast { taken, spent } = happened { Some((*taken, *spent)) } else { None });
        let (taken, spent) = costs.unwrap_or_default();
        let strike = self.take(foe, taken);
        let blast = Some(self.take(me, spent));
        let landing =
            Landing { target: other, strikes: vec![strike], missed: false, blocked: false, critical: false, matchup: fares, ailment: None, stats: Vec::new() };
        Sight::Blow(Blow { who, power: known, landings: vec![landing], drained: None, recoil: None, spent: false, blast, stats: Vec::new(), beside: false })
    }

    /// Life goes; who is to win holds on, and comes back stronger.
    pub(super) fn take(&mut self, on: (usize, usize), amount: i32) -> Strike {
        let body = &mut self.teams[on.0].bodies[on.1];
        let (mut left, mut held) = ((body.stats.hp - amount).max(0), false);
        if left == 0 && self.spared == Some(body.who) {
            (left, held) = (1, true);
            self.surging = Some(body.who);
        }
        let taken = body.stats.hp - left;
        body.stats.hp = left;
        Strike { taken, left, held }
    }
}
