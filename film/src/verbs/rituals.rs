//! The days of the game: the guild, its dungeons, its missions.

use engine::{Pose, Who};
use glam::Vec2;

use super::said::told_or_logged;
use super::{Line, Word};
use crate::emit;
use crate::film::Film;
use crate::spots::formations::on_rang;
use crate::words::plain;

pub const WORDS: &[Word] = &[
    Word::new("sleep", &["who:pokemon"], "pose sleep, status sleep").does(dort),
    Word::new("morning_call", &["leader:pokemon *who"], "shout, quake, status off, jump; then rank").does(appel_du_matin),
    Word::new("dinner", &["*who"], "seats, lookat, anim Eat, emote").does(dinner),
    Word::new("rank", &["*who", "team:word rank:thing..."], "find, goes, turn down (they line up); a team and a rank: jingle, the game's sentence").maybe(rank),
];

/// Its words, a family of the language (`Lexicon`).
pub const FAMILY: crate::verbs::Family = crate::verbs::Family::new("rituals", WORDS);

fn meal(film: &mut Film, who: &[String]) {
    // (Dinner is called; each goes to a place where the game's scenes seat
    // someone, then the plates are emptied at their own pace.)
    film.sound("mealtime");
    let spots = film.tables.usual_spots.get(&film.level.to_string()).cloned().unwrap_or_default();
    let seats: Vec<(i32, i32)> =
        spots.into_iter().filter(|(x, y)| film.rows.as_ref().is_none_or(|rows| !rows.blocked(glam::Vec2::new(*x as f32, *y as f32), 0))).collect();
    let var = film.found_var();
    // (A crowd as each of its members; one who is no one: told.)
    let members: Vec<String> = who.iter().flat_map(|one| film.everyone(one)).collect();
    match members.iter().map(|one| film.other(one)).collect::<Option<Vec<Who>>>() {
        Some(seated) => film.beat(emit::seats(var.clone(), seated, seats.iter().map(|(x, y)| Vec2::new(*x as f32, *y as f32)).collect())),
        None => film.not_a(who.join(" "), "Pokémon of the scene"),
    }
    film.beat(emit::wait_all());
    for one in who {
        film.on(one, |who| emit::look_at_point(who, emit::found_at(&var)));
    }
    for (place, one) in who.iter().enumerate() {
        let eat = scenes::vocabulary::anim::EAT;
        film.on(one, |who| emit::after(place as f32 * 12.0, 0.0, emit::anim(who, eat, false, false)));
    }
    film.wait(90 + who.len() as i64 * 12);
    for one in who {
        film.on(one, |who| emit::pose(who, Pose::Idle));
        film.emote(one, "joy", Some(40.0));
    }
    film.wait(40);
}

fn dort(film: &mut Film, line: &Line) {
    let args = &line.args;
    let words: Vec<String> = args.words("who").iter().map(|one| plain(one)).collect();
    super::fishing::faces_us(film, &words[0]);
    film.on(&words[0], |who| emit::pose(who, Pose::Sleep));
    film.on(&words[0], |who| emit::status(who, emit::status_icon("sleep")));
}

fn appel_du_matin(film: &mut Film, line: &Line) {
    let args = &line.args;
    let words: Vec<String> = args.words("who").iter().map(|one| plain(one)).collect();
    // (`morning_call loudred pikachu eevee`: the shout shakes the
    // room, all up, in a row.)
    let chief = plain(args.word("leader").unwrap_or_default());
    film.emote(&chief, "anger", Some(40.0));
    film.sound("alert");
    film.shout(&chief, "WAKE UP!!!");
    // (It goes on, less and less, while they get up.)
    film.beat(emit::quake(5.0, 120.0));
    film.wait(60);
    for one in &words {
        film.on(one, |who| emit::status(who, None));
        film.on(one, |who| emit::pose(who, Pose::Idle));
        film.jump(one, 10.0, 10.0);
        film.emote(one, "exclaim", Some(40.0));
    }
    film.sound("startle");
    film.wait(50);
    on_rang(film, &words);
}

fn dinner(film: &mut Film, line: &Line) {
    let args = &line.args;
    let words: Vec<String> = args.words("who").iter().map(|one| plain(one)).collect();
    meal(film, &words);
}

fn rank(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    // (`rank pokepals gold`: the team rises to the game's rank.)
    if let Some(team) = args.word("team") {
        let team = crate::screen::capitalized(&plain(team));
        let rank = args.words("rank").iter().map(|word| crate::screen::capitalized(&plain(word))).collect::<Vec<_>>().join(" ");
        film.beat(emit::jingle(7));
        told_or_logged(film, &format!("Team {team} reached the [CS:Y]{rank} Rank[CR]!"));
        return true;
    }
    let words: Vec<String> = args.words("who").iter().map(|one| plain(one)).collect();
    if !(words.len() >= 2) {
        return false;
    }
    on_rang(film, &words);
    true
}
