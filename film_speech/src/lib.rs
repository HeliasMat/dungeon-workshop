//! What the films say and make heard, a group each: what is said and shown
//! over the scene (`said`: bubbles, captions, a choice, the box, how lines
//! are acted) and what is heard (`heard`: the game's sounds, a move's, our
//! own, ambients, music). A line said itself (`pikachu (happy): Hi`) is the
//! core's (`film_core::compile`); these are the words around it. `film`
//! puts their `GROUP`s in its order.

pub mod heard;
pub mod said;
