//! The example games played from end to end by a script at the keys, with no
//! window (`drive`): a new game from the title screen, the shop, the menu's
//! save taken up again, a dungeon lost, the end card, a pack played.
//! (They need the extracted `data/`: ignored, `cargo test -- --ignored`.)

use std::path::{Path, PathBuf};

use crate::drive::Driver;
use crate::game_screens::GameScreen;
use crate::{Game, Mode};

/// The folder of the game's files (`DUNGEON_DATA`, else the repository's), if they are there.
fn data() -> Option<String> {
    let data = crate::testing::data_folder();
    shell::data::read(&data, "dungeons.bin").map(|_| data)
}

fn example() -> PathBuf {
    crate::testing::root().join("tests/games/test_village/test_village.game")
}

/// A folder of the test's own, empty.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("dw_e2e_{name}_{}", std::process::id()));
    _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    folder
}

/// That game on its title screen, saved in `saves`, nothing saved yet.
fn opened(game: &Path, data: &str, saves: &Path) -> Game {
    let mut game = Game::open(&crate::testing::context_on(data), game, None).expect("the game opens");
    game.saves = saves.to_path_buf();
    game.title.as_mut().expect("a title screen").saved = None;
    // (The same draw each time: the shops' wares, the boards' jobs are drawn
    // from it, and a ware dearer than what the test gives fails it now and then.)
    game.draw = rules::run::town::Draw(1);
    game
}

/// What the team carries: its money, its items by name.
fn carried(game: &Game) -> (i32, Vec<(String, u32)>) {
    let carried = game.playthrough.as_ref().map(crate::playthrough::Playthrough::carried).unwrap_or_default();
    (carried.money, carried.items.into_iter().map(|(name, count)| (name, count as u32)).collect())
}

/// Whether those lines are in what the game did, in that order.
fn in_order(log: &[String], wanted: &[&str]) -> bool {
    let mut from = 0;
    wanted.iter().all(|one| match log[from..].iter().position(|line| line == one) {
        Some(at) => {
            from += at + 1;
            true
        }
        None => false,
    })
}

/// From the title screen, a new game: its first film read, the team walking the market.
fn new_game(driver: &mut Driver) -> Result<(), String> {
    driver.press("enter");
    driver.read_to_roaming(30_000)?;
    match driver.game.story.place.as_deref() {
        Some("test_market") => Ok(()),
        _ => Err(driver.lost("not at the market")),
    }
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_new_game_buys_at_kecleon_s_saves_from_the_menu_and_is_taken_up_the_same() -> Result<(), String> {
    let data = data().expect("the extracted data/");
    let saves = folder("shop");
    let mut game = opened(&example(), &data, &saves);
    let mut driver = Driver::new(&mut game);
    new_game(&mut driver)?;
    // (The story gives no money before the forest: some, to buy with.)
    if let Some(playthrough) = driver.game.playthrough.as_mut() {
        playthrough.money(5000);
    }
    let (money, items) = carried(driver.game);
    // (Kecleon's shop: Buy, the first of its wares.)
    driver.talk_to("kecleon", 3_000)?;
    driver.wait_until(60, |game| matches!(game.screen, Some((GameScreen::Service { .. }, _))))?;
    driver.keys(&["enter", "enter"]);
    let (spent, bought) = carried(driver.game);
    assert!(spent < money, "money down: {money} then {spent}");
    let count = |items: &[(String, u32)]| items.iter().map(|(_, count)| count).sum::<u32>();
    assert_eq!(count(&bought), count(&items) + 1, "an item more in the bag: {items:?} then {bought:?}");
    // (Left: the list, then the shop.)
    driver.keys(&["escape", "escape"]);
    driver.wait_until(60, |game| game.mode() == Mode::Roam)?;
    // (The menu, M: Team, Bag, Save.)
    driver.press("m");
    driver.wait_until(60, |game| matches!(game.screen, Some((GameScreen::Menu { .. }, _))))?;
    driver.keys(&["down", "down", "enter"]);
    let file = crate::story::save_file(&saves, "test_village");
    let saved = std::fs::read_to_string(&file).map_err(|why| driver.lost(&format!("{}: {why}", file.display())))?;
    driver.keys(&["enter", "escape"]);
    driver.wait_until(60, |game| game.mode() == Mode::Roam)?;
    let (chapter, place) = (driver.game.story.chapter, driver.game.story.place.clone());
    // (Opened again: Continue, and the same place, chapter and bag.)
    let mut again = opened(&example(), &data, &saves);
    again.title.as_mut().expect("a title screen").saved = Some(saved);
    let mut driver = Driver::new(&mut again);
    driver.press("enter");
    driver.read_to_roaming(3_000)?;
    assert_eq!((driver.game.story.chapter, &driver.game.story.place), (chapter, &place));
    assert_eq!(carried(driver.game), (spent, bought), "the same bag, the same money");
    _ = std::fs::remove_dir_all(&saves);
    Ok(())
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_team_fainted_in_the_dungeon_comes_home_told_what_it_lost() -> Result<(), String> {
    let data = data().expect("the extracted data/");
    let saves = folder("ko");
    let mut game = opened(&example(), &data, &saves);
    let mut driver = Driver::new(&mut game);
    new_game(&mut driver)?;
    if let Some(playthrough) = driver.game.playthrough.as_mut() {
        playthrough.money(1000);
    }
    // (Pidgeot flies the team to the forest.)
    driver.talk_to("pidgeot", 3_000)?;
    driver.read_until(3_000, |game| game.mode() == Mode::Dungeon)?;
    // (A few steps on its first floor, the dungeon's menu opened and closed; then the team falls, and a turn goes by.)
    driver.wait(60);
    driver.hold("right", 20);
    driver.hold("down", 20);
    driver.keys(&["escape", "escape"]);
    if let Some(diving) = driver.game.diving.as_mut() {
        diving.faint_team();
    }
    driver.until(3_000, |_, _| crate::drive::keys("r"), |game| game.mode() != Mode::Dungeon)?;
    driver.read_to_roaming(3_000)?;
    let game = &*driver.game;
    assert_eq!((game.story.place.as_deref(), game.story.won), (Some("test_market"), false), "home: {:?}", game.log);
    assert!(in_order(&game.log, &["dungeon test_forest", "dungeon lost"]), "{:?}", game.log);
    let (money, _) = carried(game);
    assert!(money < 1000, "money lost: {money}");
    let told = game.place.as_ref().map(|place| place.text.clone()).unwrap_or_default();
    assert!(told.contains("narrator:"), "what was lost, said: {told}");
    _ = std::fs::remove_dir_all(&saves);
    Ok(())
}

/// In the second chapter, walking: Kecleon thanked, the end card, back to the title.
fn to_the_end(driver: &mut Driver) -> Result<(), String> {
    driver.talk_to("kecleon", 3_000)?;
    driver.read_until(20_000, |game| game.mode() == Mode::End)?;
    driver.wait(30);
    if driver.game.mode() != Mode::End {
        return Err(driver.lost("the end card waits for a key"));
    }
    driver.press("enter");
    match driver.game.mode() {
        Mode::Title => Ok(()),
        _ => Err(driver.lost("not back on the title screen")),
    }
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_finished_game_shows_its_end_card_then_its_title() -> Result<(), String> {
    let data = data().expect("the extracted data/");
    let saves = folder("end");
    let mut game = opened(&example(), &data, &saves);
    game.new_game_at(2)?;
    let mut driver = Driver::new(&mut game);
    driver.read_to_roaming(30_000)?;
    to_the_end(&mut driver)?;
    let game = &*driver.game;
    assert!(in_order(&game.log, &["chapter 2", "film back_home", "talk kecleon", "film thanks", "the end", "title"]), "{:?}", game.log);
    assert_eq!(game.title.as_ref().map(|title| title.name.as_str()), Some("Test Village"));
    _ = std::fs::remove_dir_all(&saves);
    Ok(())
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_example_packed_plays_from_its_title_to_its_end() -> Result<(), String> {
    let data = data().expect("the extracted data/");
    let out = folder("pack");
    let packed = crate::pack::pack_game(&crate::testing::context_on(&data), &example(), &out.join("test_village.dwpack"))?;
    let (path, _) = crate::pack::unpack(&packed.path, &out.join("unpacked"))?;
    let mut game = opened(&path, &data, &out.join("saves"));
    let mut driver = Driver::new(&mut game);
    // (Someone at the title; nobody through the first chapter and its forest; someone again in the second.)
    driver.press("enter");
    driver.game.nobody = true;
    driver.wait_until(200_000, |game| game.cutscene.as_ref().is_some_and(|(_, word)| word == "back_home"))?;
    driver.game.nobody = false;
    driver.read_to_roaming(30_000)?;
    to_the_end(&mut driver)?;
    let order = ["chapter 1", "film arrival", "talk pidgeot", "dungeon test_forest", "dungeon won", "chapter 2", "film thanks", "the end", "title"];
    assert!(in_order(&driver.game.log, &order), "{order:?}: {:?}", driver.game.log);
    _ = std::fs::remove_dir_all(&out);
    Ok(())
}

/// A game of `games/` by its name.
fn game_named(name: &str) -> PathBuf {
    crate::testing::root().join("tests/games").join(name).join(format!("{name}.game"))
}

/// Nobody at the keys until that film plays (a dungeon or a fight passed as
/// won on the way), then someone again.
fn nobody_until_film(driver: &mut Driver, film: &str) -> Result<usize, String> {
    driver.game.nobody = true;
    let film = film.to_string();
    let took = driver.wait_until(200_000, move |game| game.cutscene.as_ref().is_some_and(|(_, word)| *word == film));
    driver.game.nobody = false;
    took
}

/// Whether that film has ended, by the game's log.
fn film_ended(film: &'static str) -> impl Fn(&Game) -> bool {
    move |game: &Game| game.log.iter().any(|line| *line == format!("film {film} ended"))
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_lost_bell_is_played_from_its_title_to_its_end_with_a_save_in_the_middle() -> Result<(), String> {
    let data = data().expect("the extracted data/");
    let saves = folder("lost_bell");
    let mut game = opened(&game_named("test_rescue"), &data, &saves);
    let mut driver = Driver::new(&mut game);
    // (Chapter 1: the silent dawn, then the village.)
    driver.press("enter");
    driver.read_to_roaming(30_000)?;
    assert_eq!(driver.game.story.place.as_deref(), Some("test_square"));
    // (Kecleon's shop first: an Oran Berry bought, the shop left once, and on with the walk.)
    if let Some(playthrough) = driver.game.playthrough.as_mut() {
        playthrough.money(5000);
    }
    let (money, _) = carried(driver.game);
    driver.talk_to("kecleon", 3_000)?;
    driver.wait_until(60, |game| matches!(game.screen, Some((GameScreen::Service { .. }, _))))?;
    driver.keys(&["enter", "enter"]);
    assert!(carried(driver.game).0 < money, "something bought: {:?}", driver.game.log);
    driver.keys(&["escape", "escape"]);
    driver.wait_until(60, |game| game.mode() == Mode::Roam && game.screen.is_none())?;
    let talks = |game: &Game| game.log.iter().filter(|line| *line == "talk kecleon").count();
    let once = talks(driver.game);
    driver.wait(30);
    assert!(driver.game.screen.is_none() && talks(driver.game) == once, "the shop opened again: {:?}", driver.game.log);
    // (Bibarel pins the request; the team takes it.)
    driver.talk_to("bibarel", 3_000)?;
    driver.read_until(20_000, film_ended("job"))?;
    driver.read_to_roaming(20_000)?;
    assert!(driver.game.story.flags.contains("took_job"), "{:?}", driver.game.log);
    assert_eq!(talks(driver.game), once, "walked past Kecleon without talking: {:?}", driver.game.log);
    // (Marill leads into the woods: the dungeon, won, and the bear's secret.)
    driver.talk_to("marill", 3_000)?;
    driver.read_until(20_000, |game| game.cutscene.as_ref().is_some_and(|(_, word)| word == "to_the_woods"))?;
    nobody_until_film(&mut driver, "found")?;
    driver.read_to_roaming(40_000)?;
    assert_eq!((driver.game.story.chapter, driver.game.story.place.as_deref()), (2, Some("test_square")));
    assert!(in_order(&driver.game.log, &["dungeon test_woods", "dungeon won", "chapter 2", "recruit ursaring", "film after"]), "{:?}", driver.game.log);
    // (Saved from the menu, in the middle of the story.)
    driver.press("m");
    driver.wait_until(60, |game| matches!(game.screen, Some((GameScreen::Menu { .. }, _))))?;
    driver.keys(&["down", "down", "enter"]);
    let file = crate::story::save_file(&saves, "test_rescue");
    let saved = std::fs::read_to_string(&file).map_err(|why| driver.lost(&format!("{}: {why}", file.display())))?;
    driver.keys(&["enter", "escape"]);
    // (Taken up again from the title: Continue.)
    let mut again = opened(&game_named("test_rescue"), &data, &saves);
    again.title.as_mut().expect("a title screen").saved = Some(saved);
    let mut driver = Driver::new(&mut again);
    driver.press("enter");
    driver.read_to_roaming(3_000)?;
    assert_eq!(driver.game.story.chapter, 2);
    // (Marill's thanks, once; the board; then Noctowl, and the end.)
    driver.talk_to("marill", 3_000)?;
    driver.read_until(20_000, film_ended("client_thanks"))?;
    driver.read_to_roaming(10_000)?;
    assert!(driver.game.story.flags.contains("thanked"), "{:?}", driver.game.log);
    driver.talk_to("bibarel", 3_000)?;
    driver.wait_until(60, |game| matches!(game.screen, Some((GameScreen::Board(_), _))))?;
    driver.keys(&["escape", "escape", "escape"]);
    driver.wait_until(120, |game| game.mode() == Mode::Roam)?;
    driver.talk_to("noctowl", 3_000)?;
    driver.read_until(20_000, |game| game.mode() == Mode::End)?;
    assert!(in_order(&driver.game.log, &["talk noctowl", "film ending", "the end"]), "{:?}", driver.game.log);
    _ = std::fs::remove_dir_all(&saves);
    Ok(())
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_fog_over_lanternport_is_solved_by_its_zones_its_map_and_a_fight_lost_then_won() -> Result<(), String> {
    let data = data().expect("the extracted data/");
    let saves = folder("lanternport");
    let mut game = opened(&game_named("test_mystery"), &data, &saves);
    let mut driver = Driver::new(&mut game);
    // (Chapter 1: the harbour; the first clue by walking down to the tideline.)
    driver.press("enter");
    driver.read_to_roaming(30_000)?;
    assert_eq!(driver.game.story.place.as_deref(), Some("test_harbour"));
    driver.walk_until(glam::Vec2::new(100.0, 158.0), 3_000, |game| game.cutscene.is_some())?;
    driver.read_until(20_000, film_ended("clue_one"))?;
    driver.read_to_roaming(10_000)?;
    assert!(driver.game.story.flags.contains("saw_trail"), "{:?}", driver.game.log);
    // (Sableye: heard out, the second option of the film's choice.)
    driver.talk_to("sableye", 3_000)?;
    driver.read_until(20_000, |game| game.film().is_some_and(preview::film::Film::asking))?;
    driver.wait(10);
    driver.keys(&["down", "enter"]);
    driver.read_until(20_000, film_ended("the_choice"))?;
    driver.read_to_roaming(10_000)?;
    assert!(driver.game.story.flags.contains("heard_sableye"), "{:?} {:?}", driver.game.story.flags, driver.game.log);
    // (Wingull: on to the lighthouse, chapter 2, the garden.)
    driver.talk_to("wingull", 3_000)?;
    driver.read_until(40_000, film_ended("garden"))?;
    driver.read_to_roaming(10_000)?;
    assert_eq!((driver.game.story.chapter, driver.game.story.place.as_deref()), (2, Some("test_garden")));
    // (The second clue by the stairs to the lamp; then Ampharos, and night.)
    driver.walk_until(glam::Vec2::new(120.0, 228.0), 3_000, |game| game.cutscene.is_some())?;
    driver.read_until(20_000, film_ended("clue_two"))?;
    driver.read_to_roaming(10_000)?;
    assert!(driver.game.story.flags.contains("felt_cold"), "{:?}", driver.game.log);
    driver.talk_to("ampharos", 3_000)?;
    driver.read_until(40_000, film_ended("morning"))?;
    driver.read_to_roaming(10_000)?;
    assert_eq!((driver.game.story.chapter, driver.game.story.place.as_deref()), (3, Some("test_harbour")));
    // (Sableye's gift, as warm as the team was to him.)
    driver.talk_to("sableye", 3_000)?;
    driver.read_until(20_000, film_ended("gift"))?;
    driver.read_to_roaming(10_000)?;
    assert!(carried(driver.game).1.iter().any(|(name, _)| name == "Reviver Seed"), "{:?}", carried(driver.game));
    // (Pelipper's ferry: the map, the garden chosen; back down by its path.)
    driver.talk_to("pelipper", 3_000)?;
    driver.wait_until(60, |game| matches!(game.screen, Some((GameScreen::Map { .. }, _))))?;
    // (The garden by its name, wherever the map lists it.)
    let garden = match &driver.game.screen {
        Some((GameScreen::Map { ways, .. }, _)) => ways.iter().position(|way| way.to_lowercase().contains("garden")),
        _ => None,
    };
    let garden = garden.ok_or_else(|| driver.lost("no garden on the map"))?;
    for _ in 0..garden {
        driver.press("down");
    }
    driver.press("enter");
    driver.read_to_roaming(3_000)?;
    assert_eq!(driver.game.story.place.as_deref(), Some("test_garden"), "{:?}", driver.game.log);
    driver.walk_until(glam::Vec2::new(256.0, 400.0), 3_000, |game| game.story.place.as_deref() == Some("test_harbour"))?;
    driver.read_to_roaming(3_000)?;
    // (The fight on the cliff, lost: the team faints, and is told so.)
    driver.talk_to("wingull", 3_000)?;
    driver.read_until(20_000, |game| game.mode() == Mode::Dungeon)?;
    driver.wait(60);
    if let Some(diving) = driver.game.diving.as_mut() {
        diving.faint_team();
    }
    driver.until(3_000, |_, _| crate::drive::keys("r"), |game| game.mode() != Mode::Dungeon)?;
    driver.read_until(20_000, film_ended("fight_lost"))?;
    driver.read_to_roaming(10_000)?;
    assert!(in_order(&driver.game.log, &["dungeon test_thief", "dungeon lost", "film fight_lost"]), "{:?}", driver.game.log);
    // (Tried again, won, and the end.)
    driver.talk_to("wingull", 3_000)?;
    driver.read_until(20_000, |game| game.cutscene.as_ref().is_some_and(|(_, word)| word == "to_the_fight"))?;
    nobody_until_film(&mut driver, "ending")?;
    driver.read_until(20_000, |game| game.mode() == Mode::End)?;
    assert!(in_order(&driver.game.log, &["dungeon won", "film ending", "the end"]), "{:?}", driver.game.log);
    _ = std::fs::remove_dir_all(&saves);
    Ok(())
}

/// How much of the screen as the game draws it now is lit (0 to 100).
fn lit(game: &mut Game) -> usize {
    let (mut canvas, mut world) = (stage_view::ui::Canvas::default(), shell::Screen::new(1));
    game.draw(&mut canvas, &mut world);
    let lit = world.rgba.chunks(4).filter(|pixel| pixel[..3].iter().any(|value| *value > 8)).count();
    lit * 100 / (world.rgba.len() / 4).max(1)
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_place_comes_back_seen_and_walked_after_a_talk_s_film_and_its_choice() -> Result<(), String> {
    let data = data().expect("the extracted data/");
    let saves = folder("back_seen");
    let mut game = opened(&game_named("test_mystery"), &data, &saves);
    let mut driver = Driver::new(&mut game);
    driver.press("enter");
    driver.read_to_roaming(30_000)?;
    // (Sableye's film: its choice, the first option, the film over.)
    driver.talk_to("sableye", 3_000)?;
    driver.read_until(20_000, |game| game.film().is_some_and(preview::film::Film::asking))?;
    driver.wait(10);
    driver.press("enter");
    driver.read_until(20_000, film_ended("the_choice"))?;
    driver.read_to_roaming(10_000)?;
    assert!(driver.game.story.flags.contains("accused_sableye"), "{:?}", driver.game.story.flags);
    // (The harbour again: seen, not left on the film's dark, and walked.)
    driver.wait(30);
    let fade = driver.game.film().map(|film| film.play().sim.stage.light.fade.level);
    assert_eq!(fade, Some(0.0), "{}", driver.lost("the place left faded"));
    let seen = lit(driver.game);
    assert!(seen > 20, "{}", driver.lost(&format!("the place drawn dark: {seen}% lit")));
    let from = driver.game.walked_at().ok_or_else(|| driver.lost("no one walked"))?;
    driver.hold("left", 30);
    let to = driver.game.walked_at().ok_or_else(|| driver.lost("no one walked"))?;
    driver.hold("right", 30);
    let back = driver.game.walked_at().ok_or_else(|| driver.lost("no one walked"))?;
    assert!(to != from || back != to, "{}", driver.lost("the leader did not move"));
    _ = std::fs::remove_dir_all(&saves);
    Ok(())
}
