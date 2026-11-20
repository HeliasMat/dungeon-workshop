//! The floor layouts.

use super::*;

impl Generator<'_> {
    pub(super) fn generate_normal_floor(&mut self, nx: i32, ny: i32) {
        let (list_x, list_y) = Self::grid_positions(nx, ny);
        let mut grid = self.init_grid(nx, ny);
        self.place_rooms(&mut grid, nx, ny, self.prop.room_density);
        self.create_rooms(&mut grid, nx, ny, &list_x, &list_y, self.prop.flags);
        let start = (self.rand(nx), self.rand(ny));
        self.create_connections(&mut grid, nx, ny, start);
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, false);
        self.add_hallways(&mut grid, nx, ny, &list_x, &list_y);
        self.mazify(self.prop.maze_room_chance);
        self.finish_layout(&mut grid, nx, ny, true);
    }

    /// The special rooms and decorations shared by every layout.
    pub(super) fn finish_layout(&mut self, grid: &mut Grid, nx: i32, ny: i32, structures: bool) {
        self.generate_kecleon_shop(grid, nx, ny, self.prop.kecleon_shop_chance);
        self.generate_monster_house(grid, nx, ny, self.prop.monster_house_chance);
        self.generate_extra_hallways(grid, nx, ny, self.prop.extra_hallways);
        self.generate_room_imperfections(grid, nx, ny);
        if structures {
            self.generate_secondary_structures(grid, nx, ny);
        }
    }

    /// One large room that is a Monster House (only the room, disarmed).
    pub(super) fn generate_one_room_monster_house(&mut self) {
        let mut grid = self.init_grid(1, 1);
        let cell = &mut grid[gi(0, 0)];
        (cell.start_x, cell.start_y, cell.end_x, cell.end_y) = (2, 2, 54, 30);
        (cell.is_room, cell.connected, cell.invalid) = (true, true, false);
        self.carve_room(2, 2, 54, 30, 0);
        if !self.prop.disarmed_lone_house {
            self.generate_monster_house(&mut grid, 1, 1, 999);
            self.create_monster_house = true;
        }
    }

    /// Rooms (with sizes in the cell minus a margin) or single-tile anchors
    /// for the ring and crossroads layouts.
    pub(super) fn create_fixed_grid_rooms(&mut self, grid: &mut Grid, nx: i32, ny: i32, list_x: &[i32], list_y: &[i32]) {
        let mut room_number = 0u8;
        for y in 0..ny {
            for x in 0..nx {
                if grid[gi(x, y)].invalid {
                    continue;
                }
                let (cx, nxt_x) = (list_x[x as usize], list_x[x as usize + 1]);
                let (cy, nxt_y) = (list_y[y as usize], list_y[y as usize + 1]);
                if grid[gi(x, y)].is_room {
                    let (range_x, range_y) = (nxt_x - cx - 3, nxt_y - cy - 3);
                    let size_x = self.rand_swap(5, range_x);
                    let size_y = self.rand_swap(4, range_y);
                    let start_x = self.rand_force(range_x - size_x) + cx + 2;
                    let start_y = self.rand_force(range_y - size_y) + cy + 2;
                    let cell = &mut grid[gi(x, y)];
                    (cell.start_x, cell.start_y) = (start_x, start_y);
                    (cell.end_x, cell.end_y) = (start_x + size_x, start_y + size_y);
                    self.carve_room(start_x, start_y, start_x + size_x, start_y + size_y, room_number);
                    room_number += 1;
                } else {
                    let start_x = self.rand_swap(cx + 1, nxt_x - 2);
                    let start_y = self.rand_swap(cy + 1, nxt_y - 2);
                    let cell = &mut grid[gi(x, y)];
                    (cell.start_x, cell.start_y) = (start_x, start_y);
                    (cell.end_x, cell.end_y) = (start_x + 1, start_y + 1);
                    let tile = self.tile(start_x, start_y);
                    tile.set_kind(OPEN);
                    tile.room = NO_ROOM;
                }
            }
        }
    }

    /// A 4x2 block of rooms inside a ring of hallways.
    pub(super) fn generate_ring(&mut self) {
        let (nx, ny) = (6, 4);
        let (list_x, list_y) = (vec![0, 5, 0x10, 0x1C, 0x27, 0x33, 0x38], vec![2, 7, 0x10, 0x19, 0x1E]);
        let mut grid = self.init_grid(nx, ny);
        for x in 0..nx {
            for y in 0..ny {
                grid[gi(x, y)].is_room = (1..5).contains(&x) && (1..3).contains(&y);
            }
        }
        self.create_fixed_grid_rooms(&mut grid, nx, ny, &list_x, &list_y);
        // The ring itself.
        for x in 0..nx - 1 {
            for y in [0, ny - 1] {
                grid[gi(x, y)].right = true;
                grid[gi(x + 1, y)].left = true;
            }
        }
        for y in 0..ny - 1 {
            for x in [0, nx - 1] {
                grid[gi(x, y)].bottom = true;
                grid[gi(x, y + 1)].top = true;
            }
        }
        let start = (self.rand(nx), self.rand(ny));
        self.create_connections(&mut grid, nx, ny, start);
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, false);
        self.add_hallways(&mut grid, nx, ny, &list_x, &list_y);
        self.finish_layout(&mut grid, nx, ny, false);
    }

    /// A mesh of hallways with rooms sticking out of its sides.
    pub(super) fn generate_crossroads(&mut self) {
        let (nx, ny) = (5, 4);
        let (list_x, list_y) = (vec![0, 0xB, 0x16, 0x21, 0x2C, 0x38], vec![1, 9, 0x10, 0x17, 0x1F]);
        let mut grid = self.init_grid(nx, ny);
        for x in 0..nx {
            for y in 0..ny {
                let cell = &mut grid[gi(x, y)];
                cell.is_room = !((1..4).contains(&x) && (1..3).contains(&y));
                cell.invalid = (x == 0 || x == nx - 1) && (y == 0 || y == ny - 1);
            }
        }
        self.create_fixed_grid_rooms(&mut grid, nx, ny, &list_x, &list_y);
        for x in 1..4 {
            for y in 0..3 {
                grid[gi(x, y)].bottom = true;
                grid[gi(x, y + 1)].top = true;
            }
        }
        for x in 0..4 {
            for y in 1..3 {
                grid[gi(x, y)].right = true;
                grid[gi(x + 1, y)].left = true;
            }
        }
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, true);
        self.add_hallways(&mut grid, nx, ny, &list_x, &list_y);
        self.finish_layout(&mut grid, nx, ny, false);
    }

    /// Two rooms side by side, one of them a Monster House.
    pub(super) fn generate_two_rooms_monster_house(&mut self) {
        let (nx, ny) = (2, 1);
        let (list_x, list_y) = (vec![2, 0x1C, 0x36], vec![2, 0x1E]);
        let mut grid = self.init_grid(nx, ny);
        for x in 0..2 {
            let range_x = list_x[x as usize + 1] - list_x[x as usize] - 3;
            let range_y = list_y[1] - list_y[0] - 3;
            let size_x = self.rand_swap(10, range_x);
            let size_y = self.rand_swap(16, range_y);
            let start_x = self.rand_force(range_x - size_x) + list_x[x as usize] + 1;
            let start_y = self.rand_force(range_y - size_y) + list_y[0] + 1;
            let cell = &mut grid[gi(x, 0)];
            (cell.start_x, cell.start_y) = (start_x, start_y);
            (cell.end_x, cell.end_y) = (start_x + size_x, start_y + size_y);
            self.carve_room(start_x, start_y, start_x + size_x, start_y + size_y, x as u8);
        }
        grid[gi(0, 0)].right = true;
        grid[gi(1, 0)].left = true;
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, false);
        self.generate_monster_house(&mut grid, nx, ny, 999);
    }

    /// Five cells in a row.
    pub(super) fn generate_room_line(&mut self) {
        let (nx, ny) = (5, 1);
        let (list_x, list_y) = (vec![0, 0xB, 0x16, 0x21, 0x2C, 0x38], vec![4, 0xF]);
        let mut grid = self.init_grid(nx, ny);
        self.place_rooms(&mut grid, nx, ny, self.prop.room_density);
        self.create_rooms(&mut grid, nx, ny, &list_x, &list_y, self.prop.flags);
        let start = (self.rand(nx), self.rand(ny));
        self.create_connections(&mut grid, nx, ny, start);
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, true);
        self.add_hallways(&mut grid, nx, ny, &list_x, &list_y);
        self.finish_layout(&mut grid, nx, ny, false);
    }

    /// Five rooms in a plus sign.
    pub(super) fn generate_cross(&mut self) {
        let (nx, ny) = (3, 3);
        let (list_x, list_y) = (vec![0xB, 0x16, 0x21, 0x2C], vec![2, 0xB, 0x14, 0x1E]);
        let mut grid = self.init_grid(nx, ny);
        for x in 0..nx {
            for y in 0..ny {
                grid[gi(x, y)].invalid = x != 1 && y != 1;
            }
        }
        self.create_rooms(&mut grid, nx, ny, &list_x, &list_y, self.prop.flags);
        for y in 0..2 {
            grid[gi(1, y)].bottom = true;
            grid[gi(1, y + 1)].top = true;
        }
        for x in 0..2 {
            grid[gi(x, 1)].right = true;
            grid[gi(x + 1, 1)].left = true;
        }
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, true);
        self.add_hallways(&mut grid, nx, ny, &list_x, &list_y);
        self.finish_layout(&mut grid, nx, ny, false);
    }

    /// Merges two rooms of a column into the first one.
    pub(super) fn merge_rooms_vertically(&mut self, grid: &mut Grid, x: i32, y1: i32, dy: i32) {
        let y2 = y1 + dy;
        let (a, b) = (grid[gi(x, y1)], grid[gi(x, y2)]);
        let (src_x, dst_x) = (a.start_x.min(b.start_x), a.end_x.max(b.end_x));
        let (src_y, dst_y) = (a.start_y, b.end_y);
        let room = self.at(a.start_x, a.start_y).room;
        self.carve_room(src_x, src_y, dst_x, dst_y, room);
        let first = &mut grid[gi(x, y1)];
        (first.start_x, first.start_y, first.end_x, first.end_y) = (src_x, src_y, dst_x, dst_y);
        first.merged = true;
        let second = &mut grid[gi(x, y2)];
        second.merged = true;
        second.connected = false;
        second.was_merged = true;
    }

    /// A 3x3 grid whose middle column is one big room.
    pub(super) fn generate_beetle(&mut self) {
        let (nx, ny) = (3, 3);
        let (list_x, list_y) = (vec![0x5, 0xF, 0x23, 0x32], vec![2, 0xB, 0x14, 0x1E]);
        let mut grid = self.init_grid(nx, ny);
        self.create_rooms(&mut grid, nx, ny, &list_x, &list_y, self.prop.flags);
        for y in 0..3 {
            for x in 0..2 {
                grid[gi(x, y)].right = true;
                grid[gi(x + 1, y)].left = true;
            }
        }
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, true);
        self.merge_rooms_vertically(&mut grid, 1, 0, 1);
        self.merge_rooms_vertically(&mut grid, 1, 0, 2);
        self.add_hallways(&mut grid, nx, ny, &list_x, &list_y);
        self.finish_layout(&mut grid, nx, ny, false);
    }

    /// Rooms on the grid's boundary only. The connections are set up with
    /// the original's off-by-one conditions.
    pub(super) fn generate_outer_rooms(&mut self, nx: i32, ny: i32) {
        let (list_x, list_y) = Self::grid_positions(nx, ny);
        let mut grid = self.init_grid(nx, ny);
        for x in 1..nx - 1 {
            for y in 1..ny - 1 {
                grid[gi(x, y)].invalid = true;
            }
        }
        self.create_rooms(&mut grid, nx, ny, &list_x, &list_y, self.prop.flags);
        for x in 0..nx - 1 {
            if x > 0 {
                grid[gi(x, 0)].right = true;
                grid[gi(x, ny - 1)].right = true;
            }
            if x < nx - 2 {
                grid[gi(x + 1, 0)].left = true;
                grid[gi(x + 1, ny - 1)].left = true;
            }
        }
        for y in 0..ny - 1 {
            if y > 0 {
                grid[gi(0, y)].top = true;
                grid[gi(nx - 1, y)].top = true;
            }
            if y < ny - 2 {
                grid[gi(0, y)].bottom = true;
                grid[gi(nx - 1, y)].bottom = true;
            }
        }
        self.create_hallways(&mut grid, nx, ny, &list_x, &list_y, false);
        self.add_hallways(&mut grid, nx, ny, &list_x, &list_y);
        self.mazify(self.prop.maze_room_chance);
        self.finish_layout(&mut grid, nx, ny, true);
    }

    // --- finalisation ---
}
