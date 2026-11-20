//! What the story and the scenes do to the team: starting over, members
//! changing, what a scene gives or takes.

use super::Run;
use super::team::{EpisodeRows, Progress, Recruit};

impl Run {
    /// A new story, or a special episode (its rows of the characters' table):
    /// nothing of the last one stays.
    pub fn start_over(&mut self, episode: Option<EpisodeRows>) {
        self.team.progress = Progress::default();
        (self.bag.items, self.bag.money, self.bank) = (Vec::new(), 0, 0);
        let book = self.book.clone();
        let monster = |row: usize| book.tables.episode_characters.get(row).map_or(0, |character| character.0);
        let (hero, partner) = match episode {
            None => self.team.first_species,
            // (Alone: the partner's place is left empty.)
            Some((leader, partner, _)) => (monster(leader), monster(partner.unwrap_or(leader))),
        };
        self.team.change(&book, hero, partner, false);
        if let Some(rows) = episode {
            self.team.play_episode(&book, rows);
        }
    }

    /// The team becomes these monsters.
    pub fn change_team(&mut self, hero: u16, partner: u16) {
        let book = self.book.clone();
        self.team.change(&book, hero, partner, false);
    }

    /// The team becomes any two monsters, at a level (none: as they start).
    pub fn change_team_at(&mut self, hero: u16, partner: u16, level: Option<i32>) {
        let book = self.book.clone();
        self.team.change_at(&book, hero, partner, level);
    }

    /// Whether what is played is a special episode.
    pub fn in_episode(&mut self, episode: bool) {
        self.team.episode = episode;
    }

    /// Whether the third of an episode's team has joined by now.
    pub fn third_joins(&mut self, joined: bool) {
        self.team.third_joined = joined;
    }

    /// A special episode is open from now on.
    pub fn open_episode(&mut self, episode: u16) {
        let opened = &mut self.team.progress.episodes;
        if !opened.contains(&episode) {
            opened.push(episode);
        }
    }

    /// Where the story stands: from the graduation, jobs pay in full.
    pub fn set_chapter(&mut self, chapter: i32) {
        self.team.progress.chapter = chapter;
    }

    /// That dungeon cleared: what follows it opens (`unlocks::CHAINS`).
    pub fn cleared(&mut self, dungeon: u16) {
        if let Some(next) = crate::unlocks::opened_by_clearing(dungeon) {
            self.open(next);
        }
        let mazes = &mut self.team.progress.mazes;
        if crate::dojo::is_maze(dungeon) && !mazes.contains(&dungeon) {
            mazes.push(dungeon);
        }
    }

    /// Every maze of a type gone through (Marowak has his word for it).
    pub fn mazes_done(&self) -> bool {
        (crate::dojo::FIRST..crate::dojo::FINAL).all(|maze| self.team.progress.mazes.contains(&maze))
    }

    /// The dungeons of the jobs held open for good.
    pub fn open_job_dungeons(&mut self) {
        let held: Vec<u16> = self.team.progress.jobs.iter().map(|job| job.dungeon).collect();
        held.into_iter().for_each(|dungeon| self.open(dungeon));
    }

    /// A dungeon opened for good (by the café's Project P, say).
    pub fn open(&mut self, dungeon: u16) {
        if !self.team.progress.opened.contains(&dungeon) {
            self.team.progress.opened.push(dungeon);
        }
    }

    /// The team came back from that dungeon for the first time.
    pub fn returned_from(&mut self, dungeon: u16) {
        self.team.progress.returned.push(dungeon);
    }

    /// A scene gives so many of an item: in the bag, else the storage.
    pub fn give_items(&mut self, item: u16, count: u16) {
        for _ in 0..count {
            self.stow(item);
        }
    }

    /// A scene takes so many of an item: from the bag first, then the
    /// storage.
    pub fn take_items(&mut self, item: u16, count: u16) {
        for _ in 0..count {
            if let Some(at) = self.bag.items.iter().position(|&held| held == item) {
                self.bag.items.remove(at);
            } else if let Some(at) = self.team.progress.storage.iter().position(|&kept| kept == item) {
                self.team.progress.storage.remove(at);
            }
        }
    }

    /// Money given or taken: it never goes below nothing.
    pub fn add_money(&mut self, amount: i32) {
        self.bag.money = (self.bag.money + amount).max(0);
    }

    pub fn add_recruit(&mut self, recruit: Recruit) {
        self.team.progress.recruits.push(recruit);
    }

    /// The bag holds these items and no others.
    pub fn fill_bag(&mut self, items: Vec<u16>) {
        self.bag.items = items;
    }
}
