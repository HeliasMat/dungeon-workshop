//! The game's interface, drawn like the original's: the DS's two 256x192
//! screens, one above the other, on which windows (frame from
//! `FONT/frame0.wte`), text (`FONT/kanji_rd.dat`), icons
//! (`FONT/markfont.dat`) and the cursor are laid out by pixel.
//!
//! Positions and colours were measured on the original running in an
//! emulator, and window rectangles come from the game's own
//! tables (see `rom_formats::menus`).

pub use rules::strings::{HINTS, UiStrings, log_text, text};
pub use shell::images::Picture;
pub use shell::screen::{SCREEN_HEIGHT, SCREEN_WIDTH};

mod menus;
pub mod message;
mod packed;
pub mod services;
#[cfg(test)]
mod tests;
pub mod widgets;
mod windows;
mod writing;
pub use widgets::{Align, ListCursor, Picked, Rows, YesNo};
pub use writing::*;

use std::sync::Arc;

use media::font::{Font, TEXT_PITCH};

/// Both screens: the top one, then the touch screen the game is played on.
pub const CANVAS_HEIGHT: usize = SCREEN_HEIGHT * 2;
/// Windows are placed and sized in tiles of this many pixels.
pub const TILE: usize = 8;
/// Rows of the touch screen's windows start this many tiles down the canvas.
pub const BOTTOM: usize = SCREEN_HEIGHT / TILE;

/// Width of the frame texture.
const FRAME_WIDTH: usize = 128;
/// What fills a window: the game blends this grey half and half with the
/// floor on the touch screen, and shows it plain on the top screen.
const WINDOW_GREY: [u8; 3] = [32, 32, 32];
/// Where the thin frame's nine tiles start in the frame texture.
const THIN_FRAME: usize = 3;

/// Text colours.
pub mod color {
    pub const WHITE: [u8; 3] = [248, 248, 248];
    /// The leader's name.
    pub const BLUE: [u8; 3] = [0, 152, 248];
    /// Other team members.
    pub const YELLOW: [u8; 3] = [248, 248, 0];
    /// Moves.
    pub const GREEN: [u8; 3] = [0, 248, 0];
    /// Numbers that matter (`[CS:E]`), wild monsters.
    pub const CYAN: [u8; 3] = [0, 248, 248];
    /// Place names.
    pub const ORANGE: [u8; 3] = [248, 192, 96];
    /// Items.
    pub const ITEM: [u8; 3] = [248, 200, 200];
    /// The team's name.
    pub const PINK: [u8; 3] = [248, 160, 248];
    /// Warnings.
    pub const RED: [u8; 3] = [248, 88, 64];
}

/// Entries of `FONT/markfont.dat` used outside of marked-up text.
pub mod mark {
    /// The close button at the bottom of a menu.
    pub const CLOSE: usize = 11;
    /// The sort button at the bottom of the bag.
    pub const LIST: usize = 18;
    pub const POKE: usize = 33;
    /// The full yellow heart next to a team member.
    pub const HEART: usize = 48;
}

/// What the interface is drawn with, all from the ROM.
pub struct UiArt {
    pub font: Font,
    /// `FONT/frame0.wte` in its first colours: 128x32 RGBA. Its top-left
    /// 24x24 pixels are the nine tiles of a window's frame.
    pub frame: Vec<u8>,
    /// The same frame in green, as the game's top menu shows it.
    pub frame_green: Vec<u8>,
    /// The same frame in pink: the windows of a story whose hero is female.
    pub frame_pink: Vec<u8>,
    /// The four other types of frame the "Window" option offers
    /// (`FONT/frame1.wte`...), each in blue, pink and green.
    pub frame_types: Vec<Vec<u8>>,
    /// Pictures of the screens outside of dungeons, 256x192 RGBA each (empty
    /// when missing): the title's logo, the top menu's illustration and
    /// the pattern behind the controls there.
    pub title_logo: Vec<u8>,
    pub menu_art: Vec<u8>,
    pub menu_backdrop: Vec<u8>,
    /// The illustration over the menu the game starts on
    /// (`TOP/title_0.bgp`), likewise.
    pub title_art: Vec<u8>,
    /// The title scene's sprites, in the game's language
    /// (`GROUND/s05p02<language><n>.wan`): the game's subtitle under the
    /// logo, and "Press Start" (empty when missing).
    pub title_sprites: Vec<Picture>,
    /// The entries of `FONT/markfont.dat`: buttons, coin, hearts...
    pub marks: Vec<Picture>,
    /// The picture behind the top screen (`DUNGEON/sub.bgp`): 256x192
    /// RGBA. Empty: a checkerboard drawn like it.
    pub backdrop: Vec<u8>,
    /// The top line's own font, from `DUNGEON/dungeon.bin`: 128x16 RGBA,
    /// 8x8 tiles (see [`Canvas::hud`]).
    pub hud: Vec<u8>,
    /// The colours of text (`FONT/text_pal.pal`).
    pub text_colors: Vec<[u8; 3]>,
    /// The menus' yellow arrow and the grey arrow of a message waiting for
    /// a key (`FONT/cursor.wan`).
    pub cursor: Picture,
    pub more: Picture,
    pub strings: Arc<UiStrings>,
}

/// The colour of a `[CS:x]` tag, by what its letter marks: names, items,
/// moves, places, numbers, warnings... `l` and `p` mark the leader and its
/// partner in the messages the rules write; other letters are white.
pub fn code_color(_art: &UiArt, code: &str) -> [u8; 3] {
    match code.chars().next() {
        Some('l' | 'B') => color::BLUE,
        Some('p' | 'N' | 'Y') => color::YELLOW,
        Some('M' | 'G') => color::GREEN,
        Some('E' | 'V' | 'C') => color::CYAN,
        Some('P' | 'j' | 'O') => color::ORANGE,
        Some('I' | 'A') => color::ITEM,
        Some('K' | 'H') => color::PINK,
        Some('W' | 'R' | 'X') => color::RED,
        _ => color::WHITE,
    }
}

impl UiArt {
    /// A menu text without the formatting tags that follow its label.
    pub fn label(&self, index: usize) -> &str {
        let text = self.strings.menu.get(index).map_or("", String::as_str);
        text.split('[').next().unwrap_or("")
    }
}

fn pixel_at(rgba: &[u8], at: usize) -> Option<[u8; 4]> {
    rgba.get(at..at + 4)?.try_into().ok()
}

/// The colours the frames of windows come in.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Frame {
    #[default]
    Blue,
    Pink,
    Green,
}

/// Both screens being drawn: RGBA, transparent where the game shows through.
#[derive(Clone, PartialEq)]
pub struct Canvas {
    pub data: Vec<u8>,
    /// The colour of the frames of the windows drawn from now on, and
    /// their type (0 the first; see `UiArt::frame_types`).
    pub frame: Frame,
    pub frame_type: usize,
    /// The menus' cursor is in the hidden half of its blinking.
    pub cursor_hidden: bool,
    /// Where cursors were drawn on the picture before this one, and where
    /// they are on this one: a cursor that moved or appeared shows at once.
    pub cursors_before: Vec<(i32, i32)>,
    pub cursors: Vec<(i32, i32)>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            frame: Frame::Blue,
            frame_type: 0,
            cursor_hidden: false,
            cursors_before: Vec::new(),
            cursors: Vec::new(),
            data: vec![0; SCREEN_WIDTH * CANVAS_HEIGHT * 4],
        }
    }
}

impl Canvas {
    /// As new, what was drawn gone, keeping its buffer.
    pub fn clear(&mut self) {
        self.data.fill(0);
        (self.frame, self.frame_type, self.cursor_hidden) = (Frame::Blue, 0, false);
        self.cursors_before.clear();
        self.cursors.clear();
    }

    /// One pixel, blended over what is there.
    pub(crate) fn put(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= SCREEN_WIDTH as i32 || y >= CANVAS_HEIGHT as i32 {
            return;
        }
        let at = (y as usize * SCREEN_WIDTH + x as usize) * 4;
        let alpha = color[3] as u32;
        if alpha == 255 || self.data[at + 3] == 0 {
            self.data[at..at + 4].copy_from_slice(&color);
        } else if alpha > 0 {
            for (channel, &over) in color.iter().take(3).enumerate() {
                let under = self.data[at + channel] as u32;
                self.data[at + channel] = ((over as u32 * alpha + under * (255 - alpha)) / 255) as u8;
            }
            self.data[at + 3] = self.data[at + 3].max(color[3]);
        }
    }

    /// A box of one colour, blended over what is there as `Canvas::put`
    /// does, clipped to the canvas once.
    pub fn fill(&mut self, x: i32, y: i32, width: usize, height: usize, color: [u8; 4]) {
        let (left, right) = (x.max(0), (x + width as i32).min(SCREEN_WIDTH as i32));
        let (top, bottom) = (y.max(0), (y + height as i32).min(CANVAS_HEIGHT as i32));
        if left >= right || top >= bottom || color[3] == 0 {
            return;
        }
        let alpha = color[3] as u32;
        for row in top..bottom {
            let from = (row as usize * SCREEN_WIDTH + left as usize) * 4;
            for at in (from..from + (right - left) as usize * 4).step_by(4) {
                if alpha == 255 || self.data[at + 3] == 0 {
                    self.data[at..at + 4].copy_from_slice(&color);
                } else {
                    for (channel, &over) in color.iter().take(3).enumerate() {
                        let under = self.data[at + channel] as u32;
                        self.data[at + channel] = ((over as u32 * alpha + under * (255 - alpha)) / 255) as u8;
                    }
                    self.data[at + 3] = self.data[at + 3].max(color[3]);
                }
            }
        }
    }

    /// Copies an RGBA picture; its transparent pixels leave what is there.
    pub fn blit(&mut self, x: i32, y: i32, width: usize, height: usize, rgba: &[u8]) {
        for dy in 0..height {
            for dx in 0..width {
                if let Some(color) = pixel_at(rgba, (dy * width + dx) * 4) {
                    self.put(x + dx as i32, y + dy as i32, color);
                }
            }
        }
    }

    /// The top screen's backdrop.
    pub fn backdrop(&mut self, art: &UiArt) {
        self.backdrop_rows(art, 0..SCREEN_HEIGHT);
    }

    /// The backdrop over any rows of the canvas: the game's picture,
    /// repeated every screen, or without it a brown checkerboard of
    /// 4-pixel squares lit along their top and left, like the game's.
    pub fn backdrop_rows(&mut self, art: &UiArt, rows: std::ops::Range<usize>) {
        for y in rows {
            for x in 0..SCREEN_WIDTH {
                let from_art = pixel_at(&art.backdrop, (y % SCREEN_HEIGHT * SCREEN_WIDTH + x) * 4);
                let color = from_art.unwrap_or_else(|| {
                    let light = (x / 4 + y / 4) % 2 == 0;
                    let edge = x % 4 == 0 || y % 4 == 0;
                    let [r, g, b] = match (light, edge) {
                        (true, true) => [144, 128, 56],
                        (true, false) => [120, 104, 48],
                        (false, true) => [72, 56, 24],
                        (false, false) => [96, 80, 40],
                    };
                    [r, g, b, 255]
                });
                self.put(x as i32, y as i32, color);
            }
        }
    }
}
