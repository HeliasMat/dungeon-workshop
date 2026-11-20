//! Hallways between rooms.

use super::*;

impl Generator<'_> {
    /// Links grid cells by a random walk with momentum, then gives dead-end
    /// anchors more connections. The dead-end part reproduces the original's
    /// bugs: it tests the walk's last position instead of the anchor's, and
    /// checks the wrong neighbour's validity.
    pub(super) fn create_connections(&mut self, grid: &mut Grid, nx: i32, ny: i32, start: (i32, i32)) {
        let (mut pt_x, mut pt_y) = start;
        let mut direction = self.rand(4);
        for _ in 0..self.prop.floor_connectivity {
            let test = self.rand(8);
            let new_direction = self.rand(4);
            if test < 4 {
                direction = new_direction;
            }
            loop {
                let fits = match direction & 3 {
                    0 => pt_x < nx - 1,
                    1 => pt_y > 0,
                    2 => pt_x > 0,
                    _ => pt_y < ny - 1,
                };
                if fits {
                    break;
                }
                direction += 1;
            }
            direction &= 3;
            match direction {
                0 if !grid[gi(pt_x + 1, pt_y)].invalid => {
                    grid[gi(pt_x, pt_y)].right = true;
                    grid[gi(pt_x + 1, pt_y)].left = true;
                    pt_x += 1;
                }
                1 if !grid[gi(pt_x, pt_y - 1)].invalid => {
                    grid[gi(pt_x, pt_y)].top = true;
                    grid[gi(pt_x, pt_y - 1)].bottom = true;
                    pt_y -= 1;
                }
                2 if !grid[gi(pt_x - 1, pt_y)].invalid => {
                    grid[gi(pt_x, pt_y)].left = true;
                    grid[gi(pt_x - 1, pt_y)].right = true;
                    pt_x -= 1;
                }
                3 if !grid[gi(pt_x, pt_y + 1)].invalid => {
                    grid[gi(pt_x, pt_y)].bottom = true;
                    grid[gi(pt_x, pt_y + 1)].top = true;
                    pt_y += 1;
                }
                _ => {}
            }
        }
        if self.prop.dead_ends {
            return;
        }
        let mut more = true;
        while more {
            more = false;
            for y0 in 0..ny {
                // The reference changes `y` inside the row; it carries over
                // to the row's remaining cells.
                let mut y = y0;
                for x in 0..nx {
                    let cell = grid[gi(x, y)];
                    if cell.invalid || cell.is_room {
                        continue;
                    }
                    let links = [cell.top, cell.bottom, cell.left, cell.right];
                    if links.iter().filter(|&&link| link).count() != 1 {
                        continue;
                    }
                    let mut direction = self.rand(4);
                    for _ in 0..8 {
                        let free = match direction & 3 {
                            0 => pt_x < nx - 1 && !cell.right,
                            1 => pt_y > 0 && !cell.top,
                            2 => pt_x > 0 && !cell.left,
                            _ => pt_y < ny - 1 && !cell.bottom,
                        };
                        if free {
                            break;
                        }
                        direction += 1;
                    }
                    direction &= 3;
                    // (Wrong validity check for directions 1 to 3, as in the
                    // original.)
                    if grid[gi(x + 1, y)].invalid {
                        continue;
                    }
                    more = true;
                    match direction {
                        0 => {
                            grid[gi(x, y)].right = true;
                            grid[gi(x + 1, y)].left = true;
                        }
                        1 => {
                            grid[gi(x, y)].top = true;
                            grid[gi(x, y - 1)].bottom = true;
                            y -= 1;
                        }
                        2 => {
                            grid[gi(x, y)].left = true;
                            grid[gi(x - 1, y)].right = true;
                        }
                        _ => {
                            grid[gi(x, y)].bottom = true;
                            grid[gi(x, y + 1)].top = true;
                            y += 1;
                        }
                    }
                }
            }
        }
    }

    /// Digs one step of a hallway. Returns false when the hallway must stop
    /// because it met open terrain away from its start.
    pub(super) fn dig(&mut self, x: i32, y: i32, at_start: bool) -> bool {
        if self.open_at(x, y) {
            at_start
        } else {
            self.tile(x, y).set_kind(OPEN);
            true
        }
    }

    /// A hallway between two points, with its kink on a grid cell boundary.
    pub(super) fn process_hallway(&mut self, mut x: i32, mut y: i32, x2: i32, y2: i32, vertical: bool, kink_x: i32, kink_y: i32) {
        let (x0, y0) = (x, y);
        let step = |from: i32, to: i32| if from >= to { -1 } else { 1 };
        if !vertical {
            let mut counter = 0;
            while x != kink_x {
                if counter >= 56 {
                    return;
                }
                counter += 1;
                if !self.dig(x, y, x0 == x) {
                    return;
                }
                x += step(x, kink_x);
            }
            counter = 0;
            while y != y2 {
                if counter >= 56 {
                    return;
                }
                counter += 1;
                if !self.dig(x, y, x0 == x && y0 == y) {
                    return;
                }
                y += step(y, y2);
            }
            counter = 0;
            while x != x2 {
                if counter >= 56 {
                    return;
                }
                counter += 1;
                if !self.dig(x, y, x0 == x && y0 == y) {
                    return;
                }
                x += step(x, x2);
            }
        } else {
            let mut counter = 0;
            while y != kink_y {
                if counter >= 56 {
                    return;
                }
                counter += 1;
                if !self.dig(x, y, y0 == y) {
                    return;
                }
                y += step(y, kink_y);
            }
            counter = 0;
            while x != x2 {
                if counter >= 56 {
                    return;
                }
                counter += 1;
                if !self.dig(x, y, x0 == x && y0 == y) {
                    return;
                }
                x += step(x, x2);
            }
            counter = 0;
            while y != y2 {
                if counter >= 56 {
                    return;
                }
                counter += 1;
                if !self.dig(x, y, x0 == x && y0 == y) {
                    return;
                }
                y += step(y, y2);
            }
        }
    }

    /// A random point strictly inside the room along one axis.
    pub(super) fn inner(&mut self, start: i32, end: i32) -> i32 {
        self.rand_between(start + 1, end - 1)
    }

    pub(super) fn create_hallways(&mut self, grid: &mut Grid, nx: i32, ny: i32, list_x: &[i32], list_y: &[i32], no_merge: bool) {
        for y in 0..ny {
            for x in 0..nx {
                let cell = &mut grid[gi(x, y)];
                if cell.invalid {
                    (cell.work_top, cell.work_bottom, cell.work_left, cell.work_right) = (false, false, false, false);
                    continue;
                }
                // Drop connections leaving the grid.
                if x == 0 {
                    cell.left = false;
                }
                if y == 0 {
                    cell.top = false;
                }
                if x == nx - 1 {
                    cell.right = false;
                }
                if y == ny - 1 {
                    cell.bottom = false;
                }
                (cell.work_top, cell.work_bottom, cell.work_left, cell.work_right) = (cell.top, cell.bottom, cell.left, cell.right);
            }
        }
        for x in 0..nx {
            for y in 0..ny {
                let cell = grid[gi(x, y)];
                if cell.invalid {
                    continue;
                }
                let (pt_x, pt_y) = if cell.is_room {
                    let px = self.inner(cell.start_x, cell.end_x);
                    (px, self.inner(cell.start_y, cell.end_y))
                } else {
                    (cell.start_x, cell.start_y)
                };
                if grid[gi(x, y)].work_top {
                    let other = grid[gi(x, y - 1)];
                    if !other.invalid {
                        let pt2_x = if other.is_room { self.inner(other.start_x, other.end_x) } else { other.start_x };
                        let (kx, ky) = (list_x[x as usize], list_y[y as usize]);
                        self.process_hallway(pt_x, cell.start_y, pt2_x, other.end_y - 1, true, kx, ky);
                    }
                    grid[gi(x, y)].work_top = false;
                    grid[gi(x, y - 1)].work_bottom = false;
                    grid[gi(x, y)].connected = true;
                    grid[gi(x, y - 1)].connected = true;
                }
                if grid[gi(x, y)].work_bottom {
                    let other = grid[gi(x, y + 1)];
                    if !other.invalid {
                        let pt2_x = if other.is_room { self.inner(other.start_x, other.end_x) } else { other.start_x };
                        let (kx, ky) = (list_x[x as usize], list_y[y as usize + 1] - 1);
                        self.process_hallway(pt_x, cell.end_y - 1, pt2_x, other.start_y, true, kx, ky);
                    }
                    grid[gi(x, y)].work_bottom = false;
                    grid[gi(x, y + 1)].work_top = false;
                    grid[gi(x, y)].connected = true;
                    grid[gi(x, y + 1)].connected = true;
                }
                if grid[gi(x, y)].work_left {
                    let other = grid[gi(x - 1, y)];
                    if !other.invalid {
                        let pt2_y = if other.is_room { self.inner(other.start_y, other.end_y) } else { other.start_y };
                        // (The end point uses the other cell's start, as in
                        // the original; the hallway stops at the room anyway.)
                        let (kx, ky) = (list_x[x as usize], list_y[y as usize]);
                        self.process_hallway(cell.start_x, pt_y, other.start_x - 1, pt2_y, false, kx, ky);
                    }
                    grid[gi(x, y)].work_left = false;
                    grid[gi(x - 1, y)].work_right = false;
                    grid[gi(x, y)].connected = true;
                    grid[gi(x - 1, y)].connected = true;
                }
                if grid[gi(x, y)].work_right {
                    let other = grid[gi(x + 1, y)];
                    if !other.invalid {
                        let pt2_y = if other.is_room { self.inner(other.start_y, other.end_y) } else { other.start_y };
                        let (kx, ky) = (list_x[x as usize + 1] - 1, list_y[y as usize]);
                        self.process_hallway(cell.end_x - 1, pt_y, other.start_x, pt2_y, false, kx, ky);
                    }
                    grid[gi(x, y)].work_right = false;
                    grid[gi(x + 1, y)].work_left = false;
                    grid[gi(x, y)].connected = true;
                    grid[gi(x + 1, y)].connected = true;
                }
            }
        }
        if no_merge {
            return;
        }
        // Merge some connected rooms.
        let mergeable = |c: &Cell| !c.invalid && c.connected && !c.merged && !c.structured && c.is_room;
        for x in 0..nx {
            for y in 0..ny {
                let chance = self.rand(100);
                if chance >= MERGE_CHANCE || !mergeable(&grid[gi(x, y)]) {
                    continue;
                }
                let side = self.rand(4);
                let (ox, oy) = match side {
                    0 if x >= 1 => (x - 1, y),
                    1 if y >= 1 => (x, y - 1),
                    2 if x <= nx - 2 => (x + 1, y),
                    3 if y <= ny - 2 => (x, y + 1),
                    _ => continue,
                };
                if !mergeable(&grid[gi(ox, oy)]) {
                    continue;
                }
                let (this, other) = (grid[gi(x, y)], grid[gi(ox, oy)]);
                // The merged room spans from the first room's start to the
                // second's end along the merge axis, and both extents across.
                let (first, second) = if side == 0 || side == 1 { (other, this) } else { (this, other) };
                let (src_x, dst_x, src_y, dst_y) = if side == 0 || side == 2 {
                    (first.start_x, second.end_x, this.start_y.min(other.start_y), this.end_y.max(other.end_y))
                } else {
                    (this.start_x.min(other.start_x), this.end_x.max(other.end_x), first.start_y, second.end_y)
                };
                let room = self.at(this.start_x, this.start_y).room;
                self.carve_room(src_x, src_y, dst_x, dst_y, room);
                let target = &mut grid[gi(ox, oy)];
                (target.start_x, target.start_y, target.end_x, target.end_y) = (src_x, src_y, dst_x, dst_y);
                target.merged = true;
                let source = &mut grid[gi(x, y)];
                source.merged = true;
                source.connected = false;
                source.was_merged = true;
            }
        }
    }

    /// Connects rooms the walk left out to a connected neighbour, and fills
    /// in whatever is still unconnected.
    pub(super) fn add_hallways(&mut self, grid: &mut Grid, nx: i32, ny: i32, list_x: &[i32], list_y: &[i32]) {
        let linkable = |c: &Cell| !c.invalid && !c.merged && c.connected;
        for x in 0..nx {
            for y in 0..ny {
                let cell = grid[gi(x, y)];
                if cell.invalid || cell.connected || cell.was_merged {
                    continue;
                }
                if !(cell.is_room && !cell.structured) {
                    // An unconnected anchor: back to wall.
                    let tile = self.tile(cell.start_x, cell.start_y);
                    tile.set_kind(0);
                    tile.spawn &= !0x7;
                    continue;
                }
                let rnd_x = self.inner(cell.start_x, cell.end_x);
                let rnd_y = self.inner(cell.start_y, cell.end_y);
                // The reference draws both coordinates of the neighbour's
                // point even though only one is used.
                if y > 0 && linkable(&grid[gi(x, y - 1)]) {
                    let other = grid[gi(x, y - 1)];
                    let pt_x = if other.is_room {
                        let px = self.inner(other.start_x, other.end_x);
                        self.inner(other.start_y, other.end_y);
                        px
                    } else {
                        other.start_x
                    };
                    let (kx, ky) = (list_x[x as usize], list_y[y as usize]);
                    self.process_hallway(rnd_x, cell.start_y, pt_x, other.end_y - 1, true, kx, ky);
                    grid[gi(x, y)].connected = true;
                    grid[gi(x, y)].top = true;
                    grid[gi(x, y - 1)].bottom = true;
                } else if y < ny - 1 && linkable(&grid[gi(x, y + 1)]) {
                    let other = grid[gi(x, y + 1)];
                    let pt_x = if other.is_room {
                        let px = self.inner(other.start_x, other.end_x);
                        self.inner(other.start_y, other.end_y);
                        px
                    } else {
                        other.start_x
                    };
                    let (kx, ky) = (list_x[x as usize], list_y[y as usize + 1] - 1);
                    self.process_hallway(rnd_x, cell.end_y - 1, pt_x, other.start_y, true, kx, ky);
                    grid[gi(x, y)].connected = true;
                    grid[gi(x, y)].bottom = true;
                    grid[gi(x, y + 1)].top = true;
                } else if x > 0 && linkable(&grid[gi(x - 1, y)]) {
                    let other = grid[gi(x - 1, y)];
                    let pt_y = if other.is_room {
                        self.inner(other.start_x, other.end_x);
                        self.inner(other.start_y, other.end_y)
                    } else {
                        other.start_y
                    };
                    let (kx, ky) = (list_x[x as usize], list_y[y as usize]);
                    self.process_hallway(cell.start_x, rnd_y, other.start_x - 1, pt_y, false, kx, ky);
                    grid[gi(x, y)].connected = true;
                    grid[gi(x, y)].left = true;
                    grid[gi(x - 1, y)].right = true;
                } else if x < nx - 1 && linkable(&grid[gi(x + 1, y)]) {
                    let other = grid[gi(x + 1, y)];
                    let pt_y = if other.is_room {
                        self.inner(other.start_x, other.end_x);
                        self.inner(other.start_y, other.end_y)
                    } else {
                        other.start_y
                    };
                    let (kx, ky) = (list_x[x as usize + 1] - 1, list_y[y as usize]);
                    self.process_hallway(cell.end_x - 1, rnd_y, other.start_x, pt_y, false, kx, ky);
                    grid[gi(x, y)].connected = true;
                    grid[gi(x, y)].right = true;
                    // (The original flags the cell on the wrong side.)
                    grid[gi(x - 1, y)].left = true;
                }
            }
        }
        for y in 0..ny {
            for x in 0..nx {
                let cell = grid[gi(x, y)];
                if cell.invalid || cell.was_merged || cell.connected || cell.unk_f {
                    continue;
                }
                for cx in cell.start_x..cell.end_x {
                    for cy in cell.start_y..cell.end_y {
                        let tile = self.tile(cx, cy);
                        tile.set_kind(0);
                        tile.spawn &= !0x7;
                        tile.room = NO_ROOM;
                    }
                }
            }
        }
    }

    // --- mazes ---
}
