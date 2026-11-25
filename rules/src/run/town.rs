//! What the town's services and the team's own menu do to the team: the
//! bag, evolving, Spinda's Café, Croagunk's Swap Shop, Xatu's appraisal, the
//! Link Shop, the guild's bell, the boards and the shops' stock. The lists,
//! cursors and windows are the screens'; what is told comes back as words.

use crate::dungeon::{exclusive, held, iq, stack};
use crate::jobs::{self, Job};
use crate::services::Goods;
use crate::town::{self, item, swap};

use super::Run;
use super::team::{LINKED_MOVE, Member, Team, Whose, move_id, tactic};

/// The moves a member knows at most.
const MOVES_KNOWN: usize = 4;

/// What the Link Shop asks for a session: all the links, the moves
/// forgotten and remembered of a visit (overlay 18's).
pub const LINK_PRICE: i32 = 500;
/// Recruits who may come along with the leader and the partner; one more
/// when the partner stays at home (a team of four at most, the original's).
const ALONG: usize = 2;

/// The team's name before it is given one (the original's, in English).
pub const DEFAULT_TEAM_NAME: &str = "Poképals";
/// The letters a team's name has at most, as in the original.
pub const TEAM_NAME_LETTERS: usize = 10;

/// A member of the team, for the town's services.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whom {
    Leader,
    Partner,
    Recruit(usize),
}

/// What Croagunk gives for a swap: an item, had for its set (the easy
/// way), or (none) for so many Exclusive Items, whatever they are (an item
/// of the day).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapOffer {
    pub item: u16,
    pub set: Option<Vec<u16>>,
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

    /// The team's name: the one it was given, or the original's own
    /// (`DEFAULT_TEAM_NAME`) until then.
    pub fn team_name(&self) -> &str {
        if self.progress.names.2.is_empty() { DEFAULT_TEAM_NAME } else { &self.progress.names.2 }
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
            true => format!("You received {name}."),
            false => format!("The bag is full: {name} was sent to storage."),
        }
    }

    /// An item in the bag, or in the storage when it is full; whether it
    /// went in the bag.
    pub(super) fn stow(&mut self, item: u16) -> bool {
        let room = self.bag.items.len() < self.bag_room();
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
            bag_room: self.bag_room(),
            storage_room: self.storage_room(),
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
        let used = self.evolution_needs(whom, into)?;
        let stats = self.evolving_parts(whom).0;
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

    /// What a member could evolve into at Luminous Spring (by their first
    /// entries), each with whether it may now, else what it lacks: those of
    /// the data's that happen by level, IQ or items, with their extra
    /// conditions (a Link Cable, a ribbon, its gender, a move known…).
    pub fn evolution_offers(&self, whom: Whom) -> Vec<(u16, Result<(), String>)> {
        let species = self.member_species(whom);
        let happens = |into: &u16| {
            let method = self.book.species(*into).map_or(town::method::NONE, |new| new.evolution.method);
            matches!(method, town::method::LEVEL | town::method::IQ | town::method::ITEMS)
        };
        self.book.evolutions(species).into_iter().filter(happens).map(|into| (into, self.evolution_needs(whom, into).map(|_| ()))).collect()
    }

    /// Whether a member evolves into that species now: the items it uses
    /// up, or what it lacks.
    fn evolution_needs(&self, whom: Whom, into: u16) -> Result<Vec<u16>, String> {
        let Some(new) = self.book.species(into) else {
            return Err("This evolution does not happen here.".to_string());
        };
        let gender = self.book.species(self.member_species(whom)).map_or(0, |old| old.gender);
        let (stats, iq, moves) = self.evolving_parts(whom);
        let bag = self.bag_ids();
        let evolving = town::Evolving { level: stats[0], iq, attack: stats[2], defense: stats[3], gender, moves: &moves, bag: &bag };
        town::requirement(&new.evolution, &evolving)
    }

    /// A member as the conditions of evolving read it: its level and stats
    /// (level, HP, attack, defense, special attack, special defense), its
    /// IQ (none kept for a recruit), its moves.
    fn evolving_parts(&self, whom: Whom) -> ([i32; 6], Option<i32>, Vec<u16>) {
        match whom {
            Whom::Leader | Whom::Partner => {
                let member = self.team.kept_member(whom == Whom::Partner);
                let stats = &member.stats;
                (
                    [stats.level, stats.max_hp, stats.attack, stats.defense, stats.sp_attack, stats.sp_defense],
                    Some(stats.iq),
                    member.moves.iter().map(|&id| move_id(id)).collect(),
                )
            }
            Whom::Recruit(at) => match self.team.progress.recruits.get(at) {
                Some(recruit) => (recruit.stats, None, recruit.moves.iter().map(|&id| move_id(id)).collect()),
                None => ([1; 6], None, Vec::new()),
            },
        }
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

    /// The Gummis of the bag, by their places in it: what the café presses into juice.
    pub fn gummis(&self) -> Vec<usize> {
        let iq = &self.book.tables.iq;
        (0..self.bag.items.len()).filter(|&at| iq.gummi((0, 0), stack::id(self.bag.items[at])).is_some()).collect()
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
            return "You don't have the items this recycle takes.".to_string();
        };
        let found = self.project_p();
        self.take_places(places);
        self.team.progress.recycled += 1;
        let mut said = format!("Thanks for recycling! {}", self.receive(trade.item));
        if let Some(ticket) = trade.bonus_ticket()
            && (draw.below(100) as i16) < trade.bonus_odds
        {
            said = format!("{said}\nCongratulations, it came with a bonus! {}", self.receive(ticket));
        }
        // (Project P: the trades made find more.)
        match (found, self.project_p()) {
            ((false, _), (true, _)) => said.push_str("\nProject P is going well: the Recycle Shop has new trades!"),
            ((_, false), (_, true)) => said.push_str("\nProject P has found something: a land no one had seen before!"),
            _ => {}
        }
        said
    }

    /// The lottery tickets of the bag, by their places in it.
    pub fn tickets(&self) -> Vec<usize> {
        (0..self.bag.items.len()).filter(|&at| item::TICKETS.contains(&stack::id(self.bag.items[at]))).collect()
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

    /// Croagunk's items of the day: Exclusive Items made for the team's
    /// species (those for a type never), drawn by the day, the rarer the
    /// less often; those swapped for today gone (`town::swap`).
    pub fn swap_wares(&self) -> Vec<u16> {
        let made = self.exclusive_for_team();
        let for_species: Vec<u16> = made.iter().copied().filter(|&item| item >= swap::FIRST_SPECIES_SET).collect();
        let progress = &self.team.progress;
        let mut pool: Vec<u16> = if for_species.is_empty() { made } else { for_species };
        pool.retain(|item| !progress.swapped.contains(item));
        let mut day = Draw(progress.day.max(1).wrapping_mul(7_919) + 1);
        let mut picked = Vec::new();
        while picked.len() < swap::WARES && !pool.is_empty() {
            let total: usize = pool.iter().map(|&item| swap::weight(item)).sum();
            let (mut roll, mut at) = (day.below(total), 0);
            while at + 1 < pool.len() && roll >= swap::weight(pool[at]) {
                roll -= swap::weight(pool[at]);
                at += 1;
            }
            picked.push(pool.remove(at));
        }
        picked.sort_unstable();
        picked
    }

    /// The Exclusive Items the team owns, by id: the bag's, then the
    /// storage's (Croagunk takes from both, as the original's).
    fn owned_exclusives(&self) -> Vec<u16> {
        self.bag_ids().into_iter().chain(self.team.progress.storage.iter().map(|&value| stack::id(value))).filter(|&id| swap::exclusive(id)).collect()
    }

    /// What Croagunk offers: the rarer items of a four the team holds the
    /// set for (the easy way), then its items of the day.
    pub fn swap_offers(&self) -> Vec<SwapOffer> {
        let owned = self.owned_exclusives();
        let mut offers: Vec<SwapOffer> = Vec::new();
        for &held in &owned {
            for item in held.checked_sub(swap::FIRST_SET).map(|place| held - place % 4).map_or(Vec::new(), |first| vec![first + 2, first + 3]) {
                let set = swap::set_for(item).filter(|set| swap::set_taken(&owned, set).is_some());
                if let Some(set) = set.filter(|_| !offers.iter().any(|offer| offer.item == item)) {
                    offers.push(SwapOffer { item, set: Some(set) });
                }
            }
        }
        for item in self.swap_wares() {
            if !offers.iter().any(|offer| offer.item == item) {
                offers.push(SwapOffer { item, set: None });
            }
        }
        offers
    }

    /// The Exclusive Items a swap would take (by id), if the team owns
    /// them: its set, or so many of the fewest stars.
    pub fn swap_takes(&self, offer: &SwapOffer) -> Option<Vec<u16>> {
        let owned = self.owned_exclusives();
        let places = match &offer.set {
            Some(set) => swap::set_taken(&owned, set)?,
            None => swap::any_taken(&owned, offer.item, swap::to_trade(offer.item))?,
        };
        Some(places.into_iter().map(|at| owned[at]).collect())
    }

    /// The swap made: its price paid, its items put in the cauldron (from
    /// the bag first, then the storage), the item received; an item of the
    /// day is gone for the day. What is told, or why not.
    pub fn swap(&mut self, offer: &SwapOffer) -> Result<String, String> {
        if self.bag.money < swap::PRICE {
            return Err(format!("Croagunk: No Poké, no swap! It's {} Poké a swap.", swap::PRICE));
        }
        let Some(taken) = self.swap_takes(offer) else {
            return Err("Croagunk: Nope, you don't have a set of swappable items for that one.".to_string());
        };
        for item in taken {
            match self.bag_ids().iter().position(|&held| held == item) {
                Some(at) => _ = self.bag.items.remove(at),
                None => {
                    let storage = &mut self.team.progress.storage;
                    if let Some(at) = storage.iter().position(|&value| stack::id(value) == item) {
                        storage.remove(at);
                    }
                }
            }
        }
        self.bag.money -= swap::PRICE;
        if offer.set.is_none() {
            self.team.progress.swapped.push(offer.item);
        }
        let rare = if swap::stars(offer.item) >= 3 { " Whoa, that's beyond valuable! Treat it with care." } else { "" };
        Ok(format!("Into the cauldron they go... The swap worked!{rare} {}", self.receive(offer.item)))
    }

    /// The treasure boxes of the bag, by their places in it.
    pub fn boxes(&self) -> Vec<usize> {
        (0..self.bag.items.len()).filter(|&at| town::treasure_box(stack::id(self.bag.items[at])).is_some()).collect()
    }

    /// Xatu opens the treasure box at that place of the bag: what it holds
    /// (`treasure::content`); without the game's lists, an exclusive item
    /// made for the team or something of a good price.
    pub fn appraise(&mut self, at: usize, draw: &mut Draw) -> String {
        let Some(&value) = self.bag.items.get(at).filter(|&&value| town::treasure_box(stack::id(value)).is_some()) else {
            return "That is no treasure box.".to_string();
        };
        if self.bag.money < town::APPRAISAL_PRICE {
            return "You don't have enough Poké.".to_string();
        }
        self.bag.money -= town::APPRAISAL_PRICE;
        self.bag.items.remove(at);
        let mut rng = crate::dungeon::DungeonRng::new(draw.below(usize::MAX >> 1) as u32);
        let prize = match self.book.extras.rank_lists.is_empty() {
            false => super::treasure::content(value, &self.book.extras, self.book.tables.items.len(), &mut rng),
            true => {
                let found = self.exclusive_for_team();
                match found.is_empty() {
                    true => self.prize(500..5000, draw),
                    false => Some(found[draw.below(found.len())]),
                }
            }
        };
        prize.map_or_else(|| "The box was empty...".to_string(), |prize| format!("The box opens! {}", self.receive(prize)))
    }

    /// The Link Shop: the moves of the leader or its partner, each with
    /// whether it is linked to the one above (used with it in one turn).
    pub fn linked_moves(&self, partner: bool) -> Vec<(u16, bool)> {
        self.team.kept_member(partner).moves.iter().map(|&kept| (move_id(kept), kept & LINKED_MOVE != 0)).filter(|(id, _)| *id != 0).collect()
    }

    /// The Link Shop's price paid for a session, if the bag has it.
    pub fn pay_link_session(&mut self) -> bool {
        let paid = self.bag.money >= LINK_PRICE;
        if paid {
            self.bag.money -= LINK_PRICE;
        }
        paid
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
    /// (four moves at most), else in place of the move at `instead` and
    /// those linked to it; what is told.
    pub fn remember_move(&mut self, partner: bool, id: u16, instead: Option<usize>) -> String {
        let name = self.member_name(if partner { Whom::Partner } else { Whom::Leader });
        let forgot = match instead {
            Some(slot) => match self.forget_chain(partner, slot) {
                Ok(forgot) => format!("{name} forgot {forgot}, and "),
                Err(why) => return why,
            },
            None => String::new(),
        };
        let mut member = self.team.kept_member(partner);
        if member.moves.len() >= MOVES_KNOWN {
            return format!("{name} already knows four moves: it must forget one first.");
        }
        member.moves.push(id);
        self.team.keep(partner, member);
        let remembered = if forgot.is_empty() { format!("{name} remembered") } else { format!("{forgot}remembered") };
        format!("Bzzzt... Zzzat! Success! {remembered} {}!", self.book.move_name(id))
    }

    /// The Link Shop: a move forgotten, with those linked to it (one move
    /// is always kept); what is told.
    pub fn forget_move(&mut self, partner: bool, slot: usize) -> String {
        let name = self.member_name(if partner { Whom::Partner } else { Whom::Leader });
        match self.forget_chain(partner, slot) {
            Ok(forgot) => format!("Bzzzt... Zzzat! Success! {name} forgot {forgot}."),
            Err(why) => why,
        }
    }

    /// The move at that slot forgotten, and those linked to it: their names,
    /// or why not (none would be left).
    fn forget_chain(&mut self, partner: bool, slot: usize) -> Result<String, String> {
        let mut member = self.team.kept_member(partner);
        let name = self.member_name(if partner { Whom::Partner } else { Whom::Leader });
        let chain = linked_chain(&member.moves, slot);
        if chain.is_empty() || chain.len() >= member.moves.len() {
            return Err(format!("{name} wouldn't have any moves left if it forgot that."));
        }
        let gone: Vec<String> = member.moves.drain(chain).map(|kept| self.book.move_name(move_id(kept))).collect();
        // (The first links to nothing.)
        if let Some(first) = member.moves.first_mut() {
            *first &= !LINKED_MOVE;
        }
        self.team.keep(partner, member);
        Ok(gone.join(" and "))
    }

    /// A member's move goes to another place in its order; the links stay
    /// where they were (the order a chain is used in changed, as the
    /// original's Switch).
    pub fn reorder_move(&mut self, partner: bool, from: usize, to: usize) {
        let mut member = self.team.kept_member(partner);
        if from < member.moves.len() && to < member.moves.len() {
            let (a, b) = (member.moves[from], member.moves[to]);
            member.moves[from] = move_id(b) | (a & LINKED_MOVE);
            member.moves[to] = move_id(a) | (b & LINKED_MOVE);
            self.team.keep(partner, member);
        }
    }

    /// The items the bag holds now: more as the rank rises (`services::BAG_SIZES`).
    pub fn bag_room(&self) -> usize {
        self.book.tables.jobs.bag_room(self.team.progress.rank)
    }

    /// The items Kangaskhan Storage holds now, by the rank (`ExplorerRank::storage`).
    pub fn storage_room(&self) -> usize {
        self.book.tables.jobs.storage_room(self.team.progress.rank)
    }

    /// The recruits who may come along now: two, three with the partner at home.
    pub fn along_room(&self) -> usize {
        ALONG + usize::from(self.team.progress.partner_home && !self.team.alone)
    }

    /// The recruits who come along: those picked at the guild's bell, the
    /// first two until then.
    pub fn along(&self) -> Vec<usize> {
        let progress = &self.team.progress;
        let mut along = progress.chosen.clone().unwrap_or_else(|| (0..progress.recruits.len().min(ALONG)).collect());
        along.truncate(self.along_room());
        along
    }

    /// The guild's bell: a recruit picked, or let go.
    pub fn pick_along(&mut self, at: usize) {
        let mut chosen = self.along();
        match chosen.iter().position(|&picked| picked == at) {
            Some(place) => _ = chosen.remove(place),
            None if chosen.len() < self.along_room() && at < self.team.progress.recruits.len() => chosen.push(at),
            None => {}
        }
        self.team.progress.chosen = Some(chosen);
    }

    /// Whether the partner stays at home (the original lets it once the
    /// story is over), or comes again; the recruits along cut to the room.
    pub fn keep_partner_home(&mut self, home: bool) {
        self.team.progress.partner_home = home && !self.team.alone;
        let along = self.along();
        self.team.progress.chosen = Some(along);
    }

    /// Who goes into the next dungeon: the leader, the partner unless it
    /// stays at home, the recruits along.
    pub fn team_size(&self) -> usize {
        let partner = !self.team.alone && !self.team.progress.partner_home;
        1 + usize::from(partner) + self.along().len()
    }

    /// A member's HP at most.
    pub fn member_hp(&self, whom: Whom) -> i32 {
        match whom {
            Whom::Leader => self.team.kept_member(false).stats.max_hp,
            Whom::Partner => self.team.kept_member(true).stats.max_hp,
            Whom::Recruit(at) => self.team.progress.recruits.get(at).map_or(0, |recruit| recruit.stats[1]),
        }
    }

    /// The moves a member knows, by id (a recruit's not kept yet: those its
    /// species knows at its level).
    pub fn member_moves(&self, whom: Whom) -> Vec<u16> {
        let moves = match whom {
            Whom::Leader => self.team.kept_member(false).moves,
            Whom::Partner => self.team.kept_member(true).moves,
            Whom::Recruit(at) => match self.team.progress.recruits.get(at) {
                Some(recruit) if recruit.moves.is_empty() => {
                    let setup = self.book.member_at(recruit.species, false, Some(recruit.stats[0]));
                    setup.map(|setup| setup.moves.iter().map(|slot| slot.id).collect()).unwrap_or_default()
                }
                Some(recruit) => recruit.moves.clone(),
                None => Vec::new(),
            },
        };
        moves.into_iter().map(move_id).filter(|&id| id != 0).collect()
    }

    /// The tactic a member follows (the leader: none of its own, 0).
    pub fn member_tactic(&self, whom: Whom) -> u8 {
        match whom {
            Whom::Leader => 0,
            whom => self.team.tactic_of(&self.member_name(whom)),
        }
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
    /// far, none on a floor laid out by hand (a boss's arena).
    pub fn fill_board(&mut self, outlaws: bool, open: &[u16], draw: &mut Draw) {
        let filled = if outlaws { &self.team.progress.outlaws } else { &self.team.progress.offers };
        if !filled.is_empty() {
            return;
        }
        let book = self.book.clone();
        let floors: Vec<(u16, u32)> =
            open.iter().map(|&dungeon| (dungeon, book.tables.dungeons.get(dungeon as usize).map_or(0, |dungeon| dungeon.floors.len() as u32))).collect();
        let mut offered = jobs::offers(&book.tables.jobs, &floors, outlaws, |dungeon, floor| book.floor_species(dungeon, floor), |below| draw.below(below));
        offered.retain(|job| {
            let floors = book.tables.dungeons.get(job.dungeon as usize).map(|dungeon| &dungeon.floors);
            floors.and_then(|floors| floors.get((job.floor as usize).saturating_sub(1))).is_some_and(|floor| floor.fixed.is_none())
        });
        match outlaws {
            true => self.team.progress.outlaws = offered,
            false => self.team.progress.offers = offered,
        }
    }

    /// A shop not visited since the day began restocks.
    pub fn restock(&mut self, shop: usize, chapter: usize, seed: u32) {
        if self.team.progress.stock[shop].is_none() {
            self.team.progress.stock[shop] = Some(self.book.shop_stock(shop == 1, chapter as i32, seed));
        }
    }

    /// A shop not visited since the day began restocks with those items
    /// (a game's own list, in place of the original's).
    pub fn restock_with(&mut self, shop: usize, items: &[u16]) {
        if self.team.progress.stock[shop].is_none() {
            self.team.progress.stock[shop] = Some(items.to_vec());
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

    /// Whether the team may stand sentry today: once a day, as the guild's.
    pub fn sentry_free(&self) -> bool {
        !self.team.progress.sentry_today
    }

    /// A sentry duty stood (`crate::sentry`): its points kept, the day's duty
    /// done, what the guild gives for it (`sentry_reward`).
    pub fn stand_sentry(&mut self, points: i32, rating: u8, draw: &mut Draw) -> Option<String> {
        (self.team.progress.sentry_points, self.team.progress.sentry_today) = (points, true);
        self.sentry_reward(rating, draw)
    }

    /// The partner leads, the leader becomes its partner (Chimecco Assembly,
    /// once the story is over): what each has become, its name, kept; both
    /// go. A recruit cannot lead: it is kept as a recruit, not as a member
    /// that grows (`Recruit`). Whether it was done (not in a dungeon, not
    /// alone).
    pub fn swap_leader(&mut self) -> bool {
        if self.dungeon.is_some() || self.team.alone {
            return false;
        }
        let team = &mut self.team;
        std::mem::swap(&mut team.leader, &mut team.partner);
        std::mem::swap(&mut team.progress.leader, &mut team.progress.partner);
        team.species = (team.species.1, team.species.0);
        let names = &mut team.progress.names;
        std::mem::swap(&mut names.0, &mut names.1);
        team.progress.partner_home = false;
        true
    }
}

/// The slots of the chain of linked moves the one at `slot` is in (none:
/// no such slot).
pub fn linked_chain(moves: &[u16], slot: usize) -> std::ops::Range<usize> {
    if slot >= moves.len() {
        return 0..0;
    }
    let linked = |at: usize| moves.get(at).is_some_and(|&kept| kept & LINKED_MOVE != 0);
    let mut first = slot;
    while first > 0 && linked(first) {
        first -= 1;
    }
    let mut end = slot + 1;
    while linked(end) {
        end += 1;
    }
    first..end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_forgotten_takes_the_moves_linked_to_it() {
        // (Moves 1, 2 linked to it, 3 alone, 4 linked to 3.)
        let moves = [1, 2 | LINKED_MOVE, 3, 4 | LINKED_MOVE];
        assert_eq!(linked_chain(&moves, 0), 0..2);
        assert_eq!(linked_chain(&moves, 1), 0..2);
        assert_eq!(linked_chain(&moves, 3), 2..4);
        assert_eq!(linked_chain(&[5], 0), 0..1);
        assert_eq!(linked_chain(&moves, 9), 0..0);
    }
}
