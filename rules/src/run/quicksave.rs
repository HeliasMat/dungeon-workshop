//! Quicksaves: the floor is not kept, only the RNG's state it came from, so
//! that going back restarts it with its layout and its monsters. The text
//! comes out as `Sight::Save`; the disk is the game's.
//! The dungeon's number follows it: the top menu takes the exploration up
//! from there, as the original's "Resume". (Older quicksaves have none.)

use crate::dungeon::save::{PartnerSave, SaveData};
use crate::dungeon::{Belly, DungeonRng, Statuses};

use super::team::{Experience, LINKED_MOVE, known_moves};
use super::{Monster, Run, Side, Sight};

/// How a quicksave tells its dungeon.
const DUNGEON_LINE: &str = "dungeon=";

/// The dungeon a quicksave was made in.
pub fn quicksaved_dungeon(text: &str) -> Option<u16> {
    text.lines().find_map(|line| line.strip_prefix(DUNGEON_LINE))?.parse().ok()
}

fn ids(monster: &Monster) -> Vec<u16> {
    monster.moves.iter().map(|slot| slot.id).collect()
}

fn pp(monster: &Monster) -> Vec<i32> {
    monster.moves.iter().map(|slot| slot.pp).collect()
}

fn exp(monster: &Monster) -> i32 {
    monster.experience.as_ref().map_or(0, |experience| experience.growth.points)
}

fn partner_save(monster: &Monster) -> PartnerSave {
    PartnerSave { stats: monster.stats, moves: ids(monster), pp: pp(monster), exp: exp(monster) }
}

impl Run {
    pub(crate) fn quicksave(&mut self) {
        // (A save knows the hero as the one moved.)
        self.hero_controlled();
        let (items, money) = (self.bag.items.clone(), self.bag.money);
        let dungeon = self.here();
        let leader = dungeon.leader();
        let allies = || dungeon.monsters.iter().filter(|monster| monster.side == Side::Ally);
        let save = SaveData {
            floor_seed: dungeon.floor_seed,
            floor_number: dungeon.floor_number,
            turn: dungeon.turn,
            stats: leader.stats,
            pp: pp(leader),
            bag: items,
            belly: leader.belly.unwrap_or_else(Belly::full).raw(),
            sleep_turns: leader.status.sleep_turns,
            poisoned: leader.status.poisoned,
            moves: ids(leader),
            exp: Some(exp(leader)),
            money,
            partner: allies().find(|ally| ally.extra.is_none()).map(partner_save),
            extras: allies().filter(|ally| ally.extra.is_some()).map(partner_save).collect(),
        };
        let mut text = save.to_text();
        if let Some(id) = dungeon.id {
            text.push_str(&format!("\n{DUNGEON_LINE}{id}\n"));
        }
        self.show(Sight::Save(text));
        self.log.push("Game saved.".to_string());
    }

    /// Restarts the saved floor, the team as it was.
    pub(crate) fn quickload(&mut self, text: &str) {
        let Some(save) = SaveData::from_text(text) else {
            self.log.push("No valid save.".to_string());
            return;
        };
        let founder = &self.team.leader;
        let partner = &self.team.partner;
        let moves = known_moves(&founder.moves, &founder.growth, &save.moves, Some(&save.pp));
        let mut experience = Experience::new(&founder.growth, save.stats.level);
        if let Some(points) = save.exp {
            experience.growth.points = points;
        }
        let ally_kept = save.partner.as_ref().map(|saved| {
            let mut experience = Experience::new(&partner.growth, saved.stats.level);
            experience.growth.points = saved.exp;
            (saved.stats, known_moves(&partner.moves, &partner.growth, &saved.moves, Some(&saved.pp)), experience, partner.movement)
        });
        (self.bag.items, self.bag.money) = (save.bag.clone(), save.money);
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        dungeon.rng = DungeonRng::new(save.floor_seed);
        dungeon.floor_number = save.floor_number.max(1);
        dungeon.turn = save.turn;
        (dungeon.enemy_phase, dungeon.phase, dungeon.wait) = (false, None, 0);
        let leader = dungeon.leader_mut();
        (leader.stats, leader.moves, leader.experience) = (save.stats, moves, Some(experience));
        leader.tough = crate::dungeon::iq::has(&leader.stats, crate::dungeon::iq::BRICK_TOUGH);
        leader.belly = Some(Belly::from_raw(save.belly));
        leader.status = Statuses::for_species(leader.status.movement);
        leader.status.sleep_turns = save.sleep_turns;
        if save.poisoned {
            leader.status.poison();
        }
        // The others of the team as they were (those still there, in the
        // order they came in).
        let extras = dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Ally && monster.extra.is_some());
        for (monster, saved) in extras.zip(&save.extras) {
            monster.stats = saved.stats;
            monster.tough = crate::dungeon::iq::has(&monster.stats, crate::dungeon::iq::BRICK_TOUGH);
            if let Some(experience) = &mut monster.experience {
                experience.growth.points = saved.exp;
            }
            for slot in &mut monster.moves {
                let kept = saved.moves.iter().position(|&id| id & !LINKED_MOVE == slot.id).and_then(|at| saved.pp.get(at));
                slot.pp = kept.copied().unwrap_or(slot.max_pp);
            }
        }
        let is_partner = |monster: &Monster| monster.side == Side::Ally && monster.extra.is_none();
        match ally_kept {
            Some((stats, moves, experience, movement)) => {
                if let Some(ally) = dungeon.monsters.iter_mut().find(|monster| is_partner(monster)) {
                    (ally.stats, ally.moves, ally.experience) = (stats, moves, Some(experience));
                    ally.tough = crate::dungeon::iq::has(&ally.stats, crate::dungeon::iq::BRICK_TOUGH);
                    ally.status = Statuses::for_species(movement);
                }
            }
            None => dungeon.monsters.retain(|monster| !is_partner(monster)),
        }
        self.new_floor();
        self.log.push("Game loaded.".to_string());
    }
}
