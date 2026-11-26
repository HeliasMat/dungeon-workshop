//! What is said and shown over the scene: bubbles, captions, a choice, the
//! box, how lines are acted; emotions, one's or a crowd's.

use engine::Voice;
use scenes::fight::said;

use film_core::emit;
use film_core::film::Film;
use film_core::screen::in_a_bubble;
use film_core::verbs::{Line, Names, Word};
use film_core::words::{EMOTION_SOUNDS, EMOTIONS, of, plain};

pub const WORDS: &[Word] = &[
    Word::new("bubble", &["who:pokemon text..."], "bubble")
        .family("Advanced")
        .ds_close(
            "a line of Sky's box (its portrait, its words), closing by itself after the bubble's time: message_SetWaitMode(frames, 0), message_Talk, message_Close",
            "a box, not a bubble: one at a time, at the bottom of the screen, never two side by side",
        )
        .names(Names::NoOne)
        .maybe(bubble_said),
    Word::new("choice", &["text..."], "ask")
        .family("Structure")
        .ds_direct("switch ( message_SwitchMenu(0, 1) ) { case menu(\"…\"): … }, in the box of the line before")
        .maybe(choice_said),
    Word::new("box", &["top|bottom"], "boxside").does(box_said),
    Word::new("_answer", &["n:number"], "answer").does(answer_said),
    Word::new("emote", &["who:pokemon kind:word", "who:word kind:word d:duration"], "emote, sound").maybe(emotion_said),
    Word::new("chatter", &["*who [n:number]"], "short bubbles here and there among them, by chance (!, ..., a word): 6 if not told, one every few frames")
        .family("Speech")
        .ds_close(
            "each bubble as Sky's emote over the one saying it (!, ?, laughing, joy, sweat): SetEffect(EFFECT_…, 3); the film waits as long",
            "no words: Sky's emotes over the crowd instead",
        )
        .word_first()
        .does(chattered),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("said", WORDS);

/// What a crowd says when it chatters: short, of no one's story.
const CHATTER: [&str; 12] = ["!", "...", "?", "Oh!", "Hm.", "Ha ha!", "Yes!", "No way!", "Wow...", "Look!", "Hey!", "Really?"];
/// How long each of those bubbles stays, and how far apart they come.
const CHATTER_FRAMES: f32 = 50.0;
const CHATTER_GAP: u64 = 14;

/// A bubble by the word that says it.
const BUBBLES: [(&str, &str); 3] = [("bubble", "bubble"), ("thought", "think"), ("shout", "shout")];

/// In the log of a fight or of a dungeon's floor (its top line shown), else
/// as a line of the game, without a portrait, for as long as it is read.
pub fn told_or_logged(film: &mut Film, text: &str) {
    if film.logs() {
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
    film.bubble(&film_core::verbs::who_said(line), &in_a_bubble(&said), voice);
    true
}

/// `chatter crowd [n]`, `chatter a b c [n]`: short bubbles by chance among
/// them, one after another, a few frames apart (never twice the same one
/// running); the film waits for the last, unless `&`.
fn chattered(film: &mut Film, line: &Line) {
    let named: Vec<String> = line.args.words("who").iter().flat_map(|one| film.everyone(&plain(one))).collect();
    if named.is_empty() {
        return;
    }
    let count = line.args.number("n").map_or(6, |count| count.max(1.0) as u64);
    let (mut at, mut last) = (0.0, usize::MAX);
    for _ in 0..count {
        let mut one = film.draw() as usize % named.len();
        if one == last && named.len() > 1 {
            one = (one + 1) % named.len();
        }
        last = one;
        let text = CHATTER[film.draw() as usize % CHATTER.len()];
        film.on(&named[one], |who| emit::after(at, 0.0, emit::bubble(who, CHATTER_FRAMES, text)));
        at += (CHATTER_GAP + film.draw() % CHATTER_GAP) as f32;
    }
    film.wait_frames((at + CHATTER_FRAMES).round() as i64, line.wait);
}

/// `box top|bottom`: the dialogue box kept there, for this scene, whoever
/// speaks.
fn box_said(film: &mut Film, line: &Line) {
    film.box_kept = true;
    film.beat(emit::box_side(line.args.has("top")));
}

/// `choice A | B`, then `answer 1` ... `answer 2` ... `end`.
fn choice_said(film: &mut Film, line: &Line) -> bool {
    let film_core::ir::Shape::Choice { options } = &line.shape else {
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
    let (who, kind) = (film_core::verbs::who_said(line), plain(line.args.word("kind").unwrap_or_default()));
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
