//! The bag in a dungeon: using, throwing, giving and putting down items.
//! Its menu, cursor and pages are a screen's: it lists `Run::item_actions`
//! and sends the `Input`s of `Run::take_item_input`. A `Sight::BagDone` says
//! the thing asked was done and the menu closes; without it the log says
//! why not. Orbs are in `orbs`.

use crate::dungeon::status::{ItemUse, fussy_eaters, item as kind};
use crate::dungeon::{Belly, Damage, DungeonRng, FIXED_ONE, FLOOR_WIDTH, Floor, MoveSlot, Pos, eat_item, exclusive, held, iq, matchup, move_hits, stack};

use super::attack::Attack;
use super::monster::Side;
use super::orbs::Orb;
use super::team::earn;
use super::watch::watched;
use super::{Book, Input, Log, Monster, MonsterId, Run, Sight};

/// The number of the move thrown items strike with, in the move data.
const PROJECTILE_MOVE_ID: u16 = 0x195;

/// Sounds of items: picked up, money, thrown.
pub(crate) const PICK_UP_SOUND: u16 = 0x1304;
pub(crate) const MONEY_SOUND: u16 = 0x1308;
const THROW_SOUND: u16 = 0x130E;

/// Where a rock falls: on the nearest foe within reach in front of the
/// thrower (a cone widening as it goes, or the quarter of the floor a
/// diagonal faces), the one most straight ahead first; with no one there,
/// two tiles ahead, or short of a wall.
fn arc_landing(floor: &Floor, from: Pos, facing: (i32, i32), foes: &[Pos]) -> Pos {
    let target = foes.iter().filter_map(|&foe| arc_place(from, facing, foe).map(|place| (place, foe))).min_by_key(|&(place, _)| place);
    if let Some((_, foe)) = target {
        return foe;
    }
    let (fx, fy) = facing;
    let near = Pos::new(from.x + fx, from.y + fy);
    match (floor.can_step(from, fx, fy), floor.can_step(near, fx, fy)) {
        (true, true) => Pos::new(near.x + fx, near.y + fy),
        (true, false) => near,
        _ => from,
    }
}

/// How far ahead a tile is of a thrower facing that way, and how far off
/// to the side, when a thrown rock may come down on it.
fn arc_place(from: Pos, (fx, fy): (i32, i32), at: Pos) -> Option<(i32, i32)> {
    const REACH: i32 = 10;
    let (dx, dy) = (at.x - from.x, at.y - from.y);
    let (ahead, aside) = match fx != 0 && fy != 0 {
        // A diagonal: the quarter it looks into, by squares around.
        true if dx * fx >= 0 && dy * fy >= 0 => (dx.abs().max(dy.abs()), (dx.abs() - dy.abs()).abs()),
        true => return None,
        // Straight: as far aside as ahead, never quite.
        false => (dx * fx + dy * fy, (dx * fy - dy * fx).abs()),
    };
    ((1..=REACH).contains(&ahead) && aside < ahead).then_some((ahead, aside))
}

/// What can be done with an item of the bag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemAction {
    /// Use it (`Input::UseItem`). (Sticks and rocks have only `Throw`.)
    Use,
    /// `Input::ThrowItem`.
    Throw,
    /// That one of the team holds it (`Input::Give`): the leader, its
    /// partner.
    Give(MonsterId),
    /// `Input::PutDown`.
    PutDown,
}

impl Run {
    /// A monster, with what its change needs at hand, what it does to it
    /// being shown.
    pub(crate) fn work<R>(&mut self, who: MonsterId, work: impl FnOnce(&mut Monster, &mut DungeonRng, &Book, &mut Log) -> R) -> Option<R> {
        let dungeon = self.dungeon.as_mut()?;
        let rng = &mut dungeon.rng;
        let monster = dungeon.monsters.iter_mut().find(|monster| monster.id == who)?;
        let (book, log) = (&*self.book, &mut self.log);
        Some(watched(monster, &mut self.sights, |monster| work(monster, rng, book, log)))
    }

    /// "X fainted!", if it is.
    pub(crate) fn say_if_out(&mut self, who: MonsterId) {
        let out = self.dungeon.as_ref().and_then(|dungeon| dungeon.monster(who)).filter(|monster| !monster.alive()).map(|monster| monster.name.clone());
        if let Some(name) = out {
            self.log.push(self.book.knocked_out(&name));
        }
    }

    /// The bag may be opened: the leader is awake, and no move waits to be
    /// forgotten.
    pub fn bag_opens(&self) -> bool {
        self.dungeon.as_ref().is_none_or(|dungeon| dungeon.learning.is_empty() && !dungeon.leader().status.asleep())
    }

    /// What can be done with the item at that place of the bag, in order.
    pub fn item_actions(&self, index: usize) -> Vec<ItemAction> {
        let Some(&value) = self.bag.items.get(index) else {
            return Vec::new();
        };
        let item = stack::id(value);
        let mut actions = Vec::new();
        if !(1..=kind::LAST_PROJECTILE).contains(&item) {
            actions.push(ItemAction::Use);
        }
        actions.push(ItemAction::Throw);
        if held::is_held_item(value)
            && let Some(dungeon) = &self.dungeon
        {
            actions.push(ItemAction::Give(dungeon.leader().id));
            actions.extend(dungeon.monsters.iter().find(|monster| monster.side == Side::Ally).map(|ally| ItemAction::Give(ally.id)));
        }
        actions.push(ItemAction::PutDown);
        actions
    }

    /// What the player asks of the bag or of Kecleon. Whether the leader's
    /// turn was spent (`false` for anything else).
    pub(crate) fn take_item_input(&mut self, input: &Input) -> bool {
        if self.dungeon.is_none() {
            return false;
        }
        match *input {
            Input::Pay(yes) => self.pay(yes),
            Input::Sell(index) => self.sell(index),
            Input::LeaveShop => self.leave_shop(),
            Input::UseItem(index) | Input::ThrowItem(index) | Input::PutDown(index) | Input::Give { item: index, .. } => {
                if !self.bag_opens() || self.unstuck_item(index).is_none() {
                    return false;
                }
                return match *input {
                    Input::PutDown(_) => {
                        self.put_down(index);
                        false
                    }
                    Input::Give { to, .. } => {
                        self.give(index, to);
                        false
                    }
                    Input::ThrowItem(_) => self.thrown_or_used(index, true),
                    _ => self.thrown_or_used(index, false),
                };
            }
            _ => {}
        }
        false
    }

    /// The item at that place of the bag, unless it is sticky, which is
    /// said.
    fn unstuck_item(&mut self, index: usize) -> Option<u16> {
        let value = *self.bag.items.get(index)?;
        if stack::sticky(value) {
            self.log.push(format!("{} is sticky! It cannot be used.", self.book.item_name(stack::id(value))));
            return None;
        }
        Some(value)
    }

    /// Put down where the leader stands, if nothing lies there. Put back
    /// in Kecleon's shop, what was not paid for is for sale again. (It takes
    /// no turn.)
    fn put_down(&mut self, index: usize) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let (name, at) = (dungeon.leader().name.clone(), dungeon.leader().at);
        if dungeon.items.iter().any(|item| item.at == at) || dungeon.floor.stairs == at {
            self.log.push("There is already something here.".to_string());
            return;
        }
        let item = self.bag.items.remove(index);
        self.log.push(format!("{name} pose {}.", self.book.item_name(item)));
        let price = dungeon.shop.as_mut().and_then(|shop| Some(shop.owed.remove(shop.owed.iter().position(|(owed, _)| *owed == item)?).1));
        dungeon.items.push(super::dungeon::FloorItem { at, item, price });
        self.sights.push(Sight::BagDone);
    }

    /// One of the team holds the item; what it held goes back in the bag.
    /// (It takes no turn.)
    fn give(&mut self, index: usize, to: MonsterId) {
        let item = self.bag.items[index];
        let item_name = self.book.item_name(item);
        if !held::is_held_item(item) {
            self.log.push(format!("{item_name} cannot be held."));
            return;
        }
        let Some(holder) = self.dungeon.as_mut().and_then(|dungeon| dungeon.monster_mut(to)) else {
            return;
        };
        let who = holder.name.clone();
        if stack::sticky(holder.stats.held) {
            self.log.push(format!("{} is sticky: {who} cannot let go of it.", self.book.item_name(stack::id(holder.stats.held))));
            return;
        }
        self.bag.items.remove(index);
        if holder.stats.held != 0 {
            self.log.push(format!("{who} gave back {}.", self.book.item_name(holder.stats.held)));
            self.bag.items.push(holder.stats.held);
        }
        holder.stats.held = item;
        self.log.push(format!("{who} holds {item_name}."));
        self.sights.push(Sight::BagDone);
    }

    /// Throws the item, or uses it, whichever it is. Whether a turn went.
    fn thrown_or_used(&mut self, index: usize, throw: bool) -> bool {
        let Some(dungeon) = self.dungeon.as_ref() else {
            return false;
        };
        let leader = dungeon.leader();
        if leader.status.embargo_turns > 0 {
            self.log.push(format!("{} is under Embargo: no item can be used!", leader.name));
            return false;
        }
        let types = leader.stats.types;
        // (One item of a pile: see `dungeon::stack`.)
        let item = stack::id(self.bag.items[index]);
        if throw || (1..=kind::LAST_PROJECTILE).contains(&item) {
            self.throw(index, item);
            return true;
        }
        if item == kind::BLAST_SEED {
            self.bite_blast_seed(index, item);
            return true;
        }
        if let Some(orb) = Orb::of(item) {
            return self.use_orb(index, orb, item);
        }
        if (kind::FIRST_TM..=kind::LAST_TM).contains(&item) {
            return self.teach(index, item);
        }
        if self.book.tables.iq.gummi(types, item).is_some() {
            self.eat_gummi(index, item);
            return true;
        }
        self.eat(index, item)
    }

    /// The foes standing, with where they are.
    pub(crate) fn foes(&self) -> Vec<(MonsterId, Pos)> {
        let monsters = self.dungeon.iter().flat_map(|dungeon| &dungeon.monsters);
        monsters.filter(|monster| monster.side == Side::Foe && monster.alive()).map(|monster| (monster.id, monster.at)).collect()
    }

    /// Thrown from where the leader stands, the way it faces, up to ten
    /// tiles (or a wall), at the first foe in line, or all of them for a
    /// piercing throw. Rocks fly in an arc, over anyone in the way, to the
    /// foe they find (`arc_landing`).
    fn throw(&mut self, index: usize, item: u16) {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        let item_name = book.item_name(item);
        let foes = self.foes();
        let taken: Vec<Pos> = foes.iter().map(|foe| foe.1).collect();
        let Some(dungeon) = self.dungeon.as_ref() else {
            return;
        };
        let leader = dungeon.leader();
        let (me, here, facing, name) = (leader.id, leader.at, leader.facing, leader.name.clone());
        let status = &leader.status;
        let reach = if status.long_toss || status.exclusive.has(exclusive::effect::LONG_TOSS) { FLOOR_WIDTH } else { 10 };
        let pierces = status.piercing || iq::has(&leader.stats, iq::PIERCE_HURLER);
        let pitcher = if iq::has(&leader.stats, iq::POWER_PITCHER) { constants.power_pitcher } else { 0x100 };
        let mut flight = Vec::new();
        let mut at = here;
        for _ in 0..reach {
            if !dungeon.floor.can_step(at, facing.0, facing.1) {
                break;
            }
            at = Pos::new(at.x + facing.0, at.y + facing.1);
            flight.push(at);
        }
        let lobbed = kind::ROCKS.contains(&stack::id(item));
        if lobbed {
            at = arc_landing(&dungeon.floor, here, facing, &taken);
            flight = vec![at];
        }
        let mut hit: Vec<Pos> = flight.iter().copied().filter(|tile| taken.contains(tile)).collect();
        if !pierces {
            hit.truncate(1);
        }
        stack::take_one(&mut self.bag.items, index);
        self.sights.push(Sight::BagDone);
        self.log.push(format!("{name} threw {item_name}."));
        self.sights.push(Sight::Sound(THROW_SOUND));
        let Some(&target) = hit.last() else {
            // Nobody in the way: the item lands where its flight ends.
            if at == here {
                stack::add(&mut self.bag.items, item);
            } else {
                self.sights.push(Sight::Thrown(item, here, at));
                self.drop_item(at, item);
            }
            self.log.push(format!("{item_name} fell to the ground."));
            return;
        };
        self.sights.push(Sight::Thrown(item, here, target));
        let projectile = (1..=kind::LAST_PROJECTILE).contains(&item);
        // (Power Pitcher: thrown harder.)
        let power = constants.thrown_powers.get(item as usize).copied().unwrap_or(0) * pitcher / 0x100;
        let slot = MoveSlot { id: PROJECTILE_MOVE_ID, power, accuracy: constants.thrown_hit_chance, ..book.regular_attack() };
        let mut warped = None;
        for (foe, foe_at) in foes.into_iter().filter(|foe| hit.contains(&foe.1)) {
            if projectile && !kind::ROCKS.contains(&item) {
                let attack = Attack { slot, effect: None, damage_mult: FIXED_ONE, linked: false };
                self.strike(me, foe, &attack);
            } else if !self.thrown_hits(me, foe, &slot) {
                let foe_name = self.name_of(foe);
                self.log.push(format!("{item_name} missed {foe_name}."));
                // It falls short of its target.
                let short = flight
                    .iter()
                    .position(|&tile| tile == foe_at)
                    .and_then(|tile| tile.checked_sub(1))
                    .map_or(if lobbed { foe_at } else { here }, |tile| flight[tile]);
                self.drop_item(short, item);
            } else if projectile || item == kind::BLAST_SEED {
                // Rocks and Blast Seeds do a set amount of damage.
                self.scorch(foe, if projectile { slot.power } else { constants.blast_seed_damage.1 });
            } else if self.eaten_by_foe(foe, item) {
                warped = Some(foe);
            }
            self.say_if_out(foe);
        }
        if let Some(foe) = warped {
            self.warp(foe, &taken);
        }
    }

    /// Whether what is thrown reaches a foe.
    fn thrown_hits(&mut self, me: MonsterId, foe: MonsterId, slot: &MoveSlot) -> bool {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return false;
        };
        let (accuracy, female) = dungeon.monster(me).map_or((0, false), |monster| (monster.status.accuracy_stage, monster.stats.female));
        let (evasion, female_foe) = dungeon.monster(foe).map_or((0, false), |monster| (monster.status.evasion_stage, monster.stats.female));
        let weather = dungeon.weather.apparent();
        move_hits(&mut dungeon.rng, &self.book.tables.constants, slot, (accuracy, evasion), weather, (female, female_foe))
    }

    /// An item lies there, for nothing.
    fn drop_item(&mut self, at: Pos, item: u16) {
        if let Some(dungeon) = self.dungeon.as_mut() {
            dungeon.items.push(super::dungeon::FloorItem { at, item, price: None });
        }
    }

    pub(crate) fn name_of(&self, who: MonsterId) -> String {
        self.dungeon.as_ref().and_then(|dungeon| dungeon.monster(who)).map_or(String::new(), |monster| monster.name.clone())
    }

    /// A set amount of damage, the team's doing.
    fn scorch(&mut self, foe: MonsterId, amount: i32) {
        self.change(foe, |foe| {
            foe.stats.hp -= amount;
            foe.status.struck_by_team = true;
            foe.status.struck();
        });
        let name = self.name_of(foe);
        super::words::log_damage(&self.book, &mut self.log, &name, Damage { amount, matchup: matchup::NEUTRAL, critical: false });
    }

    /// Thrown, anything else has on its target the effect of eating it.
    /// Whether it is to warp.
    fn eaten_by_foe(&mut self, foe: MonsterId, item: u16) -> bool {
        let done = self.work(foe, |foe, rng, book, log| {
            let eaten = eat_item(item, rng, &book.tables.constants, &mut foe.stats, &mut foe.status, &mut Belly::full(), &mut []);
            match eaten {
                // (A Dough Seed works for the team only; a fussy food is
                // not eaten when thrown.)
                ItemUse::Nothing | ItemUse::MorePoke | ItemUse::Fussy(_) => log.push("But nothing happened.".to_string()),
                ItemUse::Warp => return true,
                ItemUse::LoseLevel if foe.stats.level > 1 => {
                    foe.stats.level -= 1;
                    log.push(format!("{} lost a level!", foe.name));
                }
                ItemUse::Done | ItemUse::Levels(_) | ItemUse::LoseLevel | ItemUse::ToStairs | ItemUse::SeeTraps => {
                    log.push(format!("{} took effect on {}.", book.item_name(item), foe.name));
                }
            }
            false
        });
        done == Some(true)
    }

    /// Sent somewhere else on the floor.
    pub(crate) fn warp(&mut self, who: MonsterId, taken: &[Pos]) {
        let name = self.name_of(who);
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        if let Some(tile) = dungeon.warp_tile(taken)
            && let Some(monster) = dungeon.monster_mut(who)
        {
            monster.at = tile;
            self.log.push(format!("{name} was warped!"));
        }
    }

    /// Eaten, a Blast Seed scorches the monster right in front.
    fn bite_blast_seed(&mut self, index: usize, item: u16) {
        self.spend(index, item, " used ");
        let front = self.front();
        let foe = self.foes().into_iter().find(|foe| Some(foe.1) == front);
        match foe {
            Some((foe, _)) => {
                self.scorch(foe, self.book.tables.constants.blast_seed_damage.0);
                self.say_if_out(foe);
            }
            None => self.log.push("But there is no target.".to_string()),
        }
    }

    /// The tile the leader faces.
    pub(crate) fn front(&self) -> Option<Pos> {
        let leader = self.dungeon.as_ref()?.leader();
        Some(Pos::new(leader.at.x + leader.facing.0, leader.at.y + leader.facing.1))
    }

    /// The item leaves the bag, told.
    fn spend(&mut self, index: usize, item: u16, verb: &str) {
        self.bag.items.remove(index);
        self.sights.push(Sight::BagDone);
        let name = self.dungeon.as_ref().map_or(String::new(), |dungeon| dungeon.leader().name.clone());
        self.log.push(format!("{name}{verb}{}.", self.book.item_name(item)));
    }

    /// A TM teaches its move to the leader, if its species can learn it,
    /// and leaves a Used TM; an HM stays.
    fn teach(&mut self, index: usize, item: u16) -> bool {
        let book = self.book.clone();
        let taught = book.item_move(item);
        let Some(dungeon) = self.dungeon.as_mut() else {
            return false;
        };
        let leader = dungeon.leader();
        let (me, name) = (leader.id, leader.name.clone());
        let move_name = book.move_name(taught);
        let Some(slot) = leader.experience.as_ref().and_then(|experience| experience.machines.iter().find(|slot| slot.id == taught).copied()) else {
            self.log.push(format!("{name} cannot learn {move_name}."));
            return false;
        };
        if leader.moves.iter().any(|known| known.id == taught) {
            self.log.push(format!("{name} already knows {move_name}."));
            return false;
        }
        if item < kind::FIRST_HM {
            self.bag.items[index] = stack::used_tm(item);
        }
        self.sights.push(Sight::BagDone);
        self.log.push(format!("{name} used {}.", book.item_name(item)));
        if leader.moves.len() < 4 {
            dungeon.leader_mut().moves.push(slot);
            self.log.push(format!("{name} learned {move_name}!"));
        } else {
            self.log.push(format!("{name} veut apprendre {move_name}."));
            dungeon.learning.push((me, slot));
        }
        true
    }

    /// A Gummi raises the IQ, and opens new IQ skills.
    fn eat_gummi(&mut self, index: usize, item: u16) {
        let Some((me, at)) = self.dungeon.as_ref().map(|dungeon| (dungeon.leader().id, dungeon.leader().at)) else {
            return;
        };
        self.spend(index, item, " ate ");
        self.sights.push(Sight::Item(item, at));
        self.work(me, |leader, _, book, log| {
            let Some((gain, belly)) = book.tables.iq.gummi(leader.stats.types, item) else {
                return;
            };
            if let Some(hunger) = &mut leader.belly {
                hunger.eat(belly);
            }
            let stats = &mut leader.stats;
            let before = stats.iq_skills;
            stats.iq = (stats.iq + gain).min(iq::MAX_IQ);
            stats.iq_skills = book.tables.iq.skills(stats.iq_group, stats.iq);
            log.push(format!("{}'s IQ rose by {gain}!", leader.name));
            for skill in iq::learnt(before, stats.iq_skills) {
                log.push(format!("{} learned {}!", leader.name, book.iq_skill_name(skill)));
            }
        });
    }

    /// Eaten by the leader. Whether it did anything (what does nothing is
    /// kept, and takes no turn).
    fn eat(&mut self, index: usize, item: u16) -> bool {
        let Some(me) = self.dungeon.as_ref().map(|dungeon| dungeon.leader().id) else {
            return false;
        };
        // (The locked doors of the rooms laid out by hand are not on the
        // floors yet: a Key finds none.)
        if item == kind::KEY {
            self.log.push("There is no door here to open.".to_string());
            return false;
        }
        let used = self.work(me, |leader, rng, book, _| {
            let Monster { stats, status, belly, moves, .. } = leader;
            let mut full = Belly::full();
            eat_item(item, rng, &book.tables.constants, stats, status, belly.as_mut().unwrap_or(&mut full), moves)
        });
        let used = used.unwrap_or(ItemUse::Nothing);
        if used == ItemUse::Nothing {
            self.log.push(format!("{}: no effect for now.", self.book.item_name(item)));
            return false;
        }
        let at = self.dungeon.as_ref().map_or(Pos::new(0, 0), |dungeon| dungeon.leader().at);
        self.spend(index, item, " used ");
        self.sights.push(Sight::Item(item, at));
        match used {
            ItemUse::Warp => {
                let taken: Vec<Pos> = self.foes().iter().map(|foe| foe.1).collect();
                self.warp(me, &taken);
            }
            ItemUse::Levels(levels) => self.gain_levels(me, levels),
            ItemUse::LoseLevel => {
                self.work(me, |leader, _, _, log| {
                    let lost = leader.experience.as_mut().is_some_and(|experience| experience.growth.lose_level(&mut leader.stats));
                    match lost {
                        true => log.push(format!("{} went down to level {}!", leader.name, leader.stats.level)),
                        false => log.push("But nothing happened.".to_string()),
                    }
                });
            }
            ItemUse::ToStairs => {
                if let Some(dungeon) = self.dungeon.as_mut() {
                    let stairs = dungeon.floor.stairs;
                    let leader = dungeon.leader_mut();
                    leader.at = stairs;
                    self.log.push(format!("{} was warped to the stairs!", leader.name));
                }
            }
            ItemUse::SeeTraps => self.see_traps(),
            ItemUse::MorePoke => {
                if let Some(dungeon) = self.dungeon.as_mut() {
                    dungeon.dough_seed = true;
                }
                self.log.push("More Poké will turn up on the next floor!".to_string());
            }
            ItemUse::Fussy(item) => self.eat_fussy(me, item),
            ItemUse::Done | ItemUse::Nothing => {}
        }
        true
    }

    /// A food for some species only: a Mix Elixir gives a Linoone back all
    /// its PP, a Gravelyrock raises a Sudowoodo's or a Bonsly's IQ (as much
    /// as a Wonder Gummi: this project's own figure). Anyone else, nothing.
    fn eat_fussy(&mut self, who: MonsterId, item: u16) {
        self.work(who, |monster, _, book, log| {
            let fits = monster.species.is_some_and(|species| fussy_eaters(item).contains(&(species % 600)));
            if !fits {
                log.push("But nothing happened.".to_string());
                return;
            }
            if item == kind::MIX_ELIXIR {
                monster.moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp);
                log.push(format!("{}'s PP were restored!", monster.name));
                return;
            }
            let stats = &mut monster.stats;
            let (before, gain) = (stats.iq_skills, book.tables.constants.wonder_gummi_iq);
            stats.iq = (stats.iq + gain).min(iq::MAX_IQ);
            stats.iq_skills = book.tables.iq.skills(stats.iq_group, stats.iq);
            log.push(format!("{}'s IQ rose by {gain}!", monster.name));
            for skill in iq::learnt(before, stats.iq_skills) {
                log.push(format!("{} learned {}!", monster.name, book.iq_skill_name(skill)));
            }
        });
    }

    fn gain_levels(&mut self, who: MonsterId, levels: i32) {
        let asked = self.work(who, |monster, _, book, log| {
            let mut asked = Vec::new();
            for _ in 0..levels {
                let Some(experience) = monster.experience.as_mut() else {
                    break;
                };
                let Some(missing) = experience.growth.to_next(monster.stats.level) else {
                    break;
                };
                asked.extend(earn(book, log, &monster.name, &mut monster.stats, &mut monster.moves, experience, missing));
            }
            asked
        });
        if let Some(dungeon) = self.dungeon.as_mut() {
            dungeon.learning.extend(asked.unwrap_or_default().into_iter().map(|slot| (who, slot)));
        }
    }

    /// The floor's traps show.
    pub(crate) fn see_traps(&mut self) {
        if let Some(dungeon) = self.dungeon.as_mut() {
            dungeon.traps.iter_mut().for_each(|trap| trap.seen = true);
        }
        self.log.push("The floor's traps appear!".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dungeon::Tile;

    #[test]
    fn rocks_land_on_a_foe_of_the_cone_or_two_tiles_ahead() {
        let mut floor = Floor::new_walled();
        for y in 5..20 {
            for x in 5..20 {
                floor.set(Pos::new(x, y), Tile::Floor { room: Some(0) });
            }
        }
        let from = Pos::new(10, 10);
        // Over the one out of the cone, to the one in it.
        assert_eq!(arc_landing(&floor, from, (1, 0), &[Pos::new(10, 12), Pos::new(14, 12)]), Pos::new(14, 12));
        // The nearest row first.
        assert_eq!(arc_landing(&floor, from, (1, 0), &[Pos::new(15, 10), Pos::new(13, 11)]), Pos::new(13, 11));
        // Nobody: two tiles ahead, or short of a wall.
        assert_eq!(arc_landing(&floor, from, (0, 1), &[]), Pos::new(10, 12));
        assert_eq!(arc_landing(&floor, Pos::new(18, 10), (1, 0), &[]), Pos::new(19, 10));
        // On a diagonal, a square before the thrower.
        assert_eq!(arc_landing(&floor, from, (1, 1), &[Pos::new(13, 11)]), Pos::new(13, 11));
    }
}
