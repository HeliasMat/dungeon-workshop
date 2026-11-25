//! The leader's turn: walking, attacking, passing, and what stepping on a
//! tile does. Its moves go as everyone's (`Run::use_chain`): its own is only
//! how it picks them.

use crate::dungeon::status::item;
use crate::dungeon::{Floor, Mobility, MoveSlot, Pos, Statuses, Tile, TrapOutcome, ai, spring_trap, stack, weather};

use super::dungeon::{DIRECTIONS, FloorItem, Outcome, STEP_FRAMES, ShopTalk};
use super::spawning::{spawn_enemy, spawn_enemy_holding};
use super::watch::watched;
use super::{Input, MonsterId, Run, Side, Sight};

/// A trap sprung.
const TRAP_SOUND: u16 = 0x11C;

/// How a monster moves on a floor with that secondary terrain.
pub(crate) fn mobility(status: &Statuses, secondary_kind: u8) -> Mobility {
    match (status.mobile, status.levitating_turns > 0) {
        (true, _) => Mobility::Intangible,
        (false, true) => Mobility::Hovering,
        // (Slip: water is walked on, as by those that swim.)
        (false, false) if status.slipping && secondary_kind == 0 => Mobility::resolve(status.movement, secondary_kind).max(Mobility::Secondary),
        (false, false) => Mobility::resolve(status.movement, secondary_kind),
    }
}

impl super::Dungeon {
    /// Where a partner standing on that tile is pushed to by the leader
    /// walking that way, with the option (`RuleOptions::push_allies`): one
    /// tile on, if it can go there; none: the two trade places.
    pub(crate) fn push_spot(&self, at: Pos, (dx, dy): (i32, i32), secondary_kind: u8) -> Option<Pos> {
        let on = Pos::new(at.x + dx, at.y + dy);
        let ally = self.monsters.iter().find(|monster| monster.side == Side::Ally && monster.at == at)?;
        let free = !self.monsters.iter().any(|monster| monster.at == on && monster.alive());
        let can = !ally.status.rooted() && self.floor.can_step_as(at, dx, dy, mobility(&ally.status, secondary_kind));
        (self.rules.options.push_allies && free && can).then_some(on)
    }
}

impl Run {
    /// The clocks of the frame: the step, the floor's banner.
    pub(crate) fn count_down(&mut self) {
        let unseen = self.unseen;
        let dungeon = self.here();
        dungeon.cooldown = dungeon.cooldown.saturating_sub(1);
        dungeon.banner = if unseen { 0 } else { dungeon.banner.saturating_sub(1) };
    }

    /// The leader has acted: the others act next.
    pub(crate) fn finish_turn(&mut self) {
        let cooldown = if self.unseen { 0 } else { STEP_FRAMES };
        let dungeon = self.here();
        dungeon.turn += 1;
        dungeon.floor_turns += 1;
        dungeon.mud_sport = (dungeon.mud_sport - 1).max(0);
        dungeon.water_sport = (dungeon.water_sport - 1).max(0);
        dungeon.enemy_phase = true;
        dungeon.cooldown = cooldown;
        self.wind();
    }

    /// The leader can act now, nothing being waited for.
    pub(crate) fn leader_ready(&self) -> bool {
        self.dungeon.as_ref().is_some_and(|dungeon| {
            let shop_talks = dungeon.shop.as_ref().is_some_and(|shop| shop.talk.is_some());
            !(dungeon.cooldown > 0
                || dungeon.banner > 0
                || !dungeon.learning.is_empty()
                || shop_talks
                || dungeon.ask.is_some()
                || dungeon.enemy_phase
                || dungeon.phase.is_some()
                || dungeon.wait > 0
                || dungeon.knocked_out.is_some())
        })
    }

    /// One action = one turn. A leader that cannot act loses its turn; a
    /// charged move is unleashed, whatever is asked.
    pub(crate) fn leader_turn(&mut self, input: &Input) {
        if !self.leader_ready() {
            return;
        }
        let (mut way, mut attack, mut key, mut pass) = ((0, 0), false, None, false);
        match *input {
            Input::Idle | Input::QueueMove(_) => {}
            Input::Go { toward } => way = toward,
            Input::Face(toward) => self.here().leader_mut().facing = toward,
            Input::Attack => attack = true,
            Input::UseMove(slot) => key = Some(slot),
            Input::Wait => pass = true,
            // (A menu is open.)
            _ => return,
        }
        let book = self.book.clone();
        let dungeon = self.here();
        let secondary_kind = dungeon.spec(&book).map_or(0, |floor| floor.secondary_kind);
        let leader = dungeon.leader();
        let (id, name) = (leader.id, leader.name.clone());
        let walks = mobility(&leader.status, secondary_kind);
        // Asleep, it snores or talks in its sleep if it can, else its turn
        // is lost.
        let sleep_use = leader.status.sleep_move(&leader.moves);
        if leader.status.cannot_act() && sleep_use.is_none() {
            return self.finish_turn();
        }
        let queued = dungeon.queued.take();
        let leader = dungeon.leader_mut();
        let chosen = key.or(queued).filter(|&slot| leader.moves.get(slot).is_some_and(MoveSlot::usable));
        // A charged move is unleashed on the next turn, whatever is pressed.
        let unleash = leader.status.charging.take().and_then(|id| leader.moves.iter().position(|slot| slot.id == id));
        let chosen = sleep_use.or(unleash).or(chosen);
        // A move key with every move out of PP: Struggle, which hurts its user.
        let out_of_pp = !leader.moves.is_empty() && leader.moves.iter().all(|slot| slot.pp <= 0);
        let struggling = out_of_pp && chosen.is_none() && key.is_some();
        let attacking = chosen.is_some() || attack || struggling;
        let cannot_attack = leader.status.cannot_attack();
        let confused = leader.status.confused();
        // Confusion sends moves and steps in a random direction.
        let stray = (confused && (attacking || way != (0, 0))).then(|| DIRECTIONS[dungeon.rng.rand_int(8) as usize]);
        if attacking && cannot_attack && sleep_use.is_none() {
            self.log.push(format!("{name} cannot attack!"));
            return self.finish_turn();
        }
        if let (true, Some(direction)) = (attacking, stray) {
            self.here().leader_mut().facing = direction;
        }
        let acted = if attacking {
            if struggling {
                self.log.push(format!("{name} has no PP left and struggles!"));
                let recoil = (self.here().leader().stats.max_hp / 4).max(1);
                self.hurt(id, recoil);
                self.log.push(format!("{name} lost {recoil} HP."));
            }
            self.use_chain(id, chosen, unleash.is_some(), sleep_use.is_none(), false, None);
            true
        } else if way != (0, 0) {
            match self.walk(stray.unwrap_or(way), walks) {
                Some(acted) => acted,
                // (Talking with Kecleon takes no turn.)
                None => return,
            }
        } else {
            pass
        };
        if acted {
            self.finish_turn();
        }
    }

    /// A step that way: whether it took the turn; `None`: it was a word
    /// with Kecleon.
    fn walk(&mut self, (dx, dy): (i32, i32), walks: Mobility) -> Option<bool> {
        let book = self.book.clone();
        let dungeon = self.here();
        let leader = dungeon.leader_mut();
        leader.facing = (dx, dy);
        let (from, name, rooted) = (leader.at, leader.name.clone(), leader.status.rooted());
        let target = Pos::new(from.x + dx, from.y + dy);
        let blocked = dungeon.monsters.iter().any(|monster| monster.at == target && monster.alive() && monster.side == Side::Foe);
        // Walking into Kecleon is talking to it: to pay, or to sell.
        if let Some(shop) = dungeon.shop.as_mut().filter(|shop| shop.keeper == Some(target)) {
            let total: i32 = shop.owed.iter().map(|(_, price)| price).sum();
            let said = if total > 0 {
                shop.talk = Some(ShopTalk::Pay);
                format!("The shopkeeper: That will be {total} Poké in all. Will you pay?")
            } else {
                shop.talk = Some(ShopTalk::Sell);
                "The shopkeeper: Do you want to sell me something?".to_string()
            };
            self.log.push(said);
            self.here().cooldown = STEP_FRAMES;
            return None;
        }
        if rooted {
            self.log.push(format!("{name} is rooted and cannot move."));
            return Some(true);
        }
        if !dungeon.floor.can_step_as(from, dx, dy, walks) || blocked {
            return Some(false);
        }
        let secondary_kind = dungeon.spec(&book).map_or(0, |floor| floor.secondary_kind);
        let pushed = dungeon.push_spot(target, (dx, dy), secondary_kind);
        // The leader trades places with a partner standing in the way (or
        // pushes it on).
        for monster in &mut dungeon.monsters {
            if monster.side == Side::Ally && monster.at == target {
                monster.at = pushed.unwrap_or(from);
            } else if monster.side == Side::Leader {
                monster.at = target;
            }
        }
        Some(true)
    }

    /// Who steps on a trap sets it off. What it does beyond changing them
    /// is for the caller to carry out.
    pub(crate) fn spring(&mut self, who: MonsterId, trap: usize) -> TrapOutcome {
        let book = self.book.clone();
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        let (kind, at) = (dungeon.traps[trap].kind, dungeon.traps[trap].at);
        dungeon.traps[trap].seen = true;
        self.sights.push(Sight::Trap(kind, at));
        let Some(monster) = dungeon.monsters.iter_mut().find(|monster| monster.id == who) else {
            return TrapOutcome::Done;
        };
        self.log.push(format!("{} stepped on a trap!", monster.name));
        if monster.side == Side::Leader {
            self.sights.push(Sight::Sound(TRAP_SOUND));
        }
        let rng = &mut dungeon.rng;
        watched(monster, &mut self.sights, |monster| {
            spring_trap(kind, rng, &book.tables.constants, &mut monster.stats, &mut monster.status, &mut monster.moves)
        })
    }

    /// Takes what lies under the leader once it came there, and springs
    /// the trap. A trap is revealed the first time and works every time.
    pub(crate) fn step_on_tile(&mut self) {
        let dungeon = self.here();
        let leader = dungeon.leader();
        let (id, at) = (leader.id, leader.at);
        if dungeon.stood == Some(at) {
            return;
        }
        dungeon.stood = Some(at);
        self.pick_up(id);
        if let Some(trap) = self.here().traps.iter().position(|trap| trap.at == at && !trap.team) {
            let outcome = self.spring(id, trap);
            self.trap_outcome(outcome);
        }
    }

    /// What a trap does around the leader that stepped on it.
    fn trap_outcome(&mut self, outcome: TrapOutcome) {
        let book = self.book.clone();
        let dungeon = self.here();
        let leader = dungeon.leader();
        let (id, name, centre, held) = (leader.id, leader.name.clone(), leader.at, leader.stats.held);
        let foes: Vec<Pos> = dungeon.monsters.iter().filter(|monster| !monster.team()).map(|monster| monster.at).collect();
        match outcome {
            TrapOutcome::Done => {}
            // Up to ten tiles in a random direction; an obstacle hurts.
            TrapOutcome::Blow => {
                let way = DIRECTIONS[dungeon.rng.rand_int(8) as usize];
                self.log.push(format!("{name} was blown away by a gust!"));
                self.blow_back(id, centre, way);
            }
            TrapOutcome::Explode(radius) => {
                // Rain keeps explosions from going off.
                if dungeon.weather.apparent() == weather::RAIN {
                    self.log.push("The explosion fizzled out.".to_string());
                } else {
                    dungeon.blasts.push((centre, radius, None));
                    self.log.push("An explosion went off!".to_string());
                }
            }
            TrapOutcome::Sticky => {
                let clean: Vec<usize> =
                    (0..self.bag.items.len()).filter(|&at| !stack::piles(self.bag.items[at]) && !stack::sticky(self.bag.items[at])).collect();
                if clean.is_empty() {
                    self.log.push("But nothing happened.".to_string());
                    return;
                }
                let at = clean[self.dungeon.as_mut().expect("a dungeon").rng.rand_int(clean.len() as i32) as usize];
                self.bag.items[at] = stack::stuck(self.bag.items[at], true);
                self.log.push(format!("{} became sticky!", book.item_name(stack::id(self.bag.items[at]))));
            }
            TrapOutcome::Trip => {
                // What it holds falls beside it.
                let lying: Vec<Pos> = dungeon.items.iter().map(|item| item.at).collect();
                let floor = &dungeon.floor;
                let beside = ai::DIRECTIONS
                    .iter()
                    .map(|&(dx, dy)| Pos::new(centre.x + dx, centre.y + dy))
                    .find(|&tile| floor.tile(tile).is_walkable() && !lying.contains(&tile) && !foes.contains(&tile) && tile != floor.stairs);
                match (held, beside) {
                    (0, _) | (_, None) => self.log.push(format!("{name} tripped!")),
                    (held, Some(at)) => {
                        dungeon.leader_mut().stats.held = 0;
                        dungeon.items.push(FloorItem { at, item: held, price: None });
                        self.log.push(format!("{name} tripped and dropped {}!", book.item_name(held)));
                    }
                }
            }
            TrapOutcome::GrimyFood => {
                let food = [item::APPLE, item::BIG_APPLE, item::HUGE_APPLE, item::GOLDEN_APPLE];
                let mut spoiled = false;
                for lying in self.bag.items.iter_mut().filter(|lying| food.contains(lying)) {
                    *lying = item::GRIMY_FOOD;
                    spoiled = true;
                }
                if spoiled {
                    self.log.push("The food in the bag went bad!".to_string());
                }
            }
            TrapOutcome::Warp => {
                let tiles: Vec<Pos> = Floor::positions().filter(|&tile| dungeon.floor.is_spawn_tile(tile)).collect();
                if !tiles.is_empty() {
                    let tile = tiles[dungeon.rng.rand_int(tiles.len() as i32) as usize];
                    dungeon.leader_mut().at = tile;
                }
            }
            TrapOutcome::Fall => {
                self.next_floor();
                self.here().leader_mut().status.reset_stages();
            }
            TrapOutcome::Summon(count, grudge) => {
                // Free open tiles within two steps of the trap.
                let floor = &dungeon.floor;
                let mut free: Vec<Pos> = (-2..=2)
                    .flat_map(|dy| (-2..=2).map(move |dx| Pos::new(centre.x + dx, centre.y + dy)))
                    .filter(|&tile| tile != centre && matches!(floor.tile(tile), Tile::Floor { .. }) && !foes.contains(&tile))
                    .collect();
                for _ in 0..count {
                    if free.is_empty() {
                        break;
                    }
                    let tile = free.swap_remove(dungeon.rng.rand_int(free.len() as i32) as usize);
                    spawn_enemy_holding(dungeon, &book, tile, false, grudge);
                }
                self.log.push("Foes appeared!".to_string());
            }
            TrapOutcome::ItemsToMonsters => {
                let room = dungeon.floor.tile(centre).room();
                let floor = &dungeon.floor;
                let affected = |at: Pos| match room {
                    Some(_) => floor.tile(at).room() == room,
                    None => (at.x - centre.x).abs() <= 1 && (at.y - centre.y).abs() <= 1,
                };
                let turned: Vec<Pos> = dungeon.items.iter().map(|item| item.at).filter(|&at| affected(at) && at != centre && !foes.contains(&at)).collect();
                dungeon.items.retain(|item| !turned.contains(&item.at));
                for tile in turned {
                    spawn_enemy(dungeon, &book, tile, false);
                }
                self.log.push("The items turned into Pokémon!".to_string());
            }
        }
    }

    /// The wind of a floor the team stays on too long: warnings as the
    /// dungeon's turn limit comes, then the team is blown out.
    // (When the warnings come, 250, 100 and 25 turns before, and their words
    // are this project's own.)
    fn wind(&mut self) {
        let dungeon = self.here();
        let limit = dungeon.rules.turn_limit as u32;
        if limit == 0 || dungeon.outcome.is_some() {
            return;
        }
        match limit.saturating_sub(dungeon.floor_turns) {
            250 => self.log.push("Something is stirring in the distance...".to_string()),
            100 => self.log.push("Something is approaching...".to_string()),
            25 => self.log.push("The wind is picking up! Hurry and leave!".to_string()),
            0 => {
                dungeon.outcome = Some(Outcome::BlownOut);
                self.log.push("A gust blew the team out of the dungeon!".to_string());
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::book::PLAYER_STATS;
    use super::super::{Dungeon, Monster};
    use super::*;

    #[test]
    fn an_ally_walked_into_is_pushed_on_with_the_option() {
        let mut dungeon = Dungeon::new(None, None, String::new(), None, 1);
        for y in 5..8 {
            for x in 5..10 {
                dungeon.floor.set(Pos::new(x, y), Tile::Floor { room: Some(0) });
            }
        }
        let monster = |id, side, x| Monster::new(MonsterId(id), side, String::new(), None, PLAYER_STATS, Vec::new(), Statuses::default(), Pos::new(x, 6));
        dungeon.monsters = vec![monster(1, Side::Leader, 6), monster(2, Side::Ally, 7)];
        // Without the option they trade places.
        assert_eq!(dungeon.push_spot(Pos::new(7, 6), (1, 0), 0), None);
        dungeon.rules.options.push_allies = true;
        assert_eq!(dungeon.push_spot(Pos::new(7, 6), (1, 0), 0), Some(Pos::new(8, 6)));
        // Not into a wall, nor onto someone.
        dungeon.monsters[1].at = Pos::new(9, 6);
        assert_eq!(dungeon.push_spot(Pos::new(9, 6), (1, 0), 0), None);
        dungeon.monsters[1].at = Pos::new(7, 6);
        dungeon.monsters.push(monster(3, Side::Foe, 8));
        assert_eq!(dungeon.push_spot(Pos::new(7, 6), (1, 0), 0), None);
    }
}
