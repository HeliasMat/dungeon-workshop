//! The sky and the light: the weather, the hour (time stopping: recipes,
//! `film/recipes/sky.film`). Nothing lays
//! a light on a scene but what is written here.

use super::{Line, Word};
use crate::emit;
use crate::film::Film;
use crate::words::{DAY, of, plain};

pub const WORDS: &[Word] = &[
    Word::new("daytime", &["moment:word [stars] [d:duration]"], "light (morning, day, evening, night; dawn); tint (18h30); stars: the stars twinkling (weather sparkles)").maybe(hour_said),
    Word::new(
        "status",
        &["who:pokemon off", "who:pokemon stat:word up|down", "who:pokemon kind:word"],
        "status: the game's status icon over him (sleep, burn, poison, frozen, confused, cringe…), off: none; a stat up or down: the game's own arrows (effect)",
    )
    .does(status_said),
    Word::new(
        "weather",
        &["off", "kind:rain|snow|sand|fog|hail|sun [heavy]"],
        "weather: the game's weather over the scene (heavy: under a storm's light), off: fair again",
    )
    .maybe(weather_said),
];

/// The light of each moment of the day.
const HOURS: [(&str, &str); 5] = [("dawn", "dawn"), ("morning", "dawn"), ("day", "normal"), ("evening", "evening"), ("night", "night")];
/// The frames the light takes to change to an hour, by default.
const TOWARD_AN_HOUR: i32 = 120;

/// The light of an hour of the day: red, green, blue, how much.
fn light_at(hour: f32) -> [i32; 4] {
    for pair in DAY.windows(2) {
        let ((start, colour, much), (end, next, next_much)) = (pair[0], pair[1]);
        if (start..=end).contains(&hour) {
            let done = (hour - start) / (end - start);
            let mix = |one: f32, other: f32| (one + (other - one) * done).round() as i32;
            return [mix(colour[0], next[0]), mix(colour[1], next[1]), mix(colour[2], next[2]), mix(much, next_much)];
        }
    }
    [DAY[0].1[0] as i32, DAY[0].1[1] as i32, DAY[0].1[2] as i32, DAY[0].2 as i32]
}

/// `18h30`, `6h` (or `18:30`): the hour, in hours.
fn an_hour(word: &str) -> Option<f32> {
    let (hours, minutes) = word.split_once('h').or_else(|| word.split_once(':'))?;
    let hours: f32 = hours.parse().ok()?;
    let minutes: f32 = if minutes.is_empty() { 0.0 } else { minutes.parse().ok()? };
    Some((hours + minutes / 60.0).min(24.0))
}

/// The words a status is also said by, and the game's icon each is.
const STATUS_WORDS: [(&str, &str); 6] =
    [("asleep", "sleep"), ("burned", "burn"), ("poisoned", "poison"), ("freeze", "frozen"), ("confuse", "confused"), ("afraid", "cringe")];

/// The game's effects that show a stat going up or down, one after the
/// other, as its moves show them (`scenes/src/fight/stage`).
fn stat_sights(stat: &str, up: bool) -> Option<&'static [u16]> {
    Some(match (stat, up) {
        ("attack" | "sp_attack", true) => &[425, 2],
        ("attack" | "sp_attack", false) => &[404, 403],
        ("defense" | "sp_defense", true) => &[2, 400],
        ("defense" | "sp_defense", false) => &[399, 401],
        ("accuracy" | "evasion", true) => &[396, 13],
        ("accuracy" | "evasion", false) => &[397, 14],
        ("speed", true) => &[395],
        ("speed", false) => &[2],
        _ => return None,
    })
}

/// `pikachu status sleep`: the game's status icon over him, as in a
/// dungeon; `pikachu status off`: none; `pikachu status attack up`: the
/// game's own arrows, a stat rising (or falling).
fn status_said(film: &mut Film, line: &Line) {
    let args = &line.args;
    let who = super::who_said(line);
    if args.has("up") || args.has("down") {
        let stat = args.word("stat").map(plain).unwrap_or_default();
        match stat_sights(&stat, args.has("up")) {
            Some(sights) => {
                for (at, sight) in sights.iter().enumerate() {
                    let id = engine::EmoteId(*sight);
                    film.on(&who, |one| emit::after(at as f32 * 30.0, 0.0, emit::emote(one, id, None)));
                }
                film.wait(30 * sights.len() as i64);
            }
            None => film.not_a(&stat, "a stat (attack, defense, sp_attack, sp_defense, speed, accuracy, evasion)"),
        }
        return;
    }
    if args.has("off") {
        film.on(&who, |one| emit::status(one, None));
        return;
    }
    let kind = args.word("kind").map(plain).unwrap_or_default();
    let named = of(&STATUS_WORDS, &kind).unwrap_or(&kind).to_string();
    match emit::status_icon(&named) {
        Some(icon) => film.on(&who, |one| emit::status(one, Some(icon))),
        None => film.unknown(
            line.number,
            "unknown status",
            &kind,
            scenes::vocabulary::POKEMON.status_icons.iter().copied(),
            "sleep, burn, poison, frozen, confused, cringe…",
        ),
    }
}

/// The weathers of `weather`, and the game's sky each is.
const WEATHER_SKIES: [(&str, &str); 6] = [("rain", "rain"), ("snow", "snow"), ("sand", "sand"), ("fog", "mist"), ("hail", "hail"), ("sun", "sun")];

/// `weather rain [heavy]`: the game's weather over the scene, under a
/// storm's light if heavy; `weather off`: fair again, in the scene's hour's
/// light (`daytime`).
/// `weather a rain`: brought by someone, his move, then the sky.
fn weather_said(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    if args.has("off") {
        film.beat(emit::weather(engine::Weather::Clear));
        film.beat(emit::tint(film.staging.daylight(), 60.0));
        return true;
    }
    if let Some(sky) = of(&WEATHER_SKIES, &args.word("kind").map(plain).unwrap_or_default()) {
        if let Some(weather) = engine::weather_named(sky) {
            film.beat(emit::weather(weather));
        }
        if args.has("heavy") {
            film.beat(emit::tint(emit::light_of(engine::Daylight::Storm), 60.0));
        }
        return true;
    }
    false
}

/// `daytime evening [2s]`: the light of a time of day; `daytime 18h30 [time]`:
/// written, so it is meant; `daytime night stars`: and the stars twinkle.
fn hour_said(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    let moment = args.word("moment").map(plain).unwrap_or_default();
    if args.has("stars")
        && (of(&HOURS, &moment).is_some() || an_hour(&moment).is_some())
        && let Some(stars) = engine::weather_named("sparkles")
    {
        film.beat(emit::weather(stars));
    }
    if let Some(light) = of(&HOURS, &moment) {
        // (Its time as written, `2s` or `40i`; 2 seconds untold.)
        let frames = args.word("d").and_then(crate::words::frames_of).unwrap_or(120) as f32;
        match engine::light_named(light) {
            Some(colour) => {
                film.staging.daylight = Some(colour);
                film.beat(emit::tint(colour, frames));
            }
            None => film.not_a(light, "a light of the game"),
        }
    } else if let Some(hour) = an_hour(&moment) {
        let [red, green, blue, much] = light_at(hour);
        let colour = [red as f32, green as f32, blue as f32, much as f32];
        film.staging.daylight = Some(colour);
        film.beat(emit::tint(colour, args.frames("d").unwrap_or(TOWARD_AN_HOUR) as f32));
    } else {
        return false;
    }
    true
}
