//! The groups of words the core leans on, a file by theme, each its words
//! (`WORDS`) and its `GROUP`; the other groups are a crate each
//! (`film_fishing`...), and `film` gathers them all in their order
//! (`Lexicon`). A group plays a line by the film's helpers (`emit`,
//! `names`, `spots`), never by another layer above it.
//!
//! - `said`, `heard`, `view`, `sky`, `pictures`: what is said and shown,
//!   heard, the view, the light, the game's pictures;
//! - `things`, `items`, `effects`, `aura`, `nodes`: things of a scene and
//!   in the hands, effects, auras, the nodes' properties;
//! - `crowds`, `rituals`, `game`, `story`: crowds, the guild's days, what a
//!   game is told, a story's moments;
//! - `timing`: time in a scene, the one rule of who waits.

pub mod aura;
pub mod crowds;
pub mod game;
pub mod heard;
pub mod items;
pub mod rituals;
pub mod said;
pub mod story;
pub mod things;
pub mod timing;
