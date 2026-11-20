//! What the things a film names look like, small: a Pokémon's face, a place
//! whole, a thing of the game's or one drawn by us. For lists to choose from
//! (`scene_player items`, the Workshop's resources).

use shell::images::{Picture, picture};

use crate::art::Art;

/// Something a film names, by what shows it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    /// A Pokémon, by its number: its Normal face.
    Monster(u16),
    /// A place, by its level: all of it.
    Place(u16),
    /// A thing of the game's scenes, by its number: its first picture.
    Prop(u16),
    /// An item of the game's, by its number: its picture of the bag.
    Item(u16),
    /// A picture of ours, by its file (`torche.4.png`: the first of four).
    Drawing(String),
}

/// What it looks like, read from the game's files in `data`.
pub fn picture_of(art: &Art, data: &str, look: &Look) -> Option<Picture> {
    match look {
        Look::Monster(monster) => art.portrait(*monster, 0).map(|rgba| (40, 40, rgba)),
        Look::Place(level) => art.backdrop(*level),
        Look::Prop(prop) => {
            let row = assets::tables::sheets(data, "props.txt").into_iter().find(|row| row.number == *prop && row.animation == 0)?;
            let (wide, high) = (row.sheet.wide as usize, row.sheet.high as usize);
            Some(cropped(&picture(&shell::data::path(data, &format!("props/{prop}_0.png")))?, wide, high))
        }
        Look::Item(item) => art.tables.item_art.get(*item as usize).filter(|rgba| !rgba.is_empty()).map(|rgba| (16, 16, rgba.clone())),
        Look::Drawing(file) => {
            let whole = picture(&shell::art::ours(data, &art.project, file).to_string_lossy())?;
            let cells: usize = file.rsplit('.').nth(1).and_then(|cells| cells.parse().ok()).unwrap_or(1).max(1);
            let high = whole.1;
            Some(cropped(&whole, whole.0 / cells, high))
        }
    }
}

/// Its top left part, so wide and high (all of it, if it is smaller).
fn cropped((wide, high, rgba): &Picture, to_wide: usize, to_high: usize) -> Picture {
    let (to_wide, to_high) = (to_wide.min(*wide), to_high.min(*high));
    let rows = (0..to_high).flat_map(|y| rgba[y * wide * 4..(y * wide + to_wide) * 4].iter().copied());
    (to_wide, to_high, rows.collect())
}

/// A picture made to fit a square of `side`, pixels kept sharp: a large one
/// one pixel in so many, a small one twice or three times as large.
pub fn fitted((wide, high, rgba): &Picture, side: usize) -> Picture {
    let most = (*wide).max(*high).max(1);
    let (times, every) = if most > side { (1, most.div_ceil(side)) } else { ((side / most).clamp(1, 3), 1) };
    let (to_wide, to_high) = ((wide.div_ceil(every)) * times, (high.div_ceil(every)) * times);
    let mut out = Vec::with_capacity(to_wide * to_high * 4);
    for y in 0..to_high {
        for x in 0..to_wide {
            let at = ((y / times * every) * wide + x / times * every) * 4;
            out.extend_from_slice(&rgba[at..at + 4]);
        }
    }
    (to_wide, to_high, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_fits_its_square_sharp() {
        let small = (2, 1, vec![1, 1, 1, 255, 2, 2, 2, 255]);
        assert_eq!(fitted(&small, 6).0, 6, "three times as large");
        assert_eq!(&fitted(&small, 6).2[..12], &[1, 1, 1, 255, 1, 1, 1, 255, 1, 1, 1, 255]);
        let large = (100, 50, vec![7; 100 * 50 * 4]);
        let fit = fitted(&large, 48);
        assert!(fit.0 <= 48 && fit.1 <= 48 && fit.0 * fit.1 * 4 == fit.2.len());
        assert_eq!(cropped(&(3, 2, (0..24).collect()), 2, 1).2, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }
}
