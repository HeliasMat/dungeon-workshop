//! The grid, and the rooms on it.

use super::*;

impl Generator<'_> {
    pub(super) fn grid_positions(nx: i32, ny: i32) -> (Vec<i32>, Vec<i32>) {
        let list_x = (0..=nx).map(|x| x * (W / nx)).collect();
        let list_y = (0..=ny).map(|y| y * (H / ny)).collect();
        (list_x, list_y)
    }

    pub(super) fn init_grid(&self, nx: i32, ny: i32) -> Grid {
        let mut grid = vec![Cell::default(); (GRID * GRID) as usize];
        for x in 0..nx {
            for y in 0..ny {
                let cell = &mut grid[gi(x, y)];
                cell.invalid = (self.floor_size == 1 && x >= nx / 2) || (self.floor_size == 2 && x >= nx * 3 / 4);
                cell.is_room = true;
            }
        }
        grid
    }

    pub(super) fn place_rooms(&mut self, grid: &mut Grid, nx: i32, ny: i32, room_density: i32) {
        let bonus = self.rand(3);
        let wanted = if room_density < 0 { -room_density } else { room_density + bonus };
        let mut rooms_ok: Vec<bool> = (0..256).map(|i| i < wanted).collect();
        let cells = nx * ny;
        for _ in 0..0x40 {
            let (a, b) = (self.rand(cells) as usize, self.rand(cells) as usize);
            rooms_ok.swap(a, b);
        }
        self.room_count = 0;
        let odd_x = nx % 2 == 1;
        let mut counter = 0;
        for x in 0..nx {
            for y in 0..ny {
                if grid[gi(x, y)].invalid {
                    continue;
                }
                if self.room_count >= 0x20 {
                    grid[gi(x, y)].is_room = false;
                }
                if rooms_ok[counter] {
                    grid[gi(x, y)].is_room = true;
                    self.room_count += 1;
                    // Never a room in the middle of the second row.
                    if odd_x && y == 1 && x == (nx - 1) / 2 {
                        grid[gi(x, y)].is_room = false;
                    }
                } else {
                    grid[gi(x, y)].is_room = false;
                }
                counter += 1;
            }
        }
        if self.room_count >= 2 {
            return;
        }
        // At least two rooms are needed: try cells until one becomes a room.
        for _ in 0..200 {
            for x in 0..nx {
                for y in 0..ny {
                    if !grid[gi(x, y)].invalid && self.rand(100) < 60 {
                        grid[gi(x, y)].is_room = true;
                        return;
                    }
                }
            }
        }
    }

    pub(super) fn create_rooms(&mut self, grid: &mut Grid, nx: i32, ny: i32, list_x: &[i32], list_y: &[i32], flags: u8) {
        let imperfect = flags & 0x4 != 0;
        let mut room_number = 0u8;
        for y in 0..ny {
            let (cur_y, next_y) = (list_y[y as usize], list_y[y as usize + 1]);
            for x in 0..nx {
                let (cur_x, next_x) = (list_x[x as usize], list_x[x as usize + 1]);
                let (range_x, range_y) = (next_x - cur_x - 4, next_y - cur_y - 3);
                if grid[gi(x, y)].invalid {
                    continue;
                }
                if !grid[gi(x, y)].is_room {
                    // A hallway anchor: one open tile inside the cell.
                    let unk_x1 = if x == 0 { 1 } else { 2 };
                    let unk_x2 = if x == nx - 1 { 2 } else { 4 };
                    let unk_y1 = if y == 0 { 1 } else { 2 };
                    let unk_y2 = if y == ny - 1 { 2 } else { 4 };
                    let pt_x = self.rand_swap(cur_x + 2 + unk_x1, cur_x + 2 + range_x - unk_x2);
                    let pt_y = self.rand_swap(cur_y + 2 + unk_y1, cur_y + 2 + range_y - unk_y2);
                    let cell = &mut grid[gi(x, y)];
                    (cell.start_x, cell.start_y) = (pt_x, pt_y);
                    (cell.end_x, cell.end_y) = (pt_x + 1, pt_y + 1);
                    let tile = self.tile(pt_x, pt_y);
                    tile.set_kind(OPEN);
                    tile.room = ANCHOR;
                    continue;
                }
                let mut size_x = self.rand_swap(5, range_x);
                let mut size_y = self.rand_swap(4, range_y);
                // Small rooms get odd dimensions.
                if size_x | 1 < range_x {
                    size_x |= 1;
                }
                if size_y | 1 < range_y {
                    size_y |= 1;
                }
                // Keep the aspect ratio between 2/3 and 3/2.
                if size_x > size_y * 3 / 2 {
                    size_x = size_y * 3 / 2;
                }
                if size_y > size_x * 3 / 2 {
                    size_y = size_x * 3 / 2;
                }
                let start_x = self.rand_force(range_x - size_x) + cur_x + 2;
                let start_y = self.rand_force(range_y - size_y) + cur_y + 2;
                let (end_x, end_y) = (start_x + size_x, start_y + size_y);
                {
                    let cell = &mut grid[gi(x, y)];
                    (cell.start_x, cell.start_y, cell.end_x, cell.end_y) = (start_x, start_y, end_x, end_y);
                }
                self.carve_room(start_x, start_y, end_x, end_y, room_number);
                // Secondary structure or imperfections, never both.
                let mut flag_secondary = self.rand(100) < SECONDARY_CHANCE;
                if self.secondary_structures == 0 {
                    flag_secondary = false;
                }
                let mut flag_imperfect = imperfect;
                if flag_secondary && flag_imperfect {
                    if self.rand(100) < 50 {
                        flag_imperfect = false;
                    } else {
                        flag_secondary = false;
                    }
                }
                let cell = &mut grid[gi(x, y)];
                cell.flag_imperfect |= flag_imperfect;
                cell.flag_secondary |= flag_secondary;
                room_number = room_number.wrapping_add(1);
            }
        }
    }
}
