//! Shops, Monster Houses, extra hallways, imperfect rooms, secondary structures.

use super::*;

impl Generator<'_> {
    pub(super) fn generate_kecleon_shop(&mut self, grid: &mut Grid, nx: i32, ny: i32, chance: i32) {
        if self.has_monster_house || chance == 0 || self.rand(100) >= chance {
            return;
        }
        // Try the grid cells in a random order.
        let shuffled = |generator: &mut Self| {
            let mut list: Vec<i32> = (0..GRID).collect();
            for _ in 0..200 {
                let (a, b) = (generator.rand(GRID) as usize, generator.rand(GRID) as usize);
                list.swap(a, b);
            }
            list
        };
        let order_x = shuffled(self);
        let order_y = shuffled(self);
        for &x in order_x.iter().filter(|&&x| x < nx) {
            for &y in order_y.iter().filter(|&&y| y < ny) {
                let c = grid[gi(x, y)];
                let plain = !c.invalid && !c.was_merged && !c.merged && c.is_room && c.connected && !c.structured && !c.is_maze && !c.flag_secondary;
                if !plain || (c.start_x - c.end_x).abs() < 5 || (c.start_y - c.end_y).abs() < 4 {
                    continue;
                }
                self.has_kecleon_shop = true;
                let (min_x, min_y, max_x) = (c.start_x, c.start_y, c.end_x);
                let max_y = if c.end_y - c.start_y < 3 { c.end_y + 1 } else { c.end_y };
                self.shop_min = (9999, 9999);
                self.shop_max = (-9999, -9999);
                // The shop itself leaves a one-tile border inside the room.
                for cx in min_x + 1..max_x - 1 {
                    for cy in min_y + 1..max_y - 1 {
                        let tile = self.tile(cx, cy);
                        tile.terrain |= SHOP;
                        tile.spawn &= !(SPAWN_STAIRS | SPAWN_ENEMY);
                        self.shop_min = (self.shop_min.0.min(cx), self.shop_min.1.min(cy));
                        self.shop_max = (self.shop_max.0.max(cx), self.shop_max.1.max(cy));
                    }
                }
                for cx in c.start_x..c.end_x {
                    for cy in c.start_y..c.end_y {
                        self.tile(cx, cy).spawn |= 0x10;
                    }
                }
                return;
            }
        }
    }

    pub(super) fn generate_monster_house(&mut self, grid: &mut Grid, nx: i32, ny: i32, chance: i32) {
        if chance == 0 || self.rand(100) >= chance || self.has_kecleon_shop {
            return;
        }
        let suitable = |c: &Cell| !c.invalid && !c.was_merged && !c.unk_c && c.is_room && c.connected && !c.structured && !c.unk_f && !c.is_maze;
        let cells: Vec<(i32, i32)> = (0..nx).flat_map(|x| (0..ny).map(move |y| (x, y))).collect();
        let valid = cells.iter().filter(|&&(x, y)| suitable(&grid[gi(x, y)])).count() as i32;
        if valid == 0 {
            return;
        }
        // One marker shuffled among the suitable rooms.
        let mut values: Vec<bool> = (0..0x100).map(|i| i == 0).collect();
        for _ in 0..0x40 {
            let (a, b) = (self.rand(valid) as usize, self.rand(valid) as usize);
            values.swap(a, b);
        }
        let mut counter = 0;
        for (x, y) in cells {
            if !suitable(&grid[gi(x, y)]) {
                continue;
            }
            if values[counter] {
                self.has_monster_house = true;
                grid[gi(x, y)].is_monster_house = true;
                let c = grid[gi(x, y)];
                for cx in c.start_x..c.end_x {
                    for cy in c.start_y..c.end_y {
                        self.tile(cx, cy).terrain |= MONSTER_HOUSE;
                        self.monster_house_room = self.at(cx, cy).room as i32;
                    }
                }
                return;
            }
            counter += 1;
        }
    }

    /// Extra hallways dug by random walks out of rooms.
    pub(super) fn generate_extra_hallways(&mut self, grid: &Grid, nx: i32, ny: i32, count: i32) {
        // Cardinal directions among the eight: down, right, up, left.
        let step = |direction: i32| DIRECTIONS[direction as usize];
        for _ in 0..count {
            let (x, y) = (self.rand(nx), self.rand(ny));
            let c = grid[gi(x, y)];
            if !(c.is_room && c.connected && !c.invalid && !c.is_maze) {
                continue;
            }
            let mut cur_x = self.rand_between(c.start_x, c.end_x);
            let mut cur_y = self.rand_between(c.start_y, c.end_y);
            let mut direction = self.rand(4) * 2;
            for _ in 0..3 {
                if direction == 0 && y >= ny - 1 {
                    direction = 2;
                }
                if direction == 2 && x >= nx - 1 {
                    direction = 4;
                }
                if direction == 4 && y <= 0 {
                    direction = 6;
                }
                if direction == 6 && x <= 0 {
                    direction = 0;
                }
            }
            let room = self.at(cur_x, cur_y).room;
            // Walk out of the room, then on until an obstacle.
            while self.at(cur_x, cur_y).room == room {
                cur_x += step(direction).0;
                cur_y += step(direction).1;
            }
            while self.open_at(cur_x, cur_y) {
                cur_x += step(direction).0;
                cur_y += step(direction).1;
            }
            // (The original compares the whole flag word here.)
            if self.at(cur_x, cur_y).terrain == SECONDARY {
                continue;
            }
            // At least two tiles away from the map border.
            if cur_x - 2 < 0 || cur_x + 2 >= W || cur_y - 2 < 0 || cur_y + 2 >= H {
                continue;
            }
            let side_open = |generator: &Self, x: i32, y: i32, direction: i32, turn: i32| {
                let (dx, dy) = step((direction + turn) & 6);
                generator.open_at(x + dx, y + dy)
            };
            if side_open(self, cur_x, cur_y, direction, 2) || side_open(self, cur_x, cur_y, direction, -2) {
                continue;
            }
            let mut steps = self.rand(3) + 3;
            loop {
                if cur_x <= 1 || cur_y <= 1 || cur_x >= 55 || cur_y >= 31 {
                    break;
                }
                let here = self.at(cur_x, cur_y);
                if here.is_open() || here.terrain & IMPASSABLE != 0 {
                    break;
                }
                // Never open a 2x2 block: each corner needs an obstacle.
                let corner_blocked =
                    |generator: &Self, sx: i32, sy: i32| [(sx, 0), (sx, sy), (0, sy)].into_iter().any(|(dx, dy)| !generator.open_at(cur_x + dx, cur_y + dy));
                let carve = corner_blocked(self, 1, 1) && corner_blocked(self, 1, -1) && corner_blocked(self, -1, 1) && corner_blocked(self, -1, -1);
                if carve {
                    self.tile(cur_x, cur_y).set_kind(OPEN);
                }
                if side_open(self, cur_x, cur_y, direction, 2) || side_open(self, cur_x, cur_y, direction, -2) {
                    break;
                }
                steps -= 1;
                if steps == 0 {
                    steps = self.rand(3) + 3;
                    direction = (direction + if self.rand(100) < 50 { 2 } else { -2 }) & 6;
                    if (cur_x >= 32 && self.floor_size == 1 && direction == 2) || (cur_x >= 48 && self.floor_size == 2 && direction == 2) {
                        break;
                    }
                }
                cur_x += step(direction).0;
                cur_y += step(direction).1;
            }
        }
    }

    /// Whether a hallway tile lies in the 3x3 block around a position.
    pub(super) fn hallway_near(&self, x: i32, y: i32) -> bool {
        (-1..=1).any(|dx| {
            (-1..=1).any(|dy| {
                let tile = self.at(x + dx, y + dy);
                tile.is_open() && tile.room == NO_ROOM
            })
        })
    }

    /// Grows walls inwards from the corners of flagged rooms.
    pub(super) fn generate_room_imperfections(&mut self, grid: &Grid, nx: i32, ny: i32) {
        for x in 0..nx {
            for y in 0..ny {
                let c = grid[gi(x, y)];
                let eligible = !c.invalid && !c.was_merged && !c.merged && c.is_room && c.connected && !c.structured && !c.is_maze && c.flag_imperfect;
                if !eligible || self.rand(100) < IMPERFECT_CHANCE {
                    continue;
                }
                let length = (((c.end_x - c.start_x) + (c.end_y - c.start_y)) / 4).max(1);
                for _ in 0..length {
                    for i in 0..2 {
                        let corner = self.rand(4);
                        // Start corner, and the direction walls grow in
                        // (counter-clockwise first, then clockwise).
                        let (mut pt_x, mut pt_y, move_x, move_y) = match (corner, i) {
                            (0, 0) => (c.start_x, c.start_y, 0, 1),
                            (0, _) => (c.start_x, c.start_y, 1, 0),
                            (1, 0) => (c.end_x - 1, c.start_y, -1, 0),
                            (1, _) => (c.end_x - 1, c.start_y, 0, 1),
                            (2, 0) => (c.end_x - 1, c.end_y - 1, 0, -1),
                            (2, _) => (c.end_x - 1, c.end_y - 1, -1, 0),
                            (_, 0) => (c.start_x, c.end_y - 1, 1, 0),
                            _ => (c.start_x, c.end_y - 1, 0, -1),
                        };
                        for _ in 0..10 {
                            let inside = (c.start_x..c.end_x).contains(&pt_x) && (c.start_y..c.end_y).contains(&pt_y);
                            if !inside {
                                break;
                            }
                            if !self.open_at(pt_x, pt_y) {
                                pt_x += move_x;
                                pt_y += move_y;
                                continue;
                            }
                            // No hallway within two tiles.
                            let clear = DIRECTIONS.iter().all(|&(dx, dy)| !self.hallway_near(pt_x + dx, pt_y + dy));
                            if clear {
                                // The cardinal neighbours must look like the
                                // corner being grown from.
                                let base = corner as usize * 8;
                                let matches = (0..8).step_by(2).all(|d| {
                                    let (dx, dy) = DIRECTIONS[d];
                                    CORNER_CHECKS[base + d] == self.open_at(pt_x + dx, pt_y + dy) as u8
                                });
                                if matches {
                                    self.tile(pt_x, pt_y).set_kind(0);
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Whether a tile or one of its cardinal neighbours is a hallway.
    pub(super) fn by_hallway(&self, x: i32, y: i32) -> bool {
        [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)].into_iter().any(|(dx, dy)| {
            let (tx, ty) = (x + dx, y + dy);
            if !(0..W).contains(&tx) || !(0..H).contains(&ty) {
                return false;
            }
            let tile = self.at(tx, ty);
            tile.is_open() && tile.room == NO_ROOM
        })
    }

    pub(super) fn flag_room(&mut self, c: &Cell, flag: u16) {
        for x in c.start_x..c.end_x {
            for y in c.start_y..c.end_y {
                self.tile(x, y).spawn |= flag;
            }
        }
    }

    /// Secondary structures inside flagged rooms: mazes, crosses, pools,
    /// islands and dividers of secondary terrain.
    pub(super) fn generate_secondary_structures(&mut self, grid: &mut Grid, nx: i32, ny: i32) {
        for y in 0..ny {
            for x in 0..nx {
                let c = grid[gi(x, y)];
                let eligible = !c.invalid && !c.is_monster_house && !c.merged && c.is_room && c.flag_secondary && !c.flag_imperfect;
                if !eligible {
                    continue;
                }
                let (width, height) = (c.end_x - c.start_x, c.end_y - c.start_y);
                let (mid_x, mid_y) = ((c.end_x + c.start_x) / 2, (c.end_y + c.start_y) / 2);
                let odd = width & 1 == 1 && height & 1 == 1;
                let budget = self.secondary_structures > 0;
                match self.rand(6) {
                    1 if budget => {
                        self.secondary_structures -= 1;
                        if odd {
                            self.flag_room(&c, 0x20);
                            let mut cell = c;
                            self.create_maze(&mut cell, true);
                            grid[gi(x, y)] = cell;
                        } else if width >= 5 && height >= 5 {
                            for (dx, dy) in [(0, 0), (0, -1), (-1, 0), (1, 0), (0, 1)] {
                                self.tile(mid_x + dx, mid_y + dy).set_kind(SECONDARY);
                            }
                        } else {
                            self.tile(mid_x, mid_y).set_kind(SECONDARY);
                        }
                        grid[gi(x, y)].structured = true;
                    }
                    2 if budget && odd => {
                        // A checkerboard of secondary terrain.
                        self.secondary_structures -= 1;
                        self.flag_room(&c, 0x20);
                        for _ in 0..0x40 {
                            let (rx, ry) = (self.rand(width), self.rand(height));
                            if (rx + ry) & 1 == 1 {
                                self.tile(c.start_x + rx, c.start_y + ry).set_kind(SECONDARY);
                            }
                        }
                        grid[gi(x, y)].structured = true;
                    }
                    3 if width >= 5 && height >= 5 => {
                        let mut x1 = self.rand_swap(c.start_x + 2, c.end_x - 3);
                        let mut y1 = self.rand_swap(c.start_y + 2, c.end_y - 3);
                        let mut x2 = self.rand_swap(c.start_x + 2, c.end_x - 3);
                        let mut y2 = self.rand_swap(c.start_y + 2, c.end_y - 3);
                        if self.secondary_structures > 0 {
                            // A pool.
                            self.secondary_structures -= 1;
                            self.flag_room(&c, 0x20);
                            if x1 > x2 {
                                std::mem::swap(&mut x1, &mut x2);
                            }
                            if y1 > y2 {
                                std::mem::swap(&mut y1, &mut y2);
                            }
                            for cx in x1..=x2 {
                                for cy in y1..=y2 {
                                    self.tile(cx, cy).set_kind(SECONDARY);
                                }
                            }
                            grid[gi(x, y)].structured = true;
                        }
                    }
                    4 if width >= 6 && height >= 6 && budget => {
                        // An island with items and a warp tile, in a moat.
                        self.secondary_structures -= 1;
                        self.flag_room(&c, 0x20);
                        const MOAT: [(i32, i32); 14] =
                            [(-2, -2), (-1, -2), (0, -2), (1, -2), (-2, -1), (-2, 0), (-2, 1), (-2, 1), (-1, 1), (0, 1), (1, -2), (1, -1), (1, 0), (1, 1)];
                        for (dx, dy) in MOAT {
                            let tile = self.tile(mid_x + dx, mid_y + dy);
                            tile.terrain = tile.terrain & !3 | SECONDARY | CORNER_CUTTABLE;
                        }
                        self.tile(mid_x - 1, mid_y - 1).spawn |= 0x54;
                        self.tile(mid_x, mid_y - 1).spawn |= 0x12;
                        self.tile(mid_x - 1, mid_y).spawn |= 0x12;
                        self.tile(mid_x, mid_y).spawn |= 0x12;
                        grid[gi(x, y)].structured = true;
                    }
                    5 if budget => {
                        // A line of secondary terrain splitting the room.
                        self.secondary_structures -= 1;
                        self.flag_room(&c, 0x20);
                        let horizontal = self.rand(2) == 0;
                        let line: Vec<(i32, i32)> =
                            if horizontal { (c.start_x..c.end_x).map(|i| (i, mid_y)).collect() } else { (c.start_y..c.end_y).map(|i| (mid_x, i)).collect() };
                        if line.iter().all(|&(lx, ly)| !self.by_hallway(lx, ly)) {
                            for (lx, ly) in line {
                                self.tile(lx, ly).set_kind(SECONDARY);
                            }
                            self.flag_room(&c, 0x80);
                            grid[gi(x, y)].structured = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // --- layouts ---
}
