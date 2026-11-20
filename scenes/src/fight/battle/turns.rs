//! Who does what, turn after turn, and what wears on.

use super::*;

impl Battle {
    /// A step of the fight: one's turn, and all that comes of it. (`choice`:
    /// that of whoever plays, when it was asked; nothing, for one chosen
    /// for it.)
    pub fn step(&mut self, choice: Option<Choice>) -> Vec<Sight> {
        let mut out = Vec::new();
        if !self.opened {
            self.opened = true;
            out.push(Sight::Opens { first: self.fighter(0).who, second: self.fighter(1).who });
            return out;
        }
        if self.finished.is_some() {
            return out;
        }
        match choice {
            Some(Choice::Changing) => {
                self.menu = Menu::Changing;
                return out;
            }
            Some(Choice::Back) => {
                self.menu = Menu::Main;
                return out;
            }
            _ => self.menu = Menu::Main,
        }
        if self.written_turn(&mut out) {
            return out;
        }
        if self.mode == Mode::Together {
            self.turn_together(&mut out);
        } else {
            self.turn_in_relay(choice, &mut out);
        }
        out
    }

    fn turn_in_relay(&mut self, choice: Option<Choice>, out: &mut Vec<Sight>) {
        let longest = if self.mode == Mode::Played { LONGEST_PLAYED } else { LONGEST_BOUT };
        self.spared = self.winner.filter(|&side| self.teams[side].standing().count() == 1).map(|side| self.fighter(side).who);
        if self.turns >= longest {
            self.bout_ended(out);
            self.falls(out);
            return;
        }
        let side = self.side;
        let (me, foe) = (self.teams[side].at, self.teams[1 - side].at);
        let changed = matches!(choice, Some(Choice::Change(_)));
        self.turn((side, me), (1 - side, foe), choice, out);
        self.side = if changed { 0 } else { 1 - side };
        self.turns += 1;
        self.falls(out);
    }

    fn turn_together(&mut self, out: &mut Vec<Sight>) {
        let side = (self.turns % 2) as usize;
        let standing = |team: &Team| team.standing().collect::<Vec<usize>>();
        self.spared = self.chosen_to_stand().or(self.winner.and_then(|at| {
            let team = &self.teams[at];
            let left = standing(team);
            (left.len() == 1).then(|| team.bodies[left[0]].who)
        }));
        let (ours, theirs) = (standing(&self.teams[side]), standing(&self.teams[1 - side]));
        if self.turns_said().is_some_and(|most| self.turns >= most) {
            self.last_blows(out);
            return;
        }
        if self.turns >= LONGEST_TOGETHER {
            let won = self.winner.unwrap_or(0);
            self.finished = Some(won);
            out.push(Sight::Won { side: won, level: self.fighter(won).stats.level });
            return;
        }
        let me = ours[(self.turns as usize / 2) % ours.len()];
        let aim = theirs[self.rng.rand_int(theirs.len() as i32) as usize];
        if ours.len() == 2 && self.rng.rand_int(100) < TOGETHER {
            let partner = ours.iter().copied().find(|&at| at != me).unwrap_or(me);
            out.push(Sight::Partners { first: self.teams[side].bodies[me].who, second: self.teams[side].bodies[partner].who });
            let known = self.pick_any(side, partner);
            let blow = self.blow((side, partner), (1 - side, aim), known);
            if let Sight::Blow(mut blow) = blow {
                blow.beside = true;
                out.push(Sight::Blow(blow));
            }
        }
        self.turn((side, me), (1 - side, aim), None, out);
        self.turns += 1;
        self.falls(out);
    }

    fn pick_any(&mut self, side: usize, at: usize) -> usize {
        let count = self.teams[side].bodies[at].moves.len() as i32;
        self.rng.rand_int(count.max(1)) as usize
    }

    /// What someone does on its turn.
    fn turn(&mut self, me: (usize, usize), foe: (usize, usize), choice: Option<Choice>, out: &mut Vec<Sight>) {
        let (who, other) = (self.teams[me.0].bodies[me.1].who, self.teams[foe.0].bodies[foe.1].who);
        let body = &mut self.teams[me.0].bodies[me.1];
        body.guarded = false;
        if let Some(ailment) = body.stopped() {
            (body.statuses.frozen_turns, body.statuses.sleep_turns) = (0, 0);
            out.push(Sight::Stopped { who, ailment });
            return;
        }
        if std::mem::take(&mut body.resting) {
            out.push(Sight::Rests { who });
            return;
        }
        let paralysed = body.statuses.paralysis_turns > 0;
        if paralysed && self.rng.rand_int(100) < PARALYSIS {
            out.push(Sight::Stopped { who, ailment: Ailment::Paralysed });
        } else {
            if self.mode != Mode::Played && self.rng.rand_int(100) < STEPS_ABOUT {
                out.push(Sight::StepsAside { who, other });
            }
            let chosen = choice.unwrap_or_else(|| self.chosen(me, foe));
            self.does(me, foe, chosen, out);
        }
        self.wears(me, out);
    }

    /// What a side's one chooses by itself. (Low, it eats; hurt, it may
    /// shield itself; now and then what does not strike; else what works
    /// best, most often.)
    fn chosen(&mut self, me: (usize, usize), foe: (usize, usize)) -> Choice {
        let (body, team) = (&self.teams[me.0].bodies[me.1], &self.teams[me.0]);
        let (hp, most) = (body.stats.hp, body.stats.max_hp);
        let (eats, shielding) = (hp * 100 < most * LOW_SHARE && team.berries > 0, hp * 2 < most && team.shields > 0);
        let roll = self.rng.rand_int(100);
        if eats {
            return Choice::Berry;
        }
        if shielding && roll < FOE_SHIELDS {
            return Choice::Shield;
        }
        let body = self.teams[me.0].bodies[me.1].clone();
        let other = self.teams[foe.0].bodies[foe.1].clone();
        let deed = body.moves.iter().position(|known| !known.strikes());
        if let Some(deed) = deed.filter(|_| roll >= 100 - FOE_DEEDS && !body.deed_used) {
            self.teams[me.0].bodies[me.1].deed_used = true;
            return Choice::Move(deed);
        }
        let mut ranked: Vec<(usize, i32)> =
            body.moves.iter().enumerate().filter(|(_, known)| known.strikes()).map(|(at, known)| (at, self.worth(&body, known, &other))).collect();
        ranked.sort_by_key(|(_, worth)| -worth);
        let Some(&(best, _)) = ranked.first() else {
            return Choice::Move(0);
        };
        let chosen: Vec<usize> = std::iter::repeat_n(best, BEST_MOVE).chain(ranked.iter().map(|(at, _)| *at)).collect();
        Choice::Move(chosen[self.rng.rand_int(chosen.len() as i32) as usize])
    }

    fn does(&mut self, me: (usize, usize), foe: (usize, usize), chosen: Choice, out: &mut Vec<Sight>) {
        let who = self.teams[me.0].bodies[me.1].who;
        match chosen {
            Choice::Berry => {
                self.teams[me.0].berries -= 1;
                let body = &mut self.teams[me.0].bodies[me.1];
                let healed = body.healed(body.stats.max_hp / BERRY_SHARE);
                out.push(Sight::Berry { who, healed });
            }
            Choice::Shield => {
                self.teams[me.0].shields -= 1;
                self.teams[me.0].bodies[me.1].guarded = true;
                out.push(Sight::Shields { who });
            }
            Choice::Change(to) => {
                out.push(Sight::GoesBack { side: me.0, who });
                self.teams[me.0].at = to;
                let body = &self.teams[me.0].bodies[to];
                out.push(Sight::Enters { side: me.0, who: body.who, left: body.stats.hp, most: body.stats.max_hp });
            }
            Choice::Move(at) => {
                let sight = self.blow(me, foe, at);
                out.push(sight);
            }
            Choice::Changing | Choice::Back => {}
        }
    }

    /// A burn, poison, the sky: a little of its life, each turn.
    pub(super) fn wears(&mut self, me: (usize, usize), out: &mut Vec<Sight>) {
        let body = &self.teams[me.0].bodies[me.1];
        if body.stats.hp == 0 {
            return;
        }
        let (who, most, types, burning) = (body.who, body.stats.max_hp, body.stats.types, body.statuses.burn_turns > 0 || body.statuses.poisoned);
        let mut sky = weather::Weather::default();
        sky.current = self.sky;
        if sky.hurts(types) {
            let strike = self.take(me, (most / HARSH_SHARE).max(1));
            out.push(Sight::Worn { who, strike, harsh: true });
        }
        if burning && self.teams[me.0].bodies[me.1].stats.hp > 0 {
            let strike = self.take(me, (most / WEARING_SHARE).max(1));
            out.push(Sight::Worn { who, strike, harsh: false });
        }
        let body = &mut self.teams[me.0].bodies[me.1];
        let was = body.ailment();
        body.statuses.paralysis_turns = (body.statuses.paralysis_turns - 1).max(0);
        body.statuses.confused_turns = (body.statuses.confused_turns - 1).max(0);
        body.statuses.cringe_turns = 0;
        if let Some(ailment) = was.filter(|_| body.ailment().is_none()) {
            out.push(Sight::Cured { who, ailment });
        }
    }

    /// It has lasted long enough: who is ahead ends it.
    fn bout_ended(&mut self, out: &mut Vec<Sight>) {
        let (first, second) = (self.fighter(0).stats.hp, self.fighter(1).stats.hp);
        let ahead = self.winner.unwrap_or(usize::from(second > first));
        let behind = 1 - ahead;
        let (mine, theirs) = ((ahead, self.teams[ahead].at), (behind, self.teams[behind].at));
        self.spared = None;
        let hit = self.fighter(ahead).moves.iter().max_by_key(|known| known.slot.power).cloned().unwrap_or_else(|| self.rules.plain_move());
        let left = self.teams[behind].bodies[theirs.1].stats.hp;
        let strike = self.take(theirs, left);
        let landing = Landing {
            target: self.teams[behind].bodies[theirs.1].who,
            strikes: vec![strike],
            missed: false,
            blocked: false,
            critical: true,
            matchup: matchup::NEUTRAL,
            ailment: None,
            stats: Vec::new(),
        };
        let who = self.teams[mine.0].bodies[mine.1].who;
        out.push(Sight::Blow(Blow {
            who,
            power: hit,
            landings: vec![landing],
            drained: None,
            recoil: None,
            spent: false,
            blast: None,
            stats: Vec::new(),
            beside: false,
        }));
    }

    /// Who has nothing left falls, or comes back; the next comes in; or
    /// it is over.
    pub(super) fn falls(&mut self, out: &mut Vec<Sight>) {
        for side in [1, 0] {
            for at in 0..self.teams[side].bodies.len() {
                let body = &mut self.teams[side].bodies[at];
                if body.stats.hp > 0 || body.down {
                    continue;
                }
                let who = body.who;
                if std::mem::take(&mut body.seed) {
                    let most = body.stats.max_hp;
                    let healed = body.healed(most);
                    body.statuses = Statuses::for_species(body.statuses.movement);
                    out.push(Sight::Reborn { who, healed });
                    continue;
                }
                body.down = true;
                let more = self.teams[side].standing().next();
                // (All on the field: no one takes its place, it is not carried off.)
                out.push(Sight::Falls { who, more: more.is_some() && self.mode != Mode::Together });
                if self.mode == Mode::Together {
                    out.push(Sight::Leaves { who });
                    continue;
                }
                if at != self.teams[side].at {
                    continue;
                }
                if let Some(next) = more {
                    self.teams[side].at = next;
                    let body = &self.teams[side].bodies[next];
                    out.push(Sight::Enters { side, who: body.who, left: body.stats.hp, most: body.stats.max_hp });
                    (self.side, self.turns) = (0, 0);
                }
            }
            if self.teams[side].standing().next().is_none() && self.finished.is_none() {
                let won = 1 - side;
                self.finished = Some(won);
                out.push(Sight::Won { side: won, level: self.fighter(won).stats.level });
            }
        }
    }
}
