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
    let Some(offered) = workshop_dungeon::offered(&session) else { return };
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

/// A small catalog, as the project's files would make it: the game's (a second gender, Unown's letters, the
/// story's Dialga, Mama, a decoy), the community's (a species past the game's, a variant painted by hand, forms),
/// SpriteCollab's not imported.
fn small_catalog() -> super::catalog::Catalog {
    use super::catalog::{Catalog, Input};
    let monster = |number: i64, words: &[&str], name: &str| (number, words.iter().map(|word| word.to_string()).collect(), name.to_string());
    let input = Input {
        monsters: vec![
            monster(4, &["charmander"], "Charmander"),
            monster(39, &["jigglypuff"], "Jigglypuff"),
            monster(40, &["wigglytuff"], "Wigglytuff"),
            monster(575, &["mama"], "Mama"),
            monster(122, &["mrmime"], "Mr. Mime"),
            monster(133, &["eevee"], "Eevee"),
            monster(733, &["eevee_733"], "Eevee"),
            monster(201, &["unown"], "Unown"),
            monster(202, &["unown_202"], "Unown"),
            monster(483, &["dialga"], "Dialga"),
            monster(552, &["dialga_552", "dialga_primal"], "Dialga"),
            monster(553, &["decoy"], "Decoy"),
            monster(2810, &["grookey"], "Grookey"),
            monster(5205, &["eeveealternate"], "Eevee Alternate"),
        ],
        variants: vec![
            (5205, 133, "eevee_alternate".to_string(), false),
            (5206, 133, "eevee_alternate_two".to_string(), false),
            (5300, 4, "charmander_pirate".to_string(), true),
        ],
        dex: [(4, 4), (39, 39), (40, 40), (575, 40), (122, 122), (133, 133), (733, 133), (201, 201), (202, 201), (483, 483), (552, 483), (553, 0)]
            .into_iter()
            .collect(),
        types: [(4, (2, 0)), (133, (1, 0)), (40, (1, 0))].into_iter().collect(),
        missing: vec![
            spritecollab::Missing { unit: (906, 0), species: "Sprigatito".to_string(), form: String::new(), word: String::new() },
            spritecollab::Missing { unit: (133, 9), species: "Eevee".to_string(), form: "Holiday_Hat".to_string(), word: "eevee_holiday_hat".to_string() },
        ],
    };
    Catalog::build(&input)
}

/// The catalog: each species once, in the Pokédex's order, its forms under it named; Mama a form of Wigglytuff,
/// not a species between Jigglypuff and Wigglytuff; the game's others last.
#[test]
fn the_catalog_groups_the_forms_under_their_species() {
    use super::catalog::Source;
    let catalog = small_catalog();
    let names: Vec<&str> = catalog.species.iter().map(|species| species.name.as_str()).collect();
    assert_eq!(names, ["Charmander", "Jigglypuff", "Wigglytuff", "Mr. Mime", "Eevee", "Unown", "Dialga", "Grookey", "Sprigatito", "Decoy"]);
    let eevee = &catalog.species[4];
    let forms: Vec<(&str, &str, Source)> = eevee.entries.iter().map(|entry| (entry.word.as_str(), entry.form.as_str(), entry.source)).collect();
    assert_eq!(
        forms,
        [
            ("eevee", "", Source::Game),
            ("eevee_alternate", "Alternate", Source::Community),
            ("eevee_alternate_two", "Alternate Two", Source::Community),
            ("eevee_holiday_hat", "Holiday Hat", Source::Missing),
        ]
    );
    assert_eq!(eevee.entries[2].name, "Eevee · Alternate Two");
    assert!(catalog.find("eevee_733").is_none() && catalog.find("unown_202").is_none(), "a word ending in a number is no film's");
    let wigglytuff = &catalog.species[2];
    assert_eq!((wigglytuff.entries[1].word.as_str(), wigglytuff.entries[1].name.as_str()), ("mama", "Wigglytuff · Mama"));
    assert_eq!((catalog.species[6].entries[1].word.as_str(), catalog.species[6].entries[1].form.as_str()), ("dialga_primal", "Primal"));
    assert_eq!(catalog.species[0].entries[1].source, Source::Hand);
    assert_eq!((catalog.species[7].generation, catalog.species[7].entries[0].source), (8, Source::Community));
    assert_eq!((catalog.species[8].entries[0].word.as_str(), catalog.species[8].entries[0].number), ("sprigatito", -1));
    assert_eq!(catalog.species[9].dex, 0);
    assert_eq!(catalog.species[0].types_said(), "Fire");
    assert_eq!(catalog.entry("mama").map(|(species, _)| species.name.as_str()), Some("Wigglytuff"));
}

/// The catalog sought: by name (accents, case, `_` and spaces left out), by number, by type, by generation; a
/// form found alone shows with its species; the closest first.
#[test]
fn the_catalog_is_sought_by_name_number_type_generation() {
    let catalog = small_catalog();
    let names = |said: &str| -> Vec<String> { catalog.search(said).iter().map(|found| catalog.species[found.species].name.clone()).collect() };
    for said in ["eevee", "Éevée", "EEVEE", "  eev  "] {
        assert_eq!(names(said), ["Eevee"], "{said}");
    }
    let eevee = &catalog.search("eevee")[0];
    assert!(eevee.whole);
    assert_eq!(eevee.entries.len(), 4, "the species found: all of its forms");
    for said in ["mr mime", "mr_mime", "Mr. Mime", "mrmime"] {
        assert_eq!(names(said), ["Mr. Mime"], "{said}");
    }
    assert_eq!(names("133"), ["Eevee"]);
    assert_eq!(names("#40"), ["Wigglytuff"]);
    assert_eq!(names("fire"), ["Charmander"]);
    assert_eq!(names("normal gen 1"), ["Wigglytuff", "Eevee"]);
    for said in ["gen 8", "gen8", "g8", "generation 8"] {
        assert_eq!(names(said), ["Grookey"], "{said}");
    }
    let alternate_two = catalog.search("alternate two");
    assert_eq!(names("alternate two"), ["Eevee"]);
    assert!(!alternate_two[0].whole);
    assert_eq!(alternate_two[0].entries, [2]);
    assert_eq!(names("mama"), ["Wigglytuff"]);
    // (The name itself first, then those beginning with it, then the rest.)
    assert_eq!(names("dialga"), ["Dialga"]);
    assert_eq!(names("pirate"), ["Charmander"]);
    assert!(names("nothing like it").is_empty());
}

/// The cards in rows: the folded species side by side, an open one on rows of its own, its forms after it; a
/// species found by a form alone opens on that form.
#[test]
fn the_cards_lie_in_rows() {
    use super::pokemon_cards::{Row, Shown, rows, shown};
    let catalog = small_catalog();
    let all = shown(&catalog, None, |species, _| species.name == "Eevee");
    assert_eq!(all.len(), catalog.species.len());
    assert_eq!(all[4].entries.len(), 4);
    let laid = rows(&all, 3);
    assert_eq!(laid[0], Row { cards: vec![(0, 0), (1, 0), (2, 0)], band: false });
    assert_eq!(laid[1], Row { cards: vec![(3, 0)], band: false }, "the row cut before the open species");
    assert_eq!(laid[2], Row { cards: vec![(4, 0), (4, 1), (4, 2)], band: true });
    assert_eq!(laid[3], Row { cards: vec![(4, 3)], band: true });
    assert_eq!(laid[4].cards[0], (5, 0));
    let found = catalog.search("alternate two");
    let opened = shown(&catalog, Some(&found[..]), |_, by_itself| by_itself);
    assert_eq!(opened, [Shown { species: 4, entries: vec![0, 2], open: true }]);
    let few = catalog.search("eevee");
    assert_eq!(shown(&catalog, Some(&few[..]), |_, by_itself| by_itself)[0].entries.len(), 4, "few species found: their forms open");
    assert_eq!(shown(&catalog, Some(&few[..]), |_, _| false)[0].entries, [0], "shut when asked");
}

/// The faces named as a film names them, the special ones too (no `15` nor `18`), in the films' order.
#[test]
fn every_face_has_its_name() {
    use super::pictures::{face_name, face_order};
    for face in 0..20 {
        assert!(face_name(face).parse::<u16>().is_err(), "face {face}: {}", face_name(face));
    }
    assert_eq!(
        (face_name(14), face_name(15), face_name(18), face_name(19)),
        ("special_one".into(), "special_two".into(), "special_three".into(), "special_four".into())
    );
    let order = face_order();
    assert_eq!(&order[..3], &[0, 1, 2]);
    assert!(order.iter().position(|face| *face == 17) < order.iter().position(|face| *face == 14), "sigh and stunned before the special ones");
    let text = "scene: beach eevee\neevee (special_two): Hm.\n";
    assert!(problems_in(text).iter().all(|problem| !problem.contains("Error")), "{:?}", problems_in(text));
}

/// A song's length: minutes, seconds, ↻ when it loops.
#[test]
fn a_song_length_is_said() {
    assert_eq!(super::music_cards::length_said(125.4, true), "2:05 ↻");
    assert_eq!(super::music_cards::length_said(59.6, false), "1:00");
}

/// The project's own catalog (with the game's files): Mama under Wigglytuff, Eevee's forms with it, every word
/// a Pokémon of the films.
#[test]
fn the_projects_catalog() {
    let session = workshop_base::session::Session::for_tests();
    let catalog = super::catalog::of_session(&session, &[]);
    if catalog.species.len() < 400 {
        return;
    }
    assert!(!catalog.species.iter().any(|species| species.name == "Mama"));
    let eevee = &catalog.search("eevee")[0];
    assert_eq!(catalog.species[eevee.species].name, "Eevee");
    let tables = session.tables();
    for species in &catalog.species {
        for entry in species.entries.iter().filter(|entry| entry.number >= 0) {
            assert!(tables.monsters.contains_key(&entry.word), "{} ({}) is no word of the films", entry.word, entry.name);
        }
    }
    let words: std::collections::HashSet<&str> = catalog.species.iter().flat_map(|species| species.entries.iter().map(|entry| entry.word.as_str())).collect();
    assert_eq!(words.len(), catalog.len(), "each word once");
}
