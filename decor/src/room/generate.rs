//! A dungeon floor's plan made as the game makes one, roughly: rooms in a
//! grid of cells, some cells empty, corridors one tile wide joining them,
//! sometimes a pond; the team's arrival in one room, the stairs in the
//! farthest, traps and items here and there. The same seed, the same floor.
//! Written as a `.dungeon` to lay with the floor's own tiles.
//!
//! Letters: s the arrival, e the stairs, p q r traps, o t items, k a shop's
//! keeper, m a Monster House's middle, a b c... the other rooms' middles.

use std::collections::HashSet;

use crate::fault::Diagnostic;
use crate::random::Rng;

/// How many cells across and down; the fewest tiles a room is wide and high.
const CELLS: (usize, usize) = (3, 2);
const LEAST: (usize, usize) = (4, 3);
/// Letters: where the team arrives, the stairs, traps, items, a shop's
/// keeper, a Monster House, then each room's middle.
const ARRIVAL: char = 's';
const STAIRS: char = 'e';
const TRAPS: &str = "pqr";
const ITEMS: &str = "ot";
const SHOP: char = 'k';
const HOUSE: char = 'm';
const ROOMS: &str = "abcdfghijlnuvwxyz";
/// How often a floor has a shop, a Monster House (in a room of its own).
const SHOPS: f64 = 0.4;
const HOUSES: f64 = 0.3;

/// A room: x, y, wide, high, in tiles.
type Room = (usize, usize, usize, usize);
type At = (usize, usize);

/// A room in most cells, inside it with a wall all round.
fn rooms_of(chance: &mut Rng, columns: usize, rows: usize) -> Vec<Room> {
    let (cell_w, cell_h) = ((columns - 2) / CELLS.0, (rows - 2) / CELLS.1);
    let mut rooms = Vec::new();
    for cy in 0..CELLS.1 {
        for cx in 0..CELLS.0 {
            if chance.random() < 0.2 && rooms.len() + (CELLS.0 * CELLS.1 - cy * CELLS.0 - cx) > 3 {
                continue;
            }
            let w = chance.between(LEAST.0, LEAST.0.max(cell_w.saturating_sub(2)));
            let h = chance.between(LEAST.1, LEAST.1.max(cell_h.saturating_sub(2)));
            let x = 1 + cx * cell_w + chance.between(1, 1.max(cell_w as i64 - w as i64 - 1) as usize);
            let y = 1 + cy * cell_h + chance.between(1, 1.max(cell_h as i64 - h as i64 - 1) as usize);
            rooms.push((x, y, w, h));
        }
    }
    rooms
}

fn middle((x, y, w, h): Room) -> At {
    (x + w / 2, y + h / 2)
}

/// One tile wide, along one way then the other.
fn corridor(plan: &mut [Vec<char>], (x0, y0): At, (x1, y1): At, chance: &mut Rng) {
    if chance.random() < 0.5 {
        (x0.min(x1)..=x0.max(x1)).for_each(|x| plan[y0][x] = '.');
        (y0.min(y1)..=y0.max(y1)).for_each(|y| plan[y][x1] = '.');
    } else {
        (y0.min(y1)..=y0.max(y1)).for_each(|y| plan[y][x0] = '.');
        (x0.min(x1)..=x0.max(x1)).for_each(|x| plan[y1][x] = '.');
    }
}

/// Water in a corner of a big room, a tile from its walls, never on a
/// letter or a room's middle: the room stays one.
fn pond(plan: &mut [Vec<char>], (x, y, w, h): Room, chance: &mut Rng, kept: &HashSet<At>) {
    if w < 7 || h < 5 {
        return;
    }
    let (pw, ph) = (chance.between(2, w / 2 - 1), chance.between(2, h / 2));
    let px = if chance.random() < 0.5 { x + 1 } else { x + w - 1 - pw };
    let py = if chance.random() < 0.5 { y + 1 } else { y + h - 1 - ph };
    for (yy, row) in plan.iter_mut().enumerate().take(py + ph).skip(py) {
        for (xx, tile) in row.iter_mut().enumerate().take(px + pw).skip(px) {
            if !kept.contains(&(xx, yy)) && *tile == '.' {
                *tile = '~';
            }
        }
    }
}

/// The text of a `.room` for that floor.
pub fn make(name: &str, dungeon: &str, floor: i64, seed: i64, columns: usize, rows: usize) -> Result<String, Diagnostic> {
    if columns < 2 + 3 * CELLS.0 || rows < 2 + 3 * CELLS.1 {
        return Err(Diagnostic::said(format!("generate: {columns}x{rows}, too small")));
    }
    let mut chance = Rng::new(&[seed as u64]);
    let mut plan = vec![vec!['#'; columns]; rows];
    let rooms = rooms_of(&mut chance, columns, rows);
    if rooms.len() < 2 {
        return Err(Diagnostic::said(format!("generate: one room only ({columns}x{rows})")));
    }
    for &(x, y, w, h) in &rooms {
        for row in plan.iter_mut().take(y + h).skip(y) {
            row[x..x + w].fill('.');
        }
    }
    // (Each room joined to the next, in the order of the grid, and now and
    // then to one more: a loop, as the game's floors have.)
    for pair in rooms.windows(2) {
        corridor(&mut plan, middle(pair[0]), middle(pair[1]), &mut chance);
    }
    if rooms.len() > 3 && chance.random() < 0.6 {
        let other = 2 + chance.below(rooms.len() - 2);
        corridor(&mut plan, middle(rooms[0]), middle(rooms[other]), &mut chance);
    }
    // (The arrival in the first room, the stairs in the one farthest from it.)
    let start = middle(rooms[0]);
    let far_of = |room: &Room| middle(*room).0.abs_diff(start.0) + middle(*room).1.abs_diff(start.1);
    let far = rooms[1..].iter().fold(rooms[1], |best, room| if far_of(room) > far_of(&best) { *room } else { best });
    let mut letters: Vec<(At, char)> = vec![(start, ARRIVAL), (middle(far), STAIRS)];
    let named = |letters: &[(At, char)], at: At| letters.iter().any(|(known, _)| *known == at);
    // (A shop, a Monster House, each in a room of its own, but the first's.)
    for (letter, often) in [(SHOP, SHOPS), (HOUSE, HOUSES)] {
        let free: Vec<Room> = rooms.iter().copied().filter(|room| !named(&letters, middle(*room))).collect();
        if !free.is_empty() && chance.random() < often {
            let room = *chance.choice(&free);
            letters.push((middle(room), letter));
        }
    }
    let mut others = ROOMS.chars();
    for room in &rooms {
        if !named(&letters, middle(*room)) {
            letters.push((middle(*room), others.next().ok_or_else(|| Diagnostic::said("generate: too many rooms".to_string()))?));
        }
    }
    let tiles: Vec<At> = rooms.iter().flat_map(|&(rx, ry, rw, rh)| (rx..rx + rw).flat_map(move |tx| (ry..ry + rh).map(move |ty| (tx, ty)))).collect();
    for letter in TRAPS.chars().chain(ITEMS.chars()) {
        let free: Vec<At> =
            tiles.iter().copied().filter(|at| !named(&letters, *at) && letters.iter().all(|((x, y), _)| at.0.abs_diff(*x) + at.1.abs_diff(*y) > 2)).collect();
        if !free.is_empty() {
            let at = *chance.choice(&free);
            letters.push((at, letter));
        }
    }
    let kept: HashSet<At> = letters
        .iter()
        .flat_map(|((x, y), _)| (0..3).flat_map(move |dx| (0..3).map(move |dy| (x + dx, y + dy))))
        .filter_map(|(x, y)| Some((x.checked_sub(1)?, y.checked_sub(1)?)))
        .collect();
    if chance.random() < 0.7 {
        let room = *chance.choice(&rooms);
        pond(&mut plan, room, &mut chance, &kept);
    }
    for ((x, y), letter) in &letters {
        plan[*y][*x] = *letter;
    }
    let mut sorted: Vec<char> = letters.iter().map(|(_, letter)| *letter).collect();
    sorted.sort();
    let kinds: Vec<String> = sorted
        .iter()
        .map(|letter| {
            let kind = match *letter {
                ARRIVAL => "arrival",
                STAIRS => "stairs",
                SHOP => "a shop",
                HOUSE => "a Monster House",
                letter if TRAPS.contains(letter) => "trap",
                letter if ITEMS.contains(letter) => "item",
                _ => "a room",
            };
            format!("{letter} ({kind})")
        })
        .collect();
    let mut lines = vec![
        format!("# A floor made by decor generate (seed {seed}): {} rooms and their corridors.", rooms.len()),
        format!("# Spots: {}.", kinds.join(", ")),
        format!("dungeon: {name} {dungeon} {floor}"),
        "layout".to_string(),
    ];
    lines.extend(plan.iter().map(|row| row.iter().collect::<String>()));
    Ok(lines.join("\n") + "\n")
}
