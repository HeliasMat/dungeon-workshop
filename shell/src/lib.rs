//! The shell a program of this project lives in: a window and its keys, a
//! picture drawn by hand, sounds played, the game's art read from its
//! folder and kept. It knows no rule and no scene: `stage_view` draws on it.
//! The pictures, letters and music themselves, as data, are `media`'s (its
//! modules most used are reached from here too).

pub mod art;
pub mod blit;
mod error;
pub use assets::data;
pub use media::{art_files, audio, images, music};
pub mod own_sounds;
pub mod record;
pub mod screen;
pub mod sheet;
pub mod sound;
pub mod window;

pub use blit::{Around, Fx, Mix, Paint, blit, blit_with};
pub use error::Error;
pub use screen::{Laid, Screen, Tint};
pub use window::{Key, Keys, Window};
