//! Status conditions and the item and trap effects built on them.
//!
//! Traps, items and moves go by the numbers the game data gives them; the
//! balance figures (durations, damages, chances) come with the extracted
//! data (`Constants`), the rules that use them are this engine's own.

mod constants;
pub use constants::*;
mod effects;
pub use effects::*;
mod moves;
pub use moves::*;
mod apply;
pub use apply::*;
mod belly;
pub use belly::*;
mod traps;
pub use traps::*;
mod items;
pub use items::*;
mod statuses;
pub use statuses::*;
mod speed;
mod turns;

use super::combat::{Fighter, MoveSlot, Stats, TypeChart};
use super::rng::DungeonRng;
use super::weather;

#[cfg(test)]
pub(crate) mod tests;
