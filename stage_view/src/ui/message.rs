//! The message box, the same for the game and a film: a line written out a
//! character a frame, the portrait of who says it, the choice above it. The
//! game's own menus show theirs through it too, at once.

use rules::run::options::TEXT_SPEEDS;
use shell::Keys;
use shell::art::Art;
use shell::sound::Hears;

use super::{BOTTOM, Canvas, ListCursor, Picked, SCREEN_HEIGHT, TILE, UiArt, key_stops, pausing, reveal};

/// Where a face goes on the right of the box, in tiles.
pub const PORTRAIT_RIGHT: usize = 25;
/// The sound of a line writing itself out, heard once every `BETWEEN`
/// frames at most, and the click of a line read.
const TICK: u16 = 0x3F05;
const BETWEEN: u32 = 4;
const READ: u16 = 0x3F02;

/// What a scene has said: its pages, its choices, who says it.
#[derive(Clone, Default)]
pub struct Said {
    /// The message, cut where the box clears (none: the choices alone).
    pub pages: Vec<String>,
    pub options: Vec<String>,
    /// The monster, its face and whether it shows on the right.
    pub portrait: Option<(u16, u16, bool)>,
    /// The sound its letters make writing themselves out (none: the game's
    /// own).
    pub voice: Option<u16>,
    /// How many times as fast as usual it writes itself out.
    pub pace: Option<f32>,
    /// It goes when the scene says, not with a key.
    pub timed: bool,
}

/// A face on screen, and its column in tiles. (Its row is the box's: see
/// `Words::draw`.)
#[derive(Clone)]
pub struct Portrait {
    pub rgba: Vec<u8>,
    pub x: usize,
}

/// What the box shows.
#[derive(Clone, Default)]
pub struct Words {
    pub pages: Vec<String>,
    pub index: usize,
    pub options: Vec<String>,
    pub cursor: ListCursor,
    pub portrait: Option<Portrait>,
    pub voice: Option<u16>,
    pub pace: Option<f32>,
    pub timed: bool,
    /// A menu's: all there at once, no arrow, its keys are its own.
    pub menu: bool,
    /// How many characters of the page are written out so far.
    revealed: f32,
    /// How many of the page's waits for a key (`[K]`) are behind.
    passed: usize,
    /// For how many frames the page has been waiting for a key.
    waited: u32,
    /// Frames before the next tick is heard.
    ticked: u32,
}

impl Words {
    pub fn said(said: Said, art: &Art) -> Self {
        let portrait = said.portrait.and_then(|(monster, shown, right)| {
            let rgba = art.portrait(monster, shown)?;
            Some(Portrait { rgba, x: if right { PORTRAIT_RIGHT } else { 2 } })
        });
        Words { pages: said.pages, options: said.options, portrait, voice: said.voice, pace: said.pace, timed: said.timed, ..Words::default() }
    }

    /// A menu's message and choices, for this frame.
    pub fn of_menu(message: String, options: Vec<String>, cursor: usize) -> Self {
        let cursor = ListCursor::at(cursor);
        Words { pages: vec![message], options, cursor, menu: true, ..Words::default() }
    }

    /// The page on screen.
    pub fn page(&self) -> Option<&str> {
        self.pages.get(self.index).map(String::as_str)
    }

    /// How far the page on screen is written out before it waits for a
    /// key, and its whole length, in characters.
    fn limits(&self) -> (usize, usize) {
        let Some(page) = self.page() else {
            return (0, 0);
        };
        // (Pauses take as long as so many characters: `[W:n]`.)
        let length = engine::units(page);
        let stop = key_stops(page).get(self.passed).copied();
        (stop.unwrap_or(length).min(length), length)
    }

    /// What is written out up to where it waits for a key, in characters,
    /// once it waits there; and for how many frames it has.
    pub fn waiting(&self) -> Option<(usize, u32)> {
        let (limit, _) = self.limits();
        (self.page().is_some() && !self.menu && self.options.is_empty() && self.revealed as usize >= limit).then_some((limit, self.waited))
    }

    /// A frame of writing out, at the options' speed.
    pub fn write_out(&mut self, speed: u8, audio: Option<&dyn Hears>) {
        if self.menu {
            return;
        }
        let (limit, _) = self.limits();
        self.ticked = self.ticked.saturating_sub(1);
        // (Not while a pause is waited through: `[W:n]`.)
        let pausing = self.page().is_some_and(|page| pausing(page, self.revealed as usize));
        if !pausing && (self.revealed as usize) < limit && self.ticked == 0 {
            self.ticked = BETWEEN;
            if let Some(audio) = audio {
                audio.sound(self.voice.unwrap_or(TICK), 1.0, 1.0);
            }
        }
        let pace = TEXT_SPEEDS[speed as usize % TEXT_SPEEDS.len()].1 * engine::LETTERS_A_FRAME * self.pace.unwrap_or(1.0);
        self.revealed = (self.revealed + pace).min(limit as f32);
        self.waited = if (self.revealed as usize) < limit { 0 } else { self.waited + 1 };
        // (A line that goes by itself goes past its waits for a key once
        // what is written is read, as a film reads it.)
        let (_, length) = self.limits();
        if self.timed && limit < length && self.waited as f32 >= engine::read_after(limit, false) {
            (self.passed, self.waited) = (self.passed + 1, 0);
        }
    }

    /// The keys of a message or a choice: what was chosen once it is over
    /// (0 for a message read).
    pub fn press(&mut self, keys: &Keys, audio: Option<&dyn Hears>) -> Option<usize> {
        if !self.options.is_empty() {
            return match self.cursor.keys(keys, self.options.len()) {
                Picked::Chosen(chosen) => Some(chosen),
                _ => None,
            };
        }
        if self.timed || !keys.enter {
            return None;
        }
        let (limit, length) = self.limits();
        if (self.revealed as usize) < limit {
            self.revealed = limit as f32;
            return None;
        }
        if limit < length {
            self.passed += 1;
            return None;
        }
        if self.index + 1 < self.pages.len() {
            (self.index, self.passed, self.revealed, self.waited) = (self.index + 1, 0, 0.0, 0);
            return None;
        }
        if let Some(audio) = audio {
            audio.sound(READ, 1.0, 1.0);
        }
        Some(0)
    }

    /// The portrait in its thin frame, the plain window, the text as far as
    /// it is written out, and the arrow once the page waits for a key: all
    /// placed as measured on the original. (`top`: the box at the top of the
    /// lower screen, the portrait under it, as a scene may ask.)
    pub fn draw(&self, canvas: &mut Canvas, art: &UiArt, top: bool) {
        let below = SCREEN_HEIGHT as i32;
        let left = (2 * TILE) as i32;
        let width = 28 * TILE - 8;
        let (window, face) = if top { (1, 7) } else { (17, 11) };
        let Some(page) = self.page() else {
            return;
        };
        if let Some(portrait) = &self.portrait {
            canvas.portrait_frame(art, portrait.x, BOTTOM + face);
            canvas.blit((portrait.x * TILE) as i32, below + (face * TILE) as i32, 40, 40, &portrait.rgba);
        }
        canvas.dialogue_window(art, 2, BOTTOM + window, 28, 5);
        let written = if self.menu { page } else { reveal(page, self.revealed as usize) };
        let top_of = below + (window * TILE) as i32;
        let end = canvas.markup_end(art, left + 4, top_of + 2, width, written);
        // (What shows that the text waits for a key: inside the text (`[K]`),
        // the menus' cursor right after what is written; at its end, the
        // arrow under it, 16 frames out of 32.)
        let (limit, length) = self.limits();
        let waiting = !self.menu && self.revealed as usize >= limit && self.options.is_empty();
        if waiting && limit < length {
            canvas.cursor(art, end.0, end.1);
        } else if waiting && (self.waited / 16).is_multiple_of(2) {
            canvas.more_arrow(art, 129, top_of + 40);
        }
    }

    /// The choices above the box (see [`Canvas::choice_box`]; `between`:
    /// in a fight, between those who fight, over its log).
    pub fn draw_choices(&self, canvas: &mut Canvas, art: &UiArt, between: bool) {
        canvas.choice_box(art, &self.options, self.cursor, between);
    }
}
