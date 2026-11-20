//! Where stairs, items, traps and monsters go.

use super::*;

impl Generator<'_> {
    pub(super) fn tiles_where(&self, valid: impl Fn(&RawTile) -> bool) -> Vec<(i32, i32)> {
        let mut found = Vec::new();
        for x in 0..W {
            for y in 0..H {
                if valid(&self.at(x, y)) {
                    found.push((x, y));
                }
            }
        }
        found
    }

    pub(super) fn shuffle(&mut self, spawns: &mut [(i32, i32)]) {
        let count = spawns.len() as i32;
        for _ in 0..count * 2 {
            let (a, b) = (self.rand(count) as usize, self.rand(count) as usize);
            spawns.swap(a, b);
        }
    }

    /// Flags `count` of the candidate tiles, after shuffling them and
    /// starting from a random one.
    pub(super) fn mark(&mut self, mut spawns: Vec<(i32, i32)>, count: i32, flag: u16) {
        self.shuffle(&mut spawns);
        let mut start = self.rand(spawns.len() as i32) as usize;
        for _ in 0..count {
            let (x, y) = spawns[start];
            start = (start + 1) % spawns.len();
            self.tile(x, y).spawn |= flag;
        }
    }

    /// Stairs, items, buried items, Monster House contents, traps, player.
    pub(super) fn mark_non_enemy_spawns(&mut self, empty_monster_house: bool) {
        let room_tile = |t: &RawTile| t.is_open() && t.room != NO_ROOM && t.terrain & (SHOP | JUNCTION | UNBREAKABLE) == 0;
        if self.stairs_spawn.0 == -1 || self.stairs_spawn.1 == -1 {
            let spots = self.tiles_where(|t| room_tile(t) && t.spawn & (SPAWN_ENEMY | 0x10) == 0);
            if !spots.is_empty() {
                let (x, y) = spots[self.rand(spots.len() as i32) as usize];
                let tile = self.tile(x, y);
                tile.spawn = tile.spawn & !SPAWN_ITEM | SPAWN_STAIRS;
                self.stairs_spawn = (x, y);
            }
        }

        let spots = self.tiles_where(|t| room_tile(t) && t.terrain & MONSTER_HOUSE == 0);
        if !spots.is_empty() {
            let mut items = self.prop.item_density;
            if items != 0 {
                items = self.rand_between(items - 2, items + 2).max(1);
            }
            if items + 1 > 0 {
                self.mark(spots, items + 1, SPAWN_ITEM);
            }
        }

        let walls = self.tiles_where(|t| t.kind() == 0);
        if !walls.is_empty() {
            let mut buried = self.prop.buried_item_density;
            if buried != 0 {
                buried = self.rand_between(buried - 2, buried + 2);
            }
            if buried > 0 {
                self.mark(walls, buried, SPAWN_ITEM);
            }
        }

        if !empty_monster_house {
            let mut spots = self.tiles_where(|t| t.terrain & SHOP == 0 && t.terrain & MONSTER_HOUSE != 0 && t.terrain & JUNCTION == 0);
            if !spots.is_empty() {
                let count = spots.len() as i32;
                let things = self.rand_swap(5 * count / 10, 8 * count / 10).clamp(6, MH_ITEMS);
                self.shuffle(&mut spots);
                let mut start = self.rand(count) as usize;
                for _ in 0..things {
                    let (x, y) = spots[start];
                    start = (start + 1) % spots.len();
                    // Half items, half traps.
                    let flag = if self.rand(2) == 1 { SPAWN_ITEM } else { SPAWN_TRAP };
                    self.tile(x, y).spawn |= flag;
                }
            }
        }

        let spots = self.tiles_where(|t| room_tile(t) && t.spawn & SPAWN_ITEM == 0);
        if !spots.is_empty() {
            let traps = self.rand_swap(self.prop.trap_density / 2, self.prop.trap_density).min(56);
            if traps > 0 {
                self.mark(spots, traps, SPAWN_TRAP);
            }
        }

        if self.player_spawn.0 == -1 || self.player_spawn.1 == -1 {
            let spots = self.tiles_where(|t| room_tile(t) && t.spawn & (SPAWN_ITEM | SPAWN_ENEMY | SPAWN_TRAP) == 0);
            if !spots.is_empty() {
                self.player_spawn = spots[self.rand(spots.len() as i32) as usize];
            }
        }
    }

    pub(super) fn mark_enemy_spawns(&mut self, empty_monster_house: bool) {
        let density = self.prop.enemy_density;
        let enemies = if density < 1 { density.abs() } else { self.rand_between(density / 2, density).max(1) };
        let player = self.player_spawn;
        let mut spots = self.tiles_where(|t| {
            t.is_open() && t.room != NO_ROOM && t.terrain & (SHOP | JUNCTION | UNBREAKABLE) == 0 && t.spawn & (SPAWN_ITEM | SPAWN_STAIRS) == 0
        });
        spots.retain(|&spot| spot != player);
        if !spots.is_empty() && enemies + 1 > 0 {
            self.mark(spots, enemies + 1, SPAWN_ENEMY);
        }
        if !self.create_monster_house {
            return;
        }
        let limit = (if empty_monster_house { 3 } else { MH_ENEMIES }) * 3 / 2;
        let mut spots = self.tiles_where(|t| t.is_open() && t.room != NO_ROOM && t.terrain & (SHOP | UNBREAKABLE) == 0 && t.terrain & MONSTER_HOUSE != 0);
        spots.retain(|&spot| spot != player);
        if !spots.is_empty() {
            let count = spots.len() as i32;
            let enemies = self.rand_between(7 * count / 10, 8 * count / 10).max(1).min(limit);
            self.mark(spots, enemies, SPAWN_ENEMY);
        }
    }

    /// Removes spawns that cannot be, and lets stairs and items win over
    /// traps.
    pub(super) fn resolve_invalid_spawns(&mut self) {
        for tile in &mut self.tiles {
            if !tile.is_open() {
                if tile.terrain & (UNBREAKABLE | IMPASSABLE) != 0 {
                    tile.spawn &= !SPAWN_ITEM;
                }
                tile.spawn &= !SPAWN_TRAP;
            }
            if tile.spawn & SPAWN_STAIRS != 0 {
                tile.terrain |= STAIRS;
                tile.spawn &= !SPAWN_TRAP;
            }
            if tile.spawn & SPAWN_ITEM != 0 {
                tile.spawn &= !SPAWN_TRAP;
            }
        }
    }

    /// Whether every walkable tile can reach the stairs, moving like a
    /// monster (no cutting corners around walls).
    pub(super) fn stairs_always_reachable(&self) -> bool {
        let (sx, sy) = self.stairs_spawn;
        // 0x1: obstacle, 0x2: secondary terrain, 0x40: queued, 0x80: visited.
        let mut marks = vec![0u8; (W * H) as usize];
        let index = |x: i32, y: i32| (x * H + y) as usize;
        for x in 0..W {
            for y in 0..H {
                let tile = self.at(x, y);
                if !tile.is_open() && tile.terrain & CORNER_CUTTABLE == 0 {
                    marks[index(x, y)] |= 0x1;
                }
                if tile.kind() == SECONDARY && tile.terrain & CORNER_CUTTABLE == 0 {
                    marks[index(x, y)] |= 0x2;
                }
            }
        }
        marks[index(sx, sy)] |= 0x50;
        let mut progressed = true;
        while progressed {
            progressed = false;
            for x in 0..W {
                for y in 0..H {
                    if marks[index(x, y)] & 0x80 != 0 || marks[index(x, y)] & 0x40 == 0 {
                        continue;
                    }
                    marks[index(x, y)] = marks[index(x, y)] & !0x40 | 0x80;
                    progressed = true;
                    for (dx, dy) in [(-1, 0), (0, -1), (1, 0), (0, 1)] {
                        let (tx, ty) = (x + dx, y + dy);
                        if (0..W).contains(&tx) && (0..H).contains(&ty) && marks[index(tx, ty)] & 0x83 == 0 {
                            marks[index(tx, ty)] |= 0x40;
                        }
                    }
                    for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                        let (tx, ty) = (x + dx, y + dy);
                        if (0..W).contains(&tx)
                            && (0..H).contains(&ty)
                            && marks[index(tx, ty)] & 0x87 == 0
                            && marks[index(x, ty)] & 0x1 == 0
                            && marks[index(tx, y)] & 0x1 == 0
                        {
                            marks[index(tx, ty)] |= 0x40;
                        }
                    }
                }
            }
        }
        (0..W).all(|x| (0..H).all(|y| marks[index(x, y)] & 0x87 != 0 || self.at(x, y).terrain & UNBREAKABLE != 0))
    }

    // --- the whole thing ---
}
