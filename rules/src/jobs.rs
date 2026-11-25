//! Jobs taken at the guild's boards, and the choice of a dungeon to leave
//! for at the crossroads.
//!
//! The story waits on jobs being done: the game counts those done in the
//! day (`REQUEST_CLEAR_COUNT`), and its routine for leaving a dungeon asks
//! whether one was done there.
//!
//! Ranks, the rank of each floor, the points they give and the kinds of
//! reward are the game's own tables (`JobTables`). Seven kinds of job:
//! someone to rescue, an item to bring to someone, someone to take to
//! another, a floor to reach, an item to find, an outlaw to arrest, who may
//! flee while still strong or come with its gang, and an outlaw to take a
//! stolen item from. The client is one of the monsters living on the floor;
//! the boards offer jobs of the dungeons the story has opened.

use serde::{Deserialize, Serialize};

use crate::services::{Input, View};

/// The `message_Menu`s of the two boards, and the number this engine
/// gives the choice of a dungeon (`main_EnterAdventure`).
pub const JOB_BOARD: u16 = 39;
pub const OUTLAW_BOARD: u16 = 40;
pub const DEPARTURE: u16 = 0x7F00;

/// Jobs a board offers, and jobs a team may hold.
const OFFERED: usize = 8;
pub const HELD: usize = 8;
/// What the guild leaves the team of a reward, before its graduation;
/// from the chapter of the graduation on, the whole reward. (A game of
/// one's own pays in full, unless it says `guild_cut: yes`.)
const TEAM_SHARE: i32 = 10;
pub const GRADUATION: i32 = 29;
/// What clients ask to be brought: Oran, Pecha, Cheri, Rawst and Chesto
/// Berries, a Heal Seed.
const DELIVERIES: [u16; 6] = [70, 81, 82, 78, 90, 69];
/// The items a job may give besides money, by its difficulty: Oran Berry,
/// Heal Seed, Apple, Pecha Berry; Sitrus Berry, Reviver Seed, Big Apple,
/// Max Elixir; the Gummis.
const PRIZES_EASY: [u16; 4] = [70, 69, 109, 81];
const PRIZES_MIDDLE: [u16; 4] = [71, 73, 110, 99];
const GUMMIS: std::ops::RangeInclusive<u16> = 119..=134;
/// The letters of a job's rank, 1 for E; past S it is shown in stars.
const RANKS_IN_LETTERS: [char; 6] = ['E', 'D', 'C', 'B', 'A', 'S'];
const TOP_RANK: u8 = 15;
/// From this rank a job's gang comes along, and exclusive items are given.
const GANG_RANK: u8 = 4;
const EXCLUSIVE_RANK: u8 = 11;
/// What a job pays, by rank.
const PAY_RANK: i32 = 2;
/// The kinds of reward, in the order of `JobTables::rewards`
/// (`mission_reward_type`).
const MONEY: usize = 0;
const MONEY_AND_ITEM: usize = 1;
const ITEM: usize = 2;
const ITEM_AND_MORE: usize = 3;
const EXCLUSIVE: usize = 4;
const HIDDEN_MONEY: usize = 5;
const JOINS: usize = 6;
const EGG_ONE_IN: usize = 8;

/// A job's rank by its letter (`E` … `S`) or its stars (`1*` … `9*`).
pub fn rank_named(word: &str) -> Option<u8> {
    let word = word.trim().to_ascii_uppercase();
    if let Some(stars) = word.strip_suffix('*') {
        let stars: u8 = stars.trim().parse().ok().filter(|stars| (1..=TOP_RANK - RANKS_IN_LETTERS.len() as u8).contains(stars))?;
        return Some(RANKS_IN_LETTERS.len() as u8 + stars);
    }
    let mut letters = word.chars();
    let letter = letters.next().filter(|_| letters.next().is_none())?;
    RANKS_IN_LETTERS.iter().position(|&known| known == letter).map(|at| at as u8 + 1)
}

/// The rank of a floor whose foes are of that level, for a dungeon that
/// does not say it: E below 9, D below 18, C below 27… (the game's own
/// floors go about so).
pub fn rank_for_level(level: i32) -> u8 {
    (1 + level.max(0) / 9).min(i32::from(TOP_RANK)) as u8
}

/// An exploration rank: its name, the points that lead past it, the room the
/// storage has at it, the item given on leaving it (0: none).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplorerRank {
    pub name: String,
    pub points: u32,
    pub storage: u32,
    pub prize: u16,
}

/// The game's tables for jobs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTables {
    pub ranks: Vec<ExplorerRank>,
    /// The points a job gives, by its rank.
    pub rank_points: Vec<u32>,
    /// The rank of each floor, by dungeon (0: none).
    pub floor_ranks: Vec<Vec<u8>>,
    /// How likely each kind of reward is, on the board and on the outlaws'.
    pub rewards: [[u16; 7]; 2],
}

impl JobTables {
    /// The rank the team is at with so many points.
    fn rank_at(&self, points: u32) -> Option<&ExplorerRank> {
        self.ranks.iter().find(|rank| points < rank.points).or(self.ranks.last())
    }

    /// The team's exploration rank for so many points.
    pub fn rank_name(&self, points: u32) -> &str {
        self.rank_at(points).map_or("", |rank| rank.name.as_str())
    }

    /// The points the next rank starts at, if there is one.
    pub fn next_rank(&self, points: u32) -> Option<u32> {
        self.rank_at(points).filter(|rank| rank.points > points && rank.points < u32::MAX / 2).map(|rank| rank.points)
    }

    /// The items of the ranks left going from `before` points to `after`.
    pub fn prizes_between(&self, before: u32, after: u32) -> Vec<u16> {
        self.ranks.iter().filter(|rank| before < rank.points && rank.points <= after && rank.prize != 0).map(|rank| rank.prize).collect()
    }

    /// The rank a floor has in the tables (0: none).
    pub(crate) fn floor_rank(&self, dungeon: u16, floor: u32) -> u8 {
        self.floor_ranks.get(dungeon as usize).and_then(|floors| floors.get(floor.saturating_sub(1) as usize)).copied().unwrap_or(0)
    }

    /// The team's exploration rank for so many points, as a number: 1 the
    /// first (Normal), 2 the next… (`if rank >= 3`).
    pub fn rank_number(&self, points: u32) -> u32 {
        let at = self.ranks.iter().position(|rank| points < rank.points).unwrap_or(self.ranks.len().saturating_sub(1));
        at as u32 + 1
    }

    /// The rank of a job of that kind there: harder to take someone along or
    /// to arrest (`GetMissionRankWithCapAndModifiers`).
    fn job_rank(&self, dungeon: u16, floor: u32, kind: Kind) -> u8 {
        let listed = self.floor_rank(dungeon, floor);
        let more = matches!(kind, Kind::Escort | Kind::Outlaw | Kind::Stolen) as u8;
        (listed.max(1) + more).min(TOP_RANK)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    /// Someone waits on the floor to be found.
    #[default]
    Rescue,
    /// Someone waits on the floor for an item.
    Deliver,
    /// An outlaw is on the floor, to be defeated.
    Outlaw,
    /// The client comes along on the floor, to someone waiting there (the
    /// job's `item` is that one's species).
    Escort,
    /// The floor is to be reached.
    Explore,
    /// An item the client lost lies on the floor, to be found.
    Find,
    /// An outlaw on the floor has stolen the item: defeated, it gives it
    /// back to its owner.
    Stolen,
}

impl Kind {
    /// The kinds by their words in a game's file (`on job done rescue`).
    pub const WORDS: [(&str, Kind); 7] = [
        ("rescue", Kind::Rescue),
        ("deliver", Kind::Deliver),
        ("escort", Kind::Escort),
        ("explore", Kind::Explore),
        ("find", Kind::Find),
        ("outlaw", Kind::Outlaw),
        ("stolen", Kind::Stolen),
    ];

    /// Its word.
    pub fn word(self) -> &'static str {
        Self::WORDS.iter().find(|(_, kind)| *kind == self).map_or("rescue", |(word, _)| word)
    }

    /// The kind of that word.
    pub fn named(word: &str) -> Option<Kind> {
        Self::WORDS.iter().find(|(known, _)| *known == word).map(|(_, kind)| *kind)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Job {
    pub dungeon: u16,
    pub floor: u32,
    /// The reward, before the guild's share.
    pub reward: i32,
    pub kind: Kind,
    /// The monster to rescue, to bring the item to, or to arrest.
    pub client: u16,
    /// The item to bring, for a delivery.
    pub item: u16,
    /// Done in the dungeon: paid once the team is out of it.
    pub done: bool,
    /// The item given besides money (0: none).
    pub prize: u16,
    /// How hard it is: 1 (E) to 15 (nine stars).
    pub rank: u8,
    /// Its reward is not told: "?" on the boards.
    pub secret: bool,
    /// The client asks to join the team, done.
    pub joins: bool,
    /// The client gives an egg, done (it hatches at Chansey's Day Care).
    pub egg: bool,
}

impl Job {
    /// Its rank as shown: a letter, then stars.
    pub fn letter(&self) -> String {
        match RANKS_IN_LETTERS.get(self.rank.saturating_sub(1) as usize) {
            Some(letter) => letter.to_string(),
            None => format!("{}*", self.rank as usize - RANKS_IN_LETTERS.len()),
        }
    }

    /// Whether an outlaw of it comes with its gang.
    pub(crate) fn with_gang(&self) -> bool {
        self.rank >= GANG_RANK
    }

    /// What the job asks, in a few words.
    pub fn title(&self, monster: impl Fn(u16) -> String, item: impl Fn(u16) -> String) -> String {
        match self.kind {
            Kind::Rescue => format!("Rescue {}", monster(self.client)),
            Kind::Deliver => format!("{} for {}", item(self.item), monster(self.client)),
            Kind::Outlaw => format!("Arrest {}", monster(self.client)),
            Kind::Escort => format!("Escort {} to {}", monster(self.client), monster(self.item)),
            Kind::Explore => format!("Explore with {}", monster(self.client)),
            Kind::Find => format!("Find {} for {}", item(self.item), monster(self.client)),
            Kind::Stolen => format!("Take back {} from {}", item(self.item), monster(self.client)),
        }
    }

    /// What is said back in town before the reward, as the original's
    /// client comes to thank the team (an outlaw's: Officer Magnezone).
    pub fn thanks(&self, monster: impl Fn(u16) -> String, item: impl Fn(u16) -> String) -> String {
        let client = monster(self.client);
        match self.kind {
            Kind::Rescue => format!("{client}: Thank you so much for rescuing me! Please accept this reward."),
            Kind::Deliver => format!("{client}: Thank you for bringing me {}! Please accept this reward.", item(self.item)),
            Kind::Escort => format!("{client}: Thank you for taking me to {}! Please accept this reward.", monster(self.item)),
            Kind::Explore => format!("{client}: Thank you for coming along with me! Please accept this reward."),
            Kind::Find => format!("{client}: You found my {}! Thank you! Please accept this reward.", item(self.item)),
            Kind::Outlaw => format!("Magnezone: ZZT! {client} is under arrest, thanks to you! Here is your reward."),
            Kind::Stolen => format!("Magnezone: ZZT! {client} is under arrest, and {} is back with its owner! Here is your reward.", item(self.item)),
        }
    }

    /// Its reward as shown: money, or "?" when it is not told.
    fn shown_reward(&self) -> String {
        match self.secret {
            true => "?".to_string(),
            false => self.reward.to_string(),
        }
    }

    /// The job in full, as a board shows it read: who asks (or who is
    /// wanted), what for, where, how hard, what it gives (`dungeon` names
    /// its place).
    pub fn details(&self, monster: impl Fn(u16) -> String, item: impl Fn(u16) -> String, dungeon: impl Fn(u16) -> String) -> String {
        let (who, objective) = match self.kind {
            Kind::Rescue => ("Client", format!("Rescue {}.", monster(self.client))),
            Kind::Deliver => ("Client", format!("Deliver {} to {}.", item(self.item), monster(self.client))),
            Kind::Escort => ("Client", format!("Escort {} to {}.", monster(self.client), monster(self.item))),
            Kind::Explore => ("Client", format!("Reach the floor with {}.", monster(self.client))),
            Kind::Find => ("Client", format!("Find {}.", item(self.item))),
            Kind::Outlaw => ("Outlaw", format!("Arrest {}.", monster(self.client))),
            Kind::Stolen => ("Outlaw", format!("Take back {} from {}.", item(self.item), monster(self.client))),
        };
        let reward = match (self.secret, self.reward, self.prize) {
            (true, ..) => "?".to_string(),
            (false, money, 0) => format!("{money} Poké"),
            (false, 0, prize) => item(prize),
            (false, money, prize) => format!("{money} Poké + {}", item(prize)),
        };
        format!(
            "{who}: {}\nObjective: {objective}\nPlace: {} {}\nDifficulty: {}  Reward: {reward}",
            monster(self.client),
            dungeon(self.dungeon),
            floor_label(self.floor),
            self.letter()
        )
    }
}

/// Jobs for a board (the outlaws', or the other), from the dungeons open
/// for jobs (`(dungeon, number of floors)`); `living(dungeon, floor)` is
/// the monsters living there, of which one is the client; `random(n)`
/// draws a number below `n`.
pub fn offers(
    tables: &JobTables,
    open: &[(u16, u32)],
    outlaws: bool,
    living: impl Fn(u16, u32) -> Vec<u16>,
    mut random: impl FnMut(usize) -> usize,
) -> Vec<Job> {
    let open: Vec<&(u16, u32)> = open.iter().filter(|(_, floors)| *floors > 0).collect();
    if open.is_empty() {
        return Vec::new();
    }
    let mut offer = |_| {
        let &(dungeon, floors) = open[random(open.len())];
        let floor = random(floors as usize) as u32 + 1;
        let there = living(dungeon, floor);
        let client = *there.get(random(there.len().max(1)))?;
        let (kind, item, pay) = match (outlaws, random(6)) {
            (true, 0 | 1) => (Kind::Stolen, DELIVERIES[random(DELIVERIES.len())], 200),
            (true, _) => (Kind::Outlaw, 0, 200),
            (false, 0) => (Kind::Deliver, DELIVERIES[random(DELIVERIES.len())], 150),
            (false, 1) => (Kind::Escort, *there.get(random(there.len()))?, 200),
            (false, 2) => (Kind::Explore, 0, 120),
            (false, 3) => (Kind::Find, DELIVERIES[random(DELIVERIES.len())], 150),
            (false, _) => (Kind::Rescue, 0, 100),
        };
        let rank = tables.job_rank(dungeon, floor, kind);
        let mut job = Job { dungeon, floor, reward: pay * PAY_RANK * rank as i32, kind, client, item, rank, ..Job::default() };
        job.reward_of(tables.rewards[outlaws as usize], &mut random);
        Some(job)
    };
    (0..OFFERED).filter_map(&mut offer).collect()
}

impl Job {
    /// Its kind of reward, drawn by the game's weights: money, an item or
    /// both; one kept from sight; an exclusive item, at the top ranks; the
    /// client joining the team.
    fn reward_of(&mut self, weights: [u16; 7], random: &mut impl FnMut(usize) -> usize) {
        let open = |kind: usize| kind != EXCLUSIVE || self.rank >= EXCLUSIVE_RANK;
        let total: usize = (0..7).filter(|&kind| open(kind)).map(|kind| weights[kind] as usize).sum();
        let mut drawn = random(total.max(1));
        let kind = (0..7).filter(|&kind| open(kind)).find(|&kind| {
            let weight = weights[kind] as usize;
            let here = drawn < weight;
            drawn = drawn.saturating_sub(weight);
            here
        });
        let prize = prize_for(self.rank, random(64));
        match kind.unwrap_or(MONEY) {
            MONEY => {}
            // (One in eight gives an egg instead of the item.)
            MONEY_AND_ITEM if random(EGG_ONE_IN) == EGG_ONE_IN - 1 => (self.egg, self.secret) = (true, true),
            MONEY_AND_ITEM => self.prize = prize,
            ITEM => (self.prize, self.reward) = (prize, 0),
            ITEM_AND_MORE => (self.prize, self.reward, self.secret) = (prize, self.reward / 2, true),
            EXCLUSIVE => {
                let count = (crate::dungeon::exclusive::END - crate::dungeon::exclusive::FIRST) as usize;
                (self.prize, self.reward, self.secret) = (crate::dungeon::exclusive::FIRST + random(count) as u16, 0, true);
            }
            HIDDEN_MONEY => self.secret = true,
            JOINS => (self.joins, self.reward, self.secret) = (true, 0, true),
            _ => {}
        }
    }
}

/// The item a job of that rank gives, drawn from a number.
fn prize_for(rank: u8, drawn: usize) -> u16 {
    match rank {
        0..=2 => PRIZES_EASY[drawn % PRIZES_EASY.len()],
        3 | 4 => PRIZES_MIDDLE[drawn % PRIZES_MIDDLE.len()],
        _ => GUMMIS.start() + (drawn % GUMMIS.clone().count()) as u16,
    }
}

/// What the jobs done in a dungeon give the team once out of it: its share
/// of their money, the points of the exploration rank, their items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Payment {
    pub money: i32,
    pub points: u32,
    pub items: Vec<u16>,
    /// Clients who join the team.
    pub joining: Vec<u16>,
    /// Clients who gave an egg.
    pub eggs: Vec<u16>,
}

/// The team is out of a dungeon: the jobs done there leave the list, and
/// the team gets what they give (none: nothing was done).
pub fn paid(tables: &JobTables, held: &mut Vec<Job>, graduated: bool) -> Option<Payment> {
    let share = if graduated { 100 } else { TEAM_SHARE };
    let done: Vec<Job> = held.iter().filter(|job| job.done).copied().collect();
    held.retain(|job| !job.done);
    (!done.is_empty()).then(|| Payment {
        money: done.iter().map(|job| job.reward).sum::<i32>() * share / 100,
        points: done.iter().filter_map(|job| tables.rank_points.get(job.rank as usize)).sum(),
        items: done.iter().map(|job| job.prize).filter(|&prize| prize != 0).collect(),
        joining: done.iter().filter(|job| job.joins).map(|job| job.client).collect(),
        eggs: done.iter().filter(|job| job.egg).map(|job| job.client).collect(),
    })
}

/// The team fainted: what it had done there is lost.
pub fn fail(held: &mut [Job]) {
    held.iter_mut().for_each(|job| job.done = false);
}

/// A list the leader picks from: a board's jobs, or the dungeons to
/// leave for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    cursor: usize,
    /// The board read is the outlaws'.
    pub outlaws: bool,
    /// The job under the cursor read in full, and the choice under the
    /// cursor there (0: take it, or give it up; 1: back to the list).
    reading: Option<usize>,
}

impl Listing {
    /// One of the two boards.
    pub fn board(outlaws: bool) -> Self {
        Self { cursor: 0, outlaws, reading: None }
    }

    /// A key on a job read in full: true once its first choice is made;
    /// back to the list on any choice or Cancel.
    fn press_reading(&mut self, input: Input) -> bool {
        let Some(choice) = self.reading.as_mut() else {
            return false;
        };
        match input {
            Input::Up | Input::Down => *choice = 1 - *choice,
            Input::Confirm => return self.reading.take() == Some(0),
            Input::Cancel => self.reading = None,
            _ => {}
        }
        false
    }

    fn step(&mut self, input: Input, count: usize) {
        match (input, count) {
            (_, 0) => self.cursor = 0,
            (Input::Down, _) => self.cursor = (self.cursor + 1) % count,
            (Input::Up, _) => self.cursor = (self.cursor + count - 1) % count,
            _ => self.cursor = self.cursor.min(count - 1),
        }
    }

    /// A key at a board: Confirm reads the job under the cursor in full,
    /// and there takes it (if there is room for it); true once the board
    /// is left.
    pub fn press_board(&mut self, input: Input, offered: &mut Vec<Job>, held: &mut Vec<Job>) -> bool {
        if self.reading.is_some() {
            if self.press_reading(input) && self.cursor < offered.len() && held.len() < HELD {
                held.push(offered.remove(self.cursor));
                self.step(Input::Confirm, offered.len());
            }
            return false;
        }
        self.step(input, offered.len());
        match input {
            Input::Cancel => return true,
            Input::Confirm if self.cursor < offered.len() => self.reading = Some(0),
            Input::Confirm => return true,
            _ => {}
        }
        false
    }

    /// A key at the Job List, the jobs held: Confirm reads the one under
    /// the cursor in full, and there gives it up; true once the list is left.
    pub fn press_held(&mut self, input: Input, held: &mut Vec<Job>) -> bool {
        if self.reading.is_some() {
            if self.press_reading(input) && self.cursor < held.len() {
                held.remove(self.cursor);
                self.step(Input::Confirm, held.len());
            }
            return false;
        }
        self.step(input, held.len());
        match input {
            Input::Cancel => return true,
            Input::Confirm if self.cursor < held.len() => self.reading = Some(0),
            Input::Confirm => return true,
            _ => {}
        }
        false
    }

    /// A key at the crossroads: `Some(Some(dungeon))` once one is chosen,
    /// `Some(None)` if the leader stays.
    pub fn press_departure(&mut self, input: Input, dungeons: &[u16]) -> Option<Option<u16>> {
        self.step(input, dungeons.len());
        match input {
            Input::Cancel => Some(None),
            Input::Confirm => Some(dungeons.get(self.cursor).copied()),
            _ => None,
        }
    }

    /// What a board shows: each job by what it asks (`title`) and its
    /// reward, and where the one under the cursor is (`name` names a
    /// dungeon); a job read in full, what `detail` says of it (`Job::details`)
    /// and its two choices.
    pub fn view_board(
        &self,
        offered: &[Job],
        held: &[Job],
        title: impl Fn(&Job) -> String,
        detail: impl Fn(&Job) -> String,
        name: impl Fn(u16) -> String,
    ) -> View {
        if let (Some(choice), Some(job)) = (self.reading, offered.get(self.cursor)) {
            let take = if held.len() < HELD { "Take job" } else { "Job List full" };
            return View { message: detail(job), options: vec![take.to_string(), "Back".to_string()], cursor: choice };
        }
        let line = |job: &Job| format!("{}  [CS:G]{}[CR]  {}", title(job), job.shown_reward(), job.letter());
        let message = match (offered.get(self.cursor), held.len() < HELD) {
            (None, _) if self.outlaws => "No outlaw is wanted.".to_string(),
            (None, _) => "No job is posted.".to_string(),
            (_, false) => "You cannot take any more jobs.".to_string(),
            (Some(job), _) => format!("{} {}", name(job.dungeon), floor_label(job.floor)),
        };
        View { message: format!("{message}\nJobs taken: {} / {HELD}", held.len()), options: offered.iter().map(line).collect(), cursor: self.cursor }
    }

    /// What the Job List shows: the jobs held, each by what it asks and
    /// where; one read in full, what `details` says and its two choices.
    pub fn view_held(&self, held: &[Job], title: impl Fn(&Job) -> String, detail: impl Fn(&Job) -> String, name: impl Fn(u16) -> String) -> View {
        if let (Some(choice), Some(job)) = (self.reading, held.get(self.cursor)) {
            return View { message: detail(job), options: vec!["Give up".to_string(), "Back".to_string()], cursor: choice };
        }
        let line = |job: &Job| format!("{}  {} {}  {}", title(job), name(job.dungeon), floor_label(job.floor), job.letter());
        let message = match held.is_empty() {
            true => "Job List: no job taken.".to_string(),
            false => format!("Job List: {} / {HELD}", held.len()),
        };
        View { message, options: held.iter().map(line).collect(), cursor: self.cursor }
    }

    pub fn view_departure(&self, dungeons: &[u16], held: &[Job], name: impl Fn(u16) -> String) -> View {
        // Dungeons with a job to do are marked.
        let line = |&dungeon: &u16| match held.iter().any(|job| job.dungeon == dungeon) {
            true => format!("{}  [CS:K]mission[CR]", name(dungeon)),
            false => name(dungeon),
        };
        View { message: "Where are you going?".to_string(), options: dungeons.iter().map(line).collect(), cursor: self.cursor }
    }
}

fn floor_label(floor: u32) -> String {
    format!("floor {floor}")
}

/// Jobs as a line of a save, and back.
pub fn to_text(jobs: &[Job]) -> String {
    let kind = |kind: Kind| match kind {
        Kind::Rescue => 0,
        Kind::Deliver => 1,
        Kind::Outlaw => 2,
        Kind::Escort => 3,
        Kind::Explore => 4,
        Kind::Find => 5,
        Kind::Stolen => 6,
    };
    let job = |job: &Job| {
        let Job { dungeon, floor, reward, client, item, prize, rank, .. } = job;
        format!(
            "{dungeon}:{floor}:{reward}:{}:{client}:{item}:{}:{prize}:{rank}:{}:{}:{}",
            kind(job.kind),
            job.done as u8,
            job.secret as u8,
            job.joins as u8,
            job.egg as u8
        )
    };
    jobs.iter().map(job).collect::<Vec<_>>().join(",")
}

pub fn from_text(text: &str) -> Vec<Job> {
    let job = |entry: &str| {
        let mut numbers = entry.split(':');
        // (Jobs saved before they had kinds are dropped.)
        Some(Job {
            dungeon: numbers.next()?.parse().ok()?,
            floor: numbers.next()?.parse().ok()?,
            reward: numbers.next()?.parse().ok()?,
            kind: match numbers.next()? {
                "0" => Kind::Rescue,
                "1" => Kind::Deliver,
                "2" => Kind::Outlaw,
                "3" => Kind::Escort,
                "4" => Kind::Explore,
                "5" => Kind::Find,
                "6" => Kind::Stolen,
                _ => return None,
            },
            client: numbers.next()?.parse().ok()?,
            item: numbers.next()?.parse().ok()?,
            done: numbers.next()? == "1",
            // (None in saves before jobs gave items, or had ranks.)
            prize: numbers.next().and_then(|prize| prize.parse().ok()).unwrap_or(0),
            rank: numbers.next().and_then(|rank| rank.parse().ok()).unwrap_or(1),
            secret: numbers.next() == Some("1"),
            joins: numbers.next() == Some("1"),
            egg: numbers.next() == Some("1"),
        })
    };
    text.split(',').filter_map(job).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_by_points_and_jobs_by_rank() {
        let tables = tables();
        assert_eq!((tables.rank_name(0), tables.rank_name(99), tables.rank_name(100)), ("Normal", "Normal", "Bronze"));
        assert_eq!((tables.rank_name(5000), tables.next_rank(0), tables.next_rank(5000)), ("Bronze", Some(100), None));
        assert_eq!(tables.prizes_between(90, 110), [71]);
        let job = |rank| Job { rank, ..Job::default() };
        assert_eq!((job(1).letter(), job(6).letter(), job(8).letter()), ("E".to_string(), "S".to_string(), "2*".to_string()));
        // (Harder to arrest than to rescue, nothing past the top.)
        assert_eq!((tables.job_rank(3, 1, Kind::Rescue), tables.job_rank(3, 1, Kind::Outlaw), tables.job_rank(3, 2, Kind::Outlaw)), (2, 3, TOP_RANK));
        assert!(PRIZES_EASY.contains(&prize_for(1, 5)) && GUMMIS.contains(&prize_for(9, 99)));
        // (A job saved before jobs gave items or had ranks: none, and E.)
        assert_eq!((from_text("3:2:200:0:7:0:1")[0].prize, from_text("3:2:200:0:7:0:1")[0].rank), (0, 1));
    }

    fn tables() -> JobTables {
        let rank = |name: &str, points, prize| ExplorerRank { name: name.to_string(), points, storage: 64, prize };
        JobTables {
            ranks: vec![rank("Normal", 100, 71), rank("Bronze", u32::MAX, 0)],
            rank_points: (0..16).map(|rank| rank * 10).collect(),
            floor_ranks: vec![Vec::new(), Vec::new(), Vec::new(), vec![2, 15]],
            rewards: [[1, 0, 0, 0, 0, 0, 0], [1, 0, 0, 0, 0, 0, 0]],
        }
    }

    #[test]
    fn a_reward_is_of_the_kind_drawn() {
        let reward = |weights, rank| {
            let mut job = Job { rank, reward: 100, ..Job::default() };
            job.reward_of(weights, &mut |_| 0);
            job
        };
        let only = |kind: usize| std::array::from_fn(|at| (at == kind) as u16);
        assert_eq!((reward(only(MONEY), 1).reward, reward(only(MONEY), 1).prize), (100, 0));
        assert!(reward(only(MONEY_AND_ITEM), 1).prize != 0 && reward(only(MONEY_AND_ITEM), 1).reward == 100);
        assert_eq!(reward(only(ITEM), 1).reward, 0);
        let mut egg = Job { rank: 1, ..Job::default() };
        egg.reward_of(only(MONEY_AND_ITEM), &mut |below| below - 1);
        assert!(egg.egg && egg.prize == 0);
        assert!(reward(only(JOINS), 1).joins && reward(only(HIDDEN_MONEY), 1).secret);
        // (No exclusive item at a low rank: what is left of the weights.)
        assert_eq!(reward([0, 0, 0, 0, 1, 0, 0], 1).prize, 0);
        assert!(reward([0, 0, 0, 0, 1, 0, 0], EXCLUSIVE_RANK).prize >= crate::dungeon::exclusive::FIRST);
    }

    #[test]
    fn boards_offer_jobs_in_the_open_dungeons() {
        let living = |dungeon: u16, floor: u32| if dungeon == 3 { vec![10 + floor as u16, 50] } else { Vec::new() };
        let tables = tables();
        let jobs = offers(&tables, &[(3, 6), (9, 0)], false, living, |below| below - 1);
        assert_eq!(jobs.len(), OFFERED);
        // Dungeons without floors are left out; floors are within range;
        // the client lives on the floor; nothing is done yet.
        assert!(jobs.iter().all(|job| job.dungeon == 3 && (1..=6).contains(&job.floor)));
        assert!(jobs.iter().all(|job| job.client == 50 && !job.done && job.kind == Kind::Rescue));
        // The other board's are outlaws; a delivery names its item.
        let outlaws = offers(&tables, &[(3, 6)], true, living, |below| (below - 1).min(2));
        assert!(outlaws.iter().all(|job| job.kind == Kind::Outlaw && job.rank == 2));
        let stolen = offers(&tables, &[(3, 6)], true, living, |_| 0);
        assert!(stolen.iter().all(|job| job.kind == Kind::Stolen && job.client == 11 && job.item == DELIVERIES[0]));
        let deliveries = offers(&tables, &[(3, 6)], false, living, |_| 0);
        assert!(deliveries.iter().all(|job| job.kind == Kind::Deliver && job.item == DELIVERIES[0]));
        // Nobody lives there: no job. No dungeon: no job.
        assert!(offers(&tables, &[(4, 6)], false, living, |_| 0).is_empty());
        assert!(offers(&tables, &[], false, living, |_| 0).is_empty());
    }

    #[test]
    fn jobs_are_taken_done_and_paid() {
        use Input::*;
        let tables = tables();
        let job = |dungeon, floor, kind| Job {
            dungeon,
            floor,
            reward: 100 * floor as i32,
            kind,
            client: 7,
            item: if kind == Kind::Deliver { 70 } else { 0 },
            prize: if kind == Kind::Rescue { 71 } else { 0 },
            rank: if kind == Kind::Outlaw { 3 } else { 1 },
            ..Job::default()
        };
        let jobs = vec![job(3, 2, Kind::Rescue), job(3, 5, Kind::Outlaw), job(4, 1, Kind::Deliver)];
        let (mut offered, mut held) = (jobs.clone(), Vec::new());
        let mut board = Listing::board(false);
        let title = |job: &Job| job.title(|monster| format!("monster {monster}"), |item| format!("item {item}"));
        let detail = |job: &Job| job.details(|monster| format!("monster {monster}"), |item| format!("item {item}"), |dungeon| format!("dungeon {dungeon}"));
        let view = board.view_board(&offered, &held, title, detail, |dungeon| format!("dungeon {dungeon}"));
        assert_eq!(view.options[0], "Rescue monster 7  [CS:G]200[CR]  E");
        assert_eq!(view.options[1], "Arrest monster 7  [CS:G]500[CR]  C");
        assert_eq!(view.options[2], "item 70 for monster 7  [CS:G]100[CR]  E");
        assert!(view.message.starts_with("dungeon 3 floor 2\n"), "{}", view.message);
        // A job read in full: who, what, where, how hard, what it gives.
        assert!(!board.press_board(Confirm, &mut offered, &mut held));
        let view = board.view_board(&offered, &held, title, detail, |dungeon| format!("dungeon {dungeon}"));
        assert_eq!(view.options, ["Take job", "Back"]);
        assert_eq!(view.message, "Client: monster 7\nObjective: Rescue monster 7.\nPlace: dungeon 3 floor 2\nDifficulty: E  Reward: 200 Poké + item 71");
        // Back, read again; the first and the last are taken.
        for input in [Down, Confirm, Confirm, Confirm, Down, Confirm, Confirm] {
            assert!(!board.press_board(input, &mut offered, &mut held));
        }
        assert_eq!((offered.clone(), held.clone()), (vec![jobs[1]], vec![jobs[0], jobs[2]]));
        assert!(board.press_board(Cancel, &mut offered, &mut held));
        // Nothing done, nothing paid; what is done is lost by fainting, or
        // paid a tenth of its reward and off the list.
        assert_eq!(paid(&tables, &mut held, false), None);
        held[0].done = true;
        fail(&mut held);
        assert_eq!(paid(&tables, &mut held, false), None);
        held[0].done = true;
        assert_eq!(from_text(&to_text(&held)), held);
        assert_eq!(paid(&tables, &mut held, false), Some(Payment { money: 20, points: 10, items: vec![71], joining: Vec::new(), eggs: Vec::new() }));
        assert_eq!(held, [jobs[2]]);
        assert_eq!(from_text(&to_text(&held)), held);
        assert_eq!(from_text("3:2:200"), []);
    }

    #[test]
    fn the_job_list_gives_jobs_up() {
        use Input::*;
        let job = |floor| Job { dungeon: 3, floor, rank: 2, client: 7, ..Job::default() };
        let mut held = vec![job(1), job(2), job(3)];
        let mut list = Listing::board(false);
        let title = |job: &Job| format!("job {}", job.floor);
        let detail = |job: &Job| format!("details {}", job.floor);
        let view = list.view_held(&held, title, detail, |dungeon| format!("dungeon {dungeon}"));
        assert_eq!((view.message.as_str(), view.options[1].as_str()), ("Job List: 3 / 8", "job 2  dungeon 3 floor 2  D"));
        // The second read, kept (Back), then given up.
        for input in [Down, Confirm, Down, Confirm, Confirm] {
            assert!(!list.press_held(input, &mut held));
        }
        assert_eq!(list.view_held(&held, title, detail, |_| String::new()).options, ["Give up", "Back"]);
        assert!(!list.press_held(Confirm, &mut held));
        assert_eq!(held, [job(1), job(3)]);
        assert!(list.press_held(Cancel, &mut held));
        assert!(list.press_held(Confirm, &mut Vec::new()), "nothing held: left");
    }

    #[test]
    fn the_outlaw_board_wants_outlaws() {
        use Input::*;
        let tables = tables();
        let living = |_: u16, floor: u32| vec![20 + floor as u16];
        let mut offered = offers(&tables, &[(3, 2)], true, living, |below| (below - 1).min(3));
        assert!(offered.iter().all(|job| job.kind == Kind::Outlaw), "{offered:?}");
        let (mut board, mut held) = (Listing::board(true), Vec::new());
        let title = |job: &Job| job.title(|monster| format!("monster {monster}"), |item| format!("item {item}"));
        let detail = |job: &Job| job.details(|monster| format!("monster {monster}"), |item| format!("item {item}"), |_| "Den".to_string());
        assert!(!board.press_board(Confirm, &mut offered, &mut held));
        assert!(board.view_board(&offered, &held, title, detail, |_| String::new()).message.starts_with("Outlaw: monster 22\nObjective: Arrest monster 22."));
        assert!(!board.press_board(Confirm, &mut offered, &mut held));
        assert_eq!((held.len(), held[0].kind, offered.len()), (1, Kind::Outlaw, OFFERED - 1));
        assert_eq!(Listing::board(true).view_board(&[], &[], title, detail, |_| String::new()).message, "No outlaw is wanted.\nJobs taken: 0 / 8");
    }

    #[test]
    fn ranks_by_name_and_by_level() {
        assert_eq!([rank_named("E"), rank_named("c"), rank_named("S"), rank_named("1*"), rank_named("9*")], [Some(1), Some(3), Some(6), Some(7), Some(15)]);
        assert_eq!([rank_named("F"), rank_named("10*"), rank_named("EE")], [None, None, None]);
        assert_eq!([rank_for_level(5), rank_for_level(9), rank_for_level(20), rank_for_level(500)], [1, 2, 3, TOP_RANK]);
    }

    #[test]
    fn a_dungeon_is_chosen_or_not() {
        use Input::*;
        let mut list = Listing::default();
        assert_eq!(list.press_departure(Down, &[1, 3, 4]), None);
        assert_eq!(list.press_departure(Confirm, &[1, 3, 4]), Some(Some(3)));
        assert_eq!(list.press_departure(Cancel, &[1, 3, 4]), Some(None));
        assert_eq!(list.press_departure(Confirm, &[]), Some(None));
    }

    #[test]
    fn kinds_by_their_words_ranks_by_number_and_the_client_s_thanks() {
        assert!(Kind::WORDS.iter().all(|(word, kind)| kind.word() == *word && Kind::named(word) == Some(*kind)));
        assert_eq!(Kind::named("party"), None);
        let tables = tables();
        assert_eq!([tables.rank_number(0), tables.rank_number(100), tables.rank_number(u32::MAX)], [1, 2, 2]);
        let job = |kind| Job { kind, client: 7, item: 3, ..Job::default() };
        let thanks = |kind| job(kind).thanks(|species| format!("M{species}"), |item| format!("I{item}"));
        assert!(thanks(Kind::Deliver).starts_with("M7: Thank you for bringing me I3!"), "{}", thanks(Kind::Deliver));
        assert!(thanks(Kind::Outlaw).starts_with("Magnezone:") && thanks(Kind::Outlaw).contains("M7 is under arrest"));
    }
}
