//! Mazes in rooms.

use super::*;

impl Generator<'_> {
    pub(super) fn store_check_tile(&mut self, x: i32, y: i32, secondary: bool, room: u8) {
        let tile = self.tile(x, y);
        tile.set_kind(0);
        if secondary && tile.room == room {
            tile.terrain |= SECONDARY;
        }
    }

    /// Random walk with a stride of two, laying obstacles, until trapped.
    pub(super) fn line_maze(&mut self, mut x: i32, mut y: i32, cell: &Cell, secondary: bool, room: u8) {
        const STRIDES: [(i32, i32); 4] = [(2, 0), (0, -2), (-2, 0), (0, 2)];
        loop {
            let mut direction = self.rand(4);
            self.store_check_tile(x, y, secondary, room);
            let mut found = None;
            for _ in 0..4 {
                direction &= 3;
                let (dx, dy) = STRIDES[direction as usize];
                let (tx, ty) = (x + dx, y + dy);
                let inside = (cell.start_x..cell.end_x).contains(&tx) && (cell.start_y..cell.end_y).contains(&ty);
                if inside && self.open_at(tx, ty) {
                    found = Some((dx, dy));
                    break;
                }
                direction += 1;
            }
            let Some((dx, dy)) = found else { return };
            self.store_check_tile(x + dx / 2, y + dy / 2, secondary, room);
            (x, y) = (x + dx, y + dy);
        }
    }

    pub(super) fn create_maze(&mut self, cell: &mut Cell, secondary: bool) {
        cell.is_maze = true;
        let c = *cell;
        let room = self.at(c.start_x, c.start_y).room;
        for x in (c.start_x + 1..c.end_x - 1).step_by(2) {
            if !self.open_at(x, c.start_y - 1) {
                self.line_maze(x, c.start_y - 1, &c, secondary, room);
            }
        }
        for y in (c.start_y + 1..c.end_y - 1).step_by(2) {
            if !self.open_at(c.end_x, y) {
                self.line_maze(c.end_x, y, &c, secondary, room);
            }
        }
        for x in (c.start_x + 1..c.end_x - 1).step_by(2) {
            if !self.open_at(x, c.end_y) {
                self.line_maze(x, c.end_y, &c, secondary, room);
            }
        }
        for y in (c.start_y + 1..c.end_y - 1).step_by(2) {
            if !self.open_at(c.start_x - 1, y) {
                self.line_maze(c.start_x - 1, y, &c, secondary, room);
            }
        }
        for x in (c.start_x + 3..c.end_x - 3).step_by(2) {
            for y in (c.start_y + 3..c.end_y - 3).step_by(2) {
                if self.open_at(x, y) {
                    self.tile(x, y).set_kind(if secondary { SECONDARY } else { 0 });
                    self.line_maze(x, y, &c, secondary, room);
                }
            }
        }
    }

    /// Maze rooms are disabled in the unpatched game: only the dice roll
    /// remains.
    pub(super) fn mazify(&mut self, maze_chance: i32) {
        if maze_chance > 0 {
            self.rand(100);
        }
    }

    // --- special rooms ---
}
