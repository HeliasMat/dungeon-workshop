//! What the town's services and the team's own menu do to the team: the
//! bag, evolving, Spinda's Café, Croagunk's Swap Shop, Xatu's appraisal, the
//! Link Shop, the guild's bell, the boards and the shops' stock. The lists,
//! cursors and windows are the screens'; what is told comes back as words.

use crate::dungeon::{exclusive, held, iq, stack};
use crate::jobs::{self, Job};
use crate::services::{BAG_CAPACITY, Goods};
use crate::town::{self, RecycleTrade, item};

use super::Run;
use super::team::{LINKED_MOVE, Member, Team, Whose, move_id, tactic};

/// The moves a member knows at most.
const MOVES_KNOWN: usize = 4;

/// Items of the day Croagunk shows, and those of the bag each costs.
const SWAP_WARES: usize = 8;
const SWAP_COST: u16 = 3;
/// Recruits who may come along.
const ALONG: usize = 2;

/// A member of the team, for the town's services.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whom {
    Leader,
    Partner,
    Recruit(usize),
}

/// Numbers drawn for the services (no clock in the rules: the screen gives
/// the seed).
#[derive(Clone, Copy, Debug)]
pub struct Draw(pub u32);

impl Draw {
    /// A number below `below`.
    pub fn below(&mut self, below: usize) -> usize {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.0 >> 16) as usize % below.max(1)
    }
}

impl Team {
    /// The leader's name: the one it was given, or its species'.
    pub fn leader_name(&self) -> &str {
        if self.progress.names.0.is_empty() { &self.leader.name } else { &self.progress.names.0 }
    }

    pub fn partner_name(&self) -> &str {
        if self.progress.names.1.is_empty() { &self.partner.name } else { &self.progress.names.1 }
    }

    /// The tactic of a member, as kept by its name.
    pub fn tactic_of(&self, name: &str) -> u8 {
        self.progress.tactics.get(name).copied().unwrap_or(0)
    }

    fn keep(&mut self, partner: bool, member: Member) {
        match partner {
            false => self.progress.leader = Some(member),
            true => self.progress.partner = Some(member),
        }
    }
}

impl Run {
    /// The items of the bag, by id.
    pub fn bag_ids(&self) -> Vec<u16> {
        self.bag.items.iter().map(|&value| stack::id(value)).collect()
    }

    /// Puts an item in the bag, or at Kangaskhan Storage when it is full.
    pub fn receive(&mut self, given: u16) -> String {
        let name = self.book.item_name(given);
        match self.stow(given) {
            true => format!("Vous recevez {name}."),
            false => format!("The bag is full: {name} was sent to storage."),
        }
    }

    /// An item in the bag, or in the storage when it is full; whether it
    /// went in the bag.
    pub(super) fn stow(&mut self, item: u16) -> bool {
        let room = self.bag.items.len() < BAG_CAPACITY;
        match room {
            true => self.bag.items.push(item),
            false => self.team.progress.storage.push(item),
        }
        room
    }

    /// The item at that place of the bag thrown away.
    pub fn throw_away(&mut self, at: usize) {
        self.bag.items.remove(at);
    }

    /// What the team owns goes through a service and back, with the stock of
    /// the shop if it is one.
    pub fn through_service<R>(&mut self, shop: Option<usize>, service: impl FnOnce(&mut Goods) -> R) -> R {
        let stock = shop.and_then(|shop| self.team.progress.stock[shop].take());
        let mut goods = Goods {
            bag: std::mem::take(&mut self.bag.items),
            money: self.bag.money,
            storage: std::mem::take(&mut self.team.progress.storage),
            bank: self.bank,
            stock: stock.unwrap_or_default(),
        };
        let answer = service(&mut goods);
        (self.bag.items, self.bag.money) = (goods.bag, goods.money);
        self.team.progress.storage = goods.storage;
        self.bank = goods.bank;
        if let Some(shop) = shop {
            self.team.progress.stock[shop] = Some(goods.stock);
        }
        answer
    }

    /// What a board offers and the jobs held, to take one from the other.
    pub fn board(&mut self, outlaws: bool) -> (&mut Vec<Job>, &mut Vec<Job>) {
        let progress = &mut self.team.progress;
        (if outlaws { &mut progress.outlaws } else { &mut progress.offers }, &mut progress.jobs)
    }

    /// A held job given up.
    pub fn give_up_job(&mut self, at: usize) {
        self.team.progress.jobs.remove(at);
    }

    /// A member of the team given a name.
    pub fn rename(&mut self, whose: Whose, text: &str) {
        self.team.progress.rename(whose, text);
    }

    /// A recruit sent away for good.
    pub fn release(&mut self, at: usize) {
        self.team.progress.release(at);
    }

    /// The next recruit waiting for a nickname, if any: it waits no more.
    pub fn next_to_name(&mut self) -> Option<usize> {
        let waiting = &mut self.team.progress.to_name;
        (!waiting.is_empty()).then(|| waiting.remove(0))
    }

    /// Takes the bag's items at these places.
    fn take_places(&mut self, mut places: Vec<usize>) {
        places.sort_unstable_by(|a, b| b.cmp(a));
        for place in places {
            if place < self.bag.items.len() {
                self.bag.items.remove(place);
            }
        }
    }

    fn take_item(&mut self, taken: u16) {
        if let Some(place) = self.bag_ids().iter().position(|&held| held == taken) {
            self.bag.items.remove(place);
        }
    }

    pub fn members(&self) -> Vec<Whom> {
        let mut members = vec![Whom::Leader];
        if !self.team.alone {
            members.push(Whom::Partner);
        }
        members.extend((0..self.team.progress.recruits.len()).map(Whom::Recruit));
        members
    }

    pub fn member_name(&self, whom: Whom) -> String {
        match whom {
            Whom::Leader => self.team.leader_name().to_string(),
            Whom::Partner => self.team.partner_name().to_string(),
            Whom::Recruit(at) => self.team.progress.recruits.get(at).map_or(String::new(), |recruit| recruit.name.clone()),
        }
    }

    pub fn member_species(&self, whom: Whom) -> u16 {
        match whom {
            Whom::Leader => self.team.species.0,
            Whom::Partner => self.team.species.1,
            Whom::Recruit(at) => self.team.progress.recruits.get(at).map_or(0, |recruit| recruit.species),
        }
    }

    pub fn member_level(&self, whom: Whom) -> i32 {
        match whom {
            Whom::Leader => self.team.kept_member(false).stats.level,
            Whom::Partner => self.team.kept_member(true).stats.level,
            Whom::Recruit(at) => self.team.progress.recruits.get(at).map_or(1, |recruit| recruit.stats[0]),
        }
    }

    /// Items of the game picked at random among those sold for between
    /// these prices (no box, ticket nor money).
    fn prize(&self, prices: std::ops::Range<i32>, draw: &mut Draw) -> Option<u16> {
        let pool: Vec<u16> = (1..self.book.tables.items.len() as u16)
            .filter(|&id| town::treasure_box(id).is_none() && !item::TICKETS.contains(&id) && id != item::POKE)
            .filter(|&id| prices.contains(&self.book.item_prices(id).0))
            .collect();
        (!pool.is_empty()).then(|| pool[draw.below(pool.len())])
    }

    /// The exclusive items made for the team's leader and partner (by
    /// type, species or family), or every one when none is.
    fn exclusive_for_team(&self) -> Vec<u16> {
        let team = [self.team.species.0, self.team.species.1];
        let fits = |kind: u8, value: i16| {
            team.iter().any(|&species| {
                let types = self.book.species(species).map_or((0, 0), |species| species.types);
                match kind {
                    1..=4 => value as u8 == types.0 || value as u8 == types.1,
                    9 | 10 => value as u16 == species,
                    _ => value as u16 == self.book.family(species),
                }
            })
        };
        let kinds = self.book.tables.exclusive.kinds.iter().enumerate();
        let all: Vec<(u16, (u8, i16))> = kinds.map(|(at, &kind)| (exclusive::FIRST + at as u16, kind)).filter(|(_, kind)| kind.0 != 0).collect();
        let fitting: Vec<u16> = all.iter().filter(|(_, (kind, value))| fits(*kind, *value)).map(|(id, _)| *id).collect();
        match fitting.is_empty() {
            true => all.into_iter().map(|(id, _)| id).collect(),
            false => fitting,
        }
    }

    /// A member evolves into another species: its stats grow by what its
    /// new species' base has more, the items it needs are used up. A
    /// recruit takes its new species' name if it had its old one's.
    pub fn evolve(&mut self, whom: Whom, into: u16) -> Result<(), String> {
        let book = self.book.clone();
        let species = self.member_species(whom);
        let (Some(new), Some(old)) = (book.species(into), book.species(species)) else {
            return Err("This evolution does not happen here.".to_string());
        };
        let (stats, iq, moves): ([i32; 6], Option<i32>, Vec<u16>) = match whom {
            Whom::Leader | Whom::Partner => {
                let member = self.team.kept_member(whom == Whom::Partner);
                let stats = &member.stats;
                (
                    [stats.level, stats.max_hp, stats.attack, stats.defense, stats.sp_attack, stats.sp_defense],
                    Some(stats.iq),
                    member.moves.iter().map(|&id| move_id(id)).collect(),
                )
            }
            Whom::Recruit(at) => {
                let recruit = &self.team.progress.recruits[at];
                (recruit.stats, None, recruit.moves.iter().map(|&id| move_id(id)).collect())
            }
        };
        let bag = self.bag_ids();
        let evolving = town::Evolving { level: stats[0], iq, attack: stats[2], defense: stats[3], gender: old.gender, moves: &moves, bag: &bag };
        let used = town::requirement(&new.evolution, &evolving)?;
        used.iter().for_each(|&used| self.take_item(used));
        let grown = town::evolved_stats([stats[1], stats[2], stats[3], stats[4], stats[5]], old.base, new.base);
        match whom {
            Whom::Recruit(at) => {
                let recruit = &mut self.team.progress.recruits[at];
                recruit.species = into;
                recruit.stats = [stats[0], grown[0], grown[1], grown[2], grown[3], grown[4]];
                if recruit.name == old.name {
                    recruit.name = new.name.clone();
                }
            }
            Whom::Leader | Whom::Partner => {
                let partner = whom == Whom::Partner;
                let mut member = self.team.kept_member(partner);
                (member.stats.max_hp, member.stats.attack, member.stats.defense, member.stats.sp_attack, member.stats.sp_defense) =
                    (grown[0], grown[1], grown[2], grown[3], grown[4]);
                member.stats.hp = member.stats.max_hp;
                self.team.keep(partner, member);
                let (hero, ally) = self.team.species;
                let (hero, ally) = if partner { (hero, into) } else { (into, ally) };
                let (alone, third, joined) = (self.team.alone, self.team.third, self.team.third_joined);
                self.team.change(&book, hero, ally, true);
                (self.team.alone, self.team.third, self.team.third_joined) = (alone, third, joined);
            }
        }
        Ok(())
    }

    /// Spinda presses a Gummi for the leader or its partner.
    pub fn drink_juice(&mut self, gummi: u16, partner: bool) -> String {
        let mut member = self.team.kept_member(partner);
        let gain = self.book.tables.iq.gummi(member.stats.types, gummi).map_or(1, |(gain, _)| gain);
        member.stats.iq = (member.stats.iq + gain).min(iq::MAX_IQ);
        self.team.keep(partner, member);
        self.take_item(gummi);
        let name = self.member_name(if partner { Whom::Partner } else { Whom::Leader });
        format!("{name} drank the juice. Its IQ rose by {gain}!")
    }

    /// The trades that make the Project P find a new region.
    const PROJECT_REGION: i32 = 30;

    /// What the café's Project P has found so far, by the trades made: new
    /// trades (once the first trade kept for later is offered), then a new
    /// region (from `PROJECT_REGION` trades on).
    pub fn project_p(&self) -> (bool, bool) {
        let made = self.team.progress.recycled;
        let first_kept = self.book.tables.recycle.iter().map(|trade| trade.recycles).filter(|&count| count > 0).min();
        (first_kept.is_some_and(|count| made >= count), made >= Self::PROJECT_REGION)
    }

    /// The trades the Recycle Shop offers, by their places in its table.
    pub fn recycle_offers(&self) -> Vec<usize> {
        let trades = &self.book.tables.recycle;
        (0..trades.len()).filter(|&at| trades[at].kind != 2 && trades[at].item != 0 && trades[at].recycles <= self.team.progress.recycled).collect()
    }

    pub fn recycle(&mut self, trade: usize, draw: &mut Draw) -> String {
        let book = self.book.clone();
        let trade = &book.tables.recycle[trade];
        let Some(places) = trade.take_from(&self.bag_ids()) else {
            return "You are missing items for this swap.".to_string();
        };
        self.take_places(places);
        self.team.progress.recycled += 1;
        let mut said = self.receive(trade.item);
        if let Some(ticket) = trade.bonus_ticket()
            && (draw.below(100) as i16) < trade.bonus_odds
        {
            said = format!("{said} En prime : {}", self.receive(ticket));
        }
        said
    }

    /// The lottery: a ticket played.
    pub fn play_ticket(&mut self, ticket: u16, draw: &mut Draw) -> String {
        self.take_item(ticket);
        let rank = town::lottery(ticket, draw.below(100) as u32);
        let prices = match rank {
            3 => 1500..100_000,
            2 => 400..1500,
            _ => 1..400,
        };
        match (rank, self.prize(prices, draw)) {
            (0, _) | (_, None) => "No luck... This ticket won nothing.".to_string(),
            (rank, Some(prize)) => format!("A win! A rank {rank} prize. {}", self.receive(prize)),
        }
    }

    /// Croagunk's items of the day: exclusive items.
    pub fn swap_wares(&self) -> Vec<u16> {
        let mut wares = self.exclusive_for_team();
        let progress = &self.team.progress;
        let mut day = Draw(progress.returned.len() as u32 * 7_919 + progress.jobs.len() as u32 + 1);
        let mut picked = Vec::new();
        while picked.len() < SWAP_WARES && !wares.is_empty() {
            let at = day.below(wares.len());
            picked.push(wares.remove(at));
        }
        picked
    }

    /// One of them, for items of the bag.
    pub fn swap(&mut self, wanted: u16) -> String {
        let cost = RecycleTrade { costs: vec![(0, SWAP_COST)], ..Default::default() };
        match cost.take_from(&self.bag_ids()) {
            None => "Three items are needed for a swap.".to_string(),
            Some(places) => {
                self.take_places(places);
                self.receive(wanted)
            }
        }
    }

    /// Xatu opens the treasure box at that place of the bag.
    pub fn appraise(&mut self, at: usize, draw: &mut Draw) -> String {
        if self.bag.money < town::APPRAISAL_PRICE {
            return "You don't have enough Poké.".to_string();
        }
        let found = self.exclusive_for_team();
        self.bag.money -= town::APPRAISAL_PRICE;
        self.bag.items.remove(at);
        let prize = match found.is_empty() {
            true => self.prize(500..5000, draw),
            false => Some(found[draw.below(found.len())]),
        };
        prize.map_or_else(|| "The box was empty...".to_string(), |prize| format!("The box opens! {}", self.receive(prize)))
    }

    /// The Link Shop: that move linked to the one above, or delinked.
    /// (Moves a member knows at most: see `remember_move`.)
    pub fn link_move(&mut self, partner: bool, slot: usize) {
        let mut member = self.team.kept_member(partner);
        if let Some(id) = member.moves.get_mut(slot).filter(|_| slot > 0) {
            *id ^= LINKED_MOVE;
            self.team.keep(partner, member);
        }
    }

    /// The Link Shop: the moves the member learnt by its level and no longer
    /// knows, to be remembered.
    pub fn forgotten_moves(&self, partner: bool) -> Vec<u16> {
        let member = self.team.kept_member(partner);
        let founder = if partner { &self.team.partner } else { &self.team.leader };
        let mut forgotten: Vec<u16> = Vec::new();
        for (_, slot) in founder.growth.learnset.iter().filter(|(at, _)| *at as i32 <= member.stats.level) {
            let known = member.moves.iter().any(|&id| move_id(id) == slot.id);
            if !known && !forgotten.contains(&slot.id) {
                forgotten.push(slot.id);
            }
        }
        forgotten
    }

    /// The Link Shop: a forgotten move remembered, if the member has room
    /// (four moves at most); what is told.
    pub fn remember_move(&mut self, partner: bool, id: u16) -> String {
        let mut member = self.team.kept_member(partner);
        let name = self.member_name(if partner { Whom::Partner } else { Whom::Leader });
        if member.moves.len() >= MOVES_KNOWN {
            return format!("{name} already knows four moves: make it forget one first.");
        }
        member.moves.push(id);
        self.team.keep(partner, member);
        format!("{name} remembered {}!", self.book.move_name(id))
    }

    /// The Link Shop: a move forgotten (one is always kept); what is told.
    pub fn forget_move(&mut self, partner: bool, slot: usize) -> String {
        let mut member = self.team.kept_member(partner);
        let name = self.member_name(if partner { Whom::Partner } else { Whom::Leader });
        if member.moves.len() <= 1 || slot >= member.moves.len() {
            return format!("{name} must keep at least one move.");
        }
        let id = move_id(member.moves.remove(slot));
        // (What was linked to it is unlinked; the first links to nothing.)
        if let Some(next) = member.moves.get_mut(slot) {
            *next &= !LINKED_MOVE;
        }
        if let Some(first) = member.moves.first_mut() {
            *first &= !LINKED_MOVE;
        }
        self.team.keep(partner, member);
        format!("{name} oublie {}.", self.book.move_name(id))
    }

    /// A member's move goes to another place in its order, and all come
    /// unlinked (a link is the Link Shop's).
    pub fn reorder_move(&mut self, partner: bool, from: usize, to: usize) {
        let mut member = self.team.kept_member(partner);
        if from < member.moves.len() && to < member.moves.len() {
            member.moves.swap(from, to);
            for id in &mut member.moves {
                *id &= !LINKED_MOVE;
            }
            self.team.keep(partner, member);
        }
    }

    /// The recruits who come along: those picked at the guild's bell, the
    /// first two until then.
    pub fn along(&self) -> Vec<usize> {
        let progress = &self.team.progress;
        progress.chosen.clone().unwrap_or_else(|| (0..progress.recruits.len().min(ALONG)).collect())
    }

    /// The guild's bell: a recruit picked, or let go.
    pub fn pick_along(&mut self, at: usize) {
        let mut chosen = self.along();
        match chosen.iter().position(|&picked| picked == at) {
            Some(place) => _ = chosen.remove(place),
            None if chosen.len() < ALONG => chosen.push(at),
            None => {}
        }
        self.team.progress.chosen = Some(chosen);
    }

    /// The item at that place of the bag held by the leader or its partner;
    /// what was held goes back in the bag.
    pub fn hold(&mut self, at: usize, partner: bool) {
        let partner = partner && !self.team.alone;
        let Some(&item) = self.bag.items.get(at).filter(|&&item| held::is_held_item(item) && !stack::sticky(item)) else {
            return;
        };
        let mut member = self.team.kept_member(partner);
        self.bag.items.remove(at);
        if member.stats.held != 0 {
            self.bag.items.push(member.stats.held);
        }
        member.stats.held = item;
        self.team.keep(partner, member);
    }

    /// The bag sorted, as the original sorts it: by item.
    pub fn sort_bag(&mut self) {
        self.bag.items.sort_by_key(|&item| (stack::id(item), item));
    }

    /// The next tactic a member may follow (`who`: 1 the partner, then the
    /// recruits).
    pub fn next_tactic(&mut self, who: usize) {
        let whom = if who == 1 { Whom::Partner } else { Whom::Recruit(who.saturating_sub(2)) };
        let (name, unlocked) = (self.member_name(whom), tactic::unlocked(self.member_level(whom)));
        let current = self.team.tactic_of(&name);
        let next = unlocked.iter().position(|&tactic| tactic == current).map_or(0, |at| (at + 1) % unlocked.len().max(1));
        if let Some(&tactic) = unlocked.get(next) {
            self.team.progress.tactics.insert(name, tactic);
        }
    }

    /// A board filled if it is empty: its jobs are in the dungeons open so
    /// far.
    pub fn fill_board(&mut self, outlaws: bool, open: &[u16], draw: &mut Draw) {
        let filled = if outlaws { &self.team.progress.outlaws } else { &self.team.progress.offers };
        if !filled.is_empty() {
            return;
        }
        let book = self.book.clone();
        let floors: Vec<(u16, u32)> =
            open.iter().map(|&dungeon| (dungeon, book.tables.dungeons.get(dungeon as usize).map_or(0, |dungeon| dungeon.floors.len() as u32))).collect();
        let offered = jobs::offers(&book.tables.jobs, &floors, outlaws, |dungeon, floor| book.floor_species(dungeon, floor), |below| draw.below(below));
        match outlaws {
            true => self.team.progress.outlaws = offered,
            false => self.team.progress.offers = offered,
        }
    }

    /// A shop not visited since the last dungeon restocks.
    pub fn restock(&mut self, shop: usize, chapter: usize, seed: u32) {
        if self.team.progress.stock[shop].is_none() {
            self.team.progress.stock[shop] = Some(self.book.shop_stock(shop == 1, chapter as i32, seed));
        }
    }

    /// What the guild gives for sentry duty: an Oran Berry, a Gummi, a
    /// Reviver Seed, the better the more (`rating`: 0 to 3).
    pub fn sentry_reward(&mut self, rating: u8, draw: &mut Draw) -> Option<String> {
        let reward = match rating {
            3 => Some(73),
            2 => Some(iq::FIRST_GUMMI + draw.below(12) as u16),
            1 => Some(70),
            _ => None,
        };
        reward.map(|item| self.receive(item))
    }
}
