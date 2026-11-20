//! Dungeons of one's own gone through, and the game kept through one and a
//! save. (They need the extracted `data/`: ignored, `cargo test -- --ignored`.)

use preview::art::Art;
use shell::Screen;
use stage_view::ui::Canvas;

/// The game's files, if they are there.
fn art() -> Option<Art> {
    let data = crate::testing::data_folder();
    shell::data::read(&data, "dungeons.bin").map(|_| Art::open_in(&data, crate::testing::library()).expect("the game's files, read"))
}

/// The example game's forest, its fight read in.
fn test_forest() -> String {
    let game = crate::testing::library().at(crate::testing::root().join("tests/games/test_village"));
    crate::dive::resolved_for(&game, &std::fs::read_to_string(game.dungeon("test_forest")).expect("the dungeon"))
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn any_pokemon_of_the_game_s_dungeons_can_go_through_one_of_one_s_own() {
    let art = art().expect("the extracted data/");
    let text = test_forest();
    // (Caterpie and Pidgey: no team the game starts with, met in its dungeons.)
    let team = crate::dive::Team { hero: 10, partner: 16, level: Some(7) };
    let mut diving = crate::dive::Diving::of_text(&art.folder, &art.project, &text, "trial", team, &crate::dive::Carried::default()).expect("it goes in");
    for _ in 0..60 {
        diving.tick(&shell::Keys::default());
    }
    assert_eq!(diving.floor(), Some((1, 4)), "three floors, then the boss's");
    assert_eq!(diving.team(), (10, 16, 7), "Caterpie leads at level 7, Pidgey follows");
    assert!(diving.ended.is_none());
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_boss_waits_in_its_room_with_its_guards() {
    let art = art().expect("the extracted data/");
    let text = "name: The Den\nfloors: 0\nlook: Apple Woods\nboss: Spearow 12\nboss_guards: Pidgey 8, Pidgey 8\nboss_hp: x3\nboss_arena: 15x9\nboss_shape: pillars\nboss_says: You will go no further!\nboss_anger: +2 Enough games!\nboss_item: Golden Apple\nboss_moves: Peck, Leer\nboss_reinforcements: 30% Pidgey 6, Pidgey 6 : To me!\nboss_heal: 25% 30%\n";
    let team = crate::dive::Team { hero: 25, partner: 25, level: Some(10) };
    let mut diving = crate::dive::Diving::of_text(&art.folder, &art.project, text, "trial", team, &crate::dive::Carried::default()).expect("it goes in");
    for _ in 0..30 {
        diving.tick(&shell::Keys::default());
    }
    assert_eq!(diving.floor(), Some((1, 1)), "the fight is the only floor");
    let foes = diving.foes();
    assert_eq!(foes.len(), 3, "the boss and its two guards: {foes:?}");
    let boss = foes.iter().max_by_key(|(_, _, most)| *most).expect("a boss");
    assert!(boss.2 > foes.iter().map(|(_, _, most)| *most).min().unwrap_or(0) * 2, "three times its life: {foes:?}");
    assert_eq!(diving.boss_life(), Some((boss.1, boss.2)), "its gauge, full");
    assert!(diving.said().iter().any(|line| line.contains("You will go no further")), "{:?}", diving.said());
    let (mut canvas, mut screen) = (Canvas::default(), Screen::new(2));
    let _ = &mut canvas;
    diving.draw(&mut screen);
    let red = screen.rgba.chunks(4).filter(|pixel| pixel[..3] == [0xE0, 0x30, 0x30]).count();
    assert!(red > 100, "the gauge drawn ({red} pixels)");
    // (At a fifth of its life: angry, its help come, healed once.)
    diving.boss_at(20);
    for _ in 0..5 {
        diving.tick(&shell::Keys::default());
    }
    let said = diving.said().join(" | ");
    assert!(said.contains("Enough games") && said.contains("To me") && said.contains("regains its strength"), "{said}");
    assert_eq!(diving.foes().len(), 5, "two more came: {:?}", diving.foes());
    let (now, most) = diving.boss_life().expect("still fighting");
    assert!(now * 100 > most * 40, "healed by 30 %: {now}/{most}");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_boss_fights_in_phases_and_may_join() {
    let art = art().expect("the extracted data/");
    let text = test_forest().replace("floors: 3", "floors: 0");
    let team = crate::dive::Team { hero: 25, partner: 25, level: Some(10) };
    let mut diving = crate::dive::Diving::of_text(&art.folder, &art.project, &text, "trial", team, &crate::dive::Carried::default()).expect("it goes in");
    diving.tick(&shell::Keys::default());
    diving.boss_at(15);
    for _ in 0..5 {
        diving.tick(&shell::Keys::default());
    }
    let said = diving.said().join(" | ");
    for wanted in ["Phase at 60%", "Phase at 40%", "started to rain", "Phase at 20%", "regains its strength", "changes the way"] {
        assert!(said.contains(wanted), "“{wanted}” in: {said}");
    }
    // (Each phase once: the anger said once only.)
    for _ in 0..30 {
        diving.tick(&shell::Keys::default());
    }
    assert_eq!(diving.said().iter().filter(|line| line.contains("Phase at 60%")).count(), 1);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_boss_room_is_laid_out_as_written() {
    let art = art().expect("the extracted data/");
    let text = "name: The Den\nfloors: 0\nlook: Apple Woods\nboss: Spearow 12\nboss_guards: Pidgey 8 @2,2, Pidgey 8\nboss_arena: 15x9\nboss_shape: pillars\nboss_at: 12 1\nboss_team_at: 2 7\nboss_floor_items: Sitrus Berry @3,4, Blast Seed @10,6\nboss_traps: spikes @4,4, slumber @9,5 hidden\n";
    let team = crate::dive::Team { hero: 25, partner: 25, level: Some(10) };
    let diving = crate::dive::Diving::of_text(&art.folder, &art.project, text, "trial", team, &crate::dive::Carried::default()).expect("it goes in");
    let (standing, items, traps) = diving.laid_out();
    // (The room starts 5 tiles in from the floor's corner: `Fixed::tile`.)
    let at = |x: i32, y: i32| (x + 5, y + 5);
    let named = |name: &str| standing.iter().filter(|(who, _)| who == name).map(|(_, at)| *at).collect::<Vec<_>>();
    assert_eq!(named("Spearow"), vec![at(12, 1)], "{standing:?}");
    assert!(named("Pidgey").contains(&at(2, 2)), "{standing:?}");
    assert!(named("Pikachu").contains(&at(2, 7)), "{standing:?}");
    assert_eq!((items.len(), traps.len()), (2, 2));
    assert!(items.contains(&at(3, 4)) && traps.contains(&at(9, 5)), "{items:?} {traps:?}");
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_boss_fight_alone_is_played_by_its_own_team_with_its_allies() {
    let art = art().expect("the extracted data/");
    let text = "name: The Den
floors: 0
look: Apple Woods
boss: Spearow 12
boss_arena: 15x9
boss_team_at: 7 7
boss_allies: Pidgey 9 @2,7, Caterpie 6
boss_hero: Eevee
boss_level: 20
";
    let team = crate::dive::Team { hero: 25, partner: 393, level: Some(5) };
    let diving = crate::dive::Diving::of_text(&art.folder, &art.project, text, "trial", team, &crate::dive::Carried::default()).expect("it goes in");
    let (standing, ..) = diving.laid_out();
    let at = |x: i32, y: i32| (x + 5, y + 5);
    let named = |name: &str| standing.iter().filter(|(who, _)| who == name).map(|(_, at)| *at).collect::<Vec<_>>();
    // (Its own leader, at its level; the partner the one that came; its allies by them.)
    assert_eq!(named("Eevee"), vec![at(7, 7)], "{standing:?}");
    assert_eq!(named("Pidgey"), vec![at(2, 7)], "{standing:?}");
    assert_eq!(named("Caterpie").len(), 1, "{standing:?}");
    assert_eq!(diving.team(), (133, 393, 20));
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_boss_fight_of_its_own_plays_alone_by_its_name() {
    let art = art().expect("the extracted data/");
    let game = crate::testing::library().at(crate::testing::root().join("tests/games/test_village"));
    let (text, path) = crate::dive::text_of(&game, "test_forest_boss").expect("the fight");
    let team = crate::dive::Team { hero: 25, partner: 25, level: Some(30) };
    let mut diving = crate::dive::Diving::of_text(&art.folder, &game, &text, &path, team, &crate::dive::Carried::default()).expect("the fight alone");
    for _ in 0..10 {
        diving.tick(&shell::Keys::default());
    }
    assert_eq!(diving.floor(), Some((1, 1)));
    assert!(diving.foes().iter().any(|(name, ..)| name == "Butterfree"), "{:?}", diving.foes());
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_game_of_a_film_is_kept_through_a_dungeon_and_a_save() {
    let art = art().expect("the extracted data/");
    let mut playthrough = crate::playthrough::Playthrough::new(&art.folder, &art.project).expect("a game");
    playthrough.team(25, 393);
    playthrough.give("Apple", 2);
    playthrough.give("Letter", 1);
    playthrough.money(150);
    let (numbers, words) = playthrough.numbers();
    let number = |name: &str| numbers.iter().find(|(known, _)| known == name).map(|(_, value)| *value);
    assert_eq!((number("bag_apple"), number("bag_letter"), number("money")), (Some(2.0), Some(1.0), Some(150.0)), "{numbers:?}");
    assert!(words.iter().any(|(name, said)| name == "game_team" && said.contains("Lv.")), "{words:?}");
    // (Lent to a dungeon at level 9, given back: the team is still at 9, its bag as it was.)
    let text = test_forest();
    let run = playthrough.lend().expect("lent");
    let diving = crate::dive::Diving::with_run(&art.folder, &art.project, &text, "trial", run, Some(9), None).map_err(|(why, _)| why).expect("in");
    assert_eq!(diving.team(), (25, 393, 9));
    playthrough.put_back(diving.give_back().expect("given back"));
    let (numbers, _) = playthrough.numbers();
    let number = |name: &str| numbers.iter().find(|(known, _)| known == name).map(|(_, value)| *value);
    assert_eq!((number("hero_level"), number("bag_apple")), (Some(9.0), Some(2.0)), "{numbers:?}");
    // (Saved and read back: the same.)
    let mut again = crate::playthrough::Playthrough::new(&art.folder, &art.project).expect("a game");
    again.read(&playthrough.written());
    assert_eq!(again.numbers(), playthrough.numbers());
    assert_eq!(again.run().map(|run| run.team().species), Some((25, 393)));
}
