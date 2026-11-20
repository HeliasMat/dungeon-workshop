//! Marowak's Dojo: its training mazes, one by type and the Final Maze. The
//! extract has none of them (`dungeons.txt` stops before them), so they are
//! made here as the tables are read: the floors of a dungeon of the
//! maze's look, the monsters of its type found anywhere in the game, near
//! the maze's level; no recruiting.

use crate::tables::{DungeonTable, Enemy, Floor, Tables};

/// The first maze's number, after the game's dungeons; the Normal Maze, then
/// one by type in the game's order, then the Final Maze.
pub const FIRST: u16 = 180;
pub const TYPES: [&str; 17] = [
    "Normal", "Fire", "Water", "Grass", "Electric", "Ice", "Fighting", "Poison", "Ground", "Flying", "Psychic", "Bug", "Rock", "Ghost", "Dragon", "Dark",
    "Steel",
];
pub const FINAL: u16 = FIRST + TYPES.len() as u16;
/// The menu of the Dojo (`game`'s own number).
pub const MENU: u16 = 0x7F02;

/// Floors, and the level the monsters are drawn near, of a type's maze and
/// of the Final Maze.
const FLOORS: usize = 5;
const LEVEL: i32 = 20;
const FINAL_FLOORS: usize = 12;
const FINAL_LEVEL: i32 = 50;
/// Monsters drawn on each floor, at most.
const KINDS: usize = 12;

/// The dungeon whose floors each type's maze takes the look of.
const LOOKS: [u16; 17] = [1, 79, 73, 85, 72, 77, 81, 85, 75, 83, 70, 85, 56, 81, 83, 81, 58];

/// Whether that dungeon is one of the mazes: the bag is left at its door,
/// fainting there costs nothing.
pub fn is_maze(dungeon: u16) -> bool {
    (FIRST..=FINAL).contains(&dungeon)
}

/// The mazes' names.
pub fn name(maze: u16) -> String {
    match maze {
        FINAL => "Final Maze".to_string(),
        _ => format!("{} Maze", TYPES.get((maze - FIRST) as usize).copied().unwrap_or("?")),
    }
}

/// The mazes open: one by type, and the Final Maze once the hole has
/// opened in the Dojo (`EVENT_M00_06`, after every other maze).
pub fn open(hole: bool) -> Vec<u16> {
    (FIRST..FINAL).chain(hole.then_some(FINAL)).collect()
}

/// The number the story keeps once every maze of a type is gone through,
/// and the one that says the hole has opened.
pub const DONE: &str = "dojo_done";
pub const HOLE: &str = "seen_EVENT_M00_06";

/// Adds the mazes to the tables read (none if the dungeons do not end where
/// the mazes start).
pub fn add_mazes(tables: &mut Tables) {
    if tables.dungeons.len() != FIRST as usize || tables.dungeons.len() <= *LOOKS.iter().max().unwrap_or(&0) as usize {
        return;
    }
    let mut mazes = Vec::new();
    for maze in FIRST..=FINAL {
        let kind = (maze - FIRST + 1) as u8;
        let look = LOOKS.get((maze - FIRST) as usize).copied().unwrap_or(83);
        let (floors, level) = if maze == FINAL { (FINAL_FLOORS, FINAL_LEVEL) } else { (FLOORS, LEVEL) };
        let enemies = monsters(tables, (maze != FINAL).then_some(kind), level);
        let source = &tables.dungeons[look as usize];
        let Some(floor) = source.floors.iter().find(|floor| floor.fixed.is_none()) else {
            return;
        };
        let mut rules = source.rules;
        rules.recruitment = false;
        rules.level_one = false;
        rules.goes_up = false;
        let floors = (0..floors)
            .filter_map(|_| {
                let mut floor: Floor = bincode::deserialize(&bincode::serialize(floor).ok()?).ok()?;
                floor.enemies = enemies.iter().filter_map(|enemy| bincode::deserialize(&bincode::serialize(enemy).ok()?).ok()).collect();
                Some(floor)
            })
            .collect();
        mazes.push(DungeonTable { goes_up: false, rules, music: source.music, floors });
    }
    for maze in FIRST..=FINAL {
        while tables.dungeon_names.len() <= maze as usize {
            tables.dungeon_names.push(String::new());
        }
        tables.dungeon_names[maze as usize] = name(maze);
    }
    tables.dungeons.extend(mazes);
}

/// The monsters of that type (any, for none) in the game's dungeons, each
/// once, the one nearest the level; as many kinds as a floor holds.
fn monsters(tables: &Tables, kind: Option<u8>, level: i32) -> Vec<&Enemy> {
    let mut found: Vec<&Enemy> = Vec::new();
    for enemy in tables.dungeons.iter().flat_map(|dungeon| &dungeon.floors).flat_map(|floor| &floor.enemies) {
        let types = tables.species.get(enemy.species as usize).map_or((0, 0), |species| species.types);
        if enemy.weight == 0 || kind.is_some_and(|kind| types.0 != kind && types.1 != kind) {
            continue;
        }
        let near = |of: &Enemy| (of.stats.level - level).abs();
        match found.iter_mut().find(|other| other.species == enemy.species) {
            Some(other) if near(enemy) < near(other) => *other = enemy,
            Some(_) => {}
            None => found.push(enemy),
        }
    }
    found.sort_by_key(|enemy| (enemy.stats.level - level).abs());
    found.truncate(KINDS);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mazes_are_named_and_opened() {
        assert_eq!(name(FIRST + 1), "Fire Maze");
        assert_eq!(name(FINAL), "Final Maze");
        assert_eq!(open(false).len(), 17);
        assert_eq!(open(true).last(), Some(&FINAL));
    }
}
