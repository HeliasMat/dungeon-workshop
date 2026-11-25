//! What comes after the actions of a frame: what moves left to do, and
//! what the monsters' state brings. `Run::after_actions` runs it all, in a
//! fixed order, once a frame.

use crate::dungeon::exclusive::{self, Exclusive};
use crate::dungeon::{FLOOR_HEIGHT, FLOOR_WIDTH, Floor, MoveEffect, Pos, Tile, ability, ai, explosion_damage, range, stack};

use super::attack::{Arena, chebyshev};
use super::dungeon::{FloorItem, Outcome, Shove};
use super::monster::{Identity, Side};
use super::watch::watched;
use super::words::effect_message;
use super::{Monster, Run, Sight};

/// Castform and Cherrim, by the weather: their form's entry (the female
/// entries are 600 further).
fn weather_form(species: u16, weather: u8) -> Option<u16> {
    use crate::dungeon::weather;
    const CASTFORM: u16 = 379;
    const CHERRIM: u16 = 460;
    let base = species % 600;
    let shift = match (base, weather) {
        (CASTFORM, weather::SUNNY) => 1,
        (CASTFORM, weather::RAIN) => 2,
        (CASTFORM, weather::HAIL | weather::SNOW) => 3,
        (CHERRIM, weather::SUNNY) => 1,
        _ => return None,
    };
    Some(species + shift)
}

/// Of that side? (Kecleon keeping its shop is of none.)
fn of_side(monster: &Monster, team: bool) -> bool {
    monster.side != Side::Keeper && monster.team() == team
}

impl Run {
    /// All that follows the actions. The order is written here.
    pub(crate) fn after_actions(&mut self) {
        let Some(mut arena) = self.arena() else {
            return;
        };
        let new_floor = arena.dungeon.identity_floor != arena.dungeon.generation;
        arena.dungeon.identity_floor = arena.dungeon.generation;
        arena.spread_effects();
        arena.pass_effects();
        arena.synchronize();
        arena.blasts();
        arena.shoves();
        arena.spite_moves();
        arena.learn_sketched();
        arena.keep_identity(new_floor);
        arena.destiny_bonds();
        arena.show_disguise();
        arena.leech_seed_heals();
        arena.knock_out();
        arena.unseal_moves(new_floor);
    }

    /// The Monster House wakes up when the leader walks into it.
    pub(crate) fn wake_monster_house(&mut self) {
        if let Some(mut arena) = self.arena() {
            arena.wake_monster_house();
        }
    }
}

impl Arena<'_> {
    /// The teammates a move on the user's side reaches: in its room (around
    /// it in a hallway) or on the whole floor.
    fn spread_effects(&mut self) {
        let book = self.book;
        let constants = &book.tables.constants;
        for spread in std::mem::take(&mut self.dungeon.spread) {
            let dungeon = &mut *self.dungeon;
            let floor = &dungeon.floor;
            let reaches = |at: Pos| match (spread.range, floor.tile(spread.from)) {
                (range::FLOOR, _) => true,
                (_, Tile::Floor { room: Some(room) }) => floor.tile(at) == (Tile::Floor { room: Some(room) }) || chebyshev(spread.from, at) <= 1,
                _ => chebyshev(spread.from, at) <= 1,
            };
            for monster in &mut dungeon.monsters {
                if monster.at == spread.from || !of_side(monster, spread.team) || !monster.alive() || !reaches(monster.at) {
                    continue;
                }
                let rng = &mut dungeon.rng;
                watched(monster, self.sights, |monster| {
                    let applied = crate::dungeon::apply_effect(spread.effect, rng, constants, &mut monster.stats, &mut monster.status);
                    if let MoveEffect::HealingWish { pp: true } = spread.effect {
                        monster.moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp);
                    }
                    self.log.push(effect_message(book, spread.effect, &monster.name, applied));
                });
            }
        }
    }

    /// What Pass Scarves sent on: to someone next to the holder, at random
    /// (no one there, it is gone). The holder's belly pays for it.
    fn pass_effects(&mut self) {
        const PASS_SCARF_BELLY: i32 = 5;
        let book = self.book;
        let constants = &book.tables.constants;
        for (from, effect) in std::mem::take(&mut self.dungeon.passed) {
            let dungeon = &mut *self.dungeon;
            if let Some(holder) = dungeon.monsters.iter_mut().find(|monster| monster.at == from && monster.alive())
                && let Some(belly) = &mut holder.belly
            {
                // (This project's own figure.)
                belly.lose(PASS_SCARF_BELLY);
            }
            let around: Vec<usize> = (0..dungeon.monsters.len())
                .filter(|&at| dungeon.monsters[at].alive() && chebyshev(dungeon.monsters[at].at, from) == 1 && dungeon.monsters[at].side != Side::Keeper)
                .collect();
            if around.is_empty() {
                continue;
            }
            let pick = around[dungeon.rng.rand_int(around.len() as i32) as usize];
            let rng = &mut dungeon.rng;
            watched(&mut dungeon.monsters[pick], self.sights, |monster| {
                let applied = crate::dungeon::apply_effect(effect, rng, constants, &mut monster.stats, &mut monster.status);
                self.log.push(effect_message(book, effect, &monster.name, applied));
            });
        }
    }

    /// What Synchronize shares: the status problem its bearer was given
    /// goes to each foe of its next to it.
    fn synchronize(&mut self) {
        let book = self.book;
        let constants = &book.tables.constants;
        for (from, team, effect) in std::mem::take(&mut self.dungeon.synchronized) {
            let dungeon = &mut *self.dungeon;
            let rng = &mut dungeon.rng;
            for monster in dungeon.monsters.iter_mut().filter(|monster| monster.alive() && chebyshev(monster.at, from) == 1 && of_side(monster, !team)) {
                watched(monster, self.sights, |monster| {
                    let applied = crate::dungeon::apply_effect(effect, rng, constants, &mut monster.stats, &mut monster.status);
                    self.log.push(effect_message(book, effect, &monster.name, applied));
                });
            }
        }
    }

    /// Blasts of moves (Selfdestruct, Explosion): the walls around break,
    /// the monsters caught are hurt as by an Explosion Trap, the items lying
    /// there are gone; Rock Smash breaks its one wall.
    fn blasts(&mut self) {
        let constants = &self.book.tables.constants;
        for (centre, radius, spared) in std::mem::take(&mut self.dungeon.blasts) {
            let dungeon = &mut *self.dungeon;
            let caught = |at: Pos| (at.x - centre.x).abs() <= radius && (at.y - centre.y).abs() <= radius;
            for tile in Floor::positions().filter(|&tile| caught(tile)) {
                let inside = tile.x > 0 && tile.y > 0 && tile.x < FLOOR_WIDTH - 1 && tile.y < FLOOR_HEIGHT - 1;
                if inside && dungeon.floor.tile(tile) == Tile::Wall {
                    dungeon.floor.set(tile, Tile::Floor { room: None });
                }
            }
            dungeon.repaints += 1;
            self.sights.push(Sight::TilesChanged);
            if radius == 0 {
                continue;
            }
            if dungeon.weather.apparent() == crate::dungeon::weather::RAIN {
                self.log.push("The explosion fizzled out.".to_string());
                continue;
            }
            for monster in &mut dungeon.monsters {
                if Some(monster.at) == spared || !monster.alive() || !caught(monster.at) {
                    continue;
                }
                // (Exclusive items halve it, or ward it off.)
                use exclusive::effect;
                let held = monster.status.exclusive;
                let full = explosion_damage(constants, radius, &monster.stats, monster.team());
                let damage = match (held.has(effect::NO_EXPLOSION_DAMAGE), held.has(effect::HALVED_EXPLOSION_DAMAGE)) {
                    (true, _) => 0,
                    (false, true) => full / 2,
                    _ => full,
                };
                watched(monster, self.sights, |monster| monster.stats.hp -= damage);
                self.log.push(format!("{} lost {damage} HP.", monster.name));
            }
            dungeon.items.retain(|item| !caught(item.at));
        }
    }

    /// The tile an item falls on: its own, or the nearest free one around.
    fn landing(&self, at: Pos) -> Option<Pos> {
        let floor = &self.dungeon.floor;
        let lying = |tile: Pos| self.dungeon.items.iter().any(|item| item.at == tile);
        std::iter::once(at)
            .chain(ai::DIRECTIONS.iter().map(|&(dx, dy)| Pos::new(at.x + dx, at.y + dy)))
            .find(|&tile| floor.tile(tile).is_walkable() && !lying(tile) && tile != floor.stairs)
    }

    /// Strength and Fling hurl the monster in front at another of its side,
    /// as the Hurl Orb does; Knock Off drops what it holds beside it; Beat
    /// Up brings the user's teammates around it; Baton Pass swaps places.
    fn shoves(&mut self) {
        // What moves drop.
        for (at, item) in std::mem::take(&mut self.dungeon.drops) {
            if let Some(spot) = self.landing(at) {
                self.dungeon.items.push(FloorItem { at: spot, item, price: None });
                self.log.push(format!("{} fell to the ground!", self.book.item_name(item)));
            }
        }
        for shove in std::mem::take(&mut self.dungeon.shoves) {
            match shove.effect {
                MoveEffect::Hurl => self.hurl(shove),
                MoveEffect::KnockOff => self.knock_off(shove),
                MoveEffect::SwitchPlaces => self.switch_places(shove),
                MoveEffect::BeatUp => self.beat_up(shove),
                _ => {}
            }
        }
    }

    /// Those standing: where, and whether of the team.
    fn standing(&self) -> Vec<(Pos, bool)> {
        self.dungeon.monsters.iter().filter(|monster| monster.alive()).map(|monster| (monster.at, monster.team())).collect()
    }

    fn hurl(&mut self, shove: Shove) {
        let front = Pos::new(shove.from.x + shove.facing.0, shove.from.y + shove.facing.1);
        let taken = self.standing();
        let dungeon = &mut *self.dungeon;
        let floor = &dungeon.floor;
        let free = |tile: Pos| floor.tile(tile).is_walkable() && !taken.iter().any(|(at, _)| *at == tile);
        let Some(&(_, side)) = taken.iter().find(|(at, _)| *at == front) else {
            self.log.push("But there is nobody in front.".to_string());
            return;
        };
        // (Suction Cups: it stays where it is.)
        if let Some(anchored) =
            dungeon.monsters.iter().find(|monster| monster.at == front && monster.alive() && ability::has(&monster.stats, ability::SUCTION_CUPS))
        {
            self.log.push(format!("{} is anchored! It can't be thrown!", anchored.name));
            return;
        }
        const REACH: i32 = 10;
        let aimed = taken
            .iter()
            .filter(|(at, team)| *team == side && *at != front && chebyshev(front, *at) <= REACH && ai::can_see(floor, shove.from, *at))
            .map(|(at, _)| *at)
            .min_by_key(|&at| chebyshev(front, at));
        let landing = match aimed {
            Some(other) => ai::DIRECTIONS
                .iter()
                .map(|&(dx, dy)| Pos::new(other.x + dx, other.y + dy))
                .filter(|&tile| free(tile))
                .min_by_key(|&tile| chebyshev(front, tile)),
            None => {
                let near: Vec<Pos> = Floor::positions()
                    .filter(|&tile| (2..=4).contains(&chebyshev(front, tile)) && free(tile) && ai::can_see(floor, shove.from, tile))
                    .collect();
                (!near.is_empty()).then(|| near[dungeon.rng.rand_int(near.len() as i32) as usize])
            }
        };
        let Some(landing) = landing else {
            self.log.push("But nothing happened.".to_string());
            return;
        };
        let damage = self.book.tables.constants.blowback_damage;
        for monster in &mut dungeon.monsters {
            if !monster.alive() || (monster.at != front && Some(monster.at) != aimed) {
                continue;
            }
            match monster.at == front {
                true => {
                    monster.at = landing;
                    self.log.push(format!("{} was knocked flying!", monster.name));
                }
                false => self.log.push(format!("{} was struck!", monster.name)),
            }
            if aimed.is_some() {
                watched(monster, self.sights, |monster| monster.stats.hp -= damage);
                self.log.push(format!("{} lost {damage} HP.", monster.name));
            }
        }
    }

    fn knock_off(&mut self, shove: Shove) {
        let front = Pos::new(shove.from.x + shove.facing.0, shove.from.y + shove.facing.1);
        let spot = self.landing(front);
        let Some(monster) = self.dungeon.monsters.iter_mut().find(|monster| monster.at == front && monster.alive()) else {
            self.log.push("But there is nobody in front.".to_string());
            return;
        };
        let sticky = ability::has(&monster.stats, ability::STICKY_HOLD);
        match (if sticky { 0 } else { monster.stats.held }, spot) {
            (0, _) | (_, None) => self.log.push("But it failed!".to_string()),
            (held, Some(spot)) => {
                monster.stats.held = 0;
                self.log.push(format!("{} dropped {}!", monster.name, self.book.item_name(held)));
                self.dungeon.items.push(FloorItem { at: spot, item: held, price: None });
            }
        }
    }

    /// Baton Pass: the user hands over its spot to the nearest of its own
    /// side it can see (in its room, or next to it), and takes theirs.
    fn switch_places(&mut self, shove: Shove) {
        let floor = &self.dungeon.floor;
        let room = |at: Pos| match floor.tile(at) {
            Tile::Floor { room: Some(room) } => Some(room),
            _ => None,
        };
        let together = |at: Pos| chebyshev(at, shove.from) <= 1 || (room(at).is_some() && room(at) == room(shove.from));
        let partner = self
            .dungeon
            .monsters
            .iter()
            .filter(|monster| monster.alive() && monster.at != shove.from && of_side(monster, shove.team) && together(monster.at))
            .map(|monster| monster.at)
            .min_by_key(|&at| chebyshev(at, shove.from));
        let Some(there) = partner else {
            self.log.push("But there is nobody.".to_string());
            return;
        };
        for monster in self.dungeon.monsters.iter_mut().filter(|monster| monster.alive()) {
            if monster.at == there {
                monster.at = shove.from;
            } else if monster.at == shove.from {
                monster.at = there;
            }
        }
        self.log.push("Places were swapped!".to_string());
    }

    fn beat_up(&mut self, shove: Shove) {
        let taken = self.standing();
        let floor = &self.dungeon.floor;
        let free = |tile: Pos| floor.tile(tile).is_walkable() && !taken.iter().any(|(at, _)| *at == tile);
        let mut around: Vec<Pos> = ai::DIRECTIONS.iter().map(|&(dx, dy)| Pos::new(shove.from.x + dx, shove.from.y + dy)).filter(|&tile| free(tile)).collect();
        let mut came = 0;
        for monster in &mut self.dungeon.monsters {
            if !monster.alive() || monster.at == shove.from || !of_side(monster, shove.team) || chebyshev(monster.at, shove.from) <= 1 {
                continue;
            }
            let Some(tile) = around.pop() else {
                break;
            };
            monster.at = tile;
            came += 1;
        }
        self.log.push(match came {
            0 => "But nobody came.".to_string(),
            _ => "The team rushes over!".to_string(),
        });
    }

    /// Spite and Grudge: the last move the monster used loses all its PP.
    /// Recycle: the bag's Used TMs are TMs again.
    fn spite_moves(&mut self) {
        if std::mem::take(&mut self.dungeon.recycle) {
            let mut renewed = 0;
            for item in &mut self.bag.items {
                if let Some(tm) = stack::recycled(*item) {
                    *item = tm;
                    renewed += 1;
                }
            }
            self.log.push(match renewed {
                0 => "But nothing happened.".to_string(),
                _ => "Used TMs became new again!".to_string(),
            });
        }
        for monster in &mut self.dungeon.monsters {
            if !std::mem::take(&mut monster.status.spited) {
                continue;
            }
            let last = monster.status.last_move;
            if let Some(slot) = monster.moves.iter_mut().find(|slot| Some(slot.id) == last) {
                slot.pp = 0;
                self.log.push(format!("{} of {} has no PP left!", self.book.move_name(slot.id), monster.name));
            }
        }
    }

    /// Sketch: the move sketched takes Sketch's place.
    fn learn_sketched(&mut self) {
        const SKETCH: u16 = 138;
        for monster in &mut self.dungeon.monsters {
            let Some(id) = monster.status.sketched.take() else {
                continue;
            };
            let (Some(learnt), Some(slot)) = (self.book.move_slot(id), monster.moves.iter_mut().find(|slot| slot.id == SKETCH)) else {
                continue;
            };
            *slot = learnt;
            self.log.push(format!("{} learned {}!", monster.name, self.book.move_name(id)));
        }
    }

    /// What monsters are by what they are made of: the weather they calm,
    /// their weight, what the bag's exclusive items do for the team; and the
    /// abilities and types moves changed, which come back as the floor is
    /// left (`Statuses::identity`, kept in `Monster::identity` past the
    /// statuses' own reset).
    fn keep_identity(&mut self, new_floor: bool) {
        let book = self.book;
        let dungeon = &mut *self.dungeon;
        let left = new_floor || dungeon.outcome.is_some();
        let items: Vec<u16> = self.bag.items.iter().map(|&item| stack::id(item)).filter(|&item| item >= exclusive::FIRST).collect();
        // Cloud Nine or Air Lock on the floor: the weather does nothing.
        let any = |which: u8| dungeon.monsters.iter().any(|monster| monster.alive() && ability::has(&monster.stats, which));
        dungeon.weather.suppressed = any(ability::CLOUD_NINE) || any(ability::AIR_LOCK);
        dungeon.weather.damp = any(ability::DAMP);
        let playing = dungeon.outcome.is_none();
        for monster in &mut dungeon.monsters {
            // A new floor: exclusive items may restore the HP or the PP.
            if left && monster.team() && playing {
                use exclusive::effect;
                if monster.status.exclusive.has(effect::RESTORE_HP_ON_NEW_FLOORS) {
                    monster.stats.hp = monster.stats.max_hp;
                }
                if monster.status.exclusive.has(effect::RESTORE_PP_ON_NEW_FLOORS) {
                    monster.moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp);
                }
            }
            // (Its species' weight, for Low Kick and Grass Knot.)
            if let Some(species) = monster.species {
                monster.status.weight = book.weight(species);
                monster.status.exclusive = match monster.team() {
                    true => Exclusive::of(&book.tables.exclusive, items.iter().copied(), species, book.family(species), monster.stats.types),
                    false => Default::default(),
                };
            }
            match (monster.identity, left) {
                (Some(identity), true) => {
                    (monster.stats.abilities, monster.stats.types) = (identity.abilities, identity.types);
                    monster.status.identity = None;
                    monster.identity = None;
                }
                (None, false) => {
                    if let Some((abilities, types)) = monster.status.identity {
                        monster.identity = Some(Identity { abilities, types });
                    }
                }
                _ => {}
            }
        }
    }

    /// Destiny Bond: what the bound user loses, its target loses too, while
    /// it lasts. (The HP of the bound are kept from a frame to the next: it
    /// answers all that hurts them.)
    fn destiny_bonds(&mut self) {
        let known = std::mem::take(&mut self.dungeon.bond_hp);
        let mut seen = Vec::new();
        let mut lost: Vec<(u32, i32)> = Vec::new();
        for monster in &mut self.dungeon.monsters {
            if monster.status.bond.is_some() && monster.status.bond_turns == 0 {
                monster.status.bond = None;
            }
            let Some((token, user)) = monster.status.bond else {
                continue;
            };
            let hp = monster.stats.hp;
            seen.push((monster.id, hp));
            let before = known.iter().find(|(id, _)| *id == monster.id).map(|(_, hp)| *hp);
            if let (true, Some(before)) = (user, before)
                && hp < before
            {
                lost.push((token, before - hp));
            }
        }
        for (token, amount) in lost {
            for monster in &mut self.dungeon.monsters {
                if monster.status.bond == Some((token, false)) && monster.alive() {
                    watched(monster, self.sights, |monster| monster.stats.hp -= amount);
                    self.log.push(format!("Destiny strikes {}: it lost {amount} HP.", monster.name));
                    if let Some(entry) = seen.iter_mut().find(|(id, _)| *id == monster.id) {
                        entry.1 = monster.stats.hp;
                    }
                }
            }
        }
        self.dungeon.bond_hp = seen;
    }

    /// Transform: the transformed look like the floor's monster they drew,
    /// until it wears off (or the floor is left). Forecast turns Castform
    /// to the weather's type and form; Cherrim blooms in the sun.
    fn show_disguise(&mut self) {
        let spec = self.dungeon.spec(self.book);
        let weather = self.dungeon.weather.apparent();
        for monster in &mut self.dungeon.monsters {
            if monster.status.disguise.is_some() && monster.status.disguise_turns == 0 {
                monster.status.disguise = None;
            }
            monster.looks_like = monster.status.disguise.and_then(|index| Some(spec?.enemies.get(index as usize)?.species));
            if ability::has(&monster.stats, ability::FORECAST) {
                let own = monster.status.identity.map_or(monster.stats.types, |(_, types)| types);
                let kind = ability::forecast(weather);
                let types = kind.map_or(own, |kind| (kind, 0));
                if monster.stats.types != types {
                    monster.status.remember_identity(&monster.stats);
                    monster.stats.types = types;
                }
            }
            if monster.looks_like.is_none() {
                monster.looks_like = monster.species.and_then(|species| weather_form(species, weather));
            }
        }
    }

    /// What Leech Seed drains goes to its sower, if it still stands.
    fn leech_seed_heals(&mut self) {
        for seeded in 0..self.dungeon.monsters.len() {
            let monster = &mut self.dungeon.monsters[seeded];
            let amount = std::mem::take(&mut monster.status.leech_drained);
            if amount <= 0 {
                continue;
            }
            let by = monster.status.leech_sower;
            let sower = self.dungeon.monsters.iter().position(|monster| Some(monster.id.0) == by && monster.alive());
            if let Some(sower) = sower {
                self.watch(sower, |monster| monster.stats.hp = (monster.stats.hp + amount).min(monster.stats.max_hp));
                self.log.push(format!("{} drained {amount} HP.", self.dungeon.monsters[sower].name));
            }
        }
    }

    /// The team down: the run ends once the last blow has been seen.
    fn knock_out(&mut self) {
        if let Some(left) = &mut self.dungeon.knocked_out {
            *left = if self.unseen { 0 } else { left.saturating_sub(1) };
            if *left == 0 {
                self.dungeon.knocked_out = None;
                self.dungeon.outcome = Some(Outcome::Fainted);
            }
        }
    }

    /// Sealed moves are usable again on a new floor.
    fn unseal_moves(&mut self, new_floor: bool) {
        if new_floor && let Some(leader) = self.dungeon.monsters.iter_mut().find(|monster| monster.side == Side::Leader) {
            leader.moves.iter_mut().for_each(|slot| slot.sealed = false);
        }
    }

    /// The Monster House wakes up when the leader walks into it.
    fn wake_monster_house(&mut self) {
        let leader = self.dungeon.monsters.iter().find(|monster| monster.side == Side::Leader).map(|leader| leader.at);
        if !leader.is_some_and(|at| self.dungeon.floor.in_monster_house(at)) || !self.dungeon.monsters.iter().any(|monster| monster.dormant) {
            return;
        }
        for monster in self.dungeon.monsters.iter_mut().filter(|monster| monster.dormant) {
            monster.status.sleep_turns = 0;
            monster.dormant = false;
        }
        self.log.push("It's a Monster House!".to_string());
    }
}
