//! What every layout ends with.

use super::*;

impl Generator<'_> {
    /// Rows 1 and 30 return to plain walls.
    pub(super) fn reset_inner_boundary_rows(&mut self) {
        for x in 0..W {
            for y in [1, H - 2] {
                *self.tile(x, y) = RawTile::DEFAULT;
                if x == 0 || x == W - 1 {
                    self.tile(x, y).terrain |= IMPASSABLE;
                }
            }
        }
    }

    pub(super) fn ensure_impassable_tiles_are_walls(&mut self) {
        for tile in &mut self.tiles {
            if tile.terrain & IMPASSABLE != 0 {
                tile.set_kind(0);
            }
        }
    }

    /// Flags room tiles next to hallways as junctions and keeps them open.
    pub(super) fn finalize_junctions(&mut self) {
        for x in 0..W {
            for y in 0..H {
                let here = self.at(x, y);
                if !here.is_open() {
                    continue;
                }
                if here.room == NO_ROOM {
                    let neighbours = [(x > 0, x - 1, y), (y > 0, x, y - 1), (y < H - 1, x, y + 1), (x < W - 1, x + 1, y)];
                    for (inside, tx, ty) in neighbours {
                        if inside && self.at(tx, ty).room != NO_ROOM {
                            let tile = self.tile(tx, ty);
                            tile.terrain |= JUNCTION;
                            if tile.kind() == SECONDARY {
                                tile.set_kind(OPEN);
                            }
                        }
                    }
                } else if here.room == ANCHOR {
                    self.tile(x, y).room = NO_ROOM;
                }
            }
        }
    }
}
