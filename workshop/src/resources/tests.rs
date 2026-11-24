//! The tests of the Resources: the lines the Inspector offers, a word replaced in a film's text.

use super::inspector::word_replaced;
use super::shelves::drawn_name;

/// The lines the Inspector offers are read by the language without a fault.
#[test]
fn the_lines_offered_are_good_lines() {
    for line in ["pikachu anim Attack", "pikachu anim Sleep", "pikachu hold torch", "music peace", "sound click", "pikachu walk to right"] {
        let text = format!("scene: beach pikachu\n{line}\n");
        let wrong: Vec<String> = film::service::diagnostics(&text, workshop_base::session::Session::for_tests().tables())
            .into_iter()
            .filter(|told| told.is_error())
            .map(|told| told.message)
            .collect();
        assert!(wrong.is_empty(), "{line}: {wrong:?}");
    }
    assert_eq!(word_replaced("hold pikachu torch\ntorch2 torch", "torch", "flare"), "hold pikachu flare\ntorch2 flare");
}

/// The grid's sums: how the cards lie, where the scroll goes, where the arrows go, a name cut, the sort said.
#[test]
fn the_grid_lays_out_its_cards() {
    use super::grid::{Layout, card_label};
    use super::toolbar::sort_said;
    let layout = Layout::of(400.0, 84.0, false);
    assert_eq!((layout.columns, layout.row_high), (4, 118.0));
    assert_eq!(Layout::of(400.0, 84.0, true).columns, 1);
    assert_eq!(Layout::of(10.0, 176.0, false).columns, 1, "at least one");
    assert_eq!(layout.offset_of(1), 0.0);
    assert_eq!(layout.offset_of(7), 118.0 - 60.0);
    assert_eq!(layout.after_arrows(4, 8, [false, true, false, false]), 5);
    assert_eq!(layout.after_arrows(4, 8, [false, false, false, true]), 7);
    assert_eq!(layout.after_arrows(1, 8, [false, false, true, false]), 0, "no farther than the first");
    assert_eq!(card_label("charmander", 5), "charm");
    assert_eq!(card_label("charmander •", 5), "cha •");
    assert_eq!((sort_said(0), sort_said(4)), ("Sort: Number", "Sort: Name"));
}

/// A drawn picture's file: its name and how many images it holds; a large one left out.
#[test]
fn a_drawn_file_names_its_picture_and_its_images() {
    assert_eq!(drawn_name("smoke.4"), Some(("smoke", 4)));
    assert_eq!(drawn_name("sign"), Some(("sign", 1)));
    assert_eq!(drawn_name("my.sign"), Some(("my.sign", 1)));
    assert_eq!(drawn_name("smoke_grand"), None);
}

/// What the film language finds wrong in a text (its errors, its warnings), each with its line.
fn problems_in(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    film::service::diagnostics(text, workshop_base::session::Session::for_tests().tables())
        .into_iter()
        .filter(|told| !told.is_remark())
        .map(|told| {
            format!(
                "{:?} line {} “{}”: {}",
                told.severity,
                told.span.start.line,
                lines.get(told.span.start.line.saturating_sub(1)).unwrap_or(&""),
                told.message
            )
        })
        .collect()
}

/// A shelf's things, as the Library lists them of the game's data (none without it).
fn listed(shelf: super::Shelf) -> Vec<super::Item> {
    let session = workshop_base::session::Session::for_tests();
    super::shelves::listed(&session, shelf)
}

/// Every line a shelf's cards copy, in one film on the beach with Pikachu: no problem.
fn every_line_good(shelf: super::Shelf) {
    let lines: Vec<String> = listed(shelf).iter().filter_map(super::Item::line).collect();
    if assets::data::has(&assets::testing::data(), "dungeons.bin") {
        assert!(!lines.is_empty(), "{shelf:?}: none listed");
    }
    let text = format!("scene: beach pikachu\n{}\n", lines.join("\nwait 0.2s\n"));
    let wrong = problems_in(&text);
    assert!(wrong.is_empty(), "{shelf:?}, {} lines: {wrong:#?}", lines.len());
}

/// The Attacks' lines (`effect Thunderbolt on pikachu`), each move with the effects it shows.
#[test]
fn the_attacks_lines_are_good() {
    every_line_good(super::Shelf::Attacks);
    let attacks = listed(super::Shelf::Attacks);
    if let Some(thunderbolt) = attacks.iter().find(|item| item.word == "thunderbolt") {
        assert_eq!(thunderbolt.line().as_deref(), Some("effect Thunderbolt on pikachu"));
    }
}

/// The status effects' lines (`status pikachu burn`).
#[test]
fn the_status_lines_are_good() {
    every_line_good(super::Shelf::Statuses);
}

/// The weathers' lines (`sky rain`).
#[test]
fn the_weather_lines_are_good() {
    every_line_good(super::Shelf::Weather);
    assert_eq!(listed(super::Shelf::Weather).len(), super::game_shelves::WEATHERS.len());
}

/// The other effects' lines (`effect pikachu 12`).
#[test]
fn the_other_effects_lines_are_good() {
    every_line_good(super::Shelf::Effects);
}

/// The items' lines (`hold pikachu Oran Berry`).
#[test]
fn the_items_lines_are_good() {
    every_line_good(super::Shelf::Items);
}

/// The portraits' lines (`pikachu (happy): ...`): each Pokémon's in a scene of its own.
#[test]
fn the_portraits_lines_are_good() {
    let mut text = String::new();
    let mut last = String::new();
    for item in listed(super::Shelf::Portraits) {
        let who = item.word.split_once(' ').map_or(item.word.clone(), |(who, _)| who.to_string());
        if who != last {
            text.push_str(&format!("scene: beach {who}\n"));
            last = who;
        }
        text.push_str(&format!("{}\n", item.line().unwrap_or_default()));
    }
    let wrong = problems_in(&text);
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert_eq!(super::game_shelves::portrait_face("pikachu (teary_eyed)"), Some(8));
}

/// The backdrops' lines (`scene: beach`): each a scene.
#[test]
fn the_backdrops_lines_are_good() {
    let lines: Vec<String> = listed(super::Shelf::Backdrops).iter().filter_map(super::Item::line).collect();
    let text = lines.iter().map(|line| format!("{line}\nwait 0.2s\n")).collect::<String>();
    let wrong = problems_in(&text);
    assert!(wrong.is_empty(), "{} scenes: {wrong:#?}", lines.len());
}

/// The tilesets' lines (`look: Beach Cave 2`): what a `.dungeon` takes the look of.
#[test]
fn the_tilesets_lines_are_good() {
    let session = workshop_base::session::Session::for_tests();
    let Some(offered) = crate::dungeon::offered(&session) else { return };
    let known = game::dungeon_check::Known::of(&offered.book);
    for line in listed(super::Shelf::Tilesets).iter().filter_map(super::Item::line) {
        let dungeon = game::dungeon_check::Dungeon::read(&format!("name: Tried\nfloors: 3\n{line}\n"));
        let wrong: Vec<String> = game::dungeon_check::problems(&dungeon, Some((&known, &offered.book)), "", std::path::Path::new("."))
            .into_iter()
            .filter(|problem| problem.place == game::dungeon_check::Place::Look)
            .map(|problem| problem.told.message)
            .collect();
        assert!(wrong.is_empty(), "{line}: {wrong:?}");
    }
}

/// The groups of the Library hold each shelf once.
#[test]
fn each_shelf_is_in_one_group() {
    for shelf in super::Shelf::ALL {
        assert_eq!(super::Shelf::GROUPS.iter().filter(|(_, shelves)| shelves.contains(&shelf)).count(), 1, "{shelf:?}");
    }
    assert_eq!(super::Shelf::Statuses.path(), "Effects › Status Effects");
    assert_eq!(super::Shelf::Pieces.path(), "Decor (Elements)");
}
