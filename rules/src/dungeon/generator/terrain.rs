//! Water, lava, chasms: the secondary terrain.

use super::*;

impl Generator<'_> {
    /// Lays secondary terrain on a plain wall.
    pub(super) fn flood(&mut self, x: i32, y: i32) {
        if !(0..W).contains(&x) || !(0..H).contains(&y) {
            return;
        }
        let tile = self.tile(x, y);
        if tile.terrain & (3 | IMPASSABLE) == 0 {
            tile.set_kind(SECONDARY);
        }
    }

    pub(super) fn secondary_at(&self, x: i32, y: i32) -> bool {
        self.at(x, y).kind() == SECONDARY
    }

    /// Rivers flowing up or down the floor, lakes on their way, and
    /// standalone lakes.
    pub(super) fn generate_secondary_terrain(&mut self) {
        if self.prop.flags & 1 == 0 {
            return;
        }
        let rivers = [1, 1, 1, 2, 2, 2, 3, 3][self.rand(8) as usize];
        for _ in 0..rivers {
            let upwards = self.rand(100) < 50;
            let (mut pt_y, mut dir_y) = if upwards { (H - 1, -1) } else { (0, 1) };
            let mut steps_until_lake = self.rand(50) + 10;
            let mut pt_x = self.rand_between(2, 54);
            let mut dir_x = 0;
            let mut done = false;
            while !done {
                for _ in 0..self.rand(6) + 2 {
                    if (0..W).contains(&pt_x) {
                        if (0..H).contains(&pt_y) && self.secondary_at(pt_x, pt_y) {
                            done = true;
                            break;
                        }
                        self.flood(pt_x, pt_y);
                    }
                    pt_x += dir_x;
                    pt_y += dir_y;
                    if !(0..H).contains(&pt_y) {
                        break;
                    }
                    steps_until_lake -= 1;
                    if steps_until_lake != 0 {
                        continue;
                    }
                    // A lake: random tiles around the cursor, kept when they
                    // touch secondary terrain, then gaps filled in.
                    for _ in 0..0x40 {
                        let (tx, ty) = (pt_x + self.rand(7) - 3, pt_y + self.rand(7) - 3);
                        if !(2..54).contains(&tx) || !(2..30).contains(&ty) {
                            continue;
                        }
                        let near = (-1..=1).any(|dx| (-1..=1).any(|dy| (dx, dy) != (0, 0) && self.secondary_at(tx + dx, ty + dy)));
                        if near {
                            self.flood(tx, ty);
                        }
                    }
                    for off_x in -3..=3 {
                        for off_y in -3..=3 {
                            let (tx, ty) = (pt_x + off_x, pt_y + off_y);
                            if !(2..54).contains(&tx) || !(2..30).contains(&ty) {
                                continue;
                            }
                            let around = (-1..=1)
                                .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
                                .filter(|&(dx, dy)| (dx, dy) != (0, 0) && self.secondary_at(tx + dx, ty + dy))
                                .count();
                            if around >= 4 {
                                self.flood(tx, ty);
                            }
                        }
                    }
                }
                if !done {
                    // Alternate between flowing sideways and onwards.
                    if dir_x != 0 {
                        dir_y = if upwards { -1 } else { 1 };
                        dir_x = 0;
                    } else {
                        dir_x = if self.rand(100) < 50 { -1 } else { 1 };
                        dir_y = 0;
                    }
                }
                if !(0..H).contains(&pt_y) {
                    done = true;
                }
            }
        }
        for _ in 0..self.prop.secondary_density {
            let mut seed = None;
            for _ in 0..200 {
                let (x, y) = (self.rand(W), self.rand(H));
                if (1..55).contains(&x) && (1..31).contains(&y) {
                    seed = Some((x, y));
                    break;
                }
            }
            let Some((lake_x, lake_y)) = seed else {
                continue;
            };
            // The border of a 10x10 table spreads inwards at random; what
            // stays clear is the lake.
            let mut table = [[false; 10]; 10];
            for (a, row) in table.iter_mut().enumerate() {
                for (b, cell) in row.iter_mut().enumerate() {
                    *cell = a == 0 || b == 0 || a == 9 || b == 9;
                }
            }
            for _ in 0..0x50 {
                let (a, b) = (self.rand(8) as usize + 1, self.rand(8) as usize + 1);
                if table[a - 1][b] || table[a + 1][b] || table[a][b + 1] || table[a][b - 1] {
                    table[a][b] = true;
                }
            }
            for (a, row) in table.iter().enumerate() {
                for (b, &dry) in row.iter().enumerate() {
                    if !dry {
                        self.flood(lake_x + a as i32 - 5, lake_y + b as i32 - 5);
                    }
                }
            }
        }
        // Secondary terrain never stays in special rooms or on the border.
        for x in 0..W {
            for y in 0..H {
                let tile = self.tile(x, y);
                if tile.kind() != SECONDARY {
                    continue;
                }
                if tile.terrain & (UNBREAKABLE | MONSTER_HOUSE | SHOP) != 0 || tile.spawn & SPAWN_STAIRS != 0 {
                    tile.set_kind(OPEN);
                } else if x <= 1 || x >= 55 || y <= 1 || y >= 31 {
                    tile.set_kind(0);
                }
            }
        }
    }

    // --- spawns ---
}
