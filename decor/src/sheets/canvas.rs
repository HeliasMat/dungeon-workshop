//! A sheet being drawn: colours, lines and boxes, pictures pasted, words in
//! small letters of our own (three pixels by five, drawn twice as large).

use std::path::Path;

use crate::fault::Diagnostic;
use crate::pixels::Picture;

pub type Rgb = [u8; 3];

/// A picture of colours, row after row.
#[derive(Clone, Debug)]
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Rgb>,
}

/// How tall the words are, how wide each letter goes.
pub const LETTER_HIGH: usize = 10;
const LETTER_WIDE: usize = 7;

impl Canvas {
    pub fn new(w: usize, h: usize, colour: Rgb) -> Canvas {
        Canvas { w, h, cells: vec![colour; w * h] }
    }

    /// What is seen of a picture: its colours, as they are.
    pub fn of(picture: &Picture) -> Canvas {
        Canvas { w: picture.w, h: picture.h, cells: picture.cells.iter().map(|[r, g, b, _]| [*r, *g, *b]).collect() }
    }

    pub fn get(&self, x: usize, y: usize) -> Rgb {
        self.cells[y * self.w + x]
    }

    /// A point, if it is on the sheet.
    pub fn dot(&mut self, x: i64, y: i64, colour: Rgb) {
        if (0..self.w as i64).contains(&x) && (0..self.h as i64).contains(&y) {
            self.cells[y as usize * self.w + x as usize] = colour;
        }
    }

    /// A box filled, its corners both in.
    pub fn fill(&mut self, (x0, y0, x1, y1): (i64, i64, i64, i64), colour: Rgb) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.dot(x, y, colour);
            }
        }
    }

    /// The outline of a box, its corners both in.
    pub fn frame(&mut self, (x0, y0, x1, y1): (i64, i64, i64, i64), colour: Rgb) {
        self.fill((x0, y0, x1, y0), colour);
        self.fill((x0, y1, x1, y1), colour);
        self.fill((x0, y0, x0, y1), colour);
        self.fill((x1, y0, x1, y1), colour);
    }

    /// Another sheet laid over this one, its top left there.
    pub fn paste(&mut self, other: &Canvas, x: i64, y: i64) {
        for row in 0..other.h {
            for column in 0..other.w {
                self.dot(x + column as i64, y + row as i64, other.get(column, row));
            }
        }
    }

    /// So many times as large, each pixel a square.
    pub fn scaled(&self, by: usize) -> Canvas {
        let mut out = Canvas::new(self.w * by, self.h * by, [0; 3]);
        for y in 0..out.h {
            for x in 0..out.w {
                out.cells[y * out.w + x] = self.get(x / by, y / by);
            }
        }
        out
    }

    /// Turned over left to right.
    pub fn mirrored(&self) -> Canvas {
        let mut out = self.clone();
        for row in out.cells.chunks_mut(self.w.max(1)) {
            row.reverse();
        }
        out
    }

    /// One pixel in so many each way: smaller.
    pub fn every(&self, by: usize) -> Canvas {
        let (w, h) = (self.w.div_ceil(by), self.h.div_ceil(by));
        let mut out = Canvas::new(w, h, [0; 3]);
        for y in 0..h {
            for x in 0..w {
                out.cells[y * w + x] = self.get(x * by, y * by);
            }
        }
        out
    }

    /// Its colours as red, green, blue and how much each is seen, all seen.
    pub fn rgba(&self) -> Vec<u8> {
        self.cells.iter().flat_map(|[r, g, b]| [*r, *g, *b, 255]).collect()
    }

    /// Words from there, in small letters; a letter we have none of, a space.
    pub fn text(&mut self, x: i64, y: i64, words: &str, colour: Rgb) {
        for (at, letter) in words.chars().enumerate() {
            let Some(rows) = letter_of(letter) else {
                continue;
            };
            let left = x + (at * LETTER_WIDE) as i64;
            for (row, bits) in rows.iter().enumerate() {
                for column in 0..3 {
                    if bits & (4 >> column) != 0 {
                        self.fill((left + column * 2, y + row as i64 * 2, left + column * 2 + 1, y + row as i64 * 2 + 1), colour);
                    }
                }
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Diagnostic> {
        let bytes: Vec<u8> = self.cells.iter().flatten().copied().collect();
        let picture =
            image::RgbImage::from_raw(self.w as u32, self.h as u32, bytes).ok_or_else(|| Diagnostic::said(format!("{}: wrong size", path.display())))?;
        picture.save(path).map_err(|why| Diagnostic::said(format!("{}: {why}", path.display())))
    }
}

/// How wide words are, in small letters.
pub fn text_wide(words: &str) -> usize {
    words.chars().count() * LETTER_WIDE
}

/// A letter's five rows, three bits each (the leftmost the highest); a
/// capital as its small one, an accent left off.
fn letter_of(letter: char) -> Option<[u8; 5]> {
    let letter = diagnostics::text::unaccented(letter.to_ascii_lowercase());
    Some(match letter {
        'a' => [2, 5, 7, 5, 5],
        'b' => [6, 5, 6, 5, 6],
        'c' => [3, 4, 4, 4, 3],
        'd' => [6, 5, 5, 5, 6],
        'e' => [7, 4, 6, 4, 7],
        'f' => [7, 4, 6, 4, 4],
        'g' => [3, 4, 5, 5, 3],
        'h' => [5, 5, 7, 5, 5],
        'i' => [7, 2, 2, 2, 7],
        'j' => [1, 1, 1, 5, 2],
        'k' => [5, 5, 6, 5, 5],
        'l' => [4, 4, 4, 4, 7],
        'm' => [5, 7, 7, 5, 5],
        'n' => [6, 5, 5, 5, 5],
        'o' => [2, 5, 5, 5, 2],
        'p' => [6, 5, 6, 4, 4],
        'q' => [2, 5, 5, 6, 3],
        'r' => [6, 5, 6, 5, 5],
        's' => [3, 4, 2, 1, 6],
        't' => [7, 2, 2, 2, 2],
        'u' => [5, 5, 5, 5, 7],
        'v' => [5, 5, 5, 5, 2],
        'w' => [5, 5, 7, 7, 5],
        'x' => [5, 5, 2, 5, 5],
        'y' => [5, 5, 2, 2, 2],
        'z' => [7, 1, 2, 4, 7],
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [6, 1, 2, 4, 7],
        '3' => [6, 1, 2, 1, 6],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 6, 1, 6],
        '6' => [3, 4, 7, 5, 7],
        '7' => [7, 1, 1, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 6],
        '_' => [0, 0, 0, 0, 7],
        '-' => [0, 0, 7, 0, 0],
        '+' => [0, 2, 7, 2, 0],
        '=' => [0, 7, 0, 7, 0],
        ',' => [0, 0, 0, 2, 4],
        '.' => [0, 0, 0, 0, 2],
        ':' => [0, 2, 0, 2, 0],
        ';' => [0, 2, 0, 2, 4],
        '/' => [1, 1, 2, 4, 4],
        '(' => [1, 2, 2, 2, 1],
        ')' => [4, 2, 2, 2, 4],
        '%' => [5, 1, 2, 4, 5],
        '?' => [6, 1, 2, 0, 2],
        '\'' => [2, 2, 0, 0, 0],
        '<' => [1, 2, 4, 2, 1],
        '>' => [4, 2, 1, 2, 4],
        _ => return None,
    })
}
