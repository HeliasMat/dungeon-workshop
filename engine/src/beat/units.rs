//! What a beat's bare numbers are: a way to face (`Direction`), a time
//! (`Frames`), and the game's ids (sounds, emotes, moves, songs,
//! animations), each a type of its own so none is taken for another.

use std::fmt;

/// A way to face, as the game counts them: starting facing down and
/// turning counter-clockwise (`Down` is 1, `DownLeft` 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Direction {
    Down = 1,
    DownRight,
    Right,
    UpRight,
    Up,
    UpLeft,
    Left,
    DownLeft,
}

impl Direction {
    /// The eight of them, from `Down`.
    pub const ALL: [Direction; 8] =
        [Direction::Down, Direction::DownRight, Direction::Right, Direction::UpRight, Direction::Up, Direction::UpLeft, Direction::Left, Direction::DownLeft];

    /// A way as written: its name (`down`, `upleft`...), never a
    /// number, which is someone.
    pub fn of(field: &str) -> Option<Direction> {
        Direction::ALL.into_iter().find(|way| way.name() == field)
    }

    /// Its name, as the files write it.
    pub fn name(self) -> &'static str {
        match self {
            Direction::Down => "down",
            Direction::DownRight => "downright",
            Direction::Right => "right",
            Direction::UpRight => "upright",
            Direction::Up => "up",
            Direction::UpLeft => "upleft",
            Direction::Left => "left",
            Direction::DownLeft => "downleft",
        }
    }

    /// The way of the game's number (1 to 8; 0 is `Down`, past 8 goes round).
    pub fn from_number(way: u16) -> Direction {
        Direction::ALL[(way.max(1) as usize - 1) % 8]
    }

    /// Its number for the stage (1 to 8).
    pub fn number(self) -> u16 {
        self as u16
    }
}

/// A time in frames, 60 a second.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Frames(pub f32);

impl Frames {
    pub const ZERO: Frames = Frames(0.0);

    /// How many frames.
    pub fn get(self) -> f32 {
        self.0
    }
}

impl From<f32> for Frames {
    fn from(frames: f32) -> Self {
        Frames(frames)
    }
}

impl fmt::Display for Frames {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, out)
    }
}

/// An id of the game's, a `u16` that cannot be taken for another kind.
macro_rules! id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u16);

        impl $name {
            /// Its number.
            pub fn get(self) -> u16 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(&self.0, out)
            }
        }
    };
}

id!(
    /// A sound effect of the game's.
    SoundId
);
id!(
    /// An effect shown on someone: an emote (`!`, `?`, sweat...) or another.
    EmoteId
);
id!(
    /// A move (an attack) of the game's.
    MoveId
);
id!(
    /// A song of the game's.
    SongId
);
id!(
    /// An animation of a body (its group in the sprite).
    AnimId
);

impl AnimId {
    /// Its group, as an index into the sprite's animations.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}
