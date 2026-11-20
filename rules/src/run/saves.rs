//! The story's save as text, written and read back. What holds it (the
//! director's numbers, the place) is given by the game; the files are its
//! too: nothing here touches the disk.

use std::collections::BTreeMap;

use crate::dungeon::save::{PartnerSave, SaveData};
use crate::dungeon::{Belly, Stats};
use crate::jobs::{self, Job};

use super::Run;
use super::team::*;

/// How a save starts the line of the special episodes the story has opened,
/// and the one of the dungeons whose end has played its scene.
pub const EPISODES_LINE: &str = "episodes=";
pub(crate) const RETURNED_LINE: &str = "returned=";
/// How many of an item the bag holds is given to scenes as this number,
/// the item's after it (`item_109`).
pub const ITEM_COUNT: &str = "item_";
/// How many recruits the team has, and of a species (its number after
/// it: `recruit_25`).
pub const RECRUITS: &str = "recruits";
pub const RECRUIT_COUNT: &str = "recruit_";

/// The numbers scenes keep (see `Director::vars`): `name:value,...`.
pub(crate) const VARS_LINE: &str = "vars=";
/// The label of the event at hand the story was saved at (see
/// `FlowBeat::Checkpoint`).
pub(crate) const CHECKPOINT_LINE: &str = "checkpoint=";
/// Words a scene kept (see `DataBeat::SetText`): one a line.
pub(crate) const TEXT_LINE: &str = "text ";
/// The numbers scenes keep, as a save writes them.
pub(crate) fn vars_text(vars: &BTreeMap<String, f64>) -> String {
    // (Not what the game gives them each frame: the money, the bag, the
    // team.)
    // (Nor the numbers of an event only, `_name`.)
    let given = |name: &str| {
        name.starts_with('_')
            || matches!(name, "money" | "items" | "hero" | "partner" | "hero_species" | "partner_species")
            || name == RECRUITS
            || [ITEM_COUNT, RECRUIT_COUNT].iter().any(|given| name.starts_with(given))
    };
    // (A whole number as it always was, `3`; a decimal `3.5`.)
    let mut vars: Vec<String> = vars.iter().filter(|(name, _)| !given(name)).map(|(name, value)| format!("{name}:{}", number_text(*value))).collect();
    vars.sort();
    vars.join(",")
}

/// A number as a save writes it: `3`, `3.5`, `-0.25` (three decimals at
/// most), as a scene writes it too (`engine::number_text`; the rules
/// know no engine).
fn number_text(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        return format!("{}", number as i64);
    }
    let written = format!("{number:.3}");
    written.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The numbers scenes keep, from a save's text.
pub(crate) fn read_vars(text: &str) -> BTreeMap<String, f64> {
    let line = text.lines().find_map(|line| line.strip_prefix(VARS_LINE)).unwrap_or_default();
    line.split(',').filter_map(|pair| pair.split_once(':')).filter_map(|(name, value)| Some((name.to_string(), value.parse().ok()?))).collect()
}

#[cfg(test)]
mod decimals {
    use super::*;

    /// The numbers a save keeps: a whole one as before (`3`), a decimal as one.
    #[test]
    fn numbers_kept_whole_or_decimal() {
        let vars: BTreeMap<String, f64> = [("a".to_string(), 3.0), ("b".to_string(), 2.5)].into();
        let text = vars_text(&vars);
        assert_eq!(text, "a:3,b:2.5");
        assert_eq!(read_vars(&format!("{VARS_LINE}{text}")), vars);
        assert_eq!(read_vars(&format!("{VARS_LINE}seen_x:1")), [("seen_x".to_string(), 1.0)].into(), "a save of before");
    }
}

pub(crate) const STORAGE_LINE: &str = "storage=";
pub(crate) const OPENED_LINE: &str = "opened=";
pub(crate) const EGGS_LINE: &str = "eggs=";
pub(crate) const SOS_LINE: &str = "sos=";
pub(crate) const MAZES_LINE: &str = "mazes=";
pub(crate) const IQ_OFF_LINE: &str = "iq_off=";
pub(crate) const SPECIES_LINE: &str = "species=";
pub(crate) const JOBS_LINE: &str = "jobs=";

/// Where the story stands, as the game gives it to be saved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    /// The next of the story's events (none: it is over).
    pub event: Option<usize>,
    /// The level walked in.
    pub town: Option<u16>,
    pub numbers: BTreeMap<String, f64>,
    pub texts: BTreeMap<String, String>,
    pub checkpoint: Option<String>,
}

/// One line of a save's text.
pub fn field(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| line.strip_prefix(name)).map(str::to_string)
}

impl Run {
    /// The team, the bag and the money as a story save keeps them: in the
    /// dungeon quicksave's format, without a floor.
    pub(crate) fn team_save(&self) -> SaveData {
        let fresh =
            |founder: &Founder| Member { stats: founder.stats, moves: Vec::new(), exp: Experience::new(&founder.growth, founder.stats.level).growth.points };
        let progress = &self.team.progress;
        let leader = progress.leader.clone().unwrap_or_else(|| fresh(&self.team.leader));
        let partner = progress.partner.clone().unwrap_or_else(|| fresh(&self.team.partner));
        SaveData {
            floor_seed: 0,
            floor_number: 0,
            turn: 0,
            stats: leader.stats,
            pp: Vec::new(),
            bag: self.bag.items.clone(),
            belly: Belly::full().raw(),
            sleep_turns: 0,
            poisoned: false,
            moves: leader.moves,
            exp: Some(leader.exp),
            partner: Some(PartnerSave { stats: partner.stats, moves: partner.moves, pp: Vec::new(), exp: partner.exp }),
            extras: Vec::new(),
            money: self.bag.money,
        }
    }

    /// The story's save, as text.
    pub fn save_text(&self, place: &Place) -> String {
        let none = || "-".to_string();
        let event = place.event.map_or_else(none, |event| event.to_string());
        let town = place.town.map_or_else(none, |level| level.to_string());
        let progress = &self.team.progress;
        let list = |ids: &[u16]| ids.iter().map(u16::to_string).collect::<Vec<_>>().join(",");
        let mut text = format!(
            "event {event}\ntown {town}\nbank {}\n{EPISODES_LINE}{}\n{RETURNED_LINE}{}\n{OPENED_LINE}{}\n{EGGS_LINE}{}\n{SOS_LINE}{}\n{MAZES_LINE}{}\n{IQ_OFF_LINE}{}\n{}\n{STORAGE_LINE}{}\n{SPECIES_LINE}{},{}\n{JOBS_LINE}{}\n{}\n{VARS_LINE}{}",
            self.bank,
            list(&progress.episodes),
            list(&progress.returned),
            list(&progress.opened),
            progress.eggs.iter().map(|(species, trips)| format!("{species}:{trips}")).collect::<Vec<_>>().join(","),
            progress.sos.as_ref().map(super::team::Sos::to_text).unwrap_or_default(),
            list(&progress.mazes),
            progress
                .iq_off
                .iter()
                .filter(|(_, off)| **off != 0)
                .map(|(name, off)| format!("{}:{off}", name.replace([',', ':'], " ")))
                .collect::<Vec<_>>()
                .join(","),
            self.team_save().to_text(),
            list(&progress.storage),
            self.team.species.0,
            self.team.species.1,
            jobs::to_text(&progress.jobs),
            progress.names_text(),
            vars_text(&place.numbers)
        );
        for (var, words) in &place.texts {
            text.push_str(&format!("\n{TEXT_LINE}{var}={}", words.replace('\n', " ")));
        }
        if let Some(label) = &place.checkpoint {
            text.push_str(&format!("\n{CHECKPOINT_LINE}{label}"));
        }
        text
    }

    /// Takes what a story save kept; where the story stands is returned.
    pub fn read_save(&mut self, text: &str) -> Place {
        self.bank = field(text, "bank ").and_then(|bank| bank.trim().parse().ok()).unwrap_or(0);
        if let Some((hero, partner)) = saved_species(text) {
            let book = self.book.clone();
            self.team.change(&book, hero, partner, true);
        }
        if let Some(save) = SaveData::from_text(text) {
            self.restore_team(save);
        }
        let progress = &mut self.team.progress;
        progress.storage = field(text, STORAGE_LINE).unwrap_or_default().split(',').filter_map(|id| id.parse().ok()).collect();
        progress.jobs = saved_jobs(text);
        progress.episodes = field(text, EPISODES_LINE).unwrap_or_default().split(',').filter_map(|episode| episode.parse().ok()).collect();
        progress.returned = field(text, RETURNED_LINE).unwrap_or_default().split(',').filter_map(|dungeon| dungeon.parse().ok()).collect();
        progress.opened = field(text, OPENED_LINE).unwrap_or_default().split(',').filter_map(|dungeon| dungeon.parse().ok()).collect();
        progress.eggs = field(text, EGGS_LINE)
            .unwrap_or_default()
            .split(',')
            .filter_map(|egg| {
                let (species, trips) = egg.split_once(':')?;
                Some((species.parse().ok()?, trips.parse().ok()?))
            })
            .collect();
        progress.iq_off = field(text, IQ_OFF_LINE)
            .unwrap_or_default()
            .split(',')
            .filter_map(|entry| {
                let (name, off) = entry.rsplit_once(':')?;
                Some((name.to_string(), off.trim().parse().ok()?))
            })
            .collect();
        progress.mazes = field(text, MAZES_LINE).unwrap_or_default().split(',').filter_map(|maze| maze.parse().ok()).collect();
        progress.sos = field(text, SOS_LINE).and_then(|sos| super::team::Sos::from_text(sos.trim()));
        progress.read_names(text);
        Place {
            event: field(text, "event ").and_then(|event| event.trim().parse().ok()),
            town: field(text, "town ").and_then(|level| level.trim().parse().ok()),
            numbers: read_vars(text),
            texts: text
                .lines()
                .filter_map(|line| line.strip_prefix(TEXT_LINE)?.split_once('='))
                .map(|(var, words)| (var.to_string(), words.to_string()))
                .collect(),
            checkpoint: field(text, CHECKPOINT_LINE),
        }
    }

    /// Gives the team, the bag and the money a story save kept.
    fn restore_team(&mut self, save: SaveData) {
        let exp = |points: Option<i32>, stats: &Stats, growing: &Growing| points.unwrap_or_else(|| Experience::new(growing, stats.level).growth.points);
        // (Abilities and the IQ group are the species': old saves lack them.)
        let abilities = |stats: Stats, species: &Stats| Stats { abilities: species.abilities, iq_group: species.iq_group, ..stats };
        let team = &mut self.team;
        team.progress.leader =
            Some(Member { exp: exp(save.exp, &save.stats, &team.leader.growth), stats: abilities(save.stats, &team.leader.stats), moves: save.moves });
        let species = team.partner.stats;
        team.progress.partner = save.partner.map(|partner| Member { stats: abilities(partner.stats, &species), moves: partner.moves, exp: partner.exp });
        (self.bag.items, self.bag.money) = (save.bag, save.money);
    }
}

/// The jobs a story save's team holds.
pub(crate) fn saved_jobs(text: &str) -> Vec<Job> {
    field(text, JOBS_LINE).map_or(Vec::new(), |line| jobs::from_text(&line))
}

/// The monsters of a story save's team, if it says.
pub(crate) fn saved_species(text: &str) -> Option<(u16, u16)> {
    let line = field(text, SPECIES_LINE)?;
    let (hero, partner) = line.split_once(',')?;
    Some((hero.parse().ok()?, partner.parse().ok()?))
}

/// What the saved story's team is, in a line: names (or species), level,
/// money.
pub fn save_summary(text: &str, book: &super::Book) -> String {
    let (hero, partner) = saved_species(text).unwrap_or((0, 0));
    let names = field(text, NAMES_LINE).unwrap_or_default();
    let mut names = names.split('|');
    let name = |given: Option<&str>, species: u16| match given.filter(|name| !name.is_empty()) {
        Some(name) => name.to_string(),
        None => book.species_name(species),
    };
    let (hero, partner) = (name(names.next(), hero), name(names.next(), partner));
    let (level, money) = (field(text, "level=").unwrap_or_default(), field(text, "money=").unwrap_or_default());
    format!("{hero} and {partner}, level {level}, {money} Poké.")
}
