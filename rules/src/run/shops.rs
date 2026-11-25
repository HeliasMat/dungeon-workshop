//! Kecleon's shops, the thief and its hunters, what is picked up, recruits
//! joining, and the questions the dungeon asks.

use crate::dungeon::{DungeonRng, MoveSlot, Pos, Stats, Statuses, ai, iq, money_amount, stack, status};
use crate::services::BAG_CAPACITY;

use super::book::ENEMY_STATS;
use super::dungeon::{Ask, FloorItem, Outcome, Shop, ShopTalk};
use super::items::{MONEY_SOUND, PICK_UP_SOUND};
use super::monster::Side;
use super::spawning::{KECLEON, hunter_stats, spawn_enemy, spawn_thief_hunter};
use super::team::{Experience, Recruit, known_moves};
use super::{Book, Monster, MonsterId, Run, Sight};

/// Haggler: Kecleon sells for a quarter less, and buys for a quarter more,
/// from a leader who has it (this project's own figures).
fn haggled(leader: &Monster, price: i32, buying: bool) -> i32 {
    match (iq::has(&leader.stats, iq::HAGGLER), buying) {
        (false, _) => price,
        (true, true) => (price * 3 / 4).max(1),
        (true, false) => price * 5 / 4,
    }
}

/// The songs of a Kecleon shop and of its alarm (`KecleonsShop`, `Outlaw`),
/// and how many more Kecleons the alarm brings.
pub(crate) const KECLEON_SHOP_SONG: u16 = 12;
pub(crate) const THIEF_SONG: u16 = 13;
const THIEF_HUNTERS: usize = 3;

/// The chance out of 100 that a tile of a Kecleon shop has an item, by its
/// place in a 3x3 pattern (the patterns from 6 on are the sixth's).
fn shop_chance(pattern: u8, x: i32, y: i32) -> i32 {
    const SURE: [[(i32, i32); 2]; 7] =
        [[(0, 1), (1, 1)], [(0, 1), (1, 2)], [(0, 1), (2, 2)], [(0, 0), (2, 1)], [(0, 0), (2, 0)], [(2, 1), (0, 2)], [(0, 1), (2, 1)]];
    let at = (x.rem_euclid(3), y.rem_euclid(3));
    if SURE[(pattern as usize).min(6)].contains(&at) { 100 } else { 30 }
}

/// Thrown items appear in piles, of as many as the item may hold
/// (`InitItem`).
pub(crate) fn piled(book: &Book, item: u16, rng: &mut DungeonRng) -> u16 {
    match stack::piles(item) {
        true => {
            let (fewest, most) = book.item_pile(item);
            stack::pile(item, fewest + rng.rand_int((most - fewest + 1) as i32) as u16)
        }
        false => item,
    }
}

impl Run {
    /// The floor's Kecleon shop, if it has one: Kecleon in its middle, an
    /// item for sale on each of its other tiles.
    pub(crate) fn place_shop(&mut self) {
        let book = self.book.clone();
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        dungeon.shop = None;
        let (Some(spec), Some(area)) = (dungeon.spec(&book), dungeon.floor.shop) else {
            return;
        };
        let keeper = Pos::new((area.x0 + area.x1) / 2, (area.y0 + area.y1) / 2);
        let name = Some(book.species_name(KECLEON)).filter(|name| !name.is_empty()).unwrap_or("The shopkeeper".to_string());
        let id = dungeon.new_id();
        dungeon.monsters.push(Monster::new(id, Side::Keeper, name, Some(KECLEON), ENEMY_STATS, Vec::new(), Statuses::default(), keeper));
        dungeon.shop = Some(Shop { keeper: Some(keeper), ..Shop::default() });
        for y in area.y0..=area.y1 {
            for x in area.x0..=area.x1 {
                let at = Pos::new(x, y);
                if at == keeper || !dungeon.floor.tile(at).is_walkable() {
                    continue;
                }
                // Some tiles always have an item, the others 30 % of the
                // time, after a 3x3 pattern of the floor's.
                if dungeon.rng.rand_int(100) >= shop_chance(spec.shop_pattern, x - area.x0, y - area.y0) {
                    continue;
                }
                let Some(item) = spec.shop_items.pick(&mut dungeon.rng) else {
                    continue;
                };
                let item = piled(&book, item, &mut dungeon.rng);
                dungeon.items.push(FloorItem { at, item, price: Some(book.item_prices(item).0.max(1)) });
            }
        }
    }

    /// The shop watching the leader: greeted on coming in; leaving it with
    /// what was not paid for makes Kecleon a foe (the original's thief
    /// alarm).
    pub(crate) fn shop_step(&mut self) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let Some(leader) = dungeon.monsters.iter().find(|monster| monster.side == Side::Leader) else {
            return;
        };
        let (here, level) = (leader.at, leader.stats.level);
        let in_shop = dungeon.floor.in_shop(here);
        let song = dungeon.song;
        let Some(shop) = dungeon.shop.as_mut() else {
            return;
        };
        if shop.keeper.is_none() {
            return;
        }
        if in_shop && !shop.welcomed {
            shop.welcomed = true;
            self.log.push("The shopkeeper: Welcome to my shop! Pick up whatever you like, then come and pay me.".to_string());
        }
        // The shop's own song plays inside it, as in the original.
        if in_shop != shop.inside {
            shop.inside = in_shop;
            self.sights.push(Sight::Music(if in_shop { Some(KECLEON_SHOP_SONG) } else { song }));
        }
        if in_shop || shop.owed.is_empty() {
            return;
        }
        shop.keeper = None;
        shop.owed.clear();
        dungeon.thief_alert = true;
        self.log.push("The shopkeeper: Thief! You won't get away with this!".to_string());
        for keeper in dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Keeper && monster.job_target.is_none()) {
            spawn_thief_hunter(keeper, level);
        }
        // The alarm: the thieves' song, and Kecleons all around the leader.
        self.sights.push(Sight::Music(Some(THIEF_SONG)));
        let taken: Vec<Pos> = dungeon.monsters.iter().filter(|monster| monster.alive()).map(|monster| monster.at).collect();
        let around = ai::DIRECTIONS.iter().map(|&(dx, dy)| Pos::new(here.x + dx * 2, here.y + dy * 2));
        let free: Vec<Pos> = around.filter(|&tile| dungeon.floor.tile(tile).is_walkable() && !taken.contains(&tile)).take(THIEF_HUNTERS).collect();
        let name = self.book.species_name(KECLEON);
        for at in free {
            let id = dungeon.new_id();
            dungeon.monsters.push(Monster::new(id, Side::Foe, name.clone(), Some(KECLEON), hunter_stats(level), Vec::new(), Statuses::default(), at));
        }
    }

    fn talk(&mut self) -> Option<&mut Shop> {
        self.dungeon.as_mut()?.shop.as_mut().filter(|shop| shop.talk.is_some())
    }

    /// The team pays for what it picked up, or says no.
    pub(crate) fn pay(&mut self, yes: bool) {
        let money = self.bag.money;
        let Some(shop) = self.talk().filter(|shop| shop.talk == Some(ShopTalk::Pay)) else {
            return;
        };
        let total: i32 = shop.owed.iter().map(|(_, price)| price).sum();
        shop.talk = None;
        if yes && money >= total {
            shop.owed.clear();
            self.bag.money -= total;
        }
        self.log.push(match (yes, money < total) {
            (false, _) => "The shopkeeper: Put back what you don't want, then.".to_string(),
            (true, true) => format!("The shopkeeper: You're short of Poké! That comes to {total} Poké."),
            (true, false) => format!("The team paid {total} Poké. The shopkeeper: Thank you very much!"),
        });
    }

    /// Kecleon buys the item at that place of the bag.
    pub(crate) fn sell(&mut self, index: usize) {
        let Some(&item) = self.bag.items.get(index) else {
            return self.leave_shop();
        };
        let book = self.book.clone();
        let Some(shop) = self.talk().filter(|shop| shop.talk == Some(ShopTalk::Sell)) else {
            return;
        };
        // (What was not paid for yet cannot be sold back.)
        if let Some(at) = shop.owed.iter().position(|(owed, _)| *owed == item) {
            shop.owed.remove(at);
            self.bag.items.remove(index);
            self.log.push(format!("The shopkeeper took back {}.", book.item_name(item)));
            return;
        }
        let price = book.item_prices(item).1;
        if price <= 0 {
            self.log.push(format!("The shopkeeper: I can't buy {}.", book.item_name(item)));
            return;
        }
        let price = haggled(self.here().leader(), price, false);
        self.bag.items.remove(index);
        self.bag.money += price;
        self.log.push(format!("The shopkeeper bought {} for {price} Poké.", book.item_name(item)));
    }

    /// Talking with Kecleon is over.
    pub(crate) fn leave_shop(&mut self) {
        if let Some(shop) = self.talk() {
            shop.talk = None;
            self.log.push("The shopkeeper: Come back any time!".to_string());
        }
    }

    /// Who stands on a tile takes what lies there: the leader all of it,
    /// a teammate one thing for the bag if there is room (never what is for
    /// sale), a foe holding nothing one thing, and holds it.
    pub(crate) fn pick_up(&mut self, who: MonsterId) {
        let Some(monster) = self.dungeon.as_ref().and_then(|dungeon| dungeon.monster(who)) else {
            return;
        };
        let (side, at, name, held) = (monster.side, monster.at, monster.name.clone(), monster.stats.held);
        match side {
            Side::Leader => {
                while let Some(item) = self.take_lying(at, true) {
                    self.leader_takes(item, &name);
                }
            }
            Side::Ally => {
                let Some(next) = self.dungeon.as_ref().and_then(|dungeon| dungeon.items.iter().find(|item| item.at == at && item.price.is_none())) else {
                    return;
                };
                if next.item != status::item::POKE && self.bag.items.len() >= BAG_CAPACITY {
                    return;
                }
                let Some(item) = self.take_lying(at, false) else {
                    return;
                };
                match item.item == status::item::POKE {
                    true => self.take_money(item.item, &name),
                    false => {
                        stack::add(&mut self.bag.items, item.item);
                        self.log.push(format!("{name} picked up {}.", self.book.item_name(item.item)));
                        self.sights.push(Sight::Sound(PICK_UP_SOUND));
                    }
                }
            }
            Side::Foe if held == 0 => {
                let Some(item) = self.take_lying(at, false) else {
                    return;
                };
                let Some(dungeon) = self.dungeon.as_mut() else {
                    return;
                };
                let seen = ai::can_see(&dungeon.floor, dungeon.leader().at, at);
                if let Some(foe) = dungeon.monster_mut(who) {
                    foe.stats.held = item.item;
                }
                if seen {
                    self.log.push(format!("{name} picked up {}.", self.book.item_name(item.item)));
                }
            }
            _ => {}
        }
    }

    /// Takes what lies on a tile, what is for sale too if asked.
    fn take_lying(&mut self, at: Pos, for_sale: bool) -> Option<FloorItem> {
        let items = &mut self.dungeon.as_mut()?.items;
        let place = items.iter().position(|item| item.at == at && (for_sale || item.price.is_none()))?;
        Some(items.remove(place))
    }

    fn leader_takes(&mut self, lying: FloorItem, name: &str) {
        let item = lying.item;
        let item_name = self.book.item_name(item);
        if let Some(price) = lying.price {
            let price = self.dungeon.as_ref().map_or(price, |dungeon| haggled(dungeon.leader(), price, true));
            self.bag.items.push(item);
            match self.dungeon.as_mut().and_then(|dungeon| dungeon.shop.as_mut()).filter(|shop| shop.keeper.is_some()) {
                Some(shop) => {
                    shop.owed.push((item, price));
                    self.log.push(format!("{name} picked up {item_name} ({price} Poké, to pay the shopkeeper)."));
                }
                None => self.log.push(format!("{name} picked up {item_name}.")),
            }
        } else if item == status::item::POKE {
            self.take_money(item, name);
        } else {
            stack::add(&mut self.bag.items, item);
            let said = self.book.phrase(crate::strings::log_text::PICKED_UP, &[("string:0", name), ("item:0", &item_name)]);
            self.log.push(said.unwrap_or_else(|| format!("{name} picked up {item_name}.")));
            self.sights.push(Sight::Sound(PICK_UP_SOUND));
        }
    }

    // Known gap: the original settles the amount when the pile appears.
    fn take_money(&mut self, item: u16, name: &str) {
        let book = self.book.clone();
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let most = dungeon.spec(&book).map_or(0, |floor| floor.max_money);
        let amount = money_amount(&mut dungeon.rng, &book.tables.money_table, most).max(1);
        // (Coin Watcher on the team: a fifth more, the game's 1.2.)
        let watcher = dungeon.monsters.iter().any(|monster| monster.team() && monster.alive() && iq::has(&monster.stats, iq::COIN_WATCHER));
        let amount = if watcher { amount * 6 / 5 } else { amount };
        self.bag.money += amount;
        self.log.push(format!("{name} picked up {amount} {}.", book.item_name(item)));
        self.sights.push(Sight::Sound(MONEY_SOUND));
    }

    /// Answers the question the dungeon asks.
    pub(crate) fn answer(&mut self, yes: bool) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let Some(ask) = dungeon.ask.take() else {
            return;
        };
        match ask {
            Ask::Stairs => dungeon.take_stairs = yes,
            Ask::Recruit(recruit) if yes => self.join(recruit),
            Ask::Recruit(recruit) => self.log.push(format!("{} went away.", recruit.name)),
            Ask::Leave if yes => {
                dungeon.outcome = Some(Outcome::Escaped);
                self.log.push("The team left the dungeon.".to_string());
            }
            Ask::Leave => {}
            Ask::HiddenStairs => dungeon.take_hidden = yes,
            Ask::Bazaar(service) if yes => self.bazaar(service),
            Ask::Bazaar(_) => self.log.push("Come back anytime.".to_string()),
        }
    }

    /// A recruit joins: in the team at once if there is room (four at
    /// most), at the guild otherwise.
    fn join(&mut self, recruit: Recruit) {
        let progress = &mut self.team.progress;
        progress.recruits.push(recruit.clone());
        let index = progress.recruits.len() - 1;
        // (A nickname offered back in town.)
        progress.to_name.push(index);
        let Some(dungeon) = self.dungeon.as_ref() else {
            return;
        };
        let room = dungeon.monsters.iter().filter(|monster| monster.side == Side::Ally && monster.alive()).count() < 3;
        let leader = dungeon.leader().at;
        let busy = |tile: Pos| dungeon.monsters.iter().any(|monster| monster.alive() && monster.at == tile);
        let free =
            ai::DIRECTIONS.iter().map(|&(dx, dy)| Pos::new(leader.x + dx, leader.y + dy)).find(|&tile| dungeon.floor.tile(tile).is_walkable() && !busy(tile));
        let placed = free.filter(|_| room).and_then(|at| self.place_recruit(Some(index), &recruit, at));
        self.log.push(match placed {
            Some(_) => format!("{} joined the team!", recruit.name),
            None => format!("{} joined the team and set off for the base!", recruit.name),
        });
    }

    /// A monster joins the team on the floor, as `recruit` says: a recruit
    /// (by its place in the list), or one that only comes along for a while
    /// (none).
    pub(crate) fn place_recruit(&mut self, index: Option<usize>, recruit: &Recruit, at: Pos) -> Option<MonsterId> {
        let setup = self.book.member(recruit.species, false)?;
        let [level, hp, attack, defense, sp_attack, sp_defense] = recruit.stats;
        let stats = Stats { level, max_hp: hp, hp, attack, defense, sp_attack, sp_defense, ..setup.stats };
        let learnt: Vec<MoveSlot> = setup.growth.learnset.iter().filter(|(when, _)| *when as i32 <= level).map(|(_, slot)| *slot).collect();
        let mut moves = match (recruit.moves.is_empty(), learnt.is_empty()) {
            (false, _) => known_moves(&setup.moves, &setup.growth, &recruit.moves, None),
            (true, false) => learnt[learnt.len().saturating_sub(4)..].to_vec(),
            (true, true) => setup.moves.clone(),
        };
        moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp);
        let dungeon = self.dungeon.as_mut()?;
        let id = dungeon.new_id();
        let mut member = Monster::new(id, Side::Ally, recruit.name.clone(), Some(recruit.species), stats, moves, Statuses::for_species(setup.movement), at);
        member.experience = Some(Experience::new(&setup.growth, level));
        member.extra = Some(index);
        // (Those of the team come first in the list.)
        let place = dungeon.monsters.iter().rposition(|monster| monster.team()).map_or(0, |last| last + 1);
        dungeon.monsters.insert(place, member);
        Some(id)
    }

    /// Turns the foe a Transfer Orb was used on into another monster of the
    /// floor, where it stood.
    pub(crate) fn transfer_foes(&mut self) {
        let book = self.book.clone();
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let Some(at) = dungeon.transfer.take() else {
            return;
        };
        let Some(place) = dungeon.monsters.iter().position(|monster| monster.side == Side::Foe && monster.at == at && monster.alive()) else {
            return;
        };
        let gone = dungeon.monsters.remove(place);
        self.log.push(format!("{} transformed!", gone.name));
        spawn_enemy(dungeon, &book, at, false);
    }
}
