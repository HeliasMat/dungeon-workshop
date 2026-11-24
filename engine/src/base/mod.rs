//! The values a beat says and the stage keeps, knowing neither: a beat's
//! bare numbers (`units`: a way to face, a time, the game's ids), how a
//! value goes from a key to the next (`ease`), the ground of a place
//! (`ground`), the game's colours (`colour`), a box, and what a node takes
//! from the one it hangs on.

use glam::Vec2;

mod colour;
mod ease;
mod ground;
mod units;

pub use colour::{Colour, colour_named};
pub use ease::{Ease, EaseFamily};
pub use ground::{Ground, TILE};
pub use units::{AnimId, Direction, EmoteId, Frames, MoveId, SongId, SoundId};

/// A box, its left under its right, its top under its bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    /// The box between two corners, in whatever order they come.
    pub fn new(a: Vec2, b: Vec2) -> Self {
        Rect { left: a.x.min(b.x), top: a.y.min(b.y), right: a.x.max(b.x), bottom: a.y.max(b.y) }
    }

    /// Whether a point is in it, or that near.
    pub fn holds(&self, at: Vec2, margin: f32) -> bool {
        at.x > self.left - margin && at.x < self.right + margin && at.y > self.top - margin && at.y < self.bottom + margin
    }

    /// The nearest point in it.
    pub fn clamp(&self, at: Vec2) -> Vec2 {
        at.clamp(Vec2::new(self.left, self.top), Vec2::new(self.right, self.bottom))
    }
}

impl From<[f32; 4]> for Rect {
    fn from([left, top, right, bottom]: [f32; 4]) -> Self {
        Rect::new(Vec2::new(left, top), Vec2::new(right, bottom))
    }
}

/// What a node takes from the one it hangs on (see `Node::parent`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inherit {
    pub pos: bool,
    pub angle: bool,
    pub scale: bool,
    pub opacity: bool,
    pub colour: bool,
}

impl Default for Inherit {
    /// All of it.
    fn default() -> Self {
        Inherit { pos: true, angle: true, scale: true, opacity: true, colour: true }
    }
}

impl Inherit {
    /// Nothing of it.
    pub const NONE: Inherit = Inherit { pos: false, angle: false, scale: false, opacity: false, colour: false };

    /// What it takes, by the words a line says (`pos angle scale opacity
    /// colour`): only those; none if a word is none of them.
    pub fn of_words(words: &[String]) -> Option<Inherit> {
        let mut inherit = Inherit::NONE;
        for word in words {
            match word.as_str() {
                "pos" | "position" => inherit.pos = true,
                "angle" => inherit.angle = true,
                "scale" => inherit.scale = true,
                "opacity" => inherit.opacity = true,
                "colour" | "tint" => inherit.colour = true,
                _ => return None,
            }
        }
        Some(inherit)
    }

    /// The words of what it takes, as the engine's lines write them.
    pub fn words(self) -> Vec<&'static str> {
        [(self.pos, "pos"), (self.angle, "angle"), (self.scale, "scale"), (self.opacity, "opacity"), (self.colour, "colour")]
            .into_iter()
            .filter_map(|(taken, word)| taken.then_some(word))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_holds_what_is_in_it() {
        let box_of = Rect::from([10.0, 10.0, 0.0, 0.0]);
        assert_eq!(box_of.left, 0.0);
        assert!(box_of.holds(Vec2::new(5.0, 5.0), 0.0) && !box_of.holds(Vec2::new(12.0, 5.0), 0.0) && box_of.holds(Vec2::new(12.0, 5.0), 3.0));
        assert_eq!(box_of.clamp(Vec2::new(20.0, 5.0)), Vec2::new(10.0, 5.0));
    }
}
