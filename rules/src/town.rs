//! The town's other services, which scripts open through the game's own
//! menus: evolving at Luminous Spring, Spinda's Café (its Juice Bar, its
//! Recycle Shop and its lottery), Croagunk's Swap Shop, Xatu's appraisal of
//! treasure boxes, Chansey's Day Care.
//!
//! What they need of the rules is here (who evolves into what and how,
//! the Recycle Shop's trades, what a ticket or a box gives); the menus are
//! the game's, shown like the bank's (`services`).

use serde::{Deserialize, Serialize};

/// The menus of `message_Menu` this module provides (the game's numbers).
pub const LINK_SHOP: u16 = 20;
pub const EVOLUTION: u16 = 21;
pub const SWAP_SHOP: u16 = 23;
pub const APPRAISAL: u16 = 25;
pub const DAY_CARE: u16 = 27;
pub const JUICE_BAR: u16 = 29;
pub const RECYCLE_SHOP: u16 = 31;
pub const LOTTERY: u16 = 33;
pub const SOS_MAIL: u16 = 35;
/// The menus of names the story opens.
pub const MENU_PLAYER_NAME: u16 = 1;
pub const MENU_PARTNER_NAME: u16 = 2;
pub const MENU_TEAM_NAME: u16 = 4;
pub const MENUS: [u16; 9] = [LINK_SHOP, EVOLUTION, SWAP_SHOP, APPRAISAL, DAY_CARE, JUICE_BAR, RECYCLE_SHOP, LOTTERY, SOS_MAIL];

/// The second entries of `monster.md` (the females of the species that
/// have both) start here: `from` names the first.
pub(crate) const GENDERED: u16 = 600;

/// How a monster evolves (`monster.md`: the species it comes from, the
/// method and its two parameters).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evolution {
    pub from: u16,
    pub method: u8,
    pub param: u16,
    pub extra: u16,
}

/// `enum evolution_method`.
pub mod method {
    pub const NONE: u8 = 0;
    pub const LEVEL: u8 = 1;
    pub const IQ: u8 = 2;
    pub const ITEMS: u8 = 3;
    pub(crate) const RECRUITED: u8 = 4;
}

/// `enum evolution_requirement_additional`.
mod extra {
    pub(crate) const LINK_CABLE: u16 = 1;
    pub(crate) const ATTACK_ABOVE: u16 = 2;
    pub(crate) const ATTACK_BELOW: u16 = 3;
    pub(crate) const ATTACK_EQUAL: u16 = 4;
    pub(crate) const SUN_RIBBON: u16 = 5;
    pub(crate) const LUNAR_RIBBON: u16 = 6;
    pub(crate) const BEAUTY_SCARF: u16 = 7;
    pub(crate) const MALE: u16 = 10;
    pub(crate) const FEMALE: u16 = 11;
    pub(crate) const ANCIENT_POWER: u16 = 12;
    pub(crate) const ROLLOUT: u16 = 13;
    pub(crate) const DOUBLE_HIT: u16 = 14;
    pub(crate) const MIMIC: u16 = 15;
}

pub mod item {
    pub(crate) const BEAUTY_SCARF: u16 = 54;
    pub(crate) const SUN_RIBBON: u16 = 55;
    pub(crate) const LUNAR_RIBBON: u16 = 56;
    pub(crate) const LINK_CABLE: u16 = 151;
    pub(crate) const PRIZE_TICKET: u16 = 169;
    pub(crate) const SILVER_TICKET: u16 = 170;
    pub(crate) const GOLD_TICKET: u16 = 171;
    pub(crate) const PRISM_TICKET: u16 = 172;
    pub const TICKETS: [u16; 4] = [PRIZE_TICKET, SILVER_TICKET, GOLD_TICKET, PRISM_TICKET];
    /// The treasure boxes: twelve kinds of three each (`ITEM_GORGEOUS_BOX_1`
    /// to `ITEM_SINISTER_BOX_3`).
    pub(crate) const FIRST_BOX: u16 = 364;
    pub(crate) const LAST_BOX: u16 = 399;
    pub const POKE: u16 = 183;
}

/// What the one evolving is: as the conditions read it.
pub(crate) struct Evolving<'a> {
    pub level: i32,
    /// `None` for those whose IQ is not kept (recruits): it is not asked.
    pub iq: Option<i32>,
    pub attack: i32,
    pub defense: i32,
    /// Its entry's gender (`monster.md`: 1 male, 2 female).
    pub gender: u8,
    /// The moves it knows (empty when not kept: none is asked).
    pub moves: &'a [u16],
    /// The items in the bag, by id.
    pub bag: &'a [u16],
}

/// Whether one evolves this way: the items it uses up, or what it lacks.
pub(crate) fn requirement(evolution: &Evolution, who: &Evolving) -> Result<Vec<u16>, String> {
    let mut used = Vec::new();
    let has = |item: u16| who.bag.contains(&item);
    match evolution.method {
        method::LEVEL if who.level < evolution.param as i32 => return Err(format!("It must reach level {}.", evolution.param)),
        method::IQ if who.iq.is_some_and(|iq| iq < evolution.param as i32) => return Err(format!("It needs an IQ of {}.", evolution.param)),
        method::ITEMS if !has(evolution.param) => return Err("It needs a particular item.".to_string()),
        method::ITEMS => used.push(evolution.param),
        method::RECRUITED | method::NONE => return Err("This evolution does not happen here.".to_string()),
        _ => {}
    }
    let knows = |id: u16| who.moves.is_empty() || who.moves.contains(&id);
    let lacking = match evolution.extra {
        extra::LINK_CABLE if !has(item::LINK_CABLE) => Some("It needs a Link Cable."),
        extra::LINK_CABLE => {
            used.push(item::LINK_CABLE);
            None
        }
        extra::ATTACK_ABOVE if who.attack <= who.defense => Some("Its Attack must be higher than its Defense."),
        extra::ATTACK_BELOW if who.attack >= who.defense => Some("Its Defense must be higher than its Attack."),
        extra::ATTACK_EQUAL if who.attack != who.defense => Some("Its Attack and Defense must be equal."),
        extra::SUN_RIBBON if !has(item::SUN_RIBBON) => Some("It needs a Sun Ribbon."),
        extra::LUNAR_RIBBON if !has(item::LUNAR_RIBBON) => Some("It needs a Lunar Ribbon."),
        extra::BEAUTY_SCARF if !has(item::BEAUTY_SCARF) => Some("It needs a Beauty Scarf."),
        extra::MALE if who.gender != 1 => Some("Only a male evolves this way."),
        extra::FEMALE if who.gender != 2 => Some("Only a female evolves this way."),
        extra::ANCIENT_POWER if !knows(93) => Some("It must know Ancient Power."),
        extra::ROLLOUT if !knows(105) => Some("It must know Rollout."),
        extra::DOUBLE_HIT if !knows(487) => Some("It must know Double Hit."),
        extra::MIMIC if !knows(327) => Some("It must know Mimic."),
        _ => None,
    };
    match lacking {
        Some(lacking) => Err(lacking.to_string()),
        None => Ok(used),
    }
}

/// The stats once evolved: each grows by what the new species' base has
/// more (hit points, attack, defense, special attack, special defense).
pub(crate) fn evolved_stats(stats: [i32; 5], before: [i32; 5], after: [i32; 5]) -> [i32; 5] {
    std::array::from_fn(|at| (stats[at] + after[at] - before[at]).max(1))
}

/// One of the Recycle Shop's trades (`RECYCLE_SHOP_ITEM_LIST`, 24 of them).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecycleTrade {
    /// What is given.
    pub item: u16,
    /// 0: a trade for items; 1: for tickets; 2: an offer of the day.
    pub kind: i32,
    /// The guild rank and the number of recycles before it is offered.
    pub rank: i32,
    pub recycles: i32,
    /// The ticket that may come with it (its key: 5 Prize, 8 Silver, 13
    /// Gold, 20 Prism) and its chances out of 100.
    pub bonus: i16,
    pub bonus_odds: i16,
    /// What it costs: items and how many of each (0: any item).
    pub costs: Vec<(u16, u16)>,
}

impl RecycleTrade {
    /// The trades from the table's bytes (36 each).
    pub fn parse(bytes: &[u8]) -> Vec<Self> {
        let word = |b: &[u8], at: usize| i32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
        let half = |b: &[u8], at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
        bytes
            .as_chunks::<36>()
            .0
            .iter()
            .map(|b| Self {
                item: half(b, 0),
                kind: word(b, 4),
                rank: word(b, 8),
                recycles: word(b, 12),
                bonus: half(b, 16) as i16,
                bonus_odds: half(b, 18) as i16,
                costs: (0..4).map(|at| (half(b, 20 + at * 4), half(b, 22 + at * 4))).filter(|&(_, count)| count > 0).collect(),
            })
            .collect()
    }

    /// The ticket its bonus gives.
    pub(crate) fn bonus_ticket(&self) -> Option<u16> {
        match self.bonus {
            5 => Some(item::PRIZE_TICKET),
            8 => Some(item::SILVER_TICKET),
            13 => Some(item::GOLD_TICKET),
            20 => Some(item::PRISM_TICKET),
            _ => None,
        }
    }

    /// Takes what it costs from the bag (by id), if all is there: the
    /// places taken, highest first. Any item (0) is taken from those
    /// neither a ticket nor asked for by name.
    pub(crate) fn take_from(&self, bag: &[u16]) -> Option<Vec<usize>> {
        let mut taken: Vec<usize> = Vec::new();
        let named: Vec<u16> = self.costs.iter().map(|&(id, _)| id).filter(|&id| id != 0).collect();
        for &(id, count) in self.costs.iter().filter(|cost| cost.0 != 0).chain(self.costs.iter().filter(|cost| cost.0 == 0)) {
            for _ in 0..count {
                let fits = |at: &usize, held: &u16| match id {
                    0 => !item::TICKETS.contains(held) && !named.contains(held) && !taken.contains(at),
                    id => *held == id && !taken.contains(at),
                };
                taken.push(bag.iter().enumerate().find(|(at, held)| fits(at, held))?.0);
            }
        }
        taken.sort_unstable_by(|a, b| b.cmp(a));
        Some(taken)
    }
}

/// What a lottery ticket wins: the prize's rank (0 none, 1 to 3; the
/// better the ticket, the better the chances), from a roll of 0 to 99.
pub fn lottery(ticket: u16, roll: u32) -> u8 {
    // Chances out of 100 of each rank, best first.
    let odds: [u32; 3] = match ticket {
        item::PRISM_TICKET => [20, 50, 30],
        item::GOLD_TICKET => [8, 32, 50],
        item::SILVER_TICKET => [3, 17, 50],
        _ => [1, 9, 40],
    };
    let mut bound = 0;
    for (rank, chance) in odds.iter().enumerate() {
        bound += chance;
        if roll < bound {
            return 3 - rank as u8;
        }
    }
    0
}

/// Whether an item is a treasure box, and its kind (0 to 11) and its rank
/// (1 to 3).
pub fn treasure_box(id: u16) -> Option<(u16, u16)> {
    (item::FIRST_BOX..=item::LAST_BOX).contains(&id).then(|| ((id - item::FIRST_BOX) / 3, (id - item::FIRST_BOX) % 3 + 1))
}

/// What appraising a box costs.
pub const APPRAISAL_PRICE: i32 = 150;

/// Croagunk's Swap Shop: Exclusive Items put in its cauldron for another,
/// as the original's. From the Silks on they come in fours (its
/// `GetFirstExclusivePrerequisite`): two of one star (a Claw, a Fang), one
/// of two, one of three; the two of one star swap for the one of two, those
/// three for the one of three ("the easy way"); Croagunk's items of the day
/// are each had for so many Exclusive Items, whatever they are. (Its
/// `SYNTH/synth.bin`, which weighs and counts them, is not extracted: the
/// counts below are the ones it has, by star, read from the ROM; the weights
/// its pattern.)
pub mod swap {
    use crate::dungeon::exclusive;

    /// What a swap costs, whichever it is (overlay 21's).
    pub const PRICE: i32 = 150;
    /// Croagunk's items of the day, at most.
    pub const WARES: usize = 8;
    /// The first Exclusive Item of the fours (the White Silk).
    pub const FIRST_SET: u16 = 506;
    /// The first of the fours made for a species (the Bulba-Claw): before
    /// it, those for a type, which Croagunk never offers of the day.
    pub const FIRST_SPECIES_SET: u16 = 574;

    /// Whether an item is an Exclusive Item.
    pub fn exclusive(item: u16) -> bool {
        (exclusive::FIRST..exclusive::END).contains(&item)
    }

    /// Its stars (1 to 3): by its place in its four; those before the
    /// fours (the Prism Ruff…) are of three.
    pub fn stars(item: u16) -> u8 {
        match item.checked_sub(FIRST_SET).map(|place| place % 4) {
            Some(0 | 1) => 1,
            Some(2) => 2,
            _ => 3,
        }
    }

    /// The set it is had for the easy way: the two of one star of its four
    /// for the one of two; those and the one of two for the one of three.
    pub fn set_for(item: u16) -> Option<Vec<u16>> {
        let first = item.checked_sub(FIRST_SET).filter(|_| exclusive(item)).map(|place| item - place % 4)?;
        match item - first {
            2 => Some(vec![first, first + 1]),
            3 => Some(vec![first, first + 1, first + 2]),
            _ => None,
        }
    }

    /// The Exclusive Items an item of the day is had for (`num_to_trade`).
    pub fn to_trade(item: u16) -> usize {
        match stars(item) {
            1 => 2,
            2 => 3,
            _ => 5,
        }
    }

    /// How often it is of the day: the rarer, the less.
    pub fn weight(item: u16) -> usize {
        match stars(item) {
            1 => 16,
            2 => 8,
            _ => 4,
        }
    }

    /// The Exclusive Items taken for an item of the day, among those held
    /// (by id): `count` of them, the fewest stars first, never the one
    /// wanted; none if there are not enough. Their places in `held`.
    pub fn any_taken(held: &[u16], wanted: u16, count: usize) -> Option<Vec<usize>> {
        let mut fitting: Vec<usize> = (0..held.len()).filter(|&at| exclusive(held[at]) && held[at] != wanted).collect();
        fitting.sort_by_key(|&at| (stars(held[at]), held[at]));
        fitting.truncate(count);
        (fitting.len() == count).then_some(fitting)
    }

    /// The items of a set taken from those held (by id): their places, if all are there.
    pub fn set_taken(held: &[u16], set: &[u16]) -> Option<Vec<usize>> {
        let mut taken: Vec<usize> = Vec::new();
        for &item in set {
            taken.push((0..held.len()).find(|at| held[*at] == item && !taken.contains(at))?);
        }
        Some(taken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evolving_asks_its_level_and_its_items() {
        let who = |level, bag: &'static [u16]| Evolving { level, iq: None, attack: 10, defense: 12, gender: 1, moves: &[], bag };
        let level = Evolution { from: 4, method: method::LEVEL, param: 16, extra: 0 };
        assert!(requirement(&level, &who(15, &[])).is_err());
        assert_eq!(requirement(&level, &who(16, &[])), Ok(vec![]));
        let stone = Evolution { from: 25, method: method::ITEMS, param: 146, extra: 0 };
        assert!(requirement(&stone, &who(5, &[1])).is_err());
        assert_eq!(requirement(&stone, &who(5, &[146])), Ok(vec![146]));
        let trade = Evolution { from: 64, method: method::LEVEL, param: 1, extra: extra::LINK_CABLE };
        assert_eq!(requirement(&trade, &who(5, &[item::LINK_CABLE])), Ok(vec![item::LINK_CABLE]));
        let tyrogue = Evolution { from: 236, method: method::LEVEL, param: 20, extra: extra::ATTACK_ABOVE };
        assert!(requirement(&tyrogue, &who(20, &[])).is_err());
        assert_eq!(evolved_stats([30, 10, 10, 10, 10], [40, 50, 50, 50, 50], [60, 70, 50, 50, 50]), [50, 30, 10, 10, 10]);
    }

    #[test]
    fn recycling_takes_what_it_costs() {
        let trade = RecycleTrade { item: 1, costs: vec![(5, 1), (0, 2)], ..Default::default() };
        assert_eq!(trade.take_from(&[5, item::PRIZE_TICKET, 7]), None);
        assert_eq!(trade.take_from(&[5, item::PRIZE_TICKET, 7, 8]), Some(vec![3, 2, 0]));
        assert_eq!(lottery(item::PRIZE_TICKET, 0), 3);
        assert_eq!(lottery(item::PRIZE_TICKET, 99), 0);
        assert_eq!(treasure_box(366), Some((0, 3)));
    }

    #[test]
    fn croagunk_swaps_a_four_s_lesser_items_for_its_rarer_ones() {
        // (The Bulba-Claw, -Fang, Grass-Guard, Leafy Tie.)
        assert_eq!([574, 575, 576, 577].map(swap::stars), [1, 1, 2, 3]);
        assert_eq!(swap::set_for(576), Some(vec![574, 575]));
        assert_eq!(swap::set_for(577), Some(vec![574, 575, 576]));
        assert_eq!((swap::set_for(574), swap::set_for(444)), (None, None));
        assert_eq!([574, 576, 577, 444].map(swap::to_trade), [2, 3, 5, 5]);
        assert_eq!(swap::set_taken(&[1, 575, 574], &[574, 575]), Some(vec![2, 1]));
        assert_eq!(swap::set_taken(&[574, 1], &[574, 575]), None);
        // (Any two Exclusive Items, the fewest stars first, never the one wanted.)
        assert_eq!(swap::any_taken(&[577, 1, 575, 600, 576], 600, 2), Some(vec![2, 4]));
        assert_eq!(swap::any_taken(&[577, 1], 600, 2), None);
    }
}
