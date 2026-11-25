//! Floor generation: a port of SkyTemple's `dungeon-eos`
//! (`DungeonAlgorithm.py`, GPL-3), itself a transcription of the game's own
//! generator. It is kept close to that reference on purpose, quirks and
//! bugs of the original included, so that a seed gives the same floor.
//!
//! Left out, as in the reference: fixed rooms (the vaults set inside a
//! floor: `run::vault`). Left out here: hidden stairs, which use a second
//! random generator (`run::secret` draws them after, with its own), and
//! rescue floors.

mod features;
mod finish;
mod floor;
mod hallways;
mod layouts;
mod maze;
mod rooms;
mod spawns;
mod terrain;

use super::floor::{FLOOR_HEIGHT, FLOOR_WIDTH, Floor, Pos, Rect, Tile};
use super::rng::DungeonRng;

/// The per-floor properties stored in `BALANCE/mappa_s.bin` that drive
/// generation.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct FloorProperties {
    /// Which generator to use (1 to 11; anything else is the standard one).
    pub layout: u8,
    /// Positive: a random count in `[n, n + 2]`. Negative: exactly `|n|`.
    pub room_density: i32,
    /// Number of random-walk steps used to link grid cells.
    pub floor_connectivity: i32,
    /// Whether hallways leading nowhere are kept.
    pub dead_ends: bool,
    pub extra_hallways: i32,
    /// Bit 0: secondary terrain formations. Bit 2: room imperfections.
    pub flags: u8,
    /// Number of standalone lakes.
    pub secondary_density: i32,
    /// Chances out of 100.
    pub kecleon_shop_chance: i32,
    pub monster_house_chance: i32,
    pub maze_room_chance: i32,
    pub empty_monster_house_chance: i32,
    /// How many rooms may get a secondary structure.
    pub secondary_structures: i32,
    /// Positive: a random count up to this. Otherwise exactly `|n|`.
    pub enemy_density: i32,
    pub item_density: i32,
    pub buried_item_density: i32,
    pub trap_density: i32,
    /// A floor of one single room is no Monster House
    /// (`RuleOptions::disarmed_houses`): never in the game's tables.
    #[serde(skip)]
    pub disarmed_lone_house: bool,
}

impl Default for FloorProperties {
    fn default() -> Self {
        Self {
            layout: 0,
            room_density: 6,
            floor_connectivity: 15,
            dead_ends: false,
            extra_hallways: 0,
            flags: 0,
            secondary_density: 0,
            kecleon_shop_chance: 0,
            monster_house_chance: 0,
            maze_room_chance: 0,
            empty_monster_house_chance: 0,
            secondary_structures: 0,
            enemy_density: 6,
            item_density: 3,
            buried_item_density: 0,
            trap_density: 4,
            disarmed_lone_house: false,
        }
    }
}

const W: i32 = FLOOR_WIDTH;
const H: i32 = FLOOR_HEIGHT;
const GRID: i32 = 15;

const MERGE_CHANCE: i32 = 5;
const IMPERFECT_CHANCE: i32 = 60;
const SECONDARY_CHANCE: i32 = 80;
const MH_ENEMIES: i32 = 30;
const MH_ITEMS: i32 = 7;

// Terrain flags.
const OPEN: u16 = 1;
const SECONDARY: u16 = 2;
const CORNER_CUTTABLE: u16 = 0x4;
const JUNCTION: u16 = 0x8;
const IMPASSABLE: u16 = 0x10;
const SHOP: u16 = 0x20;
const MONSTER_HOUSE: u16 = 0x40;
const UNBREAKABLE: u16 = 0x100;
const STAIRS: u16 = 0x200;
// Spawn flags.
const SPAWN_STAIRS: u16 = 0x1;
const SPAWN_ITEM: u16 = 0x2;
const SPAWN_TRAP: u16 = 0x4;
const SPAWN_ENEMY: u16 = 0x8;

const NO_ROOM: u8 = 0xFF;
const ANCHOR: u8 = 0xFE;

/// The eight directions, starting down and turning counter-clockwise.
const DIRECTIONS: [(i32, i32); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];
/// For each corner a room imperfection grows from, whether each of the
/// eight neighbours must be open.
const CORNER_CHECKS: [u8; 32] = [1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0];

#[derive(Clone, Copy)]
struct RawTile {
    terrain: u16,
    spawn: u16,
    room: u8,
}

impl RawTile {
    const DEFAULT: Self = Self { terrain: 0, spawn: 0, room: NO_ROOM };

    fn kind(&self) -> u16 {
        self.terrain & 3
    }

    fn is_open(&self) -> bool {
        self.kind() == OPEN
    }

    fn set_kind(&mut self, kind: u16) {
        self.terrain = self.terrain & !3 | kind;
    }
}

#[derive(Clone, Copy, Default)]
struct Cell {
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    invalid: bool,
    /// Has a secondary structure.
    structured: bool,
    is_room: bool,
    connected: bool,
    unk_c: bool,
    is_monster_house: bool,
    unk_f: bool,
    is_maze: bool,
    /// Absorbed into another room by a merge.
    was_merged: bool,
    /// Took part in a merge.
    merged: bool,
    top: bool,
    bottom: bool,
    left: bool,
    right: bool,
    work_top: bool,
    work_bottom: bool,
    work_left: bool,
    work_right: bool,
    flag_imperfect: bool,
    flag_secondary: bool,
}

type Grid = Vec<Cell>;

/// Grid cells are indexed like the reference's Python lists: a negative
/// index wraps around.
fn gi(x: i32, y: i32) -> usize {
    (x.rem_euclid(GRID) * GRID + y.rem_euclid(GRID)) as usize
}

struct Generator<'a> {
    rng: &'a mut DungeonRng,
    prop: FloorProperties,
    tiles: Vec<RawTile>,
    // Generation status.
    has_monster_house: bool,
    has_kecleon_shop: bool,
    floor_size: u8,
    room_count: i32,
    secondary_structures: i32,
    shop_min: (i32, i32),
    shop_max: (i32, i32),
    // Dungeon data.
    create_monster_house: bool,
    monster_house_room: i32,
    player_spawn: (i32, i32),
    stairs_spawn: (i32, i32),
    invalid_generation: bool,
}

pub fn generate_floor(rng: &mut DungeonRng, props: &FloorProperties) -> Floor {
    let mut generator = new_generator(rng, props);
    generator.generate();
    generator.into_floor()
}

fn new_generator<'a>(rng: &'a mut DungeonRng, props: &FloorProperties) -> Generator<'a> {
    Generator {
        rng,
        prop: *props,
        tiles: vec![RawTile::DEFAULT; (W * H) as usize],
        has_monster_house: false,
        has_kecleon_shop: false,
        floor_size: 0,
        room_count: 0,
        secondary_structures: props.secondary_structures,
        shop_min: (0, 0),
        shop_max: (0, 0),
        create_monster_house: false,
        monster_house_room: -1,
        player_spawn: (-1, -1),
        stairs_spawn: (-1, -1),
        invalid_generation: false,
    }
}

impl Generator<'_> {
    // --- random numbers, as the reference draws them ---
}

impl Generator<'_> {
    fn random(&mut self) -> i64 {
        self.rng.rand16() as i64
    }

    /// `randrange(a, b)`: no check that `a < b`.
    fn rand_between(&mut self, a: i32, b: i32) -> i32 {
        let r = self.random();
        ((((b - a) as i64 * r) >> 16) & 0xFFFF) as i32 + a
    }

    fn rand(&mut self, n: i32) -> i32 {
        self.rand_between(0, n)
    }

    /// `randrangeswap`: bounds in either order; equal bounds draw nothing.
    fn rand_swap(&mut self, a: i32, b: i32) -> i32 {
        match a.cmp(&b) {
            std::cmp::Ordering::Equal => a,
            std::cmp::Ordering::Greater => self.rand_between(b, a),
            std::cmp::Ordering::Less => self.rand_between(a, b),
        }
    }

    /// `randrangeforce`: always draws a number, even for an empty range.
    fn rand_force(&mut self, n: i32) -> i32 {
        if n == 0 {
            self.random();
            0
        } else {
            self.rand(n)
        }
    }

    // --- tiles ---

    fn tile(&mut self, x: i32, y: i32) -> &mut RawTile {
        &mut self.tiles[(x.rem_euclid(W) * H + y.rem_euclid(H)) as usize]
    }

    fn at(&self, x: i32, y: i32) -> RawTile {
        self.tiles[(x.rem_euclid(W) * H + y.rem_euclid(H)) as usize]
    }

    fn open_at(&self, x: i32, y: i32) -> bool {
        self.at(x, y).is_open()
    }

    fn carve_room(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, room: u8) {
        for x in x0..x1 {
            for y in y0..y1 {
                let tile = self.tile(x, y);
                tile.set_kind(OPEN);
                tile.room = room;
            }
        }
    }

    fn reinit_tiles(&mut self) {
        self.tiles.fill(RawTile::DEFAULT);
        for x in 0..W {
            for y in 0..H {
                if x == 0 || y == 0 || x == W - 1 || y == H - 1 {
                    self.tile(x, y).terrain |= IMPASSABLE;
                }
            }
        }
        self.stairs_spawn = (-1, -1);
    }

    // --- the grid ---
}

#[cfg(test)]
mod tests;
