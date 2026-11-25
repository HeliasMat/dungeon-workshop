//! Floor grid. Dungeon floors are always 56x32 tiles, like the original.

pub const FLOOR_WIDTH: i32 = 56;
pub const FLOOR_HEIGHT: i32 = 32;

/// Steps between two tiles, diagonals counting as one.
pub(crate) fn chebyshev(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Wall,
    /// Secondary terrain: water, lava or chasm depending on the dungeon.
    // (Who may cross it: see `Mobility`.)
    Secondary,
    /// Open terrain; `room` is `None` in hallways.
    Floor {
        room: Option<u8>,
    },
    Stairs {
        room: u8,
    },
}

impl Tile {
    pub fn is_walkable(self) -> bool {
        !matches!(self, Tile::Wall | Tile::Secondary)
    }

    pub fn room(self) -> Option<u8> {
        match self {
            Tile::Wall | Tile::Secondary => None,
            Tile::Floor { room } => room,
            Tile::Stairs { room } => Some(room),
        }
    }
}

/// What terrain a monster can stand on.
// (In order: each crosses all the one before crosses.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mobility {
    /// Open terrain only.
    #[default]
    Normal,
    /// Also the floor's secondary terrain: water for species that swim,
    /// lava for those that walk on it.
    Secondary,
    /// Any secondary terrain, chasms included.
    Hovering,
    /// Walls too.
    Intangible,
}

impl Mobility {
    /// From a species' movement type (as stored in the ROM) and the kind
    /// of secondary terrain of the floor: water (0), lava (1) or chasm (2).
    pub fn resolve(movement_type: u8, secondary_kind: u8) -> Self {
        const HOVERING: u8 = 2;
        const INTANGIBLE: u8 = 3;
        const LAVA: u8 = 4;
        const WATER: u8 = 5;
        match (movement_type, secondary_kind) {
            (HOVERING, _) => Self::Hovering,
            (INTANGIBLE, _) => Self::Intangible,
            (WATER, 0) | (LAVA, 1) => Self::Secondary,
            _ => Self::Normal,
        }
    }
}

/// Rectangle of tiles, bounds included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub fn contains(&self, p: Pos) -> bool {
        (self.x0..=self.x1).contains(&p.x) && (self.y0..=self.y1).contains(&p.y)
    }
}

#[derive(Clone, Debug)]
pub struct Floor {
    tiles: Vec<Tile>,
    pub player_spawn: Pos,
    pub stairs: Pos,
    /// Area of the Kecleon shop, if the floor has one.
    pub shop: Option<Rect>,
    /// Room that is a Monster House, if any.
    pub monster_house: Option<u8>,
    /// Generation failed too often and the default one-room floor was used.
    pub fallback: bool,
    /// Tiles the generator marked for the floor's initial items, traps and
    /// enemies.
    pub item_spawns: Vec<Pos>,
    pub trap_spawns: Vec<Pos>,
    pub enemy_spawns: Vec<Pos>,
    /// Items buried in walls.
    pub buried_item_spawns: Vec<Pos>,
    /// How far a monster sees from a hallway (less on dark floors).
    pub hallway_sight: i32,
}

impl Floor {
    pub(crate) fn new_walled() -> Self {
        Self {
            item_spawns: Vec::new(),
            trap_spawns: Vec::new(),
            enemy_spawns: Vec::new(),
            buried_item_spawns: Vec::new(),
            tiles: vec![Tile::Wall; (FLOOR_WIDTH * FLOOR_HEIGHT) as usize],
            player_spawn: Pos::new(0, 0),
            stairs: Pos::new(0, 0),
            shop: None,
            monster_house: None,
            fallback: false,
            hallway_sight: 2,
        }
    }

    pub(crate) fn in_shop(&self, p: Pos) -> bool {
        self.shop.is_some_and(|shop| shop.contains(p))
    }

    /// Whether the tile is in the floor's Monster House.
    pub(crate) fn in_monster_house(&self, p: Pos) -> bool {
        self.monster_house.is_some() && self.tile(p).room() == self.monster_house
    }

    /// A room tile right next to a hallway tile.
    pub(crate) fn is_junction(&self, p: Pos) -> bool {
        self.tile(p).room().is_some()
            && [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().any(|(dx, dy)| self.tile(Pos::new(p.x + dx, p.y + dy)) == Tile::Floor { room: None })
    }

    /// Whether stairs, items or the player may be placed here: open room
    /// terrain outside the Kecleon shop that is not a hallway junction.
    pub(crate) fn is_spawn_tile(&self, p: Pos) -> bool {
        matches!(self.tile(p), Tile::Floor { room: Some(_) }) && !self.in_shop(p) && !self.is_junction(p)
    }

    /// Out-of-bounds positions read as walls.
    pub fn tile(&self, p: Pos) -> Tile {
        if Self::in_bounds(p) { self.tiles[(p.y * FLOOR_WIDTH + p.x) as usize] } else { Tile::Wall }
    }

    pub fn set(&mut self, p: Pos, tile: Tile) {
        if Self::in_bounds(p) {
            self.tiles[(p.y * FLOOR_WIDTH + p.x) as usize] = tile;
        }
    }

    pub(crate) fn in_bounds(p: Pos) -> bool {
        (0..FLOOR_WIDTH).contains(&p.x) && (0..FLOOR_HEIGHT).contains(&p.y)
    }

    /// Whether a monster of this mobility can stand on a tile. Nothing
    /// goes into the outer border of the map.
    pub(crate) fn can_enter(&self, p: Pos, mobility: Mobility) -> bool {
        let inside = (1..FLOOR_WIDTH - 1).contains(&p.x) && (1..FLOOR_HEIGHT - 1).contains(&p.y);
        match self.tile(p) {
            Tile::Wall => inside && mobility == Mobility::Intangible,
            Tile::Secondary => mobility != Mobility::Normal,
            Tile::Floor { .. } | Tile::Stairs { .. } => true,
        }
    }

    /// [`Self::can_step`] for a monster of the given mobility. Only what
    /// goes through walls cuts across their corners.
    pub(crate) fn can_step_as(&self, from: Pos, dx: i32, dy: i32, mobility: Mobility) -> bool {
        if !self.can_enter(Pos::new(from.x + dx, from.y + dy), mobility) {
            return false;
        }
        if dx != 0 && dy != 0 && mobility != Mobility::Intangible {
            return self.tile(Pos::new(from.x + dx, from.y)) != Tile::Wall && self.tile(Pos::new(from.x, from.y + dy)) != Tile::Wall;
        }
        true
    }

    /// Whether a monster may step from `from` by `(dx, dy)` (each in -1..=1).
    /// Diagonal moves cannot cut across wall corners (secondary terrain
    /// corners do not block them).
    pub(crate) fn can_step(&self, from: Pos, dx: i32, dy: i32) -> bool {
        if !self.tile(Pos::new(from.x + dx, from.y + dy)).is_walkable() {
            return false;
        }
        if dx != 0 && dy != 0 {
            return self.tile(Pos::new(from.x + dx, from.y)) != Tile::Wall && self.tile(Pos::new(from.x, from.y + dy)) != Tile::Wall;
        }
        true
    }

    pub fn positions() -> impl Iterator<Item = Pos> {
        (0..FLOOR_HEIGHT).flat_map(|y| (0..FLOOR_WIDTH).map(move |x| Pos::new(x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobility_decides_what_can_be_crossed() {
        let mut floor = Floor::new_walled();
        for x in 2..6 {
            floor.set(Pos::new(x, 5), Tile::Floor { room: Some(0) });
        }
        floor.set(Pos::new(6, 5), Tile::Secondary);
        let from = Pos::new(5, 5);
        // Secondary terrain: not for ordinary walkers.
        assert!(!floor.can_step(from, 1, 0));
        assert!(!floor.can_step_as(from, 1, 0, Mobility::Normal));
        assert!(floor.can_step_as(from, 1, 0, Mobility::Secondary));
        assert!(floor.can_step_as(from, 1, 0, Mobility::Hovering));
        // Walls: only for what goes through them, corners included, but
        // never the map's border.
        assert!(!floor.can_step_as(from, 0, 1, Mobility::Hovering));
        assert!(floor.can_step_as(from, 0, 1, Mobility::Intangible));
        assert!(floor.can_step_as(from, -1, 1, Mobility::Intangible));
        assert!(!floor.can_step_as(Pos::new(1, 1), -1, 0, Mobility::Intangible));
        assert!(!floor.can_step_as(Pos::new(1, 1), 0, -1, Mobility::Intangible));
        // The ROM's movement types: 2 hovers, 3 goes through walls, 4
        // crosses lava and 5 water, on floors that have them.
        assert_eq!(Mobility::resolve(0, 0), Mobility::Normal);
        assert_eq!(Mobility::resolve(2, 2), Mobility::Hovering);
        assert_eq!(Mobility::resolve(3, 0), Mobility::Intangible);
        assert_eq!(Mobility::resolve(5, 0), Mobility::Secondary);
        assert_eq!(Mobility::resolve(5, 1), Mobility::Normal);
        assert_eq!(Mobility::resolve(4, 1), Mobility::Secondary);
        assert_eq!(Mobility::resolve(4, 2), Mobility::Normal);
    }
}
