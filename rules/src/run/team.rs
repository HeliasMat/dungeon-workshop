//! The team as it is kept between dungeons: the leader, its partner, the
//! recruits, their growth and the moves they know; the bag; what the story
//! keeps of them (`Progress`).

use std::collections::HashMap;
use std::sync::Arc;

use crate::dungeon::{Growth, LevelGain, MoveSlot, Stats};
use crate::jobs::Job;

use super::book::{Book, MemberSetup, PLAYER_STATS};

/// How a species grows: what each level takes and gives, and the moves
/// learnt by level.
#[derive(Clone, Default)]
pub struct Growing {
    pub table: Arc<[LevelGain]>,
    pub learnset: Arc<Vec<(u8, MoveSlot)>>,
    /// The moves TMs and HMs can teach it.
    pub machines: Arc<Vec<MoveSlot>>,
}

/// Experience of a team member.
#[derive(Clone)]
pub struct Experience {
    pub growth: Growth,
    pub learnset: Arc<Vec<(u8, MoveSlot)>>,
    pub machines: Arc<Vec<MoveSlot>>,
}

impl Experience {
    pub fn new(growing: &Growing, level: i32) -> Self {
        Self { growth: Growth::new(growing.table.clone(), level), learnset: growing.learnset.clone(), machines: growing.machines.clone() }
    }
}

/// Who a name is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whose {
    Hero,
    Partner,
    Team,
    /// A scene's: the director keeps it.
    Scene,
    /// A recruit, by its place in the list.
    Recruit(usize),
}

/// A monster that joined the team in a dungeon: kept at the guild. Up to
/// two come along in dungeons: those picked at the guild's bell
/// (`Progress::chosen`), the first two until then.
#[derive(Clone, Debug, PartialEq)]
pub struct Recruit {
    pub species: u16,
    pub name: String,
    /// Its level, HP, attack, defense, special attack and special defense.
    pub stats: [i32; 6],
    /// The moves it knows (none yet: those its species has at its level).
    pub moves: Vec<u16>,
}

impl Progress {
    /// Takes a name once typed.
    pub fn rename(&mut self, whose: Whose, text: &str) {
        let name = text.trim().to_string();
        if name.is_empty() {
            return;
        }
        match whose {
            Whose::Hero => self.names.0 = name,
            Whose::Partner => self.names.1 = name,
            Whose::Team => self.names.2 = name,
            // (A scene's: the director keeps them, see `story_menus`.)
            Whose::Scene => {}
            // (Its tactic is kept by its name: the new one keeps it.)
            Whose::Recruit(at) => {
                if let Some(recruit) = self.recruits.get_mut(at) {
                    if let Some(tactic) = self.tactics.remove(&recruit.name) {
                        self.tactics.insert(name.clone(), tactic);
                    }
                    recruit.name = name;
                }
            }
        }
    }

    /// The names and the recruits as lines of a save, and back.
    pub(crate) fn names_text(&self) -> String {
        let recruit = |recruit: &Recruit| {
            let stats: Vec<String> = recruit.stats.iter().map(i32::to_string).collect();
            format!("{}:{}:{}", recruit.species, stats.join(":"), recruit.name)
        };
        let recruits: Vec<String> = self.recruits.iter().map(recruit).collect();
        let ids = |ids: &Vec<u16>| ids.iter().map(u16::to_string).collect::<Vec<_>>().join(".");
        let moves: Vec<String> = self.recruits.iter().map(|recruit| ids(&recruit.moves)).collect();
        let chosen = self
            .chosen
            .as_ref()
            .map_or(String::new(), |chosen| format!("{CHOSEN_LINE}{}\n", chosen.iter().map(usize::to_string).collect::<Vec<_>>().join(",")));
        let (names, recruits) = (&self.names, recruits.join(";"));
        let mut tactics: Vec<String> = self.tactics.iter().map(|(name, tactic)| format!("{name}:{tactic}")).collect();
        tactics.sort();
        let tactics = format!("{TACTICS_LINE}{}\n", tactics.join(";"));
        let rank = format!("{RANK_LINE}{}\n", self.rank);
        let day = format!("{DAY_LINE}{}{}\n", self.day, if self.evening { ",evening" } else { "" });
        let home = if self.partner_home { format!("{HOME_LINE}partner\n") } else { String::new() };
        let sentry = match (self.sentry_points, self.sentry_today) {
            (0, false) => String::new(),
            (points, today) => format!("{SENTRY_LINE}{points}{}\n", if today { ",today" } else { "" }),
        };
        format!(
            "{NAMES_LINE}{}|{}|{}\n{RECRUITS_LINE}{recruits}\n{RECRUIT_MOVES_LINE}{}\n{chosen}{tactics}{rank}{day}{home}{sentry}",
            names.0,
            names.1,
            names.2,
            moves.join(";")
        )
    }

    pub(crate) fn read_names(&mut self, text: &str) {
        let line = |start: &str| text.lines().find_map(|line| line.strip_prefix(start)).unwrap_or("");
        let mut names = line(NAMES_LINE).split('|').map(str::to_string);
        self.names = (names.next().unwrap_or_default(), names.next().unwrap_or_default(), names.next().unwrap_or_default());
        let recruit = |entry: &str| {
            let mut fields = entry.splitn(8, ':');
            let species = fields.next()?.parse().ok()?;
            let mut stats = [0; 6];
            for stat in &mut stats {
                *stat = fields.next()?.parse().ok()?;
            }
            Some(Recruit { species, stats, name: fields.next()?.to_string(), moves: Vec::new() })
        };
        self.recruits = line(RECRUITS_LINE).split(';').filter_map(recruit).collect();
        // (Their moves and who comes along: absent from older saves.)
        for (recruit, moves) in self.recruits.iter_mut().zip(line(RECRUIT_MOVES_LINE).split(';')) {
            recruit.moves = moves.split('.').filter_map(|id| id.parse().ok()).collect();
        }
        let chosen = text.lines().find_map(|line| line.strip_prefix(CHOSEN_LINE));
        self.chosen = chosen.map(|chosen| chosen.split(',').filter_map(|at| at.parse().ok()).collect());
        self.rank = text.lines().find_map(|line| line.strip_prefix(RANK_LINE)).and_then(|rank| rank.parse().ok()).unwrap_or(0);
        // (The day: absent from older saves, the first, in the morning.)
        self.partner_home = text.lines().any(|line| line.strip_prefix(HOME_LINE) == Some("partner"));
        let day = text.lines().find_map(|line| line.strip_prefix(DAY_LINE)).unwrap_or("0");
        let (days, evening) = day.split_once(',').unwrap_or((day, ""));
        (self.day, self.evening) = (days.trim().parse().unwrap_or(0), evening.trim() == "evening");
        let sentry = text.lines().find_map(|line| line.strip_prefix(SENTRY_LINE)).unwrap_or("0");
        let (points, today) = sentry.split_once(',').unwrap_or((sentry, ""));
        (self.sentry_points, self.sentry_today) = (points.trim().parse().unwrap_or(0), today.trim() == "today");
        let tactics = text.lines().find_map(|line| line.strip_prefix(TACTICS_LINE)).unwrap_or("");
        self.tactics = tactics
            .split(';')
            .filter_map(|pair| {
                let (name, tactic) = pair.rsplit_once(':')?;
                Some((name.to_string(), tactic.parse().ok()?))
            })
            .collect();
    }
}

impl Progress {
    /// A new day, as the team wakes (`GroundMainNextDay`): the boards put
    /// up new jobs, the shops new wares, the eggs at the Day Care wait a
    /// day less; morning again.
    pub fn next_day(&mut self) {
        self.day += 1;
        self.evening = false;
        self.stock = [None, None];
        self.offers.clear();
        self.outlaws.clear();
        self.swapped.clear();
        self.sentry_today = false;
        self.eggs.iter_mut().for_each(|(_, days)| *days = days.saturating_sub(1));
    }

    /// A recruit sent away for good: off the list, its tactic forgotten,
    /// those picked to come along still the same ones.
    pub fn release(&mut self, at: usize) {
        if at >= self.recruits.len() {
            return;
        }
        let gone = self.recruits.remove(at);
        self.tactics.remove(&gone.name);
        self.to_name.retain(|&waiting| waiting != at);
        for waiting in self.to_name.iter_mut().filter(|waiting| **waiting > at) {
            *waiting -= 1;
        }
        if let Some(chosen) = &mut self.chosen {
            chosen.retain(|&picked| picked != at);
            for picked in chosen.iter_mut().filter(|picked| **picked > at) {
                *picked -= 1;
            }
        }
    }
}

pub(crate) const NAMES_LINE: &str = "names=";
pub(crate) const RECRUITS_LINE: &str = "recruits=";
pub(crate) const RECRUIT_MOVES_LINE: &str = "recruit_moves=";
pub(crate) const CHOSEN_LINE: &str = "chosen=";
pub(crate) const TACTICS_LINE: &str = "tactics=";
pub(crate) const RANK_LINE: &str = "rank=";
/// The days gone by, and `,evening` once back from a dungeon (not slept since).
pub(crate) const DAY_LINE: &str = "day=";
/// Who stays at the base (`home=partner`): absent, all come.
pub(crate) const HOME_LINE: &str = "home=";
/// The points of the last sentry duty, and `,today` once stood today.
pub(crate) const SENTRY_LINE: &str = "sentry=";

/// The tactics of the team's members (`enum tactic_id`).
pub mod tactic {
    pub(crate) const TOGETHER: u8 = 0;
    pub(crate) const OWN_WAY: u8 = 1;
    pub(crate) const GO_AFTER_FOES: u8 = 2;
    pub(crate) const AVOID_FIRST_HIT: u8 = 3;
    pub(crate) const AVOID_TROUBLE: u8 = 6;
    pub(crate) const BE_PATIENT: u8 = 7;
    pub(crate) const KEEP_DISTANCE: u8 = 8;
    pub(crate) const WAIT_THERE: u8 = 9;
    pub(crate) const GET_AWAY: u8 = 10;
    /// How many there are.
    pub const COUNT: usize = 11;

    /// The level from which a member may be given each
    /// (`TACTICS_UNLOCK_LEVEL_TABLE`; 999: never).
    const UNLOCK_LEVEL: [i32; COUNT] = [1, 20, -1, -1, 999, 999, 35, 30, 999, 10, 15];

    /// The tactics a member of this level may be given, in order.
    pub fn unlocked(level: i32) -> Vec<u8> {
        (0..COUNT as u8).filter(|&tactic| UNLOCK_LEVEL[tactic as usize] <= level).collect()
    }
}

/// The bit of a kept move's id that links it to the move before (move ids
/// are below 600).
pub const LINKED_MOVE: u16 = 0x8000;
/// The bit that keeps it switched off for the AI (`MoveSlot::off`).
pub const SWITCHED_OFF: u16 = 0x4000;

/// A kept move's id, without the bits kept with it.
pub fn move_id(kept: u16) -> u16 {
    kept & !(LINKED_MOVE | SWITCHED_OFF)
}

/// The team kept between dungeons: what the leader and partner start a run
/// with, and what has become of them.
pub struct Team {
    pub leader: Founder,
    pub partner: Founder,
    /// The leader goes alone (a special episode's).
    pub alone: bool,
    /// Which monsters the leader and its partner are, and which a new story
    /// starts with (before its personality test).
    pub species: (u16, u16),
    pub first_species: (u16, u16),
    /// What the team has become since: kept from one dungeon to the next
    /// and saved with the story.
    pub progress: Progress,
    /// A special episode's third member: its row in the game's table; and
    /// whether the episode has shown it yet (it comes along from then on).
    pub third: Option<usize>,
    pub third_joined: bool,
    /// A special episode is played (its windows' frames are green).
    pub episode: bool,
}

/// The leader or its partner as its species starts.
#[derive(Clone)]
pub struct Founder {
    pub name: String,
    pub stats: Stats,
    pub moves: Vec<MoveSlot>,
    pub movement: u8,
    pub growth: Growing,
}

impl From<MemberSetup> for Founder {
    fn from(member: MemberSetup) -> Self {
        Self { name: member.name, stats: member.stats, moves: member.moves, movement: member.movement, growth: member.growth }
    }
}

impl Team {
    /// The team a new game starts with.
    pub fn first(book: &Book) -> Self {
        let team = book.tables.team;
        let stand_in = || Founder { name: String::new(), stats: PLAYER_STATS, moves: Vec::new(), movement: 0, growth: Growing::default() };
        let founder = |monster, partner| book.member(monster, partner).map_or_else(stand_in, Founder::from);
        Self {
            leader: founder(team.0, false),
            partner: founder(team.1, true),
            alone: false,
            species: team,
            first_species: team,
            progress: Progress::default(),
            third: None,
            third_joined: false,
            episode: false,
        }
    }

    /// Makes these monsters the leader and its partner; `keep_progress`:
    /// with the levels and moves they have (a save being loaded), not as
    /// they start.
    pub fn change(&mut self, book: &Book, hero: u16, partner: u16, keep_progress: bool) {
        let (Some(leader), Some(ally)) = (book.member(hero, false), book.member(partner, true)) else {
            return;
        };
        self.made_of(hero, partner, leader, ally, keep_progress);
    }

    /// The team becomes any two monsters, at a level (none: as they start):
    /// a film's team going through a dungeon of its own.
    pub fn change_at(&mut self, book: &Book, hero: u16, partner: u16, level: Option<i32>) {
        let (Some(leader), Some(ally)) = (book.member_at(hero, false, level), book.member_at(partner, true, level)) else {
            return;
        };
        self.made_of(hero, partner, leader, ally, false);
    }

    fn made_of(&mut self, hero: u16, partner: u16, leader: MemberSetup, ally: MemberSetup, keep_progress: bool) {
        self.species = (hero, partner);
        (self.leader, self.partner) = (leader.into(), ally.into());
        self.alone = false;
        self.third = None;
        if !keep_progress {
            (self.progress.leader, self.progress.partner) = (None, None);
        }
    }

    /// A special episode's team, once `change` made it: as the game's table
    /// has its characters, by their rows there (the leader's, its partner's
    /// if it has one, a third's that joins later).
    pub fn play_episode(&mut self, book: &Book, rows: EpisodeRows) {
        let grown = |row: usize, founder: &mut Founder| {
            let Some((_, level, known)) = book.tables.episode_characters.get(row) else {
                return;
            };
            let mut growth = Growth::new(founder.growth.table.clone(), founder.stats.level);
            let needed = founder.growth.table.get((level - 1).max(0) as usize).map_or(0, |entry| entry.total_exp) - growth.points;
            let _ = growth.gain(&mut founder.stats, needed.max(0));
            if !known.is_empty() {
                founder.moves = known.clone();
            }
        };
        self.third = rows.2;
        grown(rows.0, &mut self.leader);
        match rows.1 {
            Some(row) => grown(row, &mut self.partner),
            None => self.alone = true,
        }
    }

    /// The leader or its partner as kept between dungeons, or as its species
    /// starts (its moves those it starts with).
    pub fn kept_member(&self, partner: bool) -> Member {
        let kept = if partner { &self.progress.partner } else { &self.progress.leader };
        let founder = if partner { &self.partner } else { &self.leader };
        let mut member = kept.clone().unwrap_or_else(|| Member::starting(founder.stats, &founder.growth, false));
        if member.moves.is_empty() {
            member.moves = founder.moves.iter().map(|slot| slot.id).collect();
        }
        member
    }
}

/// The rows of the characters' table a special episode plays: its leader,
/// its partner, a third who joins.
pub type EpisodeRows = (usize, Option<usize>, Option<usize>);

/// A team member as its last dungeon left it; `None` for one that is
/// still as its species starts.
#[derive(Clone, Default)]
pub struct Progress {
    pub leader: Option<Member>,
    pub partner: Option<Member>,
    /// The tactic of each team member but the leader, by name (`enum
    /// tactic_id`, see `tactic`).
    pub tactics: HashMap<String, u8>,
    /// Items left at Kangaskhan Storage.
    pub storage: Vec<u16>,
    /// Jobs the team holds, and those the guild's boards offer (redrawn
    /// each day: `next_day`).
    pub jobs: Vec<Job>,
    pub offers: Vec<Job>,
    pub outlaws: Vec<Job>,
    /// The names given to the hero, the partner and the team (empty:
    /// none yet), and the monsters that joined the team.
    pub names: (String, String, String),
    pub recruits: Vec<Recruit>,
    /// What the Kecleon Shop and Kecleon Wares have left to sell; `None`
    /// until visited. They restock each day.
    pub stock: [Option<Vec<u16>>; 2],
    /// The special episodes the story has opened so far.
    pub episodes: Vec<u16>,
    /// The dungeons whose end has played what it plays the first time.
    pub returned: Vec<u16>,
    /// The recruits picked at the guild's bell to come along, by their
    /// place in the list; `None`: never picked, the first two do.
    pub chosen: Option<Vec<usize>>,
    /// The partner stays at the base: a recruit more comes in its place.
    pub partner_home: bool,
    /// Trades made at the Recycle Shop (more are offered after some).
    pub recycled: i32,
    /// The points of the team's exploration rank (see `jobs::rank_name`).
    pub rank: u32,
    /// Recruits joined in the dungeon, to be offered a nickname back in
    /// town (by their places in `recruits`).
    pub to_name: Vec<usize>,
    /// The story's chapter, as the game last told it (not saved: told
    /// again on arriving anywhere).
    pub chapter: i32,
    /// The dungeons opened by play, not by the story (`unlocks`).
    pub opened: Vec<u16>,
    /// The eggs at Chansey's Day Care: what hatches from each, and the
    /// days till it does.
    pub eggs: Vec<(u16, u8)>,
    /// The days gone by since the game began (`Run::day`: the first is 1),
    /// and whether it is evening: back from a dungeon, not slept since.
    pub day: u32,
    pub evening: bool,
    /// The points of the last sentry duty, and whether it was stood today
    /// (once a day, as the guild's).
    pub sentry_points: i32,
    pub sentry_today: bool,
    /// What the team lost fainting last, for an SOS (Pelipper's mail).
    pub sos: Option<Sos>,
    /// The Dojo's mazes gone through to their end.
    pub mazes: Vec<u16>,
    /// The IQ skills switched off, by member's name (bits by skill).
    pub iq_off: HashMap<String, u128>,
    /// Croagunk's items of the day swapped for today (gone from its list
    /// till the next day; not saved).
    pub swapped: Vec<u16>,
}

/// The team fainted: where, what it lost; whether its SOS is sent, and the
/// explorations till a friendly team answers it (A-OK).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sos {
    pub dungeon: u16,
    pub floor: u32,
    pub items: Vec<u16>,
    pub money: i32,
    pub sent: bool,
    pub trips: u8,
}

impl Sos {
    pub fn to_text(&self) -> String {
        let items = self.items.iter().map(u16::to_string).collect::<Vec<_>>().join(";");
        format!("{}:{}:{}:{}:{}:{items}", self.dungeon, self.floor, self.money, self.sent as u8, self.trips)
    }

    pub fn from_text(text: &str) -> Option<Self> {
        let mut parts = text.split(':');
        Some(Self {
            dungeon: parts.next()?.parse().ok()?,
            floor: parts.next()?.parse().ok()?,
            money: parts.next()?.parse().ok()?,
            sent: parts.next()? == "1",
            trips: parts.next()?.parse().ok()?,
            items: parts.next().unwrap_or_default().split(';').filter_map(|item| item.parse().ok()).collect(),
        })
    }
}

#[derive(Clone)]
pub struct Member {
    pub stats: Stats,
    /// Ids of its moves, in slot order; `LINKED_MOVE` set on those linked to
    /// the move before.
    pub moves: Vec<u16>,
    /// Experience points.
    pub exp: i32,
}

impl Member {
    /// As its species starts (its moves those it starts with), or brought
    /// down to level 1, its stats losing what each level gave.
    pub fn starting(stats: Stats, growing: &Growing, level_one: bool) -> Self {
        let mut stats = stats;
        let mut experience = Experience::new(growing, stats.level);
        if level_one {
            while stats.level > 1 && experience.growth.lose_level(&mut stats) {}
            stats.hp = stats.max_hp;
        }
        Self { stats, moves: Vec::new(), exp: experience.growth.points }
    }

    /// As the member leaves a dungeon: healed.
    pub fn leaving(stats: &Stats, moves: &[MoveSlot], experience: &Experience) -> Self {
        Self {
            stats: Stats { hp: stats.max_hp, ..*stats },
            moves: moves.iter().map(|slot| slot.id | if slot.linked { LINKED_MOVE } else { 0 } | if slot.off { SWITCHED_OFF } else { 0 }).collect(),
            exp: experience.growth.points,
        }
    }

    /// Its stats, moves (with the PP given, or all of it) and experience,
    /// from what its species starts with and learns.
    pub fn restore(&self, start: &[MoveSlot], growing: &Growing, pp: Option<&[i32]>) -> (Stats, Vec<MoveSlot>, Experience) {
        let mut experience = Experience::new(growing, self.stats.level);
        experience.growth.points = self.exp;
        (self.stats, known_moves(start, growing, &self.moves, pp), experience)
    }
}

/// The moves with these ids, from those a species starts with or learns,
/// by level or TM (no ids: its starting moves), with the PP given or all of
/// it, linked and switched off as they were (`LINKED_MOVE`, `SWITCHED_OFF`).
pub(crate) fn known_moves(start: &[MoveSlot], growing: &Growing, ids: &[u16], pp: Option<&[i32]>) -> Vec<MoveSlot> {
    let known = |id: u16| {
        let learnt = growing.learnset.iter().map(|(_, slot)| slot);
        start.iter().chain(learnt).chain(growing.machines.iter()).find(|slot| slot.id == id).copied()
    };
    let mut moves: Vec<MoveSlot> = match ids {
        [] => start.to_vec(),
        ids => ids
            .iter()
            .filter_map(|&id| known(move_id(id)).map(|slot| MoveSlot { linked: id & LINKED_MOVE != 0, off: id & SWITCHED_OFF != 0, ..slot }))
            .collect(),
    };
    // (The first move links to nothing.)
    if let Some(first) = moves.first_mut() {
        first.linked = false;
    }
    for slot in &mut moves {
        slot.pp = slot.max_pp;
    }
    for (slot, &pp) in moves.iter_mut().zip(pp.unwrap_or(&[])) {
        slot.pp = pp;
    }
    moves
}

/// Item ids carried by the team, and its Poké.
#[derive(Clone, Default)]
pub struct Bag {
    pub items: Vec<u16>,
    pub money: i32,
}

/// Gives experience to a team member, with the level-ups it brings, the
/// messages said. The moves it would learn but has no room for are
/// returned: which to forget is asked.
#[must_use]
pub(crate) fn earn(
    book: &Book,
    log: &mut super::Log,
    name: &str,
    stats: &mut Stats,
    moves: &mut Vec<MoveSlot>,
    experience: &mut Experience,
    amount: i32,
) -> Vec<MoveSlot> {
    use crate::strings::log_text::{GREW_LEVEL, LEARNED};
    let mut asked = Vec::new();
    for level in experience.growth.gain(stats, amount) {
        let at = level.to_string();
        log.push(book.phrase(GREW_LEVEL, &[("string:0", name), ("digits_c:0", &at)]).unwrap_or_else(|| format!("{name} grew to level {level}!")));
        for &(_, slot) in experience.learnset.iter().filter(|(at, _)| *at as i32 == level) {
            if moves.iter().any(|known| known.id == slot.id) {
                continue;
            }
            let move_name = book.move_name(slot.id);
            if moves.len() < 4 {
                moves.push(slot);
                log.push(book.phrase(LEARNED, &[("string:0", name), ("move:0", &move_name)]).unwrap_or_else(|| format!("{name} learned {move_name}!")));
            } else {
                log.push(format!("{name} veut apprendre {move_name}."));
                asked.push(slot);
            }
        }
    }
    asked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_switched_off_stays_so_between_dungeons() {
        let slot = |id| MoveSlot { id, pp: 5, max_pp: 5, ..MoveSlot::fallback_regular_attack() };
        let start = [slot(33), slot(45)];
        let kept = [33 | SWITCHED_OFF, 45 | LINKED_MOVE];
        let moves = known_moves(&start, &Growing::default(), &kept, None);
        assert_eq!(moves.iter().map(|slot| (slot.id, slot.off, slot.linked)).collect::<Vec<_>>(), [(33, true, false), (45, false, true)]);
        assert_eq!(kept.map(move_id), [33, 45]);
    }

    #[test]
    fn a_new_day_redraws_the_boards_and_the_shops_and_is_saved() {
        let job = crate::jobs::Job::default();
        let mut progress = Progress { offers: vec![job.clone()], outlaws: vec![job.clone()], jobs: vec![job], evening: true, ..Progress::default() };
        (progress.stock, progress.eggs) = ([Some(vec![70]), None], vec![(25, 2)]);
        progress.next_day();
        assert_eq!((progress.day, progress.evening, progress.offers.len(), progress.outlaws.len()), (1, false, 0, 0));
        assert_eq!((progress.stock.clone(), progress.eggs.clone(), progress.jobs.len()), ([None, None], vec![(25, 1)], 1), "the jobs taken kept");
        progress.evening = true;
        let mut read = Progress::default();
        read.read_names(&progress.names_text());
        assert_eq!((read.day, read.evening), (1, true));
        read.read_names("names=a|b|c\n");
        assert_eq!((read.day, read.evening), (0, false), "a save of before: the first day, morning");
    }

    #[test]
    fn sentry_duty_is_stood_once_a_day_and_its_points_saved() {
        let mut progress = Progress { sentry_points: 4200, sentry_today: true, ..Progress::default() };
        let mut read = Progress::default();
        read.read_names(&progress.names_text());
        assert_eq!((read.sentry_points, read.sentry_today), (4200, true));
        progress.next_day();
        read.read_names(&progress.names_text());
        assert_eq!((read.sentry_points, read.sentry_today), (4200, false), "a new day: on duty again");
    }

    #[test]
    fn the_partner_left_at_home_and_the_team_s_name_are_saved() {
        let mut progress = Progress { partner_home: true, ..Progress::default() };
        progress.rename(Whose::Team, "  Thorns ");
        let mut read = Progress::default();
        read.read_names(&progress.names_text());
        assert_eq!((read.partner_home, read.names.2.as_str()), (true, "Thorns"));
        read.read_names("names=a|b|c\n");
        assert!(!read.partner_home, "a save of before: all come");
    }

    #[test]
    fn an_sos_is_saved_as_text() {
        let sos = Sos { dungeon: 12, floor: 4, items: vec![70, 109], money: 250, sent: true, trips: 1 };
        assert_eq!(Sos::from_text(&sos.to_text()), Some(sos.clone()));
        let empty = Sos { items: Vec::new(), ..sos };
        assert_eq!(Sos::from_text(&empty.to_text()), Some(empty));
    }
}
