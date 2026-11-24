//! A dungeon floor played in a scene, as the game plays it: the team come up
//! the stairs onto the floor, its log at the bottom, the stairs found and
//! taken, a trap stepped on, an item found on the ground. The rooms are laid
//! by the `decor` builder with the floor's own tiles; the stairs and the traps
//! are the game's pictures (`scene_player dungeons`). The guild's jobs, taken
//! from its board and done or failed, are its words too (`guild`).

use engine::{Cam, Pose, Spot};
use film_core::emit;
use film_core::film::Film;
use film_core::names::things::{known_thing, thing_named};
use film_core::spots::routes::{found, go};
use film_core::spots::somewhere;
use film_core::verbs::items::take_up;
use film_core::verbs::things::{arrives, thing_apart, thing_out};
use film_core::verbs::{Line, Word};
use film_core::words::plain;

pub mod guild;
mod state;
pub use state::DungeonState;
mod events;
mod floor;
mod hud;
mod shop;

use events::*;
use floor::*;
use hud::*;
pub use hud::{fight_begins, fight_ends, floor_for_a_fight, lost_in_a_fight};
use shop::*;

pub const WORDS: &[Word] = &[
    Word::new(
        "stairs",
        &["take *who [floor n:number [up]]", "[where:place]"],
        "the game's stairs laid there (on the right if not said): put, under the bodies; take: the team goes up them, the floor ends",
    )
    .does(lay_stairs),
    Word::new(
        "trap",
        &["off", "kind:word on who:pokemon", "who:pokemon kind:word"],
        "the game's trap under him: put, appear, sound, battlelog, and what it does",
    )
    .does(trapped),
    Word::new("ambush", &["*foes on who:pokemon"], "battlelog, sound, quake; each: put around him, falls from above in its dust, emote")
        .does(ambush)
        .brings(&["foes"]),
    Word::new("shop", &["who:pokemon buy thing...", "who:pokemon goods..."], "the items laid out in rows before the shopkeeper: put, thing; battlelog")
        .does(shop),
    Word::new("steal", &["who:pokemon thing..."], "picks up without paying; the shopkeeper: emote, tremble, quake, popup, sound, battlelog").does(steals),
];

/// Its words, a family of the language (`Lexicon`).
pub const FAMILY: film_core::verbs::Family = film_core::verbs::Family::new("dungeon", WORDS);

/// The stairs' and the traps' pictures (data/hand/, `scene_player dungeons`).
const STAIRS: &str = "stairs";

/// The game's sound of stairs.
const STAIR_SOUND: i32 = 0x1309;

/// How long a log line stays.
const LOGGED: i32 = 150;

/// Between two of the team coming onto a floor, or going down.
const ONE_BY_ONE: i32 = 14;

/// What a floor shows on the screen, taken away as the team leaves it (the
/// stairs, the bottom, beaten, an orb, the wind, the badge): its top line
/// and its map are not left over the black.
pub fn floor_left(film: &mut Film) {
    if film.kept::<DungeonState>().dungeon_hud.take().is_some() {
        top_line_off(film);
    }
    if film.kept::<DungeonState>().map_team.take().is_some() {
        film.beat(emit::minimap(None, false));
    }
    if std::mem::take(&mut film.kept::<DungeonState>().dark) {
        film.beat(emit::spotlight_off());
    }
}

fn logged(film: &mut Film, text: &str) {
    film.beat(emit::battle_log(LOGGED as f32, film_core::screen::glued(text)));
}
