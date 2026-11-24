//! The text of a film read, before any word is played:
//!
//! - `front`: the text read by the language, written as the tree of lines the words play
//!   (a recipe kept as the language wrote it, and written for each of its calls);
//! - `recipes`: the recipes (`define`, `repeat`, `uses`, `include`) played out on that
//!   tree, each line read on from the one before (`there`, `team`);
//! - `cast`: who a scene's lines name, and who a line puts there.
//!
//! Here too, what a line is: who says it, when it says it happens.

pub mod cast;
pub mod checked;
pub mod front;
mod lexicon;
pub mod recipes;
pub mod written;
