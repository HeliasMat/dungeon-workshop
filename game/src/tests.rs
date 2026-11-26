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

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn nobody_at_the_keys_walks_out_of_a_place_once_its_talks_are_done() {
    let data = data().expect("the extracted data/");
    let folder = std::env::temp_dir().join(format!("dungeon_workshop_walk_{}", std::process::id()));
    _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("decors")).expect("a folder");
    for place in ["test_market", "test_harbour"] {
        let file = format!("{place}.decor");
        std::fs::copy(scenes().join("decors").join(&file), folder.join("decors").join(&file)).expect("a place");
    }
    // (Wingull lives in the harbour: the market's talks done, its way out to the harbour is walked.)
    let game =
        "title: Walk\nteam: pikachu piplup\nhome: test_market south\n\nchapter 1 \"One\"\n  go test_market south\n  on talk wingull\n    finish\n  end\nend\n";
    std::fs::write(folder.join("walk.game"), game).expect("the game");
    let mut game = Game::open(&crate::testing::context_on(&data), &folder.join("walk.game"), None).expect("the game opens");
    game.play_alone(200_000);
    let order = ["go test_market", "talk kecleon", "go test_harbour", "talk wingull", "the end"];
    assert!(in_order(&game.log, &order), "in order {order:?}: {:?} {:?}", game.log, game.said);
    _ = std::fs::remove_dir_all(&folder);
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
    game.dive_told(crate::dive::DiveEnd::Lost, String::new(), &[]);
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

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_job_done_and_left_is_no_loss_and_plays_its_on_job_done() {
    let data = data().expect("the extracted data/");
    let rescue = crate::testing::root().join("tests/games/test_rescue/test_rescue.game");
    let mut game = Game::open(&crate::testing::context_on(&data), &rescue, None).expect("the game opens");
    (game.title, game.story.chapter) = (None, 2);
    game.enter("test_square", super::roam::Arrival::Entrance(None), true);
    standing(&mut game);
    let berries = |game: &Game| {
        let carried = game.playthrough.as_ref().map(crate::playthrough::Playthrough::carried).unwrap_or_default();
        carried.items.iter().filter(|(name, _)| name == "Oran Berry").map(|(_, count)| u32::from(*count)).sum::<u32>()
    };

    let before = berries(&game);
    game.dived = Some("test_woods".to_string());
    let job = rules::jobs::Job { kind: rules::jobs::Kind::Rescue, ..rules::jobs::Job::default() };
    game.dive_told(crate::dive::DiveEnd::JobDone, "Thank you!".to_string(), &[job]);
    standing(&mut game);
    assert_eq!((game.story.place.as_deref(), game.story.won, game.story.jobs_done), (Some("test_square"), false, 1), "{:?}", game.log);
    assert!(in_order(&game.log, &["dungeon left, job done", "job done rescue"]), "{:?}", game.log);
    assert!(!game.log.iter().any(|line| line == "dungeon lost"), "{:?}", game.log);
    assert!(berries(&game) > before, "its `on job done rescue` gave a berry: {:?}", game.log);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn back_in_the_evening_the_team_sleeps_and_wakes_to_a_new_day_and_its_on_morning() {
    let data = data().expect("the extracted data/");
    let rescue = crate::testing::root().join("tests/games/test_rescue/test_rescue.game");
    let mut game = Game::open(&crate::testing::context_on(&data), &rescue, None).expect("the game opens");
    (game.title, game.story.chapter) = (None, 2);
    game.enter("test_square", super::roam::Arrival::Entrance(None), true);
    standing(&mut game);
    let berries = |game: &Game| {
        let carried = game.playthrough.as_ref().map(crate::playthrough::Playthrough::carried).unwrap_or_default();
        carried.items.iter().filter(|(name, _)| name == "Oran Berry").map(|(_, count)| u32::from(*count)).sum::<u32>()
    };
    game.make_team();
    game.sync_team();
    assert_eq!((game.story.value("day"), game.story.value("evening")), (1.0, 0.0), "the first day, in the morning");

    game.dive_told(crate::dive::DiveEnd::Retreated, String::new(), &[]);
    standing(&mut game);
    assert_eq!((game.story.value("day"), game.story.value("evening")), (1.0, 1.0), "back in the evening");
    assert!(game.place.as_ref().is_some_and(|place| place.text.contains("daytime evening")), "the evening's light");

    let before = berries(&game);
    game.push(vec![super::game_file::Action::Sleep { save: false }]);
    standing(&mut game);
    assert_eq!((game.story.value("day"), game.story.value("evening")), (2.0, 0.0), "{:?}", game.log);
    assert!(in_order(&game.log, &["sleep", "day 2"]), "{:?}", game.log);
    assert_eq!(berries(&game), before + 1, "its `on night` then its `on morning` played: {:?}", game.log);
    assert!(game.place.as_ref().is_some_and(|place| !place.text.contains("daytime")), "daylight again");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn chimecco_s_assembly_sends_four_at_most_and_the_team_is_named() {
    use rules::services::Input::{Cancel, Confirm, Down, Mark, Up};
    let data = data().expect("the extracted data/");
    let rescue = crate::testing::root().join("tests/games/test_rescue/test_rescue.game");
    let mut game = Game::open(&crate::testing::context_on(&data), &rescue, None).expect("the game opens");
    game.make_team();
    let playthrough = game.playthrough.as_mut().expect("a game");
    for species in [10, 16, 19] {
        playthrough.recruit(species);
    }
    let run = playthrough.run_mut().expect("its rules");
    // (At Normal Rank: a bag of 16, the storage the rank's room.)
    assert_eq!(run.bag_room(), 16);
    assert!(run.storage_room() < usize::MAX, "the rank's room: {}", run.storage_room());
    let mut assembly = crate::assembly::Assembly::default();
    let press = |assembly: &mut crate::assembly::Assembly, keys: &[rules::services::Input], run: &mut rules::run::Run| {
        keys.iter().map(|&key| assembly.play(Some(key), run).1).last().unwrap_or_default()
    };
    // (The leader, the partner, the first two recruits: four, the third stays.)
    assert_eq!(run.team_size(), 4);
    let (said, ..) = press(&mut assembly, &[Down, Down, Down, Down, Mark], run);
    assert!(said.contains("full"), "{said}");
    press(&mut assembly, &[Confirm], run);
    assert_eq!(run.along(), [0, 1]);
    // (Graduated: the partner home, the third recruit in its place.)
    assembly.free = true;
    press(&mut assembly, &[Up, Up, Up, Mark, Down, Down, Down, Mark], run);
    assert_eq!((run.team_size(), run.along()), (4, vec![0, 1, 2]));
    // (A recruit seen, its tactic changed, then released once asked again.)
    let (page, options, _) = press(&mut assembly, &[Up, Up, Confirm], run);
    assert!(page.contains("Lv.") && page.contains("HP") && page.contains("Moves:"), "{page}");
    assert_eq!(options.len(), 4, "{options:?}");
    press(&mut assembly, &[Down, Confirm], run);
    assert_ne!(run.member_tactic(rules::run::town::Whom::Recruit(0)), 0, "its next tactic");
    let (asked, ..) = press(&mut assembly, &[Down, Confirm], run);
    assert!(asked.starts_with("Release"), "{asked}");
    let (said, ..) = press(&mut assembly, &[Up, Confirm], run);
    assert!(said.contains("left the team"), "{said}");
    press(&mut assembly, &[Cancel], run);
    assert_eq!(run.team().progress.recruits.len(), 2);
    // (The team named: `{team}` in a film.)
    assert_eq!(run.team().team_name(), rules::run::town::DEFAULT_TEAM_NAME);
    let mut naming = crate::assembly::Naming::new(run);
    let keys = shell::Keys { typed: "Thorns".chars().collect(), ..shell::Keys::default() };
    naming.play(&keys, run);
    assert!(naming.play(&shell::Keys { enter: true, ..shell::Keys::default() }, run).0);
    assert_eq!(run.team().team_name(), "Thorns");
    let (_, words) = game.playthrough.as_ref().expect("a game").numbers();
    assert!(words.contains(&("team".to_string(), "Thorns".to_string())), "{words:?}");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_town_s_other_shops_swap_link_evolve_draw_and_keep_the_bag() {
    use rules::run::town::Whom;
    use rules::services::Input::{self, Confirm, Down, Up};
    let data = data().expect("the extracted data/");
    let rescue = crate::testing::root().join("tests/games/test_rescue/test_rescue.game");
    let mut game = Game::open(&crate::testing::context_on(&data), &rescue, None).expect("the game opens");
    (game.title, game.story.chapter) = (None, 2);
    game.enter("test_square", super::roam::Arrival::Entrance(None), true);
    game.make_team();
    standing(&mut game);
    let playthrough = game.playthrough.as_mut().expect("a game");
    playthrough.money(2000);
    playthrough.recruit(133);
    for (item, count) in [("Bulba-Claw", 1), ("Bulba-Fang", 1), ("Fire Stone", 1), ("Prize Ticket", 1)] {
        playthrough.give(item, count);
    }
    let run = playthrough.run_mut().expect("its rules");
    fn press<S>(
        screen: &mut S,
        keys: &[Input],
        run: &mut rules::run::Run,
        play: impl Fn(&mut S, Option<Input>, &mut rules::run::Run) -> (bool, crate::assembly::Shown),
    ) -> (bool, crate::assembly::Shown) {
        keys.iter().map(|&key| play(screen, Some(key), run)).last().unwrap_or_default()
    }

    // (Croagunk: the Claw and the Fang for the Grass-Guard, 150 Poké.)
    let mut shop = crate::town_shops::SwapShop::default();
    let (_, (asked, ..)) = press(&mut shop, &[Confirm], run, crate::town_shops::SwapShop::play);
    assert!(asked.starts_with("Swap your Bulba-Claw, Bulba-Fang for the Grass-Guard"), "{asked}");
    let (_, (said, ..)) = press(&mut shop, &[Confirm], run, crate::town_shops::SwapShop::play);
    assert!(said.contains("The swap worked"), "{said}");
    assert!(run.bag_ids().contains(&576) && !run.bag_ids().contains(&574), "{:?}", run.bag_ids());
    assert_eq!(run.bag().money, 2000 - rules::town::swap::PRICE);

    // (The Link Shop: the leader's second move linked to its first, a session paid.)
    let mut link = crate::town_shops::LinkShop::default();
    let before = run.bag().money;
    press(&mut link, &[Confirm, Down, Confirm, Confirm], run, crate::town_shops::LinkShop::play);
    assert!(run.linked_moves(false)[1].1, "{:?}", run.linked_moves(false));
    assert_eq!(run.bag().money, before - rules::run::town::LINK_PRICE);

    // (Luminous Spring: Eevee and its Fire Stone; the leader not before graduating.)
    let mut spring = crate::town_shops::Spring::default();
    let (_, (said, ..)) = press(&mut spring, &[Confirm], run, crate::town_shops::Spring::play);
    assert!(said.contains("doesn't reach"), "{said}");
    let flareon = run.evolution_offers(Whom::Recruit(0)).iter().position(|(into, may)| *into == 136 && may.is_ok()).expect("Flareon, by its stone");
    let mut keys = vec![Confirm, Down, Down, Confirm];
    keys.extend(std::iter::repeat_n(Down, flareon));
    keys.extend([Confirm, Confirm]);
    let (left, _) = press(&mut spring, &keys, run, crate::town_shops::Spring::play);
    assert!(left, "left for the light");
    let evolved = spring.evolved.clone().expect("evolved");
    assert_eq!((evolved.from, evolved.into, run.member_species(Whom::Recruit(0))), (133, 136, 136));
    assert!(!run.bag_ids().contains(&146), "its stone used up");

    // (The prize drawing: the ticket played.)
    let mut drawing = crate::town_shops::Lottery::new(rules::run::town::Draw(7));
    let (_, (said, ..)) = press(&mut drawing, &[Confirm], run, crate::town_shops::Lottery::play);
    assert!(said.contains("drum"), "{said}");
    assert!(run.tickets().is_empty());
    let mut recycle = crate::town_shops::Recycle::new(rules::run::town::Draw(7));
    let (_, (message, ..)) = press(&mut recycle, &[Up, Down], run, crate::town_shops::Recycle::play);
    assert!(message.contains("Recycle Shop"), "{message}");

    // (The bag: an item thrown away once asked, the bag sorted.)
    let mut bag = crate::town_shops::Bag::default();
    let count = run.bag().items.len();
    let bag_play = |bag: &mut crate::town_shops::Bag, key, run: &mut rules::run::Run| bag.play(key, run, &[]);
    let (_, (asked, ..)) = press(&mut bag, &[Confirm], run, bag_play);
    assert!(asked.starts_with("Throw away"), "{asked}");
    press(&mut bag, &[Up, Confirm, Confirm], run, bag_play);
    assert_eq!(run.bag().items.len(), count - 1);

    // (The light of the films' evolution, in the place, then the place again.)
    game.evolution_scene(&evolved);
    assert!(game.cutscene.is_some() && game.said.is_none(), "{:?} {:?}", game.said, game.log);
    assert!(game.log.iter().any(|line| line.starts_with("evolved ") && line.ends_with("flareon")), "{:?}", game.log);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_guild_s_extras_sentry_duty_the_dojo_the_sos_the_quiz_and_a_new_leader() {
    use rules::run::town::Whom;
    use rules::services::Input::{Confirm, Down};
    let data = data().expect("the extracted data/");
    let rescue = crate::testing::root().join("tests/games/test_rescue/test_rescue.game");
    let mut game = Game::open(&crate::testing::context_on(&data), &rescue, None).expect("the game opens");
    (game.title, game.story.chapter) = (None, 2);
    game.enter("test_square", super::roam::Arrival::Entrance(None), true);
    game.make_team();
    standing(&mut game);
    let run = game.playthrough.as_mut().and_then(crate::playthrough::Playthrough::run_mut).expect("its rules");

    // (Sentry duty: each visitor told right, quickly; then not again today.)
    let mut duty = crate::guild::SentryDuty::new(run, rules::run::town::Draw(5));
    let (_, (asked, ..)) = duty.play(None, run);
    assert!(asked.starts_with("Sentry duty!"), "{asked}");
    let mut said = duty.play(Some(Confirm), run).1;
    for _ in 0..rules::sentry::ROUNDS {
        let (visitor, hidden) = duty.face().expect("footprints on show");
        assert!(hidden, "in shadow till told");
        let at = said.1.iter().position(|name| *name == run.book.species_name(visitor)).expect("the visitor among the four");
        for _ in 0..at {
            duty.play(Some(Down), run);
        }
        duty.play(Some(Confirm), run);
        assert_eq!(duty.face(), Some((visitor, false)), "told: the visitor as it is");
        said = duty.play(Some(Confirm), run).1;
    }
    assert!(said.0.contains("Perfect!") && said.0.contains("You received"), "{}", said.0);
    assert!(!run.sentry_free() && run.team().progress.sentry_points > 10_000);
    let (_, (refused, ..)) = crate::guild::SentryDuty::new(run, rules::run::town::Draw(5)).play(None, run);
    assert!(refused.contains("today"), "{refused}");

    // (The Dojo: its mazes by type, the first chosen.)
    let mut dojo = crate::guild::Dojo::default();
    let (_, (welcome, mazes, _)) = dojo.play(None, run);
    assert!(welcome.contains("Dojo") && mazes.first().map(String::as_str) == Some("Normal Maze"), "{welcome} {mazes:?}");
    assert_eq!(mazes.len(), rules::dojo::TYPES.len() + 1, "the Final Maze not yet: {mazes:?}");
    assert!(dojo.play(Some(Confirm), run).0 && dojo.chosen == Some(rules::dojo::FIRST));

    // (Pelipper: the SOS sent, then its A-OK Mail read, what was lost given back.)
    run.read_save(
        "sos=12:4:250:0:0:70
",
    );
    let mut sos = crate::guild::Rescue::default();
    let (_, (asked, ..)) = sos.play(None, run);
    assert!(asked.starts_with("Your team fainted at") && asked.contains("1 items and 250"), "{asked}");
    let (_, (sent, ..)) = sos.play(Some(Confirm), run);
    assert!(sent.contains("SOS") && run.sos().is_some_and(|sos| sos.sent), "{sent}");
    run.read_save(
        "sos=12:4:250:1:0:70
",
    );
    let money = run.bag().money;
    let mut sos = crate::guild::Rescue::default();
    let (_, (read, ..)) = sos.play(Some(Confirm), run);
    assert!(read.contains("A-OK Mail") && !read.contains("{item:"), "{read}");
    assert!(run.bag().money == money + 250 && run.sos().is_none());

    // (A new leader at the assembly, once graduated: the partner leads.)
    let (leader, partner) = run.team().species;
    let mut assembly = crate::assembly::Assembly::default();
    assembly.free = true;
    for key in [Down, Confirm, Down, Confirm] {
        assembly.play(Some(key), run);
    }
    assert_eq!(run.team().species, (partner, leader), "{}", run.member_name(Whom::Leader));

    // (The personality test, its pages read: no word of the game's left unsaid.)
    let mut quiz = crate::guild::QuizScreen::new(run, rules::run::town::Draw(3)).expect("the test in the data");
    let mut pages = vec![quiz.play(None, run).1.0];
    while quiz.chosen.is_none() && pages.len() < 200 {
        pages.push(quiz.play(Some(Confirm), run).1.0);
    }
    let tags: Vec<&String> = pages.iter().filter(|page| page.contains("[kind") || page.contains("[hero") || page.contains("[partner")).collect();
    assert!(quiz.chosen.is_some() && tags.is_empty(), "{tags:?}");
    let (_, partner) = quiz.chosen.expect("chosen");
    let asked = format!("Is [CS:N]{}[CR] who you want?", run.book.species_name(partner));
    assert!(pages.iter().any(|page| page.contains(&asked)), "{pages:?}");

    // (The personality test chooses the team: nobody at the keys, its first answers.)
    if let Some(project) = game.project.as_mut() {
        project.team = vec![crate::game_file::QUIZ.to_string()];
    }
    game.nobody = true;
    game.make_team();
    assert!(game.log.iter().any(|line| line.starts_with("quiz ")), "{:?} {:?}", game.log, game.said);

    // (Into the maze chosen: a dungeon of the rules' own, the team lent.)
    game.nobody = false;
    game.enter_maze(rules::dojo::FIRST);
    assert!(game.diving.is_some() && game.log.last().map(String::as_str) == Some("maze Normal Maze"), "{:?} {:?}", game.log, game.said);
}
