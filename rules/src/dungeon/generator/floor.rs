//! The whole floor, generated and handed over.

use super::*;

impl Generator<'_> {
    pub(super) fn generate(&mut self) {
        let mut spawn_attempt = 0;
        let mut secondary_terrain = false;
        while spawn_attempt < 10 {
            self.player_spawn = (-1, -1);
            self.stairs_spawn = (-1, -1);
            let mut attempt = 0;
            while attempt < 10 {
                self.invalid_generation = false;
                if attempt >= 1 {
                    self.secondary_structures = 0;
                }
                self.reinit_tiles();
                self.player_spawn = (-1, -1);

                // Random grid dimensions, limited jointly.
                let (max_x, max_y) = if self.prop.layout == 8 { (5, 4) } else { (9, 8) };
                let (mut nx, mut ny) = (2, 2);
                let mut tries = 0x20;
                while tries > 0 {
                    nx = self.rand_between(2, max_x);
                    ny = self.rand_between(2, max_y);
                    if nx <= 6 && ny <= 4 {
                        break;
                    }
                    tries -= 1;
                }
                if tries == 0 {
                    (nx, ny) = (4, 4);
                }
                if W / nx <= 7 {
                    nx = 1;
                }
                if H / ny <= 7 {
                    ny = 1;
                }

                secondary_terrain = false;
                match self.prop.layout {
                    1 | 11 => {
                        let ny = self.rand(2) + 2;
                        self.floor_size = if self.prop.layout == 1 { 1 } else { 2 };
                        self.generate_normal_floor(4, ny);
                        secondary_terrain = true;
                    }
                    2 => self.generate_one_room_monster_house(),
                    3 => {
                        self.generate_ring();
                        secondary_terrain = true;
                    }
                    4 => {
                        self.generate_crossroads();
                        secondary_terrain = true;
                    }
                    5 => {
                        self.generate_two_rooms_monster_house();
                        self.create_monster_house = true;
                    }
                    6 => {
                        self.generate_room_line();
                        secondary_terrain = true;
                    }
                    7 => self.generate_cross(),
                    9 => self.generate_beetle(),
                    10 => {
                        self.generate_outer_rooms(nx, ny);
                        secondary_terrain = true;
                    }
                    _ => {
                        self.generate_normal_floor(nx, ny);
                        secondary_terrain = true;
                    }
                }
                self.reset_inner_boundary_rows();
                self.ensure_impassable_tiles_are_walls();

                // A good layout has at least 2 rooms and 20 room tiles.
                let mut rooms = [false; 0x40];
                let mut room_tiles = 0;
                for tile in &self.tiles {
                    if tile.is_open() && tile.room < 0xF0 {
                        room_tiles += 1;
                        if tile.room < 0x40 {
                            rooms[tile.room as usize] = true;
                        }
                    }
                }
                if rooms.iter().filter(|&&room| room).count() >= 2 && room_tiles >= 20 {
                    break;
                }
                attempt += 1;
            }
            if attempt == 10 {
                self.invalid_generation = true;
                self.generate_one_room_monster_house();
            }

            self.finalize_junctions();
            if secondary_terrain {
                self.generate_secondary_terrain();
            }
            let empty_monster_house = self.rand(100) < self.prop.empty_monster_house_chance;
            self.mark_non_enemy_spawns(empty_monster_house);
            self.mark_enemy_spawns(empty_monster_house);
            self.resolve_invalid_spawns();

            let spawned = self.player_spawn.0 != -1 && self.player_spawn.1 != -1;
            let stairs = self.stairs_spawn.0 != -1 && self.stairs_spawn.1 != -1;
            if spawned && stairs && self.stairs_always_reachable() {
                break;
            }
            spawn_attempt += 1;
        }
        if spawn_attempt == 10 {
            // Nothing worked: the default one-room Monster House.
            self.invalid_generation = true;
            self.reinit_tiles();
            self.player_spawn = (-1, -1);
            self.generate_one_room_monster_house();
            self.finalize_junctions();
            self.mark_non_enemy_spawns(false);
            self.mark_enemy_spawns(false);
            self.resolve_invalid_spawns();
        }
    }

    /// A stable fingerprint of the generated tiles, to compare floors with
    /// the reference implementation.
    #[cfg(test)]
    pub(super) fn fingerprint(&self) -> u32 {
        let mut hash = 0x811C_9DC5u32;
        let mut feed = |byte: u8| hash = (hash ^ byte as u32).wrapping_mul(0x0100_0193);
        for tile in &self.tiles {
            feed((tile.terrain & 0xFF) as u8);
            feed((tile.terrain >> 8) as u8);
            feed((tile.spawn & 0xFF) as u8);
            feed(tile.room);
        }
        for value in [self.player_spawn.0, self.player_spawn.1, self.stairs_spawn.0, self.stairs_spawn.1] {
            feed(value as u8);
        }
        hash
    }

    pub(super) fn into_floor(self) -> Floor {
        let mut floor = Floor::new_walled();
        let mut has_monster_house = false;
        for x in 0..W {
            for y in 0..H {
                let raw = self.at(x, y);
                let p = Pos::new(x, y);
                let room = (raw.room != NO_ROOM && raw.room != ANCHOR).then_some(raw.room);
                match raw.kind() {
                    OPEN => floor.set(p, Tile::Floor { room }),
                    SECONDARY => floor.set(p, Tile::Secondary),
                    _ => {}
                }
                has_monster_house |= raw.terrain & MONSTER_HOUSE != 0;
                if !raw.is_open() {
                    if raw.kind() == 0 && raw.spawn & SPAWN_ITEM != 0 {
                        floor.buried_item_spawns.push(p);
                    }
                    continue;
                }
                if raw.spawn & SPAWN_ITEM != 0 {
                    floor.item_spawns.push(p);
                }
                if raw.spawn & SPAWN_TRAP != 0 {
                    floor.trap_spawns.push(p);
                }
                if raw.spawn & SPAWN_ENEMY != 0 {
                    floor.enemy_spawns.push(p);
                }
            }
        }
        floor.stairs = Pos::new(self.stairs_spawn.0, self.stairs_spawn.1);
        floor.player_spawn = Pos::new(self.player_spawn.0, self.player_spawn.1);
        let stairs_room = self.at(self.stairs_spawn.0, self.stairs_spawn.1).room;
        floor.set(floor.stairs, Tile::Stairs { room: stairs_room });
        if self.has_kecleon_shop {
            floor.shop = Some(Rect { x0: self.shop_min.0, y0: self.shop_min.1, x1: self.shop_max.0, y1: self.shop_max.1 });
        }
        if has_monster_house && self.monster_house_room >= 0 {
            floor.monster_house = Some(self.monster_house_room as u8);
        }
        floor.fallback = self.invalid_generation;
        floor
    }
}
