//! Where one can walk in a place: tiles of 8 pixels, and the ways round them.
//! The one truth of it, for who writes a scene as for who plays it.

use std::collections::VecDeque;
use std::sync::Arc;

use glam::Vec2;

/// The game's grid, in pixels.
pub const TILE: f32 = 8.0;

/// The tiles of a place, by row: true where solid.
#[derive(Clone, Debug, PartialEq)]
pub struct Ground {
    rows: Arc<Vec<Vec<bool>>>,
}

impl Ground {
    pub fn new(rows: Vec<Vec<bool>>) -> Self {
        Ground { rows: Arc::new(rows) }
    }

    /// Rows of `0` and `1` with `/` between; none for `-` (no ground known).
    pub fn parse(field: &str) -> Option<Ground> {
        (field != "-").then(|| Ground::new(field.split('/').map(|row| row.bytes().map(|tile| tile == b'1').collect()).collect()))
    }

    pub fn rows(&self) -> &[Vec<bool>] {
        &self.rows
    }

    /// Its size, in pixels.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.rows.first().map_or(0, Vec::len) as f32, self.rows.len() as f32) * TILE
    }

    /// Whether a point is solid, or that many tiles round it.
    pub fn blocked(&self, at: Vec2, margin: i32) -> bool {
        let (x, y) = ((at.x / TILE).floor() as i32, (at.y / TILE).floor() as i32);
        (-margin..=margin).any(|dy| (-margin..=margin).any(|dx| self.solid(x + dx, y + dy)))
    }

    fn solid(&self, x: i32, y: i32) -> bool {
        y < 0 || x < 0 || y as usize >= self.rows.len() || x as usize >= self.rows[0].len() || self.rows[y as usize][x as usize]
    }

    /// The same, with what stands there as walls too: each by its point and
    /// how far it reaches, wide and high.
    pub fn with(&self, marks: impl Iterator<Item = (Vec2, f32, f32)>) -> Ground {
        let mut rows = (*self.rows).clone();
        for (at, wide, high) in marks {
            let tile = |value: f32| (value / TILE).floor() as i32;
            for y in tile(at.y - high)..=tile(at.y + high) {
                for x in tile(at.x - wide)..=tile(at.x + wide) {
                    if y >= 0 && x >= 0 && (y as usize) < rows.len() && (x as usize) < rows[0].len() {
                        rows[y as usize][x as usize] = true;
                    }
                }
            }
        }
        Ground::new(rows)
    }

    /// The free spot nearest to a point.
    pub fn free(&self, at: Vec2) -> Vec2 {
        const FURTHEST: i32 = 120;
        if !self.blocked(at, 1) {
            return at;
        }
        let nearest = |spots: Vec<Vec2>| spots.into_iter().min_by(|a, b| a.distance_squared(at).total_cmp(&b.distance_squared(at)));
        for reach in (8..=FURTHEST).step_by(8) {
            let mut spots = Vec::new();
            for far in (-reach..=reach).step_by(8) {
                let (far, reach) = (far as f32, reach as f32);
                spots.extend([Vec2::new(far, -reach), Vec2::new(far, reach), Vec2::new(-reach, far), Vec2::new(reach, far)]);
            }
            let spots = spots.into_iter().map(|step| at + step).filter(|spot| !self.blocked(*spot, 1)).collect();
            if let Some(spot) = nearest(spots) {
                return spot;
            }
        }
        // (Far from any: the nearest ground there is.)
        let tiles = (0..self.rows.len()).flat_map(|y| (0..self.rows[0].len()).map(move |x| Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * TILE));
        nearest(tiles.filter(|spot| !self.blocked(*spot, 1)).collect()).unwrap_or(at)
    }

    /// Whether the straight line between two points is clear.
    pub fn clear(&self, a: Vec2, b: Vec2) -> bool {
        let steps = ((a - b).abs().max_element() / 4.0) as usize + 1;
        (0..=steps).all(|step| !self.blocked(a + (b - a) * (step as f32 / steps as f32), 0))
    }

    /// A way round the walls, as few points as can be; none for no way.
    pub fn route(&self, start: Vec2, goal: Vec2) -> Option<Vec<Vec2>> {
        if self.clear(start, goal) {
            return Some(vec![goal]);
        }
        let tile = |at: Vec2| ((at.x / TILE).floor() as i32, (at.y / TILE).floor() as i32);
        let middle = |(x, y): (i32, i32)| Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * TILE;
        let (wide, high) = (self.rows[0].len() as i32 + 2, self.rows.len() as i32 + 2);
        let index = |(x, y): (i32, i32)| ((y + 1).clamp(0, high - 1) * wide + (x + 1).clamp(0, wide - 1)) as usize;
        let (first, last) = (tile(start), tile(goal));
        let mut back: Vec<Option<(i32, i32)>> = vec![None; (wide * high) as usize];
        let mut todo = VecDeque::from([first]);
        back[index(first)] = Some(first);
        while let Some(at) = todo.pop_front() {
            if at == last {
                break;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let there = (at.0 + dx, at.1 + dy);
                if back[index(there)].is_none() && !self.blocked(middle(there), 1) {
                    back[index(there)] = Some(at);
                    todo.push_back(there);
                }
            }
        }
        back[index(last)]?;
        let (mut way, mut at) = (vec![goal], last);
        while at != first {
            at = back[index(at)]?;
            way.push(middle(at));
        }
        way.reverse();
        let (mut kept, mut here) = (Vec::new(), start);
        for (at, point) in way.iter().enumerate() {
            if *point == goal || !self.clear(here, way[at + 1]) {
                kept.push(*point);
                here = *point;
            }
        }
        Some(kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_spot_is_found_and_a_way_goes_round() {
        let ground = Ground::parse(&vec!["0".repeat(30); 30].join("/")).expect("a ground");
        let walls = ground.with([(Vec2::new(120.0, 120.0), 40.0, 8.0)].into_iter());
        assert!(walls.blocked(Vec2::new(120.0, 120.0), 0));
        let free = walls.free(Vec2::new(120.0, 120.0));
        assert!(!walls.blocked(free, 1), "{free}");
        let way = walls.route(Vec2::new(120.0, 80.0), Vec2::new(120.0, 180.0)).expect("a way");
        assert!(way.len() > 1, "{way:?}");
        assert_eq!(way.last(), Some(&Vec2::new(120.0, 180.0)));
    }

    #[test]
    fn no_way_through_a_closed_wall() {
        let ground = Ground::parse(&vec!["0".repeat(10); 10].join("/")).expect("a ground").with([(Vec2::new(40.0, 40.0), 80.0, 2.0)].into_iter());
        assert_eq!(ground.route(Vec2::new(20.0, 12.0), Vec2::new(20.0, 70.0)), None);
    }
}
