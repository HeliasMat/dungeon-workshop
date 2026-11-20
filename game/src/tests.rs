//! The example game played with nobody at the keys, and saved. (Those
//! that need the extracted `data/` are ignored: `cargo test -- --ignored`.)

use std::path::{Path, PathBuf};

use super::{Game, Mode};

/// The folder of the game's files, if they are there.
fn data() -> Option<String> {
    let data = crate::testing::data_folder();
    shell::data::read(&data, "dungeons.bin").map(|_| data)
}

/// The example project's folder.
fn scenes() -> PathBuf {
    crate::testing::root().join("tests/games/test_village")
}

/// Whether those lines are in what the game did, in that order.
fn in_order(log: &[String], wanted: &[&str]) -> bool {
    let mut from = 0;
    for one in wanted {
        match log[from..].iter().position(|line| line == one) {
            Some(at) => from += at + 1,
            None => return false,
        }
    }
    true
}
#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_example_game_plays_to_its_end_with_nobody_at_the_keys() {
    let data = data().expect("the extracted data/");
    let mut game = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    game.play_alone(200_000);
    let log = game.log.clone();
    assert!(game.rested || game.over, "it ends: {log:?} {:?}", game.said);
    let order = [
        "chapter 1",
        "film arrival",
        "film arrival ended",
        "go test_market",
        "talk pidgeot",
        "dungeon test_forest",
        "dungeon won",
        "chapter 2",
        "film back_home",
        "talk kecleon",
        "film thanks",
        "the end",
    ];
    assert!(in_order(&log, &order), "in order {order:?}: {log:?}");
    assert!(game.story.flags.contains("fireflies_back"), "{:?}", game.story);
    assert_eq!(game.story.place.as_deref(), Some("test_market"));
}

/// A copy of the example game in a folder of its own, to change its films.
fn copied(to: &Path) {
    let from = scenes();
    for folder in ["films", "decors", "saves"] {
        std::fs::create_dir_all(to.join(folder)).expect("a folder");
    }
    std::fs::copy(from.join("test_village.game"), to.join("test_village.game")).expect("the game");
    let text = std::fs::read_to_string(from.join("test_village.game")).expect("the game");
    for word in text.lines().filter_map(|line| line.trim().strip_prefix("play ")) {
        std::fs::copy(from.join("films").join(format!("{word}.film")), to.join("films").join(format!("{word}.film"))).expect("a film");
    }
    for word in text.lines().filter_map(|line| line.trim().strip_prefix("go ")).filter_map(|said| said.split_whitespace().next()) {
        std::fs::copy(from.join("decors").join(format!("{word}.decor")), to.join("decors").join(format!("{word}.decor"))).expect("a place");
    }
    let home = text.lines().find_map(|line| line.strip_prefix("home: ")).and_then(|said| said.split_whitespace().next()).unwrap_or_default().to_string();
    if !home.is_empty() {
        std::fs::copy(from.join("decors").join(format!("{home}.decor")), to.join("decors").join(format!("{home}.decor"))).expect("home");
    }
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_game_saved_in_chapter_two_is_taken_up_there_though_a_film_changed() {
    let data = data().expect("the extracted data/");
    let folder = std::env::temp_dir().join(format!("dungeon_workshop_save_{}", std::process::id()));
    _ = std::fs::remove_dir_all(&folder);
    copied(&folder);
    let path = folder.join("test_village.game");
    let mut game = Game::open(&crate::testing::context_on(&data), &path, None).expect("the game opens");
    game.saves = folder.join("saves");
    // (Nobody at the keys up to the second chapter's film; then someone, who stands in the market.)
    game.nobody = true;
    game.skip_title();
    for _ in 0..200_000 {
        if game.cutscene.as_ref().is_some_and(|(_, word)| word == "back_home") {
            break;
        }
        game.tick(shell::Keys::default(), 1.0);
    }
    game.nobody = false;
    for _ in 0..20_000 {
        if game.mode() == Mode::Roam && game.walked_at().is_some() {
            break;
        }
        game.tick(shell::Keys::default(), 1.0);
    }
    assert_eq!((game.story.chapter, game.mode()), (2, Mode::Roam), "{:?} {:?}", game.log, game.said);
    let file = game.save().expect("saved");
    let before = game.story.clone();
    // (A film changed: a line more, one less.)
    let film = folder.join("films").join("back_home.film");
    let text = std::fs::read_to_string(&film).expect("the film");
    std::fs::write(&film, text.replace("kecleon: My shop is yours.", "kecleon (happy): Thank you!\nkecleon: My shop is yours.")).expect("changed");
    let mut again = Game::open(&crate::testing::context_on(&data), &path, None).expect("the game opens again");
    let saved = std::fs::read_to_string(&file).expect("the save");
    again.title = None;
    again.load(&saved);
    for _ in 0..120 {
        again.tick(shell::Keys::default(), 1.0);
    }
    assert_eq!(again.story.chapter, 2);
    assert_eq!(again.story.flags, before.flags);
    assert_eq!(again.story.place, before.place);
    assert_eq!(again.mode(), Mode::Roam, "{:?} {:?}", again.log, again.said);
    assert_eq!(again.story.entrance, before.entrance);
    assert!(
        again.walked_at().is_some(),
        "the player walks the place again: {:?} {:?} {:?}",
        again.said,
        again.place.as_ref().map(|place| (place.film.is_some(), place.text.clone())),
        again.film().map(|film| (film.over, film.played(), film.director().scene_name().to_string()))
    );
    _ = std::fs::remove_dir_all(&folder);
}

/// The game, played until its place's scene stands (nobody at the keys of the films).
fn standing(game: &mut Game) {
    game.tick(shell::Keys::default(), 1.0);
    for _ in 0..600 {
        if game.mode() == Mode::Roam && game.walked_at().is_some() {
            return;
        }
        game.tick(shell::Keys::default(), 1.0);
    }
    panic!("no place stands: {:?} {:?}", game.log, game.said);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_place_is_walked_between_its_ways_its_lines_change_with_the_story_and_its_shop_opens() {
    let data = data().expect("the extracted data/");
    let mut game = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    (game.title, game.story.chapter) = (None, 1);
    game.enter("test_market", super::roam::Arrival::Entrance(Some("south".to_string())), true);
    standing(&mut game);
    let text = |game: &Game| game.place.as_ref().map(|place| place.text.clone()).unwrap_or_default();
    assert!(text(&game).contains("kecleon: Said if not fireflies_back."), "{}", text(&game));
    assert!(text(&game).contains("pidgeot") && text(&game).contains("game talk kecleon"), "{}", text(&game));
    assert!(text(&game).contains("way 552 216 32 48 go test_harbour left"), "{}", text(&game));
    // (A flag up: what Kecleon says changes, Pidgeot is gone.)
    game.story.flags.insert("fireflies_back".to_string());
    game.settled = false;
    standing(&mut game);
    assert!(text(&game).contains("Said if fireflies_back.") && !text(&game).contains("pidgeot"), "{}", text(&game));
    // (Talked to: its shop.)
    game.request(super::GameRequest::Ask(super::GameAsk::Talk("kecleon".to_string())));
    game.tick(shell::Keys::default(), 1.0);
    assert!(matches!(game.screen, Some((crate::game_screens::GameScreen::Service { .. }, _))), "the shop opens: {:?}", game.log);
    for _ in 0..12 {
        game.tick(shell::Keys { escape: true, ..shell::Keys::default() }, 1.0);
        game.tick(shell::Keys::default(), 1.0);
    }
    assert!(game.screen.is_none(), "the shop left");
    // (The way to the harbour taken: there, by its west, the way back.)
    game.request(super::GameRequest::Ask(super::GameAsk::Go { place: "test_harbour".to_string(), entrance: Some("left".to_string()) }));
    standing(&mut game);
    assert_eq!(game.story.place.as_deref(), Some("test_harbour"));
    assert!(text(&game).contains("go test_market east") && text(&game).contains("game talk krabby"), "{}", text(&game));
    // (Krabby's gift: talked to, the apple in the bag, his line changed.)
    game.request(super::GameRequest::Ask(super::GameAsk::Talk("krabby".to_string())));
    standing(&mut game);
    assert!(game.story.flags.contains("got_apple"), "{:?}", game.story);
    assert!(text(&game).contains("Said if got_apple."), "{}", text(&game));
    let apples = game.playthrough.as_ref().map(crate::playthrough::Playthrough::carried).unwrap_or_default();
    assert!(apples.items.iter().any(|(name, _)| name == "Golden Apple"), "{apples:?}");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_team_ko_in_a_dungeon_comes_back_home_told_what_it_lost() {
    let data = data().expect("the extracted data/");
    let mut game = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    (game.title, game.story.chapter) = (None, 1);
    game.enter("test_harbour", super::roam::Arrival::Entrance(None), true);
    standing(&mut game);
    game.dive_told(false, String::new());
    standing(&mut game);
    assert_eq!((game.story.place.as_deref(), game.story.won), (Some("test_market"), false), "home, as the game's file says");
    let text = game.place.as_ref().map(|place| place.text.clone()).unwrap_or_default();
    assert!(text.contains("narrator: The team fainted"), "{text}");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_film_reads_the_game_s_flags_and_gives_some_back() {
    let data = data().expect("the extracted data/");
    let mut game = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    (game.title, game.nobody) = (None, true);
    game.story.flags.insert("got_apple".to_string());
    game.play_film("thanks");
    assert!(game.cutscene.is_some(), "{:?}", game.said);
    let mut read = None;
    for _ in 0..20_000 {
        if let Some((film, _)) = &game.cutscene {
            read = read.or(film.number("got_apple").filter(|_| film.played() > 30.0));
        } else {
            break;
        }
        game.tick(shell::Keys::default(), 1.0);
    }
    assert_eq!(read, Some(1.0), "the game's flag, not the film's `keep`");
    assert!(game.cutscene.is_none(), "the film ended");
    assert!(game.story.flags.contains("stayed"), "the choice given back: {:?}", game.story.flags);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_game_begins_on_its_title_screen() {
    let data = data().expect("the extracted data/");
    let mut game = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    game.title.as_mut().expect("a title screen").saved = Some("event 2\nvars=flag.fireflies_back:1\ntext place=test_market".to_string());
    assert_eq!(game.mode(), Mode::Title);
    // (Continue: the save taken up.)
    game.tick(shell::Keys { enter: true, ..shell::Keys::default() }, 1.0);
    assert_eq!((game.story.chapter, game.story.place.as_deref()), (2, Some("test_market")));
    assert!(game.story.flags.contains("fireflies_back"));
    // (Quit: over.)
    let mut left = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    left.title.as_mut().expect("a title screen").saved = None;
    left.tick(shell::Keys { down: true, ..shell::Keys::default() }, 1.0);
    left.tick(shell::Keys { enter: true, ..shell::Keys::default() }, 1.0);
    assert!(left.over);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_recruit_of_the_game_s_joins_the_team_and_is_saved() {
    let data = data().expect("the extracted data/");
    let mut game = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    game.title = None;
    game.make_team();
    let species = crate::testing::context().tables().monsters.get("bulbasaur").and_then(|(number, _)| number.parse::<u16>().ok()).expect("bulbasaur");
    game.act(crate::game_file::Action::Recruit("bulbasaur".to_string()));
    let recruits =
        |game: &Game| game.playthrough.as_ref().and_then(|playthrough| playthrough.run()).map(|run| run.team().progress.recruits.clone()).unwrap_or_default();
    assert!(recruits(&game).iter().any(|recruit| recruit.species == species), "{:?} {:?}", game.log, game.said);
    assert!(game.log.iter().any(|line| line == "recruit bulbasaur"), "{:?}", game.log);
    // (Saved, taken up again: still there.)
    let saved = game.saved_text();
    let mut again = Game::open(&crate::testing::context_on(&data), &scenes().join("test_village.game"), None).expect("the game opens");
    again.title = None;
    again.load(&saved);
    assert!(recruits(&again).iter().any(|recruit| recruit.species == species), "{saved}");
}

#[test]
fn a_finished_game_shows_its_end_then_its_title_screen() {
    let mut game = Game::bare(&crate::testing::context_on("no data here"), crate::testing::library(), None, None);
    game.saves = std::env::temp_dir().join(format!("dw_end_{}", std::process::id()));
    game.project = Some(crate::game_file::GameFile::read("title: Little\n\nchapter 1\n  finish\nend\n"));
    game.new_game();
    game.tick(shell::Keys::default(), 1.0);
    assert_eq!((game.mode(), game.over), (Mode::End, false), "{:?}", game.log);
    assert!(game.log.iter().any(|line| line == "the end"), "{:?}", game.log);
    // (It waits for a key; then the title screen, its game as new.)
    game.tick(shell::Keys::default(), 1.0);
    assert_eq!(game.mode(), Mode::End);
    game.tick(shell::Keys { enter: true, ..shell::Keys::default() }, 1.0);
    assert_eq!(game.mode(), Mode::Title);
    assert_eq!(game.title.as_ref().map(|title| title.name.as_str()), Some("Little"));
    // (Nobody at the keys: over at once, as headless says it.)
    let mut alone = Game::bare(&crate::testing::context_on("no data here"), crate::testing::library(), None, None);
    alone.project = Some(crate::game_file::GameFile::read("title: Little\n\nchapter 1\n  finish\nend\n"));
    alone.play_alone(10);
    assert!(alone.over && alone.log.iter().any(|line| line == "the end"), "{:?}", alone.log);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_place_not_built_yet_is_built_on_demand_into_a_cache() {
    let data = data().expect("the extracted data/");
    let folder = std::env::temp_dir().join(format!("dw_built_{}", std::process::id()));
    _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("decors")).expect("a folder");
    let text = std::fs::read_to_string(crate::testing::root().join("tests/games/test_village/decors/test_market.decor")).expect("a decor");
    let text = text.replace("decor: test_market", "decor: dw_never_built").replace("level 477\n", "").replace("level 477\r\n", "");
    let file = folder.join("decors/dw_never_built.decor");
    std::fs::write(&file, text).expect("written");
    let cache = folder.join("built");
    let level = crate::roam::build_decor(&crate::testing::library(), &data, &file, &cache).expect("built");
    let listed = std::fs::read_to_string(cache.join("decors.txt")).expect("its line");
    assert!(listed.lines().any(|line| line.starts_with(&format!("dw_never_built\t{level}\t"))), "{listed}");
    assert!(cache.join("grounds/pictures").join(format!("{level}_f0.png")).is_file(), "its picture");
    _ = std::fs::remove_dir_all(&folder);
}
