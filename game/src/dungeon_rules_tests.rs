//! The game's own dungeons played by what `dungeon_extras.bin` and
//! `guests.txt` add: their rules, hidden stairs, vaults, guests. (They need
//! the extracted `data/`: ignored, `cargo test -- --ignored`.)

use std::sync::Arc;

use rules::run::{Options, Run};

/// The game's book with what the extraction adds, if the files are there.
fn run() -> Option<Run> {
    let data = crate::testing::data_folder();
    shell::data::read(&data, "dungeon_extras.bin")?;
    let book = crate::book::read(&data).ok()?;
    Some(Run::new(Arc::new(book), Options::default()))
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_game_s_other_rules_secret_floors_and_guests_are_read() {
    let mut run = run().expect("the extracted data/ with dungeon_extras.bin");
    let book = run.book();
    let rules = |id: usize| book.tables.dungeons[id].rules.more;
    // The Test Dungeon alone lets the leader change; Zero Isle Center keeps
    // its traps hidden; Beach Cave drops treasure, the Star Cave not.
    assert!(rules(0).leader_change && !rules(1).leader_change);
    assert!(rules(103).no_trap_uncovering && !rules(1).no_trap_uncovering);
    assert!(rules(1).treasure_boxes && !rules(123).treasure_boxes);
    assert_eq!(rules(64).max_rescues, 20);
    // The Secret Bazaar's five, the Secret Room's five boxes.
    let secret = &book.extras.secret_floors;
    assert_eq!(secret.len(), 2);
    let species: Vec<u16> = secret[0].watchers.iter().map(|placed| placed.enemy.species).collect();
    assert_eq!(species.len(), 5, "{species:?}");
    assert!([309, 320, 345, 481, 505].iter().all(|one| species.contains(one)), "{species:?}");
    assert_eq!(secret[1].items.len(), 5);
    assert!(secret[0].exit.is_some() && secret[1].exit.is_some());
    // Treasure: weights for the species, lists for the ranks.
    assert!(book.extras.drops.iter().filter(|(weights, _)| weights[0] > 0).count() > 100);
    assert_eq!(book.extras.rank_lists.len(), 15);
    // A floor whose hidden stairs always come: there they are, hidden.
    let (dungeon, floor) = sure_hidden_stairs(&run).expect("a floor with hidden stairs at 100");
    run.enter_dungeon(dungeon, 7);
    for _ in 1..floor {
        run.skip_floor();
    }
    let here = run.dungeon().expect("in the dungeon");
    assert_eq!(here.floor_number, floor);
    let hidden = here.hidden_stairs.expect("hidden stairs");
    assert!(!hidden.found && here.floor.tile(hidden.at).is_walkable() && hidden.at != here.floor.stairs);
    // A floor with a vault: its door shut, its item inside (on most
    // floors: one is set where a room holds it).
    let (dungeon, floor) = vault_floor(&run).expect("a floor with a vault");
    let shut = (1..10).any(|seed| {
        run.enter_dungeon(dungeon, seed);
        for _ in 1..floor {
            run.skip_floor();
        }
        let here = run.dungeon().expect("in the dungeon");
        here.key_doors.first().is_some_and(|&(door, _)| {
            let near = |at: rules::dungeon::Pos| (at.x - door.x).abs().max((at.y - door.y).abs()) <= 3;
            here.floor.tile(door) == rules::dungeon::Tile::Wall && here.items.iter().any(|item| near(item.at))
        })
    });
    assert!(shut, "a vault, its door shut, its item behind");
    // Craggy Coast in the story: Bidoof comes along; not on a visit chosen.
    run.enter_story_dungeon(8, 3);
    let guests: Vec<u16> = run.dungeon().expect("in").monsters.iter().filter(|monster| monster.guest).filter_map(|monster| monster.species).collect();
    assert_eq!(guests.len(), 1, "{guests:?}");
    assert_eq!(run.book().species_name(guests[0]), "Bidoof");
    run.enter_dungeon(8, 3);
    assert!(run.dungeon().expect("in").monsters.iter().all(|monster| !monster.guest));
    // Hidden Land: no change of leader, no recruiting on the story's visit.
    run.enter_story_dungeon(38, 3);
    let here = run.dungeon().expect("in");
    assert!(here.hidden_land && !here.rules.recruitment);
}

/// A dungeon and floor whose hidden stairs always come, with two floors after.
fn sure_hidden_stairs(run: &Run) -> Option<(u16, u32)> {
    let book = run.book();
    (0..book.tables.dungeons.len()).find_map(|id| {
        let floors = book.tables.dungeons[id].floors.len();
        let extras = book.extras.floors.get(id)?;
        let at = extras.iter().position(|floor| floor.hidden_chance >= 100)?;
        let fixed = book.tables.dungeons[id].floors[at].fixed.is_some();
        (at + 2 < floors && !fixed).then_some((id as u16, at as u32 + 1))
    })
}

/// A dungeon and floor with a vault.
fn vault_floor(run: &Run) -> Option<(u16, u32)> {
    let book = run.book();
    (0..book.extras.floors.len()).find_map(|id| {
        let at = book.extras.floors[id].iter().position(|floor| floor.vault.is_some())?;
        Some((id as u16, at as u32 + 1))
    })
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_cafe_presses_a_gummi_of_the_bag_into_juice_that_raises_the_iq() {
    let mut run = run().expect("the extracted data/");
    let gummi = rules::dungeon::iq::FIRST_GUMMI;
    run.give_items(109, 1); // (an Apple)
    run.give_items(gummi, 1);
    let found = run.gummis();
    assert_eq!(found.len(), 1, "the Gummi alone, not the apple: {:?}", run.bag().items);
    let before = run.team().kept_member(true).stats.iq;
    let said = run.drink_juice(gummi, true);
    assert!(run.team().kept_member(true).stats.iq > before, "{said}");
    assert!(run.gummis().is_empty(), "the Gummi drunk");
}
