//! The game played through a film, whole, kept from one scene, one dungeon,
//! one save to the next: the rules' own (`rules::run::Run`: the team and
//! what it has become, its recruits, its bag, its money, the bank, the jobs,
//! the shops' stock), and the things of one's own the game has no item for
//! (a letter, a map).
//!
//! It is what is true: the film reads it through the numbers it is given
//! (`bag_<word>` and `bag_name_<word>`, `money`, `hero_level`,
//! `partner_level`, `recruits`, the team said in `game_team`), and
//! changes it through the game (`team`, `bag`, `getmoney`, `getitem`,
//! `takeitem`, a dungeon gone through).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use assets::project::Project;
use rules::own::plain;
use rules::run::Run;
use rules::run::options::Options;
use rules::run::saves::Place;
use rules::run::town::Whom;

use crate::dive::{Carried, bag_number};

/// How a save writes the things of one's own: `own=Letter:1,Map:2`.
const OWN_LINE: &str = "own=";

/// The numbers and the words a film is given by its game.
pub type Given = (Vec<(String, f64)>, Vec<(String, String)>);

/// The game played through a film.
pub struct Playthrough {
    /// The rules' game (away while a dungeon is gone through by it).
    run: Option<Run>,
    /// The things of one's own the game has no item for, and how many.
    pub own: Vec<(String, u16)>,
    /// The dungeons of one's own the rules know (`library/dungeons`), by
    /// their words, at the number they have there: the same from one game
    /// to the next while the files are the same (the jobs name them so).
    pub dungeons: Vec<(String, u16)>,
}

/// What a bag holds, by the items' names, how many of each.
pub fn bag_of(run: &Run) -> Vec<(String, u16)> {
    use rules::dungeon::stack;
    let mut items: Vec<(String, u16)> = Vec::new();
    for &value in &run.bag().items {
        let id = stack::id(value);
        let name = run.book().tables.items.get(id as usize).cloned().unwrap_or_default();
        let many = if stack::piles(id) { stack::count(value).max(1) } else { 1 };
        match items.iter_mut().find(|(known, _)| *known == name) {
            Some((_, count)) => *count += many,
            None => items.push((name, many)),
        }
    }
    items
}

impl Playthrough {
    /// A new game, its team as the game starts it.
    pub fn new(data: &str, project: &Project) -> Result<Self, crate::Error> {
        let mut playthrough =
            Playthrough { run: Some(Run::new(Arc::new(crate::book::read(data)?), Options::default())), own: Vec::new(), dungeons: Vec::new() };
        playthrough.know_dungeons(project);
        Ok(playthrough)
    }

    /// Every dungeon of one's own given to the rules, in the order of their words.
    fn know_dungeons(&mut self, project: &Project) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        // (Its own and the shared library's.)
        for word in project.names("dungeons", "dungeon") {
            let Ok(text) = std::fs::read_to_string(project.dungeon(&word)) else {
                continue;
            };
            let Ok(own) = rules::own::read(&crate::dive::resolved_for(project, &text), &run.book.tables) else {
                continue;
            };
            let Some(book) = Arc::get_mut(&mut run.book) else {
                return;
            };
            let id = rules::own::add(&mut book.tables, own.name, own.table, &own.ranks);
            rules::own::retype(&mut book.tables, &own.species_types, &own.move_types);
            self.dungeons.push((word, id));
        }
    }

    /// The number the rules know a dungeon of one's own by.
    pub fn dungeon_id(&self, word: &str) -> Option<u16> {
        self.dungeons.iter().find(|(known, _)| known == word).map(|(_, id)| *id)
    }

    /// The dungeons of one's own that have floors before their fight (jobs
    /// go there): of those words only, if it says (the dungeons the story
    /// has opened).
    pub fn open_dungeons(&self, only: Option<&BTreeSet<String>>) -> Vec<u16> {
        let Some(run) = self.run.as_ref() else {
            return Vec::new();
        };
        let tables = &run.book.tables;
        self.dungeons
            .iter()
            .filter(|(word, _)| only.is_none_or(|only| only.contains(word)))
            .map(|(_, id)| *id)
            .filter(|id| tables.dungeons.get(*id as usize).is_some_and(|dungeon| dungeon.floors.iter().any(|floor| floor.fixed.is_none())))
            .collect()
    }

    /// The rules' game, to change it (none while a dungeon is gone through).
    pub fn run_mut(&mut self) -> Option<&mut Run> {
        self.run.as_mut()
    }

    /// The rules' game (none while a dungeon is gone through).
    pub fn run(&self) -> Option<&Run> {
        self.run.as_ref()
    }

    /// The rules' game, lent to a dungeon; given back by `put_back`.
    pub fn lend(&mut self) -> Option<Run> {
        self.run.take()
    }

    /// The game back from a dungeon, out of it.
    pub fn put_back(&mut self, mut run: Run) {
        if run.dungeon().is_some() {
            run.leave_dungeon();
        }
        self.run = Some(run);
    }

    /// The team: those two species (its leader, its partner); changed, they
    /// come in at the level the leader had. The same: as it is.
    pub fn team(&mut self, hero: u16, partner: u16) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        if run.team().species != (hero, partner) {
            let level = run.member_level(Whom::Leader);
            run.change_team_at(hero, partner, Some(level));
        }
    }

    /// The team brought to that level (what it learnt kept if it is already there).
    pub fn level(&mut self, level: i32) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        if run.member_level(Whom::Leader) != level {
            let (hero, partner) = run.team().species;
            run.change_team_at(hero, partner, Some(level));
        }
    }

    /// So many of a thing given (or taken, below 0), by its name: an item of
    /// the game into the bag, else a thing of one's own.
    pub fn give(&mut self, name: &str, count: i32) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let item = run.book().tables.items.iter().position(|item| !item.is_empty() && plain(item) == plain(name)).map(|item| item as u16);
        match item {
            Some(item) if count >= 0 => run.give_items(item, count as u16),
            Some(item) => run.take_items(item, (-count) as u16),
            None => {
                let at = self.own.iter().position(|(known, _)| plain(known) == plain(name));
                let had = at.map_or(0, |at| i32::from(self.own[at].1));
                let now = (had + count).clamp(0, i32::from(u16::MAX)) as u16;
                match at {
                    Some(at) => self.own[at].1 = now,
                    None if now > 0 => self.own.push((name.to_string(), now)),
                    None => {}
                }
                self.own.retain(|(_, count)| *count > 0);
            }
        }
    }

    /// An item of the game by its number given (or taken).
    pub fn give_item(&mut self, item: u16, count: u16, take: bool) {
        if let Some(run) = self.run.as_mut() {
            match take {
                true => run.take_items(item, count),
                false => run.give_items(item, count),
            }
        }
    }

    /// A Pokémon of that species joins the team's recruits, as the rules
    /// make one (its level and stats its species' own); kept in the save.
    pub fn recruit(&mut self, species: u16) {
        if let Some(run) = self.run.as_mut() {
            run.recruit_species(species);
        }
    }

    /// Money given (or taken).
    pub fn money(&mut self, amount: i32) {
        if let Some(run) = self.run.as_mut() {
            run.add_money(amount);
        }
    }

    /// What the bag holds, the game's items and one's own, by their names.
    pub fn carried(&self) -> Carried {
        let mut items = self.run.as_ref().map(bag_of).unwrap_or_default();
        items.extend(self.own.iter().cloned());
        Carried { items, money: self.run.as_ref().map_or(0, |run| run.bag().money) }
    }

    /// The numbers and words the film is given: the bag, the money, the
    /// team's levels, its recruits, the team said.
    pub fn numbers(&self) -> Given {
        let (mut numbers, mut words) = (Vec::new(), Vec::new());
        let Some(run) = self.run.as_ref() else {
            return (numbers, words);
        };
        let carried = self.carried();
        for (name, count) in &carried.items {
            let var = bag_number(name);
            numbers.push((var.clone(), f64::from(*count)));
            words.push((format!("bag_name_{}", var.strip_prefix("bag_").unwrap_or(&var)), name.clone()));
        }
        numbers.push((scenes::vocabulary::MONEY.to_string(), f64::from(carried.money)));
        numbers.push(("hero_level".to_string(), f64::from(run.member_level(Whom::Leader))));
        numbers.push(("partner_level".to_string(), f64::from(run.member_level(Whom::Partner))));
        numbers.push(("recruits".to_string(), run.team().progress.recruits.len() as f64));
        numbers.push(("game_played".to_string(), 1.0));
        let said: Vec<String> = run.members().into_iter().map(|whom| format!("{} Lv. {}", run.member_name(whom), run.member_level(whom))).collect();
        words.push(("game_team".to_string(), said.join(", ")));
        (numbers, words)
    }

    /// The film's numbers and words as the game has them: what it gives set,
    /// a thing no longer in the bag at 0.
    pub fn into_film(&self, numbers: &mut BTreeMap<String, f64>, texts: &mut HashMap<String, String>) {
        let (given, words) = self.numbers();
        for (name, count) in numbers.iter_mut() {
            if name.starts_with("bag_") {
                *count = 0.0;
            }
        }
        numbers.extend(given);
        texts.extend(words);
    }

    /// As a save writes it: the rules' save, and the things of one's own.
    pub fn written(&self) -> String {
        self.written_at(&Place { event: None, town: None, numbers: BTreeMap::new(), texts: BTreeMap::new(), checkpoint: None })
    }

    /// As a game's save writes it, where its story stands (`crate::story`).
    pub fn written_at(&self, place: &Place) -> String {
        let Some(run) = self.run.as_ref() else {
            return String::new();
        };
        let own: Vec<String> = self.own.iter().map(|(name, count)| format!("{}:{count}", name.replace([',', ':'], " "))).collect();
        format!("{}\n{OWN_LINE}{}", run.save_text(place), own.join(","))
    }

    /// Taken up again as a save wrote it.
    pub fn read(&mut self, text: &str) {
        if let Some(run) = self.run.as_mut() {
            run.read_save(text);
        }
        self.own = rules::run::saves::field(text, OWN_LINE)
            .unwrap_or_default()
            .split(',')
            .filter_map(|thing| {
                let (name, count) = thing.rsplit_once(':')?;
                Some((name.to_string(), count.trim().parse().ok()?))
            })
            .collect();
    }
}
