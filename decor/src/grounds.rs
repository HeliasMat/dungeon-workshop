//! The ground of every place of the game, written once in
//! `data/extra/ground.txt` (`decor ground_swatches`): where one can walk, a tile of 8
//! pixels at a time. (The places this builder makes have theirs in
//! `decor_ground.txt`.)
//!
//! Where the game gives a grid of walls (`town.txt`), it is kept as it is.
//! The other places are traced by hand in `library/tables/grounds.txt`: polygons of
//! ground, holes and round rocks in them, a soil of one colour read off the
//! picture; a place not named there is a picture no one walks on.

use std::collections::BTreeMap;
use std::path::Path;

/// How much of a place a grid of the game holds solid to say anything of it.
const WHOLE_GRID: f32 = 0.05;

/// A place's rows of tiles, true where solid; and how they were made (`jeu`: the game's grid, `main`: traced).
type Rows = Vec<Vec<bool>>;

/// What `library/tables/grounds.txt` says of a place: its words and their numbers.
type Said = Vec<(String, Vec<i32>)>;

/// The places whose walls the game gives (`town.txt`), by level.
fn game_walls(data: &str) -> BTreeMap<u16, Rows> {
    let mut found: BTreeMap<u16, Rows> = BTreeMap::new();
    let (mut level, mut high) = (None, 0);
    for line in assets::data::text(data, "town.txt").lines() {
        let row: Vec<&str> = line.split('\t').collect();
        match row.first().copied() {
            Some("place") => {
                level = row.get(1).and_then(|level| level.parse().ok());
                high = row.get(4).and_then(|high| high.parse().ok()).unwrap_or(0);
            }
            Some("solid") => {
                if let (Some(level), Some(tiles)) = (level, row.get(1)) {
                    let rows = found.entry(level).or_default();
                    if rows.len() < high {
                        rows.push(tiles.chars().map(|tile| tile == '1').collect());
                    }
                }
            }
            _ => {}
        }
    }
    found.retain(|_, rows| !rows.is_empty());
    found
}

/// What `library/tables/grounds.txt` says, by level.
fn traced_by_hand(root: &Path) -> BTreeMap<u16, Said> {
    let text = std::fs::read_to_string(root.join("library/tables/grounds.txt")).unwrap_or_default();
    let mut said: BTreeMap<u16, Said> = BTreeMap::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split('#').next().unwrap_or_default().split_whitespace().collect();
        let [level, word, numbers @ ..] = words.as_slice() else { continue };
        let Ok(level) = level.parse() else { continue };
        said.entry(level).or_default().push((word.to_string(), numbers.iter().filter_map(|number| number.parse().ok()).collect()));
    }
    said
}

/// Whether a point is inside a polygon (its corners in turn).
fn inside(points: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut within = false;
    let mut last = points.len().saturating_sub(1);
    for (at, &(px, py)) in points.iter().enumerate() {
        let (qx, qy) = points[last];
        if (py > y) != (qy > y) && x < (qx - px) * (y - py) / (qy - py) + px {
            within = !within;
        }
        last = at;
    }
    within
}

/// The rows of a place traced by hand: nothing is ground but its `area`
/// polygons, without its `hole` polygons and its `rock` circles; a tile is
/// ground if most of it is.
fn traced(wide: usize, high: usize, said: &Said) -> Rows {
    let corners = |numbers: &[i32]| numbers.chunks(2).filter(|pair| pair.len() == 2).map(|pair| (pair[0] as f32, pair[1] as f32)).collect::<Vec<_>>();
    let areas: Vec<Vec<(f32, f32)>> = said.iter().filter(|(word, _)| word == "area").map(|(_, numbers)| corners(numbers)).collect();
    let holes: Vec<Vec<(f32, f32)>> = said.iter().filter(|(word, _)| word == "hole").map(|(_, numbers)| corners(numbers)).collect();
    let rocks: Vec<(f32, f32, f32)> =
        said.iter().filter(|(word, numbers)| word == "rock" && numbers.len() == 3).map(|(_, n)| (n[0] as f32, n[1] as f32, n[2] as f32)).collect();
    let ground = |x: f32, y: f32| {
        areas.iter().any(|area| inside(area, x, y))
            && !holes.iter().any(|hole| inside(hole, x, y))
            && !rocks.iter().any(|&(cx, cy, reach)| (x - cx).powi(2) + (y - cy).powi(2) <= reach * reach)
    };
    (0..high)
        .map(|row| {
            (0..wide)
                .map(|column| {
                    let walked = (0..64).filter(|at| ground((column * 8 + at % 8) as f32 + 0.5, (row * 8 + at / 8) as f32 + 0.5)).count();
                    walked < 32
                })
                .collect()
        })
        .collect()
}

/// The walls of a place whose ground is of one colour: a tile is a wall if
/// `share` percent of it is of another; a wall of fewer than `least` tiles is
/// a pebble one walks over.
fn off_soil(picture: &image::RgbImage, (red, green, blue, share, least): (i32, i32, i32, i32, i32)) -> Rows {
    let (wide, high) = (picture.width() as usize / 8, picture.height() as usize / 8);
    let colour = [red as u8, green as u8, blue as u8];
    let mut rows: Rows = (0..high)
        .map(|row| {
            (0..wide)
                .map(|column| {
                    let other = (0..64).filter(|at| picture.get_pixel((column * 8 + at % 8) as u32, (row * 8 + at / 8) as u32).0 != colour).count() as i32;
                    other * 100 >= share * 64
                })
                .collect()
        })
        .collect();
    // (Each wall as its tiles, corners touching: the small ones taken away.)
    let mut seen = vec![vec![false; wide]; high];
    for y in 0..high {
        for x in 0..wide {
            if !rows[y][x] || seen[y][x] {
                continue;
            }
            let (mut blob, mut waiting) = (Vec::new(), vec![(x, y)]);
            seen[y][x] = true;
            while let Some((x, y)) = waiting.pop() {
                blob.push((x, y));
                for (dx, dy) in (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (dx, dy))) {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < wide && (ny as usize) < high && rows[ny as usize][nx as usize] && !seen[ny as usize][nx as usize] {
                        seen[ny as usize][nx as usize] = true;
                        waiting.push((nx as usize, ny as usize));
                    }
                }
            }
            if (blob.len() as i32) < least {
                for (x, y) in blob {
                    rows[y][x] = false;
                }
            }
        }
    }
    rows
}

/// The ground of every place it can be told of, by level: how it was made, its rows.
fn grounds(data: &str, root: &Path) -> BTreeMap<u16, (&'static str, Rows)> {
    let said = traced_by_hand(root);
    let mut out: BTreeMap<u16, (&'static str, Rows)> = BTreeMap::new();
    let mut alike: Vec<(u16, u16)> = Vec::new();
    for (level, told) in &said {
        if let Some((_, numbers)) = told.iter().find(|(word, _)| word == "like") {
            if let Some(&twin) = numbers.first() {
                alike.push((*level, twin as u16));
            }
            continue;
        }
        if !told.iter().any(|(word, _)| word == "area") {
            continue;
        }
        let Ok(picture) = image::open(assets::data::path(data, &format!("backdrops/{level}.png"))) else { continue };
        let picture = picture.to_rgb8();
        let mut rows = traced(picture.width() as usize / 8, picture.height() as usize / 8, told);
        for (_, numbers) in told.iter().filter(|(word, numbers)| word == "soil" && numbers.len() == 5) {
            let walled = off_soil(&picture, (numbers[0], numbers[1], numbers[2], numbers[3], numbers[4]));
            for (row, wall) in rows.iter_mut().zip(walled) {
                for (tile, solid) in row.iter_mut().zip(wall) {
                    *tile |= solid;
                }
            }
        }
        out.insert(*level, ("main", rows));
    }
    // (The places this builder made write their ground themselves.)
    let made: Vec<u16> = assets::data::text(data, "decors.txt").lines().filter_map(|line| line.split('\t').nth(1)?.parse().ok()).collect();
    for (level, rows) in game_walls(data) {
        let solid = rows.iter().flatten().filter(|solid| **solid).count() as f32;
        let all = (rows.len() * rows.first().map_or(1, Vec::len)).max(1) as f32;
        if !made.contains(&level) && solid / all >= WHOLE_GRID {
            out.insert(level, ("jeu", rows));
        }
    }
    for (level, twin) in alike {
        if let Some(found) = out.get(&twin).cloned() {
            out.insert(level, found);
        }
    }
    out
}

/// `data/extra/ground.txt` written: the ground of every place of the game it
/// can be told of; how many places.
pub fn write(data: &str, extra: &Path, root: &Path) -> Result<usize, String> {
    let found = grounds(data, root);
    let mut text = String::new();
    for (level, (how, rows)) in &found {
        let rows: Vec<String> = rows.iter().map(|row| row.iter().map(|solid| if *solid { '1' } else { '0' }).collect()).collect();
        text.push_str(&format!("{level}\t{how}\t{}\n", rows.join("/")));
    }
    std::fs::create_dir_all(extra).map_err(|why| format!("{} : {why}", extra.display()))?;
    let path = extra.join("ground.txt");
    std::fs::write(&path, text).map_err(|why| format!("{} : {why}", path.display()))?;
    Ok(found.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_traced_place_is_ground_inside_its_area_but_not_in_its_holes_and_rocks() {
        let said: Said = vec![
            ("area".to_string(), vec![0, 0, 64, 0, 64, 64, 0, 64]),
            ("hole".to_string(), vec![0, 0, 16, 0, 16, 16, 0, 16]),
            ("rock".to_string(), vec![44, 44, 8]),
        ];
        let rows = traced(10, 10, &said);
        assert!(!rows[4][4], "inside the area: ground");
        assert!(rows[0][0], "in the hole: a wall");
        assert!(rows[5][5], "the rock: a wall");
        assert!(rows[9][9], "outside the area: a wall");
    }
}
