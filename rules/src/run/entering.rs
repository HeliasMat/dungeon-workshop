//! Going in a dungeon and coming out of it: who comes along and as what,
//! what the bag may hold, what is kept, lost and paid at the end.

use crate::dungeon::{Belly, Growth, MoveSlot, Statuses, stack};
use crate::jobs;

/// What a Thank-You mail brings: a Reviver Seed.
const REVIVER_SEED: u16 = 73;
/// The explorations an egg takes to hatch.
const EGG_TRIPS: u8 = 2;
use crate::services::{BAG_CAPACITY, MONEY_LIMIT};

use super::team::{Experience, Founder, Growing, Member, Recruit, Sos, known_moves};
use super::{Dungeon, Monster, MonsterId, Outcome, Run, Side, Sight};

/// Crystal Lake, the 5th Station Clearing: fainting there costs nothing.
const NO_LOSS_DUNGEONS: [u16; 2] = [26, 121];
/// The aura bows, Silver Bow to Fuchsia Bow: never lost.
const AURA_BOWS: std::ops::RangeInclusive<u16> = 428..=443;

/// A dungeon left: how, and what the screen is to say.
#[derive(Clone, Debug, PartialEq)]
pub struct Left {
    pub outcome: Outcome,
    /// Where it stopped.
    pub floor: u32,
    pub message: String,
}

impl Left {
    pub fn cleared(&self) -> bool {
        self.outcome == Outcome::Cleared
    }
}

/// A recruit at the base given so much Exp.: the levels it grows (what it
/// earned past its last level is not kept, the base keeping only levels).
pub(crate) fn grow_recruit(book: &super::Book, recruit: &mut Recruit, amount: i32) {
    let Some(setup) = book.member(recruit.species, false) else {
        return;
    };
    let [level, hp, attack, defense, sp_attack, sp_defense] = recruit.stats;
    let mut stats = crate::dungeon::Stats { level, max_hp: hp, hp, attack, defense, sp_attack, sp_defense, ..setup.stats };
    let mut growth = Growth::new(setup.growth.table.clone(), level);
    let _ = growth.gain(&mut stats, amount);
    recruit.stats = [stats.level, stats.max_hp, stats.attack, stats.defense, stats.sp_attack, stats.sp_defense];
}

/// A recruit as the guild keeps it.
struct Kept<'a> {
    stats: [i32; 6],
    moves: &'a [u16],
}

impl Run {
    /// Enters a dungeon the team chose: its jobs can be done there.
    pub fn enter_dungeon(&mut self, id: u16, seed: u32) {
        self.enter(id, seed, true);
    }

    /// Enters a dungeon of the story: no job counts there.
    pub fn enter_story_dungeon(&mut self, id: u16, seed: u32) {
        self.enter(id, seed, false);
    }

    /// A new run: fresh floors, the team as its last dungeon left it, the
    /// bag and the money along. The song of the dungeon comes out with the
    /// next `step`.
    fn enter(&mut self, id: u16, seed: u32, jobs: bool) {
        let book = self.book.clone();
        let table = book.tables.dungeons.get(id as usize);
        let song = table.map(|table| table.music).filter(|&song| song != crate::tables::NO_MUSIC);
        let name = book.dungeon_name(id);
        let mut dungeon = Dungeon::new(table.map(|_| id), table, name.clone(), song, seed);
        dungeon.jobs = jobs;
        // (The settings' options of the rules, on top of the dungeon's.)
        dungeon.rules.options = dungeon.rules.options.or(self.options.rules);
        self.place_members(&mut dungeon);
        self.log.clear();
        // A maze of the Dojo takes the bag's items (its door says so).
        if crate::dojo::is_maze(id) {
            self.bag.items.clear();
        }
        // What the dungeon does not let in waits outside.
        let max_items = dungeon.rules.max_items as usize;
        if self.bag.items.len() > max_items {
            dungeon.kept_out.0 = self.bag.items.split_off(max_items);
        }
        if !dungeon.rules.money_allowed {
            dungeon.kept_out.1 = std::mem::take(&mut self.bag.money);
        }
        if !dungeon.kept_out.0.is_empty() || dungeon.kept_out.1 > 0 {
            self.log.push("What the dungeon does not allow waits outside.".to_string());
        }
        self.log.push(format!("{} entered: {name}.", dungeon.leader().name));
        self.dungeon = Some(dungeon);
        self.show(Sight::Music(song));
        self.new_floor();
    }

    /// A name given, or the one of the species.
    fn given_name(given: &str, founder: &Founder) -> String {
        if given.is_empty() { founder.name.clone() } else { given.to_string() }
    }

    /// The team as its last dungeon left it; where the dungeon brings it
    /// back to level 1, as its species is at level 1 (what it grows into
    /// there is not kept).
    fn place_members(&self, dungeon: &mut Dungeon) {
        let team = &self.team;
        let progress = &team.progress;
        let level_one = dungeon.rules.level_one;
        let member = |kept: &Option<Member>, founder: &Founder| {
            let fresh = Member::starting(founder.stats, &founder.growth, level_one);
            let member = kept.as_ref().filter(|_| !level_one).unwrap_or(&fresh);
            member.restore(&founder.moves, &founder.growth, None)
        };
        let (stats, moves, experience) = member(&progress.leader, &team.leader);
        let leader_id = dungeon.new_id();
        let mut leader = Monster::new(
            leader_id,
            Side::Leader,
            Self::given_name(&progress.names.0, &team.leader),
            Some(team.species.0),
            stats,
            moves,
            Statuses::for_species(team.leader.movement),
            dungeon.floor.player_spawn,
        );
        leader.experience = Some(experience);
        leader.belly = Some(Belly::full());
        dungeon.monsters.push(leader);
        dungeon.hero = Some(leader_id);
        // (The dungeon's limit on the team's size.)
        let mut room = (dungeon.rules.max_party.max(1) as usize).saturating_sub(1);
        if !team.alone && room >= 1 {
            room -= 1;
            let (stats, moves, experience) = member(&progress.partner, &team.partner);
            let id = dungeon.new_id();
            let mut partner = Monster::new(
                id,
                Side::Ally,
                Self::given_name(&progress.names.1, &team.partner),
                Some(team.species.1),
                stats,
                moves,
                Statuses::for_species(team.partner.movement),
                dungeon.floor.player_spawn,
            );
            partner.experience = Some(experience);
            dungeon.monsters.push(partner);
        }
        // Up to two more come along: a special episode's third (once the
        // episode has shown it), or the first of the recruits.
        let third = team.third.and_then(|row| self.book.tables.episode_characters.get(row));
        match third {
            Some((monster, level, known)) if team.third_joined => {
                self.come_along(dungeon, &mut room, None, *monster, None, *level, known.clone(), None);
            }
            Some(_) => {}
            None => {
                let recruits = &progress.recruits;
                let first: Vec<usize> = (0..recruits.len().min(2)).collect();
                for &at in progress.chosen.as_ref().unwrap_or(&first).iter().take(2) {
                    if let Some(recruit) = recruits.get(at) {
                        self.come_along(
                            dungeon,
                            &mut room,
                            Some(at),
                            recruit.species,
                            Some(recruit.name.clone()),
                            recruit.stats[0],
                            Vec::new(),
                            Some(Kept { stats: recruit.stats, moves: &recruit.moves }),
                        );
                    }
                }
            }
        }
        // (Their types as the tables now have them: a dungeon of one's own
        // may give a species or a move others, see `own::retype`.)
        for monster in &mut dungeon.monsters {
            if let Some(kind) = monster.species.and_then(|species| self.book.species(species)) {
                monster.stats.types = kind.types;
            }
            for slot in &mut monster.moves {
                if let Some(known) = self.book.move_slot(slot.id) {
                    slot.type_id = known.type_id;
                }
            }
        }
    }

    /// One more member, if there is room: as its species is at that level;
    /// a recruit, as it was left.
    fn come_along(
        &self,
        dungeon: &mut Dungeon,
        room: &mut usize,
        recruit: Option<usize>,
        monster: u16,
        name: Option<String>,
        level: i32,
        known: Vec<MoveSlot>,
        kept: Option<Kept>,
    ) {
        if *room == 0 {
            return;
        }
        let Some(setup) = self.book.member(monster, false) else {
            return;
        };
        *room -= 1;
        let mut stats = setup.stats;
        let growing: &Growing = &setup.growth;
        let mut growth = Growth::new(growing.table.clone(), stats.level);
        let needed = growing.table.get((level - 1).max(0) as usize).map_or(0, |entry| entry.total_exp) - growth.points;
        let _ = growth.gain(&mut stats, needed.max(0));
        let mut known = known;
        if let Some(Kept { stats: [_, hp, attack, defense, sp_attack, sp_defense], moves }) = kept {
            (stats.max_hp, stats.hp, stats.attack, stats.defense) = (hp, hp, attack, defense);
            (stats.sp_attack, stats.sp_defense) = (sp_attack, sp_defense);
            if !moves.is_empty() {
                known = known_moves(&setup.moves, growing, moves, None);
            }
        }
        let learnt: Vec<MoveSlot> = growing.learnset.iter().filter(|(at, _)| *at as i32 <= stats.level).map(|(_, slot)| *slot).collect();
        let mut moves = match (known.is_empty(), learnt.is_empty()) {
            (false, _) => known,
            (true, false) => learnt[learnt.len().saturating_sub(4)..].to_vec(),
            (true, true) => setup.moves.clone(),
        };
        moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp);
        let id: MonsterId = dungeon.new_id();
        let mut member = Monster::new(
            id,
            Side::Ally,
            name.unwrap_or(setup.name.clone()),
            Some(monster),
            stats,
            moves,
            Statuses::for_species(setup.movement),
            dungeon.floor.player_spawn,
        );
        member.experience = Some(Experience::new(growing, stats.level));
        member.extra = Some(recruit);
        dungeon.monsters.push(member);
    }

    /// The run is over (`Dungeon::outcome`): what the team has become is
    /// kept, what fainting costs is lost, what was kept out comes back.
    /// `None`: no dungeon to leave.
    pub fn leave_dungeon(&mut self) -> Option<Left> {
        let mut dungeon = self.dungeon.take()?;
        let progress = &mut self.team.progress;
        // (An exploration: the eggs at the Day Care grow, an SOS sent may be
        // answered.)
        progress.eggs.iter_mut().for_each(|(_, trips)| *trips = trips.saturating_sub(1));
        if let Some(sos) = progress.sos.as_mut().filter(|sos| sos.sent) {
            sos.trips = sos.trips.saturating_sub(1);
        }
        // (Not from a dungeon that brought the team back to its start.)
        for monster in dungeon.monsters.iter().filter(|_| !dungeon.rules.level_one) {
            let Some(experience) = &monster.experience else {
                continue;
            };
            let member = Member::leaving(&monster.stats, &monster.moves, experience);
            // (The hero, even when the player moved another: `switch_control`.)
            let hero = dungeon.hero.map_or(monster.side == Side::Leader, |hero| hero == monster.id);
            match (hero, monster.extra) {
                (true, _) => progress.leader = Some(member),
                (_, None) => progress.partner = Some(member),
                // (A recruit: its level and what it has grown to.)
                (_, Some(recruit)) => {
                    if let Some(recruit) = recruit.and_then(|at| progress.recruits.get_mut(at)) {
                        let stats = &monster.stats;
                        recruit.stats = [stats.level, stats.max_hp, stats.attack, stats.defense, stats.sp_attack, stats.sp_defense];
                        recruit.moves = monster.moves.iter().map(|slot| slot.id).collect();
                    }
                }
            }
        }
        // (Those waiting at the base get the Exp. shared: `RuleOptions::exp_share`.)
        if dungeon.shared_exp > 0 && !dungeon.rules.level_one {
            let came: Vec<usize> = dungeon.monsters.iter().filter_map(|monster| monster.extra.flatten()).collect();
            for (_, recruit) in progress.recruits.iter_mut().enumerate().filter(|(at, _)| !came.contains(at)) {
                grow_recruit(&self.book, recruit, dungeon.shared_exp);
            }
        }
        // The shops restock meanwhile, and the boards change their jobs.
        progress.stock = [None, None];
        progress.offers.clear();
        progress.outlaws.clear();
        // Fainting costs the jobs taken on, half the money, and about half
        // of the bag: each item is a coin toss, but the aura bows, which
        // stay. Two dungeons (and the mazes) are forgiving and take nothing.
        let outcome = dungeon.outcome.unwrap_or(Outcome::Abandoned);
        if matches!(outcome, Outcome::Fainted | Outcome::BlownOut) {
            jobs::fail(&mut progress.jobs);
            if !dungeon.id.is_some_and(|id| NO_LOSS_DUNGEONS.contains(&id) || crate::dojo::is_maze(id)) {
                let lost_money = self.bag.money - self.bag.money / 2;
                self.bag.money /= 2;
                let rng = &mut dungeon.rng;
                let mut lost = Vec::new();
                self.bag.items.retain(|&item| {
                    let kept = AURA_BOWS.contains(&stack::id(item)) || rng.rand_int(100) >= 50;
                    if !kept {
                        lost.push(item);
                    }
                    kept
                });
                // (What is lost may come back by an SOS: see `send_sos`.)
                if let Some(id) = dungeon.id {
                    progress.sos = Some(Sos { dungeon: id, floor: dungeon.floor_number, items: lost, money: lost_money, ..Sos::default() });
                }
            }
        }
        // What was kept out is given back.
        self.bag.items.extend(dungeon.kept_out.0);
        self.bag.money += dungeon.kept_out.1;
        let message = match outcome {
            Outcome::Cleared => format!("Dungeon cleared in {} turns!", dungeon.turn),
            Outcome::Fainted => format!("Fainted on floor {}...", dungeon.floor_number),
            Outcome::BlownOut => "The team was blown out of the dungeon by the wind...".to_string(),
            Outcome::Escaped => "The team escaped from the dungeon.".to_string(),
            Outcome::Abandoned => "Dungeon given up.".to_string(),
        };
        Some(Left { outcome, floor: dungeon.floor_number, message })
    }

    /// Back from a dungeon the team chose: the jobs done are paid, their
    /// items in the bag while it has room, else in storage; their points to
    /// the exploration rank. What is to be told, items by the name scenes
    /// give them (`{item:12}`); `None`: no job was done.
    pub fn pay_jobs(&mut self) -> Option<String> {
        let progress = &mut self.team.progress;
        let book = self.book.clone();
        let tables = &book.tables.jobs;
        let payment = jobs::paid(tables, &mut progress.jobs, progress.chapter >= jobs::GRADUATION)?;
        self.bag.money = (self.bag.money + payment.money).min(MONEY_LIMIT);
        let items: Vec<String> = payment.items.iter().map(|item| format!(" + {{item:{item}}}")).collect();
        let mut told = format!("Mission accomplished!\nReward: {} Poké{}", payment.money, items.concat());
        let (before, points) = (progress.rank, progress.rank + payment.points);
        progress.rank = points;
        let prizes = tables.prizes_between(before, points);
        for item in payment.items.iter().chain(&prizes).copied() {
            match self.bag.items.len() < BAG_CAPACITY {
                true => self.bag.items.push(item),
                false => progress.storage.push(item),
            }
        }
        if tables.rank_name(before) != tables.rank_name(points) {
            told.push_str(&format!("\nNew exploration rank: *{}*!", tables.rank_name(points)));
            told.extend(prizes.iter().map(|item| format!("\nA gift from the guild: {{item:{item}}}")));
        }
        for client in payment.joining {
            told.push_str(&format!("\n{} joined the team!", book.species_name(client)));
            self.recruit_species(client);
        }
        for client in payment.eggs {
            told.push_str(&format!("\n{} entrusts you with an Egg! It is waiting at the Day Care.", book.species_name(client)));
            self.team.progress.eggs.push((book.family(client), EGG_TRIPS));
        }
        Some(told)
    }

    /// A monster of that species joins the team, as the game makes it.
    pub fn recruit_species(&mut self, species: u16) {
        let name = self.book.species_name(species);
        let stats = self.book.member(species, false).map_or([5, 30, 12, 12, 12, 12], |member| {
            let of = &member.stats;
            [of.level, of.max_hp, of.attack, of.defense, of.sp_attack, of.sp_defense]
        });
        self.add_recruit(Recruit { species, name, stats, moves: Vec::new() });
    }

    /// Pelipper's mail: the team's last SOS, if any.
    pub fn sos(&self) -> Option<&Sos> {
        self.team.progress.sos.as_ref()
    }

    /// The SOS is sent: a friendly team will answer it after an exploration.
    pub fn send_sos(&mut self) -> String {
        match self.team.progress.sos.as_mut() {
            Some(sos) if !sos.sent => {
                (sos.sent, sos.trips) = (true, 1);
                "Pelipper flies off with your SOS! A friendly team will answer it soon. Come back after an exploration.".to_string()
            }
            _ => String::new(),
        }
    }

    /// The A-OK mail read: what was lost comes back, in the bag while it has
    /// room, else in storage; a Thank-You given (a Reviver Seed).
    pub fn read_aok(&mut self) -> String {
        let Some(sos) = self.team.progress.sos.take_if(|sos| sos.sent && sos.trips == 0) else {
            return String::new();
        };
        self.bag.money = (self.bag.money + sos.money).min(MONEY_LIMIT);
        for &item in sos.items.iter().chain(&[REVIVER_SEED]) {
            self.stow(item);
        }
        let mut told = format!(
            "A-OK Mail: a friendly team rescued you at {} (floor {})!\nIt gives you back {} Poké",
            self.book.dungeon_name(sos.dungeon),
            sos.floor,
            sos.money
        );
        told.push_str(&sos.items.iter().map(|item| format!(" + {{item:{item}}}")).collect::<String>());
        told.push_str(&format!(".\nYou send it a thank-you message... It gives you {{item:{REVIVER_SEED}}}!"));
        told
    }

    /// Chansey's Day Care: the eggs, and the explorations each still needs.
    pub fn eggs(&self) -> &[(u16, u8)] {
        &self.team.progress.eggs
    }

    /// The egg at that place hatches, if it is ready: what is told.
    pub fn hatch(&mut self, at: usize) -> String {
        match self.team.progress.eggs.get(at).copied() {
            Some((species, 0)) => {
                self.team.progress.eggs.remove(at);
                self.recruit_species(species);
                format!("Crack... The Egg hatched! It's {}!\n{} joined the team!", self.book.species_name(species), self.book.species_name(species))
            }
            Some((_, trips)) => format!("The Day Care: this Egg moves a little... It needs {trips} more exploration(s)!"),
            None => String::new(),
        }
    }
}
