//! The floor map drawn over the screen: what the team has seen so far.
//!
//! What gets revealed is what the leader can see (`ai::can_see`): from a
//! room, the room and the ring of tiles around it; from a hallway, a small
//! square around the leader.

use super::ai::can_see;
use super::floor::{FLOOR_HEIGHT, FLOOR_WIDTH, Floor, Pos, Tile};

/// Pixels per tile on the map.
pub const CELL: usize = 4;
pub const MAP_WIDTH: usize = FLOOR_WIDTH as usize * CELL;
pub const MAP_HEIGHT: usize = FLOOR_HEIGHT as usize * CELL;

// Plain white lines around what is open, nothing inside.
const OUTLINE: [u8; 4] = [248, 248, 248, 255];
const GROUND: [u8; 4] = [0, 0, 0, 0];
const STAIRS: [u8; 4] = [90, 200, 255, 255];

/// The tiles seen so far on the current floor.
#[derive(Clone, Debug)]
pub struct Explored(Vec<bool>);

impl Default for Explored {
    fn default() -> Self {
        Self(vec![false; (FLOOR_WIDTH * FLOOR_HEIGHT) as usize])
    }
}

impl Explored {
    /// Everything is known (Map Surveyor).
    pub(crate) fn reveal_all(&mut self) {
        self.0.iter_mut().for_each(|seen| *seen = true);
    }

    /// One tile is known, with those around it (Stair Sensor).
    pub(crate) fn reveal_around(&mut self, p: Pos) {
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let at = Pos::new(p.x + dx, p.y + dy);
            if Floor::in_bounds(at) {
                self.0[(at.y * FLOOR_WIDTH + at.x) as usize] = true;
            }
        }
    }

    pub fn seen(&self, p: Pos) -> bool {
        Floor::in_bounds(p) && self.0[(p.y * FLOOR_WIDTH + p.x) as usize]
    }

    /// Marks what is in sight from `from`. Returns whether anything new
    /// was seen.
    pub fn reveal(&mut self, floor: &Floor, from: Pos) -> bool {
        let mut changed = false;
        for p in Floor::positions() {
            let index = (p.y * FLOOR_WIDTH + p.x) as usize;
            if !self.0[index] && can_see(floor, from, p) {
                self.0[index] = true;
                changed = true;
            }
        }
        changed
    }
}

/// The map as an RGBA picture of `MAP_WIDTH` x `MAP_HEIGHT` pixels: the
/// open tiles seen so far, outlined where they meet anything else, the
/// stairs once seen, and a dot for each of `dots` (the caller chooses which
/// monsters and items show).
pub fn draw(floor: &Floor, explored: &Explored, dots: &[(Pos, [u8; 4])]) -> Vec<u8> {
    let mut data = vec![0u8; MAP_WIDTH * MAP_HEIGHT * 4];
    let mut put = |p: Pos, x: usize, y: usize, color: [u8; 4]| {
        let at = ((p.y as usize * CELL + y) * MAP_WIDTH + p.x as usize * CELL + x) * 4;
        data[at..at + 4].copy_from_slice(&color);
    };
    let open = |p: Pos| floor.tile(p).is_walkable();
    for p in Floor::positions() {
        if !explored.seen(p) || !open(p) {
            continue;
        }
        let stairs = matches!(floor.tile(p), Tile::Stairs { .. });
        // Sides facing a wall or water: left, right, up, down.
        let closed = [(-1, 0), (1, 0), (0, -1), (0, 1)].map(|(dx, dy)| !open(Pos::new(p.x + dx, p.y + dy)));
        for y in 0..CELL {
            for x in 0..CELL {
                let edge = (x == 0 && closed[0]) || (x == CELL - 1 && closed[1]) || (y == 0 && closed[2]) || (y == CELL - 1 && closed[3]);
                let rim = x == 0 || y == 0 || x == CELL - 1 || y == CELL - 1;
                let color = match (edge, stairs && rim) {
                    (true, _) => OUTLINE,
                    (false, true) => STAIRS,
                    (false, false) => GROUND,
                };
                put(p, x, y, color);
            }
        }
    }
    for &(p, color) in dots {
        if !Floor::in_bounds(p) {
            continue;
        }
        // A rounded blob: the tile without its corners.
        for y in 0..CELL {
            for x in 0..CELL {
                let corner = (x == 0 || x == CELL - 1) && (y == 0 || y == CELL - 1);
                if !corner {
                    put(p, x, y, color);
                }
            }
        }
    }
    data
}

/// One dot of the map, on its own: 4x4 RGBA.
pub fn draw_dot(color: [u8; 4]) -> Vec<u8> {
    let mut data = vec![0u8; CELL * CELL * 4];
    for y in 0..CELL {
        for x in 0..CELL {
            let corner = (x == 0 || x == CELL - 1) && (y == 0 || y == CELL - 1);
            if !corner {
                data[(y * CELL + x) * 4..][..4].copy_from_slice(&color);
            }
        }
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A room with a hallway leaving it to the right.
    fn floor() -> Floor {
        let mut floor = Floor::new_walled();
        for y in 5..10 {
            for x in 5..12 {
                floor.set(Pos::new(x, y), Tile::Floor { room: Some(0) });
            }
        }
        for x in 12..20 {
            floor.set(Pos::new(x, 7), Tile::Floor { room: None });
        }
        floor
    }

    #[test]
    fn rooms_show_at_once_and_hallways_step_by_step() {
        let floor = floor();
        let mut explored = Explored::default();
        assert!(explored.reveal(&floor, Pos::new(6, 6)));
        // The whole room and its doorstep, not the hallway beyond.
        assert!(explored.seen(Pos::new(11, 9)) && explored.seen(Pos::new(12, 7)));
        assert!(!explored.seen(Pos::new(14, 7)));
        // Nothing new from the same spot.
        assert!(!explored.reveal(&floor, Pos::new(6, 6)));
        // In the hallway, two tiles ahead.
        assert!(explored.reveal(&floor, Pos::new(14, 7)));
        assert!(explored.seen(Pos::new(16, 7)) && !explored.seen(Pos::new(17, 7)));
    }

    #[test]
    fn the_map_outlines_what_was_seen() {
        let floor = floor();
        let mut explored = Explored::default();
        let pixel = |data: &[u8], x: usize, y: usize| {
            let at = (y * MAP_WIDTH + x) * 4;
            [data[at], data[at + 1], data[at + 2], data[at + 3]]
        };
        let blank = draw(&floor, &explored, &[]);
        assert!(blank.iter().all(|&byte| byte == 0));
        explored.reveal(&floor, Pos::new(6, 6));
        let red = [255, 0, 0, 255];
        let data = draw(&floor, &explored, &[(Pos::new(8, 8), red)]);
        assert_eq!(data.len(), MAP_WIDTH * MAP_HEIGHT * 4);
        // The room's corner tile is outlined on its wall sides only.
        assert_eq!(pixel(&data, 5 * CELL, 5 * CELL), OUTLINE);
        assert_eq!(pixel(&data, 5 * CELL + 2, 5 * CELL + 2), GROUND);
        assert_eq!(pixel(&data, 5 * CELL + 3, 5 * CELL + 3), GROUND);
        // The dot is rounded: no corner.
        assert_eq!(pixel(&data, 8 * CELL, 8 * CELL), [0; 4]);
        // A dot fills its tile; unseen and wall tiles stay empty.
        assert_eq!(pixel(&data, 8 * CELL + 1, 8 * CELL + 2), red);
        assert_eq!(pixel(&data, 15 * CELL + 1, 7 * CELL + 1), [0; 4]);
        assert_eq!(pixel(&data, 4 * CELL + 1, 5 * CELL + 1), [0; 4]);
    }
}
