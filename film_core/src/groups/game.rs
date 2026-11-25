//! What a film tells the game it plays in, and the words only the game
//! writes. A film is a cutscene: it reads the game's flags (`keep met =
//! false`, then `if met`) and sets some (`set met`, `clear met`); the game
//! keeps them once it ends.
//!
//! The places one walks about are scenes the game makes of its decors
//! (`game::roam`, compiled by `film::compile_place`): there only,
//! `player` gives the hero to the player (and who follows it), `dialogue` …
//! `end` what someone says when talked to, `explore` hands over (the scene
//! waits for the player, see `engine::FlowBeat::Roam`), `game …` tells
//! the game and holds, `way …` tells it when the player walks near.
//!
//! The words a film once played the game with (`passage`, `dungeon`,
//! `screen`, `map`, `village`, `earn`, `pay`, `stow`, `take_out`) are the
//! game's now: each says what to write instead.

use engine::{Beat, Cam, Course, FlowBeat, GameAsk, PlayBeat, Who};
use glam::Vec2;

use crate::emit;
use crate::film::Film;
use crate::verbs::{Line, Word};
use crate::words::plain;

pub const WORDS: &[Word] = &[
    Word::new("set", &["flag:word off", "flag:word value:number", "flag:word"], "the game's flag is up (1): the game keeps it once the film ends; in a choice's option, the choice told to the game: set").family("Structure").ds_direct("$LOCALn = value (the scene's own; lost once it ends)").does(flag_up),
    Word::new(
        "player",
        &["who:pokemon [followed by friend:pokemon]"],
        "only in a place's scene the game makes: the player steers him with the arrows (Enter: talk, Shift: run), the camera follows him, the friend follows him; M or X: the game's menu: control, collisions, camera, follow",
    ).family("Advanced").ds_none("a cutscene has no one the player walks").word_first()
    .maybe(player)
    .instant(),
    Word::new(
        "dialogue",
        &["who:pokemon"],
        "only in a place's scene the game makes: dialogue who … end, what plays when the player talks to him (Enter in front of him), then exploring goes on: ontalk, label, return",
    ).family("Advanced").ds_none("a cutscene has no one the player talks to").word_first()
    .maybe(talk)
    .instant(),
    Word::new("explore", &[""], "only in a place's scene the game makes: the player explores, the scene waits until he talks to someone or takes a way: roam").family("Advanced").ds_none("a cutscene has no exploring").does(roam),
    Word::new("game", &["words..."], "only in a place's scene the game makes: the scene holds and the game is told (game talk kecleon, game menu)").family("Advanced").ds_none("the game's own ground play: a cutscene has none").maybe(told_to_the_game),
    Word::new(
        "way",
        &["x:number y:number width:number height:number words..."],
        "only in a place's scene the game makes: walking into that rectangle (its corner x y, its width and height, as a decor's exit or zone) tells the game (way 600 240 32 16 go port east): door",
    ).family("Advanced").ds_none("the game's own ground play: a cutscene has none")
    .maybe(way_of_the_game)
    .instant(),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: crate::verbs::Group = crate::verbs::Group::new("game", WORDS);

/// What to write instead of a word of the game's, by the word.
fn instead(word: &str) -> &'static str {
    match word {
        "player" | "dialogue" | "explore" => {
            "a place one walks about is a decor now, played by a game: `resident kecleon 300 220`, `says …`, `talk …`, `exit … to <place>` after its [life], and `go <place>` in a chapter of a .game"
        }
        "passage" => "`exit x y width height to <place> [entrance]` after the [life] of the place's decor",
        "dungeon" => "`dungeon <name>` in a chapter of a .game, then `if won`",
        "screen" => "`talk shop` (market, bank, storage, jobs, recruits, appraise) under a resident of a decor, or `shop <screen>` in a .game",
        "village" => "`home: <place> [entrance]` in the head of the .game",
        "map" => "`open <place|dungeon>` then `map` in a .game",
        "earn" | "pay" => "`money 200` (`money -50`) in a .game",
        "stow" | "take_out" => "`give [n] <item>` (`take [n] <item>`) in a .game or a decor's `on … end`",
        _ => "an action of a .game",
    }
}

/// A word of the game's in a film: said wrong, with what to write instead.
fn refused(film: &mut Film, line: &Line) {
    let word = line.verb.as_str();
    film.wrong(
        line.number,
        format!("“{word}” is the game's now, no longer a film's (a film is a cutscene): write {} (film/FILMS.md, “A game”)", instead(word)),
    );
}

/// A word only the game writes, in a film: said wrong; whether it was.
fn the_game_s_own(film: &mut Film, line: &Line) -> bool {
    if film.game.made_by_the_game {
        return false;
    }
    match line.verb.as_str() {
        "game" | "way" => film.wrong(line.number, format!("“{}” is only written by the game, in the scenes it makes of its places", line.verb)),
        _ => refused(film, line),
    }
    true
}

/// `set met_kecleon`: the game's flag up.
fn flag_up(film: &mut Film, line: &Line) {
    // (`set met_kecleon off`: down again; `set visits 3`: that value.)
    let value = match line.args.number("value") {
        Some(value) => value as f64,
        None if line.args.has("off") => 0.0,
        None => 1.0,
    };
    if let Some(flag) = line.args.word("flag") {
        film.beat(emit::set(flag, emit::number(value)));
    }
}

/// `player hero [followed by partner]`: the player's, followed (the game puts them first).
fn player(film: &mut Film, line: &Line) -> bool {
    if the_game_s_own(film, line) {
        return true;
    }
    let Some(name) = line.args.word("who").map(plain) else {
        return false;
    };
    let Some(hero) = film.actor(&name).and_then(|code| Who::parse(&code)) else {
        return false;
    };
    let friend = line.args.word("friend").map(plain).and_then(|friend| film.actor(&friend)).and_then(|code| Who::parse(&code));
    film.beat(emit::collisions(true));
    film.beat(Beat::Play(PlayBeat::Control(Some(hero))));
    film.beat(emit::camera(Some(Cam::Who(hero))));
    if let Some(friend) = friend {
        film.beat(emit::follow(friend, Some(hero), None));
    }
    film.game.player = Some(name);
    menu(film);
    true
}

/// The menu, M or X while exploring: the game's own, the scene holding.
fn menu(film: &mut Film) {
    film.game.menus += 1;
    let (label, past) = (format!("game_menu{}", film.game.menus), format!("game_menu{}_past", film.game.menus));
    film.beat(emit::goto(past.clone()));
    film.beat(emit::label(label.clone()));
    film.beat(Beat::Course(Course::Game(GameAsk::Menu)));
    film.beat(emit::back());
    film.beat(emit::label(past));
    film.beat(Beat::Flow(FlowBeat::OnMenu(label)));
}

/// `dialogue mimi` … `end`: what is played when the player talks to mimi.
fn talk(film: &mut Film, line: &Line) -> bool {
    if the_game_s_own(film, line) {
        return true;
    }
    let Some(name) = line.args.word("who").map(plain) else {
        return false;
    };
    let Some(who) = film.actor(&name).and_then(|code| Who::parse(&code)) else {
        return false;
    };
    film.game.talks += 1;
    let (label, past) = (format!("dialogue{}", film.game.talks), format!("dialogue{}_past", film.game.talks));
    film.beat(emit::goto(past.clone()));
    film.beat(emit::label(label.clone()));
    // (Who is talked to turns to the player.)
    if let Some(hero) = film.game.player.clone().and_then(|hero| film.actor(&hero)).and_then(|code| Who::parse(&code)) {
        film.beat(emit::face(who, hero));
    }
    // (Its lines follow, up to its `end`.)
    film.game.open.push((who, label, past));
    film.timing.opened.push(|film, _| {
        if let Some((who, label, past)) = film.game.open.pop() {
            film.beat(emit::back());
            film.beat(emit::label(past));
            film.beat(Beat::Flow(FlowBeat::OnTalk { who, label }));
        }
    });
    true
}

/// `explore`: the player plays.
fn roam(film: &mut Film, line: &Line) {
    if !the_game_s_own(film, line) {
        film.beat(Beat::Flow(FlowBeat::Roam));
    }
}

/// `game talk kecleon`: the scene holds, the game is told.
fn told_to_the_game(film: &mut Film, line: &Line) -> bool {
    if the_game_s_own(film, line) {
        return true;
    }
    let words = line.args.words("words");
    if words.is_empty() {
        return false;
    }
    // (What it asks, typed now: the game takes it as it is.)
    match GameAsk::of_words(words) {
        Some(ask) => film.beat(Beat::Course(Course::Game(ask))),
        None => return false,
    }
    true
}

/// `way 600 240 32 16 go port east`: walked into that rectangle, the game is
/// told (`game go port east`).
fn way_of_the_game(film: &mut Film, line: &Line) -> bool {
    if the_game_s_own(film, line) {
        return true;
    }
    let (Some(x), Some(y), Some(wide), Some(high)) = (line.args.number("x"), line.args.number("y"), line.args.number("width"), line.args.number("height"))
    else {
        return false;
    };
    let (wide, high) = (wide.max(1.0), high.max(1.0));
    let (x, y, radius) = (x + wide / 2.0, y + high / 2.0, wide.max(high) / 2.0);
    let words = line.args.words("words");
    if words.is_empty() {
        return false;
    }
    film.beat(Beat::Flow(FlowBeat::Door { scene: format!("game {}", words.join(" ")), arrive: 0.0, at: Vec2::new(x, y).into(), radius, wide, high }));
    true
}
