//! A character of a film's own drawn from a sprite sheet ([`sliced`]):
//! cells of a size side by side, a row for each way it faces (1, 4 or 8),
//! its animations by their cells along a row. What it makes is a
//! [`Sprite`] like a monster's: it walks, stands, faces, plays its own.

use crate::images::{Frame, Sprite};

/// An animation of a sheet: which of the sprite's animations it is (0
/// walk, 1 idle, 2 hurt, 3 sleep, 4 attack, 5 + n its own), its cells
/// along a row, the 60ths of a second of each (empty: 8; fewer: the last
/// kept), the row of its first way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cells {
    pub animation: usize,
    pub cells: Vec<u16>,
    pub ticks: Vec<u32>,
    pub row: u16,
}

/// The pace of a cell with none told.
const TICKS: u32 = 8;

/// The row of a sheet, past its animation's first, a way faced shows (the
/// ways from facing down, turning towards the right: down, down-right,
/// right, up-right, up, up-left, left, down-left). Four rows: down, left,
/// right, up (a slant shows the side it leans to). One: the same for all.
pub fn row_of(way: usize, ways: u8) -> u16 {
    match ways {
        8 => (way % 8) as u16,
        4 => [0, 2, 2, 2, 3, 1, 1, 1][way % 8],
        _ => 0,
    }
}

/// The sprite of a sheet (its pixels, RGBA, so wide and high), its cells
/// so wide and high, facing so many ways, with these animations. Each
/// stands on the lowest row drawn among its cells. A cell past the sheet
/// is clear. With no way of standing told, it stands on the first cell
/// of its first animation; with no walk, it walks as it stands.
pub fn sliced(rgba: &[u8], (wide, high): (usize, usize), (cell_wide, cell_high): (usize, usize), ways: u8, anims: &[Cells]) -> Sprite {
    let pixel = |x: usize, y: usize| -> [u8; 4] {
        if x >= wide || y >= high || rgba.len() < (y * wide + x + 1) * 4 {
            return [0; 4];
        }
        let at = (y * wide + x) * 4;
        [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
    };
    let cut = |column: u16, row: u16| -> Vec<u8> {
        let (left, top) = (column as usize * cell_wide, row as usize * cell_high);
        (0..cell_high).flat_map(|y| (0..cell_wide).flat_map(move |x| pixel(left + x, top + y))).collect()
    };
    // (Where it stands: the lowest row drawn among all its cells.)
    let lowest = anims
        .iter()
        .flat_map(|anim| anim.cells.iter().flat_map(move |cell| (0..ways.max(1) as u16).map(move |way| (*cell, anim.row + way))))
        .filter_map(|(column, row)| {
            let cell = cut(column, row);
            (0..cell_high).rev().find(|y| (0..cell_wide).any(|x| cell[(y * cell_wide + x) * 4 + 3] > 0))
        })
        .max()
        .unwrap_or(cell_high.saturating_sub(1));
    let origin = (cell_wide as i32 / 2, lowest as i32 - 1);
    let steps = |anim: &Cells, way: usize| -> Vec<Frame> {
        let row = anim.row + row_of(way, ways);
        anim.cells
            .iter()
            .enumerate()
            .map(|(at, column)| {
                let mut frame = Frame::still(cell_wide, cell_high, origin, cut(*column, row));
                frame.ticks = anim.ticks.get(at).or(anim.ticks.last()).copied().unwrap_or(TICKS).max(1);
                frame
            })
            .collect()
    };
    let count = anims.iter().map(|anim| anim.animation + 1).max().unwrap_or(0).max(2);
    let mut animations: Vec<Vec<Vec<Frame>>> = vec![Vec::new(); count];
    for anim in anims.iter().filter(|anim| !anim.cells.is_empty()) {
        animations[anim.animation] = (0..8).map(|way| steps(anim, way)).collect();
    }
    if animations[1].is_empty()
        && let Some(first) = anims.iter().find(|anim| !anim.cells.is_empty())
    {
        let standing = Cells { animation: 1, cells: vec![first.cells[0]], ticks: vec![60], row: first.row };
        animations[1] = (0..8).map(|way| steps(&standing, way)).collect();
    }
    if animations[0].is_empty() {
        animations[0] = animations[1].clone();
    }
    Sprite::whole(animations)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet of 3 cells of 2x2 a row, 4 rows: each cell's top left pixel
    /// its column (red) and its row (green); its lowest row drawn on the
    /// first column only.
    fn sheet() -> Vec<u8> {
        let (wide, high) = (6, 8);
        let mut rgba = vec![0u8; wide * high * 4];
        for row in 0..4 {
            for column in 0..3 {
                let at = (row * 2 * wide + column * 2) * 4;
                rgba[at..at + 4].copy_from_slice(&[column as u8, row as u8, 0, 255]);
                if column == 0 {
                    let below = ((row * 2 + 1) * wide) * 4;
                    rgba[below..below + 4].copy_from_slice(&[9, 9, 9, 255]);
                }
            }
        }
        rgba
    }

    fn mark(frame: &Frame) -> (u8, u8) {
        (frame.rgba[0], frame.rgba[1])
    }

    #[test]
    fn a_sheet_is_cut_into_its_animations_and_ways() {
        let walk = Cells { animation: 0, cells: vec![0, 1, 2], ticks: vec![6, 4], row: 0 };
        let own = Cells { animation: 5, cells: vec![2], ticks: Vec::new(), row: 0 };
        let sprite = sliced(&sheet(), (6, 8), (2, 2), 4, &[walk, own]);
        let down = &sprite.ways(0)[0];
        assert_eq!(down.iter().map(mark).collect::<Vec<_>>(), [(0, 0), (1, 0), (2, 0)]);
        assert_eq!(down.iter().map(|frame| frame.ticks).collect::<Vec<_>>(), [6, 4, 4], "the last pace kept");
        // (Four rows: down, left, right, up; a slant its side.)
        let row = |way: usize| mark(&sprite.ways(0)[way][0]).1;
        assert_eq!([row(0), row(6), row(2), row(4), row(1), row(5)], [0, 1, 2, 3, 2, 1]);
        // (Standing, untold: the first cell of the walk; its own, by its number.)
        assert_eq!(mark(&sprite.ways(1)[0][0]), (0, 0));
        assert!(sprite.has(5) && !sprite.has(2));
        assert_eq!(sprite.ways(5)[3][0].ticks, 8);
        // (It stands on its lowest drawn row.)
        assert_eq!(down[0].origin, (1, 0));
    }

    #[test]
    fn one_row_for_all_ways_and_cells_past_the_sheet_are_clear() {
        let idle = Cells { animation: 1, cells: vec![1, 7], ticks: vec![10], row: 2 };
        let sprite = sliced(&sheet(), (6, 8), (2, 2), 1, &[idle]);
        for way in 0..8 {
            assert_eq!(mark(&sprite.ways(1)[way][0]), (1, 2));
            assert_eq!(sprite.ways(1)[way][1].rgba, vec![0; 16]);
        }
        // (No walk: it walks as it stands.)
        assert_eq!(mark(&sprite.ways(0)[0][0]), (1, 2));
        assert_eq!((row_of(3, 8), row_of(3, 1)), (3, 0));
    }
}
