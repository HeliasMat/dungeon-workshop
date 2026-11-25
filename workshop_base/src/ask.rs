//! What a part of the Workshop asks of the window: a space looked at, a file
//! opened in its space, the film played (from a line), a file moved, the
//! things drawn shown, a window opened, a word said in the bar of state. A
//! part pushes an `Ask` with `Session::ask`; the window
//! (`workshop/src/app.rs`, `App::answer`) takes them all once a frame and
//! does each. A part never reaches into another's state.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use eframe::egui;

use crate::theme;

/// The spaces, each a button on the right of the bar of menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Home,
    Game,
    Film,
    Decor,
    Dungeon,
    Assets,
    Emulator,
}

impl Space {
    pub const ALL: [Space; 7] = [Space::Home, Space::Game, Space::Film, Space::Decor, Space::Dungeon, Space::Assets, Space::Emulator];

    pub fn word(self) -> &'static str {
        match self {
            Space::Home => "Home",
            Space::Game => "Game",
            Space::Film => "Film",
            Space::Decor => "Decor",
            Space::Dungeon => "Dungeon",
            Space::Assets => "Resources",
            Space::Emulator => "Emulator",
        }
    }

    pub fn accent(self) -> egui::Color32 {
        match self {
            Space::Home => theme::HOME,
            Space::Game => theme::GAME,
            Space::Film => theme::FILM,
            Space::Decor => theme::DECOR,
            Space::Dungeon => theme::DUNGEON,
            Space::Assets => theme::ASSETS,
            Space::Emulator => theme::EMULATOR,
        }
    }

    /// Its place in `ALL`.
    pub fn at(self) -> usize {
        self as usize
    }
}

/// What to open in the Dungeon space: a new one, one by its word (the open
/// game's own if it has one by that word, else the shared library's), or
/// one by its file (a dungeon's or a fight's, wherever it is).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenDungeon {
    NewDungeon,
    NewFight,
    Dungeon(String),
    Fight(String),
    File(PathBuf),
}

/// One thing a part asks of the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// That space looked at.
    Go(Space),
    /// A film opened in the Film space, the cursor on a line (1 the first; 0: where it was).
    Film { path: PathBuf, line: usize },
    /// A decor opened in the Decor space, a line of it chosen (1 the first; 0: none).
    Decor { path: PathBuf, line: usize },
    /// A dungeon or a boss fight opened in the Dungeon space.
    Dungeon(OpenDungeon),
    /// The film open played from that line (1 the first).
    PlayFrom(usize),
    /// The film open played from its start.
    Play,
    /// A file moved (renamed, or put in the bin: `None`): the film or the decor open follows it.
    Moved { from: PathBuf, to: Option<PathBuf> },
    /// The Resources looked at, on the things drawn.
    Drawn,
    /// The SpriteCollab window, on a Pokédex number or not.
    SpriteCollab(Option<u32>),
    /// Said in the bar of state.
    Say(String),
}

/// What was asked and not yet done, in the order it was asked.
#[derive(Default)]
pub struct Asks(Mutex<Vec<Ask>>);

impl Asks {
    pub fn push(&self, ask: Ask) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).push(ask);
    }

    /// All that was asked, in order; none left.
    pub fn take(&self) -> Vec<Ask> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_are_taken_in_order_once() {
        let asks = Asks::default();
        asks.push(Ask::PlayFrom(3));
        asks.push(Ask::Go(Space::Film));
        assert_eq!(asks.take(), vec![Ask::PlayFrom(3), Ask::Go(Space::Film)]);
        assert!(asks.take().is_empty());
    }
}
