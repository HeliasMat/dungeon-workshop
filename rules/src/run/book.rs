//! What the game is made of, read once from the project's files: every
//! table, name and phrase the rules count with. No pictures, no sounds:
//! those are the shell's and the game's.

use std::sync::Arc;

use crate::dungeon::{Floor as Map, MoveEffect, MoveSlot, Pos, Stats, Tile, stack};
use crate::strings::UiStrings;
use crate::tables::{Fixed, Species, Tables};

use super::team::Growing;

/// Stats used where the tables give none.
pub(crate) const PLAYER_STATS: Stats = Stats {
    level: 5,
    max_hp: 40,
    hp: 40,
    attack: 14,
    defense: 9,
    sp_attack: 14,
    sp_defense: 9,
    types: (0, 0),
    abilities: [0, 0],
    held: 0,
    iq: 1,
    iq_group: 0,
    iq_skills: 0,
    female: false,
};
pub(crate) const ENEMY_STATS: Stats = Stats {
    level: 3,
    max_hp: 16,
    hp: 16,
    attack: 9,
    defense: 6,
    sp_attack: 9,
    sp_defense: 6,
    types: (0, 0),
    abilities: [0, 0],
    held: 0,
    iq: 1,
    iq_group: 0,
    iq_skills: 0,
    female: false,
};

/// A team member as its species starts.
pub struct MemberSetup {
    pub name: String,
    pub stats: Stats,
    pub moves: Vec<MoveSlot>,
    pub movement: u8,
    pub growth: Growing,
}

/// How many chapters of the story a shop's list lasts.
const CHAPTERS_PER_STAGE: i32 = 6;
/// The chance in 100, for each place, of an item of the next list.
const RARE_FIND: i32 = 10;

pub struct Book {
    pub tables: Tables,
    /// The game's words, when there are any.
    pub strings: Option<Arc<UiStrings>>,
}

impl Book {
    /// The game from the bytes of its `dungeons.bin`.
    pub fn read(bytes: &[u8]) -> Option<Self> {
        let mut tables = bincode::deserialize::<Tables>(bytes).ok()?;
        crate::dojo::add_mazes(&mut tables);
        if tables.dungeons.len() > crate::dojo::FINAL as usize {
            tables.playable.extend(crate::dojo::FIRST..=crate::dojo::FINAL);
        }
        Some(Self { tables, strings: None })
    }

    pub fn species(&self, monster: u16) -> Option<&Species> {
        self.tables.species.get(monster as usize)
    }

    pub fn species_name(&self, monster: u16) -> String {
        self.species(monster).map_or(String::new(), |species| species.name.clone())
    }

    pub fn regular_attack(&self) -> MoveSlot {
        self.tables.regular_attack.unwrap_or_else(MoveSlot::fallback_regular_attack)
    }

    /// A move by its number, as a team member knows it.
    pub fn move_slot(&self, id: u16) -> Option<MoveSlot> {
        self.tables.move_slots.get(id as usize).copied()
    }

    pub fn growing(&self, monster: u16) -> Growing {
        match self.species(monster) {
            Some(species) => Growing {
                table: species.growth.clone().into(),
                learnset: Arc::new(species.learnset.clone()),
                machines: Arc::new(species.machines.iter().filter_map(|&id| self.move_slot(id)).collect()),
            },
            None => Growing::default(),
        }
    }

    pub fn member(&self, monster: u16, partner: bool) -> Option<MemberSetup> {
        let species = self.species(monster)?;
        let (stats, moves) = if partner { species.partner.clone()? } else { species.hero.clone()? };
        Some(MemberSetup { name: species.name.clone(), stats, moves, movement: species.movement, growth: self.growing(monster) })
    }

    /// A team member of any species, at a level (none: as it starts): as the
    /// game starts it if it can, else as the game's dungeons meet it; given a
    /// level, its moves those its species knows by then (the last four).
    pub fn member_at(&self, monster: u16, partner: bool, level: Option<i32>) -> Option<MemberSetup> {
        let species = self.species(monster)?;
        let mut setup = match self.member(monster, partner) {
            Some(setup) => setup,
            None => {
                let enemy = crate::own::met(&self.tables, monster, level.unwrap_or(5))?;
                MemberSetup { name: species.name.clone(), stats: enemy.stats, moves: Vec::new(), movement: species.movement, growth: self.growing(monster) }
            }
        };
        let Some(level) = level.or_else(|| setup.moves.is_empty().then_some(setup.stats.level)) else {
            return Some(setup);
        };
        setup.stats = crate::own::at_level(setup.stats, &species.growth, level);
        let known: Vec<MoveSlot> = species.learnset.iter().filter(|(when, _)| i32::from(*when) <= level).map(|(_, slot)| *slot).collect();
        if !known.is_empty() {
            setup.moves = known[known.len().saturating_sub(4)..].to_vec();
        }
        Some(setup)
    }

    pub fn weight(&self, monster: u16) -> i32 {
        self.species(monster).map_or(0x100, |species| species.weight as i32)
    }

    pub fn family(&self, monster: u16) -> u16 {
        self.species(monster).map_or(monster, |species| species.family)
    }

    /// The fewest and the most a pile of the item holds.
    pub(crate) fn item_pile(&self, item: u16) -> (u16, u16) {
        let (fewest, most) = self.tables.item_piles.get(item as usize).copied().unwrap_or((1, 1));
        (fewest.max(1) as u16, most.max(fewest).max(1) as u16)
    }

    /// The move an item uses (an orb's); 0 for none.
    pub fn item_move(&self, item: u16) -> u16 {
        self.tables.item_moves.get(item as usize).copied().unwrap_or(0)
    }

    /// What an item costs in a shop, and what a shop gives for it.
    pub fn item_prices(&self, item: u16) -> (i32, i32) {
        let (buy, sell) = self.tables.item_prices.get(stack::id(item) as usize).copied().unwrap_or_default();
        let count = stack::count(item) as i32;
        (buy * count, sell * count)
    }

    pub(crate) fn recruit_boost(&self, level: i32) -> i32 {
        self.tables.recruit_boosts.get(level.max(0) as usize).copied().unwrap_or(0)
    }

    /// What a shop has for sale (`wares`: the small shop of rarer goods) at
    /// this chapter of the story, drawn with `seed`: different items of
    /// the chapter's list, each with a price; now and then one of the next
    /// chapter's list as well. Sorted by item.
    pub(crate) fn shop_stock(&self, wares: bool, chapter: i32, seed: u32) -> Vec<u16> {
        let lists = self.tables.shops.get(wares as usize).map_or(&[][..], |lists| &lists[..]);
        let stage = (chapter.max(0) / CHAPTERS_PER_STAGE) as usize;
        let goods = |stage: usize| lists.get(stage.min(lists.len().saturating_sub(1))).map_or(Vec::new(), |list| self.priced(list));
        let mut rng = crate::dungeon::DungeonRng::new(seed);
        draw_stock(&mut rng, goods(stage), goods(stage + 1), if wares { 4 } else { 8 })
    }

    /// The items of a list a shop can sell: those it may hold at all, with a
    /// price.
    fn priced(&self, list: &crate::dungeon::ItemTable) -> Vec<u16> {
        let open = |category: u8| list.categories.iter().any(|&(of, weight)| of == category && weight != 0 && weight != u16::MAX);
        let mut items: Vec<u16> =
            list.items.iter().filter(|&&(id, category, weight)| weight != 0 && open(category) && self.item_prices(id).0 > 0).map(|&(id, ..)| id).collect();
        items.sort_unstable();
        items.dedup();
        items
    }

    /// The species a monster may evolve into (by their first entry).
    pub fn evolutions(&self, monster: u16) -> Vec<u16> {
        let species = &self.tables.species;
        let from = monster % crate::town::GENDERED;
        (1..species.len().min(crate::town::GENDERED as usize) as u16).filter(|&into| from != 0 && species[into as usize].evolution.from == from).collect()
    }

    /// The monsters living on a floor of a dungeon.
    pub fn floor_species(&self, dungeon: u16, floor: u32) -> Vec<u16> {
        let floor = self.tables.dungeons.get(dungeon as usize).and_then(|dungeon| dungeon.floors.get(floor.saturating_sub(1) as usize));
        floor.map_or(Vec::new(), |floor| floor.enemies.iter().filter(|enemy| enemy.weight > 0).map(|enemy| enemy.species).collect())
    }

    pub fn dungeon_name(&self, id: u16) -> String {
        match self.tables.dungeon_names.get(id as usize) {
            Some(name) if !name.is_empty() => name.clone(),
            _ => format!("Dungeon {id}"),
        }
    }

    fn named(names: &[String], id: usize, kind: &str) -> String {
        match names.get(id) {
            Some(name) if !name.is_empty() => name.clone(),
            _ => format!("{kind} {id}"),
        }
    }

    pub(crate) fn iq_skill_name(&self, skill: u8) -> String {
        Self::named(&self.tables.iq_skills, skill as usize, "IQ skill")
    }

    pub fn move_name(&self, id: u16) -> String {
        Self::named(&self.tables.moves, id as usize, "move")
    }

    /// An item's name, with how many its pile holds.
    pub fn item_name(&self, value: u16) -> String {
        let name = Self::named(&self.tables.items, stack::id(value) as usize, "item");
        match stack::piles(value) && stack::count(value) > 1 {
            true => format!("{} {name}", stack::count(value)),
            false if stack::sticky(value) => format!("{name} (sticky)"),
            false => name,
        }
    }

    /// A message of the game's text, its tags filled in (`[string:0]`,
    /// `[move:0]`...), without its colour tags. `None` without the text.
    pub(crate) fn phrase(&self, key: usize, parts: &[(&str, &str)]) -> Option<String> {
        let text = self.strings.as_ref()?.log.get(key).filter(|text| !text.is_empty())?;
        let mut text = text.replace('\n', " ");
        for (tag, value) in parts {
            text = text.replace(&format!("[{tag}]"), value);
        }
        while let Some(start) = text.find("[C") {
            let Some(end) = text[start..].find(']') else { break };
            text.replace_range(start..start + end + 1, "");
        }
        Some(text.trim().to_string())
    }

    /// A status or a stat change as the game says it, where it has the
    /// words.
    pub(crate) fn effect_phrase(&self, effect: MoveEffect, name: &str) -> Option<String> {
        use crate::strings::log_text::*;
        let stat = |which: usize, by: i32| {
            let stat = self.strings.as_ref()?.log.get(which).filter(|text| !text.is_empty())?.clone();
            let key = match by {
                ..=-2 => STAT_HARSHLY_FELL,
                -1 => STAT_FELL,
                1 => STAT_ROSE,
                _ => STAT_SHARPLY_ROSE,
            };
            self.phrase(key, &[("string:0", name), ("string:1", &stat)])
        };
        let status = |key: usize| self.phrase(key, &[("string:0", name)]);
        match effect {
            MoveEffect::UserAttack(by) | MoveEffect::TargetAttack(by) => stat(ATTACK, by),
            MoveEffect::UserDefense(by) | MoveEffect::TargetDefense(by) => stat(DEFENSE, by),
            MoveEffect::UserSpAttack(by) | MoveEffect::TargetSpAttack(by) => stat(SP_ATTACK, by),
            MoveEffect::UserSpDefense(by) | MoveEffect::TargetSpDefense(by) => stat(SP_DEFENSE, by),
            MoveEffect::TargetAccuracy(by) => stat(ACCURACY, by),
            MoveEffect::TargetEvasion(by) | MoveEffect::UserEvasion(by) => stat(EVASION, by),
            MoveEffect::Poison => status(POISONED),
            MoveEffect::BadlyPoison => status(BADLY_POISONED),
            MoveEffect::Paralyze => status(PARALYZED),
            MoveEffect::Confuse => status(CONFUSED),
            MoveEffect::Sleep => status(FELL_ASLEEP),
            MoveEffect::Freeze => status(FROZEN),
            MoveEffect::Cringe => status(CRINGED),
            _ => None,
        }
    }

    /// "X woke up!", as the game says it.
    pub(crate) fn woke_up(&self, name: &str) -> String {
        self.phrase(crate::strings::log_text::WOKE_UP, &[("string:0", name)]).unwrap_or_else(|| format!("{name} woke up!"))
    }

    /// "X fainted!", as the game says it.
    pub(crate) fn knocked_out(&self, name: &str) -> String {
        self.phrase(crate::strings::log_text::FAINTED, &[("string:1", name)]).unwrap_or_else(|| format!("{name} fainted!"))
    }
}

/// Where a floor laid out by hand starts on the map.
const FIXED_CORNER: (i32, i32) = (5, 5);

impl Fixed {
    pub fn tile(at: (usize, usize)) -> Pos {
        Pos::new(FIXED_CORNER.0 + at.0 as i32, FIXED_CORNER.1 + at.1 as i32)
    }

    /// The floor: walls all around the room. One not left with its enemies
    /// has its way on at the far corner from the leader.
    pub fn floor(&self) -> Map {
        let mut floor = Map::new_walled();
        let mut open = Vec::new();
        for (index, kind) in self.tiles.iter().enumerate() {
            let at = Self::tile((index % self.width.max(1), index / self.width.max(1)));
            match kind {
                0 => {
                    floor.set(at, Tile::Floor { room: Some(0) });
                    open.push(at);
                }
                2 => floor.set(at, Tile::Secondary),
                _ => {}
            }
        }
        floor.player_spawn = self.leader.map(Self::tile).or(open.first().copied()).unwrap_or(Pos::new(0, 0));
        let far = |tile: &&Pos| (tile.x - floor.player_spawn.x).abs() + (tile.y - floor.player_spawn.y).abs();
        let exit = self.exit.map(Self::tile).or(open.iter().max_by_key(far).copied());
        if !self.ends_with_enemies
            && let Some(stairs) = exit
        {
            floor.stairs = stairs;
            floor.set(stairs, Tile::Stairs { room: 0 });
        }
        floor
    }
}

/// `places` different items, mostly of `here`, now and then of `ahead`
/// (all of `ahead` once `here` runs out), sorted.
fn draw_stock(rng: &mut crate::dungeon::DungeonRng, mut here: Vec<u16>, mut ahead: Vec<u16>, places: usize) -> Vec<u16> {
    let mut stock = Vec::new();
    while stock.len() < places && !(here.is_empty() && ahead.is_empty()) {
        let rare = !ahead.is_empty() && (here.is_empty() || rng.rand_int(100) < RARE_FIND);
        let from = if rare { &mut ahead } else { &mut here };
        let item = from.swap_remove(rng.rand_int(from.len() as i32) as usize);
        here.retain(|&other| other != item);
        ahead.retain(|&other| other != item);
        stock.push(item);
    }
    stock.sort_unstable();
    stock
}

#[cfg(test)]
mod stock_tests {
    use super::*;

    #[test]
    fn a_shop_sells_different_items_mostly_of_its_stage() {
        let mut rng = crate::dungeon::DungeonRng::new(4);
        let (here, ahead): (Vec<u16>, Vec<u16>) = ((1..=20).collect(), (15..=40).collect());
        let mut rare = 0;
        for _ in 0..200 {
            let stock = draw_stock(&mut rng, here.clone(), ahead.clone(), 8);
            assert_eq!(stock.len(), 8);
            assert!(stock.windows(2).all(|pair| pair[0] < pair[1]));
            rare += stock.iter().filter(|&&item| item > 20).count();
        }
        // About one place in ten comes from the next list.
        assert!((80..260).contains(&rare), "{rare}");
        // Too few items: all of them, the next list's after.
        assert_eq!(draw_stock(&mut rng, vec![3, 1], vec![], 8), vec![1, 3]);
        assert_eq!(draw_stock(&mut rng, vec![], vec![7], 4), vec![7]);
    }
}
