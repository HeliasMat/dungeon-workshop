//! What goes in the windows: the cursor, the top line. (Lists, titled menus
//! and the other widgets: `widgets`.)

use super::*;

impl Canvas {
    /// The yellow arrow pointing at a menu entry whose text starts at
    /// line `y`: the game's cursor, a column right of `x`.
    pub fn cursor(&mut self, art: &UiArt, x: i32, y: i32) {
        self.cursors.push((x, y));
        if self.cursor_hidden && self.cursors_before.contains(&(x, y)) {
            return;
        }
        let (width, height, rgba) = &art.cursor;
        self.blit(x + 1, y, *width, *height, rgba);
    }

    /// A cursor is somewhere none was on the picture before: its blinking
    /// starts again.
    pub fn cursor_moved(&self) -> bool {
        self.cursors.iter().any(|place| !self.cursors_before.contains(place))
    }

    /// One 8x8 tile of the top line's font.
    fn hud_tile(&mut self, art: &UiArt, tile: usize, x: i32, y: i32) {
        const WIDTH: usize = 128;
        for dy in 0..TILE {
            for dx in 0..TILE {
                let at = ((tile / 16 * TILE + dy) * WIDTH + tile % 16 * TILE + dx) * 4;
                if let Some(color) = pixel_at(&art.hud, at)
                    && color[3] != 0
                {
                    self.put(x + dx as i32, y + dy as i32, color);
                }
            }
        }
    }

    /// The top line of the touch screen, tile for tile as the game lays it
    /// out (matched against its screen): floor, level, HP and the HP bar,
    /// one pixel per HP. `top` is the line's first pixel row.
    // (As `DisplayUi` runs: the floor and the level one after the other, the
    // HP from a fixed place.)
    pub fn hud(&mut self, art: &UiArt, top: i32, floor: u32, goes_up: bool, level: i32, hp: i32, max_hp: i32) {
        // Tiles: digits 0-9, then "floor", "level", ".", and on the second
        // row "P", "V", "/", "-" and the bar's red and green.
        const FLOOR: usize = 10;
        const LEVEL: usize = 11;
        const DOT: usize = 12;
        const FRAME: usize = 14;
        const P: usize = 26;
        const V: usize = 27;
        const SLASH: usize = 28;
        const MINUS: usize = 29;
        let digits = |value: i32| -> Vec<usize> { value.max(0).to_string().bytes().map(|digit| (digit - b'0') as usize).collect() };
        let mut x = 8;
        self.hud_tile(art, FLOOR, x, top);
        x += 8;
        if !goes_up {
            self.hud_tile(art, MINUS, x, top);
        }
        x += 8;
        for digit in digits(floor as i32) {
            self.hud_tile(art, digit, x, top);
            x += 8;
        }
        self.hud_tile(art, LEVEL, x, top);
        self.hud_tile(art, DOT, x + 8, top);
        x += 12;
        for digit in digits(level) {
            self.hud_tile(art, digit, x, top);
            x += 8;
        }
        // HP: then the slash, then the maximum.
        let hp_digits = digits(hp);
        let label = 72;
        self.hud_tile(art, P, label, top);
        self.hud_tile(art, V, label + 8, top);
        let slash = label + 16 + 8 * hp_digits.len() as i32;
        for (index, digit) in hp_digits.into_iter().enumerate() {
            self.hud_tile(art, digit, label + 16 + 8 * index as i32, top);
        }
        self.hud_tile(art, SLASH, slash, top);
        for (index, digit) in digits(max_hp).into_iter().enumerate() {
            self.hud_tile(art, digit, slash + 8 + 8 * index as i32, top);
        }
        // The bar: the frame tile repeats, one column per HP.
        let length = max_hp.clamp(1, 104);
        let filled = length * hp.clamp(0, max_hp) / max_hp.max(1);
        let block = |tile: usize| {
            let at = ((TILE + 3) * 128 + tile % 16 * TILE + 3) * 4;
            pixel_at(&art.hud, at).unwrap_or([0, 0, 0, 255])
        };
        let (red, green) = (block(30), block(31));
        for column in 0..length {
            let at_x = 144 + column;
            for dy in 0..TILE {
                let at = ((FRAME / 16 * TILE + dy) * 128 + FRAME % 16 * TILE) * 4;
                if let Some(color) = pixel_at(&art.hud, at)
                    && color[3] != 0
                {
                    self.put(at_x, top + dy as i32, color);
                }
            }
            let color = if column < filled { green } else { red };
            self.fill(at_x, top + 2, 1, 4, color);
        }
    }
}
