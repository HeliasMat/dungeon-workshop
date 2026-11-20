//! Monsters that come onto a floor: the floor's own, Kecleon on a thief.

use crate::dungeon::{Pos, Stats, Statuses, ai, pick_weighted, status, wild_moves};
use crate::tables::Enemy;

use super::book::ENEMY_STATS;
use super::monster::Recruitable;
use super::{Book, Dungeon, Monster, MonsterId, Side};

pub(crate) const KECLEON: u16 = 383;

/// A monster of the floor's table, as it is there.
pub(crate) fn wild_monster(dungeon: &mut Dungeon, book: &Book, enemy: &Enemy, at: Pos, status: Statuses) -> Monster {
    let learnset = book.species(enemy.species).map_or(&[][..], |species| &species.learnset[..]);
    let moves = wild_moves(&mut dungeon.rng, learnset, enemy.stats.level);
    let id = dungeon.new_id();
    let mut monster = Monster::new(id, Side::Foe, book.species_name(enemy.species), Some(enemy.species), enemy.stats, moves, status, at);
    monster.exp_value = enemy.exp;
    monster.recruitable = Some(Recruitable { species: enemy.species, rate: enemy.recruit_rate });
    monster
}

/// An enemy picked from the floor's spawn list (a stand-in without one) on
/// a tile.
pub(crate) fn spawn_enemy(dungeon: &mut Dungeon, book: &Book, at: Pos, dormant: bool) -> MonsterId {
    spawn_enemy_holding(dungeon, book, at, dormant, false)
}

/// The same, holding a grudge or not (a Grudge Trap's monsters: whoever
/// knocks one out loses the PP of that move).
pub(crate) fn spawn_enemy_holding(dungeon: &mut Dungeon, book: &Book, at: Pos, dormant: bool, grudge: bool) -> MonsterId {
    let spec = dungeon.spec(book);
    let picked = spec.and_then(|floor| pick_weighted(&mut dungeon.rng, floor.enemies.iter().map(|enemy| (enemy, enemy.weight))));
    let movement = picked.and_then(|enemy| book.species(enemy.species)).map_or(0, |species| species.movement);
    let mut statuses = Statuses::for_species(movement);
    if !dormant && !grudge && picked.is_some_and(|enemy| falls_asleep(dungeon, enemy.asleep_chance, at)) {
        statuses.sleep_turns = status::DEEP_SLEEP;
    }
    statuses.grudge = grudge;
    let mut monster = match picked {
        Some(enemy) => wild_monster(dungeon, book, enemy, at, statuses),
        None => Monster::new(dungeon.new_id(), Side::Foe, "Ennemi".to_string(), None, ENEMY_STATS, Vec::new(), statuses, at),
    };
    if dormant {
        // Asleep until the leader walks into the Monster House.
        monster.status = Statuses::for_species(movement);
        monster.status.sleep_turns = i32::MAX / 2;
        monster.dormant = true;
    }
    let id = monster.id;
    dungeon.monsters.push(monster);
    id
}

/// Whether a monster appearing on `at` comes asleep, to stay so until hit:
/// the floor's chance in 100, halved where the leader sees it come.
fn falls_asleep(dungeon: &mut Dungeon, chance: i32, at: Pos) -> bool {
    let leader = dungeon.monsters.iter().find(|monster| monster.side == Side::Leader).map(|monster| monster.at);
    let seen = leader.is_some_and(|leader| ai::can_see(&dungeon.floor, leader, at));
    let chance = if seen { chance / 2 } else { chance };
    chance > 0 && dungeon.rng.rand_int(100) < chance
}

/// A Kecleon hunting a thief: well above the team.
pub(crate) fn hunter_stats(leader_level: i32) -> Stats {
    let level = (leader_level + 30).min(100);
    Stats {
        level,
        max_hp: 60 + level * 4,
        hp: 60 + level * 4,
        attack: 20 + level * 2,
        defense: 15 + level * 2,
        sp_attack: 20 + level * 2,
        sp_defense: 15 + level * 2,
        types: (1, 0),
        ..ENEMY_STATS
    }
}

/// Kecleon turns on a thief: a foe much stronger than the floor's.
pub(crate) fn spawn_thief_hunter(keeper: &mut Monster, leader_level: i32) {
    keeper.stats = hunter_stats(leader_level);
    keeper.side = Side::Foe;
    keeper.species = Some(KECLEON);
    keeper.exp_value = keeper.stats.level * 20;
    keeper.moves = Vec::new();
}
