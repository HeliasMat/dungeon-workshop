//! Jobs on the floor: who waits, who is escorted, what lies there, the
//! outlaw; and a job done.

use crate::dungeon::{Floor, Pos, Stats, Statuses, Tile, ai};
use crate::jobs::{self, Job};

use super::attack::chebyshev;
use super::dungeon::{Ask, JobSpot};
use super::monster::Side;
use super::spawning::wild_monster;
use super::team::Recruit;
use super::{Book, Monster, MonsterId, Run};

/// What a job asks, with the names of its monster and its item.
pub fn job_title(job: &Job, book: &Book) -> String {
    job.title(|monster| book.species_name(monster), |item| book.item_name(item))
}

/// A job in full, as a board shows it read, by the names of the game's
/// monsters, items and dungeons.
pub fn job_details(job: &Job, book: &Book) -> String {
    job.details(|monster| book.species_name(monster), |item| book.item_name(item), |dungeon| book.dungeon_name(dungeon))
}

impl Run {
    /// The floor's jobs: who waits there to be found or for an item, and
    /// the outlaw, as one of the monsters of the floor. (Those who wait are
    /// `Side::Keeper`s with a `job_target`; a floor to reach and an item to
    /// find are `Dungeon::spots`.)
    pub(crate) fn place_job_targets(&mut self) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        dungeon.spots.clear();
        let (Some(entered), true) = (dungeon.id, dungeon.jobs) else {
            return;
        };
        let floor = dungeon.floor_number;
        let held = self.team.progress.jobs.clone();
        for (index, job) in held.iter().enumerate().filter(|(_, job)| !job.done && job.dungeon == entered && job.floor == floor) {
            self.place_job_target(index, job);
        }
    }

    fn place_job_target(&mut self, index: usize, job: &Job) {
        let book = self.book.clone();
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let spawn = dungeon.floor.player_spawn;
        // Somewhere in a room, away from where the team arrives.
        let free = |tile: &Pos| {
            matches!(dungeon.floor.tile(*tile), Tile::Floor { room: Some(_) }) && chebyshev(*tile, spawn) > 2 && !dungeon.floor.enemy_spawns.contains(tile)
        };
        let tiles: Vec<Pos> = Floor::positions().filter(free).collect();
        if tiles.is_empty() {
            return;
        }
        let tile = tiles[dungeon.rng.rand_int(tiles.len() as i32) as usize];
        // A floor to reach: reached. An item to find: where it lies.
        match job.kind {
            jobs::Kind::Explore => return dungeon.spots.push(JobSpot { job: index, at: spawn, item: None }),
            jobs::Kind::Find => return dungeon.spots.push(JobSpot { job: index, at: tile, item: Some(job.item) }),
            _ => {}
        }
        let outlaw = matches!(job.kind, jobs::Kind::Outlaw | jobs::Kind::Stolen);
        // Someone to escort: it comes along from the start, to who waits.
        let spec = dungeon.spec(&book);
        let waiting = if job.kind == jobs::Kind::Escort { job.item } else { job.client };
        if job.kind == jobs::Kind::Escort {
            let near = Floor::positions().find(|tile| chebyshev(*tile, spawn) == 1 && matches!(dungeon.floor.tile(*tile), Tile::Floor { .. }));
            let like = spec.and_then(|spec| spec.enemies.first());
            let stats = like.map_or([5, 30, 12, 12, 12, 12], |enemy| {
                let of = &enemy.stats;
                [of.level, of.max_hp, of.attack, of.defense, of.sp_attack, of.sp_defense]
            });
            let client = Recruit { species: job.client, name: book.species_name(job.client), stats, moves: Vec::new() };
            if let Some(at) = near
                && let Some(id) = self.place_recruit(None, &client, at)
                && let Some(monster) = self.dungeon.as_mut().and_then(|dungeon| dungeon.monster_mut(id))
            {
                monster.escorted = Some(index);
            }
        }
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        // As the floor's monster of that name, or its first.
        let name = book.species_name(waiting);
        let like = spec.and_then(|spec| spec.enemies.iter().find(|enemy| book.species_name(enemy.species) == name).or(spec.enemies.first()));
        let Some(enemy) = like.filter(|_| outlaw) else {
            if outlaw {
                return;
            }
            // The others only wait.
            let species = like.map_or(waiting, |enemy| enemy.species);
            let id = dungeon.new_id();
            let mut target =
                Monster::new(id, Side::Keeper, book.species_name(species), Some(species), super::book::ENEMY_STATS, Vec::new(), Statuses::default(), tile);
            target.job_target = Some(index);
            return dungeon.monsters.push(target);
        };
        // A hard outlaw comes with two of its gang, of its kind, as strong as
        // the floor's (on free tiles beside it).
        let movement = book.species(enemy.species).map_or(0, |species| species.movement);
        if job.with_gang() {
            let beside: Vec<Pos> = ai::DIRECTIONS
                .iter()
                .map(|&(dx, dy)| Pos::new(tile.x + dx, tile.y + dy))
                .filter(|at| matches!(dungeon.floor.tile(*at), Tile::Floor { .. }) && !dungeon.floor.enemy_spawns.contains(at))
                .take(2)
                .collect();
            for at in beside {
                let mut gang = wild_monster(dungeon, &book, enemy, at, Statuses::for_species(movement));
                gang.recruitable = None;
                dungeon.monsters.push(gang);
            }
        }
        // An outlaw is stronger than its kind, and worth more.
        let flees = dungeon.rng.rand_int(3) == 0;
        let mut outlaw = wild_monster(dungeon, &book, enemy, tile, Statuses::for_species(movement));
        let of = &enemy.stats;
        outlaw.stats = Stats { max_hp: of.max_hp * 2, hp: of.max_hp * 2, attack: of.attack + 2, ..*of };
        outlaw.exp_value = enemy.exp * 2;
        outlaw.recruitable = None;
        outlaw.job_target = Some(index);
        outlaw.fleeing = flees;
        dungeon.monsters.push(outlaw);
    }

    /// Jobs are done in the dungeon: someone found, an item brought to who
    /// waits for it, an outlaw defeated. A job done, it is asked whether to
    /// leave the dungeon at once, as the original asks. (Once a frame; a
    /// defeated outlaw must still be on the list.)
    pub(crate) fn job_targets(&mut self) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        // An outlaw that flees does, scared, while it has more than half its
        // HP (see the monsters' turns: a terrified one runs from the team).
        for outlaw in dungeon.monsters.iter_mut().filter(|monster| monster.fleeing) {
            let strong = outlaw.stats.hp * 2 > outlaw.stats.max_hp;
            outlaw.status.terrified_turns = if strong { outlaw.status.terrified_turns.max(2) } else { 0 };
        }
        let Some(leader) = dungeon.monsters.iter().find(|monster| monster.side == Side::Leader).map(|leader| leader.at) else {
            return;
        };
        let targets: Vec<(usize, Pos, Option<MonsterId>)> = dungeon
            .monsters
            .iter()
            .filter_map(|monster| Some((monster.job_target?, monster.at, Some(monster.id))))
            .chain(dungeon.spots.iter().map(|spot| (spot.job, spot.at, None)))
            .collect();
        for (index, at, who) in targets {
            let Some(job) = self.team.progress.jobs.get(index).copied().filter(|job| !job.done) else {
                continue;
            };
            if !self.job_done(&job, index, at, who, leader) {
                continue;
            }
            self.team.progress.jobs[index].done = true;
            let Some(dungeon) = self.dungeon.as_mut() else {
                return;
            };
            if !matches!(job.kind, jobs::Kind::Outlaw | jobs::Kind::Stolen) {
                dungeon.monsters.retain(|monster| Some(monster.id) != who);
                dungeon.spots.retain(|spot| spot.job != index);
            }
            if dungeon.ask.is_none() && dungeon.outcome.is_none() {
                dungeon.ask = Some(Ask::Leave);
            }
        }
    }

    /// Whether a job of the floor is done, told.
    fn job_done(&mut self, job: &Job, index: usize, at: Pos, who: Option<MonsterId>, leader: Pos) -> bool {
        let book = self.book.clone();
        let name = who.map_or(String::new(), |who| self.name_of(who));
        let Some(dungeon) = self.dungeon.as_mut() else {
            return false;
        };
        match job.kind {
            jobs::Kind::Outlaw | jobs::Kind::Stolen => {
                let down = who.and_then(|who| dungeon.monster(who)).is_some_and(|outlaw| !outlaw.alive());
                if down && job.kind == jobs::Kind::Stolen {
                    self.log.push(format!("{name} gave back {}! Mission accomplished.", book.item_name(job.item)));
                } else if down {
                    self.log.push(format!("{name} was arrested! Mission accomplished."));
                }
                down
            }
            jobs::Kind::Explore => {
                self.log.push(format!("Floor {} reached! Mission accomplished.", job.floor));
                true
            }
            // Found by coming next to it.
            _ if chebyshev(at, leader) > 1 => false,
            jobs::Kind::Deliver => match self.bag.items.iter().position(|&item| item == job.item) {
                Some(place) => {
                    let item = book.item_name(self.bag.items.remove(place));
                    self.log.push(format!("{name} received {item}. Mission accomplished."));
                    true
                }
                None => {
                    // Said once each time the leader comes by.
                    let told = who.map(|who| (who, leader));
                    if dungeon.told != told {
                        dungeon.told = told;
                        self.log.push(format!("{name} is waiting for: {}.", book.item_name(job.item)));
                    }
                    false
                }
            },
            jobs::Kind::Find => {
                self.log.push(format!("{} was found! Mission accomplished.", book.item_name(job.item)));
                true
            }
            jobs::Kind::Escort => {
                // (The one escorted leaves with who it was taken to.)
                let Some(place) = dungeon.monsters.iter().position(|monster| monster.escorted == Some(index)) else {
                    return false;
                };
                let client = dungeon.monsters.remove(place);
                self.log.push(format!("{} found {name}! Mission complete.", client.name));
                true
            }
            _ => {
                self.log.push(format!("{name} was rescued! Mission accomplished."));
                true
            }
        }
    }
}
