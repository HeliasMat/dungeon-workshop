//! What is said and shown over the scene: bubbles, captions, a choice, the
//! box, how lines are acted; emotions, one's or a crowd's.

use engine::Voice;
use scenes::fight::said;

use super::{Line, Names, Word};
use crate::emit;
use crate::film::Film;
use crate::screen::in_a_bubble;
use crate::words::{EMOTION_SOUNDS, EMOTIONS, of, plain};

pub const WORDS: &[Word] = &[
    Word::new("bubble", &["who:pokemon text..."], "bubble").names(Names::NoOne).maybe(bubble_said),
    Word::new("choice", &["text..."], "ask").maybe(choice_said),
    Word::new("box", &["top|bottom"], "boxside").does(box_said),
    Word::new("_answer", &["n:number"], "answer").does(answer_said),
    Word::new("emote", &["who:pokemon kind:word", "who:word kind:word d:duration"], "emote, sound").maybe(emotion_said),
];

/// A bubble by the word that says it.
const BUBBLES: [(&str, &str); 3] = [("bubble", "bubble"), ("thought", "think"), ("shout", "shout")];

/// In the log of a fight or of a dungeon's floor (its top line shown), else
/// as a line of the game, without a portrait, for as long as it is read.
pub fn told_or_logged(film: &mut Film, text: &str) {
    if film.combat.lead.is_some() || film.dungeon.dungeon_hud.is_some() {
        // (Its numbers found as it plays put in then, as a caption's.)
        film.beat(emit::battle_log(said::log_frames() as f32, text));
    } else {
        film.line_said("-", "-", &format!(" {text}"), said::read_frames(text) as i32);
    }
}

/// An emotion shown, and its sound if it has one.
fn emote(film: &mut Film, who: &str, emotion: &str, emote: &str) {
    film.emote(who, emote, Some(50.0));
    if let Some(sound) = of(&EMOTION_SOUNDS, emotion) {
        film.sound(sound);
    }
}

/// The words said on the line, joined.
fn said_of(line: &Line) -> String {
    line.args.words("text").join(" ")
}

/// `bubble a words`, `thought a words`, `shout a words`: the scene goes on.
fn bubble_said(film: &mut Film, line: &Line) -> bool {
    let said = said_of(line);
    if said.is_empty() {
        return false;
    }
    let voice = match of(&BUBBLES, &line.verb) {
        Some("shout") => Voice::Shouted,
        Some("think") => Voice::Thought,
        _ => Voice::Said,
    };
    film.bubble(&super::who_said(line), &in_a_bubble(&said), voice);
    true
}

/// `box top|bottom`: the dialogue box kept there, for this scene, whoever
/// speaks.
fn box_said(film: &mut Film, line: &Line) {
    film.box_kept = true;
    film.beat(emit::box_side(line.args.has("top")));
}

/// `choice A | B`, then `answer 1` ... `answer 2` ... `end`.
fn choice_said(film: &mut Film, line: &Line) -> bool {
    let crate::ir::Shape::Choice { options } = &line.shape else {
        return false;
    };
    let options = film.names_in(&options.join("|"));
    film.beat(emit::ask(options.split('|').map(str::to_string).collect()));
    film.timing.opened.push(choice_ends);
    true
}

/// The `end` of a choice.
fn choice_ends(film: &mut Film, _: &Line) {
    film.beat(emit::rejoin());
}

/// `emote a joy`: the emotion shown; one told for so long is the engine's.
fn emotion_said(film: &mut Film, line: &Line) -> bool {
    if line.args.has("d") {
        return false;
    }
    let (who, kind) = (super::who_said(line), plain(line.args.word("kind").unwrap_or_default()));
    match of(&EMOTIONS, &kind) {
        Some(shown) => emote(film, &who, &kind, shown),
        None => {
            let names: Vec<&str> = EMOTIONS.iter().map(|(name, _)| *name).collect();
            film.unknown(line.number, "unknown emote", &kind, names.iter().copied(), &names.join(", "));
        }
    }
    true
}

/// `_answer 2`: a choice's second option opens here (the library's own:
/// `choice` … `option` writes it).
fn answer_said(film: &mut Film, line: &Line) {
    let at = line.args.number("n").map_or(1, |answer| answer as i32) - 1;
    match u16::try_from(at) {
        Ok(at) => film.beat(emit::answer(at)),
        Err(_) => film.not_a(at + 1, "a number of an answer"),
    }
}
