//! What every part of Dungeon Workshop shares: the `Session` made once by
//! the Workshop's `main` and handed to every space (the library, the data,
//! how it was launched, what is read and kept), the look (`theme`, `icons`),
//! the project's files (`project`, `game`), safe saves (`safe`), the system's
//! dialogs (`pick`), a long work on its thread (`tool_run`), a line played as
//! an example (`example`), the own sounds read as samples (`own_sounds`),
//! what a making asks of the Resources (`done`), the tab of an open making
//! (`tabs`).
//!
//! It uses the engine's crates (`assets`, `film`, `game`, `preview`...) and
//! `egui`; every other crate of the Workshop uses it.

pub mod done;
pub mod example;
pub mod game;
pub mod icons;
pub mod launch;
pub mod own_sounds;
pub mod pick;
pub mod project;
pub mod safe;
pub mod session;
pub mod tabs;
pub mod theme;
pub mod tool_run;
