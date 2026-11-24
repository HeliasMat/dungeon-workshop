//! The days of the game: the guild, its dungeons, its missions.

use engine::{Pose, Who};
use glam::Vec2;

use film_core::emit;
use film_core::film::Film;
use film_core::spots::formations::on_rang;
use film_core::verbs::{Line, Word};
use film_core::words::plain;
use film_speech::said::told_or_logged;

pub const WORDS: &[Word] = &[
    Word::new("sleep", &["who:pokemon"], "pose sleep, status sleep").does(dort),
    Word::new("morning_call", &["leader:pokemon *who"], "shout, quake, status off, jump; then rank")
        .family("Guild")
        .ds_work("Sky's own morning at the guild, its routine called")
        .word_first()
        .does(appel_du_matin),
    Word::new("dinner", &["*who"], "seats, lookat, anim Eat, emote")
        .family("Guild")
        .ds_work("Sky's own dinner at the guild, its routine called")
        .word_first()
        .does(dinner),
    Word::new("rank", &["*who", "team:word rank:thing..."], "find, goes, turn down (they line up); a team and a rank: jingle, the game's sentence")
        .family("Guild")
        .ds_work("the team's rank, a game variable")
        .word_first()
        .maybe(rank),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("rituals", WORDS);

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

/// A fisher turned to us and kept so while he sleeps: his sleep is painted
/// from the front only (his walk, his cast and his pull all round).
fn faces_us(film: &mut Film, who: &str) {
    // (The engine turns him to the first way he is painted in by itself.)
    if film_core::names::fisher(film, who) {
        film.on(who, |who| emit::views(who, vec![engine::Direction::Down]));
    }
}

fn dort(film: &mut Film, line: &Line) {
    let args = &line.args;
    let words: Vec<String> = args.words("who").iter().map(|one| plain(one)).collect();
    faces_us(film, &words[0]);
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
        let team = film_core::screen::capitalized(&plain(team));
        let rank = args.words("rank").iter().map(|word| film_core::screen::capitalized(&plain(word))).collect::<Vec<_>>().join(" ");
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
