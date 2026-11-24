//! Life at the guild, between two dungeons: a mission taken from the board
//! and told back (remembered from one line to the next), a bout of
//! training. (The reward, the night, the chores, a drink, signing up, an
//! announcement, a diploma: recipes, `film/recipes/guild.film`.)

use super::{Line, Word};
use crate::emit;
use crate::film::Film;
use crate::screen::on_screen;

pub const WORDS: &[Word] =
    &[Word::new("job", &["take|done|fail name:text..."], "take: cutaway job_board, narrate, jingle; done: music mission_accomplished, narrate; fail: narrate")
        .names(super::Names::NoOne)
        .does(job)];

/// Its words, a family of the language (`Lexicon`).
pub const FAMILY: crate::verbs::Family = crate::verbs::Family::new("guild", WORDS);

/// `job take Find Bellsprout`: a job taken from the guild's board: the
/// board, the words, the tune; `job done Find Bellsprout`: the job done, the
/// game's tune of a mission accomplished; `job fail …`: failed.
fn job(film: &mut Film, line: &Line) {
    let name = line.args.words("name").join(" ");
    if line.args.has("take") {
        let shown = film.picture("job_board");
        super::pictures::cut_to(film, &shown);
        film.sound("click");
        film.narrate(&on_screen(&format!("Job accepted: {name}")));
        film.dungeon.mission = Some(name);
        film.beat(emit::jingle(7));
        super::pictures::cut_back(film);
    } else if line.args.has("done") {
        film.beat(emit::music(scenes::vocabulary::POKEMON.song("mission_accomplished").map(engine::SongId)));
        film.narrate(&on_screen(&format!("Job complete: {name}!")));
        film.dungeon.mission = None;
    } else {
        film.sound("annoyed");
        film.narrate(&on_screen(&format!("Job failed: {name}...")));
        film.dungeon.mission = None;
    }
}
