//! The game's text font: letters of bits, drawn to pictures.

use std::collections::HashMap;

/// Glyphs are 12x12 pixels.
pub const GLYPH_SIZE: usize = 12;
/// Distance between two lines of text.
pub const LINE_HEIGHT: usize = 13;
/// Width of a character the font lacks.
const BLANK_WIDTH: usize = 4;
/// The game's own line pitch in text windows (measured on its screen).
pub const TEXT_PITCH: usize = 13;

struct Glyph {
    width: usize,
    /// One bit per pixel, lowest bit leftmost.
    rows: [u16; GLYPH_SIZE],
}

#[derive(Default)]
pub struct Font {
    /// By table and character code (`table << 8 | code`).
    glyphs: HashMap<u16, Glyph>,
}

impl Font {
    /// The font as a file of the project's own, and back.
    pub fn pack(&self, packer: &mut crate::pack::Packer) {
        packer.number(self.glyphs.len());
        // (By code, so that the same font packs to the same bytes.)
        let mut glyphs: Vec<_> = self.glyphs.iter().collect();
        glyphs.sort_by_key(|(code, _)| **code);
        for (code, glyph) in glyphs {
            packer.number(*code as usize);
            packer.number(glyph.width);
            glyph.rows.iter().for_each(|row| packer.number(*row as usize));
        }
    }

    pub fn unpack(unpacker: &mut crate::pack::Unpacker) -> Option<Self> {
        let mut glyphs = HashMap::new();
        for _ in 0..unpacker.number()? {
            let (code, width) = (unpacker.number()? as u16, unpacker.number()?);
            let mut rows = [0; GLYPH_SIZE];
            for row in &mut rows {
                *row = unpacker.number()? as u16;
            }
            glyphs.insert(code, Glyph { width, rows });
        }
        Some(Self { glyphs })
    }

    /// A letter: its code, its width, its rows.
    pub fn add(&mut self, code: u16, width: usize, rows: [u16; GLYPH_SIZE]) {
        self.glyphs.insert(code, Glyph { width, rows });
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(&encode(c)?)
    }

    fn line_width(&self, line: &str) -> usize {
        line.chars().map(|c| self.glyph(c).map_or(BLANK_WIDTH, |glyph| glyph.width)).sum()
    }

    /// Draws `text` (lines split by `\n`) in white with the dark shadow the
    /// game gives its text. Returns `(width, height, RGBA pixels)`.
    pub fn render(&self, text: &str) -> (usize, usize, Vec<u8>) {
        self.render_colored(text, [255, 255, 255])
    }

    /// Width in pixels of one line of text.
    pub fn width(&self, line: &str) -> usize {
        self.line_width(line)
    }

    /// [`Self::render`] in another colour; the shadow stays black, as in
    /// the game.
    pub fn render_colored(&self, text: &str, color: [u8; 3]) -> (usize, usize, Vec<u8>) {
        self.draw(text, color, true)
    }

    /// The same without its shadow: for text on something light, where a
    /// shadow reads as the text written twice.
    pub fn render_plain(&self, text: &str, color: [u8; 3]) -> (usize, usize, Vec<u8>) {
        self.draw(text, color, false)
    }

    fn draw(&self, text: &str, color: [u8; 3], shadowed: bool) -> (usize, usize, Vec<u8>) {
        let ink = [color[0], color[1], color[2], 255];
        const SHADOW: [u8; 4] = [0, 0, 0, 255];
        let lines: Vec<&str> = text.split('\n').collect();
        // One more pixel each way for the shadow.
        let width = lines.iter().map(|line| self.line_width(line)).max().unwrap_or(0) + 1;
        let height = lines.len() * LINE_HEIGHT + 1;
        let mut data = vec![0u8; width * height * 4];
        let mut put = |x: usize, y: usize, color: [u8; 4], over: bool| {
            let at = (y * width + x) * 4;
            if over || data[at + 3] == 0 {
                data[at..at + 4].copy_from_slice(&color);
            }
        };
        for (number, line) in lines.iter().enumerate() {
            let (mut x, top) = (0, number * LINE_HEIGHT);
            for c in line.chars() {
                let Some(glyph) = self.glyph(c) else {
                    x += BLANK_WIDTH;
                    continue;
                };
                for (row, bits) in glyph.rows.iter().enumerate() {
                    for column in 0..GLYPH_SIZE.min(width - 1 - x) {
                        if bits >> column & 1 != 0 {
                            put(x + column, top + row, ink, true);
                            if shadowed {
                                put(x + column + 1, top + row + 1, SHADOW, false);
                            }
                        }
                    }
                }
                x += glyph.width;
            }
        }
        (width, height, data)
    }
}

/// The two-byte letters of table 0x81, with their first byte.
pub const WIDE: [(u8, char); 7] = [(0x40, '\u{3000}'), (0x60, '～'), (0xA8, '→'), (0xA9, '←'), (0xAA, '↑'), (0xAB, '↓'), (0xF4, '♪')];

/// The game's code of a character, with its table.
fn encode(c: char) -> Option<u16> {
    if let Some(&(code, _)) = WIDE.iter().find(|&&(_, wide)| wide == c) {
        return Some(0x8100 | code as u16);
    }
    match c {
        '…' => Some(0x85),
        'Œ' => Some(0x8C),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        'œ' => Some(0x9C),
        '—' | '–' => Some(b'-' as u16),
        c => u8::try_from(c as u32).ok().map(u16::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A font with an "A" three pixels wide: a full first row.
    fn tiny() -> Font {
        let mut font = Font::default();
        let mut rows = [0; GLYPH_SIZE];
        rows[0] = 0b111;
        font.add(b'A' as u16, 3, rows);
        font
    }

    #[test]
    fn text_is_drawn_with_its_shadow() {
        let font = tiny();
        let (width, height, data) = font.render("AA\nA");
        assert_eq!((width, height), (7, 2 * LINE_HEIGHT + 1));
        let pixel = |x: usize, y: usize| data[(y * width + x) * 4..][..4].to_vec();
        // Two glyphs side by side on the first line, one on the second.
        assert_eq!(pixel(0, 0), vec![255; 4]);
        assert_eq!(pixel(5, 0), vec![255; 4]);
        assert_eq!(pixel(2, LINE_HEIGHT), vec![255; 4]);
        assert_eq!(pixel(4, LINE_HEIGHT)[3], 0);
        // The shadow, one pixel down and right, never over the text.
        assert_eq!(pixel(6, 1), vec![0, 0, 0, 255]);
        assert_eq!(pixel(1, 1), vec![0, 0, 0, 255]);
        // Unknown characters leave a gap.
        assert_eq!(font.render("?A").0, BLANK_WIDTH + 3 + 1);
    }
}
