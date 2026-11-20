//! A film as it is worked on, with no window: read from a file or from a
//! text being written, played as a player would, gone to any moment at once,
//! drawn, told what is wrong with it. (The game that plays films as its
//! cutscenes and places is `game`, which uses this crate.)
//!
//! - `art`: what a film shows, read from the folder of the game's files;
//! - `check`: what is wrong in a film, as it reads;
//! - `debug`: what a film shows and lets do for whoever writes it;
//! - `film`: a film played, a frame at a time, and kept whole at a moment
//!   (`Snapshot`) to go back to it; what it asks of a game (`GameRequest`);
//! - `looks`: what the things a film names look like, small;
//! - `played`: a film played at once: when each line and scene plays, who
//!   moves when (`Timeline`);
//! - `session`: a film being written: its text (`film::doc`), played anew
//!   as it changes, a moment kept every two seconds, any moment drawn;
//! - `state`: who is where at a moment, the camera, the objects (what
//!   `scene_player state` prints, the Workshop's State panel shows).

pub mod art;
pub mod check;
pub mod debug;
pub mod film;
pub mod looks;
pub mod played;
pub mod session;
pub mod state;

#[cfg(test)]
mod tests;
