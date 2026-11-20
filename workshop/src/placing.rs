//! Laying things on the Scene, without drawing anything: where a piece
//! lands under the pointer (on the pixel, or on the grid when snapping is
//! on, Alt turning it the other way for a moment), how far the arrows nudge,
//! the stamp (a piece laid at each click, again and again, until Esc), the
//! layers a piece goes through. The Scene (`scene/`) and the Library
//! (`library/`) draw and ask; what is decided is here, and tested.

/// A point of the decor on the pixel, or on the grid of that step.
pub fn snap(point: [f32; 2], step: f32, on: bool) -> [f32; 2] {
    if on && step >= 1.0 { [(point[0] / step).round() * step, (point[1] / step).round() * step] } else { [point[0].round(), point[1].round()] }
}

/// Whether snapping holds now: as asked, Alt held turning it the other way.
pub fn snapping(asked: bool, alt: bool) -> bool {
    asked != alt
}

/// Where a piece of that size lands, its middle under the pointer: its top left corner.
pub fn landing(pointer: [f32; 2], size: [f32; 2], step: f32, on: bool) -> [f32; 2] {
    snap([pointer[0] - size[0] / 2.0, pointer[1] - size[1] / 2.0], step, on)
}

/// How far an arrow nudges what is chosen: a pixel, eight with Shift; `way` is −1, 0 or 1 across and down.
pub fn nudge(way: [i8; 2], big: bool) -> [f32; 2] {
    let step = if big { 8.0 } else { 1.0 };
    [f32::from(way[0]) * step, f32::from(way[1]) * step]
}

/// The layers a piece is laid on, from under to over: each its word, its name, what it means.
pub const PIECE_LAYERS: [(&str, &str, &str); 3] = [
    ("ground", "Ground", "under everything, under the characters too"),
    ("objects", "Objects", "as in the game: its bottom under the characters, its top in front of them"),
    ("over", "Over", "in front of everything, the characters too"),
];

/// The layer after this one (L): ground, objects, over, then ground again.
pub fn next_layer(layer: &str) -> &'static str {
    let at = PIECE_LAYERS.iter().position(|(word, ..)| *word == layer).unwrap_or(1);
    PIECE_LAYERS[(at + 1) % PIECE_LAYERS.len()].0
}

/// The name a layer is shown by.
pub fn layer_name(layer: &str) -> &'static str {
    PIECE_LAYERS.iter().find(|(word, ..)| *word == layer).map_or("Objects", |(_, name, _)| name)
}

/// The stamp: a piece of the Library laid at each click on the scene, until
/// Esc (or its card clicked again); how many it laid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub name: String,
    pub laid: usize,
}

/// The stamp once a card is clicked: that piece's (none if it was that one's already: stopped).
pub fn stamp_clicked(stamp: Option<&Stamp>, name: &str) -> Option<Stamp> {
    match stamp {
        Some(known) if known.name == name => None,
        _ => Some(Stamp { name: name.to_string(), laid: 0 }),
    }
}

/// The stamp once it laid one more.
pub fn stamped(stamp: &Stamp) -> Stamp {
    Stamp { name: stamp.name.clone(), laid: stamp.laid + 1 }
}

#[cfg(test)]
mod tests {
    use super::{Stamp, landing, layer_name, next_layer, nudge, snap, snapping, stamp_clicked, stamped};

    /// On the pixel, or on the grid's step; Alt turns snapping the other way.
    #[test]
    fn points_snap_to_the_pixel_or_the_grid() {
        assert_eq!(snap([13.4, 4.6], 16.0, false), [13.0, 5.0]);
        assert_eq!(snap([13.4, 4.6], 16.0, true), [16.0, 0.0]);
        assert_eq!(snap([13.4, 4.6], 8.0, true), [16.0, 8.0]);
        assert_eq!(snap([13.4, 4.6], 0.0, true), [13.0, 5.0], "no step: the pixel");
        assert!(snapping(true, false) && !snapping(true, true) && snapping(false, true) && !snapping(false, false));
    }

    /// The ghost under the pointer is where the piece lands: its middle there, its corner snapped.
    #[test]
    fn a_piece_lands_its_middle_under_the_pointer() {
        assert_eq!(landing([100.0, 50.0], [32.0, 20.0], 8.0, false), [84.0, 40.0]);
        assert_eq!(landing([100.0, 50.0], [30.0, 20.0], 8.0, true), [88.0, 40.0]);
        assert_eq!(landing([5.0, 5.0], [24.0, 24.0], 16.0, true), [0.0, 0.0]);
    }

    /// The arrows: a pixel, Shift eight; the layers in turn.
    #[test]
    fn nudges_and_layers() {
        assert_eq!(nudge([-1, 0], false), [-1.0, 0.0]);
        assert_eq!(nudge([0, 1], true), [0.0, 8.0]);
        assert_eq!(next_layer("ground"), "objects");
        assert_eq!(next_layer("objects"), "over");
        assert_eq!(next_layer("over"), "ground");
        assert_eq!(next_layer("residents"), "over", "unknown: as objects");
        assert_eq!(layer_name("over"), "Over");
    }

    /// A card clicked arms the stamp; clicked again, it stops; another one takes its place.
    #[test]
    fn the_stamp_is_armed_counted_and_stopped() {
        let armed = stamp_clicked(None, "e1_1").expect("armed");
        assert_eq!(armed, Stamp { name: "e1_1".to_string(), laid: 0 });
        let twice = stamped(&stamped(&armed));
        assert_eq!(twice.laid, 2);
        assert_eq!(stamp_clicked(Some(&twice), "e1_1"), None, "the same card: stopped");
        assert_eq!(stamp_clicked(Some(&twice), "e2_1").map(|stamp| stamp.name), Some("e2_1".to_string()));
    }
}
