//! What is shown of no one's doing, placed exactly: effects, particles, a
//! flash, the screen trembling.

use crate::view::aim;
use film_core::film::Film;
use film_core::verbs::{Line, Names, Word};
use film_core::words::{frames_of, of, plain};
use film_core::{emit, fields};

pub const WORDS: &[Word] = &[
    Word::new(
        "effect",
        &[
            "move:thing... from source:place to target:place [in d:duration]",
            "move:thing... on target:place",
            "move:thing... at target:place",
            "who:pokemon [n:number]",
        ],
        "moveeffect, bolt (on a); moveshot (from a to b)",
    ).family("Effects").ds_close("SetEffect<actor>(n), WaitEffect: a script effect of the move's type (fire, water, lightning…), not the move's own", "a script effect of the move's type, not the move's own").word_first()
    .maybe(effect_line),
    Word::new("flash", &["colour:word at after:duration", "colour:word [d:duration]", ""], "flash; after (at 12); alone, white with the game's shock").family("Screen").ds_direct("white: screen_WhiteOut(1, 2), screen_FadeIn; a colour: screen_FlushOut / FlushIn(1, n, 2, r, g, b)")
        .maybe(flash_said),
    Word::new("shake", &["off", "[soft|hard] [d:duration]"], "shake: the view shakes up and down until shake off (or for so long)").family("Camera").ds_direct("camera_SetEffect(2, 1, 1.0) soft, (2, 2, 3.0) hard, (0, 0, 0) off, as Sky's")
        .names(Names::NoOne)
        .does(shaking)
        .instant(),
    Word::new("tremble", &["screen strength:number d:duration [weak|strong]", "thing... strength:number [d:duration]"], "tremble screen: quake").maybe(shaken),
    Word::new(
        "aura",
        &["on who:pokemon", "off who:pokemon"],
        "an ambience of his kind's type, never by itself, in sharp pixels: a fire kind's cinders about him (emitter), a spark off an electric kind angry, a drop off a water kind scared, leaves off a grass kind dancing (particles); off: none",
    ).family("Effects").ds_close("nothing shown: the cinders, sparks, drops and leaves of a type are the engine's own particles", "no aura: Sky's scripts have no particles of their own")
    .does(film_core::groups::aura::aura),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("effects", WORDS);

/// The word that is no move: it has its own bolt.
const BOLT: &str = "bolt";
/// The first of the unseen points effects are put on.
pub const FIRST_MARK: usize = 900;
/// How much of a shake each word leaves.
const SHAKES: [(&str, f32); 2] = [("weak", 0.5), ("strong", 2.0)];

fn in_seconds(word: &str) -> Option<i32> {
    word.ends_with('s').then(|| frames_of(word)).flatten()
}

/// Someone an effect shows on: who is told, or an unseen point put there.
fn on_someone(film: &mut Film, goal: Vec<String>) -> String {
    if goal[0] != "@" {
        return goal[0].clone();
    }
    let mark = format!("p{}", FIRST_MARK + film.timing.marks);
    film.timing.marks += 1;
    match (goal[1].parse::<f32>(), goal[2].parse::<f32>()) {
        (Ok(x), Ok(y)) => film.on(&mark, |mark| emit::put(mark, x, y, engine::Direction::Down)),
        _ => film.put_at(&mark, &goal[1], &goal[2]),
    }
    mark
}

/// A move by its words, or the bolt; told when neither.
fn effect_said(film: &mut Film, words: &[String], number: usize) -> Option<String> {
    let name = plain(&words.concat());
    if let Some(row) = film.tables.moves.get(&name) {
        return Some(row.0.clone());
    }
    let tables = film.tables.clone();
    film.unknown(number, "unknown effect", &words.join(" "), std::iter::once(BOLT).chain(tables.moves.keys().map(String::as_str)), "");
    None
}

/// `effect Hydro Pump from a to b in 40`: what it throws, and no blow.
fn thrown(film: &mut Film, line: &Line) {
    // (`in 40`: frames, or seconds, `in 1s`.)
    let frames = line.args.frames("d");
    let shot = effect_said(film, line.args.words("move"), line.number);
    let (source, goal) = (aim(film, line.args.words("source"), line.number), aim(film, line.args.words("target"), line.number));
    let (Some(shot), Some(source), Some(goal)) = (shot, source, goal) else {
        return;
    };
    let (source, goal) = (on_someone(film, source), on_someone(film, goal));
    match (film.other(&source), film.other(&goal), shot.parse::<u16>()) {
        (Some(from), Some(to), Ok(id)) => film.beat(emit::move_shot(from, to, engine::MoveId(id), frames.map(|frames| frames as f32))),
        _ => film.unread(format!("{source} {goal} {shot}")),
    }
    if let Some(frames) = frames.filter(|_| line.wait) {
        film.wait(i64::from(frames));
    }
}

/// `effect Hydro Pump from a to b`; `effect Bolt on a`, `effect Surf on 300
/// 200`, `on between a b`. Else the engine's.
fn effect_line(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    if args.has("to") {
        thrown(film, line);
        return true;
    }
    if !args.has("on") && !args.has("at") {
        return false;
    }
    let Some(goal) = aim(film, args.words("target"), line.number) else {
        return true;
    };
    let named = args.words("move").to_vec();
    if plain(&named.concat()) == BOLT {
        // (A bolt, a white flash, its thunder a little later.)
        match crate::view::cam_of(film, &goal) {
            Some(on) => {
                film.beat(emit::lightning(on));
                film.beat(emit::flash(emit::colour(engine::Colour::White).map(f32::from), 8.0));
                if let Some(id) = scenes::vocabulary::POKEMON.sound("flash") {
                    film.beat(emit::after(20.0, 0.0, emit::sound(engine::SoundId(id))));
                }
            }
            None => film.unread(goal.join(" ")),
        }
    } else if let Some(effect) = effect_said(film, &named, line.number) {
        let who = on_someone(film, goal);
        match (film.someone(&who), effect.parse::<u16>()) {
            (Some(_), Ok(id)) => film.on(&who, |who| emit::move_effect(who, engine::MoveId(id))),
            _ => film.not_a(&effect, "a move effect of the game"),
        }
    }
    true
}

/// `flash white at 12`: that far into what follows; `flash red 0.5s`. Else the engine's.
fn flash_said(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    // (`flash`, `flash red`: as the game's, quick, with its sound; `flash white 30`: the engine's.)
    if !args.has("at") && args.word("d").is_none() {
        let colour = args.word("colour").unwrap_or("white");
        let Some(shown) = engine::colour_named(colour) else {
            return false;
        };
        film.sound("flash");
        film.beat(emit::flash(shown, 12.0));
        return true;
    }
    if !args.has("at") && !args.word("d").is_some_and(|word| in_seconds(word).is_some()) {
        return false;
    }
    let mut fields = Vec::new();
    if args.has("at") {
        fields = fields!["after", args.frames("after").unwrap_or(0)];
    }
    let colour = args.word("colour").unwrap_or("white");
    fields.extend(fields!["flash", colour]);
    let length = match args.word("d") {
        Some(length) => in_seconds(length).map_or(length.to_string(), |frames| frames.to_string()),
        None => "20".to_string(),
    };
    fields.push(length.clone());
    // (A colour by its name, the frames as the engine reads them: 20 when they are no number.)
    let after = args.has("at").then(|| args.frames("after").unwrap_or(0));
    match (engine::colour_named(colour), after.map(u32::try_from).transpose()) {
        (Some(colour), Ok(after)) => {
            let flash = emit::flash(colour, length.parse().unwrap_or(20.0));
            film.beat(match after {
                Some(frames) => emit::after(frames as f32, 0.0, flash),
                None => flash,
            });
        }
        _ => film.unread(fields.join(" ")),
    }
    true
}

/// `tremble screen 3 40 weak`: the screen, less and less. Else the engine's.
fn shaken(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    if !args.has("screen") {
        return false;
    }
    let much = ["weak", "strong"].iter().find(|word| args.has(word)).and_then(|word| of(&SHAKES, word));
    let strength = args.number("strength").unwrap_or(3.0);
    let frames = args.frames("d").unwrap_or(40);
    let strength = format!("{:.1}", strength * much.unwrap_or(1.0));
    match strength.parse::<f32>() {
        Ok(strength) => film.beat(emit::quake(strength, frames as f32)),
        Err(_) => film.not_a(&strength, "a strength"),
    }
    true
}

/// How hard `shake` shakes the view, by its word (none: in between).
const SHAKING: [(&str, f32); 2] = [("soft", 1.0), ("hard", 4.0)];

/// `shake [soft|hard] [2s]`: the view shakes up and down as the game's
/// ground does, until `shake off` (or for so long); the film goes on.
fn shaking(film: &mut Film, line: &Line) {
    let args = &line.args;
    if args.has("off") {
        film.beat(emit::shake(2, 0.0, None));
        return;
    }
    let strength = ["soft", "hard"].iter().find(|word| args.has(word)).and_then(|word| of(&SHAKING, word)).unwrap_or(2.0);
    film.beat(emit::shake(2, strength, args.frames("d").map(|frames| frames as f32)));
}
