//! This game on the scene engine (`engine/`): the words of its scenes, what
//! they ask of it, its fights and how they show. The rules they use are
//! `rules`'s, which knows no engine.

/// What this game's scenes ask of it.
pub mod commands;
/// A fight, its rules and how it shows.
pub mod fight;
/// The words of this game in its scenes.
pub mod vocabulary;
