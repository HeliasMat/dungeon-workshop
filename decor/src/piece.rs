//! A piece as it is laid (`Cut`): its layers of every frame, its palette
//! colours, its walls, its shade, where it stands. `cut` cuts them from the
//! game's places, `terrain` paints them, `build` lays them.

use crate::matte::Shade;
use crate::pixels::{Grid, Mask, Picture};

/// A piece as it is laid: its lower and upper layers, the palette colour of
/// each pixel where its colours change with time, its level (-1: one of the
/// game's things), which pixels are walls, its shade on the lower layer.
#[derive(Clone, Debug)]
pub struct Cut {
    pub lower: Vec<Picture>,
    pub upper: Vec<Picture>,
    pub solid: Mask,
    pub level: i64,
    pub lower_index: Option<Vec<Grid<u8>>>,
    pub upper_index: Option<Vec<Grid<u8>>>,
    pub shade: Shade,
    /// Where it stands, from its top left.
    pub foot: (i64, i64),
    pub walked: bool,
    pub seconds: f64,
}

impl Cut {
    /// Its width and height.
    pub fn size(&self) -> (usize, usize) {
        (self.lower[0].w, self.lower[0].h)
    }

    /// Opaque on either layer, any frame.
    pub fn seen(&self) -> Mask {
        let (w, h) = self.size();
        let mut seen = Mask::new(w, h);
        for frame in self.lower.iter().chain(&self.upper) {
            for (cell, rgba) in seen.cells.iter_mut().zip(&frame.cells) {
                *cell |= rgba[3] > 0;
            }
        }
        seen
    }

    /// Turned over left to right.
    pub fn mirrored(&self) -> Cut {
        let flip = |frames: &Vec<Picture>| frames.iter().map(Grid::mirrored).collect();
        let flip_index = |frames: &Option<Vec<Grid<u8>>>| frames.as_ref().map(|all| all.iter().map(Grid::mirrored).collect());
        Cut {
            lower: flip(&self.lower),
            upper: flip(&self.upper),
            solid: self.solid.mirrored(),
            lower_index: flip_index(&self.lower_index),
            upper_index: flip_index(&self.upper_index),
            shade: self.shade.mirrored(),
            foot: (self.size().0 as i64 - 1 - self.foot.0, self.foot.1),
            ..self.clone()
        }
    }

    /// Living that way, at that pace (`anim`): its frames made anew, room
    /// kept around it if it moves (its corner `animate::MARGIN` further up
    /// and left), its colours no longer turned by the game's palette.
    pub fn living(&self, kind: &str, pace: Option<&str>) -> Cut {
        let room = if crate::animate::moves(kind) { crate::animate::MARGIN } else { 0 };
        let count = crate::animate::frames(pace);
        let grown = |frames: &Vec<Picture>| -> Vec<Picture> {
            let padded: Vec<Picture> = frames.iter().map(|frame| crate::animate::padded(frame, room)).collect();
            crate::animate::animated(&padded, kind, count)
        };
        Cut {
            lower: grown(&self.lower),
            upper: grown(&self.upper),
            solid: crate::animate::padded(&self.solid, room),
            lower_index: None,
            upper_index: None,
            shade: Shade {
                factor: crate::animate::padded(&self.shade.factor, room),
                colour: crate::animate::padded(&self.shade.colour, room),
                grounds: self.shade.grounds.clone(),
            },
            foot: (self.foot.0 + room as i64, self.foot.1 + room as i64),
            seconds: crate::animate::SECONDS,
            ..self.clone()
        }
    }

    pub fn unshaded(&self) -> Cut {
        let (w, h) = self.size();
        Cut { shade: Shade::none(w, h), ..self.clone() }
    }
}
