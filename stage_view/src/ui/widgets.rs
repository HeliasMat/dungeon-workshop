//! The interface's widgets, the same for every screen of the game (its title,
//! its end card, its menu, the shops, the bank, the map; the dungeon's menu,
//! its bag, its questions) and for a film's choices: a framed window (see
//! [`Canvas::window`]), a header, rows with the cursor, a column of values
//! on their right, a titled list, a box of choices, a panel of text. All
//! drawn at the game's pixels, nothing soft.
//!
//! What the keys do to them is state of their own, tested without a picture:
//! [`ListCursor`] (up and down, wrapping round, what is seen of a long
//! list), [`YesNo`].

use rules::services::Input;
use shell::Keys;

use super::*;

/// A key as the menus read it: the arrows, Enter (confirm), Escape
/// (cancel); Space marks, R sorts. One a frame, the arrows first.
pub fn input(keys: &Keys) -> Option<Input> {
    let named = |key: &str| keys.named.iter().any(|name| name == key);
    match () {
        _ if keys.up => Some(Input::Up),
        _ if keys.down => Some(Input::Down),
        _ if keys.left => Some(Input::Left),
        _ if keys.right => Some(Input::Right),
        _ if keys.enter => Some(Input::Confirm),
        _ if keys.escape => Some(Input::Cancel),
        _ if named("space") => Some(Input::Mark),
        _ if named("r") => Some(Input::Sort),
        _ => None,
    }
}

/// What a key did to a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Picked {
    Nothing,
    /// The cursor went up or down.
    Moved,
    /// The row under the cursor chosen (Enter).
    Chosen(usize),
    /// The list left (Escape).
    Left,
}

/// A list's cursor: up and down go round from one end to the other; a long
/// list shows so many rows at a time, the cursor's among them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListCursor {
    pub at: usize,
}

impl ListCursor {
    pub fn at(at: usize) -> Self {
        ListCursor { at }
    }

    /// Kept on one of `rows` rows (the last, if they became fewer).
    pub fn within(&mut self, rows: usize) {
        self.at = self.at.min(rows.max(1) - 1);
    }

    /// A key read as one input (see [`input`]).
    pub fn press(&mut self, pressed: Option<Input>, rows: usize) -> Picked {
        let rows = rows.max(1);
        self.within(rows);
        match pressed {
            Some(Input::Down) => self.at = (self.at + 1) % rows,
            Some(Input::Up) => self.at = (self.at + rows - 1) % rows,
            Some(Input::Confirm) => return Picked::Chosen(self.at),
            Some(Input::Cancel) => return Picked::Left,
            _ => return Picked::Nothing,
        }
        Picked::Moved
    }

    /// The keys of a frame: down, then up (both held: back where it was),
    /// then Enter, else Escape.
    pub fn keys(&mut self, keys: &Keys, rows: usize) -> Picked {
        let moved = self.arrows(keys, rows);
        match () {
            _ if keys.enter => Picked::Chosen(self.at),
            _ if keys.escape => Picked::Left,
            _ if moved => Picked::Moved,
            _ => Picked::Nothing,
        }
    }

    /// Only the arrows of a frame, down then up: whether one was pressed.
    pub fn arrows(&mut self, keys: &Keys, rows: usize) -> bool {
        let rows = rows.max(1);
        self.within(rows);
        if keys.down {
            self.at = (self.at + 1) % rows;
        }
        if keys.up {
            self.at = (self.at + rows - 1) % rows;
        }
        keys.down || keys.up
    }

    /// The first row seen of a list showing `shown` at a time, scrolled so
    /// that the cursor is the last seen once it goes past them.
    pub fn first(&self, shown: usize) -> usize {
        self.at.saturating_sub(shown.max(1) - 1)
    }

    /// The first row of the cursor's page, a list read by pages of `shown`.
    pub fn page(&self, shown: usize) -> usize {
        self.at / shown.max(1) * shown.max(1)
    }
}

/// A question answered yes or no: any arrow turns it, Enter answers it,
/// Escape says no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YesNo {
    pub yes: bool,
}

impl Default for YesNo {
    fn default() -> Self {
        YesNo { yes: true }
    }
}

impl YesNo {
    /// The keys of a frame: the answer, once given.
    pub fn press(&mut self, keys: &Keys) -> Option<bool> {
        if keys.up || keys.down || keys.left || keys.right {
            self.yes = !self.yes;
        }
        match () {
            _ if keys.enter => Some(self.yes),
            _ if keys.escape => Some(false),
            _ => None,
        }
    }

    /// Its rows and the cursor's, to draw it as a list.
    pub const ROWS: [(&'static str, [u8; 3]); 2] = [("Yes", color::WHITE), ("No", color::WHITE)];

    pub fn cursor(&self) -> usize {
        if self.yes { 0 } else { 1 }
    }
}

/// Where a list's rows go: the top-left of the first (in pixels of the
/// canvas), how far apart, how far in their text starts from the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rows {
    pub left: i32,
    pub top: i32,
    pub pitch: i32,
    pub indent: i32,
}

/// How a panel lays out its lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Centred,
}

impl Canvas {
    /// A window's header: its title, then the line under it (the window's
    /// inside starting at `left`, `width` pixels wide).
    pub fn header(&mut self, art: &UiArt, left: i32, top: i32, width: usize, (title, colour): (&str, [u8; 3])) {
        self.text(art, left + 8, top, title, colour);
        self.separator(left + 2, top + 13, width.saturating_sub(4));
    }

    /// A list's rows, each in its colour, the cursor on the one it is on
    /// (none: no cursor). Returns the line of each, for what goes beside.
    pub fn rows(&mut self, art: &UiArt, at: Rows, rows: &[(&str, [u8; 3])], cursor: Option<usize>) -> Vec<i32> {
        let mut lines = Vec::with_capacity(rows.len());
        for (index, (row, colour)) in rows.iter().enumerate() {
            let line = at.top + index as i32 * at.pitch;
            self.text(art, at.left + at.indent, line, row, *colour);
            if cursor == Some(index) {
                self.cursor(art, at.left, line);
            }
            lines.push(line);
        }
        lines
    }

    /// The second column of a two-column list (a name, its price): each
    /// value ending at `right`, on its row's line.
    pub fn right_column(&mut self, art: &UiArt, right: i32, lines: &[i32], values: &[(&str, [u8; 3])]) {
        for (line, (value, colour)) in lines.iter().zip(values) {
            self.text_right(art, right, *line, value, *colour);
        }
    }

    /// A panel of text: its window (in tiles, as [`Canvas::window`]), its
    /// lines `pitch` apart from `inset` pixels down, from the left or each
    /// centred on the screen. An empty line leaves its room.
    pub fn text_panel(
        &mut self,
        art: &UiArt,
        (x, y, width, height): (usize, usize, usize, usize),
        lines: &[(&str, [u8; 3])],
        (inset, pitch): (i32, i32),
        align: Align,
    ) {
        self.window(art, x, y, width, height);
        let top = (y * TILE) as i32 + inset;
        for (row, (line, colour)) in lines.iter().enumerate().filter(|(_, (line, _))| !line.is_empty()) {
            let left = match align {
                Align::Left => (x * TILE) as i32 + 4,
                Align::Centred => (SCREEN_WIDTH as i32 - art.font.width(line) as i32).max(8) / 2,
            };
            self.text(art, left, top + row as i32 * pitch, line, *colour);
        }
    }

    /// One of the game's titled menus: the title, a line, the entries 16
    /// pixels apart (`slots` of them at least) and `indent` pixels in (12,
    /// or 16 for moves), a line and the close button. `y` is in canvas
    /// tiles. Returns the line of each entry, for what the caller adds to
    /// it.
    pub fn titled_menu(
        &mut self,
        art: &UiArt,
        (x, y, width): (usize, usize, usize),
        title: (&str, [u8; 3]),
        rows: &[(&str, [u8; 3])],
        slots: usize,
        cursor: Option<usize>,
        indent: i32,
    ) -> Vec<i32> {
        const ROW: usize = 16;
        let slots = slots.max(rows.len());
        self.window(art, x, y, width, (ROW * (slots + 2)) / TILE);
        let (left, top) = ((x * TILE) as i32, (y * TILE) as i32);
        let inside = width * TILE - 2;
        self.header(art, left, top, inside + 2, title);
        let lines = self.rows(art, Rows { left, top: top + 18, pitch: ROW as i32, indent }, rows, cursor);
        let foot = top + 16 + (slots * ROW) as i32;
        self.foot(art, left, foot, inside, false);
        lines
    }

    /// The foot of a list: a line, the close button on its right (and the
    /// sort button on its left, the bag's).
    pub fn foot(&mut self, art: &UiArt, left: i32, y: i32, inside: usize, sort: bool) {
        self.separator(left + 2, y, inside - 2);
        if sort {
            self.mark(art, left + 4, y + 3, mark::LIST);
        }
        if let Some((mark_width, _, _)) = art.marks.get(mark::CLOSE) {
            let at = left + inside as i32 - *mark_width as i32 - 4;
            self.mark(art, at, y + 3, mark::CLOSE);
        }
    }

    /// A box of choices above the dialogue box, laid out as measured on
    /// the original's: the window ends at tile (30, 15), its entries 14
    /// pixels apart, centred in its height, their text 13 pixels in; eight
    /// at a time. (`between`: in a fight, between those who fight, over its
    /// log.)
    pub fn choice_box(&mut self, art: &UiArt, options: &[String], cursor: ListCursor, between: bool) {
        const ENTRY: usize = 14;
        const SHOWN: usize = 8;
        if options.is_empty() {
            return;
        }
        let below = SCREEN_HEIGHT as i32;
        let widest = options.iter().map(|option| art.font.width(&strip_tags(option)));
        let wide = (widest.max().unwrap_or(0) + 16).div_ceil(TILE);
        let first = cursor.first(SHOWN);
        let rows = options.len().min(SHOWN);
        let high = (rows * ENTRY).div_ceil(TILE);
        let spare = ((high * TILE - rows * ENTRY) / 2) as i32;
        let (x, y) = match between {
            true => (32usize.saturating_sub(wide) / 2, 18usize.saturating_sub(high)),
            false => (30usize.saturating_sub(wide), 15usize.saturating_sub(high)),
        };
        self.dialogue_window(art, x, BOTTOM + y, wide, high);
        for (row, option) in options.iter().skip(first).take(SHOWN).enumerate() {
            let top = below + (y * TILE) as i32 + spare + (row * ENTRY) as i32;
            self.markup(art, (x * TILE) as i32 + 13, top, wide * TILE, option);
            if row + first == cursor.at {
                self.cursor(art, (x * TILE) as i32, top);
            }
        }
    }
}
