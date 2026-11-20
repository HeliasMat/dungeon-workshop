//! The game's rules and data: dungeons, attacks, a run, the town, the
//! words of its interface. No window, no sound, no drawing, no ROM, no
//! scene engine (this game on the engine: `scenes`).

/// One attack played out, for the floors and the fights alike.
pub mod attack;
/// Marowak's Dojo and its training mazes.
pub mod dojo;
/// The dungeon engine: floors, rules, combat, the foes' minds.
pub mod dungeon;
pub mod jobs;
/// A dungeon of one's own, written as text (`.dungeon`).
pub mod own;
pub mod quiz;
/// Rule options, each off unless asked for (`rules:`).
pub mod rule_options;
/// A game in plain data: the team, the dungeon, its turns.
pub mod run;
pub mod sentry;
pub mod services;
pub mod songs;
/// The interface's words.
pub mod strings;
/// What `dungeons.bin` holds.
pub mod tables;
pub mod town;
/// The dungeons opened by jobs and by clearing others.
pub mod unlocks;
