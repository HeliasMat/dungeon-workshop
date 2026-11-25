//! Hidden stairs, and the floors they lead to: the Secret Bazaar and the
//! Secret Room (fixed rooms 112 and 113 of the game).
//!
//! As the game, a floor may have them by its own chance
//! (`FloorExtras::hidden_chance`, likelier with the exclusive item for it),
//! where at least two floors are left; they lie hidden on a tile of a room
//! until the leader steps on them. They are drawn with a second random
//! generator of their own, as the game's are, so that the floors stay seed
//! for seed those of the generator. Their stairs lead on to the next floor.
//!
//! In the Bazaar, the game's five: Kirlia greets the team; Swalot sells
//! grab bags (an item of the floor's list), Shedinja shows the way out,
//! Mime Jr. heals the team (once a visit), Lickilicky cleans the sticky
//! items, each for the game's 100 Poké. In the Secret Room, the game's five
//! Deluxe Boxes stand for what they hold: items of the floor's list of
//! chests, lying there.

use crate::dungeon::{DungeonRng, Floor, Pos, stack};
use crate::services::BAG_CAPACITY;
use crate::tables::{Fixed, FloorExtras};

use super::dungeon::{Ask, Outcome};
use super::{Run, Side};

/// Where hidden stairs lead (`enum hidden_stairs_type` less one): the
/// place of its room among `Extras::secret_floors`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Secret {
    Bazaar,
    Room,
}

/// Hidden stairs on a floor: where, to what, whether found yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HiddenStairs {
    pub at: Pos,
    pub to: Secret,
    pub found: bool,
}

/// What one of the Bazaar does for the team.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bazaar {
    GrabBag,
    Escape,
    Spa,
    Clean,
}

/// Kirlia, Shedinja, Swalot, Mime Jr., Lickilicky.
const KIRLIA: u16 = 309;
const SHEDINJA: u16 = 320;
const SWALOT: u16 = 345;
const MIME_JR: u16 = 481;
const LICKILICKY: u16 = 505;
/// What each service costs (the game's, in its overlay 29).
pub const BAZAAR_PRICE: i32 = 100;
/// The chance the exclusive item for hidden stairs raises, as the game's
/// 1.2 (`HIDDEN_STAIRS_SPAWN_CHANCE_MULTIPLIER`, 8 fraction bits).
const MORE_HIDDEN_STAIRS_BOOST: i32 = 307;
/// `hidden_stairs_type` for either floor.
const EITHER: u8 = 255;
/// What the second generator mixes into the floor's seed.
const SECOND_SEED: u32 = 0x5EC2_E75A;

impl Secret {
    pub fn name(self) -> &'static str {
        match self {
            Secret::Bazaar => "Secret Bazaar",
            Secret::Room => "Secret Room",
        }
    }
}

impl Bazaar {
    /// What one of the Bazaar's offers, by its species (Kirlia none).
    pub fn of(species: u16) -> Option<Self> {
        match species {
            SWALOT => Some(Self::GrabBag),
            SHEDINJA => Some(Self::Escape),
            MIME_JR => Some(Self::Spa),
            LICKILICKY => Some(Self::Clean),
            _ => None,
        }
    }

    /// What it asks, short (what it offers is in the log, as the game says
    /// it).
    pub fn question(self) -> &'static str {
        match self {
            Self::GrabBag => "A grab bag: 100 Poké?",
            Self::Escape => "Escape: 100 Poké?",
            Self::Spa => "Heal the team: 100 Poké?",
            Self::Clean => "Clean items: 100 Poké?",
        }
    }
}

/// Where a floor's hidden stairs are, if it has some: its chance in 100
/// (`boosted` by the exclusive item), its kind, and a tile of a room not
/// taken by the stairs, the leader's start or what the generator placed.
pub(crate) fn hidden_stairs(floor: &Floor, seed: u32, extras: &FloorExtras, boosted: bool) -> Option<HiddenStairs> {
    let mut rng = DungeonRng::new(seed ^ SECOND_SEED);
    let chance = i32::from(extras.hidden_chance);
    let chance = if boosted { chance * MORE_HIDDEN_STAIRS_BOOST / 256 } else { chance };
    if chance <= 0 || rng.rand_int(100) >= chance {
        return None;
    }
    let to = match extras.hidden_kind {
        EITHER if rng.rand_int(2) == 0 => Secret::Bazaar,
        EITHER => Secret::Room,
        0 => Secret::Bazaar,
        _ => Secret::Room,
    };
    let taken = |tile: &Pos| {
        *tile == floor.stairs
            || *tile == floor.player_spawn
            || floor.item_spawns.contains(tile)
            || floor.trap_spawns.contains(tile)
            || floor.enemy_spawns.contains(tile)
    };
    let tiles: Vec<Pos> = Floor::positions().filter(|&tile| floor.is_spawn_tile(tile) && !taken(&tile)).collect();
    let at = *tiles.get(rng.rand_int(tiles.len() as i32) as usize)?;
    Some(HiddenStairs { at, to, found: false })
}

impl Run {
    /// The floor laid out by hand the team is on: a secret floor's, or the
    /// floor's own.
    pub(crate) fn fixed_room(&self) -> Option<&Fixed> {
        let dungeon = self.dungeon.as_ref()?;
        match dungeon.secret {
            Some(secret) => self.book.extras.secret_floors.get(secret as usize),
            None => dungeon.spec(&self.book)?.fixed.as_ref(),
        }
    }

    /// The leader comes onto hidden stairs: found, as the game says it, and
    /// whether to take them asked.
    pub(crate) fn step_on_hidden_stairs(&mut self) {
        let dungeon = self.here();
        let at = dungeon.leader().at;
        let Some(hidden) = dungeon.hidden_stairs.as_mut().filter(|hidden| hidden.at == at) else {
            return;
        };
        if !std::mem::replace(&mut hidden.found, true) {
            self.log.push("You've found the Hidden Stairs! An enticing scent lingers in the air...".to_string());
        }
        let dungeon = self.here();
        if dungeon.ask.is_none() {
            dungeon.ask = Some(Ask::HiddenStairs);
        }
    }

    /// Down the hidden stairs the leader agreed to take: to their floor, the
    /// floor's number kept (its stairs lead to the next).
    pub(crate) fn take_hidden_stairs(&mut self) {
        let dungeon = self.here();
        let at = dungeon.leader().at;
        let Some(hidden) = dungeon.hidden_stairs.filter(|hidden| hidden.at == at && std::mem::take(&mut dungeon.take_hidden)) else {
            dungeon.take_hidden = false;
            return;
        };
        if self.book.extras.secret_floors.get(hidden.to as usize).is_none() {
            return;
        }
        let dungeon = self.here();
        (dungeon.secret, dungeon.hidden_stairs, dungeon.spa_used) = (Some(hidden.to), None, false);
        self.new_floor();
        self.log.push(format!("The team arrived at the {}!", hidden.to.name()));
        let dungeon = self.here();
        let at = dungeon.leader().at;
        dungeon.stairs_seen = Some((dungeon.generation, at));
    }

    /// Walking into one of the Bazaar is talking to it: what it offers is
    /// asked. Whether it was one of them.
    pub(crate) fn talk_in_bazaar(&mut self, to: Pos) -> bool {
        let dungeon = self.here();
        if dungeon.secret != Some(Secret::Bazaar) {
            return false;
        }
        let Some(species) = dungeon.monsters.iter().find(|monster| monster.at == to && monster.side == Side::Keeper).and_then(|monster| monster.species) else {
            return false;
        };
        let said = match Bazaar::of(species) {
            Some(service) => {
                dungeon.ask = Some(Ask::Bazaar(service));
                match service {
                    Bazaar::GrabBag => "Swalot: Hello and welcome, Explorers! I've got some wonderful, wonderful grab bags here.",
                    Bazaar::Escape => "Shedinja: I'm Shedinja the Escape Guide. I know a secret way outta the dungeon...",
                    Bazaar::Spa => "Mime Jr.: Hee hee. Hello! This is Mime Jr.'s Spa.",
                    Bazaar::Clean => "Lickilicky: Bloo-bloo... You have itemth that have gotten thticky?",
                }
            }
            None if species == KIRLIA => "Kirlia: Congrats. You've found the Secret Bazaar! I'm Kirlia the Bazaar Guide. Feel free to explore the shops here.",
            None => return false,
        };
        self.log.push(said.to_string());
        true
    }

    /// What one of the Bazaar does, paid for.
    pub(crate) fn bazaar(&mut self, service: Bazaar) {
        let sticky: Vec<usize> = (0..self.bag.items.len()).filter(|&at| stack::sticky(self.bag.items[at])).collect();
        let refused = match service {
            _ if self.bag.money < BAZAAR_PRICE => Some("You don't have enough money..."),
            Bazaar::GrabBag if self.bag.items.len() >= BAG_CAPACITY => Some("Oh, too bad. Your Treasure Bag seems to be full."),
            Bazaar::Spa if self.here().spa_used => Some("Mime Jr.: I'm sorry, I'm too tired to continue today. Come back again."),
            Bazaar::Clean if sticky.is_empty() => Some("Lickilicky: Whaaaaa? You don't have thticky itemth in your bag."),
            _ => None,
        };
        if let Some(refused) = refused {
            self.log.push(refused.to_string());
            return;
        }
        self.bag.money -= BAZAAR_PRICE;
        match service {
            Bazaar::GrabBag => {
                let book = self.book.clone();
                let dungeon = self.here();
                let list = dungeon.id.and_then(|id| book.floor_extras(id, dungeon.floor_number)).map(|extras| &extras.grab_bag);
                let found = list.and_then(|list| list.pick(&mut dungeon.rng)).filter(|&item| item != crate::dungeon::status::item::POKE);
                match found {
                    Some(item) => {
                        let item = super::shops::piled(&book, item, &mut self.here().rng);
                        stack::add(&mut self.bag.items, item);
                        self.log.push(format!("The grab bag opened! Inside there was {}!", book.item_name(item)));
                    }
                    None => self.log.push("The grab bag opened! There was nothing inside...".to_string()),
                }
            }
            Bazaar::Escape => {
                self.log.push("Shedinja: Awright... Follow this secret path and you're outta here.".to_string());
                self.here().outcome = Some(Outcome::Escaped);
            }
            Bazaar::Spa => {
                let dungeon = self.here();
                dungeon.spa_used = true;
                for monster in dungeon.monsters.iter_mut().filter(|monster| monster.team() && monster.alive()) {
                    monster.stats.hp = monster.stats.max_hp;
                    monster.moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp);
                    if let Some(belly) = monster.belly.as_mut() {
                        belly.eat(200);
                    }
                }
                self.log.push("Mime Jr.'s healing beeeeam! HP, Belly, and PP were completely restored!".to_string());
            }
            Bazaar::Clean => {
                for at in sticky {
                    self.bag.items[at] = stack::stuck(self.bag.items[at], false);
                }
                self.log.push("Bloo-bloo-bloo... Phew... All clean. The stickiness came off the items!".to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dungeon::Tile;

    fn room() -> Floor {
        let mut floor = Floor::new_walled();
        for y in 4..10 {
            for x in 4..12 {
                floor.set(Pos::new(x, y), Tile::Floor { room: Some(0) });
            }
        }
        (floor.stairs, floor.player_spawn) = (Pos::new(5, 5), Pos::new(6, 6));
        floor
    }

    #[test]
    fn hidden_stairs_come_by_the_floor_s_chance_on_a_free_tile_of_a_room() {
        let floor = room();
        let extras = |kind, chance| FloorExtras { hidden_kind: kind, hidden_chance: chance, ..FloorExtras::default() };
        let found: Vec<HiddenStairs> = (0..1000).filter_map(|seed| hidden_stairs(&floor, seed, &extras(255, 15), false)).collect();
        assert!((100..200).contains(&found.len()), "{}", found.len());
        assert!(found.iter().all(|hidden| floor.is_spawn_tile(hidden.at) && hidden.at != floor.stairs && hidden.at != floor.player_spawn && !hidden.found));
        let bazaars = found.iter().filter(|hidden| hidden.to == Secret::Bazaar).count();
        assert!(bazaars > found.len() / 4 && bazaars < found.len() * 3 / 4, "{bazaars}");
        // The exclusive item for them: a fifth more.
        let boosted = (0..1000).filter(|&seed| hidden_stairs(&floor, seed, &extras(255, 15), true).is_some()).count();
        assert!(boosted > found.len(), "{boosted}");
        // None without a chance; always where it is 100; the kind asked.
        assert!((0..100).all(|seed| hidden_stairs(&floor, seed, &extras(0, 0), true).is_none()));
        assert!((0..100).all(|seed| hidden_stairs(&floor, seed, &extras(1, 100), false).is_some_and(|hidden| hidden.to == Secret::Room)));
    }

    #[test]
    fn the_bazaar_s_five_are_known_by_their_species() {
        assert_eq!(Bazaar::of(SWALOT), Some(Bazaar::GrabBag));
        assert_eq!(Bazaar::of(SHEDINJA), Some(Bazaar::Escape));
        assert_eq!(Bazaar::of(MIME_JR), Some(Bazaar::Spa));
        assert_eq!(Bazaar::of(LICKILICKY), Some(Bazaar::Clean));
        assert_eq!(Bazaar::of(KIRLIA), None);
    }
}
