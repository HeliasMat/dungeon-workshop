//! Life at the guild, between two dungeons: a mission taken from the board
//! and told back (remembered from one line to the next), a bout of
//! training.

use crate::DungeonState;
use film_core::emit;
use film_core::film::Film;
use film_core::screen::on_screen;
use film_core::verbs::{Line, Word};

pub const WORDS: &[Word] =
    &[Word::new("job", &["take|done|fail name:text..."], "take: cutaway job_board, narrate, jingle; done: music mission_accomplished, narrate; fail: narrate")
        .family("Guild")
        .ds_work("the guild's job lists (ProcessSpecial)")
        .names(film_core::verbs::Names::NoOne)
        .does(job)];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("guild", WORDS);

/// `job take Find Bellsprout`: a job taken from the guild's board: the
/// board, the words, the tune; `job done Find Bellsprout`: the job done, the
/// game's tune of a mission accomplished; `job fail …`: failed.
fn job(film: &mut Film, line: &Line) {
    let name = line.args.words("name").join(" ");
    if line.args.has("take") {
        let shown = film.picture("job_board");
        film_core::verbs::pictures::cut_to(film, &shown);
        film.sound("click");
        film.narrate(&on_screen(&format!("Job accepted: {name}")));
        film.kept::<DungeonState>().mission = Some(name);
        film.beat(emit::jingle(7));
        film_core::verbs::pictures::cut_back(film);
    } else if line.args.has("done") {
        film.beat(emit::music(scenes::vocabulary::POKEMON.song("mission_accomplished").map(engine::SongId)));
        film.narrate(&on_screen(&format!("Job complete: {name}!")));
        film.kept::<DungeonState>().mission = None;
    } else {
        film.sound("annoyed");
        film.narrate(&on_screen(&format!("Job failed: {name}...")));
        film.kept::<DungeonState>().mission = None;
    }
}
