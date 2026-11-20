//! What films take from the game's dungeons: the tileset of each floor (for
//! the `decor` builder, which lays a dungeon room as a place), and the stairs
//! and the traps as pictures of things (`stairs`, `trap_slumber`...): made by
//! `game::setup`, which the Workshop's first launch runs too.

use std::sync::Arc;

use ::film::Context;

/// `scene_player dungeons`: `data/extra/dungeon_floors.txt` (each floor's tileset)
/// and the pictures in `data/hand/` (made of the game's: never committed).
pub fn prepare(context: &Arc<Context>) -> Result<(), String> {
    for line in game::setup::prepare_dungeons(context.data.as_str())? {
        println!("{line}");
    }
    Ok(())
}
