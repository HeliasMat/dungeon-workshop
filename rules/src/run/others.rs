//! The turns of the others: once the leader has acted, its team, then the
//! foes, as the game's AI picks for them. Those who walk move together;
//! each attack is seen to its end before the next one (`Dungeon::wait`).
//!
//! Team members follow their tactics (see `Progress::tactics`): they pick up
//! what they walk on, spring the traps and act at their own speed. ("All
//! for one" and "Group safety" are never unlocked, see `tactic`.)

use crate::dungeon::ai::{self, AiChoice, AiUser, Candidate};
use crate::dungeon::combat::TYPE_COUNT;
use crate::dungeon::exclusive::effect::NO_WEATHER_DAMAGE;
use crate::dungeon::status::{DEEP_SLEEP, item, problems};
use crate::dungeon::{
    Action, Belly, DungeonRng, Floor, Mobility, MoveEffect, Pos, Tile, TrapOutcome, ability, apply_effect, eat_item, enemy_action, held, iq, stack,
};

use super::attack::chebyshev;
use super::dungeon::{DIRECTIONS, Phase, weather_name};
use super::leader::mobility;
use super::team::tactic;
use super::watch::watched;
use super::{Book, Dungeon, Monster, MonsterId, Run, Side};

/// What came of a monster's action.
enum Acted {
    /// Others may act now.
    Done,
    /// It attacked: that is to be seen.
    Attacked,
    /// It will do nothing more this round.
    Over,
}

/// Index in `ai::DIRECTIONS` of where a monster faces.
fn facing_index(facing: (i32, i32)) -> usize {
    ai::DIRECTIONS.iter().position(|&way| way == facing).unwrap_or(0)
}

/// How many actions a monster has this round: slow ones act every other
/// round, fast ones several times.
fn actions(monster: &Monster, weather: u8, round: u32) -> i32 {
    match monster.status.speed_stage_in(&monster.stats, weather) {
        0 => round.is_multiple_of(2) as i32,
        stage => stage,
    }
}

impl Run {
    pub(crate) fn others_turn(&mut self) {
        let dungeon = self.here();
        if !dungeon.learning.is_empty() || dungeon.ask.is_some() || dungeon.banner > 0 || dungeon.knocked_out.is_some() {
            return;
        }
        // An attack is still being seen: the others wait for it.
        if dungeon.wait > 0 {
            dungeon.wait -= 1;
            return;
        }
        if dungeon.phase.is_none() && !dungeon.enemy_phase {
            return;
        }
        // Where their turn was stopped for an attack to be seen, or the start
        // of a new one.
        let mut phase = match dungeon.phase.take() {
            Some(phase) => phase,
            None => match self.round_starts() {
                Some(phase) => phase,
                None => return,
            },
        };
        let previous = phase.previous;
        while phase.rounds > 0 {
            let dungeon = self.here();
            if !phase.begun {
                phase.begun = true;
                dungeon.round += 1;
                // (The team first, as the team acts before the enemies.)
                let of = |side: Side| {
                    dungeon.monsters.iter().filter(move |monster| monster.side == side && (side == Side::Ally || monster.alive())).map(|monster| monster.id)
                };
                phase.order = of(Side::Ally).chain(of(Side::Foe)).collect();
            }
            while phase.next < phase.order.len() {
                let who = phase.order[phase.next];
                let dungeon = self.here();
                let (weather, round, silence) = (dungeon.weather.apparent(), dungeon.round, dungeon.silence);
                let Some(monster) = dungeon.monster_mut(who) else {
                    (phase.next, phase.act) = (phase.next + 1, 0);
                    continue;
                };
                let side = monster.side;
                if silence && side == Side::Foe {
                    monster.moves.iter_mut().for_each(|slot| slot.sealed = true);
                }
                if phase.act >= actions(monster, weather, round) {
                    (phase.next, phase.act) = (phase.next + 1, 0);
                    continue;
                }
                let act = phase.act;
                phase.act += 1;
                let acted = match side {
                    Side::Foe => self.foe_acts(who, act),
                    _ => self.ally_acts(who, previous),
                };
                match acted {
                    Acted::Done => {}
                    Acted::Over => (phase.next, phase.act) = (phase.next + 1, 0),
                    Acted::Attacked => {
                        let wait = self.attack_wait();
                        let dungeon = self.here();
                        dungeon.wait = wait;
                        dungeon.phase = Some(phase);
                        return;
                    }
                }
            }
            phase = Phase { rounds: phase.rounds - 1, previous, ..Phase::default() };
        }
    }

    /// The leader's action is over: unless it is fast enough to act again,
    /// what the end of a turn brings, and the others' rounds to come.
    fn round_starts(&mut self) -> Option<Phase> {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        let dungeon = self.here();
        dungeon.enemy_phase = false;
        // Where the leader stood before its last action: the partner walks
        // in its footsteps.
        let at = dungeon.leader().at;
        let previous = dungeon.trail.replace(at);
        // A fast leader acts several times before the others move.
        let leader = dungeon.leader();
        let (id, name) = (leader.id, leader.name.clone());
        let speed = leader.status.speed_stage_in(&leader.stats, dungeon.weather.apparent());
        dungeon.actions_this_round += 1;
        if dungeon.actions_this_round < speed {
            return None;
        }
        dungeon.actions_this_round = 0;
        // The leader's turn is over: hunger and passive status effects apply.
        let chain_belly = std::mem::take(&mut dungeon.chain_belly);
        let belly = self.change(id, |leader| {
            leader.status.end_turn(&mut leader.stats, constants);
            let Some(belly) = &mut leader.belly else {
                return (0, 0);
            };
            let before = belly.points();
            belly.end_turn(&mut leader.stats, constants);
            belly.lose(chain_belly);
            (before, belly.points())
        });
        // Warnings as the belly empties.
        match belly.unwrap_or_default() {
            (before, now) if before > 20 && now <= 20 => self.log.push(format!("{name} is getting hungry...")),
            (before, now) if before > 10 && now <= 10 => self.log.push(format!("{name} is very hungry! It must eat something!")),
            (before, 0) if before > 0 => self.log.push(format!("{name} is starving! Its HP is dropping!")),
            _ => {}
        }
        // Team members recover HP as turns pass, silently; enemies do not.
        let dungeon = self.here();
        let leader = dungeon.leader_mut();
        leader.status.regenerate(&mut leader.stats, constants);
        let allies: Vec<MonsterId> =
            dungeon.monsters.iter().filter(|monster| monster.side == Side::Ally && monster.alive()).map(|monster| monster.id).collect();
        for ally in allies {
            self.change(ally, |ally| ally.status.end_turn(&mut ally.stats, constants));
            if let Some(ally) = self.here().monster_mut(ally) {
                ally.status.regenerate(&mut ally.stats, constants);
            }
        }
        self.weather_turn();
        self.held_items_turn();
        self.bad_dreams_turn();
        // At half speed the others get two rounds for each of the leader's.
        Some(Phase { rounds: if speed == 0 { 2 } else { 1 }, previous, ..Phase::default() })
    }

    /// Bad Dreams: each turn, the foes of its bearer asleep where it sees
    /// them lose HP.
    fn bad_dreams_turn(&mut self) {
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        let dreamers: Vec<(Pos, bool)> = dungeon
            .monsters
            .iter()
            .filter(|monster| monster.alive() && monster.side != Side::Keeper && ability::has(&monster.stats, ability::BAD_DREAMS))
            .map(|monster| (monster.at, monster.team()))
            .collect();
        for (from, team) in dreamers {
            let floor = &dungeon.floor;
            for monster in dungeon.monsters.iter_mut() {
                let tormented = monster.alive() && monster.side != Side::Keeper && monster.team() != team && monster.status.asleep();
                if !tormented || monster.at == from || !ai::can_see(floor, from, monster.at) {
                    continue;
                }
                watched(monster, &mut self.sights, |monster| monster.stats.hp -= ability::BAD_DREAMS_DAMAGE);
                self.log.push(format!("{} is tormented by Bad Dreams and lost {} HP!", monster.name, ability::BAD_DREAMS_DAMAGE));
            }
        }
    }

    /// Held items that act as turns pass: a Warp Scarf now and then warps
    /// its holder; a team member's No-Slip Cap now and then makes an item
    /// of the bag sticky.
    fn held_items_turn(&mut self) {
        let dungeon = self.here();
        let holders: Vec<(MonsterId, bool, bool)> = dungeon
            .monsters
            .iter()
            .filter(|monster| monster.alive() && monster.side != Side::Keeper)
            .map(|monster| (monster.id, held::holds(&monster.stats, held::WARP_SCARF), monster.team() && held::holds(&monster.stats, held::NO_SLIP_CAP)))
            .filter(|&(_, warp, slip)| warp || slip)
            .collect();
        for (who, warp, slip) in holders {
            let dungeon = self.here();
            let rng = &mut dungeon.rng;
            let Some(monster) = dungeon.monsters.iter_mut().find(|monster| monster.id == who) else {
                continue;
            };
            let warps = warp && held::ticks(&mut monster.status.warp_scarf_count, rng);
            let sticks = slip && held::ticks(&mut monster.status.no_slip_count, rng);
            if warps {
                let taken: Vec<Pos> = dungeon.monsters.iter().filter(|monster| monster.alive()).map(|monster| monster.at).collect();
                self.warp(who, &taken);
            }
            if sticks {
                self.stick_bag_item();
            }
        }
    }

    /// What the weather does at the end of a turn: a monster's ability may
    /// bring it; sandstorms and hail strike everyone they can hurt; in their
    /// weather, some abilities hurt, heal or cure.
    fn weather_turn(&mut self) {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        // A monster on the floor whose ability brings a weather keeps it
        // there (Drizzle, Drought, Sand Stream, Snow Warning).
        let met = dungeon.monsters.iter().filter(|monster| monster.side != Side::Keeper && monster.alive());
        if let Some(kind) = met.map(|monster| &monster.stats).find_map(ability::brings_weather) {
            dungeon.weather.set(kind, 2);
        }
        let before = dungeon.weather.current;
        let strikes = dungeon.weather.end_turn() && !dungeon.weather.suppressed;
        let (damage, now) = (constants.bad_weather_damage, dungeon.weather.apparent());
        let every_ten = dungeon.turn.is_multiple_of(10);
        let weather = dungeon.weather;
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.side != Side::Keeper) {
            // (An exclusive item may spare a team member.)
            let team = monster.team();
            let banded = held::holds(&monster.stats, held::WEATHER_BAND);
            let spared = banded || (team && (monster.status.exclusive.has(NO_WEATHER_DAMAGE) || ability::has(&monster.stats, ability::MAGIC_GUARD)));
            if !monster.alive() || spared {
                continue;
            }
            let lost = watched(monster, &mut self.sights, |monster| {
                let mut lost = 0;
                if strikes && weather.hurts(monster.stats.types) && !ability::weathers(&monster.stats, now) {
                    lost += damage;
                }
                if every_ten {
                    lost += ability::weather_damage(&monster.stats, now, damage);
                }
                monster.stats.hp = (monster.stats.hp - lost).min(monster.stats.max_hp);
                lost
            });
            if lost > 0 && monster.side == Side::Leader {
                self.log.push(format!("{} lost {lost} HP to the weather.", monster.name));
            }
        }
        // Shed Skin, and Hydration in the rain, cure status problems.
        let rng = &mut dungeon.rng;
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.side != Side::Keeper) {
            if monster.alive() && monster.status.ailing() && ability::cures(rng, &monster.stats, now) {
                apply_effect(MoveEffect::CureStatus, rng, constants, &mut monster.stats, &mut monster.status);
                self.log.push(format!("{} was healed thanks to its ability!", monster.name));
            }
        }
        if dungeon.weather.current != before {
            self.log.push(format!("The weather changed: {}.", weather_name(dungeon.weather.current)));
        }
    }

    /// A team member's action: an attack, else a step as its tactic says.
    fn ally_acts(&mut self, who: MonsterId, previous: Option<Pos>) -> Acted {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        let tactics = self.team.progress.tactics.clone();
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        let secondary_kind = dungeon.spec(&book).map_or(0, |floor| floor.secondary_kind);
        let leader = dungeon.leader();
        let (leader_at, leader_alive, leader_decoy) = (leader.at, leader.alive(), leader.status.decoy);
        let Some(me) = dungeon.monster(who) else {
            return Acted::Over;
        };
        let sleep_use = me.status.sleep_move(&me.moves);
        if !me.alive() || !leader_alive || (me.status.cannot_act() && sleep_use.is_none()) {
            return Acted::Done;
        }
        // (Item Master: it eats what it holds when that does it good.)
        if worth_eating(me) {
            let name = me.name.clone();
            return self.eats_held(who, &name);
        }
        let at = me.at;
        let foes: Vec<Pos> = dungeon.monsters.iter().filter(|monster| monster.side == Side::Foe && monster.alive()).map(|monster| monster.at).collect();
        let mut beside: Vec<Pos> =
            dungeon.monsters.iter().filter(|monster| monster.side == Side::Ally && monster.id != who && monster.alive()).map(|monster| monster.at).collect();
        beside.extend(dungeon.shop.as_ref().and_then(|shop| shop.keeper));
        // (What its IQ keeps it off, it walks round.)
        beside.extend(shunned(dungeon, me, secondary_kind));
        let walks = mobility(&me.stats, &me.status, secondary_kind);
        let direction = facing_index(me.facing);
        // Its tactic:
        // some keep away from fights, at low HP or always; one waits for a
        // foe to come close.
        let tactic = tactics.get(&me.name).copied().unwrap_or(tactic::TOGETHER);
        // (Run Away: below half its HP it flees, whatever its tactic; Stall:
        // its moves only on a foe next to it.)
        let low = me.stats.hp * 2 <= me.stats.max_hp;
        let flees = tactic == tactic::GET_AWAY || (tactic == tactic::AVOID_TROUBLE && low) || ability::runs_away(&me.stats, false);
        let close_foe = foes.iter().any(|&foe| chebyshev(at, foe) <= 1);
        let confused = me.status.confused();
        let around = Around { at, leader_at, walks, foes: &foes, beside: &beside };
        // Dedicated Traveler: it walks first, and strikes only when it has
        // nowhere to go.
        let travels = iq::has(&me.stats, iq::DEDICATED_TRAVELER)
            && !me.status.rooted()
            && around.ally_step(&dungeon.floor, &mut dungeon.rng.clone(), tactic, low, flees, previous, false).is_some();
        let holds_back = flees
            || travels
            || tactic == tactic::KEEP_DISTANCE
            || (tactic == tactic::AVOID_FIRST_HIT && !close_foe)
            || ability::stalls(&me.stats, false, close_foe);
        let choice = if let Some(slot) = sleep_use {
            Some(AiChoice { slot: Some(slot), direction })
        } else if me.status.cannot_attack() || holds_back {
            None
        } else if me.status.charging.is_some() {
            Some(AiChoice { slot: None, direction })
        } else {
            // (A leader made a decoy is a foe to its allies.)
            let seen = |monster: &Monster| monster.alive() && (monster.side == Side::Foe || monster.side == Side::Leader);
            let others: Vec<Candidate> = dungeon
                .monsters
                .iter()
                .filter(|monster| seen(monster))
                .map(|monster| candidate(&book, me, monster, monster.side == Side::Foe || leader_decoy))
                .collect();
            let user = ai_user(me, direction, walks);
            let moves = me.moves.clone();
            ai::choose_move(&mut dungeon.rng, &dungeon.floor, &constants.ai_regular_attack_weights, &user, &moves, &others)
        };
        // Confused, it strikes any way.
        let choice = match choice {
            Some(choice) if confused && sleep_use.is_none() => Some(AiChoice { direction: dungeon.rng.rand_int(8) as usize, ..choice }),
            choice => choice,
        };
        let Some(me) = dungeon.monster_mut(who) else {
            return Acted::Over;
        };
        let unleash = me.status.charging.take().and_then(|id| me.moves.iter().position(|slot| slot.id == id));
        if let Some(choice) = choice {
            me.facing = ai::DIRECTIONS[choice.direction];
            self.use_chain(who, unleash.or(choice.slot), unleash.is_some(), true, true, None);
            return Acted::Attacked;
        }
        if me.status.rooted() {
            return Acted::Done;
        }
        let step = around.ally_step(&dungeon.floor, &mut dungeon.rng, tactic, low, flees, previous, confused);
        let Some((dx, dy)) = step else {
            return Acted::Done;
        };
        let to = Pos::new(at.x + dx, at.y + dy);
        if let Some(me) = dungeon.monster_mut(who) {
            (me.facing, me.at) = ((dx, dy), to);
        }
        // What lies there goes in the bag, if there is room.
        self.pick_up(who);
        // A trap goes off under it.
        if let Some(trap) = self.here().traps.iter().position(|trap| trap.at == to && !trap.team) {
            self.spring_elsewhere(who, trap);
        }
        Acted::Done
    }

    /// A trap under one that is not the leader: of the traps that move
    /// their victim, all send it elsewhere on the floor; the others do
    /// nothing more to it.
    fn spring_elsewhere(&mut self, who: MonsterId, trap: usize) {
        let outcome = self.spring(who, trap);
        if matches!(outcome, TrapOutcome::Warp | TrapOutcome::Fall | TrapOutcome::Blow) {
            let taken = self.standing();
            let dungeon = self.here();
            if let Some(tile) = dungeon.warp_tile(&taken)
                && let Some(monster) = dungeon.monster_mut(who)
            {
                monster.at = tile;
            }
        }
    }

    /// A foe's action: `act` counts those of its round.
    fn foe_acts(&mut self, who: MonsterId, act: i32) -> Acted {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        let secondary_kind = dungeon.spec(&book).map_or(0, |floor| floor.secondary_kind);
        let Some(me) = dungeon.monster(who).cloned() else {
            return Acted::Over;
        };
        if !me.alive() || !dungeon.leader().alive() {
            return Acted::Over;
        }
        let at = me.at;
        // The team members still standing; the closest one it sees is hunted
        // (out of sight, it wanders).
        let standing = || dungeon.monsters.iter().filter(|monster| monster.alive());
        let foes: Vec<Pos> = standing().filter(|monster| monster.team()).map(|monster| monster.at).collect();
        let visible = || standing().filter(|monster| monster.team() && monster.status.invisible_turns == 0);
        let mut seen_foes: Vec<Pos> = visible().map(|monster| monster.at).collect();
        let kin: Vec<Pos> = standing().filter(|monster| monster.side == Side::Foe && monster.id != who).map(|monster| monster.at).collect();
        // A decoy among the others draws their blows (Decoy Orb).
        let decoy = standing().find(|monster| monster.side == Side::Foe && monster.id != who && monster.status.decoy).map(|monster| (monster.id, monster.at));
        const NOWHERE: Pos = Pos::new(-100, -100);
        let nearest = seen_foes.iter().copied().min_by_key(|&foe| chebyshev(at, foe));
        // A team member under Follow Me, a decoy (Substitute) or one in
        // Gaggle Specs, in sight, draws it.
        let draws = |monster: &&Monster| monster.status.drawing_turns > 0 || monster.status.decoy || held::holds(&monster.stats, held::GAGGLE_SPECS);
        let drawing = visible().find(draws).map(|monster| monster.at);
        let drawn_to = decoy.map(|(_, tile)| tile).or(drawing.filter(|&tile| ai::can_see(&dungeon.floor, at, tile)));
        let target = drawn_to.or(nearest).unwrap_or(NOWHERE);
        // (A decoy is the one foe it goes for.)
        if let Some(decoy) = drawn_to {
            seen_foes = vec![decoy];
        }
        let keeper = dungeon.shop.as_ref().and_then(|shop| shop.keeper);
        // A monster that appeared asleep wakes once a team member stands
        // next to it (serebii.net's notes on wild monsters), unless in a
        // Sneak Scarf; one with a Racket Band wakes it from where it sees it.
        let wakes = |monster: &Monster| {
            let next = chebyshev(at, monster.at) <= 1 && !held::holds(&monster.stats, held::SNEAK_SCARF);
            next || (held::holds(&monster.stats, held::RACKET_BAND) && ai::can_see(&dungeon.floor, monster.at, at))
        };
        if me.status.sleep_turns == DEEP_SLEEP && standing().filter(|monster| monster.team()).any(wakes) {
            if let Some(me) = dungeon.monster_mut(who) {
                me.status.sleep_turns = 0;
            }
            self.log.push(book.woke_up(&me.name));
            return Acted::Over;
        }
        // (Item Master: hurt, it eats the berry it holds; ailing, what cures
        // it.)
        if worth_eating(&me) {
            return self.eats_held(who, &me.name);
        }
        let walks = mobility(&me.stats, &me.status, secondary_kind);
        let avoided = shunned(dungeon, &me, secondary_kind);
        let occupied = |tile: Pos| tile == at || foes.contains(&tile) || kin.contains(&tile) || Some(tile) == keeper || avoided.contains(&tile);
        // The attack comes first; a monster without one in reach walks.
        let direction = facing_index(me.facing);
        // (Run Away: below half its HP it flees as the terrified do; Stall:
        // its moves only on a foe next to it, its blows otherwise.)
        let terrified = me.status.terrified_turns > 0 || ability::runs_away(&me.stats, false);
        let stalls = ability::stalls(&me.stats, false, foes.iter().any(|&foe| chebyshev(at, foe) <= 1));
        let choice = if let Some(slot) = me.status.sleep_move(&me.moves) {
            Some(AiChoice { slot: Some(slot), direction })
        } else if me.status.cannot_attack() || terrified || stalls {
            None
        } else if me.status.charging.is_some() {
            // A charged move goes off where its user faces.
            Some(AiChoice { slot: None, direction })
        } else {
            let at_tile = |pos: Pos| standing().find(|monster| monster.at == pos);
            let hostile = seen_foes.iter().filter_map(|&pos| at_tile(pos)).map(|monster| candidate(&book, &me, monster, true));
            let others: Vec<Candidate> =
                hostile.chain(kin.iter().filter_map(|&pos| at_tile(pos)).map(|monster| candidate(&book, &me, monster, false))).collect();
            let user = ai_user(&me, direction, walks);
            ai::choose_move(&mut dungeon.rng, &dungeon.floor, &constants.ai_regular_attack_weights, &user, &me.moves, &others)
        };
        let action = foe_action(&dungeon.floor, &mut dungeon.rng, &me, target, choice, occupied, &foes, walks);
        self.foe_does(who, act, at, action, choice, decoy.map(|(id, _)| id))
    }

    /// A monster eats what it holds (Item Master).
    fn eats_held(&mut self, who: MonsterId, name: &str) -> Acted {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        let Dungeon { rng, monsters, .. } = dungeon;
        let Some(eater) = monsters.iter_mut().find(|monster| monster.id == who) else {
            return Acted::Over;
        };
        let berry = std::mem::take(&mut eater.stats.held);
        watched(eater, &mut self.sights, |eater| eat_item(berry, rng, constants, &mut eater.stats, &mut eater.status, &mut Belly::full(), &mut eater.moves));
        self.log.push(format!("{name} ate {}!", book.item_name(berry)));
        Acted::Done
    }

    /// A foe's action done: its turn's end on its first action, then its
    /// attack (the move the AI chose, or the regular attack) or its step.
    fn foe_does(&mut self, who: MonsterId, act: i32, at: Pos, action: Action, choice: Option<AiChoice>, decoy: Option<MonsterId>) -> Acted {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        if act == 0 {
            self.change(who, |me| me.status.end_turn(&mut me.stats, constants));
        }
        let Some(me) = self.here().monster_mut(who) else {
            return Acted::Over;
        };
        // A charged move is lost when its user cannot strike.
        if !matches!(action, Action::Attack(..)) {
            me.status.charging = None;
        }
        if let Action::Attack(dx, dy) | Action::Move(dx, dy) = action {
            me.facing = (dx, dy);
        }
        match action {
            Action::Attack(..) => {
                // The move the AI chose (see `dungeon::ai`), or the regular
                // attack.
                let unleash = me.status.charging.take().and_then(|id| me.moves.iter().position(|slot| slot.id == id));
                let pick = choice.and_then(|choice| choice.slot);
                self.use_chain(who, unleash.or(pick), unleash.is_some(), false, true, decoy);
                Acted::Attacked
            }
            Action::Move(dx, dy) => {
                let to = Pos::new(at.x + dx, at.y + dy);
                me.at = to;
                self.foe_steps(who, to);
                Acted::Done
            }
            Action::Wait => Acted::Done,
        }
    }

    /// A foe came on a tile: what lies there, a trap of the team's.
    fn foe_steps(&mut self, who: MonsterId, to: Pos) {
        let book = self.book.clone();
        // A foe holding nothing picks up what it walks on (Rescue Team's
        // `dungeon_ai_items`), and holds it.
        self.pick_up(who);
        // A trap of the team's goes off under it.
        if let Some(trap) = self.here().traps.iter().position(|trap| trap.at == to && trap.team) {
            self.spring_elsewhere(who, trap);
            if let Some(me) = self.here().monster(who).filter(|me| !me.alive()) {
                let name = me.name.clone();
                self.log.push(book.knocked_out(&name));
            }
        }
    }
}

/// Where a team member stands, and who stands around it: what its step is
/// chosen by.
struct Around<'a> {
    at: Pos,
    leader_at: Pos,
    walks: Mobility,
    /// The foes still standing.
    foes: &'a [Pos],
    /// The other team members, and a shopkeeper.
    beside: &'a [Pos],
}

impl Around<'_> {
    /// A team member's step, as its tactic says: towards the closest enemy
    /// in sight, or back to the leader; away from a foe; none. Confused, it
    /// steps any way it can.
    fn ally_step(&self, floor: &Floor, rng: &mut DungeonRng, tactic: u8, low: bool, flees: bool, previous: Option<Pos>, confused: bool) -> Option<(i32, i32)> {
        let Around { at, leader_at, walks, foes, beside } = *self;
        let free = |(dx, dy): (i32, i32)| {
            let to = Pos::new(at.x + dx, at.y + dy);
            (dx, dy) != (0, 0) && floor.can_step_as(at, dx, dy, walks) && to != leader_at && !foes.contains(&to) && !beside.contains(&to)
        };
        let towards = |goal: Pos| ((goal.x - at.x).signum(), (goal.y - at.y).signum());
        let chase = |foe: Pos| {
            let (sx, sy) = towards(foe);
            [(sx, sy), (sx, 0), (0, sy)].into_iter().find(|&step| free(step))
        };
        let in_sight = || foes.iter().copied().filter(|&foe| ai::can_see(floor, at, foe));
        // (The team goes together: an enemy is gone for only close by, and
        // close to the leader.)
        let seen = in_sight().filter(|&foe| chebyshev(at, foe) <= 3 && chebyshev(leader_at, foe) <= 4).min_by_key(|&foe| chebyshev(at, foe));
        let footstep = previous.filter(|&tile| tile != leader_at && chebyshev(at, tile) == 1).map(towards).filter(|&step| free(step));
        // Any foe in sight, however far, for those that go after them or
        // their own way.
        let hunted = in_sight().min_by_key(|&foe| chebyshev(at, foe));
        let nearest = hunted.filter(|&foe| chebyshev(at, foe) <= 2);
        // Away from a foe: the free step that leaves it farthest.
        let after_step = |(dx, dy): (i32, i32), from: Pos| chebyshev(Pos::new(at.x + dx, at.y + dy), from);
        let away = |from: Pos| {
            ai::DIRECTIONS
                .iter()
                .copied()
                .filter(|&step| free(step))
                .max_by_key(|&step| after_step(step, from))
                .filter(|&step| after_step(step, from) > chebyshev(at, from))
        };
        let wander = |rng: &mut DungeonRng| {
            let steps: Vec<(i32, i32)> = ai::DIRECTIONS.iter().copied().filter(|&step| free(step)).collect();
            (!steps.is_empty()).then(|| steps[rng.rand_int(steps.len() as i32) as usize])
        };
        let back = || ai::step_towards(floor, at, leader_at, walks, |tile| foes.contains(&tile) || beside.contains(&tile));
        let step = match tactic {
            tactic::WAIT_THERE => None,
            tactic::BE_PATIENT if low => None,
            _ if flees => nearest.and_then(away),
            tactic::KEEP_DISTANCE if nearest.is_some_and(|foe| chebyshev(at, foe) <= 1) => nearest.and_then(away),
            tactic::GO_AFTER_FOES | tactic::OWN_WAY => match hunted {
                Some(foe) => chase(foe),
                None if tactic == tactic::OWN_WAY => wander(rng),
                None => back(),
            },
            _ => match seen {
                None if chebyshev(at, leader_at) <= 1 => None,
                None if footstep.is_some() => footstep,
                Some(foe) => chase(foe),
                // Lost sight of the leader: the shortest way back.
                None => back(),
            },
        };
        match step {
            Some(_) if confused => Some(DIRECTIONS[rng.rand_int(8) as usize]).filter(|&way| free(way)),
            step => step,
        }
    }
}

/// Item Master: whether a monster eats what it holds now, as the berry or
/// seed does it good: one that heals when hurt, one that cures what ails it.
fn worth_eating(me: &Monster) -> bool {
    let (stats, status) = (&me.stats, &me.status);
    if !iq::has(stats, iq::ITEM_MASTER) || status.embargo_turns > 0 || stack::sticky(stats.held) {
        return false;
    }
    match stack::id(stats.held) {
        item::ORAN_BERRY | item::SITRUS_BERRY => stats.hp * 2 <= stats.max_hp,
        item::PECHA_BERRY => status.poisoned,
        item::RAWST_BERRY => status.burn_turns > 0,
        item::CHERI_BERRY => status.paralysis_turns > 0,
        item::HEAL_SEED => status.ailing(),
        _ => false,
    }
}

/// Another monster as the AI of `user` sees it: its status problems, and
/// for a team member choosing among foes, what makes one worth more (Exp.
/// Go-Getter: what beating it is worth; Efficiency Expert: the less HP it
/// has, the game's 500 less its HP).
fn candidate(book: &Book, user: &Monster, other: &Monster, hostile: bool) -> Candidate {
    const EFFICIENCY_TOP: i32 = 500;
    let mut seen = Candidate::new(other.at, hostile);
    seen.problems = problems(&other.status);
    if user.team() && !other.team() {
        seen.worth = match (iq::has(&user.stats, iq::EXP_GO_GETTER), iq::has(&user.stats, iq::EFFICIENCY_EXPERT)) {
            (true, _) => other.exp_value,
            (false, true) => EFFICIENCY_TOP - other.stats.hp,
            _ => 1,
        }
        .max(1);
    }
    if let Some(chart) = book.tables.types.as_ref() {
        for (kind, fares) in seen.matchups.iter_mut().enumerate().take(TYPE_COUNT) {
            *fares = chart.overall(kind as u8, &other.stats);
        }
    }
    seen
}

/// A monster as its own AI goes by.
fn ai_user(me: &Monster, facing: usize, mobility: Mobility) -> AiUser {
    AiUser { pos: me.at, facing, mobility, full_hp: me.stats.hp >= me.stats.max_hp, skills: me.stats.iq_skills, types: me.stats.types }
}

/// The tiles a monster's IQ keeps it off as it walks: lava it could cross
/// (Lava Evader), the traps it sees that would go off under it (Trap
/// Avoider), a Monster House not yet woken (House Avoider).
fn shunned(dungeon: &Dungeon, me: &Monster, secondary_kind: u8) -> Vec<Pos> {
    const LAVA: u8 = 1;
    let floor = &dungeon.floor;
    let mut tiles = Vec::new();
    if secondary_kind == LAVA && iq::has(&me.stats, iq::LAVA_EVADER) {
        tiles.extend(Floor::positions().filter(|&tile| floor.tile(tile) == Tile::Secondary));
    }
    if iq::has(&me.stats, iq::TRAP_AVOIDER) {
        tiles.extend(dungeon.traps.iter().filter(|trap| trap.seen && trap.team != me.team()).map(|trap| trap.at));
    }
    if iq::has(&me.stats, iq::HOUSE_AVOIDER) && dungeon.monsters.iter().any(|monster| monster.dormant) {
        tiles.extend(Floor::positions().filter(|&tile| floor.in_monster_house(tile)));
    }
    tiles
}

/// Confused (or seeing things) without Nontraitor, a monster's blows may
/// strike its own side.
pub(crate) fn traitor(me: &Monster) -> bool {
    me.status.confused() && !iq::has(&me.stats, iq::NONTRAITOR)
}

/// A foe's action: the attack the AI chose, else fleeing the closest of the
/// team if terrified, else a step towards `target`; confusion sends it any
/// way, and what it cannot do it waits instead of.
fn foe_action(
    floor: &Floor,
    rng: &mut DungeonRng,
    me: &Monster,
    target: Pos,
    choice: Option<AiChoice>,
    occupied: impl Fn(Pos) -> bool + Copy,
    foes: &[Pos],
    walks: Mobility,
) -> Action {
    let at = me.at;
    let terrified = me.status.terrified_turns > 0 || ability::runs_away(&me.stats, false);
    let action = match choice {
        Some(choice) => {
            let (dx, dy) = ai::DIRECTIONS[choice.direction];
            Action::Attack(dx, dy)
        }
        None if me.status.cannot_act() => Action::Wait,
        // Terrified, it flees from the closest of the team.
        None if terrified => {
            let steps = ai::DIRECTIONS.iter().copied().filter(|&(dx, dy)| floor.can_step_as(at, dx, dy, walks) && !occupied(Pos::new(at.x + dx, at.y + dy)));
            let away = |(dx, dy): (i32, i32)| chebyshev(Pos::new(at.x + dx, at.y + dy), target);
            match steps.max_by_key(|&step| away(step)) {
                Some(step) if away(step) > chebyshev(at, target) => Action::Move(step.0, step.1),
                _ => Action::Wait,
            }
        }
        None => enemy_action(floor, rng, at, target, occupied, walks),
    };
    let action = match action {
        // Confusion sends moves and steps in a random direction.
        Action::Attack(..) | Action::Move(..) if me.status.confused() => {
            let (dx, dy) = DIRECTIONS[rng.rand_int(8) as usize];
            let tile = Pos::new(at.x + dx, at.y + dy);
            if !floor.can_step_as(at, dx, dy, walks) {
                Action::Wait
            } else if foes.contains(&tile) || (traitor(me) && occupied(tile)) {
                Action::Attack(dx, dy)
            } else if occupied(tile) {
                Action::Wait
            } else {
                Action::Move(dx, dy)
            }
        }
        action => action,
    };
    match action {
        Action::Attack(..) if me.status.cannot_attack() => Action::Wait,
        Action::Move(..) if me.status.rooted() => Action::Wait,
        action => action,
    }
}
