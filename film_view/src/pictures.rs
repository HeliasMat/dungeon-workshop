//! The game's pictures: looks at them, visions, what is made of them.

use film_core::emit;
use film_core::film::Film;
use film_core::screen::{BREAK, on_screen};
use film_core::tables::Picture;
use film_core::verbs::{Line, Names, Word};
use film_core::words::plain;

pub const WORDS: &[Word] = &[
    Word::new("memory", &[""], "whiteout, a sepia tint, whitein; at its end, the same back")
        .family("Screen")
        .ds_direct("screen_FlushOut(1, 0, 8, 0, 0, 0): Sky's own sepia of memories; screen_FlushIn(1, 0, 1, …) after")
        .does(souvenir),
    Word::new(
        "transition",
        &["kind:fade|white|boss|wipe|iris|cut in|out [d:duration]"],
        "fadeout/fadein, whiteout/whitein, iris, wipe: the screen goes, or comes back, now",
    )
    .names(Names::NoOne)
    .does(screen_goes),
    Word::new("banner", &["kind:chapter|place|dungeon text..."], "typecaption on the dark (chapter, dungeon), caption over the scene (place)")
        .family("Structure")
        .ds_close(
            "message_Narration on the black screen (Sky's chapter banners need a picture of their own and say “Chapter n”)",
            "a narration on black, not Sky's chapter picture",
        )
        .names(Names::NoOne)
        .does(bannered),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("pictures", WORDS);

/// A picture in the place of the scene, at once.
fn show(film: &mut Film, shown: &Picture) {
    film.beat(emit::cutaway(shown.level, shown.x as f32, shown.y as f32, false, 0.0));
}

/// The screen fades out (or in) from black in so many frames, and the
/// scene waits for it.
fn faded(film: &mut Film, out: bool, frames: f32) {
    film.beat(emit::fade(out, false, frames));
    film.beat(emit::wait_fade());
}

/// The picture instead of the scene, through a quick dark.
pub fn cut_to(film: &mut Film, shown: &Picture) {
    faded(film, true, 12.0);
    show(film, shown);
    faded(film, false, 12.0);
}

pub fn cut_back(film: &mut Film) {
    faded(film, true, 12.0);
    film.beat(emit::cutaway_off());
    faded(film, false, 12.0);
}

/// Words over the scene (`typed`: a letter at a time) for that many
/// frames, their lines cut at `BREAK`; `{name:pikachu}` in words not typed
/// by its code, as the game puts names in.
fn caption(film: &mut Film, frames: f32, text: &str, typed: bool) {
    film.caption(frames, text, typed);
}

/// `transition fade out 1s`, `transition iris in`: the screen goes (or
/// comes back) now, in the middle of a scene, the film waiting for it.
fn screen_goes(film: &mut Film, line: &Line) {
    let args = &line.args;
    let out = args.has("out");
    let frames = args.frames("d").unwrap_or(30) as f32;
    match plain(args.word("kind").unwrap_or_default()).as_str() {
        "white" => film.beat(emit::fade(out, true, frames)),
        "boss" if out => film_core::emit::boss_flashes(film),
        "boss" => film.beat(emit::fade(false, true, frames)),
        "iris" => {
            let on = film.speaking.last_speaker.clone().and_then(|last| film.other(&last));
            film.beat(emit::iris(!out, frames, on));
        }
        "wipe" => film.beat(emit::wipe(out, 1, frames)),
        "cut" => film.beat(emit::fade(out, false, 1.0)),
        _ => film.beat(emit::fade(out, false, frames)),
    }
    film.beat(emit::wait_fade());
}

/// `banner chapter The Storm`: a chapter's title typed alone on the dark;
/// `banner dungeon Foggy Woods`, a dungeon's name on the dark as the team
/// goes in; `banner place Treasure Town`, a place's name over the scene.
fn bannered(film: &mut Film, line: &Line) {
    let text = line.args.words("text").join(" ");
    match plain(line.args.word("kind").unwrap_or_default()).as_str() {
        "place" => {
            let frames = 120 + text.chars().count() * 3;
            caption(film, frames as f32, &on_screen(&text), false);
            film.wait(frames as i64);
        }
        "dungeon" => {
            let frames = 140 + text.chars().count() * 2;
            faded(film, true, 24.0);
            caption(film, frames as f32, &on_screen(&text), false);
            film.wait(frames as i64);
            faded(film, false, 24.0);
        }
        _ => on_black(film, &text, ""),
    }
}

/// Words alone on a dark screen, then the scene again.
pub fn on_black(film: &mut Film, text: &str, under: &str) {
    let frames = 100 + (text.chars().count() + under.chars().count()) * 3;
    faded(film, true, 24.0);
    let shown = if under.is_empty() { on_screen(text) } else { format!("{}{BREAK}{under}", on_screen(text)) };
    caption(film, frames as f32, &shown, true);
    film.wait(frames as i64);
    faded(film, false, 24.0);
}

/// Who a vision or a memory brought on is gone with it, in its white.
fn woken(film: &mut Film) {
    for one in std::mem::take(&mut film.staging.dreamt) {
        film.on(&one, emit::gone);
    }
}

/// The brown a memory is seen through: red, green, blue, how much.
const SEPIA: [f32; 4] = [120.0, 80.0, 30.0, 96.0];

/// `memory` ... `end`: a memory, in sepia between two white fades.
fn souvenir(film: &mut Film, _: &Line) {
    film.timing.opened.push(|film, _| remembering(film, false));
    remembering(film, true);
}

/// Into a memory, or back from it.
fn remembering(film: &mut Film, remembered: bool) {
    film.staging.vision = remembered;
    film.beat(emit::fade(true, true, 30.0));
    film.beat(emit::wait_fade());
    if !remembered {
        woken(film);
    }
    // (A memory is in sepia: the colours gone, a warm brown laid over, as an old picture.)
    film.beat(emit::grey(remembered));
    match remembered {
        true => film.beat(emit::tint(SEPIA, 1.0)),
        false => film.beat(emit::tint(film.staging.daylight(), 1.0)),
    }
    film.beat(emit::fade(false, true, 30.0));
    film.beat(emit::wait_fade());
}
