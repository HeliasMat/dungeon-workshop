//! The groups of words the core keeps, a file by theme, each its words
//! (`WORDS`) and its `GROUP`: those its compile calls (`things`, `timing`,
//! `story`, `aura`) and those beside them (`items`, `game`, `crowds`). The
//! other groups are a crate each (`film_view`, `film_speech`,
//! `film_acting`...), and `film` gathers them all in their order
//! (`Lexicon`). A group plays a line by the film's helpers (`emit`,
//! `names`, `spots`) and calls another word by `Film::call` (`play`).

pub mod aura;
pub mod crowds;
pub mod game;
pub mod items;
pub mod story;
pub mod things;
pub mod timing;
