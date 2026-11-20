//! Where one can walk, as far as a fight has to know: who runs up to its
//! target does not go through a wall, who is struck is not pushed into one.

use glam::Vec2;

/// The ground of a place, in tiles of 8 pixels (none where it is not known).
#[derive(Clone, Debug, Default)]
pub struct Ground {
    rows: Vec<Vec<bool>>,
}

const TILE: f32 = 8.0;

impl Ground {
    /// A place's, row by row: whether each tile is solid (`data/extra/ground.txt`,
    /// as `assets::tables::GroundRows::tiles` reads it).
    pub fn from_rows(rows: Vec<Vec<bool>>) -> Self {
        Ground { rows }
    }

    pub fn known(&self) -> bool {
        !self.rows.is_empty()
    }

    /// How wide it is, in pixels.
    pub fn wide(&self) -> Option<f32> {
        self.rows.first().map(|row| row.len() as f32 * TILE)
    }

    /// Whether that spot is no ground, or `margin` tiles around it.
    pub fn blocked(&self, at: Vec2, margin: i32) -> bool {
        let (x, y) = ((at.x / TILE).floor() as i32, (at.y / TILE).floor() as i32);
        let solid = |x: i32, y: i32| {
            let row = usize::try_from(y).ok().and_then(|y| self.rows.get(y));
            row.and_then(|row| row.get(usize::try_from(x).ok()?)).copied().unwrap_or(true)
        };
        (-margin..=margin).any(|dy| (-margin..=margin).any(|dx| solid(x + dx, y + dy)))
    }

    /// Whether nothing stands between two spots (known ground only).
    pub fn clear(&self, from: Vec2, to: Vec2) -> bool {
        if !self.known() {
            return true;
        }
        let steps = ((from.x - to.x).abs().max((from.y - to.y).abs()) / 4.0) as i32 + 1;
        (0..=steps).all(|step| !self.blocked(from.lerp(to, step as f32 / steps as f32), 0))
    }

    /// How far a blow can push someone: against a wall, less, or not at all.
    pub(crate) fn room_behind(&self, at: Vec2, by: Vec2, far: f32) -> f32 {
        if !self.known() {
            return far;
        }
        let way = (at - by) / at.distance(by).max(1.0);
        let fits = |much: &i32| self.clear(at, at + way * *much as f32);
        (0..=(far as i32 / 2)).map(|step| far as i32 - step * 2).find(|much| *much > 0 && fits(much)).unwrap_or(0) as f32
    }

    /// The free spot nearest to a point; the point where walls are not
    /// known.
    pub fn free(&self, at: Vec2) -> Vec2 {
        if !self.known() || !self.blocked(at, 1) {
            return at;
        }
        for reach in (8..=120).step_by(8) {
            let reach = reach as f32;
            let side = (0..=(reach as i32 / 4)).map(|step| -reach + step as f32 * 8.0);
            let spots = side
                .flat_map(|along| [at + Vec2::new(along, -reach), at + Vec2::new(along, reach), at + Vec2::new(-reach, along), at + Vec2::new(reach, along)]);
            let nearest = spots.filter(|spot| !self.blocked(*spot, 1)).min_by(|a, b| a.distance(at).total_cmp(&b.distance(at)));
            if let Some(spot) = nearest {
                return spot;
            }
        }
        at
    }
}
